use super::{State, render_image};
use crate::config::PinCloseAction;
use anyhow::Result;
use gpui::{prelude::*, *};
use image::RgbaImage;
use std::sync::Arc;

struct Pin {
    image: Arc<RgbaImage>,
    rendered: Arc<RenderImage>,
    focus: FocusHandle,
    zoom: f32,
    message: String,
}

pub fn open(image: Arc<RgbaImage>, position: Option<(f32, f32)>, cx: &mut App) -> Result<()> {
    let zoom = (900. / image.width() as f32)
        .min(650. / image.height() as f32)
        .min(1.);
    let dimensions = size(
        px(image.width() as f32 * zoom),
        px(image.height() as f32 * zoom),
    );
    let position = position
        .or_else(|| {
            cx.global::<State>()
                .x11
                .pointer()
                .ok()
                .map(|(x, y)| (x as f32, y as f32))
        })
        .unwrap_or((100., 100.));
    let desktop = cx.global::<State>().x11.desktop()?;
    let x = position.0.clamp(
        0.,
        (desktop.width as f32 - f32::from(dimensions.width)).max(0.),
    );
    let y = position.1.clamp(
        0.,
        (desktop.height as f32 - f32::from(dimensions.height)).max(0.),
    );
    let rendered = render_image(&image);
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(x), px(y)),
                dimensions,
            ))),
            titlebar: None,
            app_id: Some(crate::APP_ID.into()),
            window_decorations: Some(WindowDecorations::Client),
            is_resizable: false,
            ..Default::default()
        },
        |window, cx| {
            cx.new(|cx| {
                let focus = cx.focus_handle();
                focus.focus(window);
                Pin {
                    image,
                    rendered,
                    focus,
                    zoom,
                    message: String::new(),
                }
            })
        },
    )?;
    handle.update(cx, |_, window, cx| -> Result<()> {
        let xid = cx.global::<State>().x11.bind_window(
            window,
            "FreePinPaste · 贴图（拖动移动 / 滚轮缩放 / Esc 关闭）",
        )?;
        cx.global::<State>().x11.borderless(xid)?;
        cx.global::<State>().x11.above(xid, true)?;
        cx.global::<State>().x11.move_to(xid, x as i32, y as i32)?;
        Ok(())
    })??;
    Ok(())
}
impl Render for Pin {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0x40bce7))
            .track_focus(&self.focus)
            .child(
                img(self.rendered.clone())
                    .size_full()
                    .object_fit(ObjectFit::Fill),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if event.click_count == 2 {
                        if cx.global::<State>().config.pin_close_action
                            == PinCloseAction::DoubleClick
                        {
                            window.remove_window();
                            return;
                        }
                        this.zoom = 1.;
                        window.resize(size(
                            px(this.image.width() as f32),
                            px(this.image.height() as f32),
                        ));
                        cx.notify();
                    } else {
                        window.start_window_move();
                    }
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|_, _, window, cx| {
                    if cx.global::<State>().config.pin_close_action == PinCloseAction::RightClick {
                        window.remove_window();
                    }
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                let delta = event.delta.pixel_delta(px(20.));
                this.zoom =
                    (this.zoom * if delta.y > px(0.) { 1.1 } else { 1. / 1.1 }).clamp(0.1, 4.);
                window.resize(size(
                    px((this.image.width() as f32 * this.zoom).max(24.)),
                    px((this.image.height() as f32 * this.zoom).max(24.)),
                ));
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => window.remove_window(),
                    "c" if event.keystroke.modifiers.control => {
                        if let Err(e) = super::copy_image(&this.image, cx) {
                            this.message = e.to_string();
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            }))
            .when(!self.message.is_empty(), |d| {
                d.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .bg(rgb(0xffffff))
                        .child(self.message.clone()),
                )
            })
    }
}
