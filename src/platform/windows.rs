use super::{Desktop, WindowId};
use anyhow::{Context, Result, bail};
use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::*,
        System::{
            LibraryLoader::GetModuleHandleW, RemoteDesktop::ProcessIdToSessionId,
            Threading::GetCurrentProcessId,
        },
        UI::{
            Input::KeyboardAndMouse::ReleaseCapture,
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
    core::{BOOL, PCWSTR},
};

pub struct Platform;
fn hwnd(id: WindowId) -> HWND {
    HWND(id as *mut _)
}

impl Platform {
    pub fn connect() -> Result<Self> {
        let mut session = 0;
        unsafe {
            ProcessIdToSessionId(GetCurrentProcessId(), &mut session)?;
        }
        if session == 0 {
            bail!("截图程序必须在已登录的交互桌面运行；SSH 的 Session 0 仅用于构建和命令行 OCR");
        }
        Ok(Self)
    }
    pub fn bind_window(&self, window: &mut Window, title: &str) -> Result<WindowId> {
        window.set_window_title(title);
        match HasWindowHandle::window_handle(window)?.as_raw() {
            RawWindowHandle::Win32(handle) => {
                unsafe {
                    let icon = LoadIconW(
                        Some(GetModuleHandleW(None)?.into()),
                        PCWSTR(std::ptr::without_provenance(1)),
                    )?;
                    for kind in [ICON_BIG, ICON_SMALL] {
                        SendMessageW(
                            hwnd(handle.hwnd.get()),
                            WM_SETICON,
                            Some(WPARAM(kind as usize)),
                            Some(LPARAM(icon.0 as isize)),
                        );
                    }
                }
                Ok(handle.hwnd.get())
            }
            _ => bail!("GPUI 未返回 Windows 原生窗口"),
        }
    }
    pub fn desktop(&self) -> Result<Desktop> {
        desktop()
    }
    pub fn monitors(&self) -> Result<Vec<Desktop>> {
        unsafe extern "system" fn collect(
            _: HMONITOR,
            _: HDC,
            rect: *mut RECT,
            data: LPARAM,
        ) -> BOOL {
            // EnumDisplayMonitors invokes this synchronously with valid pointers.
            unsafe {
                let r = *rect;
                (&mut *(data.0 as *mut Vec<Desktop>)).push(Desktop {
                    x: r.left,
                    y: r.top,
                    width: (r.right - r.left) as u32,
                    height: (r.bottom - r.top) as u32,
                });
            }
            BOOL(1)
        }
        let mut monitors = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(collect),
                LPARAM(&mut monitors as *mut _ as isize),
            )
            .ok()?;
        }
        Ok(monitors)
    }
    pub fn pointer(&self) -> Result<(i32, i32)> {
        let mut point = POINT::default();
        unsafe {
            GetCursorPos(&mut point)?;
        }
        Ok((point.x, point.y))
    }
    pub fn borderless(&self, id: WindowId) -> Result<()> {
        unsafe {
            // GPUI 0.2.2 keeps Windows 10 resize insets even without a titlebar.
            // Give these image windows a true full-size client area. WS_POPUP
            // also avoids the DPI-scaled minimum size of an overlapped window.
            SetWindowSubclass(hwnd(id), Some(borderless_proc), 1, 0).ok()?;
            let style = GetWindowLongW(hwnd(id), GWL_STYLE) as u32;
            SetWindowLongW(
                hwnd(id),
                GWL_STYLE,
                ((style | WS_POPUP.0)
                    & !(WS_CAPTION.0
                        | WS_THICKFRAME.0
                        | WS_SYSMENU.0
                        | WS_MINIMIZEBOX.0
                        | WS_MAXIMIZEBOX.0)) as i32,
            );
            let ex = GetWindowLongW(hwnd(id), GWL_EXSTYLE) as u32;
            SetWindowLongW(
                hwnd(id),
                GWL_EXSTYLE,
                ((ex & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0) as i32,
            );
            SetWindowPos(
                hwnd(id),
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }
    pub fn above(&self, id: WindowId, enabled: bool) -> Result<()> {
        unsafe {
            SetWindowPos(
                hwnd(id),
                Some(if enabled {
                    HWND_TOPMOST
                } else {
                    HWND_NOTOPMOST
                }),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }
    pub fn overlay(&self, id: WindowId) -> Result<()> {
        self.borderless(id)?;
        unsafe {
            SetWindowSubclass(hwnd(id), Some(borderless_proc), 1, 1).ok()?;
        }
        let d = desktop()?;
        unsafe {
            SetWindowPos(
                hwnd(id),
                Some(HWND_TOPMOST),
                d.x,
                d.y,
                d.width as i32,
                d.height as i32,
                SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            )?;
            let _ = SetForegroundWindow(hwnd(id));
        }
        Ok(())
    }
    pub fn move_to(&self, id: WindowId, x: i32, y: i32) -> Result<()> {
        unsafe {
            SetWindowPos(
                hwnd(id),
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }
    pub fn resize_physical(&self, window: &mut Window, width: f32, height: f32) -> Result<()> {
        let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window)?.as_raw()
        else {
            bail!("无效 Windows 窗口");
        };
        unsafe {
            SetWindowPos(
                hwnd(handle.hwnd.get()),
                None,
                0,
                0,
                width.round() as i32,
                height.round() as i32,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }
    pub fn start_move(&self, window: &Window) -> Result<()> {
        let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window)?.as_raw()
        else {
            bail!("无效 Windows 窗口");
        };
        unsafe {
            let _ = ReleaseCapture();
            PostMessageW(
                Some(hwnd(handle.hwnd.get())),
                WM_SYSCOMMAND,
                WPARAM((SC_MOVE | 2) as usize),
                LPARAM(0),
            )?;
        }
        Ok(())
    }
    pub fn hide_keeper(&self, id: WindowId) -> Result<()> {
        self.borderless(id)?;
        self.show(id, false)
    }
    pub fn show(&self, id: WindowId, show: bool) -> Result<()> {
        unsafe {
            let _ = ShowWindow(hwnd(id), if show { SW_SHOW } else { SW_HIDE });
        }
        Ok(())
    }
}

unsafe extern "system" fn borderless_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    id: usize,
    overlay: usize,
) -> LRESULT {
    unsafe {
        match message {
            WM_NCCALCSIZE => return LRESULT(0),
            WM_NCHITTEST => return LRESULT(HTCLIENT as isize),
            WM_GETDPISCALEDSIZE if overlay == 0 => {
                // Pins are physical-pixel images, not DPI-scaled application
                // windows. Tell the native move loop before WM_DPICHANGED so
                // its suggested rectangle preserves both size and drag anchor.
                let mut rect = RECT::default();
                if GetWindowRect(hwnd, &mut rect).is_ok() {
                    *(lparam.0 as *mut SIZE) = SIZE {
                        cx: rect.right - rect.left,
                        cy: rect.bottom - rect.top,
                    };
                    return LRESULT(1);
                }
            }
            WM_DPICHANGED if overlay == 1 => {
                let result = DefSubclassProc(hwnd, message, wparam, lparam);
                let _ = PostMessageW(Some(hwnd), WM_APP + 99, WPARAM(0), LPARAM(0));
                return result;
            }
            m if m == WM_APP + 99 => {
                if let Ok(d) = desktop() {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        d.x,
                        d.y,
                        d.width as i32,
                        d.height as i32,
                        SWP_NOACTIVATE,
                    );
                }
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(hwnd, Some(borderless_proc), id);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }
}

pub(crate) fn desktop() -> Result<Desktop> {
    let d = unsafe {
        Desktop {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            width: GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
            height: GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
        }
    };
    if d.width == 0 || d.height == 0 {
        bail!("当前桌面没有可捕获的显示器");
    }
    Ok(d)
}

/// Capture SDR virtual-desktop physical pixels. GDI handles differing monitor DPIs
/// when the process manifest is PerMonitorV2; blank gaps remain black.
pub(crate) fn capture() -> Result<image::RgbaImage> {
    let d = desktop()?;
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            bail!("无法获取桌面 DC");
        }
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, d.width as i32, d.height as i32);
        if memory.is_invalid() || bitmap.is_invalid() {
            if !memory.is_invalid() {
                let _ = DeleteDC(memory);
            }
            if !bitmap.is_invalid() {
                let _ = DeleteObject(bitmap.into());
            }
            ReleaseDC(None, screen);
            bail!("无法分配桌面截图缓冲区");
        }
        let previous = SelectObject(memory, bitmap.into());
        let copied = BitBlt(
            memory,
            0,
            0,
            d.width as i32,
            d.height as i32,
            Some(screen),
            d.x,
            d.y,
            SRCCOPY | CAPTUREBLT,
        );
        SelectObject(memory, previous);
        let mut bytes = vec![0u8; d.width as usize * d.height as usize * 4];
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: d.width as i32,
                biHeight: -(d.height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let rows = if copied.is_ok() {
            GetDIBits(
                memory,
                bitmap,
                0,
                d.height,
                Some(bytes.as_mut_ptr().cast()),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        copied.context("读取桌面像素失败")?;
        if rows != d.height as i32 {
            bail!("桌面截图像素读取不完整");
        }
        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
        image::RgbaImage::from_raw(d.width, d.height, bytes).context("无效截图尺寸")
    }
}
