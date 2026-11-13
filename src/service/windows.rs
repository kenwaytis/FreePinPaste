use super::{Command, Event};
use anyhow::{Context, Result, bail};
use async_channel::Sender;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use windows::{
    Win32::{
        Foundation::*,
        Security::{Authorization::*, *},
        Storage::FileSystem::*,
        System::{Pipes::*, RemoteDesktop::ProcessIdToSessionId, Threading::*},
    },
    core::{HSTRING, PWSTR},
};

struct Handle(HANDLE);
// Owned kernel handles may be transferred between threads; each is closed once.
unsafe impl Send for Handle {}
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
struct LocalAllocation(*mut std::ffi::c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0)));
        }
    }
}

pub struct Instance {
    _mutex: Handle,
    stop: Arc<AtomicBool>,
    listener: Option<JoinHandle<()>>,
}
impl Drop for Instance {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(listener) = self.listener.take() {
            let _ = listener.join();
        }
        unsafe {
            let _ = ReleaseMutex(self._mutex.0);
        }
    }
}
fn identity() -> Result<(String, u32)> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let token = Handle(token);
        let mut needed = 0;
        let _ = GetTokenInformation(token.0, TokenUser, None, 0, &mut needed);
        // Allocate pointer-aligned storage for TOKEN_USER and its SID.
        let mut buffer = vec![0usize; (needed as usize).div_ceil(std::mem::size_of::<usize>())];
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )?;
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut text)?;
        let _text = LocalAllocation(text.0.cast());
        let mut session = 0;
        ProcessIdToSessionId(GetCurrentProcessId(), &mut session)?;
        Ok((text.to_string()?, session))
    }
}
fn send(name: &HSTRING, command: &Command) -> Result<()> {
    let data = serde_json::to_vec(command)?;
    if data.len() > 65536 {
        bail!("单实例命令过长");
    }
    let mut client = None;
    // The winner owns the mutex before publishing the pipe.
    for _ in 0..50 {
        match unsafe {
            CreateFileW(
                name,
                GENERIC_READ.0 | GENERIC_WRITE.0,
                FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        } {
            Ok(handle) => {
                client = Some(Handle(handle));
                break;
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let client = client.context("已有实例正在启动或退出，请稍后重试")?;
    unsafe {
        WriteFile(client.0, Some(&data), None, None)?;
        // Acknowledgement keeps the pipe alive until the server consumes the command.
        let mut ack = [0u8; 1];
        ReadFile(client.0, Some(&mut ack), None, None)?;
    }
    Ok(())
}
pub fn claim(command: &Command, events: Sender<Event>) -> Result<Option<Instance>> {
    let (sid, session) = identity()?;
    if session == 0 {
        bail!("请在已登录的 Windows 桌面启动 FreePinPaste，不能在 SSH Session 0 启动 GUI");
    }
    let name = format!("FreePinPaste.{sid}.{session}");
    let pipe_name = HSTRING::from(format!(r"\\.\pipe\{name}"));
    unsafe {
        // Restrict control to this user and SYSTEM, and reject network clients.
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(format!("D:P(A;;GA;;;SY)(A;;GA;;;{sid})")),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )?;
        let _descriptor = LocalAllocation(descriptor.0);
        let security = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: windows::core::BOOL(0),
        };
        let mutex = Handle(CreateMutexW(
            Some(&security),
            true,
            &HSTRING::from(format!(r"Local\{name}")),
        )?);
        if GetLastError() == ERROR_ALREADY_EXISTS {
            send(&pipe_name, command)?;
            return Ok(None);
        }
        let pipe = CreateNamedPipeW(
            &pipe_name,
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            65536,
            65536,
            0,
            Some(&security),
        );
        if pipe.is_invalid() {
            return Err(windows::core::Error::from_win32()).context("创建本地控制管道失败");
        }
        let pipe = Handle(pipe);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let listener = std::thread::spawn(move || {
            let pipe = pipe; // Keep the owned handle, rather than capturing its raw field.
            let mut connected = false;
            let mut buffer = [0u8; 65536];
            while !stopped.load(Ordering::Acquire) {
                if !connected {
                    connected = ConnectNamedPipe(pipe.0, None).is_ok()
                        || GetLastError() == ERROR_PIPE_CONNECTED;
                }
                if connected {
                    let mut available = 0;
                    if PeekNamedPipe(pipe.0, None, 0, None, Some(&mut available), None).is_err() {
                        let _ = DisconnectNamedPipe(pipe.0);
                        connected = false;
                    } else if available > 0 {
                        let mut read = 0;
                        if ReadFile(pipe.0, Some(&mut buffer), Some(&mut read), None).is_ok()
                            && let Ok(command) = serde_json::from_slice(&buffer[..read as usize])
                        {
                            let _ = events.try_send(Event::Command(command));
                        }
                        let _ = WriteFile(pipe.0, Some(&[1]), None, None);
                        // FlushFileBuffers would block waiting for a client; instead keep
                        // the connected pipe until the client consumes its acknowledgement.
                        while !stopped.load(Ordering::Acquire) {
                            if PeekNamedPipe(pipe.0, None, 0, None, None, None).is_err() {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        let _ = DisconnectNamedPipe(pipe.0);
                        connected = false;
                    }
                }
                std::thread::sleep(Duration::from_millis(12));
            }
        });
        Ok(Some(Instance {
            _mutex: mutex,
            stop,
            listener: Some(listener),
        }))
    }
}
