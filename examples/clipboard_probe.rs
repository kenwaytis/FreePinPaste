//! Manual interop probe: cargo run --example clipboard_probe -- image.png
//! While it is alive, paste into a native Wayland app or run wl-paste -t image/png.
fn main() -> anyhow::Result<()> {
    let image = image::open(std::env::args().nth(1).expect("PNG path required"))?.to_rgba8();
    let clipboard = freepinpaste::clipboard::ImageClipboard::new()?;
    clipboard.copy(&image)?;
    println!("Serving image/png for 20 seconds");
    std::thread::sleep(std::time::Duration::from_secs(20));
    Ok(())
}
