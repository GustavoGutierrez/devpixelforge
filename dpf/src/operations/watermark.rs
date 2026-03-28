//! Image watermark operations (text and image overlay).
//!
//! Supports text watermarks with rusttype rendering and image watermarks
//! with opacity control and 9-position grid placement.

use anyhow::{Context, Result};
use image::{DynamicImage, GenericImageView, Rgba};
use rusttype::{Font, Scale};
use serde::{Deserialize, Serialize};

use base64::Engine;

use super::utils;
use crate::{JobResult, OutputFile};

/// Parameters for watermark operation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WatermarkParams {
    /// Path to source image
    pub input: String,
    /// Path to output image
    pub output: String,
    /// Text watermark (alternative to image)
    pub text: Option<String>,
    /// Image watermark path (alternative to text)
    pub image: Option<String>,
    /// Position: top-left, top-center, top-right, center-left, center, center-right, bottom-left, bottom-center, bottom-right
    #[serde(default = "default_position")]
    pub position: String,
    /// Opacity 0.0-1.0 (default: 1.0 = fully opaque)
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    /// For text: font size in pixels (default: 24)
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    /// For text: hex color (default: #FFFFFF)
    #[serde(default = "default_color")]
    pub color: String,
    /// Horizontal offset from edge in pixels (default: 10)
    #[serde(default = "default_padding")]
    pub offset_x: u32,
    /// Vertical offset from edge in pixels (default: 10)
    #[serde(default = "default_padding")]
    pub offset_y: u32,
    /// Output format: "png", "jpeg", "webp", "avif" (default: same as input)
    pub format: Option<String>,
    /// Quality JPEG/WebP (1-100, default 85)
    pub quality: Option<u8>,
    /// Generate output inline as base64
    #[serde(default)]
    pub inline: bool,
}

fn default_position() -> String {
    "bottom-right".to_string()
}

fn default_opacity() -> f32 {
    1.0
}

fn default_font_size() -> u32 {
    24
}

fn default_color() -> String {
    "#FFFFFF".to_string()
}

fn default_padding() -> u32 {
    10
}

/// Watermark position in a 3x3 grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatermarkPosition {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl WatermarkPosition {
    /// Parse position string to enum
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "top-left" | "topleft" => Ok(WatermarkPosition::TopLeft),
            "top-center" | "topcenter" | "top" | "center-top" => Ok(WatermarkPosition::TopCenter),
            "top-right" | "topright" => Ok(WatermarkPosition::TopRight),
            "center-left" | "centerleft" | "left" | "middle-left" => {
                Ok(WatermarkPosition::CenterLeft)
            }
            "center" | "middle" => Ok(WatermarkPosition::Center),
            "center-right" | "centerright" | "right" | "middle-right" => {
                Ok(WatermarkPosition::CenterRight)
            }
            "bottom-left" | "bottomleft" => Ok(WatermarkPosition::BottomLeft),
            "bottom-center" | "bottomcenter" | "bottom" | "center-bottom" => {
                Ok(WatermarkPosition::BottomCenter)
            }
            "bottom-right" | "bottomright" => Ok(WatermarkPosition::BottomRight),
            _ => anyhow::bail!(
                "Invalid position '{}'. Valid options: top-left, top-center, top-right, \
                 center-left, center, center-right, bottom-left, bottom-center, bottom-right",
                s
            ),
        }
    }
}

/// Execute watermark operation
pub fn execute(params: WatermarkParams) -> Result<JobResult> {
    let mut img = utils::load_image(&params.input)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    // Validate that either text or image watermark is provided
    if params.text.is_none() && params.image.is_none() {
        anyhow::bail!("Either 'text' or 'image' watermark must be specified");
    }

    // Validate opacity range
    if !(0.0..=1.0).contains(&params.opacity) {
        anyhow::bail!(
            "Opacity must be between 0.0 and 1.0, got {}",
            params.opacity
        );
    }

    // Parse position
    let position = WatermarkPosition::from_str(&params.position)?;

    // Apply watermark based on type
    if let Some(text) = &params.text {
        img = apply_text_watermark(&img, text, &params)?;
    } else if let Some(watermark_path) = &params.image {
        img = apply_image_watermark(&img, watermark_path, &params, position)?;
    }

    let (final_w, final_h) = (img.width(), img.height());
    let quality = params.quality.unwrap_or(85);

    // Determine output format
    let out_ext = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&params.input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    });

    // Save the watermarked image
    utils::ensure_parent_dir(&params.output)?;
    utils::save_image(&img, &params.output, out_ext, quality)?;

    // Generate base64 if inline requested
    let data_base64 = if params.inline {
        let bytes = std::fs::read(&params.output)?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    } else {
        None
    };

    Ok(JobResult {
        success: true,
        operation: "watermark".into(),
        outputs: vec![OutputFile {
            path: params.output.clone(),
            format: out_ext.to_string(),
            width: final_w,
            height: final_h,
            size_bytes: utils::file_size(&params.output),
            data_base64,
        }],
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "original_width": orig_w,
            "original_height": orig_h,
            "position": params.position,
            "opacity": params.opacity,
            "watermark_type": if params.text.is_some() { "text" } else { "image" },
        })),
    })
}

/// Apply text watermark using rusttype
fn apply_text_watermark(
    img: &DynamicImage,
    text: &str,
    params: &WatermarkParams,
) -> Result<DynamicImage> {
    // Load the embedded font
    let font_data = include_bytes!("../fonts/Roboto-Regular.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).context("Failed to load embedded font")?;

    // Parse color from hex
    let color = parse_hex_color(&params.color)?;
    let scale = Scale::uniform(params.font_size as f32);

    // Calculate text dimensions
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(text, scale, rusttype::point(0.0, v_metrics.ascent))
        .collect();

    let text_width = glyphs
        .iter()
        .last()
        .and_then(|g| g.pixel_bounding_box())
        .map(|bb| bb.max.x)
        .unwrap_or(0) as u32;

    let text_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;

    // Parse position
    let position = WatermarkPosition::from_str(&params.position)?;

    // Calculate position
    let (x, y) = calculate_position(
        img.width(),
        img.height(),
        text_width,
        text_height,
        position,
        params.offset_x,
        params.offset_y,
    );

    // Create output image (RGBA for alpha blending)
    let mut rgba = img.to_rgba8();

    // Draw text with opacity
    let opacity = (params.opacity * 255.0) as u8;

    for glyph in glyphs {
        let glyph_bb = match glyph.pixel_bounding_box() {
            Some(bb) => bb,
            None => continue,
        };

        // Draw glyph pixels
        glyph.draw(|gx, gy, intensity| {
            let px = x as i32 + gx as i32 + glyph_bb.min.x;
            let py = y as i32 + gy as i32 + glyph_bb.min.y;

            // Check bounds
            if px < 0 || px >= rgba.width() as i32 || py < 0 || py >= rgba.height() as i32 {
                return;
            }

            if intensity > 0.0 {
                let alpha = ((intensity / 255.0) * (opacity as f32 / 255.0) * 255.0) as u8;

                // Blend with existing pixel
                let existing = rgba.get_pixel(px as u32, py as u32);
                let blended = blend_pixel(*existing, Rgba([color[0], color[1], color[2], alpha]));
                rgba.put_pixel(px as u32, py as u32, blended);
            }
        });
    }

    Ok(DynamicImage::ImageRgba8(rgba))
}

/// Apply image watermark (logo overlay)
fn apply_image_watermark(
    img: &DynamicImage,
    watermark_path: &str,
    params: &WatermarkParams,
    position: WatermarkPosition,
) -> Result<DynamicImage> {
    // Load watermark image
    let watermark = utils::load_image(watermark_path)?;
    let (wm_w, wm_h) = (watermark.width(), watermark.height());

    // Calculate position
    let (x, y) = calculate_position(
        img.width(),
        img.height(),
        wm_w,
        wm_h,
        position,
        params.offset_x,
        params.offset_y,
    );

    // Create output image (RGBA for alpha blending)
    let mut rgba = img.to_rgba8();

    // Blend watermark with opacity
    for dy in 0..wm_h {
        for dx in 0..wm_w {
            let px = x + dx;
            let py = y + dy;

            // Check bounds
            if px >= rgba.width() || py >= rgba.height() {
                continue;
            }

            let wm_pixel = watermark.get_pixel(dx, dy);
            let bg_pixel = *rgba.get_pixel(px, py);

            // Apply opacity
            let alpha = (wm_pixel[3] as f32 * params.opacity / 255.0) as u8;

            if alpha > 0 {
                let blended = blend_pixel(
                    bg_pixel,
                    Rgba([wm_pixel[0], wm_pixel[1], wm_pixel[2], alpha]),
                );
                rgba.put_pixel(px, py, blended);
            }
        }
    }

    Ok(DynamicImage::ImageRgba8(rgba))
}

/// Calculate watermark position based on grid position
fn calculate_position(
    img_w: u32,
    img_h: u32,
    wm_w: u32,
    wm_h: u32,
    position: WatermarkPosition,
    offset_x: u32,
    offset_y: u32,
) -> (u32, u32) {
    match position {
        WatermarkPosition::TopLeft => (offset_x, offset_y),
        WatermarkPosition::TopCenter => ((img_w.saturating_sub(wm_w)) / 2, offset_y),
        WatermarkPosition::TopRight => (
            img_w.saturating_sub(wm_w).saturating_sub(offset_x),
            offset_y,
        ),
        WatermarkPosition::CenterLeft => (offset_x, (img_h.saturating_sub(wm_h)) / 2),
        WatermarkPosition::Center => (
            (img_w.saturating_sub(wm_w)) / 2,
            (img_h.saturating_sub(wm_h)) / 2,
        ),
        WatermarkPosition::CenterRight => (
            img_w.saturating_sub(wm_w).saturating_sub(offset_x),
            (img_h.saturating_sub(wm_h)) / 2,
        ),
        WatermarkPosition::BottomLeft => (
            offset_x,
            img_h.saturating_sub(wm_h).saturating_sub(offset_y),
        ),
        WatermarkPosition::BottomCenter => (
            (img_w.saturating_sub(wm_w)) / 2,
            img_h.saturating_sub(wm_h).saturating_sub(offset_y),
        ),
        WatermarkPosition::BottomRight => (
            img_w.saturating_sub(wm_w).saturating_sub(offset_x),
            img_h.saturating_sub(wm_h).saturating_sub(offset_y),
        ),
    }
}

/// Parse hex color string to RGB array
fn parse_hex_color(hex: &str) -> Result<[u8; 3]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        anyhow::bail!("Invalid color format. Use: #RRGGBB");
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| anyhow::anyhow!("Invalid red component in color"))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| anyhow::anyhow!("Invalid green component in color"))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| anyhow::anyhow!("Invalid blue component in color"))?;

    Ok([r, g, b])
}

/// Blend two RGBA pixels using alpha compositing
fn blend_pixel(bottom: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    let alpha_top = top[3] as f32 / 255.0;
    let alpha_bottom = bottom[3] as f32 / 255.0;

    // Alpha compositing formula (over operator)
    let alpha_out = alpha_top + alpha_bottom * (1.0 - alpha_top);

    if alpha_out == 0.0 {
        return Rgba([0, 0, 0, 0]);
    }

    let r = ((top[0] as f32 * alpha_top + bottom[0] as f32 * alpha_bottom * (1.0 - alpha_top))
        / alpha_out) as u8;
    let g = ((top[1] as f32 * alpha_top + bottom[1] as f32 * alpha_bottom * (1.0 - alpha_top))
        / alpha_out) as u8;
    let b = ((top[2] as f32 * alpha_top + bottom[2] as f32 * alpha_bottom * (1.0 - alpha_top))
        / alpha_out) as u8;
    let a = (alpha_out * 255.0) as u8;

    Rgba([r, g, b, a])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        let mut img = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128u8;
                img.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
        DynamicImage::ImageRgba8(img)
    }

    fn create_watermark_image(width: u32, height: u32) -> DynamicImage {
        let mut img = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                img.put_pixel(x, y, Rgba([255, 255, 255, 200]));
            }
        }
        DynamicImage::ImageRgba8(img)
    }

    // =========================================================================
    // Tests for WatermarkPosition parsing
    // =========================================================================

    #[test]
    fn test_position_parsing_top_left() {
        assert_eq!(
            WatermarkPosition::from_str("top-left").unwrap(),
            WatermarkPosition::TopLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("topleft").unwrap(),
            WatermarkPosition::TopLeft
        );
    }

    #[test]
    fn test_position_parsing_top_center() {
        assert_eq!(
            WatermarkPosition::from_str("top-center").unwrap(),
            WatermarkPosition::TopCenter
        );
        assert_eq!(
            WatermarkPosition::from_str("topcenter").unwrap(),
            WatermarkPosition::TopCenter
        );
        assert_eq!(
            WatermarkPosition::from_str("top").unwrap(),
            WatermarkPosition::TopCenter
        );
    }

    #[test]
    fn test_position_parsing_top_right() {
        assert_eq!(
            WatermarkPosition::from_str("top-right").unwrap(),
            WatermarkPosition::TopRight
        );
        assert_eq!(
            WatermarkPosition::from_str("topright").unwrap(),
            WatermarkPosition::TopRight
        );
    }

    #[test]
    fn test_position_parsing_center_left() {
        assert_eq!(
            WatermarkPosition::from_str("center-left").unwrap(),
            WatermarkPosition::CenterLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("centerleft").unwrap(),
            WatermarkPosition::CenterLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("left").unwrap(),
            WatermarkPosition::CenterLeft
        );
    }

    #[test]
    fn test_position_parsing_center() {
        assert_eq!(
            WatermarkPosition::from_str("center").unwrap(),
            WatermarkPosition::Center
        );
        assert_eq!(
            WatermarkPosition::from_str("middle").unwrap(),
            WatermarkPosition::Center
        );
    }

    #[test]
    fn test_position_parsing_center_right() {
        assert_eq!(
            WatermarkPosition::from_str("center-right").unwrap(),
            WatermarkPosition::CenterRight
        );
        assert_eq!(
            WatermarkPosition::from_str("centerright").unwrap(),
            WatermarkPosition::CenterRight
        );
        assert_eq!(
            WatermarkPosition::from_str("right").unwrap(),
            WatermarkPosition::CenterRight
        );
    }

    #[test]
    fn test_position_parsing_bottom_left() {
        assert_eq!(
            WatermarkPosition::from_str("bottom-left").unwrap(),
            WatermarkPosition::BottomLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("bottomleft").unwrap(),
            WatermarkPosition::BottomLeft
        );
    }

    #[test]
    fn test_position_parsing_bottom_center() {
        assert_eq!(
            WatermarkPosition::from_str("bottom-center").unwrap(),
            WatermarkPosition::BottomCenter
        );
        assert_eq!(
            WatermarkPosition::from_str("bottomcenter").unwrap(),
            WatermarkPosition::BottomCenter
        );
        assert_eq!(
            WatermarkPosition::from_str("bottom").unwrap(),
            WatermarkPosition::BottomCenter
        );
    }

    #[test]
    fn test_position_parsing_bottom_right() {
        assert_eq!(
            WatermarkPosition::from_str("bottom-right").unwrap(),
            WatermarkPosition::BottomRight
        );
        assert_eq!(
            WatermarkPosition::from_str("bottomright").unwrap(),
            WatermarkPosition::BottomRight
        );
    }

    #[test]
    fn test_position_parsing_invalid() {
        let result = WatermarkPosition::from_str("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid position"));
    }

    #[test]
    fn test_position_case_insensitive() {
        assert_eq!(
            WatermarkPosition::from_str("TOP-LEFT").unwrap(),
            WatermarkPosition::TopLeft
        );
        assert_eq!(
            WatermarkPosition::from_str("BottomRight").unwrap(),
            WatermarkPosition::BottomRight
        );
    }

    // =========================================================================
    // Tests for color parsing
    // =========================================================================

    #[test]
    fn test_parse_hex_color_white() {
        let color = parse_hex_color("#FFFFFF").unwrap();
        assert_eq!(color, [255, 255, 255]);
    }

    #[test]
    fn test_parse_hex_color_black() {
        let color = parse_hex_color("#000000").unwrap();
        assert_eq!(color, [0, 0, 0]);
    }

    #[test]
    fn test_parse_hex_color_red() {
        let color = parse_hex_color("#FF0000").unwrap();
        assert_eq!(color, [255, 0, 0]);
    }

    #[test]
    fn test_parse_hex_color_green() {
        let color = parse_hex_color("#00FF00").unwrap();
        assert_eq!(color, [0, 255, 0]);
    }

    #[test]
    fn test_parse_hex_color_blue() {
        let color = parse_hex_color("#0000FF").unwrap();
        assert_eq!(color, [0, 0, 255]);
    }

    #[test]
    fn test_parse_hex_color_without_hash() {
        let color = parse_hex_color("FFFFFF").unwrap();
        assert_eq!(color, [255, 255, 255]);
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        let result = parse_hex_color("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hex_color_too_short() {
        let result = parse_hex_color("#FFF");
        assert!(result.is_err());
    }

    // =========================================================================
    // Tests for position calculation
    // =========================================================================

    #[test]
    fn test_calculate_position_top_left() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::TopLeft, 10, 10);
        assert_eq!(x, 10);
        assert_eq!(y, 10);
    }

    #[test]
    fn test_calculate_position_top_center() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::TopCenter, 10, 10);
        assert_eq!(x, 40); // (100 - 20) / 2
        assert_eq!(y, 10);
    }

    #[test]
    fn test_calculate_position_top_right() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::TopRight, 10, 10);
        assert_eq!(x, 70); // 100 - 20 - 10
        assert_eq!(y, 10);
    }

    #[test]
    fn test_calculate_position_center_left() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::CenterLeft, 10, 10);
        assert_eq!(x, 10);
        assert_eq!(y, 45); // (100 - 10) / 2
    }

    #[test]
    fn test_calculate_position_center() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::Center, 10, 10);
        assert_eq!(x, 40); // (100 - 20) / 2
        assert_eq!(y, 45); // (100 - 10) / 2
    }

    #[test]
    fn test_calculate_position_center_right() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::CenterRight, 10, 10);
        assert_eq!(x, 70);
        assert_eq!(y, 45);
    }

    #[test]
    fn test_calculate_position_bottom_left() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::BottomLeft, 10, 10);
        assert_eq!(x, 10);
        assert_eq!(y, 80); // 100 - 10 - 10
    }

    #[test]
    fn test_calculate_position_bottom_center() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::BottomCenter, 10, 10);
        assert_eq!(x, 40);
        assert_eq!(y, 80);
    }

    #[test]
    fn test_calculate_position_bottom_right() {
        let (x, y) = calculate_position(100, 100, 20, 10, WatermarkPosition::BottomRight, 10, 10);
        assert_eq!(x, 70);
        assert_eq!(y, 80);
    }

    #[test]
    fn test_calculate_position_watermark_larger_than_image() {
        // Should not panic with saturating arithmetic
        let (x, y) = calculate_position(50, 50, 100, 100, WatermarkPosition::TopLeft, 10, 10);
        assert_eq!(x, 10);
        assert_eq!(y, 10);
    }

    // =========================================================================
    // Tests for image watermark
    // =========================================================================

    #[test]
    fn test_image_watermark_bottom_right() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let wm_path = format!("{}/watermark.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        let wm = create_watermark_image(50, 20);

        img.save(&input_path).unwrap();
        wm.save(&wm_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: None,
            image: Some(wm_path),
            position: "bottom-right".to_string(),
            opacity: 0.8,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 10,
            offset_y: 10,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Watermark failed");
        assert!(result.success);
        assert_eq!(result.operation, "watermark");
    }

    #[test]
    fn test_image_watermark_top_left() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let wm_path = format!("{}/watermark.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        let wm = create_watermark_image(30, 15);

        img.save(&input_path).unwrap();
        wm.save(&wm_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: None,
            image: Some(wm_path),
            position: "top-left".to_string(),
            opacity: 1.0,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 5,
            offset_y: 5,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Watermark failed");
        assert!(result.success);
    }

    #[test]
    fn test_image_watermark_center() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let wm_path = format!("{}/watermark.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        let wm = create_watermark_image(40, 20);

        img.save(&input_path).unwrap();
        wm.save(&wm_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: None,
            image: Some(wm_path),
            position: "center".to_string(),
            opacity: 0.5,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 0,
            offset_y: 0,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Watermark failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for text watermark
    // =========================================================================

    #[test]
    fn test_text_watermark_bottom_right() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: Some("© 2024".to_string()),
            image: None,
            position: "bottom-right".to_string(),
            opacity: 0.8,
            font_size: 20,
            color: "#FFFFFF".to_string(),
            offset_x: 10,
            offset_y: 10,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Text watermark failed");
        assert!(result.success);
        assert_eq!(result.operation, "watermark");
    }

    #[test]
    fn test_text_watermark_custom_color() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: Some("Test".to_string()),
            image: None,
            position: "top-left".to_string(),
            opacity: 1.0,
            font_size: 16,
            color: "#FF5500".to_string(),
            offset_x: 5,
            offset_y: 5,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Text watermark with color failed");
        assert!(result.success);
    }

    #[test]
    fn test_text_watermark_different_font_size() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: Some("Small".to_string()),
            image: None,
            position: "center".to_string(),
            opacity: 1.0,
            font_size: 12,
            color: "#FFFFFF".to_string(),
            offset_x: 0,
            offset_y: 0,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Text watermark with small font failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for edge cases
    // =========================================================================

    #[test]
    fn test_no_watermark_specified() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path,
            text: None,
            image: None,
            position: "bottom-right".to_string(),
            opacity: 1.0,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 10,
            offset_y: 10,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("text' or 'image' watermark"));
    }

    #[test]
    fn test_invalid_opacity_too_high() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path,
            text: Some("Test".to_string()),
            image: None,
            position: "bottom-right".to_string(),
            opacity: 1.5, // Invalid
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 10,
            offset_y: 10,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Opacity must be between"));
    }

    #[test]
    fn test_invalid_opacity_negative() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path,
            text: Some("Test".to_string()),
            image: None,
            position: "bottom-right".to_string(),
            opacity: -0.5, // Invalid
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 10,
            offset_y: 10,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Opacity must be between"));
    }

    #[test]
    fn test_watermark_image_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path,
            text: None,
            image: Some("/nonexistent/path/watermark.png".to_string()),
            position: "bottom-right".to_string(),
            opacity: 1.0,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 10,
            offset_y: 10,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_opacity() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: Some("Full".to_string()),
            image: None,
            position: "center".to_string(),
            opacity: 1.0,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 0,
            offset_y: 0,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Full opacity watermark failed");
        assert!(result.success);
    }

    #[test]
    fn test_zero_opacity() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: Some("Invisible".to_string()),
            image: None,
            position: "center".to_string(),
            opacity: 0.0,
            font_size: 24,
            color: "#FFFFFF".to_string(),
            offset_x: 0,
            offset_y: 0,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Zero opacity watermark failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for inline output
    // =========================================================================

    #[test]
    fn test_watermark_with_inline() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path,
            text: Some("Inline".to_string()),
            image: None,
            position: "top-left".to_string(),
            opacity: 1.0,
            font_size: 16,
            color: "#FFFFFF".to_string(),
            offset_x: 5,
            offset_y: 5,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: true,
        };

        let result = execute(params).expect("Inline watermark failed");
        let output = &result.outputs[0];
        assert!(output.data_base64.is_some());
        let b64 = output.data_base64.as_ref().unwrap();
        assert!(!b64.is_empty());
    }

    // =========================================================================
    // Tests for metadata
    // =========================================================================

    #[test]
    fn test_watermark_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = WatermarkParams {
            input: input_path,
            output: output_path.clone(),
            text: Some("Metadata".to_string()),
            image: None,
            position: "bottom-right".to_string(),
            opacity: 0.75,
            font_size: 20,
            color: "#FF0000".to_string(),
            offset_x: 15,
            offset_y: 20,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Watermark failed");

        let metadata = result.metadata.expect("Should have metadata");
        assert_eq!(metadata["original_width"], 200);
        assert_eq!(metadata["original_height"], 100);
        assert_eq!(metadata["position"], "bottom-right");
        assert_eq!(metadata["opacity"], 0.75);
        assert_eq!(metadata["watermark_type"], "text");
    }
}
