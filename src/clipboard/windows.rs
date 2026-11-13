use anyhow::Result;
use image::RgbaImage;
use std::{borrow::Cow, cell::RefCell};

/// CF_DIBV5 supports native Windows applications, including Paint and Office.
/// The OS owns the copied data, so it survives FreePinPaste exiting.
pub struct ImageClipboard(RefCell<arboard::Clipboard>);
impl ImageClipboard {
    pub fn new() -> Result<Self> {
        Ok(Self(RefCell::new(arboard::Clipboard::new()?)))
    }
    pub fn copy(&self, image: &RgbaImage) -> Result<()> {
        self.0.borrow_mut().set_image(arboard::ImageData {
            width: image.width() as usize,
            height: image.height() as usize,
            bytes: Cow::Borrowed(image.as_raw()),
        })?;
        Ok(())
    }
    pub fn read(&self) -> Option<RgbaImage> {
        let image = self.0.borrow_mut().get_image().ok()?;
        RgbaImage::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.into_owned(),
        )
    }
}
