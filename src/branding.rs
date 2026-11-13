use anyhow::Result;
use directories::BaseDirs;
use std::path::{Path, PathBuf};

pub const LOGO: &[u8] = include_bytes!("../assets/logo.png");
const SYMBOLIC: &[u8] = include_bytes!("../assets/tray-symbolic.svg");

fn icon_root() -> PathBuf {
    BaseDirs::new().unwrap().data_dir().join("icons/hicolor")
}

pub fn tray_theme_path() -> PathBuf {
    icon_root().join("scalable/status")
}

pub fn install() -> Result<()> {
    install_into(&icon_root())
}

fn install_into(root: &Path) -> Result<()> {
    for (relative, bytes) in [
        (format!("512x512/apps/{}.png", crate::APP_ID), LOGO),
        (
            format!("scalable/status/{}-symbolic.svg", crate::APP_ID),
            SYMBOLIC,
        ),
    ] {
        let path = root.join(relative);
        if std::fs::read(&path).ok().as_deref() != Some(bytes) {
            crate::config::atomic_write(&path, bytes)?;
            // Icon theme caches use the theme directory mtime for invalidation.
            #[cfg(target_os = "linux")]
            std::fs::File::open(root)?.set_modified(std::time::SystemTime::now())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_embedded_icons() {
        let directory = tempfile::tempdir().unwrap();
        install_into(directory.path()).unwrap();
        install_into(directory.path()).unwrap();
        let png = directory
            .path()
            .join(format!("512x512/apps/{}.png", crate::APP_ID));
        let image = image::open(png).unwrap().to_rgba8();
        assert_eq!(image.dimensions(), (512, 512));
        assert_eq!(image.get_pixel(0, 0).0[3], 0);
        let svg = directory
            .path()
            .join(format!("scalable/status/{}-symbolic.svg", crate::APP_ID));
        assert_eq!(std::fs::read(svg).unwrap(), SYMBOLIC);
    }
}
