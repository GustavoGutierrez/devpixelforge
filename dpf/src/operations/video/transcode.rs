//! Video transcoding using FFmpeg CLI.
//!
//! Supports H.264, VP8, VP9, and AV1 codecs with configurable bitrate and preset.

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

/// Video codec selection.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VideoCodec {
    /// H.264/AVC codec - best compatibility
    H264,
    /// VP8 codec - WebM, good for web
    Vp8,
    /// VP9 codec - WebM, better quality than VP8
    Vp9,
    /// AV1 codec - newest, best compression
    Av1,
}

impl VideoCodec {
    /// Returns the FFmpeg codec name for this codec.
    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "libx264",
            VideoCodec::Vp8 => "libvpx",
            VideoCodec::Vp9 => "libvpx-vp9",
            VideoCodec::Av1 => "libaom-av1",
        }
    }

    /// Returns the default bitrate for this codec in kbps.
    pub fn default_bitrate_kbps(&self) -> u32 {
        match self {
            VideoCodec::H264 => 2000,
            VideoCodec::Vp8 => 2500,
            VideoCodec::Vp9 => 2000,
            VideoCodec::Av1 => 1500,
        }
    }

    /// Parses a codec string to VideoCodec.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "h264" | "libx264" | "x264" => Some(VideoCodec::H264),
            "vp8" | "libvpx" => Some(VideoCodec::Vp8),
            "vp9" | "libvpx-vp9" => Some(VideoCodec::Vp9),
            "av1" | "libaom-av1" | "aom" => Some(VideoCodec::Av1),
            _ => None,
        }
    }
}

impl Default for VideoCodec {
    fn default() -> Self {
        VideoCodec::H264
    }
}

/// Parameters for video transcoding.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct VideoTranscodeParams {
    /// Source video path
    pub input: String,
    /// Destination video path
    pub output: String,
    /// Target codec (default: h264)
    pub codec: Option<String>,
    /// Target bitrate in kbps (e.g., "2000" for 2Mbps)
    pub bitrate: Option<String>,
    /// Encoding preset: ultrafast, fast, medium, slow, veryslow (default: medium)
    pub preset: Option<String>,
    /// CRF for constant quality (0-51, lower = better quality)
    pub crf: Option<u8>,
    /// Audio codec to use (default: aac)
    pub audio_codec: Option<String>,
    /// Audio bitrate in kbps (default: 128)
    pub audio_bitrate: Option<u32>,
}

impl VideoTranscodeParams {
    /// Parses the codec string to VideoCodec enum.
    pub fn parse_codec(&self) -> VideoCodec {
        self.codec
            .as_deref()
            .and_then(VideoCodec::from_str)
            .unwrap_or(VideoCodec::H264)
    }

    /// Parses the bitrate string to kbps value.
    pub fn parse_bitrate(&self) -> u32 {
        if let Some(ref br) = self.bitrate {
            let br = br.trim().to_uppercase();
            if br.ends_with('M') || br.ends_with("MBPS") {
                br.trim_end_matches(|c: char| !c.is_ascii_digit())
                    .parse()
                    .map(|v: u32| v * 1000)
                    .unwrap_or(2000)
            } else {
                br.trim_end_matches(|c: char| c == 'K' || !c.is_ascii_digit())
                    .parse()
                    .unwrap_or(2000)
            }
        } else {
            self.parse_codec().default_bitrate_kbps()
        }
    }

    /// Returns the FFmpeg preset string.
    pub fn parse_preset(&self) -> &'static str {
        match self.preset.as_deref() {
            Some("ultrafast") => "ultrafast",
            Some("superfast") => "superfast",
            Some("veryfast") => "veryfast",
            Some("faster") => "faster",
            Some("fast") => "fast",
            Some("slow") => "slow",
            Some("slower") => "slower",
            Some("veryslow") => "veryslow",
            _ => "medium",
        }
    }

    /// Returns the audio codec name.
    pub fn parse_audio_codec(&self) -> &'static str {
        match self.audio_codec.as_deref() {
            Some("aac") | None => "aac",
            Some("mp3") | Some("libmp3lame") => "libmp3lame",
            Some("opus") | Some("libopus") => "libopus",
            Some("vorbis") | Some("libvorbis") => "libvorbis",
            Some("copy") => "copy",
            _ => "aac",
        }
    }
}

/// Executes video transcoding using FFmpeg CLI.
pub fn execute(params: VideoTranscodeParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    let input_path = &params.input;
    let output_path = &params.output;

    // Validate input exists
    if !std::path::Path::new(input_path).exists() {
        anyhow::bail!("Input video not found: {}", input_path);
    }

    let codec = params.parse_codec();
    let codec_name = codec.ffmpeg_name();
    let bitrate = params.parse_bitrate();
    let preset = params.parse_preset();
    let audio_codec = params.parse_audio_codec();
    let audio_bitrate = params.audio_bitrate.unwrap_or(128);

    // Build FFmpeg command
    let mut cmd = FfmpegCommand::new();
    cmd.input(input_path)
        .output(output_path)
        .overwrite()
        .args(["-c:v", codec_name])
        .args(["-b:v", &format!("{}k", bitrate)]);

    // H.264 specific: apply preset
    if codec == VideoCodec::H264 {
        cmd.args(["-preset", preset]);
    }

    // CRF if specified
    if let Some(crf) = params.crf {
        let crf_val = crf.min(51);
        match codec {
            VideoCodec::H264 | VideoCodec::Vp8 | VideoCodec::Vp9 | VideoCodec::Av1 => {
                cmd.args(["-crf", &crf_val.to_string()]);
                cmd.args(["-b:v", "0"]);
            }
        }
    }

    // Audio codec
    cmd.args(["-c:a", audio_codec]);
    if audio_codec != "copy" {
        cmd.args(["-b:a", &format!("{}k", audio_bitrate)]);
    }

    // Execute command
    let mut child = cmd.spawn().context("Failed to spawn FFmpeg process")?;
    let result = child.wait();

    if let Err(e) = result {
        anyhow::bail!("FFmpeg transcoding failed: {}", e);
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    // Probe output for dimensions
    let (width, height) = probe_dimensions(output_path).unwrap_or((0, 0));

    Ok(JobResult {
        success: true,
        operation: "video_transcode".into(),
        outputs: vec![crate::OutputFile {
            path: output_path.clone(),
            format: std::path::Path::new(output_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mp4")
                .to_string(),
            width,
            height,
            size_bytes: file_size,
            data_base64: None,
        }],
        elapsed_ms,
        metadata: Some(serde_json::json!({
            "codec": codec_name,
            "bitrate_kbps": bitrate,
            "preset": preset,
            "audio_codec": audio_codec,
            "audio_bitrate_kbps": audio_bitrate,
        })),
    })
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
        .context("Failed to run ffprobe")?;

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
    fn test_video_codec_defaults() {
        let params = VideoTranscodeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            crf: None,
            audio_codec: None,
            audio_bitrate: None,
        };

        assert_eq!(params.parse_codec(), VideoCodec::H264);
        assert_eq!(params.parse_bitrate(), 2000);
        assert_eq!(params.parse_preset(), "medium");
    }

    #[test]
    fn test_video_codec_parsing() {
        let test_cases = vec![
            (Some("h264"), VideoCodec::H264),
            (Some("vp8"), VideoCodec::Vp8),
            (Some("vp9"), VideoCodec::Vp9),
            (Some("av1"), VideoCodec::Av1),
            (Some("invalid"), VideoCodec::H264), // Falls back to H264
            (None, VideoCodec::H264),
        ];

        for (codec_str, expected) in test_cases {
            let params = VideoTranscodeParams {
                input: "in.mp4".to_string(),
                output: "out.mp4".to_string(),
                codec: codec_str.map(String::from),
                bitrate: None,
                preset: None,
                crf: None,
                audio_codec: None,
                audio_bitrate: None,
            };
            assert_eq!(params.parse_codec(), expected);
        }
    }

    #[test]
    fn test_bitrate_parsing() {
        let test_cases = vec![
            (Some("2000"), 2000),
            (Some("2000k"), 2000),
            (Some("2000K"), 2000),
            (Some("2M"), 2000),
            (Some("2m"), 2000),
            (Some("1M"), 1000),
            (None, 2000), // Default for H264
        ];

        for (bitrate_str, expected) in test_cases {
            let params = VideoTranscodeParams {
                input: "in.mp4".to_string(),
                output: "out.mp4".to_string(),
                codec: Some("h264".to_string()),
                bitrate: bitrate_str.map(String::from),
                preset: None,
                crf: None,
                audio_codec: None,
                audio_bitrate: None,
            };
            assert_eq!(params.parse_bitrate(), expected);
        }
    }

    #[test]
    fn test_preset_parsing() {
        let presets: Vec<(Option<&str>, &str)> = vec![
            (Some("ultrafast"), "ultrafast"),
            (Some("fast"), "fast"),
            (Some("medium"), "medium"),
            (Some("slow"), "slow"),
            (Some("slower"), "slower"),
            (Some("veryslow"), "veryslow"),
            (Some("invalid"), "medium"), // Falls back to medium
            (None, "medium"),            // None falls back to medium
        ];

        for (preset_str, expected) in presets {
            let params = VideoTranscodeParams {
                input: "in.mp4".to_string(),
                output: "out.mp4".to_string(),
                codec: None,
                bitrate: None,
                preset: preset_str.map(String::from),
                crf: None,
                audio_codec: None,
                audio_bitrate: None,
            };
            assert_eq!(params.parse_preset(), expected);
        }
    }

    #[test]
    fn test_audio_codec_parsing() {
        let params = VideoTranscodeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            crf: None,
            audio_codec: Some("opus".to_string()),
            audio_bitrate: None,
        };
        assert_eq!(params.parse_audio_codec(), "libopus");

        let params = VideoTranscodeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            crf: None,
            audio_codec: None,
            audio_bitrate: None,
        };
        assert_eq!(params.parse_audio_codec(), "aac");
    }

    #[test]
    fn test_video_transcode_input_not_found() {
        let params = VideoTranscodeParams {
            input: "/nonexistent/video.mp4".to_string(),
            output: "/tmp/out.mp4".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            crf: None,
            audio_codec: None,
            audio_bitrate: None,
        };

        let result = execute(params);
        assert!(result.is_err());
    }

    // =========================================================================
    // JSON Serialization Tests
    // =========================================================================

    #[test]
    fn test_video_transcode_params_json_serialize() {
        let params = VideoTranscodeParams {
            input: "input.mp4".to_string(),
            output: "output.mp4".to_string(),
            codec: Some("h264".to_string()),
            bitrate: Some("2000k".to_string()),
            preset: Some("fast".to_string()),
            crf: Some(23),
            audio_codec: Some("aac".to_string()),
            audio_bitrate: Some(128),
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        assert!(json.contains("\"input\":\"input.mp4\""));
        assert!(json.contains("\"codec\":\"h264\""));
        assert!(json.contains("\"bitrate\":\"2000k\""));
        assert!(json.contains("\"preset\":\"fast\""));
        assert!(json.contains("\"crf\":23"));
        assert!(json.contains("\"audio_codec\":\"aac\""));
    }

    #[test]
    fn test_video_transcode_params_json_deserialize() {
        let json = r#"{
            "input": "input.mp4",
            "output": "output.webm",
            "codec": "vp9",
            "bitrate": "1500k",
            "preset": "slow",
            "audio_codec": "opus"
        }"#;

        let params: VideoTranscodeParams = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(params.input, "input.mp4");
        assert_eq!(params.output, "output.webm");
        assert_eq!(params.codec, Some("vp9".to_string()));
        assert_eq!(params.bitrate, Some("1500k".to_string()));
        assert_eq!(params.preset, Some("slow".to_string()));
        assert_eq!(params.audio_codec, Some("opus".to_string()));
    }

    #[test]
    fn test_video_transcode_params_json_roundtrip() {
        let params = VideoTranscodeParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            codec: Some("av1".to_string()),
            bitrate: Some("1M".to_string()),
            preset: Some("veryslow".to_string()),
            crf: Some(30),
            audio_codec: None,
            audio_bitrate: None,
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        let restored: VideoTranscodeParams =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(restored.input, params.input);
        assert_eq!(restored.output, params.output);
        assert_eq!(restored.codec, params.codec);
        assert_eq!(restored.bitrate, params.bitrate);
        assert_eq!(restored.preset, params.preset);
        assert_eq!(restored.crf, params.crf);
    }

    #[test]
    fn test_video_codec_serde() {
        // Test that VideoCodec serializes to kebab-case
        let codec = VideoCodec::H264;
        let json = serde_json::to_string(&codec).expect("Should serialize");
        assert_eq!(json, "\"h264\"");

        let codec = VideoCodec::Vp9;
        let json = serde_json::to_string(&codec).expect("Should serialize");
        assert_eq!(json, "\"vp9\"");

        // Deserialize
        let restored: VideoCodec = serde_json::from_str("\"av1\"").expect("Should deserialize");
        assert_eq!(restored, VideoCodec::Av1);
    }

    #[test]
    fn test_video_transcode_params_defaults() {
        // All optional fields should default correctly
        let json = r#"{"input": "in.mp4", "output": "out.mp4"}"#;
        let params: VideoTranscodeParams = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(params.input, "in.mp4");
        assert!(params.codec.is_none());
        assert!(params.bitrate.is_none());
        assert!(params.preset.is_none());
        assert!(params.crf.is_none());
        assert!(params.audio_codec.is_none());
        assert!(params.audio_bitrate.is_none());
    }
}
