//! Video processing module.
//!
//! Provides video transcoding, resize, trim, thumbnail extraction, and web profiles
//! using FFmpeg via the `ffmpeg-next` crate.

pub mod metadata;
pub mod profiles;
pub mod resize;
pub mod thumbnail;
pub mod transcode;
pub mod trim;

// Re-export params for convenience
pub use metadata::VideoMetadataParams;
pub use profiles::{VideoProfileParams, VideoProfileType};
pub use resize::VideoResizeParams;
pub use thumbnail::VideoThumbnailParams;
pub use transcode::{VideoCodec, VideoTranscodeParams};
pub use trim::VideoTrimParams;
