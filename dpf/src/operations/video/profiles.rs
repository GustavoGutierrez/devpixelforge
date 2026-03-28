//! Video encoding profiles for web optimization using FFmpeg CLI.
//!
//! Provides preset configurations for common web delivery scenarios:
//! - web-low: 480p, 1Mbps H.264
//! - web-mid: 720p, 2.5Mbps H.264
//! - web-high: 1080p, 5Mbps H.264

use crate::JobResult;
use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};

/// Video profile type.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VideoProfileType {
    /// Low quality: 480p, 1Mbps
    WebLow,
    /// Medium quality: 720p, 2.5Mbps
    WebMid,
    /// High quality: 1080p, 5Mbps
    WebHigh,
}

impl VideoProfileType {
    /// Returns the target height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            VideoProfileType::WebLow => 480,
            VideoProfileType::WebMid => 720,
            VideoProfileType::WebHigh => 1080,
        }
    }

    /// Returns the target bitrate in kbps.
    pub fn bitrate_kbps(&self) -> u32 {
        match self {
            VideoProfileType::WebLow => 1000,
            VideoProfileType::WebMid => 2500,
            VideoProfileType::WebHigh => 5000,
        }
    }

    /// Returns the FFmpeg preset for encoding.
    pub fn preset(&self) -> &'static str {
        match self {
            VideoProfileType::WebLow => "fast",
            VideoProfileType::WebMid => "medium",
            VideoProfileType::WebHigh => "slow",
        }
    }

    /// Returns the audio bitrate in kbps.
    pub fn audio_bitrate_kbps(&self) -> u32 {
        match self {
            VideoProfileType::WebLow => 64,
            VideoProfileType::WebMid => 128,
            VideoProfileType::WebHigh => 192,
        }
    }

    /// Returns the scale filter for this profile.
    pub fn scale_filter(&self) -> String {
        format!("scale=-2:{}", self.height())
    }

    /// Parses a string to VideoProfileType.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "web-low" | "webl" | "low" => Some(VideoProfileType::WebLow),
            "web-mid" | "webm" | "mid" | "medium" => Some(VideoProfileType::WebMid),
            "web-high" | "webh" | "high" => Some(VideoProfileType::WebHigh),
            _ => None,
        }
    }
}

impl Default for VideoProfileType {
    fn default() -> Self {
        VideoProfileType::WebMid
    }
}

/// Parameters for video profile encoding.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct VideoProfileParams {
    /// Source video path
    pub input: String,
    /// Destination video path
    pub output: String,
    /// Profile type: "web-low", "web-mid", "web-high"
    pub profile: String,
    /// Override codec (default: h264)
    pub codec: Option<String>,
    /// Override bitrate in kbps (e.g., "2000")
    pub bitrate: Option<String>,
    /// Override preset
    pub preset: Option<String>,
    /// Audio codec (default: aac)
    pub audio_codec: Option<String>,
    /// Audio bitrate in kbps
    pub audio_bitrate: Option<u32>,
    /// Apply fast start for web streaming (default: true)
    #[serde(default = "default_true")]
    pub fast_start: bool,
}

fn default_true() -> bool {
    true
}

impl VideoProfileParams {
    /// Returns the profile type.
    pub fn parse_profile(&self) -> Result<VideoProfileType> {
        VideoProfileType::from_str(&self.profile)
            .ok_or_else(|| anyhow::anyhow!("Invalid profile: {}", self.profile))
    }

    /// Returns the codec name.
    pub fn codec(&self) -> &'static str {
        match self.codec.as_deref() {
            Some("h264") | Some("libx264") => "libx264",
            Some("vp8") | Some("libvpx") => "libvpx",
            Some("vp9") | Some("libvpx-vp9") => "libvpx-vp9",
            Some("av1") | Some("libaom-av1") => "libaom-av1",
            _ => "libx264", // Default to H.264
        }
    }

    /// Returns the audio codec.
    pub fn audio_codec(&self) -> &'static str {
        match self.audio_codec.as_deref() {
            Some("aac") | None => "aac",
            Some("libmp3lame") | Some("mp3") => "libmp3lame",
            Some("libopus") | Some("opus") => "libopus",
            Some("copy") => "copy",
            _ => "aac",
        }
    }

    /// Returns the FFmpeg preset string.
    pub fn preset(&self) -> &'static str {
        if let Some(ref p) = self.preset {
            match p.to_lowercase().as_str() {
                "ultrafast" => "ultrafast",
                "superfast" => "superfast",
                "veryfast" => "veryfast",
                "faster" => "faster",
                "fast" => "fast",
                "medium" => "medium",
                "slow" => "slow",
                "slower" => "slower",
                "veryslow" => "veryslow",
                _ => "medium",
            }
        } else {
            self.parse_profile().map(|p| p.preset()).unwrap_or("medium")
        }
    }
}

/// Executes profile-based video encoding.
pub fn execute(params: VideoProfileParams) -> Result<JobResult> {
    let start = std::time::Instant::now();

    // Validate input exists
    if !std::path::Path::new(&params.input).exists() {
        anyhow::bail!("Input video not found: {}", params.input);
    }

    let profile = params.parse_profile()?;
    let input_path = &params.input;
    let output_path = &params.output;

    // Calculate bitrate
    let bitrate = params
        .bitrate
        .as_ref()
        .map(|b| {
            b.trim()
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse::<u32>()
                .unwrap_or(profile.bitrate_kbps())
        })
        .unwrap_or(profile.bitrate_kbps());

    // Calculate audio bitrate
    let audio_bitrate = params.audio_bitrate.unwrap_or(profile.audio_bitrate_kbps());

    let codec = params.codec();
    let preset = params.preset();
    let audio_codec = params.audio_codec();

    // Build FFmpeg command
    let mut cmd = FfmpegCommand::new();
    cmd.input(input_path)
        .output(output_path)
        .overwrite()
        .args(["-vf", &profile.scale_filter()])
        .args(["-c:v", codec])
        .args(["-b:v", &format!("{}k", bitrate)])
        .args(["-preset", preset])
        .args(["-c:a", audio_codec]);

    if audio_codec != "copy" {
        cmd.args(["-b:a", &format!("{}k", audio_bitrate)]);
    }

    // H.264 specific settings
    if codec == "libx264" {
        // Profile for compatibility
        cmd.args(["-profile:v", "high"]);
        cmd.args(["-level", "4.0"]);

        // Fast start for web streaming
        if params.fast_start {
            cmd.args(["-movflags", "+faststart"]);
        }
    }

    let mut child = cmd.spawn().context("Failed to spawn FFmpeg process")?;
    let result = child.wait();

    if let Err(e) = result {
        anyhow::bail!("FFmpeg profile encoding failed: {}", e);
    }

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let file_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    // Get output dimensions
    let (width, height) = probe_dimensions(output_path).unwrap_or((0, profile.height()));

    Ok(JobResult {
        success: true,
        operation: "video_profile".into(),
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
            "profile": params.profile,
            "codec": codec,
            "bitrate_kbps": bitrate,
            "audio_bitrate_kbps": audio_bitrate,
            "preset": preset,
            "fast_start": params.fast_start,
            "target_height": profile.height(),
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
    fn test_profile_web_low() {
        let profile = VideoProfileType::WebLow;
        assert_eq!(profile.height(), 480);
        assert_eq!(profile.bitrate_kbps(), 1000);
        assert_eq!(profile.audio_bitrate_kbps(), 64);
        assert_eq!(profile.preset(), "fast");
        assert_eq!(profile.scale_filter(), "scale=-2:480");
    }

    #[test]
    fn test_profile_web_mid() {
        let profile = VideoProfileType::WebMid;
        assert_eq!(profile.height(), 720);
        assert_eq!(profile.bitrate_kbps(), 2500);
        assert_eq!(profile.audio_bitrate_kbps(), 128);
        assert_eq!(profile.preset(), "medium");
        assert_eq!(profile.scale_filter(), "scale=-2:720");
    }

    #[test]
    fn test_profile_web_high() {
        let profile = VideoProfileType::WebHigh;
        assert_eq!(profile.height(), 1080);
        assert_eq!(profile.bitrate_kbps(), 5000);
        assert_eq!(profile.audio_bitrate_kbps(), 192);
        assert_eq!(profile.preset(), "slow");
        assert_eq!(profile.scale_filter(), "scale=-2:1080");
    }

    #[test]
    fn test_profile_from_str() {
        assert_eq!(
            VideoProfileType::from_str("web-low"),
            Some(VideoProfileType::WebLow)
        );
        assert_eq!(
            VideoProfileType::from_str("web-mid"),
            Some(VideoProfileType::WebMid)
        );
        assert_eq!(
            VideoProfileType::from_str("web-high"),
            Some(VideoProfileType::WebHigh)
        );
        assert_eq!(
            VideoProfileType::from_str("webl"),
            Some(VideoProfileType::WebLow)
        );
        assert_eq!(
            VideoProfileType::from_str("low"),
            Some(VideoProfileType::WebLow)
        );
        assert_eq!(
            VideoProfileType::from_str("mid"),
            Some(VideoProfileType::WebMid)
        );
        assert_eq!(
            VideoProfileType::from_str("high"),
            Some(VideoProfileType::WebHigh)
        );
        assert_eq!(VideoProfileType::from_str("invalid"), None);
    }

    #[test]
    fn test_params_parse_profile() {
        let params = VideoProfileParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            profile: "web-high".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            audio_codec: None,
            audio_bitrate: None,
            fast_start: true,
        };

        assert_eq!(params.parse_profile().unwrap(), VideoProfileType::WebHigh);
    }

    #[test]
    fn test_params_invalid_profile() {
        let params = VideoProfileParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            profile: "invalid".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            audio_codec: None,
            audio_bitrate: None,
            fast_start: true,
        };

        assert!(params.parse_profile().is_err());
    }

    #[test]
    fn test_params_codec_parsing() {
        let params = VideoProfileParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            profile: "web-mid".to_string(),
            codec: Some("vp9".to_string()),
            bitrate: None,
            preset: None,
            audio_codec: None,
            audio_bitrate: None,
            fast_start: true,
        };

        assert_eq!(params.codec(), "libvpx-vp9");
    }

    #[test]
    fn test_params_audio_codec() {
        let params = VideoProfileParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            profile: "web-mid".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            audio_codec: Some("opus".to_string()),
            audio_bitrate: None,
            fast_start: true,
        };

        assert_eq!(params.audio_codec(), "libopus");
    }

    #[test]
    fn test_params_preset_override() {
        let params = VideoProfileParams {
            input: "in.mp4".to_string(),
            output: "out.mp4".to_string(),
            profile: "web-mid".to_string(),
            codec: None,
            bitrate: None,
            preset: Some("ultrafast".to_string()),
            audio_codec: None,
            audio_bitrate: None,
            fast_start: true,
        };

        assert_eq!(params.preset(), "ultrafast");
    }

    #[test]
    fn test_profile_input_not_found() {
        let params = VideoProfileParams {
            input: "/nonexistent/video.mp4".to_string(),
            output: "/tmp/out.mp4".to_string(),
            profile: "web-mid".to_string(),
            codec: None,
            bitrate: None,
            preset: None,
            audio_codec: None,
            audio_bitrate: None,
            fast_start: true,
        };

        let result = execute(params);
        assert!(result.is_err());
    }
}
