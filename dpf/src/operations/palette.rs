use anyhow::{Context, Result};
use image::{DynamicImage, ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};

use super::utils;
use crate::{JobResult, OutputFile};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PaletteParams {
    /// Ruta de la imagen fuente
    pub input: String,
    /// Directorio de salida
    pub output_dir: String,
    /// Número máximo de colores en la paleta (default: 256)
    pub max_colors: Option<u32>,
    /// Nivel de dithering: 0 (sin dithering) a 1.0 (máximo)
    /// El dithering suaviza bandas en degradados a costa de ruido visual
    pub dithering: Option<f32>,
    /// Formato de salida: "png" (default), "gif"
    pub format: Option<String>,
    /// Generar output inline como base64
    #[serde(default)]
    pub inline: bool,
}

pub fn execute(params: PaletteParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let (src_w, src_h) = (img.width(), img.height());

    let max_colors = params.max_colors.unwrap_or(256).clamp(2, 256) as usize;
    let dither_level = params.dithering.unwrap_or(0.0).clamp(0.0, 1.0);
    let out_ext = params.format.as_deref().unwrap_or("png");

    // Reducir la paleta
    let quantized = reduce_palette(&img, max_colors, dither_level)?;

    let filename = format!(
        "{}_palette_{}.{}",
        std::path::Path::new(&params.input)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image"),
        max_colors,
        out_ext
    );
    let out_path = format!("{}/{}", params.output_dir, filename);

    utils::ensure_parent_dir(&out_path)?;

    // Guardar según el formato
    match out_ext {
        "gif" => {
            // Para GIF, convertir a paleta indexada
            let rgb = quantized.to_rgb8();
            let mut gif_encoder =
                image::codecs::gif::GifEncoder::new(std::fs::File::create(&out_path)?);
            gif_encoder
                .encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ColorType::Rgb8.into(),
                )
                .context("GIF encode failed")?;
        }
        _ => {
            // PNG u otros formatos
            quantized
                .save(&out_path)
                .with_context(|| format!("Failed to save as {}", out_ext))?;
        }
    }

    let data_base64 = if params.inline {
        let bytes = std::fs::read(&out_path)?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    } else {
        None
    };

    let output = OutputFile {
        path: out_path.clone(),
        format: out_ext.to_string(),
        width: quantized.width(),
        height: quantized.height(),
        size_bytes: utils::file_size(&out_path),
        data_base64,
    };

    Ok(JobResult {
        success: true,
        operation: "palette".into(),
        outputs: vec![output],
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "source_width": src_w,
            "source_height": src_h,
            "max_colors": max_colors,
            "dithering": dither_level,
            "colors_in_palette": max_colors,
        })),
    })
}

/// Reduce la paleta de colores usando cuantización median cut simplificada
fn reduce_palette(
    img: &DynamicImage,
    max_colors: usize,
    dither_level: f32,
) -> Result<DynamicImage> {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels: Vec<Rgba<u8>> = rgba.pixels().cloned().collect();

    // Si ya tiene pocos colores, no hacer nada
    if pixels.len() <= max_colors {
        return Ok(img.clone());
    }

    // Extraer colores únicos con su frecuencia
    let mut color_counts: std::collections::HashMap<[u8; 4], usize> =
        std::collections::HashMap::new();
    for pixel in &pixels {
        *color_counts.entry(pixel.0).or_insert(0) += 1;
    }

    // Si ya tenemos pocos colores únicos, solo mapear
    if color_counts.len() <= max_colors {
        return Ok(img.clone());
    }

    // Crear paleta usando k-means simplificado (Lloyd's algorithm)
    let palette = kmeans_palette(&color_counts, max_colors, 10)?;

    // Crear imagen de salida
    let mut output = ImageBuffer::new(width, height);

    // Para dithering, usamos error diffusion simple (Floyd-Steinberg modificado)
    let mut errors: Vec<[f32; 4]> = vec![[0.0; 4]; pixels.len()];

    for (idx, (x, y, pixel)) in rgba.enumerate_pixels().enumerate() {
        let original = {
            let e = errors[idx];
            [
                (pixel[0] as f32 + e[0]).clamp(0.0, 255.0),
                (pixel[1] as f32 + e[1]).clamp(0.0, 255.0),
                (pixel[2] as f32 + e[2]).clamp(0.0, 255.0),
                (pixel[3] as f32 + e[3]).clamp(0.0, 255.0),
            ]
        };

        // Encontrar color más cercano en la paleta
        let nearest = find_nearest_color(&original, &palette);

        output.put_pixel(x, y, Rgba(nearest));

        // Propagar error si hay dithering
        if dither_level > 0.0 {
            let error = [
                (original[0] - nearest[0] as f32) * dither_level,
                (original[1] - nearest[1] as f32) * dither_level,
                (original[2] - nearest[2] as f32) * dither_level,
                (original[3] - nearest[3] as f32) * dither_level,
            ];

            // Diffuse to neighboring pixels (simplified Floyd-Steinberg)
            let w = width as usize;
            let i = idx;

            // Right
            if (i + 1) % w != 0 && i + 1 < errors.len() {
                for c in 0..4 {
                    errors[i + 1][c] += error[c] * 7.0 / 16.0;
                }
            }
            // Bottom-left
            if i + w > 0 && i + w - 1 < errors.len() && i % w != 0 {
                for c in 0..4 {
                    errors[i + w - 1][c] += error[c] * 3.0 / 16.0;
                }
            }
            // Bottom
            if i + w < errors.len() {
                for c in 0..4 {
                    errors[i + w][c] += error[c] * 5.0 / 16.0;
                }
            }
            // Bottom-right
            if i + w + 1 < errors.len() && (i + 1) % w != 0 {
                for c in 0..4 {
                    errors[i + w + 1][c] += error[c] * 1.0 / 16.0;
                }
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(output))
}

/// K-means simplificado para generar paleta de colores
fn kmeans_palette(
    color_counts: &std::collections::HashMap<[u8; 4], usize>,
    k: usize,
    max_iterations: usize,
) -> Result<Vec<[u8; 4]>> {
    let colors: Vec<([u8; 4], usize)> = color_counts.iter().map(|(&c, &n)| (c, n)).collect();

    if colors.len() <= k {
        return Ok(colors.into_iter().map(|(c, _)| c).collect());
    }

    // Inicializar centroides con colores aleatorios ponderados por frecuencia
    let mut centroids: Vec<[f32; 4]> = Vec::with_capacity(k);
    let total_pixels: usize = colors.iter().map(|(_, n)| n).sum();

    // Seleccionar centroides iniciales distribuidos
    let step = total_pixels / k;
    let mut accum = 0;
    let mut centroid_idx = 0;

    for (color, count) in &colors {
        accum += count;
        if accum >= step * (centroid_idx + 1) && centroids.len() < k {
            centroids.push([
                color[0] as f32,
                color[1] as f32,
                color[2] as f32,
                color[3] as f32,
            ]);
            centroid_idx += 1;
        }
    }

    // Asegurar que tenemos k centroides
    while centroids.len() < k {
        let idx = centroids.len() % colors.len();
        centroids.push([
            colors[idx].0[0] as f32,
            colors[idx].0[1] as f32,
            colors[idx].0[2] as f32,
            colors[idx].0[3] as f32,
        ]);
    }

    // Iterar k-means
    for _ in 0..max_iterations {
        let mut new_centroids: Vec<[f32; 4]> = vec![[0.0; 4]; k];
        let mut counts: Vec<f32> = vec![0.0; k];

        // Asignar cada color al centroide más cercano
        for (color, weight) in &colors {
            let c = [
                color[0] as f32,
                color[1] as f32,
                color[2] as f32,
                color[3] as f32,
            ];
            let nearest = find_nearest_centroid(&c, &centroids);

            for i in 0..4 {
                new_centroids[nearest][i] += c[i] * *weight as f32;
            }
            counts[nearest] += *weight as f32;
        }

        // Actualizar centroides
        let mut changed = false;
        for i in 0..k {
            if counts[i] > 0.0 {
                let new_c = [
                    (new_centroids[i][0] / counts[i]).clamp(0.0, 255.0),
                    (new_centroids[i][1] / counts[i]).clamp(0.0, 255.0),
                    (new_centroids[i][2] / counts[i]).clamp(0.0, 255.0),
                    (new_centroids[i][3] / counts[i]).clamp(0.0, 255.0),
                ];

                if color_distance(&new_c, &centroids[i]) > 1.0 {
                    changed = true;
                }
                centroids[i] = new_c;
            }
        }

        if !changed {
            break;
        }
    }

    Ok(centroids
        .into_iter()
        .map(|c| [c[0] as u8, c[1] as u8, c[2] as u8, c[3] as u8])
        .collect())
}

fn find_nearest_color(color: &[f32; 4], palette: &[[u8; 4]]) -> [u8; 4] {
    let mut min_dist = f32::MAX;
    let mut nearest = palette[0];

    for &p in palette {
        let dist = color_distance(color, &[p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]);
        if dist < min_dist {
            min_dist = dist;
            nearest = p;
        }
    }

    nearest
}

fn find_nearest_centroid(color: &[f32; 4], centroids: &[[f32; 4]]) -> usize {
    let mut min_dist = f32::MAX;
    let mut nearest = 0;

    for (i, c) in centroids.iter().enumerate() {
        let dist = color_distance(color, c);
        if dist < min_dist {
            min_dist = dist;
            nearest = i;
        }
    }

    nearest
}

fn color_distance(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    // Distancia euclidiana en RGBA con peso para alpha
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    let da = a[3] - b[3];

    // Dar más peso a diferencias de color que de alpha
    (dr * dr + dg * dg + db * db + da * da * 0.5).sqrt()
}

use base64::Engine;
