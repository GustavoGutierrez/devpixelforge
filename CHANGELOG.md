# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-03-28

### Documentation

- Complete README reorganization with clear sections (What, How, Usage)
- Added centered logo image (300px) and tagline
- Added Rust/Go version badges, GPL badge, and platform badge
- Created comprehensive `docs/README.md` (English index)
- Created `docs/examples.md` with working JSON examples for all operations
- Converted `docs/testing/README.md` to English with updated test counts

### License

- Changed from MIT to GNU General Public License v3.0 (GPL-3.0)

## [0.1.4] - 2026-03-15

### Video & Audio Processing

- Video operations: transcode, resize, trim, thumbnail, profile, metadata
- Audio operations: transcode, trim, normalize (LUFS), silence_trim
- Go bridge support for video/audio job types
- Integration tests for video/audio operations

### Video Codecs
- H.264, VP8, VP9, AV1

### Audio Codecs
- MP3, AAC, Opus, Vorbis, FLAC, WAV

### Video Profiles
- `web-low`: 480p @ 1M bitrate
- `web-mid`: 720p @ 2.5M bitrate
- `web-high`: 1080p @ 5M bitrate

## [0.1.3] - 2026-02-15

### Added
- Complete image operations suite (resize, crop, rotate, watermark, etc.)
- Go FFI bridge for MCP integration
- Streaming mode for persistent processes
- Parallel batch processing

### Features
- PNG, JPEG, WebP, GIF, SVG, ICO, AVIF support
- Smart crop with focal point and entropy detection
- Auto-quality optimization via binary search
- EXIF metadata handling

## [0.1.0] - 2026-01-01

### Added
- Initial project setup
- Core image processing engine
- Basic resize and format conversion
- CLI interface
