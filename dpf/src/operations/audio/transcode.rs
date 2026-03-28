//! Audio transcoding using FFmpeg CLI.
//!
//! Supports MP3, AAC, Opus, Vorbis, FLAC, and WAV formats.

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudioCodec {
    Mp3,
    Aac,
    Opus,
    Vorbis,
    Flac,
    Wav,
}

impl AudioCodec {
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            AudioCodec::Mp3 => "libmp3lame",
            AudioCodec::Aac => "aac",
            AudioCodec::Opus => "libopus",
            AudioCodec::Vorbis => "libvorbis",
            AudioCodec::Flac => "flac",
            AudioCodec::Wav => "pcm_s16le",
        }
    }

    pub fn default_bitrate_kbps(&self) -> u32 {
        match self {
            AudioCodec::Mp3 => 192,
            AudioCodec::Aac => 192,
            AudioCodec::Opus => 128,
            AudioCodec::Vorbis => 192,
            AudioCodec::Flac | AudioCodec::Wav => 0,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mp3" | "libmp3lame" => Some(AudioCodec::Mp3),
            "aac" => Some(AudioCodec::Aac),
            "opus" | "libopus" => Some(AudioCodec::Opus),
            "vorbis" | "libvorbis" => Some(AudioCodec::Vorbis),
            "flac" => Some(AudioCodec::Flac),
            "wav" | "pcm" => Some(AudioCodec::Wav),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct AudioTranscodeParams {
    pub input: String,
    pub output: String,
    pub codec: Option<String>,
    pub bitrate: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub quality: Option<u8>,
}

impl AudioTranscodeParams {
    pub fn parse_codec(&self) -> AudioCodec {
        self.codec
            .as_deref()
            .and_then(AudioCodec::from_str)
            .unwrap_or(AudioCodec::Mp3)
    }

    pub fn parse_bitrate(&self) -> Option<u32> {
        self.bitrate.as_ref().map(|br| {
            let br = br.trim().to_uppercase();
            br.trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .unwrap_or(192)
        })
    }

    pub fn default_bitrate(&self) -> u32 {
        self.parse_bitrate()
            .unwrap_or_else(|| self.parse_codec().default_bitrate_kbps())
    }
}

pub fn execute(params: AudioTranscodeParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input audio not found: {}", params.input);
    }

    let codec = params.parse_codec();
    let codec_name = codec.ffmpeg_name();

    let mut cmd = FfmpegCommand::new();
    cmd.input(&params.input)
        .output(&params.output)
        .overwrite()
        .args(["-c:a", codec_name]);

    if params.default_bitrate() > 0 {
        cmd.args(["-b:a", &format!("{}k", params.default_bitrate())]);
    }

    if let Some(sr) = params.sample_rate {
        cmd.args(["-ar", &sr.to_string()]);
    }

    if let Some(ch) = params.channels {
        cmd.args(["-ac", &ch.to_string()]);
    }

    if let Some(q) = params.quality {
        let q_val = q.min(10);
        match codec {
            AudioCodec::Mp3 => {
                cmd.args(["-q:a", &(9 - q_val).to_string()]);
            }
            AudioCodec::Opus => {
                let bitrate = 64000 + (q_val as u32) * 16000;
                cmd.args(["-b:a", &format!("{}", bitrate)]);
            }
            AudioCodec::Vorbis => {
                cmd.args(["-q:a", &q_val.to_string()]);
            }
            _ => {}
        }
    }

    let mut child = cmd.spawn().context("Failed to spawn FFmpeg")?;
    child.wait().context("FFmpeg audio transcode error")?;

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(&params.output)
        .map(|m| m.len())
        .unwrap_or(0);
    let (duration, sample_rate, channels) = probe_audio_info(&params.output).unwrap_or((0.0, 0, 0));

    Ok(JobResult {
        success: true,
        operation: "audio_transcode".into(),
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
            "codec": codec_name,
            "bitrate_kbps": params.default_bitrate(),
            "sample_rate": sample_rate,
            "channels": channels,
            "duration_secs": duration,
        })),
    })
}

fn probe_audio_info(path: &str) -> Result<(f64, u32, u32)> {
    let output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=sample_rate,channels",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()
        .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let info = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = info.trim().split(',').collect();

    let duration = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let sample_rate = parts
        .get(1)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let channels = parts
        .get(2)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    Ok((duration, sample_rate, channels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_codec_from_str() {
        assert_eq!(AudioCodec::from_str("mp3"), Some(AudioCodec::Mp3));
        assert_eq!(AudioCodec::from_str("aac"), Some(AudioCodec::Aac));
        assert_eq!(AudioCodec::from_str("opus"), Some(AudioCodec::Opus));
        assert_eq!(AudioCodec::from_str("flac"), Some(AudioCodec::Flac));
        assert_eq!(AudioCodec::from_str("invalid"), None);
    }

    #[test]
    fn test_audio_codec_ffmpeg_name() {
        assert_eq!(AudioCodec::Mp3.ffmpeg_name(), "libmp3lame");
        assert_eq!(AudioCodec::Aac.ffmpeg_name(), "aac");
        assert_eq!(AudioCodec::Opus.ffmpeg_name(), "libopus");
        assert_eq!(AudioCodec::Flac.ffmpeg_name(), "flac");
    }

    #[test]
    fn test_params_parse_codec() {
        let params = AudioTranscodeParams {
            input: "in.mp3".to_string(),
            output: "out.aac".to_string(),
            codec: Some("aac".to_string()),
            bitrate: None,
            sample_rate: None,
            channels: None,
            quality: None,
        };
        assert_eq!(params.parse_codec(), AudioCodec::Aac);
    }

    #[test]
    fn test_params_parse_bitrate() {
        let params = AudioTranscodeParams {
            input: "in.mp3".to_string(),
            output: "out.mp3".to_string(),
            codec: Some("mp3".to_string()),
            bitrate: Some("320".to_string()),
            sample_rate: None,
            channels: None,
            quality: None,
        };
        assert_eq!(params.parse_bitrate(), Some(320));
    }

    #[test]
    fn test_audio_transcode_input_not_found() {
        let params = AudioTranscodeParams {
            input: "/nonexistent/audio.mp3".to_string(),
            output: "/tmp/out.aac".to_string(),
            codec: Some("aac".to_string()),
            bitrate: None,
            sample_rate: None,
            channels: None,
            quality: None,
        };
        let result = execute(params);
        assert!(result.is_err());
    }

    // =========================================================================
    // JSON Serialization Tests
    // =========================================================================

    #[test]
    fn test_audio_transcode_params_json_serialize() {
        let params = AudioTranscodeParams {
            input: "input.mp3".to_string(),
            output: "output.aac".to_string(),
            codec: Some("aac".to_string()),
            bitrate: Some("192k".to_string()),
            sample_rate: Some(44100),
            channels: Some(2),
            quality: Some(8),
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        assert!(json.contains("\"input\":\"input.mp3\""));
        assert!(json.contains("\"codec\":\"aac\""));
        assert!(json.contains("\"bitrate\":\"192k\""));
        assert!(json.contains("\"sample_rate\":44100"));
        assert!(json.contains("\"channels\":2"));
    }

    #[test]
    fn test_audio_transcode_params_json_deserialize() {
        let json = r#"{
            "input": "input.mp3",
            "output": "output.opus",
            "codec": "opus",
            "bitrate": "128k",
            "sample_rate": 48000,
            "channels": 1
        }"#;

        let params: AudioTranscodeParams = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(params.input, "input.mp3");
        assert_eq!(params.output, "output.opus");
        assert_eq!(params.codec, Some("opus".to_string()));
        assert_eq!(params.bitrate, Some("128k".to_string()));
        assert_eq!(params.sample_rate, Some(48000));
        assert_eq!(params.channels, Some(1));
    }

    #[test]
    fn test_audio_transcode_params_json_roundtrip() {
        let params = AudioTranscodeParams {
            input: "in.flac".to_string(),
            output: "out.wav".to_string(),
            codec: Some("wav".to_string()),
            bitrate: None,
            sample_rate: Some(96000),
            channels: Some(2),
            quality: None,
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        let restored: AudioTranscodeParams =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(restored.input, params.input);
        assert_eq!(restored.output, params.output);
        assert_eq!(restored.codec, params.codec);
        assert_eq!(restored.sample_rate, params.sample_rate);
        assert_eq!(restored.channels, params.channels);
    }

    #[test]
    fn test_audio_codec_serde() {
        // Test that AudioCodec serializes to kebab-case
        let codec = AudioCodec::Mp3;
        let json = serde_json::to_string(&codec).expect("Should serialize");
        assert_eq!(json, "\"mp3\"");

        let codec = AudioCodec::Opus;
        let json = serde_json::to_string(&codec).expect("Should serialize");
        assert_eq!(json, "\"opus\"");

        // Deserialize
        let restored: AudioCodec = serde_json::from_str("\"flac\"").expect("Should deserialize");
        assert_eq!(restored, AudioCodec::Flac);
    }

    #[test]
    fn test_audio_transcode_params_defaults() {
        // Minimal JSON with only required fields
        let json = r#"{"input": "in.mp3", "output": "out.mp3"}"#;
        let params: AudioTranscodeParams = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(params.input, "in.mp3");
        assert!(params.codec.is_none());
        assert!(params.bitrate.is_none());
        assert!(params.sample_rate.is_none());
        assert!(params.channels.is_none());
        assert!(params.quality.is_none());
    }
}
