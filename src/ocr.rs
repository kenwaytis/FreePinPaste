use anyhow::{Context, Result, bail, ensure};
use image::RgbaImage;
use serde::Deserialize;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

mod desktop;
pub use desktop::{DesktopOcr, OcrClient};

const DPI: &str = "250";
const TIMEOUT: Duration = Duration::from_secs(90);

#[cfg(windows)]
const BINARIES: [&str; 3] = [
    "bin/lit.exe",
    "bin/pdfium.dll",
    "models/ocria-nano/runtime/onnxruntime.dll",
];
#[cfg(not(windows))]
const BINARIES: [&str; 3] = [
    "bin/lit",
    "bin/libpdfium.so",
    "models/ocria-nano/runtime/libonnxruntime.so",
];
pub const DESCRIPTION: &str =
    "OCRia-nano-1 · CPU · 自动识别文字系统（含中日韩、拉丁及从右到左的文字）";

fn scratch_dir() -> std::io::Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("freepinpaste-ocr-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir()
}

pub struct Ocr {
    directory: PathBuf,
}
impl Ocr {
    fn open(directory: PathBuf) -> Result<Self> {
        for file in BINARIES.into_iter().chain([desktop::WORKER]).chain(
            include_str!("../packaging/models.sha256")
                .lines()
                .map(|line| line.split_whitespace().last().unwrap()),
        ) {
            ensure!(
                directory.join(file).is_file(),
                "OCRia-nano-1 运行时缺少 {}",
                directory.join(file).display()
            );
        }
        Ok(Self {
            directory: directory.canonicalize()?,
        })
    }

    pub fn discover() -> Result<Self> {
        if let Some(path) = std::env::var_os("FREEPINPASTE_OCR_DIR") {
            return Self::open(path.into());
        }
        let exe = std::env::current_exe()?;
        let parent = exe.parent().context("无法定位程序目录")?;
        let mut candidates = vec![parent.join("ocr"), parent.join("../share/freepinpaste/ocr")];
        // A packaged release must never silently pick up a developer's runtime.
        if cfg!(debug_assertions)
            || parent == Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release")
        {
            candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/ocria"));
        }
        let directory = candidates
            .into_iter()
            .find(|p| p.is_dir())
            .context("未找到 OCRia-nano-1；请完整解压发行包，或运行 scripts/build-ocr 脚本")?;
        Self::open(directory)
    }

    fn command(&self, executable: &str, scratch: &Path) -> Command {
        let mut command = Command::new(self.directory.join(executable));
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        command
            .env("LITEPARSE_MODEL_DIR", self.directory.join("models"))
            .env("PDFIUM_LIB_PATH", self.directory.join("bin"))
            .env("ORT_DYLIB_PATH", self.directory.join(BINARIES[2]))
            .env("ORT_DISABLE_TELEMETRY", "1")
            .env("TMPDIR", scratch)
            .env("TMP", scratch)
            .env("TEMP", scratch);
        command
    }

    pub fn recognize(&self, image: &RgbaImage) -> Result<String> {
        let temp = scratch_dir()?;
        let input = temp.path().join("input.png");
        image.save(&input)?;
        let output = temp.path().join("result.json");
        let log = temp.path().join("stderr");
        let mut child = self
            .command(BINARIES[0], temp.path())
            .args([
                "parse",
                "--ocr-backend",
                "ocria-nano",
                "--dpi",
                DPI,
                "--format",
                "json",
                "--quiet",
                "--output",
            ])
            .arg(&output)
            .arg(input)
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log)?)
            .spawn()
            .context("无法启动随包 LiteParse")?;
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if start.elapsed() > TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                bail!("OCR 超时（90 秒），请缩小选区");
            }
            std::thread::sleep(Duration::from_millis(40));
        };
        if !status.success() {
            bail!(
                "OCRia-nano-1 识别失败（{status}）：{}",
                String::from_utf8_lossy(&std::fs::read(log)?).trim()
            );
        }
        // CLI text output adds a synthetic page header. The editor wants only the screenshot text.
        #[derive(Deserialize)]
        struct Page {
            text: String,
        }
        #[derive(Deserialize)]
        struct Screenshot {
            pages: [Page; 1],
        }
        let Screenshot { pages: [page] } = serde_json::from_reader(std::fs::File::open(output)?)
            .context("LiteParse 未返回有效的单页 OCR 结果")?;
        Ok(page.text.trim().to_string())
    }
}
