use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use serde::{Deserialize, Serialize};

use crate::{JobResult, OutputFile};
use super::utils;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlaceholderParams {
    /// Imagen fuente
    pub input: String,
    /// Ruta de salida (para LQIP image)
    pub output: Option<String>,
    /// Tipo: "lqip" (Low Quality Image Placeholder), "dominant_color", "css_gradient"
    pub kind: Option<String>,
    /// Ancho del LQIP (default: 20px, se escala con CSS)
    pub lqip_width: Option<u32>,
    /// Devolver resultado inline
    #[serde(default = "default_true")]
    pub inline: bool,
}

fn default_true() -> bool { true }

pub fn execute(params: PlaceholderParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let kind = params.kind.as_deref().unwrap_or("lqip");

    match kind {
        "dominant_color" => generate_dominant_color(&img, &params),
        "css_gradient" => generate_css_gradient(&img, &params),
        _ => generate_lqip(&img, &params),
    }
}

fn generate_lqip(img: &DynamicImage, params: &PlaceholderParams) -> Result<JobResult> {
    let lqip_w = params.lqip_width.unwrap_or(20);
    let (src_w, src_h) = (img.width(), img.height());
    let lqip_h = (lqip_w as f64 * src_h as f64 / src_w as f64) as u32;

    let tiny = img.resize_exact(lqip_w, lqip_h.max(1), FilterType::Triangle);
    // Aplicar un blur suave
    let blurred = tiny.blur(1.5);

    let mut outputs = Vec::new();
    let mut data_b64 = None;

    if let Some(ref out_path) = params.output {
        utils::ensure_parent_dir(out_path)?;
        blurred.save(out_path).context("Failed to save LQIP")?;
        outputs.push(OutputFile {
            path: out_path.clone(),
            format: "png".into(),
            width: lqip_w,
            height: lqip_h,
            size_bytes: utils::file_size(out_path),
            data_base64: None,
        });
    }

    if params.inline {
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        blurred.write_to(&mut cursor, image::ImageFormat::Png)?;
        use base64::Engine;
        data_b64 = Some(base64::engine::general_purpose::STANDARD.encode(&buf));
    }

    Ok(JobResult {
        success: true,
        operation: "placeholder".into(),
        outputs,
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "kind": "lqip",
            "lqip_width": lqip_w,
            "lqip_height": lqip_h,
            "data_uri": data_b64.as_ref().map(|b| format!("data:image/png;base64,{}", b)),
        })),
    })
}

fn generate_dominant_color(img: &DynamicImage, _params: &PlaceholderParams) -> Result<JobResult> {
    // Reducir a 1x1 para obtener color promedio dominante
    let tiny = img.resize_exact(1, 1, FilterType::Lanczos3);
    let pixel = tiny.get_pixel(0, 0);
    let hex = format!("#{:02x}{:02x}{:02x}", pixel[0], pixel[1], pixel[2]);

    Ok(JobResult {
        success: true,
        operation: "placeholder".into(),
        outputs: vec![],
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "kind": "dominant_color",
            "color_hex": hex,
            "color_rgb": [pixel[0], pixel[1], pixel[2]],
        })),
    })
}

fn generate_css_gradient(img: &DynamicImage, _params: &PlaceholderParams) -> Result<JobResult> {
    // Reducir a 4x4 y generar gradiente CSS
    let tiny = img.resize_exact(4, 4, FilterType::Lanczos3);
    let tl = tiny.get_pixel(0, 0);
    let tr = tiny.get_pixel(3, 0);
    let bl = tiny.get_pixel(0, 3);
    let br = tiny.get_pixel(3, 3);

    let to_hex = |p: image::Rgba<u8>| format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]);

    let css = format!(
        "background: linear-gradient(135deg, {} 0%, {} 33%, {} 66%, {} 100%);",
        to_hex(tl), to_hex(tr), to_hex(bl), to_hex(br)
    );

    Ok(JobResult {
        success: true,
        operation: "placeholder".into(),
        outputs: vec![],
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "kind": "css_gradient",
            "css": css,
            "corners": {
                "top_left": to_hex(tl),
                "top_right": to_hex(tr),
                "bottom_left": to_hex(bl),
                "bottom_right": to_hex(br),
            }
        })),
    })
}
