# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-05-03

### Added

- Enabled GFM math rendering (`$...$`, `$$...$$`) via glyphweaveforge `math` feature with Typst backend.
- Enabled Rust-native Mermaid subset diagram rendering via glyphweaveforge `mermaid` feature (no Node/npm required).
- Added `markdown_to_pdf_math` and `markdown_to_pdf_mermaid` feature flags to `caps` metadata.
- Added `ThemeOverride` struct to Go bridge (`BodyFontSize`, `CodeFontSize`, `HeadingScale`, `MarginMM`) for typed theme customization.
- Added `TestThemeOverrideSerialization` and `TestClientMarkdownToPDFWithThemeOverride` Go tests.
- Documented math, mermaid, and theme customization in the integration guide with Go examples.

### Changed

- Upgraded glyphweaveforge from `0.1.3` to `0.1.6` (features: `renderer-typst`, `math`, `mermaid`).
- Go bridge `Client.MarkdownToPDF` and `StreamClient.MarkdownToPDF` auto-apply `ThemeOverride` into `theme_config` JSON.
- Updated integration guide with theme customization fields, math/mermaid support, and `ThemeOverride` usage from Go.

## [0.4.2] - 2026-04-03

### Changed

- Upgraded the Markdown-to-PDF integration to GlyphWeaveForge `0.1.3` and kept the vendored Typst/font override aligned with the new release.
- Refactored the Rust integration to keep using GlyphWeaveForge's public `Forge` builder API with explicit Typst backend selection for file, directory, and in-memory conversion flows.
- Added `resource_files` support for inline Markdown assets so local href mappings can be injected through GlyphWeaveForge's public resource resolver.
- Revalidated all built-in themes (`invoice`, `scientific_article`, `professional`, `engineering`, `informational`) and refreshed the local release-readiness notes.

## [0.4.1] - 2026-04-03

### Changed

- Upgraded the Markdown-to-PDF integration to GlyphWeaveForge `0.1.2` after upstream deprecation of earlier releases.
- Refreshed vendored regression coverage and lockfiles to keep the Typst escaping patch validated.

## [0.4.0] - 2026-04-02

### Added

- Added `markdown_to_pdf` to the Rust JSON/CLI protocol with GlyphWeaveForge `0.1.2` and explicit Typst rendering.
- Added Go bridge support for `MarkdownToPDFJob`, `Client.MarkdownToPDF`, and `StreamClient.MarkdownToPDF`.
- Added Markdown-to-PDF integration tests, examples, schema docs, and inline/file usage guidance.
- Added repository-backed Markdown-to-PDF CLI validation docs plus generated `README.md` and `AGENTS.md` PDF artifacts using the built-in `engineering` theme.

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
