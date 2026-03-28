//! Srcset generation for responsive images.
//!
//! Generates multiple image variants at specified widths and creates
//! HTML img/srcset snippets for use in web pages.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::utils;
use crate::{JobResult, OutputFile};

/// Parameters for srcset operation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SrcsetParams {
    /// Path to source image
    pub input: String,
    /// Output directory for generated variants
    pub output_dir: String,
    /// Widths to generate (e.g., [320, 640, 960, 1280, 1920])
    pub widths: Vec<u32>,
    /// Density descriptors to include (default: [1, 2])
    pub densities: Option<Vec<f32>>,
    /// Output format: "png", "jpeg", "webp", "avif" (default: same as input)
    pub format: Option<String>,
    /// Quality for JPEG/WebP (default: 85)
    pub quality: Option<u8>,
    /// Generate HTML img/srcset snippet
    #[serde(default)]
    pub generate_html: bool,
    /// Use linear RGB for resize (better quality)
    #[serde(default)]
    pub linear_rgb: bool,
}

/// Output file with width and descriptor info
#[derive(Debug, Serialize)]
pub struct SrcsetOutput {
    /// Width of this variant
    pub width: u32,
    /// File path relative to output_dir
    pub file: String,
    /// Descriptor string (e.g., "320w" or "2x")
    pub descriptor: String,
    /// File size in bytes
    pub size_bytes: u64,
}

/// Result of srcset generation
#[derive(Debug, Serialize)]
pub struct SrcsetResult {
    /// All generated output files
    pub outputs: Vec<SrcsetOutput>,
    /// Complete srcset attribute value
    pub srcset: String,
    /// HTML snippet for img tag (if generate_html is true)
    pub html_snippet: Option<String>,
}

/// Execute srcset generation
pub fn execute(params: SrcsetParams) -> Result<JobResult> {
    let img = utils::load_image(&params.input)?;
    let (src_w, src_h) = (img.width(), img.height());
    let quality = params.quality.unwrap_or(85);
    let densities = params.densities.unwrap_or_else(|| vec![1.0, 2.0]);

    // Validate inputs
    if params.widths.is_empty() {
        anyhow::bail!("At least one width must be specified");
    }

    if densities.is_empty() {
        anyhow::bail!("At least one density must be specified");
    }

    // Determine output format
    let out_ext = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&params.input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    });

    // Get base filename without extension
    let base_filename = std::path::Path::new(&params.input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");

    // Ensure output directory exists
    utils::ensure_parent_dir(&format!("{}/.", params.output_dir))?;

    // Generate all variants in parallel
    let mut all_outputs: Vec<SrcsetOutput> = Vec::new();
    let mut srcset_parts: Vec<String> = Vec::new();

    // For each width
    for &target_w in &params.widths {
        // Skip if target is larger than source
        if target_w > src_w {
            continue;
        }

        // Calculate height maintaining aspect ratio
        let ratio = src_h as f32 / src_w as f32;
        let target_h = (target_w as f32 * ratio) as u32;

        // Resize image
        let resized = if params.linear_rgb {
            // For linear RGB, resize in floating point space for better quality
            // Convert to RGB8 for resize operation, then convert back
            let rgb = img.to_rgb8();
            let resized = image::imageops::resize(
                &rgb,
                target_w,
                target_h,
                image::imageops::FilterType::Lanczos3,
            );
            image::DynamicImage::ImageRgb8(resized)
        } else {
            img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
        };

        // For each density at this width
        for &density in &densities {
            // Calculate actual output width based on density
            let actual_w = (target_w as f32 * density) as u32;
            let actual_h = (target_h as f32 * density) as u32;

            // Final resize if density != 1.0
            let final_img = if density != 1.0 {
                if params.linear_rgb {
                    let rgb = resized.to_rgb8();
                    let resized2 = image::imageops::resize(
                        &rgb,
                        actual_w,
                        actual_h,
                        image::imageops::FilterType::Lanczos3,
                    );
                    image::DynamicImage::ImageRgb8(resized2)
                } else {
                    resized.resize_exact(actual_w, actual_h, image::imageops::FilterType::Lanczos3)
                }
            } else {
                resized.clone()
            };

            // Generate filename
            let filename = if density == 1.0 {
                format!("{}-{}w.{}", base_filename, target_w, out_ext)
            } else {
                format!(
                    "{}-{}w-{}x.{}",
                    base_filename,
                    target_w,
                    density
                        .to_string()
                        .trim_end_matches('0')
                        .trim_end_matches('.'),
                    out_ext
                )
            };

            let output_path = format!("{}/{}", params.output_dir, filename);

            // Save image
            utils::save_image(&final_img, &output_path, out_ext, quality)?;

            let file_size = utils::file_size(&output_path);

            // Create descriptor
            let descriptor = if density == 1.0 {
                format!("{}w", target_w)
            } else {
                format!(
                    "{}x",
                    density
                        .to_string()
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                )
            };

            // Add to srcset string
            srcset_parts.push(format!("{} {}", filename, descriptor));

            all_outputs.push(SrcsetOutput {
                width: actual_w,
                file: filename.clone(),
                descriptor,
                size_bytes: file_size,
            });
        }
    }

    // Build srcset string
    let srcset_string = srcset_parts.join(", ");

    // Generate HTML snippet if requested
    let html_snippet = if params.generate_html {
        let largest_width = params.widths.iter().max().copied().unwrap_or(src_w);
        let largest_file = all_outputs
            .iter()
            .max_by_key(|o| o.width)
            .map(|o| o.file.clone())
            .unwrap_or_else(|| format!("{}-{}.{}", base_filename, src_w, out_ext));

        Some(format!(
            r#"<img src="{}" srcset="{}" sizes="(max-width: {}px) 100vw, {}px" alt="">"#,
            largest_file, srcset_string, largest_width, largest_width
        ))
    } else {
        None
    };

    // Count unique widths (before density multiplication)
    let unique_widths = params.widths.iter().filter(|&&w| w <= src_w).count();

    Ok(JobResult {
        success: true,
        operation: "srcset".into(),
        outputs: all_outputs
            .iter()
            .map(|o| OutputFile {
                path: format!("{}/{}", params.output_dir, o.file),
                format: out_ext.to_string(),
                width: o.width,
                height: (o.width as f32 / (src_w as f32 / src_h as f32)) as u32,
                size_bytes: o.size_bytes,
                data_base64: None,
            })
            .collect(),
        elapsed_ms: 0,
        metadata: Some(serde_json::json!({
            "source_width": src_w,
            "source_height": src_h,
            "unique_widths_requested": params.widths.len(),
            "unique_widths_generated": unique_widths,
            "total_variants": all_outputs.len(),
            "densities": densities,
            "format": out_ext,
            "quality": quality,
            "srcset": srcset_string,
            "html_snippet": html_snippet,
        })),
    })
}

/// Generate srcset string from widths and densities
pub fn generate_srcset_string(
    widths: &[u32],
    densities: &[f32],
    base_filename: &str,
    ext: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    for &width in widths {
        for &density in densities {
            let descriptor = if density == 1.0 {
                format!("{}w", width)
            } else {
                format!("{}x", density)
            };

            let filename = if density == 1.0 {
                format!("{}-{}w.{}", base_filename, width, ext)
            } else {
                format!("{}-{}w-{}x.{}", base_filename, width, density, ext)
            };

            parts.push(format!("{} {}", filename, descriptor));
        }
    }

    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;
    use tempfile::TempDir;

    fn fixtures_dir() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/{}", fixtures_dir(), name)
    }

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
    // Tests for basic srcset generation
    // =========================================================================

    #[test]
    fn test_srcset_single_width() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: None,
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Srcset generation failed");
        assert!(result.success);
        assert_eq!(result.operation, "srcset");

        // Should have 2 variants (1x and 2x density)
        assert_eq!(result.outputs.len(), 2);

        // Check files exist
        for output in &result.outputs {
            let full_path = &output.path;
            assert!(std::path::Path::new(full_path).exists());
        }
    }

    #[test]
    fn test_srcset_multiple_widths() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(1920, 1080);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320, 640, 1024, 1920],
            densities: None,
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Multiple widths srcset failed");
        assert!(result.success);

        // 4 widths × 2 densities = 8 variants
        assert_eq!(result.outputs.len(), 8);
    }

    // =========================================================================
    // Tests for different formats
    // =========================================================================

    #[test]
    fn test_srcset_jpeg_output() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320, 640],
            densities: None,
            format: Some("jpeg".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("JPEG srcset failed");
        assert!(result.success);

        // All outputs should be jpeg format
        for output in &result.outputs {
            assert_eq!(output.format, "jpeg");
            assert!(output.path.ends_with(".jpg") || output.path.ends_with(".jpeg"));
        }
    }

    #[test]
    fn test_srcset_webp_output() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: None,
            format: Some("webp".to_string()),
            quality: Some(80),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("WebP srcset failed");
        assert!(result.success);

        for output in &result.outputs {
            assert_eq!(output.format, "webp");
        }
    }

    // =========================================================================
    // Tests for density descriptors
    // =========================================================================

    #[test]
    fn test_srcset_single_density() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320, 640],
            densities: Some(vec![1.0]), // Only 1x
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Single density srcset failed");

        // 2 widths × 1 density = 2 variants
        assert_eq!(result.outputs.len(), 2);
    }

    #[test]
    fn test_srcset_multiple_densities() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: Some(vec![1.0, 2.0, 3.0]),
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Multiple densities srcset failed");

        // 1 width × 3 densities = 3 variants
        assert_eq!(result.outputs.len(), 3);
    }

    // =========================================================================
    // Tests for HTML generation
    // =========================================================================

    #[test]
    fn test_srcset_html_generation() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(1920, 1080);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320, 640, 1024],
            densities: None,
            format: Some("webp".to_string()),
            quality: Some(85),
            generate_html: true,
            linear_rgb: false,
        };

        let result = execute(params).expect("HTML srcset failed");

        let metadata = result.metadata.unwrap();
        let html_snippet = metadata["html_snippet"].as_str().unwrap();

        // Check HTML contains expected elements
        assert!(html_snippet.contains("<img"));
        assert!(html_snippet.contains("srcset="));
        assert!(html_snippet.contains("320w"));
        assert!(html_snippet.contains("640w"));
        assert!(html_snippet.contains("1024w"));
    }

    #[test]
    fn test_srcset_no_html_by_default() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: None,
            format: None,
            quality: None,
            generate_html: false, // Default
            linear_rgb: false,
        };

        let result = execute(params).expect("Srcset failed");

        let metadata = result.metadata.unwrap();
        assert!(metadata["html_snippet"].is_null());
    }

    // =========================================================================
    // Tests for metadata
    // =========================================================================

    #[test]
    fn test_srcset_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(1920, 1080);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320, 640, 1024],
            densities: Some(vec![1.0, 2.0]),
            format: Some("jpeg".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: true,
        };

        let result = execute(params).expect("Metadata test failed");

        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["source_width"], 1920);
        assert_eq!(metadata["source_height"], 1080);
        assert_eq!(metadata["unique_widths_requested"], 3);
        assert_eq!(metadata["unique_widths_generated"], 3);
        assert_eq!(metadata["total_variants"], 6); // 3 widths × 2 densities
                                                   // Verify densities array exists and has correct values
        let densities = metadata["densities"].as_array().unwrap();
        assert_eq!(densities.len(), 2);
        assert_eq!(densities[0].as_f64().unwrap(), 1.0);
        assert_eq!(densities[1].as_f64().unwrap(), 2.0);
        assert_eq!(metadata["format"], "jpeg");
        assert_eq!(metadata["quality"], 85);
        assert!(metadata["srcset"].is_string());
        assert!(metadata["srcset"].as_str().unwrap().contains("320w"));
    }

    // =========================================================================
    // Tests for srcset string generation
    // =========================================================================

    #[test]
    fn test_generate_srcset_string_basic() {
        let srcset = generate_srcset_string(&[320, 640], &[1.0, 2.0], "image", "webp");

        // Debug: print actual output
        eprintln!("Generated srcset: {}", srcset);

        assert!(srcset.contains("image-320w.webp 320w"));
        assert!(srcset.contains("image-320w-2x.webp 2x"));
        assert!(srcset.contains("image-640w.webp 640w"));
        assert!(srcset.contains("image-640w-2x.webp 2x"));
    }

    #[test]
    fn test_generate_srcset_string_single_density() {
        let srcset = generate_srcset_string(&[320, 640, 1024], &[1.0], "photo", "jpg");

        assert!(srcset.contains("photo-320w.jpg 320w"));
        assert!(srcset.contains("photo-640w.jpg 640w"));
        assert!(srcset.contains("photo-1024w.jpg 1024w"));
        assert!(!srcset.contains("1x")); // No 1x descriptor for 1.0 density
    }

    // =========================================================================
    // Tests for edge cases
    // =========================================================================

    #[test]
    fn test_srcset_empty_widths() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![], // Empty
            densities: None,
            format: None,
            quality: None,
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("width"));
    }

    #[test]
    fn test_srcset_empty_densities() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: Some(vec![]), // Empty
            format: None,
            quality: None,
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("density"));
    }

    #[test]
    fn test_srcset_width_larger_than_source() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        // Small source image
        let img = create_test_image(400, 300);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![200, 320, 640, 1024], // 640 and 1024 are larger than source
            densities: None,
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Srcset failed");

        // Should only generate 200w and 320w (the ones smaller than source)
        assert_eq!(result.outputs.len(), 4); // 2 widths × 2 densities

        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["unique_widths_requested"], 4);
        assert_eq!(metadata["unique_widths_generated"], 2);
    }

    #[test]
    fn test_srcset_linear_rgb() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: None,
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: true, // Enable linear RGB
        };

        let result = execute(params).expect("Linear RGB srcset failed");
        assert!(result.success);
    }

    // =========================================================================
    // Tests for file naming
    // =========================================================================

    #[test]
    fn test_srcset_filename_format() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: Some(vec![1.0, 2.0]),
            format: Some("webp".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Filename test failed");

        // Check filenames
        let files: Vec<&str> = result
            .outputs
            .iter()
            .map(|o| o.path.split('/').last().unwrap())
            .collect();

        // 1x density: test_input-320w.webp
        assert!(files.contains(&"test_input-320w.webp"));
        // 2x density: test_input-320w-2x.webp
        assert!(files.contains(&"test_input-320w-2x.webp"));
    }

    #[test]
    fn test_srcset_complex_filename() {
        let temp_dir = TempDir::new().unwrap();
        // Image with dots in name
        let input_path = format!("{}/my.image.test.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        let img = create_test_image(800, 600);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: None,
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Complex filename test failed");
        assert!(result.success);

        // Filename should be based on stem (my.image.test)
        let files: Vec<&str> = result
            .outputs
            .iter()
            .map(|o| o.path.split('/').last().unwrap())
            .collect();

        assert!(files[0].starts_with("my.image.test-320w"));
    }

    // =========================================================================
    // Tests for dimensions
    // =========================================================================

    #[test]
    fn test_srcset_aspect_ratio_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        // 16:9 aspect ratio
        let img = create_test_image(1920, 1080);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![640],
            densities: Some(vec![1.0]),
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Aspect ratio test failed");

        // 640w should have height of 360 (16:9 ratio)
        assert_eq!(result.outputs[0].width, 640);
        assert_eq!(result.outputs[0].height, 360);
    }

    #[test]
    fn test_srcset_different_aspect_ratios() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_dir = temp_dir.path().to_str().unwrap();

        // Portrait image
        let img = create_test_image(600, 800);
        img.save(&input_path).unwrap();

        let params = SrcsetParams {
            input: input_path,
            output_dir: output_dir.to_string(),
            widths: vec![320],
            densities: Some(vec![1.0]),
            format: Some("png".to_string()),
            quality: Some(85),
            generate_html: false,
            linear_rgb: false,
        };

        let result = execute(params).expect("Portrait aspect ratio test failed");

        // 320w should have height of ~427 (600:800 = 3:4, so 320:427)
        assert_eq!(result.outputs[0].width, 320);
        assert!(result.outputs[0].height > 300 && result.outputs[0].height < 500);
    }
}
