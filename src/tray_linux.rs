use crate::service::{Command, Event};
use async_channel::Sender;
use ksni::{MenuItem, Tray, blocking::TrayMethods, menu::StandardItem};

struct AppTray {
    events: Sender<Event>,
}

impl Tray for AppTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        crate::APP_ID.into()
    }

    fn title(&self) -> String {
        "FreePinPaste".into()
    }

    fn icon_name(&self) -> String {
        format!("{}-symbolic", crate::APP_ID)
    }

    fn icon_theme_path(&self) -> String {
        crate::branding::tray_theme_path()
            .to_string_lossy()
            .into_owned()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        // Hosts without symbolic icon/theme support can use the embedded logo.
        let image =
            image::load_from_memory(crate::branding::LOGO).expect("embedded logo is valid PNG");
        [32, 64]
            .into_iter()
            .map(|size| {
                let rgba = image
                    .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                    .to_rgba8();
                ksni::Icon {
                    width: size as i32,
                    height: size as i32,
                    data: rgba
                        .pixels()
                        .flat_map(|p| [p[3], p[0], p[1], p[2]])
                        .collect(),
                }
            })
            .collect()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: self.icon_name(),
            title: self.title(),
            description: "截屏、标注与贴图".into(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        [
            ("截屏", Command::Capture),
            ("截屏并自动复制", Command::CaptureCopy),
            ("设置", Command::Settings),
            ("重新启动", Command::Restart),
            ("退出", Command::Quit),
        ]
        .into_iter()
        .map(|(label, command)| {
            StandardItem {
                label: label.into(),
                activate: Box::new(move |tray: &mut Self| {
                    let _ = tray.events.try_send(Event::Command(command.clone()));
                }),
                ..Default::default()
            }
            .into()
        })
        .collect()
    }

    fn watcher_offline(&self, reason: ksni::OfflineReason) -> bool {
        eprintln!("FreePinPaste: 托盘宿主不可用（{reason:?}）；仍可从启动器打开设置");
        // Login autostart can precede GNOME's tray extension. Keep watching so
        // that the icon appears when the host starts or is enabled again.
        true
    }
}

pub fn start(events: Sender<Event>) {
    std::thread::spawn(move || {
        if let Err(error) = (AppTray {
            events: events.clone(),
        })
        .assume_sni_available(true)
        .spawn()
        {
            let _ = events.try_send(Event::Status(format!(
                "托盘启动失败：{error}；仍可从启动器打开设置"
            )));
        }
        // ksni's service outlives its handle; no dynamic menu updates are needed.
        // The D-Bus connection is closed on process exit (including restart).
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_order_and_commands() {
        let (events, receiver) = async_channel::unbounded();
        let mut tray = AppTray { events };
        let menu = tray.menu();
        assert_eq!(menu.len(), 5);
        for (item, (label, command)) in menu.into_iter().zip([
            ("截屏", "Capture"),
            ("截屏并自动复制", "CaptureCopy"),
            ("设置", "Settings"),
            ("重新启动", "Restart"),
            ("退出", "Quit"),
        ]) {
            let MenuItem::Standard(item) = item else {
                panic!("expected a standard menu item");
            };
            assert_eq!(item.label, label);
            (item.activate)(&mut tray);
            let Event::Command(actual) = receiver.try_recv().unwrap() else {
                panic!("expected a command");
            };
            // The same commands can also cross the single-instance socket.
            let json = serde_json::to_string(&actual).unwrap();
            assert_eq!(json, format!("\"{command}\""));
            assert!(serde_json::from_str::<Command>(&json).is_ok());
        }
    }
}
