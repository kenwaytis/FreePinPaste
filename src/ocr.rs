use anyhow::{Context, Result, bail};
use image::RgbaImage;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Ocr {
    pub directory: PathBuf,
}
impl Ocr {
    pub fn discover() -> Result<Self> {
        let exe = std::env::current_exe()?;
        let mut candidates = Vec::new();
        if let Some(path) = std::env::var_os("FREEPINPASTE_OCR_DIR") {
            candidates.push(PathBuf::from(path));
        }
        candidates.push(exe.parent().unwrap().join("ocr"));
        candidates.push(exe.parent().unwrap().join("../share/freepinpaste/ocr"));
        candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/ocr"));
        let directory = candidates.into_iter().find(|p| p.join("bin/tesseract").is_file())
            .context("未找到随包 OCR 引擎。开发环境请先运行 scripts/build-ocr.sh；发行包应包含 ocr/bin/tesseract")?;
        Ok(Self { directory })
    }
    pub fn languages(&self) -> Result<Vec<String>> {
        let mut languages = std::fs::read_dir(self.directory.join("tessdata"))?
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                (p.extension()?.to_str()? == "traineddata")
                    .then(|| p.file_stem().unwrap().to_string_lossy().into_owned())
            })
            .collect::<Vec<_>>();
        languages.sort();
        Ok(languages)
    }
    pub fn recognize(&self, image: &RgbaImage, languages: &str) -> Result<String> {
        validate_languages(languages)?;
        let available = self.languages()?;
        for language in languages.split('+') {
            if !available.iter().any(|s| s == language) {
                bail!("发行包缺少 OCR 模型：{language}");
            }
        }
        let temp = tempfile::tempdir()?;
        let input = temp.path().join("input.png");
        // Upscale small UI text; no network, GPU runtime or Python process is involved.
        if image.width() < 1800 && image.height() < 1200 {
            image::imageops::resize(
                image,
                image.width() * 2,
                image.height() * 2,
                image::imageops::FilterType::CatmullRom,
            )
            .save(&input)?;
        } else {
            image.save(&input)?;
        }
        let output_base = temp.path().join("result");
        let stderr = std::fs::File::create(temp.path().join("stderr"))?;
        let mut child = Command::new(self.directory.join("bin/tesseract"))
            .arg(input)
            .arg(&output_base)
            .args(["--tessdata-dir"])
            .arg(self.directory.join("tessdata"))
            .args(["-l", languages, "--oem", "1", "--psm", "11"])
            .env("OMP_THREAD_LIMIT", "4")
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .context("无法启动内置 Tesseract")?;
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if start.elapsed() > Duration::from_secs(90) {
                let _ = child.kill();
                let _ = child.wait();
                bail!("OCR 超时（90 秒），请缩小选区或减少同时识别的语言");
            }
            std::thread::sleep(Duration::from_millis(40));
        };
        if !status.success() {
            bail!(
                "OCR 失败：{}",
                std::fs::read_to_string(temp.path().join("stderr"))?
            );
        }
        Ok(std::fs::read_to_string(output_base.with_extension("txt"))?
            .trim()
            .to_string())
    }
}

pub fn validate_languages(languages: &str) -> Result<()> {
    if languages.is_empty()
        || !languages
            .split('+')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
    {
        bail!("OCR 语言使用模型名并以 + 分隔，例如 chi_sim+eng、jpn+eng");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_validation() {
        for value in ["chi_sim+eng", "jpn+kor", "ara+rus"] {
            assert!(validate_languages(value).is_ok());
        }
        for value in ["", "+eng", "eng++fra", "../eng", "eng;true"] {
            assert!(validate_languages(value).is_err());
        }
    }
}
