use super::Request;
use crate::{
    config::Config,
    service::{Command, Event},
};
use anyhow::{Context, Result, bail};
use async_channel::{Receiver, Sender};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::{Com::*, LibraryLoader::GetModuleHandleW},
        UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
    core::{HSTRING, PCWSTR, w},
};

pub struct Service {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Hotkey {
    modifiers: HOT_KEY_MODIFIERS,
    key: u32,
}
fn parse_hotkey(text: &str) -> Result<Hotkey> {
    let mut modifiers = HOT_KEY_MODIFIERS(0);
    let mut key = None;
    for part in text.trim().split('+') {
        let part = part.trim().to_ascii_uppercase();
        let modifier = match part.as_str() {
            "CTRL" | "CONTROL" => Some(MOD_CONTROL),
            "ALT" => Some(MOD_ALT),
            "SHIFT" => Some(MOD_SHIFT),
            "WIN" | "SUPER" => Some(MOD_WIN),
            _ => None,
        };
        if let Some(m) = modifier {
            if modifiers.contains(m) {
                bail!("重复的快捷键修饰符：{part}");
            }
            modifiers |= m;
            continue;
        }
        if key.is_some() {
            bail!("快捷键只能包含一个主键：{text}");
        }
        key = Some(
            if let Some(f) = part
                .strip_prefix('F')
                .and_then(|n| n.parse::<u32>().ok())
                .filter(|n| (1..=24).contains(n))
            {
                VK_F1.0 as u32 + f - 1
            } else if part.len() == 1 && part.as_bytes()[0].is_ascii_alphanumeric() {
                part.as_bytes()[0] as u32
            } else {
                match part.as_str() {
                    "PRINTSCREEN" | "PRTSC" => VK_SNAPSHOT.0 as u32,
                    "SPACE" => VK_SPACE.0 as u32,
                    _ => bail!("不支持的快捷键：{text}；请使用 F1–F24、字母、数字或 PrintScreen"),
                }
            },
        );
    }
    Ok(Hotkey {
        modifiers,
        key: key.context("快捷键缺少主键")?,
    })
}
pub fn validate_shortcuts(config: &Config) -> Result<()> {
    let capture = parse_hotkey(&config.capture_shortcut)?;
    let pin = parse_hotkey(&config.pin_shortcut)?;
    if capture == pin {
        bail!("截图和置顶快捷键不能相同");
    }
    Ok(())
}

#[derive(Default)]
struct Hotkeys {
    active: Vec<(Hotkey, i32)>,
    next_id: i32,
}
impl Hotkeys {
    fn configure(&mut self, config: &Config) -> Result<()> {
        validate_shortcuts(config)?;
        let desired = [
            parse_hotkey(&config.capture_shortcut)?,
            parse_hotkey(&config.pin_shortcut)?,
        ];
        let mut registered = Vec::new();
        let mut next = Vec::new();
        for key in desired {
            let id = if let Some((_, id)) = self.active.iter().find(|(old, _)| *old == key) {
                *id
            } else {
                self.next_id += 1;
                let id = self.next_id;
                if let Err(error) =
                    unsafe { RegisterHotKey(None, id, key.modifiers | MOD_NOREPEAT, key.key) }
                {
                    for id in registered {
                        unsafe {
                            let _ = UnregisterHotKey(None, id);
                        }
                    }
                    return Err(error)
                        .context("快捷键已被其他程序占用或被系统保留；原有绑定保持不变");
                }
                registered.push(id);
                id
            };
            next.push((key, id));
        }
        for (_, id) in &self.active {
            if !next.iter().any(|(_, next_id)| next_id == id) {
                unsafe {
                    let _ = UnregisterHotKey(None, *id);
                }
            }
        }
        self.active = next;
        Ok(())
    }
    fn command(&self, id: i32) -> Option<Command> {
        self.active
            .iter()
            .position(|(_, active)| *active == id)
            .map(|i| {
                if i == 0 {
                    Command::Capture
                } else {
                    Command::Pin
                }
            })
    }
}
impl Drop for Hotkeys {
    fn drop(&mut self) {
        for (_, id) in &self.active {
            unsafe {
                let _ = UnregisterHotKey(None, *id);
            }
        }
    }
}

pub fn start(
    config: Config,
    requests: Receiver<Request>,
    events: Sender<Event>,
    enable_shortcuts: bool,
) -> Service {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let thread = std::thread::spawn(move || {
        if let Err(error) = run(config, requests, events.clone(), enable_shortcuts, stopped) {
            let _ = events.try_send(Event::Status(format!("Windows 桌面服务失败：{error:#}")));
        }
    });
    Service {
        stop,
        thread: Some(thread),
    }
}
fn shortcut_status(
    hotkeys: &mut Hotkeys,
    config: &Config,
    events: &Sender<Event>,
) -> Result<(), String> {
    let result = hotkeys.configure(config).map_err(|e| format!("{e:#}"));
    let text = match &result {
        Ok(()) => format!(
            "截图：{} · 置顶：{}",
            config.capture_shortcut, config.pin_shortcut
        ),
        Err(e) => format!("全局快捷键不可用：{e}；仍可使用托盘菜单"),
    };
    let _ = events.try_send(Event::Status(text));
    result
}
fn run(
    config: Config,
    requests: Receiver<Request>,
    events: Sender<Event>,
    enable_shortcuts: bool,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let mut state = Box::new(TrayState {
        events: events.clone(),
        taskbar_created: unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
    });
    let tray = TrayWindow::new(&mut state)?;
    let mut hotkeys = Hotkeys::default();
    if enable_shortcuts {
        let _ = shortcut_status(&mut hotkeys, &config, &events);
    }
    let _ = events.try_send(Event::CapturePermission(true));
    while !stop.load(Ordering::Acquire) {
        let mut message = MSG::default();
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_HOTKEY {
                    if let Some(command) = hotkeys.command(message.wParam.0 as i32) {
                        let _ = events.try_send(Event::Command(command));
                    }
                } else {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
        while let Ok(request) = requests.try_recv() {
            match request {
                Request::Capture => {
                    let events = events.clone();
                    std::thread::spawn(move || {
                        // Let the settings/tray menu disappear before reading the desktop.
                        std::thread::sleep(std::time::Duration::from_millis(150));
                        let result = crate::platform::capture().map_err(|e| format!("{e:#}"));
                        let _ = events.send_blocking(Event::Captured(result));
                    });
                }
                Request::AuthorizeCapture(_) => {
                    let _ = events.try_send(Event::CaptureAuthorized(Ok(())));
                }
                Request::UpdateShortcuts(config, reply) => {
                    let result = if enable_shortcuts {
                        shortcut_status(&mut hotkeys, &config, &events)
                    } else {
                        Ok(())
                    };
                    let _ = reply.try_send(result);
                }
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(12));
    }
    drop(tray); // Remove the notification icon before starting a replacement process.
    Ok(())
}

const TRAY_MESSAGE: u32 = WM_APP + 1;
struct TrayState {
    events: Sender<Event>,
    taskbar_created: u32,
}
struct TrayWindow(HWND);
impl TrayWindow {
    fn new(state: &mut TrayState) -> Result<Self> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(tray_proc),
                hInstance: instance.into(),
                lpszClassName: w!("FreePinPaste.Desktop"),
                ..Default::default()
            };
            if RegisterClassW(&class) == 0 {
                bail!("无法注册托盘窗口类");
            }
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                class.lpszClassName,
                w!("FreePinPaste.Desktop"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance.into()),
                Some(state as *mut _ as _),
            )?;
            let window = Self(hwnd);
            if !Shell_NotifyIconW(NIM_ADD, &tray_data(hwnd)).as_bool() {
                // Login startup can precede Explorer. Keep the message window
                // alive to receive TaskbarCreated; capture/hotkeys still work.
                let _ = state.events.try_send(Event::Status(
                    "通知区域尚未就绪，等待 Explorer 启动；仍可使用快捷键和命令行".into(),
                ));
            }
            Ok(window)
        }
    }
}
impl Drop for TrayWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &tray_data(self.0));
            let _ = DestroyWindow(self.0);
            let _ = UnregisterClassW(
                w!("FreePinPaste.Desktop"),
                Some(GetModuleHandleW(None).unwrap_or_default().into()),
            );
        }
    }
}
fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: TRAY_MESSAGE,
        ..Default::default()
    };
    unsafe {
        data.hIcon = LoadIconW(
            Some(GetModuleHandleW(None).unwrap_or_default().into()),
            PCWSTR(std::ptr::without_provenance(1)), // MAKEINTRESOURCEW(1), not a string pointer
        )
        .unwrap_or_default();
    }
    let tip: Vec<_> = "FreePinPaste · 截图、标注与贴图\0".encode_utf16().collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    data
}
unsafe extern "system" fn tray_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut TrayState;
        if let Some(state) = state.as_ref() {
            if message == state.taskbar_created {
                let _ = Shell_NotifyIconW(NIM_ADD, &tray_data(hwnd));
                return LRESULT(0);
            }
            if message == TRAY_MESSAGE
                && (lparam.0 as u32 == WM_RBUTTONUP || lparam.0 as u32 == WM_LBUTTONUP)
            {
                let _ = show_menu(hwnd, &state.events);
                return LRESULT(0);
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
fn show_menu(hwnd: HWND, events: &Sender<Event>) -> Result<()> {
    unsafe {
        let menu = CreatePopupMenu()?;
        let actions = [
            ("截屏", Command::Capture),
            ("截屏并自动复制", Command::CaptureCopy),
            ("设置", Command::Settings),
            ("重新启动", Command::Restart),
            ("退出", Command::Quit),
        ];
        for (i, (label, _)) in actions.iter().enumerate() {
            AppendMenuW(menu, MF_STRING, i + 1, &HSTRING::from(*label))?;
        }
        let mut pointer = windows::Win32::Foundation::POINT::default();
        GetCursorPos(&mut pointer)?;
        let _ = SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            pointer.x,
            pointer.y,
            None,
            hwnd,
            None,
        )
        .0;
        DestroyMenu(menu)?;
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        if selected > 0
            && let Some((_, command)) = actions.get(selected as usize - 1)
        {
            let _ = events.try_send(Event::Command(command.clone()));
        }
    }
    Ok(())
}

pub async fn save_file(bytes: Vec<u8>, directory: PathBuf) -> Result<Option<PathBuf>> {
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe {
                CoUninitialize();
            }
        }
    }
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let _com = Com;
        let dialog: IFileSaveDialog =
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
        dialog.SetOptions(
            dialog.GetOptions()? | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_OVERWRITEPROMPT,
        )?;
        dialog.SetTitle(w!("保存截图"))?;
        dialog.SetFileTypes(&[Common::COMDLG_FILTERSPEC {
            pszName: w!("PNG 图片"),
            pszSpec: w!("*.png"),
        }])?;
        dialog.SetDefaultExtension(w!("png"))?;
        dialog.SetFileName(&HSTRING::from(format!(
            "FreePinPaste-{}.png",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        )))?;
        let folder: IShellItem =
            SHCreateItemFromParsingName(&HSTRING::from(directory.as_os_str()), None)?;
        dialog.SetFolder(&folder)?;
        if let Err(error) = dialog.Show(None) {
            if error.code().0 as u32 == 0x800704c7 {
                return Ok(None);
            }
            return Err(error.into());
        }
        let name = dialog.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
        let path = name.to_string();
        CoTaskMemFree(Some(name.0.cast()));
        let path = PathBuf::from(path?);
        crate::config::atomic_write(&path, &bytes)?;
        Ok(Some(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "RegisterHotKey requires an interactive window station; run via windows-smoke.py --hotkey-test-only"]
    fn hotkey_conflict_keeps_old_bindings_and_swapping_reuses_them() {
        // Claims uncommon Ctrl+Shift+F20..F23 bindings only for this test's lifetime.
        struct Reservation;
        impl Drop for Reservation {
            fn drop(&mut self) {
                unsafe {
                    let _ = UnregisterHotKey(None, 2500);
                }
            }
        }
        unsafe {
            RegisterHotKey(
                None,
                2500,
                MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT,
                VK_F22.0 as u32,
            )
            .unwrap();
        }
        let _reserved = Reservation;
        let mut hotkeys = Hotkeys::default();
        let mut config = Config {
            capture_shortcut: "Ctrl+Shift+F20".into(),
            pin_shortcut: "Ctrl+Shift+F21".into(),
            ..Default::default()
        };
        hotkeys.configure(&config).unwrap();
        let original = hotkeys.active.clone();
        config.capture_shortcut = "Ctrl+Shift+F23".into();
        config.pin_shortcut = "Ctrl+Shift+F22".into();
        assert!(hotkeys.configure(&config).is_err());
        assert_eq!(hotkeys.active, original);
        config.capture_shortcut = "Ctrl+Shift+F21".into();
        config.pin_shortcut = "Ctrl+Shift+F20".into();
        hotkeys.configure(&config).unwrap();
        assert_eq!(hotkeys.active, vec![original[1], original[0]]);
    }
    #[test]
    fn shortcuts_normalize_modifiers_and_reject_invalid_keys() {
        assert_eq!(
            parse_hotkey(" Ctrl + Shift + a ").unwrap(),
            parse_hotkey("shift+control+A").unwrap()
        );
        assert_eq!(parse_hotkey("F3").unwrap().key, VK_F3.0 as u32);
        for text in ["", "Ctrl", "F25", "Ctrl++A", "Ctrl+A+B", "Ctrl+Ctrl+A"] {
            assert!(parse_hotkey(text).is_err(), "{text}");
        }
        let config = Config {
            capture_shortcut: "Ctrl+A".into(),
            pin_shortcut: "control+a".into(),
            ..Default::default()
        };
        assert!(validate_shortcuts(&config).is_err());
    }
}
