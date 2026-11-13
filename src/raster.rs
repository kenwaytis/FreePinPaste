//! One CPU renderer for export, OCR and committed editor previews.
use crate::document::{Annotation, Document, Tool};
use anyhow::{Context, Result};
use image::RgbaImage;
use std::sync::{Arc, OnceLock};
use tiny_skia::{
    BlendMode, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

fn fonts() -> Arc<resvg::usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = resvg::usvg::fontdb::Database::new();
            db.load_system_fonts();
            // Match the system's sans-serif choice, including CJK glyphs.
            if let Ok(output) = std::process::Command::new("fc-match")
                .args(["sans-serif", "-f", "%{family}"])
                .output()
            {
                let name = String::from_utf8_lossy(&output.stdout);
                if let Some(name) = name.split(',').next().filter(|s| !s.is_empty()) {
                    db.set_sans_serif_family(name);
                }
            }
            Arc::new(db)
        })
        .clone()
}

pub fn warm_fonts() {
    let _ = fonts();
}

pub fn annotation_path(annotation: &Annotation) -> Option<tiny_skia::Path> {
    let a = annotation.points[0];
    let b = *annotation.points.last()?;
    let mut p = PathBuilder::new();
    match annotation.tool {
        Tool::Rectangle | Tool::Ellipse => {
            let r = annotation.bounds();
            let r = tiny_skia::Rect::from_xywh(r.x, r.y, r.w.max(0.1), r.h.max(0.1))?;
            if annotation.tool == Tool::Ellipse {
                p.push_oval(r);
            } else {
                p.push_rect(r);
            }
        }
        Tool::Arrow => {
            p.move_to(a.x, a.y);
            p.line_to(b.x, b.y);
            let angle = (b.y - a.y).atan2(b.x - a.x);
            let len = (annotation.style.width * 4.)
                .max(12.)
                .min((b.x - a.x).hypot(b.y - a.y) * 0.45);
            for delta in [-0.48f32, 0.48] {
                p.move_to(
                    b.x - len * (angle + delta).cos(),
                    b.y - len * (angle + delta).sin(),
                );
                p.line_to(b.x, b.y);
            }
        }
        Tool::Line | Tool::Pen | Tool::Eraser => {
            p.move_to(a.x, a.y);
            for point in annotation.points.iter().skip(1) {
                p.line_to(point.x, point.y);
            }
            if annotation.points.iter().all(|point| *point == a) {
                p.line_to(a.x + 0.01, a.y);
            }
        }
        _ => return None,
    }
    p.finish()
}

pub fn paint_annotation(layer: &mut Pixmap, a: &Annotation) -> Result<()> {
    if a.tool == Tool::Text {
        return paint_text(layer, a);
    }
    let Some(path) = annotation_path(a) else {
        return Ok(());
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (a.style.color >> 16) as u8,
        (a.style.color >> 8) as u8,
        a.style.color as u8,
        255,
    );
    paint.anti_alias = true;
    if a.tool == Tool::Eraser {
        paint.blend_mode = BlendMode::Clear;
    }
    if a.style.filled && matches!(a.tool, Tool::Rectangle | Tool::Ellipse) {
        layer.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    } else {
        layer.stroke_path(
            &path,
            &paint,
            &Stroke {
                width: a.style.width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }
    Ok(())
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn paint_text(layer: &mut Pixmap, a: &Annotation) -> Result<()> {
    let p = a.points[0];
    let lines = a
        .text
        .lines()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                p.x,
                p.y + a.style.font_size * (1. + i as f32 * 1.3),
                escape(s)
            )
        })
        .collect::<String>();
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><text xml:space=\"preserve\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\" font-style=\"{}\" fill=\"#{:06x}\">{lines}</text></svg>",
        layer.width(),
        layer.height(),
        escape(&a.style.font_family),
        a.style.font_size,
        if a.style.bold { "700" } else { "400" },
        if a.style.italic { "italic" } else { "normal" },
        a.style.color
    );
    let options = resvg::usvg::Options {
        fontdb: fonts(),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(&svg, &options)?;
    resvg::render(&tree, Transform::identity(), &mut layer.as_mut());
    Ok(())
}

pub fn composite(document: &Document) -> Result<RgbaImage> {
    let mut output = (*document.source).clone();
    if document.annotations().is_empty() {
        return Ok(output);
    }
    let mut layer = Pixmap::new(output.width(), output.height()).context("截图尺寸无效")?;
    for a in document.annotations() {
        paint_annotation(&mut layer, a)?;
    }
    for (pixel, annotation) in output.pixels_mut().zip(layer.pixels()) {
        let alpha = u32::from(annotation.alpha());
        if alpha == 0 {
            continue;
        }
        // tiny-skia is premultiplied; screenshot RGB is straight alpha (normally opaque).
        let background_alpha = u32::from(pixel[3]);
        let out_alpha = alpha + (background_alpha * (255 - alpha) + 127) / 255;
        for (index, value) in [annotation.red(), annotation.green(), annotation.blue()]
            .into_iter()
            .enumerate()
        {
            let premult = u32::from(value) * 255
                + (u32::from(pixel[index]) * background_alpha * (255 - alpha) + 127) / 255;
            pixel[index] = ((premult + out_alpha / 2) / out_alpha).min(255) as u8;
        }
        pixel[3] = out_alpha as u8;
    }
    Ok(output)
}

pub fn export(document: &Document) -> Result<RgbaImage> {
    let r = document.selection.context("请先拖动选择截图区域")?;
    let (x, y, w, h) = r
        .pixel_bounds(document.source.width(), document.source.height())
        .context("选区为空")?;
    Ok(image::imageops::crop_imm(&composite(document)?, x, y, w, h).to_image())
}

pub fn png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Point, Rect, Style};
    #[test]
    fn eraser_restores_original_pixels_and_undo_restores_ink() {
        let source = RgbaImage::from_pixel(32, 32, image::Rgba([20, 30, 40, 255]));
        let mut doc = Document::new(source);
        doc.selection = Some(Rect {
            x: 4.,
            y: 4.,
            w: 20.,
            h: 20.,
        });
        let mut rect = Annotation::new(
            Tool::Rectangle,
            Point::new(5., 5.),
            Style {
                filled: true,
                ..Default::default()
            },
        );
        rect.extend(Point::new(25., 25.));
        doc.push(rect);
        let before = composite(&doc).unwrap();
        let mut eraser = Annotation::new(
            Tool::Eraser,
            Point::new(15., 5.),
            Style {
                width: 8.,
                ..Default::default()
            },
        );
        eraser.extend(Point::new(15., 25.));
        doc.push(eraser);
        assert_eq!(
            composite(&doc).unwrap().get_pixel(15, 15),
            doc.source.get_pixel(15, 15)
        );
        doc.undo();
        assert_eq!(composite(&doc).unwrap(), before);
        assert_eq!(export(&doc).unwrap().dimensions(), (20, 20));
    }
    #[test]
    fn png_roundtrip() {
        let image = RgbaImage::from_pixel(4, 3, image::Rgba([255, 40, 90, 255]));
        assert_eq!(
            image::load_from_memory(&png(&image).unwrap())
                .unwrap()
                .to_rgba8(),
            image
        );
    }
}
