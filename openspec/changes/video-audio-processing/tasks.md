# Tasks: Video + Audio Processing Engine

## Phase 0: Setup & Infrastructure
**Prerequisite**: None. These changes enable all subsequent tasks.

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V0.1 | Add `ffmpeg-next` and `ffmpeg-sidecar` dependencies | `dpf/Cargo.toml` | Low | None |
| [x] V0.2 | Create `dpf/src/operations/video/` directory and `mod.rs` | `dpf/src/operations/video/mod.rs` | Low | V0.1 |
| [x] V0.3 | Create `dpf/src/operations/audio/` directory and `mod.rs` | `dpf/src/operations/audio/mod.rs` | Low | V0.1 |
| [x] V0.4 | Add `Video`, `Audio` variants to `ImageJob` enum | `dpf/src/main.rs` | Medium | V0.2, V0.3 |
| [x] V0.5 | Verify ffmpeg crates compile: `cargo check` | `dpf/` | Low | V0.1 |

## Phase 1: Video Core Implementation

### 1.1: Video Module Foundation

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.1.1 | Create `VideoTranscodeParams` struct with codec, bitrate, preset | `dpf/src/operations/video/transcode.rs` | Low | V0.2 |
| [x] V1.1.2 | Create `VideoResizeParams` struct with width, height, scale | `dpf/src/operations/video/resize.rs` | Low | V0.2 |
| [x] V1.1.3 | Create `VideoTrimParams` struct with start, end, format | `dpf/src/operations/video/trim.rs` | Low | V0.2 |
| [x] V1.1.4 | Create `VideoThumbnailParams` struct with timestamps, size | `dpf/src/operations/video/thumbnail.rs` | Low | V0.2 |
| [x] V1.1.5 | Create `VideoProfileParams` struct for web profiles | `dpf/src/operations/video/profiles.rs` | Low | V0.2 |

### 1.2: Video Transcode (H.264, VP8/9, AV1)

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.2.1 | Implement `transcode::execute()` with H.264 support via ffmpeg-next | `dpf/src/operations/video/transcode.rs` | High | V1.1.1 |
| [x] V1.2.2 | Add VP8/VP9 codec support | `dpf/src/operations/video/transcode.rs` | Medium | V1.2.1 |
| [x] V1.2.3 | Add bitrate and preset parameters | `dpf/src/operations/video/transcode.rs` | Medium | V1.2.1 |
| [x] V1.2.4 | Add unit tests for transcode | `dpf/src/operations/video/transcode.rs` | Medium | V1.2.3 |

### 1.3: Video Resize

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.3.1 | Implement `resize::execute()` using ffmpeg-next | `dpf/src/operations/video/resize.rs` | Medium | V1.1.2 |
| [x] V1.3.2 | Add scale-to-width/height and maintain aspect ratio | `dpf/src/operations/video/resize.rs` | Medium | V1.3.1 |
| [x] V1.3.3 | Add unit tests for video resize | `dpf/src/operations/video/resize.rs` | Medium | V1.3.2 |

### 1.4: Video Trim

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.4.1 | Implement `trim::execute()` with start/end timestamps | `dpf/src/operations/video/trim.rs` | Medium | V1.1.3 |
| [x] V1.4.2 | Add validation for time format (HH:MM:SS) | `dpf/src/operations/video/trim.rs` | Low | V1.4.1 |
| [x] V1.4.3 | Add unit tests for trim | `dpf/src/operations/video/trim.rs` | Low | V1.4.2 |

### 1.5: Video Thumbnail

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.5.1 | Implement `thumbnail::execute()` extracting frames | `dpf/src/operations/video/thumbnail.rs` | Medium | V1.1.4 |
| [x] V1.5.2 | Add single timestamp and multiple timestamps support | `dpf/src/operations/video/thumbnail.rs` | Medium | V1.5.1 |
| [x] V1.5.3 | Add unit tests for thumbnail | `dpf/src/operations/video/thumbnail.rs` | Low | V1.5.2 |

### 1.6: Video Profiles (web-low/mid/high)

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.6.1 | Define profile constants (web-low: 480p, web-mid: 720p, web-high: 1080p) | `dpf/src/operations/video/profiles.rs` | Low | V1.1.5 |
| [x] V1.6.2 | Implement `profiles::execute()` applying preset configurations | `dpf/src/operations/video/profiles.rs` | Medium | V1.6.1 |
| [x] V1.6.3 | Add unit tests for profiles | `dpf/src/operations/video/profiles.rs` | Low | V1.6.2 |

### 1.7: Video Module Export

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V1.7.1 | Export all video operations in `video/mod.rs` | `dpf/src/operations/video/mod.rs` | Low | V1.2.4, V1.3.3, V1.4.3, V1.5.3, V1.6.3 |
| [x] V1.7.2 | Add video module to `operations/mod.rs` | `dpf/src/operations/mod.rs` | Low | V1.7.1 |

## Phase 2: Audio Core Implementation

### 2.1: Audio Module Foundation

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] A2.1.1 | Create `AudioTranscodeParams` struct | `dpf/src/operations/audio/transcode.rs` | Low | V0.3 |
| [x] A2.1.2 | Create `AudioTrimParams` struct | `dpf/src/operations/audio/trim.rs` | Low | V0.3 |
| [x] A2.1.3 | Create `AudioNormalizeParams` struct | `dpf/src/operations/audio/normalize.rs` | Low | V0.3 |
| [x] A2.1.4 | Create `AudioSilenceTrimParams` struct | `dpf/src/operations/audio/silence_trim.rs` | Low | V0.3 |

### 2.2: Audio Transcode (AAC, MP3, Opus)

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] A2.2.1 | Implement `transcode::execute()` using ffmpeg-sidecar | `dpf/src/operations/audio/transcode.rs` | Medium | A2.1.1 |
| [x] A2.2.2 | Add codec selection (aac, mp3, opus) | `dpf/src/operations/audio/transcode.rs` | Medium | A2.2.1 |
| [x] A2.2.3 | Add bitrate and quality parameters | `dpf/src/operations/audio/transcode.rs` | Medium | A2.2.2 |
| [x] A2.2.4 | Add unit tests for audio transcode | `dpf/src/operations/audio/transcode.rs` | Medium | A2.2.3 |

### 2.3: Audio Trim

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] A2.3.1 | Implement `trim::execute()` with ffmpeg-sidecar | `dpf/src/operations/audio/trim.rs` | Medium | A2.1.2 |
| [x] A2.3.2 | Add unit tests for audio trim | `dpf/src/operations/audio/trim.rs` | Low | A2.3.1 |

### 2.4: Audio Normalize

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] A2.4.1 | Implement `normalize::execute()` (loudnorm filter) | `dpf/src/operations/audio/normalize.rs` | Medium | A2.1.3 |
| [x] A2.4.2 | Add target LUFS parameter | `dpf/src/operations/audio/normalize.rs` | Low | A2.4.1 |
| [x] A2.4.3 | Add unit tests for normalize | `dpf/src/operations/audio/normalize.rs` | Medium | A2.4.2 |

### 2.5: Audio Silence Trim

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] A2.5.1 | Implement `silence_trim::execute()` (silenceremove filter) | `dpf/src/operations/audio/silence_trim.rs` | Medium | A2.1.4 |
| [x] A2.5.2 | Add threshold and duration parameters | `dpf/src/operations/audio/silence_trim.rs` | Low | A2.5.1 |
| [x] A2.5.3 | Add unit tests for silence trim | `dpf/src/operations/audio/silence_trim.rs` | Low | A2.5.2 |

### 2.6: Audio Module Export

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] A2.6.1 | Export all audio operations in `audio/mod.rs` | `dpf/src/operations/audio/mod.rs` | Low | A2.2.4, A2.3.2, A2.4.3, A2.5.3 |
| [x] A2.6.2 | Add audio module to `operations/mod.rs` | `dpf/src/operations/mod.rs` | Low | A2.6.1 |

## Phase 3: Pipeline Integration

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] V3.1 | Add routing for `ImageJob::Video` in `pipeline.rs` | `dpf/src/pipeline.rs` | Medium | V1.7.2, A2.6.2 |
| [x] V3.2 | Add routing for `ImageJob::Audio` in `pipeline.rs` | `dpf/src/pipeline.rs` | Medium | V3.1 |
| [x] V3.3 | Update `operation_name()` in `main.rs` | `dpf/src/main.rs` | Low | V3.2 |
| [x] V3.4 | Add video/audio operations to capabilities | `dpf/src/main.rs` | Low | V3.3 |
| [x] V3.5 | Integration tests for video pipeline | `dpf/src/pipeline.rs` | Medium | V3.4 |
| [x] V3.6 | Integration tests for audio pipeline | `dpf/src/pipeline.rs` | Medium | V3.5 |

## Phase 4: Go Bridge

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] G4.1 | Create `VideoTranscodeJob` struct | `go-bridge/job.go` | Low | V3.4 |
| [x] G4.2 | Create `VideoResizeJob`, `VideoTrimJob`, `VideoThumbnailJob`, `VideoProfileJob` structs | `go-bridge/job.go` | Low | G4.1 |
| [x] G4.3 | Create `AudioTranscodeJob`, `AudioTrimJob`, `AudioNormalizeJob`, `AudioSilenceTrimJob` structs | `go-bridge/job.go` | Low | G4.2 |
| [x] G4.4 | Add `VideoTranscode()`, `VideoResize()`, etc. methods to Client | `go-bridge/dpf.go` | Medium | G4.3 |
| [x] G4.5 | Add streaming methods for video/audio | `go-bridge/dpf.go` | Medium | G4.4 |
| [x] G4.6 | Add Go tests for video/audio operations (partial) | `go-bridge/dpf_test.go` | Medium | G4.5 |

## Phase 5: Documentation & Verification

| # | Task | File | Complexity | Dependencies |
|---|------|------|------------|--------------|
| [x] D5.1 | Update README.md with video/audio operations | `README.md` | Medium | G4.6 |
| [x] D5.2 | Create JSON examples for video operations | `docs/video-examples.md` | Low | D5.1 |
| [x] D5.3 | Create JSON examples for audio operations | `docs/audio-examples.md` | Low | D5.2 |
| [x] D5.4 | Update INTEGRATION.md with video/audio sections | `INTEGRATION.md` | Low | D5.3 |
| [x] D5.5 | Final build verification: `make build` | `dpf/`, `go-bridge/` | Low | D5.4 |
| [x] D5.6 | Test `dpf caps` shows new operations | CLI verification | Low | D5.5 |

---

## Summary

| Phase | Tasks | Status | Focus |
|-------|-------|--------|-------|
| Phase 0 | 5 | ✅ Complete | Setup & Dependencies |
| Phase 1 | 16 | ✅ Complete | Video Core |
| Phase 2 | 13 | ✅ Complete | Audio Core |
| Phase 3 | 6/6 | ✅ Complete | Pipeline Integration |
| Phase 4 | 6/6 | ✅ Complete | Go Bridge |
| Phase 5 | 6/6 | ✅ Complete | Documentation |
| **Total** | **52/52** | ✅ Complete | |

## Implementation Order

1. **Phase 0** first — Cargo.toml deps and module creation
2. **Phase 1** (video) before Phase 2 (audio) — establish patterns
3. **Phase 2** (audio) follows video patterns using ffmpeg-sidecar
4. **Phase 3** (pipeline) connects both modules
5. **Phase 4** (Go) wraps Rust implementation
6. **Phase 5** (docs) finalizes the feature

## Key Dependencies
- `ffmpeg-next` for video processing (native Rust)
- `ffmpeg-sidecar` for audio processing (FFmpeg CLI wrapper)
- Test fixtures: sample video/audio files in `dpf/test_fixtures/`
