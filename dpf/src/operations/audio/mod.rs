//! Audio processing module.
//!
//! Provides audio transcoding, trimming, normalization, and silence removal
//! using FFmpeg CLI via the `ffmpeg-sidecar` crate.

pub mod normalize;
pub mod silence_trim;
pub mod transcode;
pub mod trim;

pub use normalize::AudioNormalizeParams;
pub use silence_trim::AudioSilenceTrimParams;
pub use transcode::{AudioCodec, AudioTranscodeParams};
pub use trim::AudioTrimParams;
