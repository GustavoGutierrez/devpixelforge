//! Audio trimming by timestamp ranges using FFmpeg CLI.

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AudioTrimParams {
    pub input: String,
    pub output: String,
    pub start: f64,
    pub end: f64,
}

impl AudioTrimParams {
    pub fn validate(&self) -> Result<()> {
        if self.start < 0.0 {
            anyhow::bail!("Start time cannot be negative");
        }
        if self.end <= self.start {
            anyhow::bail!("End time must be greater than start time");
        }
        Ok(())
    }
}

pub fn execute(params: AudioTrimParams) -> Result<JobResult> {
    let start_time = std::time::Instant::now();

    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input audio not found: {}", params.input);
    }

    params.validate()?;

    let duration = params.end - params.start;
    let start_str = format_duration(params.start);
    let duration_str = format_duration(duration);

    let mut cmd = FfmpegCommand::new();
    cmd.input(&params.input)
        .output(&params.output)
        .overwrite()
        .args(["-ss", &start_str])
        .args(["-t", &duration_str])
        .args(["-c", "copy"]);

    let mut child = cmd.spawn().context("Failed to spawn FFmpeg")?;
    child.wait().context("FFmpeg audio trim error")?;

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(&params.output)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(JobResult {
        success: true,
        operation: "audio_trim".into(),
        outputs: vec![crate::OutputFile {
            path: params.output.clone(),
            format: std::path::Path::new(&params.output)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp3")
                .to_string(),
            width: 0,
            height: 0,
            size_bytes: file_size,
            data_base64: None,
        }],
        elapsed_ms,
        metadata: Some(serde_json::json!({
            "start_secs": params.start,
            "end_secs": params.end,
            "duration_secs": duration,
        })),
    })
}

fn format_duration(secs: f64) -> String {
    let hours = (secs / 3600.0).floor() as u32;
    let minutes = ((secs % 3600.0) / 60.0).floor() as u32;
    let seconds = secs % 60.0;
    format!("{:02}:{:02}:{:06.3}", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_params_validate_valid() {
        let params = AudioTrimParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            start: 10.0,
            end: 30.0,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_trim_params_validate_negative_start() {
        let params = AudioTrimParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            start: -5.0,
            end: 30.0,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_trim_params_validate_end_before_start() {
        let params = AudioTrimParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            start: 30.0,
            end: 10.0,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "00:00:00.000");
        assert_eq!(format_duration(30.5), "00:00:30.500");
        assert_eq!(format_duration(90.0), "00:01:30.000");
        assert_eq!(format_duration(3661.5), "01:01:01.500");
    }

    #[test]
    fn test_audio_trim_input_not_found() {
        let params = AudioTrimParams {
            input: "/nonexistent/audio.mp3".to_string(),
            output: "/tmp/out.mp3".to_string(),
            start: 0.0,
            end: 10.0,
        };
        let result = execute(params);
        assert!(result.is_err());
    }
}
