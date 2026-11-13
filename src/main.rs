#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]

use anyhow::{Context, Result, bail};
use freepinpaste::{
    config::Config,
    service::{Command, Event},
    ui,
};
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn main() {
    #[cfg(target_os = "windows")]
    if std::env::args().skip(1).any(|arg| {
        matches!(
            arg.as_str(),
            "--help" | "-h" | "--version" | "--ocr" | "--languages"
        )
    }) {
        // Keep redirected stdout intact; interactive CLI users reuse the parent's
        // console. Normal GUI launches never allocate or flash a console window.
        use windows::Win32::{Storage::FileSystem::*, System::Console::*};
        unsafe {
            let redirected = GetStdHandle(STD_OUTPUT_HANDLE)
                .is_ok_and(|handle| matches!(GetFileType(handle), FILE_TYPE_PIPE | FILE_TYPE_DISK));
            if !redirected {
                let _ = AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
    if let Err(error) = run() {
        eprintln!("FreePinPaste: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut command = Command::Settings;
    let mut background = false;
    let mut no_shortcuts = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "FreePinPaste 0.1.0\n\n  --capture       截图（默认 F1）\n  --capture-copy  框选后自动复制\n  --restart       后台重启已有实例\n  --pin           置顶选区 / 剪贴板图片 / 最近截图（默认 F3）\n  --settings      打开设置\n  --background    后台常驻\n  --quit          退出已有实例\n  --edit FILE     在编辑器中打开本地图片\n  --ocr FILE      使用内置 CPU OCR 输出文本\n  --languages     列出随包 OCR 模型\n  --no-shortcuts  本次启动不申请全局快捷键（调试用）\n"
                );
                return Ok(());
            }
            "--version" => {
                println!("FreePinPaste {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--install-desktop" => {
                #[cfg(target_os = "linux")]
                {
                    freepinpaste::config::install_desktop_entry()?;
                    return Ok(());
                }
                #[cfg(target_os = "windows")]
                bail!("Windows 开始菜单入口由安装程序创建");
            }
            "--capture" => command = Command::Capture,
            "--capture-copy" => command = Command::CaptureCopy,
            "--restart" => command = Command::Restart,
            "--pin" => command = Command::Pin,
            "--settings" => command = Command::Settings,
            "--quit" => command = Command::Quit,
            "--background" => {
                background = true;
                command = Command::Background;
            }
            "--no-shortcuts" => no_shortcuts = true,
            "--edit" => {
                command = Command::Edit(std::fs::canonicalize(
                    args.next().context("--edit 需要文件路径")?,
                )?)
            }
            "--ocr" => {
                let path = args.next().context("--ocr 需要文件路径")?;
                let image = image::open(path)?.to_rgba8();
                println!(
                    "{}",
                    freepinpaste::ocr::Ocr::discover()?
                        .recognize(&image, &Config::load()?.ocr_languages)?
                );
                return Ok(());
            }
            "--languages" => {
                println!(
                    "{}",
                    freepinpaste::ocr::Ocr::discover()?.languages()?.join("\n")
                );
                return Ok(());
            }
            _ => bail!("未知参数 {arg}；使用 --help 查看帮助"),
        }
    }
    // GPUI components enable GPUI's Wayland feature transitively. Select X11 before
    // starting any threads; the desktop itself remains GNOME Wayland.
    // SAFETY: this is the single-threaded startup path, before GPUI/Tokio/X11 init.
    #[cfg(target_os = "linux")]
    unsafe {
        std::env::remove_var("WAYLAND_DISPLAY");
    }
    let config = Config::load()?;
    let (events, receiver) = async_channel::unbounded::<Event>();
    let Some(instance) = freepinpaste::service::claim(&command, events.clone())? else {
        return Ok(());
    };
    if matches!(command, Command::Quit) {
        return Ok(());
    }
    // With no running instance, --restart simply starts in the background.
    if matches!(command, Command::Restart) {
        background = true;
    }
    #[cfg(target_os = "linux")]
    freepinpaste::config::ensure_desktop_entry()?;
    if no_shortcuts {
        let _ = events.try_send(Event::Status("调试模式：本次未注册全局快捷键".into()));
    }
    let (desktop_tx, desktop_rx) = async_channel::unbounded();
    let _desktop_service =
        freepinpaste::desktop::start(config.clone(), desktop_rx, events.clone(), !no_shortcuts);
    #[cfg(target_os = "linux")]
    freepinpaste::tray::start(events.clone());
    let restart = Arc::new(AtomicBool::new(false));
    let restart_requested = restart.clone();
    gpui::Application::new().run(move |cx| {
        if let Err(e) = ui::init(
            cx,
            config,
            events,
            desktop_tx,
            receiver,
            if background { None } else { Some(command) },
            restart_requested,
        ) {
            eprintln!("FreePinPaste: {e:#}");
            cx.quit();
        }
    });
    #[cfg(target_os = "windows")]
    drop(_desktop_service);
    drop(instance);
    if restart.load(Ordering::SeqCst) {
        // Never launch while holding the single-instance lock: a new process
        // would only forward its command to the dying instance. exec also
        // closes the old Portal, tray, clipboard and X11 connections.
        let mut process = std::process::Command::new(std::env::current_exe()?);
        process.arg("--background");
        if no_shortcuts {
            process.arg("--no-shortcuts");
        }
        #[cfg(target_os = "linux")]
        return Err(process.exec()).context("重新启动 FreePinPaste 失败");
        #[cfg(target_os = "windows")]
        process.spawn().context("重新启动 FreePinPaste 失败")?;
    }
    Ok(())
}
