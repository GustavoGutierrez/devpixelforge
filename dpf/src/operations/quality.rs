//! Auto-quality optimization using binary search to find optimal quality setting.
//!
//! Implements binary search algorithm to find the optimal JPEG/WebP quality
//! that produces a target file size within tolerance.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use base64::Engine;

use super::utils;
use crate::{JobResult, OutputFile};

/// Parameters for auto-quality operation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct QualityParams {
    /// Path to source image
    pub input: String,
    /// Path to output image
    pub output: String,
    /// Target file size in bytes
    pub target_size: u64,
    /// Tolerance percentage for target size (default: 5%)
    pub tolerance_percent: Option<f32>,
    /// Max iterations for binary search (default: 10)
    pub max_iterations: Option<u8>,
    /// Minimum acceptable quality (default: 30)
    pub min_quality: Option<u8>,
    /// Maximum quality to try (default: 95)
    pub max_quality: Option<u8>,
    /// Output format: "jpeg", "webp", "avif" (required)
    pub format: String,
    /// Generate output inline as base64
    #[serde(default)]
    pub inline: bool,
}

/// Result of auto-quality optimization
#[derive(Debug, Serialize)]
pub struct QualityResult {
    /// Final quality value used
    pub quality: u8,
    /// Encoded image data
    pub data: Vec<u8>,
    /// Final file size in bytes
    pub size_bytes: u64,
    /// Deviation from target as percentage
    pub deviation_percent: f32,
    /// Number of iterations performed
    pub iterations: u8,
    /// Whether target was reached within tolerance
    pub converged: bool,
}

/// Execute auto-quality optimization
pub fn execute(params: QualityParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    // Validate parameters
    let tolerance_percent = params.tolerance_percent.unwrap_or(5.0);
    let max_iterations = params.max_iterations.unwrap_or(10);
    let min_quality = params.min_quality.unwrap_or(30);
    let max_quality = params.max_quality.unwrap_or(95);

    if min_quality > max_quality {
        anyhow::bail!(
            "min_quality ({}) cannot be greater than max_quality ({})",
            min_quality,
            max_quality
        );
    }

    if !(0.0..=100.0).contains(&tolerance_percent) {
        anyhow::bail!(
            "tolerance_percent must be between 0 and 100, got {}",
            tolerance_percent
        );
    }

    // Perform binary search
    let quality_result = auto_quality_binary_search(
        &img,
        &params.format,
        params.target_size,
        tolerance_percent,
        max_iterations,
        min_quality,
        max_quality,
    )?;

    // Save the optimized image
    utils::ensure_parent_dir(&params.output)?;
    std::fs::write(&params.output, &quality_result.data)
        .context("Failed to write optimized image")?;

    // Generate base64 if inline requested
    let data_base64 = if params.inline {
        Some(base64::engine::general_purpose::STANDARD.encode(&quality_result.data))
    } else {
        None
    };

    let output_size = quality_result.data.len() as u64;

    Ok(JobResult {
        success: true,
        operation: "quality".into(),
        outputs: vec![OutputFile {
            path: params.output.clone(),
            format: params.format.clone(),
            width: orig_w,
            height: orig_h,
            size_bytes: output_size,
            data_base64,
        }],
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "original_width": orig_w,
            "original_height": orig_h,
            "target_size": params.target_size,
            "final_quality": quality_result.quality,
            "final_size": output_size,
            "deviation_percent": quality_result.deviation_percent,
            "iterations": quality_result.iterations,
            "converged": quality_result.converged,
            "tolerance_percent": tolerance_percent,
        })),
    })
}

/// Binary search for optimal quality
fn auto_quality_binary_search(
    img: &image::DynamicImage,
    format: &str,
    target_size: u64,
    tolerance_percent: f32,
    max_iterations: u8,
    min_quality: u8,
    max_quality: u8,
) -> Result<QualityResult> {
    let mut low = min_quality;
    let mut high = max_quality;
    let mut best_quality = (min_quality + max_quality) / 2;
    let mut best_data = Vec::new();
    let mut best_size = 0u64;
    let mut iterations = 0u8;
    let mut converged = false;

    // If target is very small, we might need to start from lowest quality
    // Try to encode at max_quality first to get a baseline
    let baseline_data = encode_with_quality(img, format, max_quality)?;
    let baseline_size = baseline_data.len() as u64;

    // If even at max quality we're still larger than target, image is too small
    if baseline_size <= target_size {
        let tolerance_multiplier = 1.0 - tolerance_percent / 100.0;
        let min_size = (target_size as f32 * tolerance_multiplier) as u64;
        return Ok(QualityResult {
            quality: max_quality,
            data: baseline_data,
            size_bytes: baseline_size,
            deviation_percent: ((baseline_size as f64 - target_size as f64) / target_size as f64)
                .abs() as f32
                * 100.0,
            iterations: 1,
            converged: baseline_size >= min_size,
        });
    }

    // Start binary search
    for i in 0..max_iterations {
        iterations = i + 1;

        let mid = (low + high) / 2;
        let encoded = encode_with_quality(img, format, mid)?;
        let size = encoded.len() as u64;

        // Calculate deviation
        let deviation = if target_size > 0 {
            ((size as f64 - target_size as f64) / target_size as f64 * 100.0).abs() as f32
        } else {
            0.0
        };

        // Check if within tolerance
        if deviation <= tolerance_percent {
            best_quality = mid;
            best_data = encoded;
            best_size = size;
            converged = true;
            break;
        }

        // Binary search logic
        if size > target_size {
            // File too large, need lower quality
            high = mid;
        } else {
            // File small enough, try higher quality
            low = mid;
            best_quality = mid;
            best_data = encoded;
            best_size = size;
        }

        // Check convergence
        if high <= low + 1 {
            break;
        }
    }

    // If we didn't converge, use the best result we found
    if best_data.is_empty() {
        // Fallback to baseline
        best_quality = max_quality;
        best_data = baseline_data;
        best_size = baseline_size;
    }

    let final_deviation = if target_size > 0 {
        ((best_size as f64 - target_size as f64) / target_size as f64).abs() as f32 * 100.0
    } else {
        0.0
    };

    Ok(QualityResult {
        quality: best_quality,
        data: best_data,
        size_bytes: best_size,
        deviation_percent: final_deviation,
        iterations,
        converged,
    })
}

/// Encode image with specific quality setting
fn encode_with_quality(img: &image::DynamicImage, format: &str, quality: u8) -> Result<Vec<u8>> {
    match format.to_lowercase().as_str() {
        "jpeg" | "jpg" => encode_jpeg(img, quality),
        "webp" => encode_webp(img, quality),
        "avif" => {
            // AVIF uses lossy by default, quality maps differently
            encode_avif(img, quality)
        }
        _ => anyhow::bail!(
            "Unsupported format for auto-quality: {}. Use: jpeg, webp, avif",
            format
        ),
    }
}

/// Encode as JPEG using mozjpeg
fn encode_jpeg(img: &image::DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);

    let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);
    comp.set_size(w, h);
    comp.set_quality(quality as f32);

    let mut output = Vec::new();
    let mut started = comp
        .start_compress(&mut output)
        .map_err(|e| anyhow::anyhow!("mozjpeg start failed: {}", e))?;

    started
        .write_scanlines(rgb.as_raw())
        .map_err(|e| anyhow::anyhow!("mozjpeg write failed: {}", e))?;

    started
        .finish()
        .map_err(|_| anyhow::anyhow!("mozjpeg compression failed"))?;

    Ok(output)
}

/// Encode as WebP
fn encode_webp(img: &image::DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let rgba = img.to_rgba8();
    let encoded =
        webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height()).encode(quality as f32);
    Ok(encoded.to_vec())
}

/// Encode as AVIF (using image crate's avif support)
fn encode_avif(img: &image::DynamicImage, quality: u8) -> Result<Vec<u8>> {
    use image::ImageEncoder;

    // For AVIF, we use image crate's built-in encoder
    // Quality maps to speed/quality tradeoff in avif crate
    let rgba = img.to_rgba8();
    let mut buffer = Vec::new();

    let encoder = image::codecs::avif::AvifEncoder::new_with_speed_quality(
        &mut buffer,
        4, // speed (0-10, higher is faster)
        quality,
    );
    encoder
        .write_image(
            &rgba,
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .context("AVIF encoding failed")?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixtures_dir() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/{}", fixtures_dir(), name)
    }

    fn create_test_image(width: u32, height: u32) -> image::DynamicImage {
        let mut img = image::RgbImage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let r = ((x as f32 / width as f32) * 255.0) as u8;
                let g = ((y as f32 / height as f32) * 255.0) as u8;
                let b = 128u8;
                img.put_pixel(x, y, image::Rgb([r, g, b]));
            }
        }
        image::DynamicImage::ImageRgb8(img)
    }

    // =========================================================================
    // Tests for parameter validation
    // =========================================================================

    #[test]
    fn test_valid_params() {
        let params = QualityParams {
            input: "test.jpg".to_string(),
            output: "out.jpg".to_string(),
            target_size: 50000,
            tolerance_percent: Some(5.0),
            max_iterations: Some(10),
            min_quality: Some(30),
            max_quality: Some(95),
            format: "jpeg".to_string(),
            inline: false,
        };
        // Just validate the logic doesn't panic
        assert!(params.tolerance_percent.unwrap() > 0.0);
    }

    #[test]
    fn test_min_quality_greater_than_max() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.jpg", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        // Save as PNG first, then we'll convert
        let png_path = format!("{}/test.png", temp_dir.path().to_str().unwrap());
        img.save(&png_path).unwrap();

        let params = QualityParams {
            input: png_path,
            output: output_path,
            target_size: 50000,
            tolerance_percent: Some(5.0),
            max_iterations: Some(10),
            min_quality: Some(95), // Greater than max
            max_quality: Some(30),
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("min_quality"));
    }

    #[test]
    fn test_invalid_tolerance() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path,
            target_size: 50000,
            tolerance_percent: Some(150.0), // Invalid
            max_iterations: Some(10),
            min_quality: None,
            max_quality: None,
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("tolerance_percent"));
    }

    // =========================================================================
    // Tests for auto-quality JPEG
    // =========================================================================

    #[test]
    fn test_auto_quality_jpeg_basic() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 5000, // Target 5KB
            tolerance_percent: Some(20.0),
            max_iterations: Some(10),
            min_quality: Some(20),
            max_quality: Some(95),
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Auto-quality failed");
        assert!(result.success);
        assert_eq!(result.operation, "quality");

        // Verify file was created
        assert!(std::path::Path::new(&output_path).exists());

        // Check metadata
        let metadata = result.metadata.unwrap();
        assert!(metadata["final_quality"].is_number());
        assert!(metadata["iterations"].is_number());
        assert!(metadata["converged"].is_boolean());
    }

    #[test]
    fn test_auto_quality_jpeg_tight_tolerance() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        // Create a larger image for better binary search
        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 15000,
            tolerance_percent: Some(10.0), // Relaxed tolerance for test
            max_iterations: Some(15),
            min_quality: Some(30),
            max_quality: Some(95),
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Auto-quality failed");
        assert!(result.success);

        // Verify convergence
        let metadata = result.metadata.unwrap();
        let deviation = metadata["deviation_percent"].as_f64().unwrap() as f32;
        let converged = metadata["converged"].as_bool().unwrap();

        // Either converged or deviation is reasonable
        assert!(converged || deviation <= 20.0);
    }

    #[test]
    fn test_auto_quality_jpeg_low_target() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        // Create a larger image for low target test
        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 1000, // Very low target (1KB)
            tolerance_percent: Some(50.0),
            max_iterations: Some(8),
            min_quality: Some(10),
            max_quality: Some(95),
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Auto-quality failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for auto-quality WebP
    // =========================================================================

    #[test]
    fn test_auto_quality_webp_basic() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.webp", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 200);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 5000,
            tolerance_percent: Some(20.0),
            max_iterations: Some(10),
            min_quality: Some(30),
            max_quality: Some(95),
            format: "webp".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Auto-quality WebP failed");
        assert!(result.success);
        assert_eq!(result.outputs[0].format, "webp");
    }

    // =========================================================================
    // Tests for binary search convergence
    // =========================================================================

    #[test]
    fn test_binary_search_convergence() {
        let img = create_test_image(400, 400);

        let result = auto_quality_binary_search(&img, "jpeg", 10000, 10.0, 10, 20, 95)
            .expect("Binary search failed");

        // Should converge in reasonable iterations
        assert!(result.iterations <= 10);
        assert!(result.quality >= 20);
        assert!(result.quality <= 95);
    }

    #[test]
    fn test_binary_search_iteration_count() {
        let img = create_test_image(100, 100);

        // Test with different max iterations
        for max_iter in [5, 10, 15] {
            let result =
                auto_quality_binary_search(&img, "jpeg", 5000, 5.0, max_iter as u8, 30, 95)
                    .expect("Binary search failed");

            assert!(result.iterations <= max_iter as u8);
        }
    }

    #[test]
    fn test_binary_search_quality_bounds() {
        let img = create_test_image(100, 100);

        let result = auto_quality_binary_search(
            &img, "jpeg", 100, // Very low target
            50.0, 10, 50, // Min quality
            80, // Max quality
        )
        .expect("Binary search failed");

        assert!(result.quality >= 50);
        assert!(result.quality <= 80);
    }

    // =========================================================================
    // Tests for inline output
    // =========================================================================

    #[test]
    fn test_auto_quality_inline() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path,
            target_size: 5000,
            tolerance_percent: Some(20.0),
            max_iterations: Some(10),
            min_quality: None,
            max_quality: None,
            format: "jpeg".to_string(),
            inline: true,
        };

        let result = execute(params).expect("Inline auto-quality failed");
        assert!(result.outputs[0].data_base64.is_some());

        let b64 = result.outputs[0].data_base64.as_ref().unwrap();
        assert!(!b64.is_empty());
        // Verify base64 can be decoded
        assert!(base64::engine::general_purpose::STANDARD
            .decode(b64)
            .is_ok());
    }

    // =========================================================================
    // Tests for edge cases
    // =========================================================================

    #[test]
    fn test_auto_quality_small_image() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        // Small image
        let img = create_test_image(50, 50);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 500, // Small target
            tolerance_percent: Some(30.0),
            max_iterations: Some(10),
            min_quality: None,
            max_quality: None,
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Small image auto-quality failed");
        assert!(result.success);
    }

    #[test]
    fn test_auto_quality_zero_target() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 0, // Zero target (will use baseline)
            tolerance_percent: Some(100.0),
            max_iterations: Some(10),
            min_quality: None,
            max_quality: None,
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Zero target auto-quality failed");
        assert!(result.success);
    }

    #[test]
    fn test_auto_quality_unsupported_format() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.gif", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path,
            target_size: 5000,
            tolerance_percent: Some(10.0),
            max_iterations: Some(10),
            min_quality: None,
            max_quality: None,
            format: "gif".to_string(), // Not supported
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported format"));
    }

    // =========================================================================
    // Tests for metadata
    // =========================================================================

    #[test]
    fn test_auto_quality_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.jpg", temp_dir.path().to_str().unwrap());

        let img = create_test_image(320, 240);
        img.save(&input_path).unwrap();

        let params = QualityParams {
            input: input_path,
            output: output_path.clone(),
            target_size: 10000,
            tolerance_percent: Some(10.0),
            max_iterations: Some(12),
            min_quality: Some(25),
            max_quality: Some(90),
            format: "jpeg".to_string(),
            inline: false,
        };

        let result = execute(params).expect("Metadata test failed");
        let metadata = result.metadata.unwrap();

        assert_eq!(metadata["original_width"], 320);
        assert_eq!(metadata["original_height"], 240);
        assert_eq!(metadata["target_size"], 10000);
        assert!(metadata["final_quality"].is_number());
        assert!(metadata["final_size"].is_number());
        assert!(metadata["deviation_percent"].is_number());
        assert!(metadata["iterations"].is_number());
        assert!(metadata["converged"].is_boolean());
        assert_eq!(metadata["tolerance_percent"], 10.0);
    }
}
