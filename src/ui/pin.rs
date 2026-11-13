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
    scale_factor: f32,
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
                .platform
                .pointer()
                .ok()
                .map(|(x, y)| (x as f32, y as f32))
        })
        .unwrap_or((100., 100.));
    let desktop = cx.global::<State>().platform.desktop()?;
    let (x, y) = desktop.clamp_position(
        position.0,
        position.1,
        dimensions.width.into(),
        dimensions.height.into(),
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
                    scale_factor: window.scale_factor(),
                    message: String::new(),
                }
            })
        },
    )?;
    handle.update(cx, |pin, window, cx| -> Result<()> {
        let id = cx.global::<State>().platform.bind_window(
            window,
            "FreePinPaste · 贴图（拖动移动 / 滚轮缩放 / Esc 关闭）",
        )?;
        cx.global::<State>().platform.borderless(id)?;
        cx.global::<State>().platform.above(id, true)?;
        cx.global::<State>()
            .platform
            .move_to(id, x as i32, y as i32)?;
        // zoom=1 means one source pixel per physical display pixel, at any DPI.
        cx.global::<State>().platform.resize_physical(
            window,
            pin.image.width() as f32 * pin.zoom,
            pin.image.height() as f32 * pin.zoom,
        )?;
        Ok(())
    })??;
    Ok(())
}
impl Pin {
    fn resize(&mut self, window: &mut Window, cx: &App) {
        if let Err(error) = cx.global::<State>().platform.resize_physical(
            window,
            (self.image.width() as f32 * self.zoom).max(1.),
            (self.image.height() as f32 * self.zoom).max(1.),
        ) {
            self.message = error.to_string();
        }
    }
}
impl Render for Pin {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scale = window.scale_factor();
        if self.scale_factor != scale {
            self.scale_factor = scale;
            self.resize(window, cx);
        }
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
                        this.resize(window, cx);
                        cx.notify();
                    } else {
                        if let Err(error) = cx.global::<State>().platform.start_move(window) {
                            this.message = error.to_string();
                            cx.notify();
                        }
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
                this.resize(window, cx);
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
