//! Private stdio worker. The parent owns the scratch directory and serializes requests.
use liteparse::{LiteParse, LiteParseConfig, OutputFormat, config::OcrBackend, ocr::ocria};
use std::io::{self, BufRead, Write};

fn reply(result: Result<&str, String>) -> io::Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, &result)?;
    out.write_all(b"\n")?;
    out.flush()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpi = std::env::args().nth(1).ok_or("missing DPI")?.parse()?;
    let engine = match ocria::shared_engine() {
        Ok(engine) => engine,
        Err(e) => return Ok(reply(Err(e.to_string()))?),
    };
    let config = LiteParseConfig {
        ocr_enabled: true, // LiteParse's library default is false in builds without Tesseract.
        ocr_backend: Some(OcrBackend::OcriaNano),
        dpi: Some(dpi),
        output_format: OutputFormat::Json,
        quiet: true,
        ..Default::default()
    };
    // The parser loads LayoutNet lazily even for page text. Open its shared sessions
    // before announcing readiness, not on the user's first click.
    let table = config
        .effective_table_model()
        .and_then(|p| liteparse::table_model::load(p.as_ref()));
    let mut parser = LiteParse::new(config).with_ocr_engine(engine);
    if let Some(net) = table {
        parser = parser.with_table_net(net);
    }
    reply(Ok("ready"))?;
    for line in io::stdin().lock().lines() {
        if line? != "parse" {
            return Err("unexpected OCR worker command".into());
        }
        let parsed = parser.parse("input.png").await;
        reply(match &parsed {
            Ok(result) => match result.pages.as_slice() {
                [page] => Ok(page.text.trim()),
                _ => Err("LiteParse 未返回有效的单页 OCR 结果".into()),
            },
            Err(e) => Err(e.to_string()),
        })?;
    }
    Ok(())
}
