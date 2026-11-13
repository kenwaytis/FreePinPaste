use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum CanvasCursor {
    System(CursorStyle),
    Move,
    Pen,
    Draw,
    Eraser(f32, f32),
}

// Use the same screen-space hit radius for interaction and cursor feedback.
pub(super) fn resize_handle(rect: Rect, p: P, scale: (f32, f32)) -> Option<usize> {
    rect.handles()
        .iter()
        .position(|handle| ((handle.x - p.x) * scale.0).hypot((handle.y - p.y) * scale.1) < 8.)
}

fn resize_cursor(handle: usize) -> CanvasCursor {
    CanvasCursor::System(match handle {
        0 | 4 => CursorStyle::ResizeUpLeftDownRight,
        2 | 6 => CursorStyle::ResizeUpRightDownLeft,
        1 | 5 => CursorStyle::ResizeUpDown,
        3 | 7 => CursorStyle::ResizeLeftRight,
        _ => unreachable!("selection has eight handles"),
    })
}

fn tool_cursor(tool: Tool, width: f32, scale: (f32, f32)) -> CanvasCursor {
    match tool {
        Tool::Select => CanvasCursor::Move,
        Tool::Pen => CanvasCursor::Pen,
        Tool::Text => CanvasCursor::System(CursorStyle::IBeam),
        Tool::Eraser => CanvasCursor::Eraser(width * 5. * scale.0, width * 5. * scale.1),
        Tool::Rectangle | Tool::Ellipse | Tool::Line | Tool::Arrow => CanvasCursor::Draw,
    }
}

fn resolve_cursor(
    selection: Option<Rect>,
    p: P,
    scale: (f32, f32),
    tool: Tool,
    drag: Option<&Drag>,
    width: f32,
) -> CanvasCursor {
    // An active gesture wins over hover targets, even outside the selection.
    match drag {
        Some(Drag::Select(_)) => return CanvasCursor::System(CursorStyle::Crosshair),
        Some(Drag::Move(..)) => return CanvasCursor::Move,
        Some(Drag::Resize(_, handle)) => return resize_cursor(*handle),
        Some(Drag::Draw) => return tool_cursor(tool, width, scale),
        None => {}
    }
    if let Some(rect) = selection {
        if let Some(handle) = resize_handle(rect, p, scale) {
            return resize_cursor(handle);
        }
        if rect.contains(p) {
            return tool_cursor(tool, width, scale);
        }
    }
    CanvasCursor::System(CursorStyle::Crosshair)
}

impl Editor {
    pub(super) fn cursor_layer(&self, window: &Window) -> impl IntoElement {
        let cursor = if self.panel.is_some() || self.busy {
            CanvasCursor::System(CursorStyle::Arrow)
        } else {
            // Keep a drawing gesture's tool/width even if a shortcut changes the next tool.
            let (tool, width) = self
                .draft
                .as_ref()
                .map(|draft| {
                    let width = if draft.tool == Tool::Eraser {
                        draft.style.width / 5.
                    } else {
                        draft.style.width
                    };
                    (draft.tool, width)
                })
                .unwrap_or((self.tool, self.style.width));
            resolve_cursor(
                self.document.selection,
                self.source_point(window.mouse_position()),
                self.scale,
                tool,
                self.drag.as_ref(),
                width,
            )
        };
        canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                let hovered = hitbox.is_hovered(window);
                let hover_hitbox = hitbox.clone();
                // Repaint when entering a control, where the root's move listener is occluded.
                window.on_mouse_event(move |_: &MouseMoveEvent, phase, window, _| {
                    if phase == DispatchPhase::Capture && hovered != hover_hitbox.is_hovered(window)
                    {
                        window.refresh();
                    }
                });
                let style = match cursor {
                    CanvasCursor::System(style) => style,
                    _ => CursorStyle::None,
                };
                window.set_cursor_style(style, &hitbox);
                if hovered {
                    paint_cursor(cursor, window.mouse_position(), window);
                }
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }
}

// These decorations are UI-only: they never enter Document or raster::composite.
// White outer strokes and dark inner strokes remain visible on light/dark captures.
fn paint_cursor(cursor: CanvasCursor, origin: gpui::Point<Pixels>, window: &mut Window) {
    match cursor {
        CanvasCursor::System(_) => {}
        CanvasCursor::Move => paint_lines(
            &[
                &[(-10., 0.), (10., 0.)],
                &[(0., -10.), (0., 10.)],
                &[(-6., -4.), (-10., 0.), (-6., 4.)],
                &[(6., -4.), (10., 0.), (6., 4.)],
                &[(-4., -6.), (0., -10.), (4., -6.)],
                &[(-4., 6.), (0., 10.), (4., 6.)],
            ],
            origin,
            window,
        ),
        CanvasCursor::Pen => paint_lines(
            &[
                &[
                    (0., 0.),
                    (3., -8.),
                    (14., -19.),
                    (19., -14.),
                    (8., -3.),
                    (0., 0.),
                ],
                &[(3., -8.), (8., -3.)],
                &[(11., -16.), (16., -11.)],
            ],
            origin,
            window,
        ),
        CanvasCursor::Draw => paint_lines(
            &[
                &[(-8., 0.), (-2., 0.)],
                &[(2., 0.), (8., 0.)],
                &[(0., -8.), (0., -2.)],
                &[(0., 2.), (0., 8.)],
                &[(-0.5, 0.), (0.5, 0.)],
            ],
            origin,
            window,
        ),
        CanvasCursor::Eraser(width, height) => {
            let ring: Vec<_> = (0..=64)
                .map(|i| {
                    let angle = i as f32 * std::f32::consts::TAU / 64.;
                    (angle.cos() * width / 2., angle.sin() * height / 2.)
                })
                .collect();
            paint_lines(&[&ring], origin, window);
        }
    }
}

fn paint_lines(lines: &[&[(f32, f32)]], origin: gpui::Point<Pixels>, window: &mut Window) {
    for (width, color) in [(3., rgb(0xffffff)), (1., rgb(0x17212b))] {
        let mut path = PathBuilder::stroke(px(width));
        for line in lines {
            for (i, &(x, y)) in line.iter().enumerate() {
                let p = origin + point(px(x), px(y));
                if i == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CanvasCursor, resize_cursor, resize_handle, resolve_cursor};
    use crate::{
        document::{Point as P, Rect, Tool},
        ui::editor::Drag,
    };
    use gpui::CursorStyle;

    const RECT: Rect = Rect {
        x: 20.,
        y: 20.,
        w: 100.,
        h: 80.,
    };

    #[test]
    fn hover_matches_the_available_operation() {
        let resolve = |selection, p, tool| resolve_cursor(selection, p, (1., 1.), tool, None, 3.);
        let crosshair = CanvasCursor::System(CursorStyle::Crosshair);
        assert_eq!(resolve(None, P::new(50., 50.), Tool::Select), crosshair);
        assert_eq!(resolve(Some(RECT), P::new(0., 0.), Tool::Pen), crosshair);
        for (tool, expected) in [
            (Tool::Select, CanvasCursor::Move),
            (Tool::Pen, CanvasCursor::Pen),
            (Tool::Text, CanvasCursor::System(CursorStyle::IBeam)),
            (Tool::Rectangle, CanvasCursor::Draw),
            (Tool::Ellipse, CanvasCursor::Draw),
            (Tool::Line, CanvasCursor::Draw),
            (Tool::Arrow, CanvasCursor::Draw),
            (Tool::Eraser, CanvasCursor::Eraser(15., 15.)),
        ] {
            assert_eq!(resolve(Some(RECT), P::new(50., 50.), tool), expected);
        }
    }

    #[test]
    fn handles_take_priority_and_keep_the_screen_space_hit_radius() {
        let styles = [
            CursorStyle::ResizeUpLeftDownRight,
            CursorStyle::ResizeUpDown,
            CursorStyle::ResizeUpRightDownLeft,
            CursorStyle::ResizeLeftRight,
            CursorStyle::ResizeUpLeftDownRight,
            CursorStyle::ResizeUpDown,
            CursorStyle::ResizeUpRightDownLeft,
            CursorStyle::ResizeLeftRight,
        ];
        for (p, style) in RECT.handles().into_iter().zip(styles) {
            assert_eq!(
                resolve_cursor(Some(RECT), p, (2., 2.), Tool::Pen, None, 3.),
                CanvasCursor::System(style),
            );
        }
        assert_eq!(resize_handle(RECT, P::new(23., 20.), (2., 2.)), Some(0));
        assert_eq!(resize_handle(RECT, P::new(24., 20.), (2., 2.)), None);
    }

    #[test]
    fn gestures_keep_their_cursor_across_selection_boundaries() {
        for (drag, tool, expected) in [
            (
                Drag::Select(P::default()),
                Tool::Pen,
                CanvasCursor::System(CursorStyle::Crosshair),
            ),
            (
                Drag::Move(RECT, P::default()),
                Tool::Select,
                CanvasCursor::Move,
            ),
            (Drag::Resize(RECT, 0), Tool::Select, resize_cursor(0)),
            (Drag::Draw, Tool::Pen, CanvasCursor::Pen),
            (Drag::Draw, Tool::Eraser, CanvasCursor::Eraser(30., 15.)),
        ] {
            for p in [P::new(0., 0.), P::new(50., 50.), RECT.handles()[2]] {
                assert_eq!(
                    resolve_cursor(Some(RECT), p, (2., 1.), tool, Some(&drag), 3.),
                    expected,
                );
            }
        }
    }
}
