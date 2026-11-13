mod toolbar;

use super::{CaptureMode, State};
use crate::{
    document::{Annotation, Document, Point as P, Rect, Style, Tool},
    raster,
};
use gpui::{prelude::*, *};
use gpui_component::input::{Input, InputState};
use image::RgbaImage;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq)]
enum Panel {
    Text,
    Ocr,
}
enum Drag {
    Select(P),
    Resize(Rect, usize),
    Move(Rect, P),
    Draw,
}
#[derive(Clone, Copy)]
enum Action {
    Shape,
    Line,
    Pen,
    Text,
    Eraser,
    Ocr,
    Undo,
    Redo,
    Close,
    Pin,
    Save,
    Copy,
}

pub struct Editor {
    pub(super) xid: Option<u32>,
    pub(super) capture_mode: CaptureMode,
    document: Document,
    preview: Arc<RenderImage>,
    focus: FocusHandle,
    tool: Tool,
    style: Style,
    drag: Option<Drag>,
    draft: Option<Annotation>,
    scale: (f32, f32),
    panel: Option<Panel>,
    text: Entity<InputState>,
    text_anchor: P,
    message: String,
    busy: bool,
}
impl Editor {
    pub(super) fn new(
        image: RgbaImage,
        capture_mode: CaptureMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let preview = super::render_image(&image);
        let focus = cx.focus_handle();
        focus.focus(window);
        let text = cx.new(|cx| InputState::new(window, cx).multi_line(true));
        let style = Style {
            font_family: cx.global::<State>().config.font_family.clone(),
            ..Default::default()
        };
        Self {
            xid: None,
            capture_mode,
            document: Document::new(image),
            preview,
            focus,
            tool: Tool::Select,
            style,
            drag: None,
            draft: None,
            scale: (1., 1.),
            panel: None,
            text,
            text_anchor: P::default(),
            message: match capture_mode {
                CaptureMode::Edit => "拖动框选 · Esc 退出 · 选区内双击复制 · 空格切换选区移动",
                CaptureMode::Copy => "拖动框选 · 松开鼠标自动复制 · Esc / 右键取消",
            }
            .into(),
            busy: false,
        }
    }
    fn source_point(&self, p: gpui::Point<Pixels>) -> P {
        self.document.clamp(P::new(
            f32::from(p.x) / self.scale.0,
            f32::from(p.y) / self.scale.1,
        ))
    }
    fn screen_rect(&self, r: Rect) -> Rect {
        Rect {
            x: r.x * self.scale.0,
            y: r.y * self.scale.1,
            w: r.w * self.scale.0,
            h: r.h * self.scale.1,
        }
    }
    fn refresh_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match raster::composite(&self.document) {
            Ok(image) => {
                let old = std::mem::replace(&mut self.preview, super::render_image(&image));
                cx.drop_image(old, Some(window));
            }
            Err(e) => self.message = format!("渲染失败：{e:#}"),
        }
        cx.notify();
    }
    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.panel.is_some() || self.busy {
            return;
        }
        self.focus.focus(window);
        let p = self.source_point(event.position);
        if let Some(rect) = self.document.selection {
            for (index, handle) in rect.handles().iter().enumerate() {
                if ((handle.x - p.x) * self.scale.0).hypot((handle.y - p.y) * self.scale.1) < 8. {
                    self.drag = Some(Drag::Resize(rect, index));
                    return;
                }
            }
            if rect.contains(p) {
                if event.click_count == 2 && self.tool == Tool::Select {
                    self.perform(Action::Copy, window, cx);
                    return;
                }
                match self.tool {
                    Tool::Select => self.drag = Some(Drag::Move(rect, p)),
                    Tool::Text => {
                        self.text_anchor = p;
                        self.panel = Some(Panel::Text);
                        self.text.update(cx, |input, cx| {
                            input.set_value("", window, cx);
                            input.focus(window, cx);
                        });
                    }
                    tool => {
                        let mut style = self.style.clone();
                        if tool == Tool::Eraser {
                            style.width *= 5.;
                        }
                        self.draft = Some(Annotation::new(tool, p, style));
                        self.drag = Some(Drag::Draw);
                    }
                }
                cx.notify();
                return;
            }
        }
        // Starting another selection does not discard the undo history.
        self.document.selection = Some(Rect::between(p, p));
        self.drag = Some(Drag::Select(p));
        cx.notify();
    }
    fn mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let mut p = self.source_point(event.position);
        match self.drag {
            Some(Drag::Select(start)) => self.document.selection = Some(Rect::between(start, p)),
            Some(Drag::Resize(rect, handle)) => {
                self.document.selection = Some(rect.resize(handle, p))
            }
            Some(Drag::Move(rect, start)) => {
                self.document.selection = Some(Rect {
                    x: (rect.x + p.x - start.x)
                        .clamp(0., self.document.source.width() as f32 - rect.w),
                    y: (rect.y + p.y - start.y)
                        .clamp(0., self.document.source.height() as f32 - rect.h),
                    ..rect
                });
            }
            Some(Drag::Draw) => {
                if let Some(draft) = &mut self.draft {
                    if event.modifiers.shift
                        && matches!(
                            draft.tool,
                            Tool::Rectangle | Tool::Ellipse | Tool::Line | Tool::Arrow
                        )
                    {
                        let a = draft.points[0];
                        let dx = p.x - a.x;
                        let dy = p.y - a.y;
                        if matches!(draft.tool, Tool::Line | Tool::Arrow) {
                            let angle = (dy.atan2(dx) / (std::f32::consts::FRAC_PI_4)).round()
                                * std::f32::consts::FRAC_PI_4;
                            p = P::new(
                                a.x + dx.hypot(dy) * angle.cos(),
                                a.y + dx.hypot(dy) * angle.sin(),
                            );
                        } else {
                            let length = dx.abs().max(dy.abs());
                            p = P::new(a.x + length * dx.signum(), a.y + length * dy.signum());
                        }
                    }
                    draft.extend(p);
                }
            }
            None => return,
        }
        cx.notify();
    }
    fn mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        if let Drag::Select(start) = drag {
            self.document.selection = Some(Rect::between(start, self.source_point(event.position)));
        }
        if let Some(draft) = self.draft.take() {
            self.document.push(draft);
            self.refresh_preview(window, cx);
        }
        if let Some(rect) = self.document.selection
            && (rect.w < 2. || rect.h < 2.)
        {
            self.document.selection = None;
        }
        if self.capture_mode == CaptureMode::Copy && self.document.selection.is_some() {
            self.perform(Action::Copy, window, cx);
            return;
        }
        cx.notify();
    }
    fn commit_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panel != Some(Panel::Text) {
            return;
        }
        let value = self.text.read(cx).value().to_string();
        if !value.trim().is_empty() {
            let mut a = Annotation::new(Tool::Text, self.text_anchor, self.style.clone());
            a.text = value;
            self.document.push(a);
            self.refresh_preview(window, cx);
        }
        self.panel = None;
        self.focus.focus(window);
        cx.notify();
    }
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.global_mut::<State>().editor = None;
        cx.drop_image(self.preview.clone(), Some(window));
        window.remove_window();
    }
    pub fn pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.perform(Action::Pin, window, cx);
    }
    fn perform(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy && !matches!(action, Action::Close) {
            return;
        }
        match action {
            Action::Shape => {
                self.tool = if matches!(self.tool, Tool::Rectangle | Tool::Ellipse) {
                    Tool::Select
                } else {
                    Tool::Rectangle
                };
            }
            Action::Line => {
                self.tool = if matches!(self.tool, Tool::Line | Tool::Arrow) {
                    Tool::Select
                } else {
                    Tool::Arrow
                };
            }
            Action::Pen => {
                self.tool = if self.tool == Tool::Pen {
                    Tool::Select
                } else {
                    Tool::Pen
                }
            }
            Action::Text => self.tool = Tool::Text,
            Action::Eraser => self.tool = Tool::Eraser,
            Action::Undo => {
                self.document.undo();
                self.refresh_preview(window, cx);
            }
            Action::Redo => {
                self.document.redo();
                self.refresh_preview(window, cx);
            }
            Action::Close => self.close(window, cx),
            Action::Copy | Action::Pin | Action::Save | Action::Ocr => {
                self.commit_text(window, cx);
                match raster::export(&self.document) {
                    Ok(image) => {
                        let image = Arc::new(image);
                        cx.global_mut::<State>().last_image = Some(image.clone());
                        match action {
                            Action::Copy => match super::copy_image(&image, cx) {
                                Ok(()) => self.close(window, cx),
                                Err(e) => self.message = e.to_string(),
                            },
                            Action::Pin => {
                                let r = self.screen_rect(self.document.selection.unwrap());
                                match super::pin::open(image, Some((r.x, r.y)), cx) {
                                    Ok(()) => self.close(window, cx),
                                    Err(e) => self.message = e.to_string(),
                                }
                            }
                            Action::Save => self.save(image, window, cx),
                            Action::Ocr => self.ocr(image, window, cx),
                            _ => {}
                        }
                    }
                    Err(e) => self.message = e.to_string(),
                }
            }
        }
        cx.notify();
    }
    fn save(&mut self, image: Arc<RgbaImage>, window: &mut Window, cx: &mut Context<Self>) {
        let directory = cx.global::<State>().config.save_directory.clone();
        let xid = self.xid;
        // The fullscreen editor must not cover the system save dialog.
        if let Some(id) = xid {
            let _ = cx.global::<State>().x11.show(id, false);
        }
        self.busy = true;
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<_> {
                std::fs::create_dir_all(&directory)?;
                let bytes = raster::png(&image)?;
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(crate::portal::save_file(bytes, directory))
            })()
            .map_err(|e| format!("{e:#}"));
            let _ = tx.send_blocking(result);
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(result) = rx.recv().await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.busy = false;
                    if let Some(id) = xid {
                        let _ = cx.global::<State>().x11.show(id, true);
                        window.activate_window();
                    }
                    match result {
                        Ok(Some(path)) => {
                            super::status(format!("已保存 {}", path.display()), cx);
                            this.close(window, cx);
                        }
                        Ok(None) => this.message = "已取消保存，标注已保留".into(),
                        Err(e) => this.message = format!("保存失败：{e}"),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }
    fn ocr(&mut self, image: Arc<RgbaImage>, window: &mut Window, cx: &mut Context<Self>) {
        self.busy = true;
        self.panel = Some(Panel::Ocr);
        self.message = "正在本地 CPU 上识别…".into();
        self.text
            .update(cx, |input, cx| input.set_value("正在识别…", window, cx));
        let languages = cx.global::<State>().config.ocr_languages.clone();
        let (tx, rx) = async_channel::bounded(1);
        std::thread::spawn(move || {
            let result = crate::ocr::Ocr::discover()
                .and_then(|ocr| ocr.recognize(&image, &languages))
                .map_err(|e| format!("{e:#}"));
            let _ = tx.send_blocking(result);
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(result) = rx.recv().await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.busy = false;
                    let text = match result {
                        Ok(text) => {
                            this.message = if text.is_empty() {
                                "没有识别到文字".into()
                            } else {
                                "识别完成 · 可编辑后复制".into()
                            };
                            text
                        }
                        Err(e) => {
                            this.message = format!("OCR 失败：{e}");
                            String::new()
                        }
                    };
                    this.text.update(cx, |input, cx| {
                        input.set_value(text, window, cx);
                        input.focus(window, cx);
                    });
                    cx.notify();
                });
            }
        })
        .detach();
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let m = event.keystroke.modifiers;
        if self.capture_mode == CaptureMode::Copy {
            match key {
                "escape" => self.close(window, cx),
                "enter" if self.document.selection.is_some() => {
                    self.perform(Action::Copy, window, cx)
                }
                "c" if m.control && self.document.selection.is_some() => {
                    self.perform(Action::Copy, window, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            return;
        }
        if self.panel.is_some() {
            if key == "escape" {
                if self.busy {
                    self.close(window, cx);
                    return;
                }
                self.panel = None;
                self.focus.focus(window);
                cx.notify();
            }
            if key == "enter" && m.control {
                self.commit_text(window, cx);
            }
            return;
        }
        let action = match key {
            "escape" => Some(Action::Close),
            "enter" => Some(Action::Copy),
            "z" if m.control => Some(if m.shift { Action::Redo } else { Action::Undo }),
            "y" if m.control => Some(Action::Redo),
            "c" if m.control => Some(Action::Copy),
            "s" if m.control => Some(Action::Save),
            "r" => Some(Action::Shape),
            "l" => Some(Action::Line),
            "p" => Some(Action::Pen),
            "t" => Some(Action::Text),
            "e" => Some(Action::Eraser),
            "o" => Some(Action::Ocr),
            "space" => {
                self.tool = Tool::Select;
                cx.notify();
                None
            }
            _ => None,
        };
        if let Some(action) = action {
            self.perform(action, window, cx);
            cx.stop_propagation();
        }
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = window.viewport_size();
        let width = f32::from(viewport.width);
        let height = f32::from(viewport.height);
        self.scale = (
            width / self.document.source.width() as f32,
            height / self.document.source.height() as f32,
        );
        let selected = self.document.selection.map(|r| self.screen_rect(r));
        let mut root = div()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(black())
            .track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, window, cx| {
                    if this.tool != Tool::Select {
                        this.tool = Tool::Select;
                        cx.notify();
                    } else {
                        this.close(window, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(Self::key))
            .child(
                img(self.preview.clone())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .object_fit(ObjectFit::Fill),
            );
        if let Some(r) = selected {
            let draft = self.draft.clone();
            let scale = self.scale;
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        if let Some(draft) = draft {
                            let mask = ContentMask {
                                bounds: Bounds::new(
                                    point(px(r.x), px(r.y)),
                                    size(px(r.w), px(r.h)),
                                ),
                            };
                            window.with_content_mask(Some(mask), |window| {
                                paint_draft(&draft, scale, window)
                            });
                        }
                        for (x, y, w, h) in [
                            (0., 0., width, r.y),
                            (0., r.bottom(), width, (height - r.bottom()).max(0.)),
                            (0., r.y, r.x, r.h),
                            (r.right(), r.y, (width - r.right()).max(0.), r.h),
                        ] {
                            window.paint_quad(fill(
                                Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
                                rgba(0x07121f72),
                            ));
                        }
                        window.paint_quad(outline(
                            Bounds::new(point(px(r.x), px(r.y)), size(px(r.w), px(r.h))),
                            rgb(0x39c8ee),
                            BorderStyle::Solid,
                        ));
                        for p in r.handles() {
                            window.paint_quad(quad(
                                Bounds::new(
                                    point(px(p.x - 3.5), px(p.y - 3.5)),
                                    size(px(7.), px(7.)),
                                ),
                                px(3.5),
                                rgb(0xffffff),
                                px(1.5),
                                rgb(0x39c8ee),
                                BorderStyle::Solid,
                            ));
                        }
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            );
            let source = self.document.selection.unwrap();
            root = root.child(
                div()
                    .absolute()
                    .left(px(r.x))
                    .top(px((r.y - 31.).max(4.)))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(rgba(0x142536e8))
                    .text_color(white())
                    .text_size(px(13.))
                    .child(format!(
                        "{} × {} px",
                        source.w.ceil() as u32,
                        source.h.ceil() as u32
                    )),
            );
            if self.capture_mode == CaptureMode::Edit
                && self.drag.is_none()
                && source.w >= 2.
                && source.h >= 2.
            {
                let monitor = cx
                    .displays()
                    .into_iter()
                    .map(|d| d.bounds())
                    .find(|b| b.contains(&point(px(r.right() - 1.), px(r.bottom() - 1.))));
                let screen = monitor
                    .map(|b| Rect {
                        x: f32::from(b.origin.x),
                        y: f32::from(b.origin.y),
                        w: f32::from(b.size.width),
                        h: f32::from(b.size.height),
                    })
                    .unwrap_or(Rect {
                        x: 0.,
                        y: 0.,
                        w: width,
                        h: height,
                    });
                let toolbar = crate::document::toolbar_position(
                    r,
                    screen,
                    if self.tool == Tool::Text { 600. } else { 518. },
                    if self.tool == Tool::Select { 48. } else { 100. },
                );
                root = root.child(self.toolbar(toolbar, cx));
            }
        } else {
            root = root.child(div().absolute().inset_0().bg(rgba(0x07121f65)));
        }
        // Position the hint on the monitor containing the cursor, not in the 3-screen gap.
        let pointer = cx.global::<State>().x11.pointer().unwrap_or((0, 0));
        let active = cx
            .displays()
            .into_iter()
            .find(|d| {
                d.bounds()
                    .contains(&point(px(pointer.0 as f32), px(pointer.1 as f32)))
            })
            .map(|d| d.bounds());
        let hint_x = active.map(|b| f32::from(b.origin.x) + 20.).unwrap_or(20.);
        root = root.child(
            div()
                .absolute()
                .left(px(hint_x))
                .top(px(16.))
                .max_w(px(1000.))
                .px_3()
                .py_2()
                .rounded_md()
                .bg(rgba(0x142536e8))
                .text_size(px(12.))
                .text_color(white())
                .child(self.message.clone()),
        );
        if let Some(panel) = self.panel {
            root = root.child(self.panel_view(panel, window, cx));
        }
        root
    }
}

fn paint_draft(a: &Annotation, scale: (f32, f32), window: &mut Window) {
    let Some(path) = raster::annotation_path(a) else {
        return;
    };
    let filled = a.style.filled && matches!(a.tool, Tool::Rectangle | Tool::Ellipse);
    let mut builder = if filled {
        PathBuilder::fill()
    } else {
        PathBuilder::stroke(px(a.style.width * scale.0))
    };
    let p = |p: tiny_skia::Point| point(px(p.x * scale.0), px(p.y * scale.1));
    for segment in path.segments() {
        match segment {
            tiny_skia::PathSegment::MoveTo(a) => builder.move_to(p(a)),
            tiny_skia::PathSegment::LineTo(a) => builder.line_to(p(a)),
            tiny_skia::PathSegment::QuadTo(a, b) => builder.curve_to(p(b), p(a)),
            tiny_skia::PathSegment::CubicTo(a, b, c) => builder.cubic_bezier_to(p(c), p(a), p(b)),
            tiny_skia::PathSegment::Close => builder.close(),
        }
    }
    if let Ok(path) = builder.build() {
        window.paint_path(
            path,
            if a.tool == Tool::Eraser {
                rgba(0xffffffaa)
            } else {
                rgb(a.style.color)
            },
        );
    }
}
