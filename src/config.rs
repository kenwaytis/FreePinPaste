use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub capture_shortcut: String,
    pub pin_shortcut: String,
    pub ocr_languages: String,
    pub font_family: String,
    pub save_directory: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            capture_shortcut: "F1".into(),
            pin_shortcut: "F3".into(),
            ocr_languages: "chi_sim+eng".into(),
            font_family: "sans-serif".into(),
            save_directory: BaseDirs::new().unwrap().home_dir().join("Pictures"),
        }
    }
}

pub fn config_path() -> PathBuf {
    BaseDirs::new()
        .unwrap()
        .config_dir()
        .join("freepinpaste/config.toml")
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                toml::from_str(&text).with_context(|| format!("配置文件无效：{}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save(&self) -> Result<()> {
        if self.capture_shortcut.trim().is_empty() || self.pin_shortcut.trim().is_empty() {
            bail!("快捷键不能为空");
        }
        if self
            .capture_shortcut
            .eq_ignore_ascii_case(&self.pin_shortcut)
        {
            bail!("截图和置顶快捷键不能相同");
        }
        crate::ocr::validate_languages(&self.ocr_languages)?;
        atomic_write(&config_path(), toml::to_string_pretty(self)?.as_bytes())
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().context("文件没有父目录")?;
    std::fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}

pub fn desktop_entry(executable: &Path, background: bool) -> Result<String> {
    let executable = executable.to_str().context("可执行文件路径不是 UTF-8")?;
    if executable.contains(['\n', '\r']) {
        bail!("可执行文件路径不能含换行");
    }
    // Desktop Entry quoting is not shell quoting. Escape reserved characters and field codes.
    let escaped = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    Ok(format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName=FreePinPaste\nComment=截图、标注、贴图与离线 OCR\nExec=\"{escaped}\" {}\nIcon={}\nTerminal=false\nCategories=Graphics;Utility;\nStartupWMClass={}\nX-GNOME-Autostart-enabled=true\n",
        if background {
            "--background"
        } else {
            "--settings"
        },
        crate::APP_ID,
        crate::APP_ID
    ))
}

pub fn autostart_path() -> PathBuf {
    BaseDirs::new()
        .unwrap()
        .config_dir()
        .join("autostart")
        .join(format!("{}.desktop", crate::APP_ID))
}

pub fn set_autostart(enabled: bool) -> Result<()> {
    write_autostart(&autostart_path(), &std::env::current_exe()?, enabled)
}

fn write_autostart(path: &Path, executable: &Path, enabled: bool) -> Result<()> {
    if enabled {
        atomic_write(path, desktop_entry(executable, true)?.as_bytes())
    } else {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

fn desktop_path() -> PathBuf {
    BaseDirs::new()
        .unwrap()
        .data_dir()
        .join("applications")
        .join(format!("{}.desktop", crate::APP_ID))
}

pub fn ensure_desktop_entry() -> Result<()> {
    crate::branding::install()?;
    let path = desktop_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            // Upgrade the old stock icon without changing the installed Exec path.
            let updated = text.replace(
                "\nIcon=camera-photo\n",
                &format!("\nIcon={}\n", crate::APP_ID),
            );
            if updated != text {
                atomic_write(&path, updated.as_bytes())?;
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => install_desktop_entry(),
        Err(e) => Err(e.into()),
    }
}

pub fn install_desktop_entry() -> Result<()> {
    crate::branding::install()?;
    atomic_write(
        &desktop_path(),
        desktop_entry(&std::env::current_exe()?, false)?.as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_roundtrip() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.capture_shortcut, "F1");
        assert_eq!(c.pin_shortcut, "F3");
        assert_eq!(
            toml::from_str::<Config>(&toml::to_string(&c).unwrap())
                .unwrap()
                .ocr_languages,
            "chi_sim+eng"
        );
    }
    #[test]
    fn autostart_enable_disable_and_atomic_write() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("autostart/app.desktop");
        write_autostart(&path, Path::new("/opt/FreePinPaste/freepinpaste"), true).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("--background"));
        assert!(text.contains("X-GNOME-Autostart-enabled=true"));
        assert!(text.contains(&format!("Icon={}\n", crate::APP_ID)));
        write_autostart(&path, Path::new("/opt/FreePinPaste/freepinpaste"), false).unwrap();
        assert!(!path.exists());
        write_autostart(&path, Path::new("/opt/FreePinPaste/freepinpaste"), false).unwrap();
    }
    #[test]
    fn desktop_exec_is_not_a_shell_command() {
        let text = desktop_entry(Path::new("/tmp/a b/$x%foo"), true).unwrap();
        assert!(text.contains("Exec=\"/tmp/a b/\\$x%%foo\" --background"));
        assert!(desktop_entry(Path::new("/tmp/a\nb"), true).is_err());
    }
}
