use super::{Command, Event};
use anyhow::{Context, Result};
use async_channel::Sender;
use std::{
    fs::File,
    os::unix::{fs::PermissionsExt, net::UnixDatagram},
    path::PathBuf,
};

/// The lock outlives the listener; a crashed process leaves no live lock.
pub struct Instance {
    _lock: File,
    socket: PathBuf,
}
impl Drop for Instance {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
    }
}

pub fn claim(command: &Command, events: Sender<Event>) -> Result<Option<Instance>> {
    let base = std::env::var_os("XDG_RUNTIME_DIR").context("需要桌面会话的 XDG_RUNTIME_DIR")?;
    let directory = PathBuf::from(base).join("freepinpaste");
    std::fs::create_dir_all(&directory)?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    let socket_path = directory.join("control.sock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("instance.lock"))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            let socket = UnixDatagram::unbound()?;
            socket
                .connect(&socket_path)
                .context("已有实例正在启动，请稍后重试")?;
            socket.send(&serde_json::to_vec(command)?)?;
            return Ok(None);
        }
        Err(e) => return Err(e.into()),
    }
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    let socket = UnixDatagram::bind(&socket_path)?;
    std::thread::spawn(move || {
        let mut buffer = [0u8; 65536];
        while let Ok(n) = socket.recv(&mut buffer) {
            if let Ok(command) = serde_json::from_slice(&buffer[..n])
                && events.send_blocking(Event::Command(command)).is_err()
            {
                break;
            }
        }
    });
    Ok(Some(Instance {
        _lock: lock,
        socket: socket_path,
    }))
}
