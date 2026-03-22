use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{JobResult, OutputFile};
use super::utils;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConvertParams {
    /// Imagen fuente
    pub input: String,
    /// Ruta de salida
    pub output: String,
    /// Formato destino: "png", "jpeg", "webp", "ico"
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
            let (w, h) = utils::fit_dimensions(
                img.width(), img.height(),
                params.width, params.height,
            );
            img = img.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
        }
        img
    } else {
        utils::load_image(&params.input)?
    };

    utils::ensure_parent_dir(&params.output)?;

    match params.format.as_str() {
        "jpeg" | "jpg" => {
            let rgb = img.to_rgb8();
            let mut file = std::fs::File::create(&params.output)?;
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut file, quality
            );
            encoder.encode_image(&rgb)
                .context("JPEG encode failed")?;
        }
        "webp" => {
            let rgba = img.to_rgba8();
            let encoded = webp::Encoder::from_rgba(
                rgba.as_raw(), img.width(), img.height()
            ).encode(quality as f32);
            std::fs::write(&params.output, &*encoded)?;
        }
        "ico" => {
            // ICO con múltiples tamaños estándar
            let sizes = [16, 32, 48];
            let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

            for &size in &sizes {
                let resized = img.resize_exact(
                    size, size, image::imageops::FilterType::Lanczos3
                );
                let rgba = resized.to_rgba8();
                let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
                icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
            }

            let file = std::fs::File::create(&params.output)?;
            icon_dir.write(file)?;
        }
        _ => {
            // PNG y otros
            img.save(&params.output)
                .with_context(|| format!("Save as {} failed", params.format))?;
        }
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
            format: params.format,
            width: img.width(),
            height: img.height(),
            size_bytes: utils::file_size(&params.output),
            data_base64,
        }],
        elapsed_ms: 0,
        metadata: None,
    })
}
