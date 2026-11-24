//! Run after scripts/build-ocr.sh: cargo test --test ocr_smoke -- --ignored
use freepinpaste::{
    document::{Annotation, Document, Point, Style, Tool},
    ocr::Ocr,
    raster,
};
use image::{Rgba, RgbaImage};

#[test]
#[ignore = "requires the bundled OCR runtime and system CJK fonts"]
fn bundled_cpu_ocr_reads_chinese_and_english() {
    let ocr = Ocr::discover().unwrap();
    let mut doc = Document::new(RgbaImage::from_pixel(1000, 250, Rgba([255, 255, 255, 255])));
    let mut text = Annotation::new(
        Tool::Text,
        Point::new(40., 30.),
        Style {
            color: 0x121212,
            font_size: 48.,
            ..Default::default()
        },
    );
    text.text = "FreePinPaste 2026\n快捷截图 文字识别".into();
    doc.push(text);
    let image = raster::composite(&doc).unwrap();
    let result = ocr.recognize(&image).unwrap();
    eprintln!("OCR: {result}");
    assert!(result.contains("FreePinPaste"));
    assert!(result.contains("2026"));
    assert!(result.replace(' ', "").contains("文字识别"));
}
