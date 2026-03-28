# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - Video + Audio Processing

### Added

- Video operations: transcode, resize, trim, thumbnail, profile, metadata
- Audio operations: transcode, trim, normalize (LUFS), silence_trim
- Go bridge support for video/audio job types
- Integration tests for video/audio operations
- Test fixtures generated with FFmpeg

### Video Codecs
- H.264, VP8, VP9, AV1

### Audio Codecs
- MP3, AAC, Opus, Vorbis, FLAC, WAV

### Video Profiles
- `web-low`: 480p @ 1M bitrate
- `web-mid`: 720p @ 2.5M bitrate
- `web-high`: 1080p @ 5M bitrate

### Test Coverage
- 20 integration tests passing
- 280 unit tests passing

## [0.3.0] - 2026-03-15

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

## [0.2.0] - 2026-02-01

### Added
- Core image processing engine
- Basic resize and format conversion
- CLI interface

## [0.1.0] - 2026-01-01

### Added
- Initial project setup
- Cargo workspace configuration
