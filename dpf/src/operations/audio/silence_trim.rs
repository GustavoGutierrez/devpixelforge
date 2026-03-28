//! Audio silence trimming using FFmpeg silenceremove filter.
//!
//! Removes leading and trailing silence from audio files.

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AudioSilenceTrimParams {
    pub input: String,
    pub output: String,
    pub threshold_db: Option<f64>,
    pub min_duration: Option<f64>,
}

impl AudioSilenceTrimParams {
    pub fn validate(&self) -> Result<()> {
        if let Some(threshold) = self.threshold_db {
            if threshold > 0.0 {
                anyhow::bail!("Threshold must be 0 or negative dB");
            }
        }
        if let Some(duration) = self.min_duration {
            if duration <= 0.0 {
                anyhow::bail!("Min duration must be positive");
            }
        }
        Ok(())
    }
}

pub fn execute(params: AudioSilenceTrimParams) -> Result<JobResult> {
    let start_time = std::time::Instant::now();

    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input audio not found: {}", params.input);
    }

    params.validate()?;

    let threshold = params.threshold_db.unwrap_or(-40.0);
    let min_duration = params.min_duration.unwrap_or(0.5);

    let filter = format!(
        "silenceremove=start_periods=1:start_duration={}:start_threshold={}dB:detection=peak,silenceremove=stop_periods=-1:stop_duration={}:stop_threshold={}dB:detection=peak",
        min_duration, threshold, min_duration, threshold
    );

    let mut cmd = FfmpegCommand::new();
    cmd.input(&params.input)
        .output(&params.output)
        .overwrite()
        .args(["-af", &filter]);

    let mut child = cmd.spawn().context("Failed to spawn FFmpeg")?;
    child.wait().context("FFmpeg silence trim error")?;

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(&params.output)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(JobResult {
        success: true,
        operation: "audio_silence_trim".into(),
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
            "threshold_db": threshold,
            "min_duration_secs": min_duration,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_trim_params_validate_valid() {
        let params = AudioSilenceTrimParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            threshold_db: Some(-30.0),
            min_duration: Some(1.0),
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_silence_trim_params_validate_default() {
        let params = AudioSilenceTrimParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            threshold_db: None,
            min_duration: None,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_silence_trim_params_validate_threshold_positive() {
        let params = AudioSilenceTrimParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            threshold_db: Some(5.0),
            min_duration: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_audio_silence_trim_input_not_found() {
        let params = AudioSilenceTrimParams {
            input: "/nonexistent/audio.mp3".to_string(),
            output: "/tmp/out.mp3".to_string(),
            threshold_db: None,
            min_duration: None,
        };
        let result = execute(params);
        assert!(result.is_err());
    }
}
