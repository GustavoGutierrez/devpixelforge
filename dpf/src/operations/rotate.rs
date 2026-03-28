use anyhow::Result;
use image::DynamicImage;
use serde::{Deserialize, Serialize};

use base64::Engine;

use super::{exif_ops, utils};
use crate::{JobResult, OutputFile};

/// Parameters for rotate operation
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RotateParams {
    /// Path to source image
    pub input: String,
    /// Path to output image
    pub output: String,
    /// Rotation angle in degrees: 90, 180, or 270
    pub angle: Option<u16>,
    /// Arbitrary rotation angle (requires interpolation, -360 to 360)
    pub angle_f: Option<f32>,
    /// Flip mode: "horizontal" or "vertical"
    pub flip: Option<String>,
    /// Auto-orient based on EXIF orientation tag
    #[serde(default)]
    pub auto_orient: bool,
    /// Background color for non-90-degree rotations (hex format: "#RRGGBB")
    pub background: Option<String>,
    /// Output format: "png", "jpeg", "webp", "avif" (default: same as input)
    pub format: Option<String>,
    /// Quality JPEG/WebP (1-100, default 85)
    pub quality: Option<u8>,
    /// Generate output inline as base64
    #[serde(default)]
    pub inline: bool,
}

/// Execute rotate operation
pub fn execute(params: RotateParams) -> Result<JobResult> {
    let mut img = utils::load_image(&params.input)?;
    let (orig_w, orig_h) = (img.width(), img.height());
    let quality = params.quality.unwrap_or(85);

    // Apply transformations in order: auto-orient -> rotate -> flip

    // 1. Auto-orient based on EXIF if requested
    if params.auto_orient {
        img = apply_auto_orient(img, &params.input)?;
    }

    // 2. Apply rotation
    if let Some(angle) = params.angle {
        img = apply_rotation(img, angle)?;
    } else if let Some(angle_f) = params.angle_f {
        img = apply_rotation_f(img, angle_f, params.background.as_deref())?;
    }

    // 3. Apply flip
    if let Some(flip) = &params.flip {
        img = apply_flip(img, flip)?;
    }

    let (final_w, final_h) = (img.width(), img.height());

    // Determine output format
    let out_ext = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(&params.input)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    });

    // Save the rotated image
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
        operation: "rotate".into(),
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
            "angle": params.angle,
            "angle_f": params.angle_f,
            "flip": params.flip,
            "auto_orient": params.auto_orient,
        })),
    })
}

/// Apply rotation by fixed angles (90, 180, 270)
fn apply_rotation(img: DynamicImage, angle: u16) -> Result<DynamicImage> {
    match angle {
        90 => Ok(img.rotate90()),
        180 => Ok(img.rotate180()),
        270 => Ok(img.rotate270()),
        0 => Ok(img), // No rotation
        _ => anyhow::bail!(
            "Invalid rotation angle: {}. Supported angles: 0, 90, 180, 270",
            angle
        ),
    }
}

/// Apply rotation by arbitrary angle (requires imageproc)
fn apply_rotation_f(
    img: DynamicImage,
    angle: f32,
    background: Option<&str>,
) -> Result<DynamicImage> {
    // Clamp angle to reasonable range
    if !(-360.0..=360.0).contains(&angle) {
        anyhow::bail!("Angle must be between -360 and 360 degrees");
    }

    // For angles close to 90-degree increments, use the optimized rotation
    let angle_normalized = ((angle % 360.0) + 360.0) % 360.0;

    if (angle_normalized - 0.0).abs() < 0.1 {
        return Ok(img);
    } else if (angle_normalized - 90.0).abs() < 0.1 {
        return Ok(img.rotate90());
    } else if (angle_normalized - 180.0).abs() < 0.1 {
        return Ok(img.rotate180());
    } else if (angle_normalized - 270.0).abs() < 0.1 {
        return Ok(img.rotate270());
    }

    // For arbitrary angles, use imageproc rotation
    use imageproc::geometric_transformations::{rotate_about_center, Interpolation};

    // Parse background color or use transparent/black
    let background_color = parse_background_color(background)?;

    // Convert to RGBA for rotation to preserve transparency
    let rgba_img = img.to_rgba8();

    // Convert angle to radians
    let radians = angle.to_radians();

    // Perform rotation
    let rotated = rotate_about_center(
        &rgba_img,
        radians,
        Interpolation::Bilinear,
        background_color,
    );

    Ok(DynamicImage::ImageRgba8(rotated))
}

/// Apply flip transformation
fn apply_flip(img: DynamicImage, flip: &str) -> Result<DynamicImage> {
    match flip {
        "horizontal" | "h" => Ok(img.fliph()),
        "vertical" | "v" => Ok(img.flipv()),
        _ => anyhow::bail!(
            "Invalid flip mode: {}. Use: horizontal, h, vertical, or v",
            flip
        ),
    }
}

/// Apply auto-orientation based on EXIF orientation tag
fn apply_auto_orient(img: DynamicImage, input_path: &str) -> Result<DynamicImage> {
    // Read EXIF orientation from image file
    let orientation = exif_ops::read_exif_orientation(input_path).unwrap_or(1);

    // Apply transformation based on EXIF orientation
    // Orientation values:
    // 1 = Normal
    // 2 = Flipped horizontally
    // 3 = Rotated 180
    // 4 = Flipped vertically
    // 5 = Rotated 90 CW and flipped horizontally
    // 6 = Rotated 90 CW
    // 7 = Rotated 90 CCW and flipped horizontally
    // 8 = Rotated 90 CCW
    let result = match orientation {
        1 => img,                     // Normal
        2 => img.fliph(),             // Flipped horizontally
        3 => img.rotate180(),         // Rotated 180
        4 => img.flipv(),             // Flipped vertically
        5 => img.rotate90().fliph(),  // Rotated 90 CW and flipped
        6 => img.rotate90(),          // Rotated 90 CW
        7 => img.rotate270().fliph(), // Rotated 90 CCW and flipped
        8 => img.rotate270(),         // Rotated 90 CCW
        _ => img,                     // Unknown, assume normal
    };

    Ok(result)
}

/// Parse background color from hex string
fn parse_background_color(color: Option<&str>) -> Result<image::Rgba<u8>> {
    match color {
        Some(hex) => {
            let hex = hex.trim_start_matches('#');
            if hex.len() != 6 {
                anyhow::bail!("Invalid background color format. Use: #RRGGBB");
            }

            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| anyhow::anyhow!("Invalid red component"))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| anyhow::anyhow!("Invalid green component"))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| anyhow::anyhow!("Invalid blue component"))?;

            Ok(image::Rgba([r, g, b, 255]))
        }
        None => Ok(image::Rgba([0, 0, 0, 0])), // Transparent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use tempfile::TempDir;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        // Create a simple test image with a gradient
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
    // Tests for 90/180/270 degree rotations
    // =========================================================================

    #[test]
    fn test_rotate_90() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100); // Wide image
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: Some(90),
            angle_f: None,
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Rotate failed");

        assert!(result.success);
        assert_eq!(result.operation, "rotate");

        let output = &result.outputs[0];
        assert_eq!(output.width, 100); // Dimensions swapped
        assert_eq!(output.height, 200);
    }

    #[test]
    fn test_rotate_180() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: Some(180),
            angle_f: None,
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Rotate failed");

        let output = &result.outputs[0];
        assert_eq!(output.width, 200); // Dimensions unchanged
        assert_eq!(output.height, 100);
    }

    #[test]
    fn test_rotate_270() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: Some(270),
            angle_f: None,
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Rotate failed");

        let output = &result.outputs[0];
        assert_eq!(output.width, 100); // Dimensions swapped
        assert_eq!(output.height, 200);
    }

    #[test]
    fn test_rotate_0_no_change() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: Some(0),
            angle_f: None,
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Rotate failed");

        let output = &result.outputs[0];
        assert_eq!(output.width, 200);
        assert_eq!(output.height, 100);
    }

    #[test]
    fn test_rotate_invalid_angle() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path,
            angle: Some(45), // Invalid angle
            angle_f: None,
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid rotation angle"));
    }

    // =========================================================================
    // Tests for flip
    // =========================================================================

    #[test]
    fn test_flip_horizontal() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: None,
            angle_f: None,
            flip: Some("horizontal".to_string()),
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Flip failed");

        let output = &result.outputs[0];
        assert_eq!(output.width, 100); // Dimensions unchanged
        assert_eq!(output.height, 100);
    }

    #[test]
    fn test_flip_vertical() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: None,
            angle_f: None,
            flip: Some("vertical".to_string()),
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Flip failed");

        assert!(result.success);
    }

    #[test]
    fn test_flip_horizontal_short() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: None,
            angle_f: None,
            flip: Some("h".to_string()),
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Flip failed");
        assert!(result.success);
    }

    #[test]
    fn test_flip_invalid_mode() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path,
            angle: None,
            angle_f: None,
            flip: Some("diagonal".to_string()),
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid flip mode"));
    }

    // =========================================================================
    // Tests for combined operations
    // =========================================================================

    #[test]
    fn test_rotate_and_flip() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: Some(90),
            angle_f: None,
            flip: Some("horizontal".to_string()),
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Rotate+flip failed");

        let output = &result.outputs[0];
        assert_eq!(output.width, 100);
        assert_eq!(output.height, 200);
    }

    // =========================================================================
    // Tests for arbitrary angle rotation (angle_f)
    // =========================================================================

    #[test]
    fn test_rotate_arbitrary_45() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path.clone(),
            angle: None,
            angle_f: Some(45.0),
            flip: None,
            auto_orient: false,
            background: Some("#FFFFFF".to_string()),
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Arbitrary rotation failed");

        // Output should be larger due to rotation
        assert!(result.success);
        assert!(result.outputs[0].width >= 100);
        assert!(result.outputs[0].height >= 100);
    }

    #[test]
    fn test_rotate_arbitrary_angle_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path,
            angle: None,
            angle_f: Some(400.0), // Too large
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("between -360 and 360"));
    }

    // =========================================================================
    // Tests for helper functions
    // =========================================================================

    #[test]
    fn test_apply_rotation_helper() {
        let img = create_test_image(200, 100);

        let rotated = apply_rotation(img.clone(), 90).unwrap();
        assert_eq!(rotated.width(), 100);
        assert_eq!(rotated.height(), 200);

        let rotated = apply_rotation(img.clone(), 180).unwrap();
        assert_eq!(rotated.width(), 200);
        assert_eq!(rotated.height(), 100);

        let rotated = apply_rotation(img.clone(), 270).unwrap();
        assert_eq!(rotated.width(), 100);
        assert_eq!(rotated.height(), 200);

        let rotated = apply_rotation(img.clone(), 0).unwrap();
        assert_eq!(rotated.width(), 200);
        assert_eq!(rotated.height(), 100);
    }

    #[test]
    fn test_apply_flip_helper() {
        let img = create_test_image(100, 100);

        let flipped = apply_flip(img.clone(), "horizontal").unwrap();
        assert_eq!(flipped.width(), 100);
        assert_eq!(flipped.height(), 100);

        let flipped = apply_flip(img.clone(), "vertical").unwrap();
        assert_eq!(flipped.width(), 100);
        assert_eq!(flipped.height(), 100);

        let result = apply_flip(img.clone(), "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_background_color() {
        let color = parse_background_color(Some("#FF0000")).unwrap();
        assert_eq!(color.0, [255, 0, 0, 255]);

        let color = parse_background_color(Some("#00FF00")).unwrap();
        assert_eq!(color.0, [0, 255, 0, 255]);

        let color = parse_background_color(Some("#0000FF")).unwrap();
        assert_eq!(color.0, [0, 0, 255, 255]);

        // Default (transparent)
        let color = parse_background_color(None).unwrap();
        assert_eq!(color.0, [0, 0, 0, 0]);

        // Invalid format
        let result = parse_background_color(Some("invalid"));
        assert!(result.is_err());
    }

    // =========================================================================
    // Tests for inline output
    // =========================================================================

    #[test]
    fn test_rotate_with_inline() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(100, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path,
            angle: Some(90),
            angle_f: None,
            flip: None,
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: true,
        };

        let result = execute(params).expect("Rotate failed");

        let output = &result.outputs[0];
        assert!(output.data_base64.is_some());
        let b64 = output.data_base64.as_ref().unwrap();
        assert!(!b64.is_empty());
        assert!(!b64.contains(' '));
    }

    // =========================================================================
    // Tests for metadata
    // =========================================================================

    #[test]
    fn test_rotate_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = format!("{}/test_input.png", temp_dir.path().to_str().unwrap());
        let output_path = format!("{}/output.png", temp_dir.path().to_str().unwrap());

        let img = create_test_image(200, 100);
        img.save(&input_path).unwrap();

        let params = RotateParams {
            input: input_path,
            output: output_path,
            angle: Some(90),
            angle_f: None,
            flip: Some("horizontal".to_string()),
            auto_orient: false,
            background: None,
            format: Some("png".to_string()),
            quality: Some(85),
            inline: false,
        };

        let result = execute(params).expect("Rotate failed");

        let metadata = result.metadata.expect("Should have metadata");
        assert_eq!(metadata["original_width"], 200);
        assert_eq!(metadata["original_height"], 100);
        assert_eq!(metadata["angle"], 90);
        assert_eq!(metadata["flip"], "horizontal");
        assert_eq!(metadata["auto_orient"], false);
    }
}
