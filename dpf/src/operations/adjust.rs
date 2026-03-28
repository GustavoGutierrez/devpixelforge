//! Image adjustment operations (brightness, contrast, saturation, blur, sharpen).
//!
//! Uses imageproc crate for efficient pixel operations with optional linear RGB conversion.

use anyhow::Result;
use image::DynamicImage;
use serde::{Deserialize, Serialize};

use base64::Engine;

use super::utils;
use crate::{JobResult, OutputFile};

/// Parameters for adjust operation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AdjustParams {
    /// Path to source image
    pub input: String,
    /// Path to output image
    pub output: String,
    /// Brightness adjustment: -1.0 to 1.0 (0 = no change, negative = darker, positive = brighter)
    pub brightness: Option<f32>,
    /// Contrast adjustment: -1.0 to 1.0 (0 = no change, negative = less contrast, positive = more contrast)
    pub contrast: Option<f32>,
    /// Saturation adjustment: -1.0 to 1.0 (0 = original, -1 = grayscale, 1 = super-saturated)
    pub saturation: Option<f32>,
    /// Gaussian blur sigma (0.0 = no blur, typical range: 0.5-5.0)
    pub blur: Option<f32>,
    /// Sharpen amount: 0.0 = no sharpen, typical range: 0.5-3.0
    pub sharpen: Option<f32>,
    /// Use linear RGB for color adjustments (default: true, produces better results)
    #[serde(default = "default_true")]
    pub linear_rgb: bool,
    /// Output format: "png", "jpeg", "webp", "avif" (default: same as input)
    pub format: Option<String>,
    /// Quality JPEG/WebP (1-100, default 85)
    pub quality: Option<u8>,
    /// Generate output inline as base64
    #[serde(default)]
    pub inline: bool,
}

fn default_true() -> bool {
    true
}

/// Execute adjust operation
pub fn execute(params: AdjustParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    // Validate parameters
    validate_params(&params)?;

    // Apply adjustments
    let mut result = img;

    // Convert to linear RGB if needed for color operations
    let use_linear = params.linear_rgb
        && (params.brightness.is_some()
            || params.contrast.is_some()
            || params.saturation.is_some());

    if use_linear {
        result = result.to_rgb32f().into();
    }

    // Apply brightness
    if let Some(b) = params.brightness {
        result = apply_brightness(&result, b)?;
    }

    // Apply contrast
    if let Some(c) = params.contrast {
        result = apply_contrast(&result, c)?;
    }

    // Apply saturation
    if let Some(s) = params.saturation {
        result = apply_saturation(&result, s)?;
    }

    // Convert back from linear RGB if needed for blur/sharpen
    if use_linear {
        result = result.to_rgb8().into();
    }

    // Apply blur
    if let Some(sigma) = params.blur {
        if sigma > 0.0 {
            result = apply_blur(&result, sigma)?;
        }
    }

    // Apply sharpen
    if let Some(amount) = params.sharpen {
        if amount > 0.0 {
            result = apply_sharpen(&result, amount)?;
        }
    }

    let (final_w, final_h) = (result.width(), result.height());
    let quality = params.quality.unwrap_or(85);

    // Determine output format
    let out_ext = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&params.input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    });

    // Save the adjusted image
    utils::ensure_parent_dir(&params.output)?;
    utils::save_image(&result, &params.output, out_ext, quality)?;

    // Generate base64 if inline requested
    let data_base64 = if params.inline {
        let bytes = std::fs::read(&params.output)?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    } else {
        None
    };

    Ok(JobResult {
        success: true,
        operation: "adjust".into(),
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
            "brightness": params.brightness.unwrap_or(0.0),
            "contrast": params.contrast.unwrap_or(0.0),
            "saturation": params.saturation.unwrap_or(0.0),
            "blur": params.blur.unwrap_or(0.0),
            "sharpen": params.sharpen.unwrap_or(0.0),
            "linear_rgb": params.linear_rgb,
        })),
    })
}

/// Validate adjustment parameters
fn validate_params(params: &AdjustParams) -> Result<()> {
    if let Some(b) = params.brightness {
        if !(-1.0..=1.0).contains(&b) {
            anyhow::bail!("Brightness must be between -1.0 and 1.0, got {}", b);
        }
    }

    if let Some(c) = params.contrast {
        if !(-1.0..=1.0).contains(&c) {
            anyhow::bail!("Contrast must be between -1.0 and 1.0, got {}", c);
        }
    }

    if let Some(s) = params.saturation {
        if !(-1.0..=1.0).contains(&s) {
            anyhow::bail!("Saturation must be between -1.0 and 1.0, got {}", s);
        }
    }

    if let Some(blur) = params.blur {
        if blur < 0.0 {
            anyhow::bail!("Blur sigma must be >= 0.0, got {}", blur);
        }
    }

    if let Some(sharpen) = params.sharpen {
        if sharpen < 0.0 {
            anyhow::bail!("Sharpen amount must be >= 0.0, got {}", sharpen);
        }
    }

    Ok(())
}

/// Apply brightness adjustment using pixel manipulation
fn apply_brightness(img: &DynamicImage, amount: f32) -> Result<DynamicImage> {
    // Handle both RGB8 and RGB32F images
    if let Some(rgb_f) = img.as_rgb32f() {
        let mut output = rgb_f.clone();
        // For floating point images, amount is scaled
        let adjustment = amount / 3.0;
        for pixel in output.pixels_mut() {
            for i in 0..3 {
                pixel[i] = (pixel[i] + adjustment).clamp(0.0, 1.0);
            }
        }
        Ok(DynamicImage::ImageRgb32F(output))
    } else {
        let rgb = img.to_rgb8();
        let mut output = rgb.clone();

        // Convert amount to pixel value adjustment (-255 to 255)
        let adjustment = (amount * 255.0).round() as i32;

        for pixel in output.pixels_mut() {
            for i in 0..3 {
                let val = pixel[i] as i32 + adjustment;
                pixel[i] = val.clamp(0, 255) as u8;
            }
        }

        Ok(DynamicImage::ImageRgb8(output))
    }
}

/// Apply contrast adjustment using pixel manipulation
fn apply_contrast(img: &DynamicImage, amount: f32) -> Result<DynamicImage> {
    // Handle both RGB8 and RGB32F images
    if let Some(rgb_f) = img.as_rgb32f() {
        let mut output = rgb_f.clone();

        // Calculate mean luminance
        let total: f32 = output.pixels().map(|p| (p[0] + p[1] + p[2]) / 3.0).sum();
        let count = (output.width() * output.height()) as f32;
        let mean = total / count;

        // Contrast factor: 1.0 = no change, > 1.0 = more contrast, < 1.0 = less contrast
        let factor = 1.0 + amount;

        for pixel in output.pixels_mut() {
            for i in 0..3 {
                pixel[i] = ((pixel[i] - mean) * factor + mean).clamp(0.0, 1.0);
            }
        }

        Ok(DynamicImage::ImageRgb32F(output))
    } else {
        let rgb = img.to_rgb8();
        let mut output = rgb.clone();

        // Calculate mean luminance
        let total: u32 = output
            .pixels()
            .map(|p| (p[0] as u32 + p[1] as u32 + p[2] as u32) / 3)
            .sum();
        let count = output.width() * output.height();
        let mean = (total / count) as f32;

        // Contrast factor
        let factor = 1.0 + amount;

        for pixel in output.pixels_mut() {
            for i in 0..3 {
                let val = pixel[i] as f32;
                let adjusted = ((val - mean) * factor + mean).clamp(0.0, 255.0);
                pixel[i] = adjusted.round() as u8;
            }
        }

        Ok(DynamicImage::ImageRgb8(output))
    }
}

/// Apply saturation adjustment
fn apply_saturation(img: &DynamicImage, amount: f32) -> Result<DynamicImage> {
    // Handle both RGB8 and RGB32F images
    if let Some(rgb_f) = img.as_rgb32f() {
        let mut output = rgb_f.clone();

        // amount: -1 = grayscale, 0 = original, 1 = super-saturated
        // Convert to saturation factor: 0 = desaturated, 1 = original, 2 = double saturation
        let saturation_factor = 1.0 + amount;

        for pixel in output.pixels_mut() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];

            // Calculate luminance (perceptual weights)
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;

            // Interpolate between original and grayscale
            pixel[0] = (lum + saturation_factor * (r - lum)).clamp(0.0, 1.0);
            pixel[1] = (lum + saturation_factor * (g - lum)).clamp(0.0, 1.0);
            pixel[2] = (lum + saturation_factor * (b - lum)).clamp(0.0, 1.0);
        }

        Ok(DynamicImage::ImageRgb32F(output))
    } else {
        let rgb = img.to_rgb8();
        let mut output = rgb.clone();

        // amount: -1 = grayscale, 0 = original, 1 = super-saturated
        let saturation_factor = 1.0 + amount;

        for pixel in output.pixels_mut() {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;

            // Calculate luminance (perceptual weights)
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;

            // Interpolate between original and grayscale
            let new_r = (lum + saturation_factor * (r - lum)).clamp(0.0, 1.0);
            let new_g = (lum + saturation_factor * (g - lum)).clamp(0.0, 1.0);
            let new_b = (lum + saturation_factor * (b - lum)).clamp(0.0, 1.0);

            pixel[0] = (new_r * 255.0) as u8;
            pixel[1] = (new_g * 255.0) as u8;
            pixel[2] = (new_b * 255.0) as u8;
        }

        Ok(DynamicImage::ImageRgb8(output))
    }
}

/// Apply gaussian blur using imageproc
fn apply_blur(img: &DynamicImage, sigma: f32) -> Result<DynamicImage> {
    use imageproc::filter::gaussian_blur_f32;

    // Clamp sigma to reasonable range
    let sigma = sigma.clamp(0.1, 50.0);

    // Handle both RGB8 and RGB32F images
    if let Some(rgb_f) = img.as_rgb32f() {
        let blurred = gaussian_blur_f32(rgb_f, sigma);
        Ok(DynamicImage::ImageRgb32F(blurred))
    } else {
        let rgb = img.to_rgb8();
        let blurred = gaussian_blur_f32(&rgb, sigma);
        Ok(DynamicImage::ImageRgb8(blurred))
    }
}

/// Apply sharpen using custom unsharp mask implementation
fn apply_sharpen(img: &DynamicImage, amount: f32) -> Result<DynamicImage> {
    use imageproc::filter::gaussian_blur_f32;

    // Clamp amount to reasonable range
    let amount = amount.clamp(0.1, 10.0);

    // Only works with RGB8 currently
    let rgb = img.to_rgb8();
    let (width, height) = rgb.dimensions();

    // Create blurred version
    let blurred = gaussian_blur_f32(&rgb, 1.0);

    // Apply unsharp mask: result = original + amount * (original - blurred)
    let mut output = rgb.clone();

    // The amount determines how much sharpening to apply
    // amount = 1.0 gives: original + (original - blurred) = 2*original - blurred
    // This is equivalent to classic unsharp mask with amount=1.0
    let blend_factor = (amount - 1.0).clamp(0.0, 2.0);

    for y in 0..height {
        for x in 0..width {
            let orig_pixel = rgb.get_pixel(x, y);
            let blur_pixel = blurred.get_pixel(x, y);
            let out_pixel = output.get_pixel_mut(x, y);

            for i in 0..3 {
                let orig = orig_pixel[i] as f32 / 255.0;
                let blur = blur_pixel[i] as f32;

                // Unsharp mask: enhanced = orig + factor * (orig - blur)
                // For factor=1.0: enhanced = orig + (orig - blur) = 2*orig - blur
                // This creates sharpening effect
                let enhanced: f32 = orig + blend_factor * (orig - blur);
                out_pixel[i] = (enhanced.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
    }

    Ok(DynamicImage::ImageRgb8(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        let mut img = image::RgbImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128u8;
                img.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
        DynamicImage::ImageRgb8(img)
    }

    // =========================================================================
    // Tests for parameter validation
    // =========================================================================

    #[test]
    fn test_valid_params() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: Some(0.5),
            contrast: Some(-0.3),
            saturation: Some(0.2),
            blur: Some(1.5),
            sharpen: Some(1.0),
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        assert!(validate_params(&params).is_ok());
    }

    #[test]
    fn test_brightness_out_of_range_high() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: Some(1.5),
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        let result = validate_params(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Brightness must be between"));
    }

    #[test]
    fn test_brightness_out_of_range_low() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: Some(-1.5),
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        let result = validate_params(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Brightness must be between"));
    }

    #[test]
    fn test_contrast_out_of_range() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: None,
            contrast: Some(2.0),
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        let result = validate_params(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Contrast must be between"));
    }

    #[test]
    fn test_saturation_out_of_range() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: None,
            contrast: None,
            saturation: Some(-2.0),
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        let result = validate_params(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Saturation must be between"));
    }

    #[test]
    fn test_blur_negative() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: Some(-1.0),
            sharpen: None,
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        let result = validate_params(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Blur sigma must be"));
    }

    #[test]
    fn test_sharpen_negative() {
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: Some(-0.5),
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        let result = validate_params(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Sharpen amount must be"));
    }

    #[test]
    fn test_no_adjustments() {
        // All None is valid (no-op)
        let params = AdjustParams {
            input: "test.png".to_string(),
            output: "out.png".to_string(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: None,
            quality: None,
            inline: false,
        };
        assert!(validate_params(&params).is_ok());
    }

    // =========================================================================
    // Tests for brightness adjustment
    // =========================================================================

    #[test]
    fn test_brightness_positive() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.2),
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Brightness adjust failed");
        assert!(result.success);
        assert_eq!(result.operation, "adjust");
    }

    #[test]
    fn test_brightness_negative() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(-0.3),
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Negative brightness failed");
        assert!(result.success);
    }

    #[test]
    fn test_brightness_zero() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.0),
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Zero brightness failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for contrast adjustment
    // =========================================================================

    #[test]
    fn test_contrast_positive() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: Some(0.3),
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Contrast adjust failed");
        assert!(result.success);
    }

    #[test]
    fn test_contrast_negative() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: Some(-0.2),
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Negative contrast failed");
        assert!(result.success);
    }

    #[test]
    fn test_contrast_zero() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: Some(0.0),
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Zero contrast failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for saturation adjustment
    // =========================================================================

    #[test]
    fn test_saturation_positive() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: Some(0.5),
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Saturation adjust failed");
        assert!(result.success);
    }

    #[test]
    fn test_saturation_negative() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: Some(-0.5),
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Negative saturation failed");
        assert!(result.success);
    }

    #[test]
    fn test_saturation_grayscale() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: Some(-1.0), // Full grayscale
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Grayscale saturation failed");
        assert!(result.success);
    }

    #[test]
    fn test_saturation_zero() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: Some(0.0),
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Zero saturation failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for blur adjustment
    // =========================================================================

    #[test]
    fn test_blur_mild() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: Some(1.0),
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Blur adjust failed");
        assert!(result.success);
    }

    #[test]
    fn test_blur_strong() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: Some(5.0),
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Strong blur failed");
        assert!(result.success);
    }

    #[test]
    fn test_blur_zero() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: Some(0.0),
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Zero blur failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for sharpen adjustment
    // =========================================================================

    #[test]
    fn test_sharpen_mild() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: Some(1.0),
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Sharpen adjust failed");
        assert!(result.success);
    }

    #[test]
    fn test_sharpen_strong() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: Some(2.5),
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Strong sharpen failed");
        assert!(result.success);
    }

    #[test]
    fn test_sharpen_zero() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: None,
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: Some(0.0),
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Zero sharpen failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for combined adjustments
    // =========================================================================

    #[test]
    fn test_brightness_and_contrast() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.2),
            contrast: Some(0.1),
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Combined brightness/contrast failed");
        assert!(result.success);
    }

    #[test]
    fn test_all_adjustments() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.1),
            contrast: Some(0.1),
            saturation: Some(0.2),
            blur: Some(0.5),
            sharpen: Some(0.5),
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("All adjustments failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for inline output
    // =========================================================================

    #[test]
    fn test_adjust_with_inline() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path,
            brightness: Some(0.2),
            contrast: None,
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: true,
        };

        let result = execute(params).expect("Inline adjust failed");
        let output = &result.outputs[0];
        assert!(output.data_base64.is_some());
        let b64 = output.data_base64.as_ref().unwrap();
        assert!(!b64.is_empty());
    }

    // =========================================================================
    // Tests for metadata
    // =========================================================================

    #[test]
    fn test_adjust_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.3),
            contrast: Some(-0.2),
            saturation: Some(0.5),
            blur: Some(1.0),
            sharpen: Some(1.5),
            linear_rgb: false,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Adjust failed");

        let metadata = result.metadata.expect("Should have metadata");
        assert_eq!(metadata["original_width"], 200);
        assert_eq!(metadata["original_height"], 100);
        // Use approximate comparison for floating point values
        let brightness = metadata["brightness"].as_f64().unwrap();
        assert!(
            (brightness - 0.3).abs() < 0.001,
            "brightness: expected ~0.3, got {}",
            brightness
        );
        let contrast = metadata["contrast"].as_f64().unwrap();
        assert!(
            (contrast - (-0.2)).abs() < 0.001,
            "contrast: expected ~-0.2, got {}",
            contrast
        );
        let saturation = metadata["saturation"].as_f64().unwrap();
        assert!(
            (saturation - 0.5).abs() < 0.001,
            "saturation: expected ~0.5, got {}",
            saturation
        );
        let blur = metadata["blur"].as_f64().unwrap();
        assert!(
            (blur - 1.0).abs() < 0.001,
            "blur: expected ~1.0, got {}",
            blur
        );
        let sharpen = metadata["sharpen"].as_f64().unwrap();
        assert!(
            (sharpen - 1.5).abs() < 0.001,
            "sharpen: expected ~1.5, got {}",
            sharpen
        );
        assert_eq!(metadata["linear_rgb"], false);
    }

    // =========================================================================
    // Tests for edge cases
    // =========================================================================

    #[test]
    fn test_very_large_image() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(1000, 1000);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.1),
            contrast: None,
            saturation: None,
            blur: Some(2.0),
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Large image adjust failed");
        assert!(result.success);
        assert_eq!(result.outputs[0].width, 1000);
        assert_eq!(result.outputs[0].height, 1000);
    }

    #[test]
    fn test_small_image() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(10, 10);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.5),
            contrast: Some(0.5),
            saturation: None,
            blur: None,
            sharpen: None,
            linear_rgb: true,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Small image adjust failed");
        assert!(result.success);
        assert_eq!(result.outputs[0].width, 10);
        assert_eq!(result.outputs[0].height, 10);
    }

    #[test]
    fn test_linear_rgb_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = AdjustParams {
            input: input_path,
            output: output_path.clone(),
            brightness: Some(0.2),
            contrast: Some(0.1),
            saturation: Some(0.1),
            blur: None,
            sharpen: None,
            linear_rgb: false,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Linear RGB disabled failed");
        assert!(result.success);
    }
}
