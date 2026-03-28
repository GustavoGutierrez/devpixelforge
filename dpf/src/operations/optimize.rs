use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::utils;
use crate::{JobResult, OutputFile};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OptimizeParams {
    /// Archivos a optimizar (pueden ser múltiples)
    pub inputs: Vec<String>,
    /// Directorio de salida (si no se indica, sobreescribe los originales)
    pub output_dir: Option<String>,
    /// Nivel de optimización: "lossless", "lossy", "aggressive"
    pub level: Option<String>,
    /// Calidad para lossy (1-100, default 80)
    pub quality: Option<u8>,
    /// Convertir a WebP además de optimizar el original
    #[serde(default)]
    pub also_webp: bool,
}

pub fn execute(params: OptimizeParams) -> Result<JobResult> {
    let level = params.level.as_deref().unwrap_or("lossless");
    let quality = params.quality.unwrap_or(80);

    let outputs: Vec<Result<Vec<OutputFile>>> = params
        .inputs
        .par_iter()
        .map(|input| {
            let mut results = Vec::new();
            let ext = std::path::Path::new(input)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let out_path = if let Some(ref dir) = params.output_dir {
                let filename = std::path::Path::new(input)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("output");
                let p = format!("{}/{}", dir, filename);
                utils::ensure_parent_dir(&p)?;
                // Copiar original al destino primero
                std::fs::copy(input, &p)?;
                p
            } else {
                input.clone()
            };

            match ext.as_str() {
                "png" => {
                    optimize_png(&out_path, level)?;
                }
                "jpeg" | "jpg" => {
                    optimize_jpeg(&out_path, quality, level)?;
                }
                _ => {
                    // Para otros formatos, solo copiar si hay output_dir
                }
            }

            let img = image::open(&out_path)?;
            results.push(OutputFile {
                path: out_path.clone(),
                format: ext.clone(),
                width: img.width(),
                height: img.height(),
                size_bytes: utils::file_size(&out_path),
                data_base64: None,
            });

            // Generar WebP adicional si se solicitó
            if params.also_webp {
                let webp_path = format!(
                    "{}.webp",
                    std::path::Path::new(&out_path).with_extension("").display()
                );
                let rgba = img.to_rgba8();
                let encoded = webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height())
                    .encode(quality as f32);
                std::fs::write(&webp_path, &*encoded)?;

                results.push(OutputFile {
                    path: webp_path.clone(),
                    format: "webp".into(),
                    width: img.width(),
                    height: img.height(),
                    size_bytes: utils::file_size(&webp_path),
                    data_base64: None,
                });
            }

            Ok(results)
        })
        .collect();

    let mut all_outputs = Vec::new();
    for result in outputs {
        all_outputs.extend(result?);
    }

    let total_saved: i64 = all_outputs.iter().map(|o| o.size_bytes as i64).sum();

    Ok(JobResult {
        success: true,
        operation: "optimize".into(),
        outputs: all_outputs,
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "level": level,
            "total_output_bytes": total_saved,
            "files_processed": params.inputs.len(),
        })),
    })
}

fn optimize_png(path: &str, level: &str) -> Result<()> {
    let opts = match level {
        "aggressive" => oxipng::Options {
            strip: oxipng::StripChunks::Safe,
            interlace: Some(oxipng::Interlacing::None),
            ..oxipng::Options::max_compression()
        },
        "lossy" => oxipng::Options {
            strip: oxipng::StripChunks::Safe,
            ..oxipng::Options::from_preset(4)
        },
        _ => oxipng::Options {
            strip: oxipng::StripChunks::Safe,
            ..oxipng::Options::from_preset(2)
        },
    };

    oxipng::optimize(
        &oxipng::InFile::Path(path.into()),
        &oxipng::OutFile::Path {
            path: Some(path.into()),
            preserve_attrs: true,
        },
        &opts,
    )
    .context("PNG optimization failed")?;

    Ok(())
}

fn optimize_jpeg(path: &str, quality: u8, _level: &str) -> Result<()> {
    let img = image::open(path)?;
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);

    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(w, h);
    comp.set_quality(quality as f32);
    let mut started = comp
        .start_compress(Vec::new())
        .map_err(|e| anyhow::anyhow!("mozjpeg start failed: {}", e))?;
    started
        .write_scanlines(rgb.as_raw())
        .map_err(|e| anyhow::anyhow!("mozjpeg write failed: {}", e))?;
    let data = started
        .finish()
        .map_err(|_| anyhow::anyhow!("mozjpeg compression failed"))?;

    std::fs::write(path, &data)?;
    Ok(())
}
