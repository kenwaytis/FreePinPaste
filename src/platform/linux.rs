//! X11 is used only for OUR windows and display geometry. Never for capturing the desktop.
use super::Desktop;
use anyhow::{Context, Result};
use gpui::Window;
use x11rb::{
    connection::Connection,
    protocol::{xinerama, xproto::*},
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

pub struct Platform {
    connection: RustConnection,
    root: u32,
}

impl Platform {
    pub fn connect() -> Result<Self> {
        let (connection, screen) =
            x11rb::connect(None).context("无法连接 XWayland；需要 DISPLAY")?;
        let root = connection.setup().roots[screen].root;
        Ok(Self { connection, root })
    }
    /// GPUI 0.2.2's X11 HasWindowHandle implementation panics. Identify only our
    /// own freshly created client by PID + a unique temporary WM_NAME instead.
    /// This works before Mutter has added it to _NET_CLIENT_LIST as well.
    pub fn bind_window(&self, window: &mut Window, title: &str) -> Result<u32> {
        let marker = format!(
            "freepinpaste-{}-{}",
            std::process::id(),
            window.window_handle().window_id().as_u64()
        );
        window.set_window_title(&marker);
        let pid_atom = self.atom("_NET_WM_PID")?;
        let mut pending = vec![(self.root, 0)];
        let mut found = None;
        while let Some((parent, depth)) = pending.pop() {
            let Ok(tree) = self.connection.query_tree(parent)?.reply() else {
                continue;
            };
            for child in tree.children {
                let Ok(pid) = self
                    .connection
                    .get_property(false, child, pid_atom, AtomEnum::CARDINAL, 0, 1)?
                    .reply()
                else {
                    continue;
                };
                if pid.value32().and_then(|mut values| values.next()) == Some(std::process::id()) {
                    let name = self
                        .connection
                        .get_property(false, child, AtomEnum::WM_NAME, AtomEnum::ANY, 0, 256)?
                        .reply()?;
                    if name.value == marker.as_bytes() {
                        found = Some(child);
                        break;
                    }
                }
                if depth < 2 {
                    pending.push((child, depth + 1));
                }
            }
            if found.is_some() {
                break;
            }
        }
        window.set_window_title(title);
        found.context("无法定位本进程的 GPUI X11 窗口")
    }
    pub fn desktop(&self) -> Result<Desktop> {
        let g = self.connection.get_geometry(self.root)?.reply()?;
        Ok(Desktop {
            x: 0,
            y: 0,
            width: g.width.into(),
            height: g.height.into(),
        })
    }
    pub fn monitors(&self) -> Result<Vec<Desktop>> {
        Ok(xinerama::query_screens(&self.connection)?
            .reply()?
            .screen_info
            .into_iter()
            .map(|s| Desktop {
                x: s.x_org.into(),
                y: s.y_org.into(),
                width: s.width.into(),
                height: s.height.into(),
            })
            .collect())
    }
    pub fn pointer(&self) -> Result<(i32, i32)> {
        let p = self.connection.query_pointer(self.root)?.reply()?;
        Ok((p.root_x.into(), p.root_y.into()))
    }
    fn atom(&self, name: &str) -> Result<u32> {
        Ok(self
            .connection
            .intern_atom(false, name.as_bytes())?
            .reply()?
            .atom)
    }
    fn message(&self, window: u32, atom: &str, data: [u32; 5]) -> Result<()> {
        self.connection
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                ClientMessageEvent::new(32, window, self.atom(atom)?, data),
            )?
            .check()?;
        self.connection.flush()?;
        Ok(())
    }
    pub fn above(&self, window: u32, enabled: bool) -> Result<()> {
        self.message(
            window,
            "_NET_WM_STATE",
            [
                if enabled { 1 } else { 0 },
                self.atom("_NET_WM_STATE_ABOVE")?,
                0,
                1,
                0,
            ],
        )
    }
    pub fn borderless(&self, window: u32) -> Result<()> {
        self.connection
            .change_property32(
                PropMode::REPLACE,
                window,
                self.atom("_MOTIF_WM_HINTS")?,
                self.atom("_MOTIF_WM_HINTS")?,
                &[2, 0, 0, 0, 0],
            )?
            .check()?;
        self.connection.flush()?;
        Ok(())
    }
    pub fn overlay(&self, window: u32) -> Result<()> {
        self.borderless(window)?;
        // EWMH multi-monitor fullscreen: indices are from Xinerama, NOT connector order.
        let screens = xinerama::query_screens(&self.connection)?
            .reply()?
            .screen_info;
        if !screens.is_empty() {
            let top = screens
                .iter()
                .enumerate()
                .min_by_key(|(_, s)| s.y_org)
                .unwrap()
                .0;
            let bottom = screens
                .iter()
                .enumerate()
                .max_by_key(|(_, s)| i32::from(s.y_org) + i32::from(s.height))
                .unwrap()
                .0;
            let left = screens
                .iter()
                .enumerate()
                .min_by_key(|(_, s)| s.x_org)
                .unwrap()
                .0;
            let right = screens
                .iter()
                .enumerate()
                .max_by_key(|(_, s)| i32::from(s.x_org) + i32::from(s.width))
                .unwrap()
                .0;
            self.message(
                window,
                "_NET_WM_FULLSCREEN_MONITORS",
                [top as u32, bottom as u32, left as u32, right as u32, 1],
            )?;
        }
        self.message(
            window,
            "_NET_WM_STATE",
            [1, self.atom("_NET_WM_STATE_FULLSCREEN")?, 0, 1, 0],
        )?;
        self.above(window, true)?;
        self.message(
            window,
            "_NET_ACTIVE_WINDOW",
            [1, x11rb::CURRENT_TIME, 0, 0, 0],
        )
    }
    pub fn move_to(&self, window: u32, x: i32, y: i32) -> Result<()> {
        self.message(
            window,
            "_NET_MOVERESIZE_WINDOW",
            [
                10 | (1 << 8) | (1 << 9) | (1 << 12),
                x as u32,
                y as u32,
                0,
                0,
            ],
        )
    }
    pub fn resize_physical(&self, window: &mut Window, width: f32, height: f32) -> Result<()> {
        window.resize(gpui::size(
            gpui::px(width / window.scale_factor()),
            gpui::px(height / window.scale_factor()),
        ));
        Ok(())
    }
    pub fn start_move(&self, window: &Window) -> Result<()> {
        window.start_window_move();
        Ok(())
    }
    pub fn hide_keeper(&self, window: u32) -> Result<()> {
        self.message(
            window,
            "_NET_WM_STATE",
            [
                1,
                self.atom("_NET_WM_STATE_SKIP_TASKBAR")?,
                self.atom("_NET_WM_STATE_SKIP_PAGER")?,
                1,
                0,
            ],
        )?;
        // Use the WM's ordered transition, not an early UnmapWindow that can race
        // Mutter's initial MapRequest and leave a visible 1x1 service window.
        self.message(window, "WM_CHANGE_STATE", [3, 0, 0, 0, 0])
    }
    pub fn show(&self, window: u32, show: bool) -> Result<()> {
        if show {
            self.connection.map_window(window)?.check()?;
        } else {
            self.connection.unmap_window(window)?.check()?;
        }
        self.connection.flush()?;
        Ok(())
    }
}
