use anyhow::{Context, Result};
use image::imageops::FilterType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{JobResult, OutputFile};
use super::utils;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResizeParams {
    /// Ruta de la imagen fuente
    pub input: String,
    /// Directorio de salida
    pub output_dir: String,
    /// Anchos deseados (genera una imagen por cada uno)
    /// Ej: [320, 640, 1024, 1920] para responsive
    pub widths: Vec<u32>,
    /// Alto máximo opcional (mantiene aspect ratio)
    pub max_height: Option<u32>,
    /// Formato de salida: "png", "jpeg", "webp" (default: mismo que input)
    pub format: Option<String>,
    /// Calidad JPEG/WebP (1-100, default 85)
    pub quality: Option<u8>,
    /// Filtro de resize: "lanczos3" (default), "nearest", "triangle", "catmullrom"
    pub filter: Option<String>,
    /// Generar output inline como base64
    #[serde(default)]
    pub inline: bool,
}

pub fn execute(params: ResizeParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let (src_w, src_h) = (img.width(), img.height());
    let quality = params.quality.unwrap_or(85);

    let filter = match params.filter.as_deref() {
        Some("nearest") => FilterType::Nearest,
        Some("triangle") => FilterType::Triangle,
        Some("catmullrom") => FilterType::CatmullRom,
        _ => FilterType::Lanczos3,
    };

    let out_ext = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&params.input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    });

    // Generar todas las variantes en paralelo con rayon
    let outputs: Vec<Result<OutputFile>> = params.widths
        .par_iter()
        .map(|&target_w| {
            let (w, h) = utils::fit_dimensions(src_w, src_h, Some(target_w), params.max_height);
            let resized = img.resize_exact(w, h, filter);

            let filename = format!(
                "{}_{}w.{}",
                std::path::Path::new(&params.input)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("image"),
                w,
                out_ext
            );
            let out_path = format!("{}/{}", params.output_dir, filename);

            utils::ensure_parent_dir(&out_path)?;
            save_image(&resized, &out_path, out_ext, quality)?;

            let data_base64 = if params.inline {
                let bytes = std::fs::read(&out_path)?;
                Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
            } else {
                None
            };

            Ok(OutputFile {
                path: out_path.clone(),
                format: out_ext.to_string(),
                width: w,
                height: h,
                size_bytes: utils::file_size(&out_path),
                data_base64,
            })
        })
        .collect();

    let mut final_outputs = Vec::new();
    for result in outputs {
        final_outputs.push(result?);
    }

    Ok(JobResult {
        success: true,
        operation: "resize".into(),
        outputs: final_outputs,
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "source_width": src_w,
            "source_height": src_h,
            "variants_generated": params.widths.len(),
        })),
    })
}

use base64::Engine;

fn save_image(img: &image::DynamicImage, path: &str, format: &str, quality: u8) -> Result<()> {
    match format {
        "jpeg" | "jpg" => {
            let rgb = img.to_rgb8();
            let mut file = std::fs::File::create(path)?;
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut file, quality
            );
            encoder.encode_image(&rgb)
                .context("JPEG encode failed")?;
        }
        "webp" => {
            // Usar image crate para WebP
            let rgba = img.to_rgba8();
            let encoded = webp::Encoder::from_rgba(
                rgba.as_raw(), img.width(), img.height()
            ).encode(quality as f32);
            std::fs::write(path, &*encoded)?;
        }
        _ => {
            // PNG y otros formatos via image crate
            img.save(path)
                .with_context(|| format!("Failed to save as {}", format))?;
        }
    }
    Ok(())
}
