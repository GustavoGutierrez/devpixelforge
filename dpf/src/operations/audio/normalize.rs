//! Audio loudness normalization using FFmpeg loudnorm filter.
//!
//! Supports LUFS target normalization (e.g., -14 LUFS for YouTube, -16 for Spotify).

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AudioNormalizeParams {
    pub input: String,
    pub output: String,
    pub target_lufs: f64,
    pub threshold_lufs: Option<f64>,
}

impl AudioNormalizeParams {
    pub fn validate(&self) -> Result<()> {
        if self.target_lufs < -24.0 || self.target_lufs > -9.0 {
            anyhow::bail!("Target LUFS must be between -24 and -9");
        }
        if let Some(threshold) = self.threshold_lufs {
            if threshold < -70.0 || threshold > -9.0 {
                anyhow::bail!("Threshold LUFS must be between -70 and -9");
            }
        }
        Ok(())
    }
}

pub fn execute(params: AudioNormalizeParams) -> Result<JobResult> {
    let start_time = std::time::Instant::now();

    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input audio not found: {}", params.input);
    }

    params.validate()?;

    let threshold = params.threshold_lufs.unwrap_or(-50.0);
    let target = params.target_lufs;

    let mut cmd = FfmpegCommand::new();
    cmd.input(&params.input)
        .output(&params.output)
        .overwrite()
        .args([
            "-af",
            &format!("loudnorm=I={}:TP={}:LRA=11", target, threshold),
        ]);

    let mut child = cmd.spawn().context("Failed to spawn FFmpeg")?;
    child.wait().context("FFmpeg audio normalize error")?;

    let elapsed_ms = start_time.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(&params.output)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(JobResult {
        success: true,
        operation: "audio_normalize".into(),
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
            "target_lufs": target,
            "threshold_lufs": threshold,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_params_validate_valid() {
        let params = AudioNormalizeParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            target_lufs: -14.0,
            threshold_lufs: None,
        };
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_normalize_params_validate_target_too_low() {
        let params = AudioNormalizeParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            target_lufs: -30.0,
            threshold_lufs: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_normalize_params_validate_target_too_high() {
        let params = AudioNormalizeParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            target_lufs: -5.0,
            threshold_lufs: None,
        };
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_audio_normalize_input_not_found() {
        let params = AudioNormalizeParams {
            input: "/nonexistent/audio.mp3".to_string(),
            output: "/tmp/out.mp3".to_string(),
            target_lufs: -14.0,
            threshold_lufs: None,
        };
        let result = execute(params);
        assert!(result.is_err());
    }

    // =========================================================================
    // JSON Serialization Tests
    // =========================================================================

    #[test]
    fn test_audio_normalize_params_json_serialize() {
        let params = AudioNormalizeParams {
            input: "input.mp3".to_string(),
            output: "output.mp3".to_string(),
            target_lufs: -14.0,
            threshold_lufs: Some(-50.0),
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        assert!(json.contains("\"input\":\"input.mp3\""));
        assert!(json.contains("\"target_lufs\":-14"));
        assert!(json.contains("\"threshold_lufs\":-50"));
    }

    #[test]
    fn test_audio_normalize_params_json_deserialize() {
        let json = r#"{
            "input": "input.mp3",
            "output": "output.mp3",
            "target_lufs": -16.0,
            "threshold_lufs": -40.0
        }"#;

        let params: AudioNormalizeParams = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(params.input, "input.mp3");
        assert_eq!(params.target_lufs, -16.0);
        assert_eq!(params.threshold_lufs, Some(-40.0));
    }

    #[test]
    fn test_audio_normalize_params_json_roundtrip() {
        let params = AudioNormalizeParams {
            input: "in.wav".to_string(),
            output: "out.flac".to_string(),
            target_lufs: -23.0,
            threshold_lufs: None,
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        let restored: AudioNormalizeParams =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(restored.input, params.input);
        assert_eq!(restored.output, params.output);
        assert_eq!(restored.target_lufs, params.target_lufs);
        assert_eq!(restored.threshold_lufs, params.threshold_lufs);
    }

    #[test]
    fn test_audio_normalize_lufs_presets() {
        // Test common LUFS presets
        let presets = vec![
            ("youtube", -14.0),
            ("spotify", -16.0),
            ("apple_podcasts", -16.0),
            ("broadcast", -23.0),
        ];

        for (_, target) in presets {
            let params = AudioNormalizeParams {
                input: "in.mp3".to_string(),
                output: "out.mp3".to_string(),
                target_lufs: target,
                threshold_lufs: None,
            };

            // Should serialize/deserialize correctly
            let json = serde_json::to_string(&params).expect("Should serialize");
            let restored: AudioNormalizeParams =
                serde_json::from_str(&json).expect("Should deserialize");
            assert_eq!(restored.target_lufs, target);
        }
    }
}
