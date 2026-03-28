use anyhow::{Context, Result};
use image::imageops::FilterType;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::utils;
use crate::{JobResult, OutputFile};

/// Tamaños estándar de favicon para web.
const FAVICON_SIZES: &[u32] = &[16, 32, 48, 64, 128, 180, 192, 512];

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FaviconParams {
    /// Imagen o SVG fuente
    pub input: String,
    /// Directorio de salida
    pub output_dir: String,
    /// Tamaños específicos (default: set completo para web)
    pub sizes: Option<Vec<u32>>,
    /// Generar también el .ico multi-tamaño
    #[serde(default = "default_true")]
    pub generate_ico: bool,
    /// Generar manifest.json con los iconos
    #[serde(default)]
    pub generate_manifest: bool,
    /// Nombre base del archivo (default: "favicon")
    pub prefix: Option<String>,
}

fn default_true() -> bool {
    true
}

pub fn execute(params: FaviconParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let sizes = params.sizes.as_deref().unwrap_or(FAVICON_SIZES);
    let prefix = params.prefix.as_deref().unwrap_or("favicon");

    utils::ensure_parent_dir(&format!("{}/x", params.output_dir))?;

    // Generar PNGs en paralelo
    let png_outputs: Vec<Result<OutputFile>> = sizes
        .par_iter()
        .map(|&size| {
            let resized = img.resize_exact(size, size, FilterType::Lanczos3);
            let path = format!("{}/{}-{}x{}.png", params.output_dir, prefix, size, size);
            resized
                .save(&path)
                .with_context(|| format!("Failed to save favicon {}x{}", size, size))?;

            Ok(OutputFile {
                path,
                format: "png".into(),
                width: size,
                height: size,
                size_bytes: 0, // Se actualiza después
                data_base64: None,
            })
        })
        .collect();

    let mut outputs: Vec<OutputFile> = Vec::new();
    for result in png_outputs {
        let mut out = result?;
        out.size_bytes = utils::file_size(&out.path);
        outputs.push(out);
    }

    // Generar .ico con 16, 32, 48
    if params.generate_ico {
        let ico_path = format!("{}/{}.ico", params.output_dir, prefix);
        let ico_sizes = [16u32, 32, 48];
        let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

        for &size in &ico_sizes {
            let resized = img.resize_exact(size, size, FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let icon_image = ico::IconImage::from_rgba_data(size, size, rgba.into_raw());
            icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image)?);
        }

        let file = std::fs::File::create(&ico_path)?;
        icon_dir.write(file)?;

        outputs.push(OutputFile {
            path: ico_path.clone(),
            format: "ico".into(),
            width: 48,
            height: 48,
            size_bytes: utils::file_size(&ico_path),
            data_base64: None,
        });
    }

    // Generar manifest.json para PWA
    if params.generate_manifest {
        let icons: Vec<serde_json::Value> = sizes
            .iter()
            .map(|&s| {
                serde_json::json!({
                    "src": format!("{}-{}x{}.png", prefix, s, s),
                    "sizes": format!("{}x{}", s, s),
                    "type": "image/png"
                })
            })
            .collect();

        let manifest = serde_json::json!({
            "icons": icons
        });

        let manifest_path = format!("{}/manifest.json", params.output_dir);
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    }

    Ok(JobResult {
        success: true,
        operation: "favicon".into(),
        outputs,
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "sizes_generated": sizes,
            "ico_generated": params.generate_ico,
            "manifest_generated": params.generate_manifest,
        })),
    })
}
