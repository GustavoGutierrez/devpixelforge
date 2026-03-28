use anyhow::{Context, Result};
use image::DynamicImage;
use std::path::Path;

/// Guarda una imagen en el formato especificado soportando PNG, JPEG, WebP, ICO y AVIF.
pub fn save_image(img: &DynamicImage, path: &str, format: &str, quality: u8) -> Result<()> {
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => {
            let rgb = img.to_rgb8();
            let mut file = std::fs::File::create(path)?;
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);
            encoder.encode_image(&rgb).context("JPEG encode failed")?;
        }
        "webp" => {
            let rgba = img.to_rgba8();
            let encoded = webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height())
                .encode(quality as f32);
            std::fs::write(path, &*encoded)?;
        }
        "avif" => {
            // Usar image crate para AVIF
            let rgba = img.to_rgba8();
            let file = std::fs::File::create(path)?;
            rgba.write_with_encoder(image::codecs::avif::AvifEncoder::new_with_speed_quality(
                std::io::BufWriter::new(file),
                4, // speed (0-10, higher is faster)
                quality,
            ))
            .context("AVIF encode failed")?;
        }
        "ico" => {
            // Para ICO, creamos un icono con el tamaño actual de la imagen
            let rgba = img.to_rgba8();
            let icon_image =
                ico::IconImage::from_rgba_data(img.width(), img.height(), rgba.into_raw());
            let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
            icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
            let file = std::fs::File::create(path)?;
            icon_dir.write(file)?;
        }
        _ => {
            // PNG y otros formatos via image crate
            img.save(path)
                .with_context(|| format!("Failed to save as {}", format))?;
        }
    }
    Ok(())
}

/// Carga una imagen desde archivo, soportando formatos comunes + SVG + AVIF.
pub fn load_image(path: &str) -> Result<DynamicImage> {
    let path = Path::new(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "svg" {
        load_svg(path)
    } else {
        // image crate soporta AVIF nativamente con el feature "avif"
        image::open(path).with_context(|| format!("Cannot open image: {}", path.display()))
    }
}

/// Rasteriza SVG a imagen usando resvg.
pub fn load_svg(path: &Path) -> Result<DynamicImage> {
    let svg_data =
        std::fs::read(path).with_context(|| format!("Cannot read SVG: {}", path.display()))?;

    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    let options = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(&svg_data, &options).context("Failed to parse SVG")?;

    let size = tree.size();
    let width = size.width() as u32;
    let height = size.height() as u32;

    let mut pixmap =
        resvg::tiny_skia::Pixmap::new(width, height).context("Failed to create pixmap for SVG")?;

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
    src_w: u32,
    src_h: u32,
    max_w: Option<u32>,
    max_h: Option<u32>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixtures_dir() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/{}", fixtures_dir(), name)
    }

    // =========================================================================
    // Tests para fit_dimensions
    // =========================================================================

    #[test]
    fn test_fit_dimensions_both_dimensions() {
        // Ambas dimensiones especificadas - debe mantener aspect ratio
        let (w, h) = fit_dimensions(1000, 500, Some(500), Some(200));
        // Ratio w: 500/1000 = 0.5, ratio h: 200/500 = 0.4
        // Debe usar el ratio menor (0.4)
        assert_eq!(w, 400);
        assert_eq!(h, 200);
    }

    #[test]
    fn test_fit_dimensions_width_only() {
        // Solo ancho especificado
        let (w, h) = fit_dimensions(1000, 500, Some(500), None);
        assert_eq!(w, 500);
        assert_eq!(h, 250); // 500 * (500/1000)
    }

    #[test]
    fn test_fit_dimensions_height_only() {
        // Solo alto especificado
        let (w, h) = fit_dimensions(1000, 500, None, Some(250));
        assert_eq!(w, 500); // 250 * (1000/500)
        assert_eq!(h, 250);
    }

    #[test]
    fn test_fit_dimensions_none() {
        // Sin dimensiones - retorna original
        let (w, h) = fit_dimensions(1000, 500, None, None);
        assert_eq!(w, 1000);
        assert_eq!(h, 500);
    }

    #[test]
    fn test_fit_dimensions_square_to_portrait() {
        // Cuadrado a portrait
        let (w, h) = fit_dimensions(1000, 1000, Some(500), Some(1000));
        // Ratio w: 0.5, ratio h: 1.0 -> usa 0.5
        assert_eq!(w, 500);
        assert_eq!(h, 500);
    }

    // =========================================================================
    // Tests para load_image
    // =========================================================================

    #[test]
    fn test_load_image_png() {
        let path = fixture_path("sample.png");
        let img = load_image(&path).expect("Failed to load PNG");
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_load_image_jpeg() {
        let path = fixture_path("sample.jpg");
        let img = load_image(&path).expect("Failed to load JPEG");
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_load_image_svg() {
        let path = fixture_path("sample.svg");
        let img = load_image(&path).expect("Failed to load SVG");
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_load_image_not_found() {
        let result = load_image("/nonexistent/path/image.png");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_image_corrupt() {
        let path = fixture_path("corrupt/bad.png");
        let result = load_image(&path);
        assert!(result.is_err());
    }

    // =========================================================================
    // Tests para save_image
    // =========================================================================

    #[test]
    fn test_save_image_png() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.png");

        let img = image::DynamicImage::new_rgba8(50, 50);
        save_image(&img, output_path.to_str().unwrap(), "png", 85).expect("Failed to save PNG");

        assert!(output_path.exists());
        let loaded = image::open(&output_path).expect("Failed to reload saved PNG");
        assert_eq!(loaded.width(), 50);
        assert_eq!(loaded.height(), 50);
    }

    #[test]
    fn test_save_image_jpeg() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.jpg");

        let img = image::DynamicImage::new_rgba8(50, 50);
        save_image(&img, output_path.to_str().unwrap(), "jpeg", 90).expect("Failed to save JPEG");

        assert!(output_path.exists());
        let loaded = image::open(&output_path).expect("Failed to reload saved JPEG");
        assert_eq!(loaded.width(), 50);
        assert_eq!(loaded.height(), 50);
    }

    #[test]
    fn test_save_image_webp() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.webp");

        let img = image::DynamicImage::new_rgba8(50, 50);
        save_image(&img, output_path.to_str().unwrap(), "webp", 80).expect("Failed to save WebP");

        assert!(output_path.exists());
        // WebP puede cargarse como imagen genérica
        assert!(image::open(&output_path).is_ok());
    }

    #[test]
    fn test_save_image_ico() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.ico");

        let img = image::DynamicImage::new_rgba8(32, 32);
        save_image(&img, output_path.to_str().unwrap(), "ico", 85).expect("Failed to save ICO");

        assert!(output_path.exists());
        assert!(image::open(&output_path).is_ok());
    }

    #[test]
    fn test_save_image_avif() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.avif");

        let img = image::DynamicImage::new_rgba8(50, 50);
        // AVIF encoding puede ser lento, usamos calidad baja
        save_image(&img, output_path.to_str().unwrap(), "avif", 50).expect("Failed to save AVIF");

        assert!(output_path.exists());
    }

    #[test]
    fn test_save_image_invalid_format() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_output.unknown");

        let img = image::DynamicImage::new_rgba8(50, 50);
        // Formatos desconocidos causan error (no hay formato default)
        let result = save_image(&img, output_path.to_str().unwrap(), "unknown", 85);
        // Debería fallar porque "unknown" no es un formato soportado
        // y el default usa image::save que requiere extensión conocida
        assert!(result.is_err());
    }

    // =========================================================================
    // Tests para file_size
    // =========================================================================

    #[test]
    fn test_file_size_existing() {
        let path = fixture_path("sample.png");
        let size = file_size(&path);
        assert!(size > 0);
    }

    #[test]
    fn test_file_size_not_found() {
        let size = file_size("/nonexistent/file.png");
        assert_eq!(size, 0);
    }

    // =========================================================================
    // Tests para ensure_parent_dir
    // =========================================================================

    #[test]
    fn test_ensure_parent_dir_creates_nested() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("a/b/c/test.png");

        ensure_parent_dir(nested_path.to_str().unwrap()).expect("Failed to create parent dirs");

        assert!(temp_dir.path().join("a/b/c").exists());
    }

    #[test]
    fn test_ensure_parent_dir_existing() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path().join("test.png");

        // No debe fallar si el directorio ya existe
        ensure_parent_dir(existing_path.to_str().unwrap()).expect("Failed on existing dir");
    }

    #[test]
    fn test_ensure_parent_dir_root_file() {
        // Archivo sin directorio padre (ej: "test.png" en directorio actual)
        ensure_parent_dir("test.png").expect("Should not fail for files without parent");
    }
}
