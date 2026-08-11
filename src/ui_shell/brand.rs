//! Load brand assets for window / tray icons.

use anyhow::{bail, Context, Result};
use image::imageops::FilterType;
use image::ImageReader;
use std::io::Cursor;
use std::path::PathBuf;

const EMBEDDED_LOGO_PNG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/brand/logo.png"));

fn logo_path_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("ui/brand/logo.png"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/brand/logo.png"),
    ]
}

fn decode_png_bytes(bytes: &[u8]) -> Result<image::RgbaImage> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("guess logo image format")?;
    Ok(reader.decode().context("decode logo png")?.into_rgba8())
}

fn load_logo_rgba() -> Result<image::RgbaImage> {
    for p in logo_path_candidates() {
        if p.is_file() {
            let bytes = std::fs::read(&p).with_context(|| format!("read {}", p.display()))?;
            return decode_png_bytes(&bytes);
        }
    }
    decode_png_bytes(EMBEDDED_LOGO_PNG)
}

pub fn brand_icon_rgba(size: u32) -> Result<(Vec<u8>, u32, u32)> {
    if size == 0 {
        bail!("icon size must be > 0");
    }
    let img = load_logo_rgba()?;
    let resized = image::imageops::resize(&img, size, size, FilterType::Lanczos3);
    let (w, h) = resized.dimensions();
    Ok((resized.into_raw(), w, h))
}
