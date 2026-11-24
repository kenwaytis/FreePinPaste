mod editor;
mod icons;
mod pin;
mod settings;

use crate::{
    config::Config,
    desktop,
    platform::Platform,
    service::{Command, Event},
};
use anyhow::{Context as _, Result};
use async_channel::{Receiver, Sender};
use gpui::{prelude::*, *};
use gpui_component::{Root, Theme, ThemeMode};
use image::RgbaImage;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum CaptureMode {
    #[default]
    Edit,
    Copy,
}

pub struct State {
    pub config: Config,
    pub events: Sender<Event>,
    pub desktop: Sender<desktop::Request>,
    pub status: String,
    pub editor: Option<(WindowHandle<Root>, Entity<editor::Editor>)>,
    pub settings: Option<WindowHandle<Root>>,
    pub last_image: Option<Arc<RgbaImage>>,
    pub capturing: bool,
    pub capture_authorized: bool,
    capture_permission_known: bool,
    capture_pending: Option<CaptureMode>,
    capture_mode: CaptureMode,
    restart: Arc<AtomicBool>,
    pub platform: Platform,
    pub image_clipboard: crate::clipboard::ImageClipboard,
}
impl Global for State {}
impl Global for crate::ocr::OcrClient {}

pub fn init(
    cx: &mut App,
    config: Config,
    events: Sender<Event>,
    desktop: Sender<desktop::Request>,
    receiver: Receiver<Event>,
    initial: Option<Command>,
    restart: Arc<AtomicBool>,
) -> Result<()> {
    gpui_component::init(cx);
    Theme::change(ThemeMode::Light, None, cx);
    cx.set_global(State {
        config,
        events,
        desktop,
        status: "正在注册全局快捷键…".into(),
        editor: None,
        settings: None,
        last_image: None,
        capturing: false,
        capture_authorized: false,
        capture_permission_known: false,
        capture_pending: None,
        capture_mode: CaptureMode::Edit,
        restart,
        platform: Platform::connect()?,
        image_clipboard: crate::clipboard::ImageClipboard::new()?,
    });
    // GPUI 0.2.2 stops its X11 loop when the last window is destroyed. Keep an
    // unmapped window alive; never use a visible off-screen window as a substitute.
    let keeper = cx.open_window(
        WindowOptions {
            show: false,
            focus: false,
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(0.), px(0.)),
                size(px(1.), px(1.)),
            ))),
            ..Default::default()
        },
        |_, cx| cx.new(|_| Keeper),
    )?;
    // This GPUI release maps X11 windows even when show=false.
    keeper.update(cx, |_, window, cx| -> Result<()> {
        let platform = &cx.global::<State>().platform;
        let id = platform.bind_window(window, "FreePinPaste · 后台服务")?;
        platform.hide_keeper(id)
    })??;
    // Mutter may finish managing the first MapRequest after the initial hide.
    // Reapply after the first compositor frames; the empty keeper must not remain
    // focusable/visible in a background-only launch.
    #[cfg(target_os = "linux")]
    cx.spawn(async move |cx| {
        Timer::after(std::time::Duration::from_millis(250)).await;
        let _ = keeper.update(cx, |_, window, cx| -> Result<()> {
            let platform = &cx.global::<State>().platform;
            let id = platform.bind_window(window, "FreePinPaste · 后台服务")?;
            platform.hide_keeper(id)?;
            platform.show(id, false)
        });
    })
    .detach();
    cx.spawn(async move |cx| {
        while let Ok(event) = receiver.recv().await {
            if cx
                .update(|cx| {
                    let result = match event {
                        Event::Command(command) => dispatch(command, cx),
                        Event::Status(message) => {
                            status(message, cx);
                            Ok(())
                        }
                        Event::CapturePermission(allowed) => {
                            let state = cx.global_mut::<State>();
                            state.capture_authorized = allowed;
                            state.capture_permission_known = true;
                            let pending = state.capture_pending.take();
                            if let Some(mode) = pending {
                                request_capture(mode, cx)
                            } else {
                                if let Some(window) = cx.global::<State>().settings {
                                    let _ = window.update(cx, |_, w, _| w.refresh());
                                }
                                Ok(())
                            }
                        }
                        Event::CaptureAuthorized(result) => {
                            cx.global_mut::<State>().capturing = false;
                            match result {
                                Ok(()) => {
                                    cx.global_mut::<State>().capture_authorized = true;
                                    let mode = cx
                                        .global_mut::<State>()
                                        .capture_pending
                                        .take()
                                        .unwrap_or_default();
                                    request_capture(mode, cx)
                                }
                                Err(e) => {
                                    status(
                                        format!("授权未完成：{e}。请保持此窗口在前台后重试。"),
                                        cx,
                                    );
                                    Ok(())
                                }
                            }
                        }
                        Event::Captured(result) => {
                            cx.global_mut::<State>().capturing = false;
                            match result {
                                Ok(image) => {
                                    let mode = cx.global::<State>().capture_mode;
                                    open_editor(image, mode, cx)
                                }
                                Err(e) => {
                                    #[cfg(target_os = "linux")]
                                    {
                                        let state = cx.global_mut::<State>();
                                        state.capture_pending = Some(state.capture_mode);
                                        state.capture_authorized = false;
                                    }
                                    status(format!("截图未完成：{e}"), cx);
                                    open_settings(cx)
                                }
                            }
                        }
                    };
                    if let Err(e) = result {
                        status(format!("{e:#}"), cx);
                        let _ = open_settings(cx);
                    }
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
    std::thread::spawn(crate::raster::warm_fonts);
    if let Some(command) = initial {
        dispatch(command, cx)?;
    }
    Ok(())
}

struct Keeper;
impl Render for Keeper {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub fn status(message: String, cx: &mut App) {
    eprintln!("FreePinPaste: {message}");
    cx.global_mut::<State>().status = message;
    if let Some(window) = cx.global::<State>().settings {
        let _ = window.update(cx, |_, window, _| window.refresh());
    }
}

pub fn send(command: Command, cx: &App) {
    let _ = cx
        .global::<State>()
        .events
        .try_send(Event::Command(command));
}

pub fn dispatch(command: Command, cx: &mut App) -> Result<()> {
    match command {
        Command::Quit => cx.quit(),
        Command::Restart => {
            cx.global::<State>().restart.store(true, Ordering::SeqCst);
            cx.quit();
        }
        Command::Background => {}
        Command::Settings => open_settings(cx)?,
        Command::AuthorizeCapture => {
            if cx.global::<State>().capturing {
                return Ok(());
            }
            open_settings(cx)?;
            let handle = cx.global::<State>().settings.unwrap();
            handle.update(cx, |_, window, cx| -> Result<()> {
                let id = cx
                    .global::<State>()
                    .platform
                    .bind_window(window, "FreePinPaste · 设置")?;
                cx.global::<State>()
                    .desktop
                    .try_send(desktop::Request::AuthorizeCapture(id))?;
                cx.global_mut::<State>().capturing = true;
                Ok(())
            })??;
        }
        Command::Capture => request_capture(CaptureMode::Edit, cx)?,
        Command::CaptureCopy => request_capture(CaptureMode::Copy, cx)?,
        Command::Pin => {
            if let Some((handle, entity)) = cx.global::<State>().editor.clone() {
                if handle
                    .update(cx, |_, window, cx| {
                        entity.update(cx, |editor, cx| editor.pin(window, cx))
                    })
                    .is_ok()
                {
                    return Ok(());
                }
                cx.global_mut::<State>().editor = None;
            }
            #[cfg(target_os = "windows")]
            let clipboard = cx.global::<State>().image_clipboard.read().map(Arc::new);
            #[cfg(target_os = "linux")]
            let clipboard = cx.read_from_clipboard().and_then(|item| {
                item.entries().iter().find_map(|entry| {
                    if let ClipboardEntry::Image(image) = entry {
                        image::load_from_memory(image.bytes())
                            .ok()
                            .map(|i| Arc::new(i.to_rgba8()))
                    } else {
                        None
                    }
                })
            });
            let image = clipboard
                .or_else(|| cx.global::<State>().last_image.clone())
                .context("剪贴板和历史中都没有图片，请先截图")?;
            pin::open(image, None, cx)?;
        }
        Command::Edit(path) => {
            open_editor(image::open(path)?.to_rgba8(), CaptureMode::Edit, cx)?;
        }
    }
    Ok(())
}

fn request_capture(mode: CaptureMode, cx: &mut App) -> Result<()> {
    if cx.global::<State>().capturing {
        return Ok(());
    }
    if !cx.global::<State>().capture_permission_known {
        cx.global_mut::<State>().capture_pending = Some(mode);
        return Ok(());
    }
    if !cx.global::<State>().capture_authorized {
        cx.global_mut::<State>().capture_pending = Some(mode);
        open_settings(cx)?;
        status(
            "首次使用需要截图授权：请点击右上角「授权截图」，在 GNOME 弹窗中允许。".into(),
            cx,
        );
        return Ok(());
    }
    cx.global_mut::<State>().capture_pending = None;
    if let Some((window, entity)) = cx.global::<State>().editor.clone() {
        if entity.read(cx).capture_mode == mode {
            if window.update(cx, |_, w, _| w.activate_window()).is_ok() {
                return Ok(());
            }
        } else {
            // Switching between editing and quick copy starts a fresh capture.
            let _ = window.update(cx, |_, w, _| w.remove_window());
        }
        cx.global_mut::<State>().editor = None;
    }
    if let Some(window) = cx.global_mut::<State>().settings.take() {
        let _ = window.update(cx, |_, w, cx| {
            #[cfg(target_os = "windows")]
            if let Ok(id) = cx
                .global::<State>()
                .platform
                .bind_window(w, "FreePinPaste · 设置")
            {
                let _ = cx.global::<State>().platform.show(id, false);
            }
            #[cfg(target_os = "linux")]
            let _ = cx;
            w.remove_window();
        });
    }
    cx.global::<State>()
        .desktop
        .try_send(desktop::Request::Capture)?;
    let state = cx.global_mut::<State>();
    state.capture_mode = mode;
    state.capturing = true;
    Ok(())
}

fn open_editor(image: RgbaImage, mode: CaptureMode, cx: &mut App) -> Result<()> {
    if let Some((window, _)) = cx.global_mut::<State>().editor.take() {
        let _ = window.update(cx, |_, w, _| w.remove_window());
    }
    let desktop = cx.global::<State>().platform.desktop()?;
    let bounds = Bounds::new(
        point(px(desktop.x as f32), px(desktop.y as f32)),
        size(px(desktop.width as f32), px(desktop.height as f32)),
    );
    let mut editor = None;
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            app_id: Some(crate::APP_ID.into()),
            window_decorations: Some(WindowDecorations::Client),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| editor::Editor::new(image, mode, desktop, window, cx));
            editor = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    let editor = editor.unwrap();
    handle.update(cx, |_, window, cx| -> Result<()> {
        let platform = &cx.global::<State>().platform;
        let id = platform.bind_window(window, "FreePinPaste · 截图编辑")?;
        platform.overlay(id)?;
        editor.update(cx, |editor, _| editor.native_window = Some(id));
        Ok(())
    })??;
    cx.global_mut::<State>().editor = Some((handle, editor));
    Ok(())
}

pub fn open_settings(cx: &mut App) -> Result<()> {
    if let Some(handle) = cx.global::<State>().settings
        && handle.update(cx, |_, w, _| w.activate_window()).is_ok()
    {
        return Ok(());
    }
    let dimensions = cx
        .primary_display()
        .map(|display| {
            let available = display.bounds().size;
            size(
                px(660_f32.min((f32::from(available.width) - 64.).max(320.))),
                px(900_f32.min((f32::from(available.height) - 100.).max(300.))),
            )
        })
        .unwrap_or(size(px(660.), px(900.)));
    let handle = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None, dimensions, cx,
            ))),
            app_id: Some(crate::APP_ID.into()),
            titlebar: Some(TitlebarOptions {
                title: Some("FreePinPaste · 设置".into()),
                ..Default::default()
            }),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|cx| settings::Settings::new(window, cx));
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    #[cfg(target_os = "windows")]
    handle.update(cx, |_, window, cx| {
        cx.global::<State>()
            .platform
            .bind_window(window, "FreePinPaste · 设置")
    })??;
    cx.global_mut::<State>().settings = Some(handle);
    Ok(())
}

pub fn render_image(image: &RgbaImage) -> Arc<RenderImage> {
    let mut bgra = image.clone();
    for p in bgra.pixels_mut() {
        p.0.swap(0, 2);
    }
    Arc::new(RenderImage::new([image::Frame::new(bgra)]))
}

pub fn copy_image(image: &RgbaImage, cx: &mut App) -> Result<()> {
    cx.global::<State>().image_clipboard.copy(image)
}

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .bg(rgb(0xeaf1f7))
        .text_color(rgb(0x263747))
        .text_size(px(13.))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0xdceaf5)))
        .child(label.into())
}
