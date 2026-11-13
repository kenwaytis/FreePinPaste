pub mod branding;
pub mod clipboard;
pub mod config;
pub mod desktop;
pub mod document;
pub mod ocr;
pub mod platform;
pub mod raster;
pub mod service;
#[cfg(target_os = "linux")]
#[path = "tray_linux.rs"]
pub mod tray;
pub mod ui;

pub const APP_ID: &str = "io.github.freepinpaste.FreePinPaste";
