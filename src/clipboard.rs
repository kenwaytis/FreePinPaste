//! GPUI 0.2.2's X11 writer calls set_text even for ClipboardEntry::Image.
//! Keep a real PNG selection owner alive for the lifetime of the application.
use anyhow::Result;
use image::RgbaImage;

pub struct ImageClipboard(x11_clipboard::Clipboard);
impl ImageClipboard {
    pub fn new() -> Result<Self> {
        Ok(Self(x11_clipboard::Clipboard::new()?))
    }
    pub fn copy(&self, image: &RgbaImage) -> Result<()> {
        let target = self.0.setter.get_atom("image/png")?;
        self.0.store(
            self.0.setter.atoms.clipboard,
            target,
            crate::raster::png(image)?,
        )?;
        Ok(())
    }
}
