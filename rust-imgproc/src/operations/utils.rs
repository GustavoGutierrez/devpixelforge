use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::Path;

/// Carga una imagen desde archivo, soportando formatos comunes + SVG.
pub fn load_image(path: &str) -> Result<DynamicImage> {
    let path = Path::new(path);
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "svg" {
        load_svg(path)
    } else {
        image::open(path)
            .with_context(|| format!("Cannot open image: {}", path.display()))
    }
}

/// Rasteriza SVG a imagen usando resvg.
pub fn load_svg(path: &Path) -> Result<DynamicImage> {
    let svg_data = std::fs::read(path)
        .with_context(|| format!("Cannot read SVG: {}", path.display()))?;

    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let options = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(&svg_data, &options)
        .context("Failed to parse SVG")?;

    let size = tree.size();
    let width = size.width() as u32;
    let height = size.height() as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .context("Failed to create pixmap for SVG")?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );

    let img = image::RgbaImage::from_raw(width, height, pixmap.data().to_vec())
        .context("Failed to create image from SVG raster")?;

    Ok(DynamicImage::ImageRgba8(img))
}

/// Calcula dimensiones manteniendo aspect ratio.
pub fn fit_dimensions(
    src_w: u32, src_h: u32,
    max_w: Option<u32>, max_h: Option<u32>,
) -> (u32, u32) {
    match (max_w, max_h) {
        (Some(w), Some(h)) => {
            let ratio_w = w as f64 / src_w as f64;
            let ratio_h = h as f64 / src_h as f64;
            let ratio = ratio_w.min(ratio_h);
            ((src_w as f64 * ratio) as u32, (src_h as f64 * ratio) as u32)
        }
        (Some(w), None) => {
            let ratio = w as f64 / src_w as f64;
            (w, (src_h as f64 * ratio) as u32)
        }
        (None, Some(h)) => {
            let ratio = h as f64 / src_h as f64;
            ((src_w as f64 * ratio) as u32, h)
        }
        (None, None) => (src_w, src_h),
    }
}

/// Obtiene el tamaño en bytes de un archivo.
pub fn file_size(path: &str) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Asegura que el directorio de salida exista.
pub fn ensure_parent_dir(path: &str) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Cannot create output dir: {}", parent.display()))?;
    }
    Ok(())
}
