use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage, imageops::FilterType};
use serde::{Deserialize, Serialize};

use crate::{JobResult, OutputFile};
use super::utils;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SpriteParams {
    /// Lista de imágenes para el sprite
    pub inputs: Vec<String>,
    /// Ruta de salida del sprite sheet
    pub output: String,
    /// Tamaño de cada celda (cuadrado). Default: 64
    pub cell_size: Option<u32>,
    /// Columnas del grid. Default: auto (sqrt de inputs)
    pub columns: Option<u32>,
    /// Padding entre celdas en px. Default: 0
    pub padding: Option<u32>,
    /// Generar CSS con las coordenadas del sprite
    #[serde(default)]
    pub generate_css: bool,
}

pub fn execute(params: SpriteParams) -> Result<JobResult> {
    let cell = params.cell_size.unwrap_or(64);
    let pad = params.padding.unwrap_or(0);
    let n = params.inputs.len() as u32;
    let cols = params.columns.unwrap_or_else(|| (n as f64).sqrt().ceil() as u32);
    let rows = (n + cols - 1) / cols;

    let total_w = cols * cell + (cols.saturating_sub(1)) * pad;
    let total_h = rows * cell + (rows.saturating_sub(1)) * pad;

    let mut sprite = RgbaImage::new(total_w, total_h);

    let images: Vec<(usize, DynamicImage)> = params.inputs.iter()
        .enumerate()
        .map(|(i, path)| {
            let img = utils::load_image(path)
                .with_context(|| format!("Failed to load sprite input: {}", path))?;
            let img = img.resize_exact(cell, cell, FilterType::Lanczos3);
            Ok((i, img))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut css_entries = Vec::new();

    for (i, img) in &images {
        let col = *i as u32 % cols;
        let row = *i as u32 / cols;
        let x = col * (cell + pad);
        let y = row * (cell + pad);

        image::imageops::overlay(&mut sprite, &img.to_rgba8(), x as i64, y as i64);

        if params.generate_css {
            let fallback = format!("icon-{}", i);
            let name = std::path::Path::new(&params.inputs[*i])
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&fallback);
            css_entries.push(format!(
                ".sprite-{} {{ background-position: -{}px -{}px; width: {}px; height: {}px; }}",
                name, x, y, cell, cell
            ));
        }
    }

    utils::ensure_parent_dir(&params.output)?;
    DynamicImage::ImageRgba8(sprite).save(&params.output)
        .context("Failed to save sprite sheet")?;

    let mut outputs = vec![OutputFile {
        path: params.output.clone(),
        format: "png".into(),
        width: total_w,
        height: total_h,
        size_bytes: utils::file_size(&params.output),
        data_base64: None,
    }];

    if params.generate_css {
        let css_path = format!("{}.css", params.output.trim_end_matches(".png"));
        let sprite_filename = std::path::Path::new(&params.output)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("sprite.png");

        let css = format!(
            "[class^=\"sprite-\"] {{\n  background-image: url('{}');\n  background-repeat: no-repeat;\n  display: inline-block;\n}}\n\n{}",
            sprite_filename,
            css_entries.join("\n")
        );
        std::fs::write(&css_path, &css)?;
        outputs.push(OutputFile {
            path: css_path.clone(),
            format: "css".into(),
            width: 0,
            height: 0,
            size_bytes: utils::file_size(&css_path),
            data_base64: None,
        });
    }

    Ok(JobResult {
        success: true,
        operation: "sprite".into(),
        outputs,
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "cells": n,
            "columns": cols,
            "rows": rows,
            "cell_size": cell,
        })),
    })
}
