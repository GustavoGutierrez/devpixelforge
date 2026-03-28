//! Video trimming with start/end timestamps using FFmpeg CLI.
//!
//! Extracts a segment from a video using precise timestamp seeking.

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

/// Parameters for video trimming.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct VideoTrimParams {
    /// Source video path
    pub input: String,
    /// Destination video path
    pub output: String,
    /// Start timestamp (HH:MM:SS or seconds as float)
    pub start: String,
    /// End timestamp (HH:MM:SS or seconds as float)
    pub end: String,
    /// Output format override (default: inferred from output extension)
    pub format: Option<String>,
}

impl VideoTrimParams {
    /// Parses a timestamp string to seconds.
    ///
    /// Supports formats:
    /// - "1.5" (seconds)
    /// - "1:30" (MM:SS)
    /// - "1:30:45" (HH:MM:SS)
    /// - "00:01:30.500" (HH:MM:SS.mmm)
    pub fn parse_timestamp(&self, ts: &str) -> Result<f64> {
        let ts = ts.trim();

        // Try seconds as float
        if let Ok(secs) = ts.parse::<f64>() {
            return Ok(secs);
        }

        // Try HH:MM:SS.mmm format
        let parts: Vec<&str> = ts.split(':').collect();
        if parts.is_empty() {
            anyhow::bail!("Invalid timestamp format: {}", ts);
        }

        let seconds: f64 = parts
            .last()
            .unwrap()
            .parse()
            .context(format!("Invalid seconds in timestamp: {}", ts))?;

        let minutes: f64 = if parts.len() >= 2 {
            parts[parts.len() - 2].parse().unwrap_or(0.0)
        } else {
            0.0
        };

        let hours: f64 = if parts.len() >= 3 {
            parts[parts.len() - 3].parse().unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(hours * 3600.0 + minutes * 60.0 + seconds)
    }

    /// Validates the timestamp parameters.
    pub fn validate(&self) -> Result<()> {
        let start_secs = self.parse_timestamp(&self.start)?;
        let end_secs = self.parse_timestamp(&self.end)?;

        if start_secs < 0.0 {
            anyhow::bail!("Start timestamp cannot be negative: {}", self.start);
        }

        if end_secs <= start_secs {
            anyhow::bail!(
                "End timestamp ({}) must be greater than start ({})",
                self.end,
                self.start
            );
        }

        if end_secs > 86400.0 * 7.0 {
            // 7 days max
            anyhow::bail!("End timestamp exceeds maximum duration (7 days)");
        }

        Ok(())
    }
}

/// Executes video trimming using FFmpeg CLI.
pub fn execute(params: VideoTrimParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    params.validate()?;

    // Validate input exists
    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input video not found: {}", params.input);
    }

    let start_secs = params.parse_timestamp(&params.start)?;
    let end_secs = params.parse_timestamp(&params.end)?;
    let duration = end_secs - start_secs;

    let input_path = &params.input;
    let output_path = &params.output;

    let output_format = params.format.as_deref().unwrap_or_else(|| {
        std::path::Path::new(output_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
    });

    // Build FFmpeg command for trimming
    // Note: we use -ss before -i for fast seeking
    let mut cmd = FfmpegCommand::new();
    cmd.args(["-ss", &format!("{:.3}", start_secs)])
        .input(input_path)
        .args(["-t", &format!("{:.3}", duration)])
        .args(["-c", "copy"])
        .output(output_path)
        .overwrite()
        .args(["-f", output_format]);

    let mut child = cmd.spawn().context("Failed to spawn FFmpeg process")?;
    let result = child.wait();

    if let Err(e) = result {
        anyhow::bail!("FFmpeg trim failed: {}", e);
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    Ok(JobResult {
        success: true,
        operation: "video_trim".into(),
        outputs: vec![crate::OutputFile {
            path: output_path.clone(),
            format: output_format.to_string(),
            width: 0, // Would need to probe the output file
            height: 0,
            size_bytes: file_size,
            data_base64: None,
        }],
        elapsed_ms,
        metadata: Some(serde_json::json!({
            "start": start_secs,
            "end": end_secs,
            "duration": duration,
            "start_raw": params.start,
            "end_raw": params.end,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_seconds() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "30.5".to_string(),
            end: "60.0".to_string(),
            format: None,
        };

        assert!((params.parse_timestamp(&params.start).unwrap() - 30.5).abs() < 0.001);
        assert!((params.parse_timestamp(&params.end).unwrap() - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_timestamp_colon_format() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "1:30".to_string(),
            end: "2:00".to_string(),
            format: None,
        };

        assert!((params.parse_timestamp(&params.start).unwrap() - 90.0).abs() < 0.001);
        assert!((params.parse_timestamp(&params.end).unwrap() - 120.0).abs() < 0.001);
    }

    #[test]
    fn test_timestamp_hours() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "1:30:45".to_string(),
            end: "2:00:00".to_string(),
            format: None,
        };

        let start = params.parse_timestamp(&params.start).unwrap();
        assert!((start - 5445.0).abs() < 0.001); // 1*3600 + 30*60 + 45
    }

    #[test]
    fn test_timestamp_milliseconds() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "00:00:01.500".to_string(),
            end: "00:00:02.500".to_string(),
            format: None,
        };

        let start = params.parse_timestamp(&params.start).unwrap();
        assert!((start - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_validation_valid() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "0".to_string(),
            end: "10".to_string(),
            format: None,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_validation_negative_start() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "-5".to_string(),
            end: "10".to_string(),
            format: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validation_end_before_start() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "10".to_string(),
            end: "5".to_string(),
            format: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validation_equal_times() {
        let params = VideoTrimParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            start: "5".to_string(),
            end: "5".to_string(),
            format: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_trim_input_not_found() {
        let params = VideoTrimParams {
            input: "/nonexistent/video.mp4".to_string(),
            output: "/tmp/out.mp4".to_string(),
            start: "0".to_string(),
            end: "10".to_string(),
            format: None,
        };

        let result = execute(params);
        assert!(result.is_err());
    }
}
