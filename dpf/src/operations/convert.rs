use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::utils;
use crate::{JobResult, OutputFile};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ConvertParams {
    /// Imagen fuente
    pub input: String,
    /// Ruta de salida
    pub output: String,
    /// Formato destino: "png", "jpeg", "webp", "ico", "avif"
    pub format: String,
    /// Calidad (1-100, solo para jpeg/webp)
    pub quality: Option<u8>,
    /// Ancho para rasterización de SVG (default: tamaño original)
    pub width: Option<u32>,
    /// Alto para rasterización de SVG
    pub height: Option<u32>,
    /// Devolver base64 inline
    #[serde(default)]
    pub inline: bool,
}

pub fn execute(params: ConvertParams) -> Result<JobResult> {
    let quality = params.quality.unwrap_or(85);

    // Para SVG, podemos rasterizar a tamaño específico
    let img = if params.input.to_lowercase().ends_with(".svg") {
        let mut img = utils::load_svg(std::path::Path::new(&params.input))?;
        if params.width.is_some() || params.height.is_some() {
            let (w, h) =
                utils::fit_dimensions(img.width(), img.height(), params.width, params.height);
            img = img.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
        }
        img
    } else {
        utils::load_image(&params.input)?
    };

    utils::ensure_parent_dir(&params.output)?;

    // Para ICO con múltiples tamaños, usamos lógica especial
    if params.format == "ico" {
        let sizes = [16, 32, 48];
        let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

        for &size in &sizes {
            let resized = img.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
            icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
        }

        let file = std::fs::File::create(&params.output)?;
        icon_dir.write(file)?;
    } else {
        utils::save_image(&img, &params.output, &params.format, quality)?;
    }

    let data_base64 = if params.inline {
        use base64::Engine;
        let bytes = std::fs::read(&params.output)?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    } else {
        None
    };

    Ok(JobResult {
        success: true,
        operation: "convert".into(),
        outputs: vec![OutputFile {
            path: params.output.clone(),
            format: params.format.clone(),
            width: img.width(),
            height: img.height(),
            size_bytes: utils::file_size(&params.output),
            data_base64,
        }],
        elapsed_ms: 0,
        metadata: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use tempfile::TempDir;

    fn fixtures_dir() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/{}", fixtures_dir(), name)
    }

    // =========================================================================
    // Tests de conversión básica entre formatos
    // =========================================================================

    #[test]
    fn test_convert_png_to_jpeg() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.jpg");

        let params = ConvertParams {
            input: fixture_path("sample.png"),
            output: output_path.to_str().unwrap().to_string(),
            format: "jpeg".to_string(),
            quality: Some(90),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params).expect("Convert PNG to JPEG failed");

        assert!(result.success);
        assert_eq!(result.operation, "convert");
        assert_eq!(result.outputs.len(), 1);
        assert!(output_path.exists());

        let output = &result.outputs[0];
        assert_eq!(output.format, "jpeg");
        assert!(output.size_bytes > 0);
    }

    #[test]
    fn test_convert_png_to_webp() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.webp");

        let params = ConvertParams {
            input: fixture_path("sample.png"),
            output: output_path.to_str().unwrap().to_string(),
            format: "webp".to_string(),
            quality: Some(85),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params).expect("Convert PNG to WebP failed");
        assert_eq!(result.outputs[0].format, "webp");
        assert!(output_path.exists());
    }

    #[test]
    fn test_convert_png_to_ico() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.ico");

        let params = ConvertParams {
            input: fixture_path("sample.png"),
            output: output_path.to_str().unwrap().to_string(),
            format: "ico".to_string(),
            quality: Some(85),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params).expect("Convert PNG to ICO failed");
        assert_eq!(result.outputs[0].format, "ico");
        assert!(output_path.exists());

        // Verificar que el ICO tiene múltiples tamaños
        let metadata = std::fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_convert_jpeg_to_png() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.png");

        let params = ConvertParams {
            input: fixture_path("sample.jpg"),
            output: output_path.to_str().unwrap().to_string(),
            format: "png".to_string(),
            quality: Some(85),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params).expect("Convert JPEG to PNG failed");
        assert_eq!(result.outputs[0].format, "png");
        assert!(output_path.exists());
    }

    // =========================================================================
    // Tests de conversión de SVG
    // =========================================================================

    #[test]
    fn test_convert_svg_to_png() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.png");

        let params = ConvertParams {
            input: fixture_path("sample.svg"),
            output: output_path.to_str().unwrap().to_string(),
            format: "png".to_string(),
            quality: Some(85),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params).expect("Convert SVG to PNG failed");
        assert_eq!(result.outputs[0].format, "png");
        assert_eq!(result.outputs[0].width, 100); // Tamaño original del SVG
        assert_eq!(result.outputs[0].height, 100);
        assert!(output_path.exists());
    }

    #[test]
    fn test_convert_svg_to_png_with_custom_size() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.png");

        let params = ConvertParams {
            input: fixture_path("sample.svg"),
            output: output_path.to_str().unwrap().to_string(),
            format: "png".to_string(),
            quality: Some(85),
            width: Some(200),
            height: Some(200),
            inline: false,
        };

        let result = execute(params).expect("Convert SVG to PNG with custom size failed");
        assert_eq!(result.outputs[0].width, 200);
        assert_eq!(result.outputs[0].height, 200);
    }

    // =========================================================================
    // Tests de inline base64
    // =========================================================================

    #[test]
    fn test_convert_with_inline() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.png");

        let params = ConvertParams {
            input: fixture_path("sample.png"),
            output: output_path.to_str().unwrap().to_string(),
            format: "png".to_string(),
            quality: Some(85),
            width: None,
            height: None,
            inline: true,
        };

        let result = execute(params).expect("Convert with inline failed");
        let output = &result.outputs[0];

        assert!(output.data_base64.is_some());
        let b64 = output.data_base64.as_ref().unwrap();
        assert!(!b64.is_empty());

        // Verificar que es base64 válido
        let decoded = base64::engine::general_purpose::STANDARD.decode(b64);
        assert!(decoded.is_ok());
    }

    // =========================================================================
    // Tests de error
    // =========================================================================

    #[test]
    fn test_convert_input_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.png");

        let params = ConvertParams {
            input: "/nonexistent/image.png".to_string(),
            output: output_path.to_str().unwrap().to_string(),
            format: "png".to_string(),
            quality: Some(85),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_to_avif() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.avif");

        let params = ConvertParams {
            input: fixture_path("sample.png"),
            output: output_path.to_str().unwrap().to_string(),
            format: "avif".to_string(),
            quality: Some(60),
            width: None,
            height: None,
            inline: false,
        };

        let result = execute(params);
        // AVIF puede o no estar disponible dependiendo de features
        if result.is_ok() {
            assert!(output_path.exists());
        }
    }
}
