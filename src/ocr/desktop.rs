use super::{DPI, Ocr, TIMEOUT};
use anyhow::{Context, Result, anyhow, ensure};
use async_channel::{Receiver, Sender};
use image::RgbaImage;
use std::{process::Stdio, sync::Arc, thread};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdout, Command},
    time::timeout,
};

pub(super) const WORKER: &str = if cfg!(windows) {
    "bin/freepinpaste-ocr-worker.exe"
} else {
    "bin/freepinpaste-ocr-worker"
};

type Reply = Sender<Result<String>>;
type Job = (Arc<RgbaImage>, Reply);

/// Owned by the desktop instance, not its individual windows. Drop before restarting.
pub struct DesktopOcr {
    stop: Sender<()>,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct OcrClient(Sender<Job>);

impl DesktopOcr {
    pub fn start() -> (Self, OcrClient) {
        let (tx, jobs) = async_channel::unbounded();
        let (stop, shutdown) = async_channel::bounded(1);
        let thread = thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let mut worker = Worker(None);
                tokio::select! {
                    biased;
                    _ = shutdown.recv() => (),
                    _ = worker.serve(jobs) => (),
                }
                worker.reset().await;
            });
        });
        (
            Self {
                stop,
                thread: Some(thread),
            },
            OcrClient(tx),
        )
    }
}

impl Drop for DesktopOcr {
    fn drop(&mut self) {
        self.stop.close();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl OcrClient {
    pub async fn recognize(&self, image: Arc<RgbaImage>) -> Result<String> {
        let (tx, rx) = async_channel::bounded(1);
        self.0
            .send((image, tx))
            .await
            .map_err(|_| anyhow!("OCR 服务已停止"))?;
        rx.recv().await.context("OCR 服务已停止")?
    }
}

struct Session {
    child: Child,
    stdout: Lines<BufReader<ChildStdout>>,
    scratch: tempfile::TempDir,
}
impl Session {
    fn spawn() -> Result<Self> {
        let ocr = Ocr::discover()?;
        let scratch = super::scratch_dir()?;
        let mut cmd = Command::from(ocr.command(WORKER, scratch.path()));
        let mut child = cmd
            .arg(DPI)
            .current_dir(scratch.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("无法启动常驻 OCR，请重新准备 OCR 运行时")?;
        let stdout = BufReader::new(child.stdout.take().unwrap()).lines();
        Ok(Self {
            child,
            stdout,
            scratch,
        })
    }

    async fn response(&mut self) -> Result<String> {
        let line = self.stdout.next_line().await?.context("OCR 子进程已退出")?;
        serde_json::from_str::<Result<String, String>>(&line)
            .context("无效的 OCR 响应")?
            .map_err(anyhow::Error::msg)
    }
}

struct Worker(Option<Session>);
impl Worker {
    async fn ready(&mut self) -> Result<&mut Session> {
        if let Some(s) = &mut self.0
            && s.child.try_wait()?.is_some()
        {
            self.0 = None;
        }
        if self.0.is_none() {
            let s = self.0.insert(Session::spawn()?);
            ensure!(s.response().await? == "ready", "OCR 预热失败");
        }
        Ok(self.0.as_mut().unwrap())
    }

    async fn recognize(&mut self, image: &RgbaImage) -> Result<String> {
        let session = self.ready().await?;
        let input = session.scratch.path().join("input.png");
        image.save(&input)?;
        session
            .child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"parse\n")
            .await?;
        let text = session.response().await?;
        std::fs::remove_file(input)?;
        Ok(text)
    }

    async fn reset(&mut self) {
        if let Some(session) = &mut self.0 {
            // Keep ownership through wait: shutdown can interrupt a request's reset.
            let _ = session.child.kill().await;
            self.0 = None;
        }
    }

    async fn serve(&mut self, jobs: Receiver<Job>) {
        if let Err(e) = timeout(TIMEOUT, self.ready())
            .await
            .context("OCR 预热超时")
            .and_then(|r| r)
        {
            eprintln!("FreePinPaste: OCR 预热失败：{e:#}");
            self.reset().await;
        }
        while let Ok((image, reply)) = jobs.recv().await {
            let result = timeout(TIMEOUT, self.recognize(&image))
                .await
                .context("OCR 超时（90 秒），请缩小选区")
                .and_then(|r| r);
            if result.is_err() {
                self.reset().await;
            }
            let _ = reply.try_send(result);
        }
    }
}
