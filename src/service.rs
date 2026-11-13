use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
pub use linux::claim;
#[cfg(target_os = "windows")]
pub use windows::claim;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    Capture,
    CaptureCopy,
    AuthorizeCapture,
    Pin,
    Settings,
    Background,
    Restart,
    Quit,
    Edit(PathBuf),
}

pub enum Event {
    Command(Command),
    Status(String),
    Captured(Result<image::RgbaImage, String>),
    CapturePermission(bool),
    CaptureAuthorized(Result<(), String>),
}
