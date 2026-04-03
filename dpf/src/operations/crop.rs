use anyhow::Result;
use image::DynamicImage;
use serde::{Deserialize, Serialize};

use base64::Engine;

use super::utils;
use crate::{JobResult, OutputFile};

/// Parameters for crop operation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct CropParams {
    /// Path to source image
    pub input: String,
    /// Path to output image
    pub output: String,
    /// Manual crop rectangle (x, y, width, height). Optional if using gravity.
    pub rect: Option<CropRect>,
    /// Smart crop mode: center, focal_point, entropy
    pub gravity: Option<String>,
    /// For focal_point gravity: 0.0-1.0 normalized coordinates
    pub focal_x: Option<f32>,
    pub focal_y: Option<f32>,
    /// Target width for smart crop (required when using gravity)
    pub width: Option<u32>,
    /// Target height for smart crop (required when using gravity)
    pub height: Option<u32>,
    /// Output format: "png", "jpeg", "webp", "avif" (default: same as input)
    pub format: Option<String>,
    /// Quality JPEG/WebP (1-100, default 85)
    pub quality: Option<u8>,
    /// Generate output inline as base64
    #[serde(default)]
    pub inline: bool,
}

/// Rectangle for manual crop
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Execute crop operation
pub fn execute(params: CropParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let (src_w, src_h) = (img.width(), img.height());
    let quality = params.quality.unwrap_or(85);

    // Determine crop rectangle
    let (x, y, w, h) = if let Some(rect) = &params.rect {
        // Manual crop mode
        validate_crop_rect(rect, src_w, src_h)?;
        (rect.x, rect.y, rect.width, rect.height)
    } else if let Some(gravity) = &params.gravity {
        // Smart crop mode
        let target_w = params
            .width
            .ok_or_else(|| anyhow::anyhow!("width is required when using gravity crop mode"))?;
        let target_h = params
            .height
            .ok_or_else(|| anyhow::anyhow!("height is required when using gravity crop mode"))?;

        match gravity.as_str() {
            "center" => crop_center(src_w, src_h, target_w, target_h),
            "focal_point" => {
                let fx = params.focal_x.unwrap_or(0.5);
                let fy = params.focal_y.unwrap_or(0.5);
                crop_focal_point(src_w, src_h, target_w, target_h, fx, fy)
            }
            "entropy" => crop_entropy(&img, target_w, target_h),
            _ => anyhow::bail!(
                "Invalid gravity mode: {}. Use: center, focal_point, entropy",
                gravity
            ),
        }
    } else {
        anyhow::bail!("Must specify either 'rect' for manual crop or 'gravity' for smart crop")
    };

    // Perform the crop
    let cropped = img.crop_imm(x, y, w, h);

    // Determine output format
    let out_ext = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&params.input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    });

    // Save the cropped image
    utils::ensure_parent_dir(&params.output)?;
    utils::save_image(&cropped, &params.output, out_ext, quality)?;

    // Generate base64 if inline requested
    let data_base64 = if params.inline {
        let bytes = std::fs::read(&params.output)?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    } else {
        None
    };

    Ok(JobResult {
        success: true,
        operation: "crop".into(),
        outputs: vec![OutputFile {
            path: params.output.clone(),
            format: out_ext.to_string(),
            width: w,
            height: h,
            size_bytes: utils::file_size(&params.output),
            data_base64,
        }],
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "source_width": src_w,
            "source_height": src_h,
            "crop_x": x,
            "crop_y": y,
            "crop_width": w,
            "crop_height": h,
            "gravity": params.gravity,
        })),
    })
}

/// Validate crop rectangle is within image bounds
fn validate_crop_rect(rect: &CropRect, img_w: u32, img_h: u32) -> Result<()> {
    if rect.x + rect.width > img_w {
        anyhow::bail!(
            "Invalid crop rectangle: x={} + width={} exceeds image width {}",
            rect.x,
            rect.width,
            img_w
        );
    }
    if rect.y + rect.height > img_h {
        anyhow::bail!(
            "Invalid crop rectangle: y={} + height={} exceeds image height {}",
            rect.y,
            rect.height,
            img_h
        );
    }
    if rect.width == 0 || rect.height == 0 {
        anyhow::bail!("Invalid crop rectangle: width and height must be > 0");
    }
    Ok(())
}

/// Center gravity crop - crop from center of image
fn crop_center(src_w: u32, src_h: u32, target_w: u32, target_h: u32) -> (u32, u32, u32, u32) {
    let target_w = target_w.min(src_w);
    let target_h = target_h.min(src_h);

    let x = (src_w - target_w) / 2;
    let y = (src_h - target_h) / 2;

    (x, y, target_w, target_h)
}

/// Focal point crop - center crop around focal point
fn crop_focal_point(
    src_w: u32,
    src_h: u32,
    target_w: u32,
    target_h: u32,
    focal_x: f32,
    focal_y: f32,
) -> (u32, u32, u32, u32) {
    let target_w = target_w.min(src_w);
    let target_h = target_h.min(src_h);

    // Clamp focal point to 0.0-1.0
    let fx = focal_x.clamp(0.0, 1.0);
    let fy = focal_y.clamp(0.0, 1.0);

    // Calculate focal point in pixels
    let focal_px = (fx * src_w as f32) as u32;
    let focal_py = (fy * src_h as f32) as u32;

    // Calculate top-left corner, ensuring we stay within bounds
    let x = focal_px.saturating_sub(target_w / 2).min(src_w - target_w);
    let y = focal_py.saturating_sub(target_h / 2).min(src_h - target_h);

    (x, y, target_w, target_h)
}

/// Entropy-based smart crop - find region with highest brightness variance
fn crop_entropy(img: &DynamicImage, target_w: u32, target_h: u32) -> (u32, u32, u32, u32) {
    let target_w = target_w.min(img.width());
    let target_h = target_h.min(img.height());

    // Convert to grayscale for variance calculation
    let gray = img.to_luma8();
    let (w, h) = (gray.width(), gray.height());

    // If target is same size or larger, return full image
    if target_w >= w && target_h >= h {
        return (0, 0, w, h);
    }

    // Use a simplified approach: divide into grid and find interesting regions
    // A more sophisticated approach would use sliding window variance
    let grid_size = 8u32;
    let cell_w = w / grid_size;
    let cell_h = h / grid_size;

    if cell_w == 0 || cell_h == 0 {
        // Image too small, just center crop
        return crop_center(w, h, target_w, target_h);
    }

    let mut max_variance: f64 = 0.0;
    let mut best_x: u32 = 0;
    let mut best_y: u32 = 0;

    // Sample grid positions
    for gy in 0..grid_size {
        for gx in 0..grid_size {
            let x = gx * cell_w;
            let y = gy * cell_h;

            // Calculate variance for this region
            let region_w = cell_w.min(w - x);
            let region_h = cell_h.min(h - y);

            if region_w > 0 && region_h > 0 {
                let variance = calculate_variance(&gray, x, y, region_w, region_h);

                // Weight by distance from edges (prefer center regions)
                let center_x = (gx as f32 / grid_size as f32 - 0.5).abs();
                let center_y = (gy as f32 / grid_size as f32 - 0.5).abs();
                let center_weight = 1.0 - (center_x + center_y) * 0.3; // Slight center bias

                let weighted_variance = variance * center_weight as f64;

                if weighted_variance > max_variance {
                    max_variance = weighted_variance;
                    best_x = x;
                    best_y = y;
                }
            }
        }
    }

    // Center the crop around the highest variance point
    let x = best_x.saturating_sub(target_w / 2).min(w - target_w);
    let y = best_y.saturating_sub(target_h / 2).min(h - target_h);

    (x, y, target_w, target_h)
}

/// Calculate brightness variance for a region
fn calculate_variance(gray: &image::GrayImage, x: u32, y: u32, w: u32, h: u32) -> f64 {
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    let mut count: u64 = 0;

    let max_x = (x + w).min(gray.width());
    let max_y = (y + h).min(gray.height());

    for py in y..max_y {
        for px in x..max_x {
            let pixel = gray.get_pixel(px, py);
            let val = pixel[0] as f64;
            sum += val;
            sum_sq += val * val;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let mean = sum / count as f64;
    let variance = (sum_sq / count as f64) - (mean * mean);

    variance.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use tempfile::TempDir;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        // Create a simple gradient image for testing
        let mut img = image::RgbaImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128;
                img.put_pixel(x, y, Rgba([r, g, b, 255]));
            }
        }
        DynamicImage::ImageRgba8(img)
    }

    // =========================================================================
    // Tests for manual crop
    // =========================================================================

    #[test]
    fn test_crop_manual_valid_rect() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        // Create test image
        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: Some(CropRect {
                x: 50,
                y: 50,
                width: 100,
                height: 100,
            }),
            gravity: None,
            focal_x: None,
            focal_y: None,
            width: None,
            height: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        assert!(result.success);
        assert_eq!(result.operation, "crop");
        assert_eq!(result.outputs.len(), 1);

        let output = &result.outputs[0];
        assert_eq!(output.width, 100);
        assert_eq!(output.height, 100);
        assert!(std::path::Path::new(&output_path).exists());
    }

    #[test]
    fn test_crop_manual_invalid_rect_exceeds_width() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path,
            rect: Some(CropRect {
                x: 50,
                y: 0,
                width: 100,
                height: 50,
            }),
            gravity: None,
            focal_x: None,
            focal_y: None,
            width: None,
            height: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("exceeds image width"));
    }

    #[test]
    fn test_crop_manual_invalid_rect_zero_size() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path,
            rect: Some(CropRect {
                x: 0,
                y: 0,
                width: 0,
                height: 100,
            }),
            gravity: None,
            focal_x: None,
            focal_y: None,
            width: None,
            height: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
    }

    // =========================================================================
    // Tests for gravity center crop
    // =========================================================================

    #[test]
    fn test_crop_gravity_center() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: None,
            gravity: Some("center".to_string()),
            focal_x: None,
            focal_y: None,
            width: Some(100),
            height: Some(100),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        assert!(result.success);
        let output = &result.outputs[0];
        assert_eq!(output.width, 100);
        assert_eq!(output.height, 100);
    }

    #[test]
    fn test_crop_gravity_center_clamps_to_image_size() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: None,
            gravity: Some("center".to_string()),
            focal_x: None,
            focal_y: None,
            width: Some(200), // Larger than image
            height: Some(200),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        let output = &result.outputs[0];
        assert_eq!(output.width, 100); // Clamped to image size
        assert_eq!(output.height, 100);
    }

    // =========================================================================
    // Tests for focal point crop
    // =========================================================================

    #[test]
    fn test_crop_focal_point_center() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: None,
            gravity: Some("focal_point".to_string()),
            focal_x: Some(0.5),
            focal_y: Some(0.5),
            width: Some(100),
            height: Some(100),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        assert!(result.success);
        let output = &result.outputs[0];
        assert_eq!(output.width, 100);
        assert_eq!(output.height, 100);

        // Check metadata
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["crop_x"], 50);
        assert_eq!(metadata["crop_y"], 50);
    }

    #[test]
    fn test_crop_focal_point_top_left() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: None,
            gravity: Some("focal_point".to_string()),
            focal_x: Some(0.0), // Top-left corner
            focal_y: Some(0.0),
            width: Some(100),
            height: Some(100),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        let metadata = result.metadata.unwrap();
        // Should be clamped to 0,0
        assert_eq!(metadata["crop_x"], 0);
        assert_eq!(metadata["crop_y"], 0);
    }

    #[test]
    fn test_crop_focal_point_bottom_right() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: None,
            gravity: Some("focal_point".to_string()),
            focal_x: Some(1.0), // Bottom-right corner
            focal_y: Some(1.0),
            width: Some(100),
            height: Some(100),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        let metadata = result.metadata.unwrap();
        // Should be clamped to stay within bounds
        assert_eq!(metadata["crop_x"], 100);
        assert_eq!(metadata["crop_y"], 100);
    }

    // =========================================================================
    // Tests for entropy crop
    // =========================================================================

    #[test]
    fn test_crop_entropy_basic() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path.clone(),
            rect: None,
            gravity: Some("entropy".to_string()),
            focal_x: None,
            focal_y: None,
            width: Some(100),
            height: Some(100),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Crop failed");

        assert!(result.success);
        let output = &result.outputs[0];
        assert_eq!(output.width, 100);
        assert_eq!(output.height, 100);
    }

    // =========================================================================
    // Tests for edge cases
    // =========================================================================

    #[test]
    fn test_crop_missing_rect_and_gravity() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path,
            rect: None,
            gravity: None,
            focal_x: None,
            focal_y: None,
            width: None,
            height: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Must specify"));
    }

    #[test]
    fn test_crop_gravity_missing_dimensions() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path,
            rect: None,
            gravity: Some("center".to_string()),
            focal_x: None,
            focal_y: None,
            width: None, // Missing!
            height: Some(50),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("width is required"));
    }

    #[test]
    fn test_crop_invalid_gravity_mode() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path,
            rect: None,
            gravity: Some("invalid_mode".to_string()),
            focal_x: None,
            focal_y: None,
            width: Some(50),
            height: Some(50),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid gravity mode"));
    }

    #[test]
    fn test_crop_with_inline_output() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = CropParams {
            input: input_path,
            output: output_path,
            rect: Some(CropRect {
                x: 0,
                y: 0,
                width: 50,
                height: 50,
            }),
            gravity: None,
            focal_x: None,
            focal_y: None,
            width: None,
            height: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: true,
        };

        let result = execute(params).expect("Crop failed");

        let output = &result.outputs[0];
        assert!(output.data_base64.is_some());
        let b64 = output.data_base64.as_ref().unwrap();
        assert!(!b64.is_empty());
        assert!(!b64.contains(' '));
    }

    #[test]
    fn test_crop_center_helper() {
        // Test the crop_center helper function directly
        let (x, y, w, h) = crop_center(200, 200, 100, 100);
        assert_eq!(x, 50);
        assert_eq!(y, 50);
        assert_eq!(w, 100);
        assert_eq!(h, 100);

        // Test with odd dimensions
        let (x, y, _w, _h) = crop_center(201, 201, 100, 100);
        assert_eq!(x, 50); // (201-100)/2 = 50.5, truncated to 50
        assert_eq!(y, 50);
    }

    #[test]
    fn test_crop_focal_point_helper() {
        // Test the crop_focal_point helper function directly
        let (x, y, w, h) = crop_focal_point(200, 200, 100, 100, 0.5, 0.5);
        assert_eq!(x, 50); // Center focal point
        assert_eq!(y, 50);
        assert_eq!(w, 100);
        assert_eq!(h, 100);

        // Test with focal point at top-left
        let (x, y, _w, _h) = crop_focal_point(200, 200, 100, 100, 0.0, 0.0);
        assert_eq!(x, 0); // Clamped to edge
        assert_eq!(y, 0);

        // Test with focal point at bottom-right
        let (x, y, _w, _h) = crop_focal_point(200, 200, 100, 100, 1.0, 1.0);
        assert_eq!(x, 100); // 200 - 100
        assert_eq!(y, 100);
    }
}
