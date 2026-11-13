use crate::{
    config::Config,
    service::{Command, Event},
};
use anyhow::{Context, Result};
use ashpd::desktop::{
    global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut},
    screenshot::Screenshot,
};
use async_channel::{Receiver, Sender};
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug)]
pub enum Request {
    Capture,
    AuthorizeCapture(u32),
    ConfigureShortcuts,
}

pub fn start(
    config: Config,
    requests: Receiver<Request>,
    events: Sender<Event>,
    enable_shortcuts: bool,
) {
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            // A newly installed .desktop file reaches the portal's GAppInfo cache
            // asynchronously. Retry only this observed first-launch cache race.
            let mut registration = ashpd::register_host_app(crate::APP_ID.parse().unwrap()).await;
            for _ in 0..5 {
                if !registration
                    .as_ref()
                    .is_err_and(|e| e.to_string().contains("App info not found"))
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                registration = ashpd::register_host_app(crate::APP_ID.parse().unwrap()).await;
            }
            if let Err(e) = registration {
                let _ = events
                    .send(Event::Status(format!("Portal 应用注册失败：{e}")))
                    .await;
            }
            let _ = events
                .send(Event::CapturePermission(
                    capture_permission().await.unwrap_or(false),
                ))
                .await;
            let (configure_tx, configure_rx) = async_channel::bounded(1);
            let hotkeys = async {
                if enable_shortcuts {
                    shortcuts(config, configure_rx, events.clone()).await;
                }
            };
            let busy = Arc::new(AtomicBool::new(false));
            let captures = async {
                while let Ok(request) = requests.recv().await {
                    if matches!(request, Request::ConfigureShortcuts) {
                        let _ = configure_tx.try_send(());
                        continue;
                    }
                    if busy.swap(true, Ordering::SeqCst) {
                        continue;
                    }
                    let parent = match request {
                        Request::AuthorizeCapture(xid) => Some(xid),
                        _ => None,
                    };
                    let events = events.clone();
                    let busy = busy.clone();
                    tokio::spawn(async move {
                        // Only hide the UI AFTER permission was granted from a focused window.
                        if parent.is_none() {
                            tokio::time::sleep(std::time::Duration::from_millis(180)).await;
                        }
                        let result = capture(parent).await.map_err(|e| format!("{e:#}"));
                        busy.store(false, Ordering::SeqCst);
                        let event = if parent.is_some() {
                            Event::CaptureAuthorized(result.map(|_| ()))
                        } else {
                            Event::Captured(result)
                        };
                        let _ = events.send(event).await;
                    });
                }
            };
            tokio::join!(hotkeys, captures);
        });
    });
}

// Read the GNOME portal's own decision; never manufacture or write a grant.
async fn capture_permission() -> Result<bool> {
    let connection = ashpd::zbus::Connection::session().await?;
    let proxy = ashpd::zbus::Proxy::new(
        &connection,
        "org.freedesktop.impl.portal.PermissionStore",
        "/org/freedesktop/impl/portal/PermissionStore",
        "org.freedesktop.impl.portal.PermissionStore",
    )
    .await?;
    let (permissions, _): (
        std::collections::HashMap<String, Vec<String>>,
        ashpd::zvariant::OwnedValue,
    ) = proxy.call("Lookup", &("screenshot", "screenshot")).await?;
    Ok(permissions
        .get(crate::APP_ID)
        .is_some_and(|values| values.iter().any(|value| value == "yes")))
}

async fn capture(parent: Option<u32>) -> Result<image::RgbaImage> {
    let result = Screenshot::request()
        .identifier(parent.map(|xid| ashpd::WindowIdentifier::from_xid(xid.into())))
        .interactive(false)
        .modal(parent.is_some())
        .send()
        .await?
        .response()?;
    let path = result
        .uri()
        .to_file_path()
        .map_err(|_| anyhow::anyhow!("Portal 未返回本地 file URI"))?;
    tokio::task::spawn_blocking(move || {
        image::open(&path)
            .map(|image| image.to_rgba8())
            .with_context(|| format!("读取 Portal 截图 {}", path.display()))
    })
    .await?
}

fn describe(shortcuts: &[Shortcut]) -> String {
    shortcuts
        .iter()
        .map(|s| {
            format!(
                "{}：{}",
                if s.id() == "capture" {
                    "截图"
                } else {
                    "置顶"
                },
                s.trigger_description()
            )
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

async fn shortcuts(config: Config, configure: Receiver<()>, events: Sender<Event>) {
    if let Err(e) = shortcut_session(config, configure, events.clone()).await {
        let _ = events
            .send(Event::Status(format!(
                "全局快捷键不可用：{e:#}；仍可使用 --capture / --pin"
            )))
            .await;
    }
}

async fn shortcut_session(
    config: Config,
    configure: Receiver<()>,
    events: Sender<Event>,
) -> Result<()> {
    let portal = GlobalShortcuts::new().await?;
    let session = portal.create_session().await?;
    let mut activated = portal.receive_activated().await?;
    let mut changed = portal.receive_shortcuts_changed().await?;
    // ashpd 0.12 keeps Session::path private, but its public Serialize contract is an object path.
    let session_path = serde_json::to_value(&session)?;
    let shortcuts = [
        NewShortcut::new("capture", "FreePinPaste · 截图")
            .preferred_trigger(config.capture_shortcut.as_str()),
        NewShortcut::new("pin", "FreePinPaste · 置顶选区或剪贴板图片")
            .preferred_trigger(config.pin_shortcut.as_str()),
    ];
    let response = portal
        .bind_shortcuts(&session, &shortcuts, None)
        .await?
        .response()?;
    events
        .send(Event::Status(describe(response.shortcuts())))
        .await?;
    loop {
        tokio::select! {
            Ok(()) = configure.recv() => {
                if let Err(e) = portal.configure_shortcuts(&session, None, None).await {
                    events.send(Event::Status(format!("打开系统快捷键设置失败：{e}"))).await?;
                }
            }
            Some(event) = activated.next() => {
                if Some(event.session_handle().as_str()) != session_path.as_str() { continue; }
                let command = match event.shortcut_id() { "capture" => Command::Capture, "pin" => Command::Pin, _ => continue };
                events.send(Event::Command(command)).await?;
            }
            Some(event) = changed.next() => {
                if Some(event.session_handle().as_str()) == session_path.as_str() {
                    events.send(Event::Status(describe(event.shortcuts()))).await?;
                }
            }
        }
    }
}

pub async fn save_file(
    bytes: Vec<u8>,
    directory: std::path::PathBuf,
) -> Result<Option<std::path::PathBuf>> {
    use ashpd::desktop::{
        ResponseError,
        file_chooser::{FileFilter, SelectedFiles},
    };
    let request = SelectedFiles::save_file()
        .title("保存截图")
        .current_name(
            format!(
                "FreePinPaste-{}.png",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            )
            .as_str(),
        )
        .current_folder(&directory)?
        .filter(FileFilter::new("PNG 图片").glob("*.png"))
        .send()
        .await?;
    let response = match request.response() {
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => return Ok(None),
        result => result?,
    };
    let uri = response.uris().first().context("未选择保存文件")?;
    let path = uri
        .to_file_path()
        .map_err(|_| anyhow::anyhow!("请选择本地文件"))?;
    // Do not alter the confirmed filename: appending .png could overwrite a
    // DIFFERENT existing file that the system dialog never asked to replace.
    crate::config::atomic_write(&path, &bytes)?;
    Ok(Some(path))
}
