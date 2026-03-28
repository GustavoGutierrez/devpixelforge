//! Video thumbnail extraction using FFmpeg CLI.
//!
//! Extracts frames at specific timestamps or percentages of the video duration.

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parameters for thumbnail extraction.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct VideoThumbnailParams {
    /// Source video path
    pub input: String,
    /// Output directory for thumbnails
    pub output_dir: String,
    /// Single timestamp to extract (HH:MM:SS.mmm)
    pub timestamp: Option<String>,
    /// Percentage of video duration (0-100)
    pub percentage: Option<f32>,
    /// Multiple timestamps as array
    pub timestamps: Option<Vec<String>>,
    /// Output width (default: original)
    pub width: Option<u32>,
    /// Output height (default: original)
    pub height: Option<u32>,
    /// Output format (default: jpg)
    pub format: Option<String>,
    /// Output filename pattern (default: "{input}_{timestamp}.{ext}")
    pub pattern: Option<String>,
    /// JPEG quality for thumbnails (1-100, default: 85)
    pub quality: Option<u8>,
}

impl VideoThumbnailParams {
    /// Returns the output format.
    pub fn format(&self) -> &str {
        self.format.as_deref().unwrap_or("jpg")
    }

    /// Returns the JPEG quality.
    pub fn quality(&self) -> u8 {
        self.quality.unwrap_or(85).min(100)
    }

    /// Returns the scale filter string for dimensions.
    pub fn scale_filter(&self) -> Option<String> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some(format!("scale={}:{}", w, h)),
            (Some(w), None) => Some(format!("scale={}:-1", w)),
            (None, Some(h)) => Some(format!("scale=-1:{}", h)),
            (None, None) => None,
        }
    }

    /// Generates the output filename for a given timestamp.
    pub fn generate_filename(&self, ts: &str) -> String {
        let ext = self.format();
        let input_stem = Path::new(&self.input)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("thumb");

        let pattern = self.pattern.as_deref().unwrap_or("{input}_{ts}.{ext}");
        pattern
            .replace("{input}", input_stem)
            .replace("{ts}", ts)
            .replace("{ext}", ext)
    }

    /// Validates the parameters.
    pub fn validate(&self) -> Result<()> {
        if self.timestamp.is_none() && self.percentage.is_none() && self.timestamps.is_none() {
            anyhow::bail!(
                "At least one of 'timestamp', 'percentage', or 'timestamps' must be specified"
            );
        }

        if let Some(pct) = self.percentage {
            if !(0.0..=100.0).contains(&pct) {
                anyhow::bail!("Percentage must be between 0 and 100, got: {}", pct);
            }
        }

        Ok(())
    }
}

/// Executes thumbnail extraction.
pub fn execute(params: VideoThumbnailParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    params.validate()?;

    // Validate input exists
    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input video not found: {}", params.input);
    }

    // Create output directory if needed
    std::fs::create_dir_all(&params.output_dir).context(format!(
        "Failed to create output directory: {}",
        params.output_dir
    ))?;

    let input_path = &params.input;
    let output_dir = &params.output_dir;

    // Collect all timestamps to extract
    let mut timestamps_to_extract: Vec<String> = Vec::new();

    // Add single timestamp
    if let Some(ref ts) = params.timestamp {
        timestamps_to_extract.push(ts.clone());
    }

    // Add percentage as timestamp (requires probing video duration)
    if let Some(pct) = params.percentage {
        let duration = probe_duration(input_path)?;
        let ts = duration * (pct as f64 / 100.0);
        timestamps_to_extract.push(format_timestamp(ts));
    }

    // Add multiple timestamps
    if let Some(ref ts_list) = params.timestamps {
        timestamps_to_extract.extend(ts_list.clone());
    }

    let timestamps_count = timestamps_to_extract.len();

    // Extract each thumbnail
    let mut outputs = Vec::new();
    let mut extracted_count = 0;

    for ts in timestamps_to_extract {
        let filename = params.generate_filename(&ts);
        let output_path = format!("{}/{}", output_dir, filename);

        let mut cmd = FfmpegCommand::new();
        cmd.args(["-ss", &ts])
            .input(input_path)
            .args(["-vframes", "1"]);

        // Add scale filter if specified
        if let Some(ref scale) = params.scale_filter() {
            cmd.args(["-vf", scale]);
        }

        // Output format and quality
        let fmt = params.format();
        if fmt == "jpg" || fmt == "jpeg" {
            // Convert quality to qscale (2 = high quality, 31 = low quality)
            let qscale = 2 + (100 - params.quality()) / 12;
            cmd.args(["-qscale:v", &qscale.to_string()]);
        }

        cmd.output(&output_path).overwrite().args(["-f", "image2"]);

        match cmd.spawn() {
            Ok(mut child) => {
                if child.wait().is_ok() {
                    if let Ok(metadata) = std::fs::metadata(&output_path) {
                        outputs.push(crate::OutputFile {
                            path: output_path,
                            format: fmt.to_string(),
                            width: params.width.unwrap_or(0),
                            height: params.height.unwrap_or(0),
                            size_bytes: metadata.len(),
                            data_base64: None,
                        });
                        extracted_count += 1;
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to extract thumbnail at {}: {}", ts, e);
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(JobResult {
        success: extracted_count > 0,
        operation: "video_thumbnail".into(),
        outputs,
        elapsed_ms,
        metadata: Some(serde_json::json!({
            "timestamps_requested": timestamps_count,
            "timestamps_extracted": extracted_count,
            "format": params.format(),
            "quality": params.quality(),
        })),
    })
}

/// Probes the video file to get its duration in seconds.
fn probe_duration(input_path: &str) -> Result<f64> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            input_path,
        ])
        .output()
        .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse duration")
}

/// Formats a duration in seconds to a timestamp string.
fn format_timestamp(secs: f64) -> String {
    let hours = (secs / 3600.0).floor() as u32;
    let minutes = ((secs % 3600.0) / 60.0).floor() as u32;
    let seconds = secs % 60.0;

    if hours > 0 {
        format!("{:02}:{:02}:{:05.2}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:05.2}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "00:00.00");
        assert_eq!(format_timestamp(5.5), "00:05.50");
        assert_eq!(format_timestamp(65.0), "01:05.00");
        assert_eq!(format_timestamp(3661.5), "01:01:01.50");
    }

    #[test]
    fn test_thumbnail_params_validation() {
        // Valid - timestamp specified
        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: Some("00:01:00".to_string()),
            percentage: None,
            timestamps: None,
            width: None,
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert!(params.validate().is_ok());

        // Valid - percentage specified
        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: None,
            percentage: Some(50.0),
            timestamps: None,
            width: None,
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert!(params.validate().is_ok());

        // Valid - timestamps array specified
        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: None,
            percentage: None,
            timestamps: Some(vec!["00:01:00".to_string(), "00:02:00".to_string()]),
            width: None,
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert!(params.validate().is_ok());

        // Invalid - nothing specified
        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: None,
            percentage: None,
            timestamps: None,
            width: None,
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert!(params.validate().is_err());

        // Invalid - percentage out of range
        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: None,
            percentage: Some(150.0),
            timestamps: None,
            width: None,
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_generate_filename() {
        let params = VideoThumbnailParams {
            input: "/path/to/video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: Some("00:01:00".to_string()),
            percentage: None,
            timestamps: None,
            width: None,
            height: None,
            format: Some("jpg".to_string()),
            pattern: None,
            quality: None,
        };

        let filename = params.generate_filename("00:01:00");
        assert!(filename.contains("video"));
        assert!(filename.contains("00:01:00"));
        assert!(filename.contains("jpg"));
    }

    #[test]
    fn test_generate_filename_custom_pattern() {
        let params = VideoThumbnailParams {
            input: "/path/to/video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: Some("00:01:00".to_string()),
            percentage: None,
            timestamps: None,
            width: None,
            height: None,
            format: Some("png".to_string()),
            pattern: Some("thumb_{ts}_{input}.{ext}".to_string()),
            quality: None,
        };

        let filename = params.generate_filename("00:01:00");
        assert!(filename.starts_with("thumb_"));
        assert!(filename.contains("00:01:00"));
        assert!(filename.ends_with("_video.png"));
    }

    #[test]
    fn test_scale_filter() {
        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: Some("00:01:00".to_string()),
            percentage: None,
            timestamps: None,
            width: Some(320),
            height: Some(240),
            format: None,
            pattern: None,
            quality: None,
        };
        assert_eq!(params.scale_filter(), Some("scale=320:240".to_string()));

        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: Some("00:01:00".to_string()),
            percentage: None,
            timestamps: None,
            width: Some(320),
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert_eq!(params.scale_filter(), Some("scale=320:-1".to_string()));

        let params = VideoThumbnailParams {
            input: "video.mp4".to_string(),
            output_dir: "/tmp".to_string(),
            timestamp: Some("00:01:00".to_string()),
            percentage: None,
            timestamps: None,
            width: None,
            height: None,
            format: None,
            pattern: None,
            quality: None,
        };
        assert_eq!(params.scale_filter(), None);
    }
}
