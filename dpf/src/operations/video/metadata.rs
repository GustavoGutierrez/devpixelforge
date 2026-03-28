//! Video metadata extraction using FFmpeg CLI.
//!
//! Extracts video information such as dimensions, duration, codec, bitrate, etc.

use crate::JobResult;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Parameters for video metadata extraction.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct VideoMetadataParams {
    /// Video file path
    pub input: String,
    /// Include audio stream info (default: true)
    #[serde(default = "default_true")]
    pub include_audio: bool,
}

/// Creates a default true value for bool fields.
fn default_true() -> bool {
    true
}

/// Video metadata information.
#[derive(Debug, Serialize)]
pub struct VideoMetadata {
    /// File path
    pub path: String,
    /// Format/container (e.g., "mp4", "webm")
    pub format: String,
    /// Duration in seconds
    pub duration: f64,
    /// Total file size in bytes
    pub size_bytes: u64,
    /// Overall bitrate in bps
    pub bitrate: u64,
    /// Video stream info
    pub video: Option<VideoStreamInfo>,
    /// Audio stream info
    pub audio: Option<AudioStreamInfo>,
}

/// Video stream information.
#[derive(Debug, Serialize)]
pub struct VideoStreamInfo {
    /// Codec name (e.g., "h264", "vp9")
    pub codec: String,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Frame rate (fps)
    pub frame_rate: String,
    /// Video bitrate in bps
    pub bitrate: u64,
    /// Pixel format
    pub pixel_format: String,
    /// Number of frames
    pub frames: u64,
}

/// Audio stream information.
#[derive(Debug, Serialize)]
pub struct AudioStreamInfo {
    /// Codec name (e.g., "aac", "mp3", "opus")
    pub codec: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u32,
    /// Audio bitrate in bps
    pub bitrate: u64,
    /// Language (if available)
    pub language: Option<String>,
}

/// Executes metadata extraction.
pub fn execute(params: VideoMetadataParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    let input_path = &params.input;

    // Validate input exists
    if !std::path::Path::new(input_path).exists() {
        anyhow::bail!("Input video not found: {}", input_path);
    }

    let file_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);

    // Probe video info using ffprobe
    let probe_info = probe_with_ffprobe(input_path)?;

    let metadata = VideoMetadata {
        path: input_path.clone(),
        format: probe_info.format,
        duration: probe_info.duration,
        size_bytes: file_size,
        bitrate: probe_info.bitrate,
        video: probe_info.video,
        audio: if params.include_audio {
            probe_info.audio
        } else {
            None
        },
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(JobResult {
        success: true,
        operation: "video_metadata".into(),
        outputs: vec![], // Metadata doesn't produce output files
        elapsed_ms,
        metadata: Some(serde_json::to_value(&metadata)?),
    })
}

/// Probe information from ffprobe.
struct ProbeInfo {
    format: String,
    duration: f64,
    bitrate: u64,
    video: Option<VideoStreamInfo>,
    audio: Option<AudioStreamInfo>,
}

/// Probes video file using ffprobe.
fn probe_with_ffprobe(input_path: &str) -> Result<ProbeInfo> {
    // Get format info
    let format_output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=format_name,duration,bit_rate",
            "-of",
            "json",
            input_path,
        ])
        .output()
        .context("Failed to run ffprobe")?;

    if !format_output.status.success() {
        anyhow::bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&format_output.stderr)
        );
    }

    let format_json: serde_json::Value =
        serde_json::from_slice(&format_output.stdout).context("Failed to parse ffprobe output")?;

    let format_info = format_json
        .pointer("/format")
        .context("No format info in ffprobe output")?;

    let format_name = format_info["format_name"]
        .as_str()
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .to_string();

    let duration = format_info["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let bitrate = format_info["bit_rate"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Get stream info
    let stream_output = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0,a:0",
            "-show_entries",
            "stream=codec_name,width,height,r_frame_rate,bit_rate,pix_fmt,nb_frames,sample_rate,channels",
            "-of",
            "json",
            input_path,
        ])
        .output()
        .context("Failed to run ffprobe for streams")?;

    let stream_json: serde_json::Value =
        serde_json::from_slice(&stream_output.stdout).context("Failed to parse stream info")?;

    let streams = stream_json
        .pointer("/streams")
        .and_then(|s| s.as_array())
        .map(|arr| arr.to_vec())
        .unwrap_or_default();

    let video = streams.iter().find(|s| {
        s.pointer("/codec_type")
            .and_then(|v| v.as_str())
            .map(|t| t == "video")
            .unwrap_or(false)
    });

    let audio = streams.iter().find(|s| {
        s.pointer("/codec_type")
            .and_then(|v| v.as_str())
            .map(|t| t == "audio")
            .unwrap_or(false)
    });

    let video_info = video.map(|v| {
        let frame_rate = v["r_frame_rate"].as_str().unwrap_or("0/1");
        let (num, den) = parse_fraction(frame_rate);
        let fps = if den > 0 {
            format!("{:.2}", num as f64 / den as f64)
        } else {
            "unknown".to_string()
        };

        VideoStreamInfo {
            codec: v["codec_name"].as_str().unwrap_or("unknown").to_string(),
            width: v["width"].as_u64().unwrap_or(0) as u32,
            height: v["height"].as_u64().unwrap_or(0) as u32,
            frame_rate: fps,
            bitrate: v["bit_rate"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            pixel_format: v["pix_fmt"].as_str().unwrap_or("unknown").to_string(),
            frames: v["nb_frames"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
        }
    });

    let audio_info = audio.map(|a| AudioStreamInfo {
        codec: a["codec_name"].as_str().unwrap_or("unknown").to_string(),
        sample_rate: a["sample_rate"]
            .as_str()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0),
        channels: a["channels"].as_u64().unwrap_or(0) as u32,
        bitrate: a["bit_rate"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0),
        language: None,
    });

    Ok(ProbeInfo {
        format: format_name,
        duration,
        bitrate,
        video: video_info,
        audio: audio_info,
    })
}

/// Parses a fraction string like "30000/1001" into (num, den).
fn parse_fraction(s: &str) -> (u32, u32) {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return (0, 1);
    }
    let num = parts[0].parse().unwrap_or(0);
    let den = parts[1].parse().unwrap_or(1);
    (num, den)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fraction() {
        assert_eq!(parse_fraction("30000/1001"), (30000, 1001));
        assert_eq!(parse_fraction("25/1"), (25, 1));
        assert_eq!(parse_fraction("invalid"), (0, 1));
    }

    #[test]
    fn test_metadata_input_not_found() {
        let params = VideoMetadataParams {
            input: "/nonexistent/video.mp4".to_string(),
            include_audio: true,
        };

        let result = execute(params);
        assert!(result.is_err());
    }
}
