//! Video resize with aspect ratio maintenance using FFmpeg CLI.
//!
//! Provides scale-based resizing with optional max width/height constraints.

use crate::JobResult;
use anyhow::Result;
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

/// Parameters for video resizing.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct VideoResizeParams {
    /// Source video path
    pub input: String,
    /// Destination video path
    pub output: String,
    /// Target width in pixels (optional)
    pub width: Option<u32>,
    /// Target height in pixels (optional)
    pub height: Option<u32>,
    /// Scale mode: "fit", "fill", "limit", "scale" (default: "fit")
    pub mode: Option<String>,
    /// Apply fast bilinear scaling instead of full quality
    #[serde(default)]
    pub fast: bool,
}

impl VideoResizeParams {
    /// Returns the resize mode.
    pub fn parse_mode(&self) -> &'static str {
        match self.mode.as_deref() {
            Some("fit") => "fit",
            Some("fill") => "fill",
            Some("limit") => "limit",
            Some("scale") => "scale",
            _ => "fit",
        }
    }

    /// Validates that at least one dimension is specified.
    pub fn validate(&self) -> Result<()> {
        if self.width.is_none() && self.height.is_none() {
            anyhow::bail!("At least one of 'width' or 'height' must be specified");
        }
        Ok(())
    }
}

/// Executes video resizing using FFmpeg CLI.
pub fn execute(params: VideoResizeParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    params.validate()?;

    let input_path = &params.input;
    let output_path = &params.output;

    // Validate input exists
    if !std::path::Path::new(input_path).exists() {
        anyhow::bail!("Input video not found: {}", input_path);
    }

    let target_width = params.width;
    let target_height = params.height;

    // Build scale filter based on mode
    let scale_filter = build_scale_filter(params.parse_mode(), target_width, target_height);

    // Build FFmpeg command
    let mut cmd = FfmpegCommand::new();
    cmd.input(input_path)
        .output(output_path)
        .overwrite()
        .args(["-vf", &scale_filter]);

    // Use fast preset for speed
    if params.fast {
        cmd.args(["-c:v", "libx264"]).args(["-preset", "ultrafast"]);
    } else {
        cmd.args(["-c:v", "libx264"]).args(["-preset", "fast"]);
    }

    cmd.args(["-c:a", "copy"]);

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn FFmpeg: {}", e))?;
    let result = child.wait();

    if let Err(e) = result {
        anyhow::bail!("FFmpeg resize failed: {}", e);
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    // Probe output dimensions
    let (out_w, out_h) = probe_dimensions(output_path)
        .unwrap_or((target_width.unwrap_or(0), target_height.unwrap_or(0)));

    Ok(JobResult {
        success: true,
        operation: "video_resize".into(),
        outputs: vec![crate::OutputFile {
            path: output_path.clone(),
            format: std::path::Path::new(output_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4")
                .to_string(),
            width: out_w,
            height: out_h,
            size_bytes: file_size,
            data_base64: None,
        }],
        elapsed_ms,
        metadata: Some(serde_json::json!({
            "mode": params.parse_mode(),
            "target_width": target_width,
            "target_height": target_height,
            "fast": params.fast,
        })),
    })
}

/// Builds the FFmpeg scale filter string.
fn build_scale_filter(mode: &str, target_w: Option<u32>, target_h: Option<u32>) -> String {
    match mode {
        "fit" => {
            // Scale to fit within bounds, maintain aspect ratio
            let w = target_w.unwrap_or(0);
            let h = target_h.unwrap_or(0);
            if w > 0 && h > 0 {
                format!(
                    "scale={}:{}:force_original_aspect_ratio=decrease,scale='min({},iw)':min({},ih)':force_original_aspect_ratio=decrease",
                    w, h, w, h
                )
            } else if w > 0 {
                format!("scale={}:-2", w)
            } else {
                format!("scale=-2:{}", h)
            }
        }
        "fill" => {
            // Scale to fill, crop excess
            let w = target_w.unwrap_or(0);
            let h = target_h.unwrap_or(0);
            if w > 0 && h > 0 {
                format!(
                    "scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}",
                    w, h, w, h
                )
            } else if w > 0 {
                format!("scale={}:-2", w)
            } else {
                format!("scale=-2:{}", h)
            }
        }
        "limit" => {
            // Only downscale, don't upscale
            let w = target_w.unwrap_or(0);
            let h = target_h.unwrap_or(0);
            if w > 0 && h > 0 {
                format!(
                    "scale='min({},iw)':min({},ih)':force_original_aspect_ratio=decrease",
                    w, h
                )
            } else if w > 0 {
                format!("scale='min({},iw)':-2", w)
            } else {
                format!("scale=-2:'min({},ih)'", h)
            }
        }
        "scale" => {
            // Direct scale to exact dimensions
            let w = target_w.unwrap_or(0);
            let h = target_h.unwrap_or(0);
            format!("scale={}:{}", w, h)
        }
        _ => {
            let w = target_w.unwrap_or(0);
            let h = target_h.unwrap_or(0);
            if w > 0 && h > 0 {
                format!("scale={}:{}", w, h)
            } else if w > 0 {
                format!("scale={}:-2", w)
            } else {
                format!("scale=-2:{}", h)
            }
        }
    }
}

/// Probes the output file to get video dimensions.
fn probe_dimensions(path: &str) -> Result<(u32, u32)> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=s=x:p=0",
            path,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        anyhow::bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let dims = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = dims.trim().split('x').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid dimensions: {}", dims);
    }

    let width = parts[0].parse().unwrap_or(0);
    let height = parts[1].parse().unwrap_or(0);

    Ok((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_validation() {
        let valid_params = VideoResizeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            width: Some(640),
            height: None,
            mode: None,
            fast: false,
        };
        assert!(valid_params.validate().is_ok());

        let invalid_params = VideoResizeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            width: None,
            height: None,
            mode: None,
            fast: false,
        };
        assert!(invalid_params.validate().is_err());
    }

    #[test]
    fn test_mode_parsing() {
        let params = VideoResizeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            width: Some(640),
            height: None,
            mode: Some("fill".to_string()),
            fast: false,
        };
        assert_eq!(params.parse_mode(), "fill");

        let default_params = VideoResizeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            width: Some(640),
            height: None,
            mode: None,
            fast: false,
        };
        assert_eq!(default_params.parse_mode(), "fit");
    }

    #[test]
    fn test_scale_filter_fit() {
        let filter = build_scale_filter("fit", Some(640), Some(480));
        assert!(filter.contains("scale"));
        assert!(filter.contains("force_original_aspect_ratio"));
    }

    #[test]
    fn test_scale_filter_width_only() {
        let filter = build_scale_filter("fit", Some(640), None);
        assert!(filter.contains("scale=640"));
        assert!(filter.contains("-2")); // Maintain aspect
    }

    #[test]
    fn test_scale_filter_height_only() {
        let filter = build_scale_filter("fit", None, Some(480));
        assert!(filter.contains("scale"));
        assert!(filter.contains("-2"));
    }

    #[test]
    fn test_resize_input_not_found() {
        let params = VideoResizeParams {
            input: "/nonexistent/video.mp4".to_string(),
            output: "/tmp/out.mp4".to_string(),
            width: Some(640),
            height: None,
            mode: None,
            fast: false,
        };

        let result = execute(params);
        assert!(result.is_err());
    }

    // =========================================================================
    // JSON Serialization Tests
    // =========================================================================

    #[test]
    fn test_video_resize_params_json_serialize() {
        let params = VideoResizeParams {
            input: "input.mp4".to_string(),
            output: "output.mp4".to_string(),
            width: Some(1280),
            height: Some(720),
            mode: Some("fit".to_string()),
            fast: true,
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        assert!(json.contains("\"input\":\"input.mp4\""));
        assert!(json.contains("\"width\":1280"));
        assert!(json.contains("\"height\":720"));
        assert!(json.contains("\"mode\":\"fit\""));
        assert!(json.contains("\"fast\":true"));
    }

    #[test]
    fn test_video_resize_params_json_deserialize() {
        let json = r#"{
            "input": "input.mp4",
            "output": "output_720p.mp4",
            "width": 1280,
            "height": 720,
            "mode": "fill",
            "fast": false
        }"#;

        let params: VideoResizeParams = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(params.input, "input.mp4");
        assert_eq!(params.output, "output_720p.mp4");
        assert_eq!(params.width, Some(1280));
        assert_eq!(params.height, Some(720));
        assert_eq!(params.mode, Some("fill".to_string()));
        assert_eq!(params.fast, false);
    }

    #[test]
    fn test_video_resize_params_json_roundtrip() {
        let params = VideoResizeParams {
            input: "in.mp4".to_string(),
            output: "out.webm".to_string(),
            width: Some(640),
            height: None,
            mode: Some("limit".to_string()),
            fast: false,
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        let restored: VideoResizeParams = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(restored.input, params.input);
        assert_eq!(restored.output, params.output);
        assert_eq!(restored.width, params.width);
        assert_eq!(restored.height, params.height);
        assert_eq!(restored.mode, params.mode);
        assert_eq!(restored.fast, params.fast);
    }

    #[test]
    fn test_video_resize_params_defaults() {
        // Minimal JSON with only required fields
        let json = r#"{"input": "in.mp4", "output": "out.mp4", "width": 320}"#;
        let params: VideoResizeParams = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(params.input, "in.mp4");
        assert_eq!(params.width, Some(320));
        assert!(params.height.is_none());
        assert!(params.mode.is_none());
        assert_eq!(params.fast, false); // Default from serde(default)
    }

    #[test]
    fn test_video_resize_modes_serialization() {
        let modes = vec!["fit", "fill", "limit", "scale"];

        for mode in modes {
            let params = VideoResizeParams {
                input: "in.mp4".to_string(),
                output: format!("out_{}.mp4", mode),
                width: Some(1920),
                height: Some(1080),
                mode: Some(mode.to_string()),
                fast: false,
            };

            let json = serde_json::to_string(&params).expect("Should serialize");
            let restored: VideoResizeParams =
                serde_json::from_str(&json).expect("Should deserialize");
            assert_eq!(restored.mode, Some(mode.to_string()));
        }
    }
}
