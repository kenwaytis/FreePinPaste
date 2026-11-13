//! Desktop services: capture, global shortcuts and native file dialogs.
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
pub use linux::{save_file, start};
#[cfg(target_os = "windows")]
pub use windows::{save_file, start, validate_shortcuts};

#[derive(Clone, Debug)]
pub enum Request {
    Capture,
    AuthorizeCapture(crate::platform::WindowId),
    ConfigureShortcuts,
    #[cfg(target_os = "windows")]
    UpdateShortcuts(
        crate::config::Config,
        async_channel::Sender<Result<(), String>>,
    ),
}
