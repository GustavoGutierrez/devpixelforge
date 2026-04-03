# Go Bridge Integration Guide

This guide explains how to integrate devpixelforge (dpf) into any Go project.

---

## 1. What You Need to Copy

### Files to Copy

```bash
# In your Go project
mkdir -p bin internal/dpf

# Copy Rust binary (choose one)
# Regular build:
cp devpixelforge/dpf/target/release/dpf ./bin/dpf
# Static build (no dependencies):
cp devpixelforge/dpf/target/x86_64-unknown-linux-musl/release/dpf ./bin/dpf

# Copy Go client files
cp devpixelforge/go-bridge/dpf.go ./internal/dpf/
cp devpixelforge/go-bridge/markdown_to_pdf_job.go ./internal/dpf/
cp devpixelforge/go-bridge/video_job.go ./internal/dpf/
cp devifforge/go-bridge/audio_job.go ./internal/dpf/
```

### Final Structure

```
your-project/
├── bin/
│   └── dpf                    # Rust binary
├── internal/
│   └── dpf/
│       ├── dpf.go             # Main client
│       ├── markdown_to_pdf_job.go
│       ├── video_job.go       # Video types
│       └── audio_job.go       # Audio types
└── main.go
```

---

## 2. Quick Start

### Basic Usage

```go
package main

import (
    "context"
    "log"
    "time"
    "your-project/internal/dpf"
)

func main() {
    // Create client
    client := dpf.NewClient("./bin/dpf")
    client.SetTimeout(60 * time.Second)
    ctx := context.Background()

    // Video Transcode
    result, err := client.VideoTranscode(ctx, &dpf.VideoTranscodeJob{
        Input:   "input.mp4",
        Output:  "output.mp4",
        Codec:   "h264",
        Bitrate: "2M",
    })
    if err != nil {
        log.Fatal(err)
    }
    log.Printf("Success: %+v", result)
}
```

---

## 3. Markdown to PDF

Use `MarkdownToPDF` when an MCP server needs deterministic PDF output while keeping the existing `JobResult` envelope.

### Inline Markdown to inline PDF

```go
markdown := "# MCP Report\n\nGenerated directly from memory."

result, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
    MarkdownText: &markdown,
    Inline:       true,
    Theme:        func(s string) *string { return &s }("professional"),
})
if err != nil {
    log.Fatal(err)
}

// result.Outputs[0].DataBase64 contains the PDF bytes.
```

### Markdown file to PDF file

```go
result, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
    Input:    "docs/report.md",
    Output:   "/tmp/report.pdf",
    PageSize: func(s string) *string { return &s }("letter"),
    Theme:    func(s string) *string { return &s }("engineering"),
})
if err != nil {
    log.Fatal(err)
}
```

Rules:

- Provide exactly one input source: `Input`, `MarkdownText`, or `MarkdownBase64`.
- Provide at least one output mode: `Output`, `OutputDir`, or `Inline`.
- When using `OutputDir` with inline Markdown, also provide `FileName`.
- PDF metadata stays inside `result.Metadata`; no new response envelope is introduced.

---

## 4. Video Operations

### Video Transcode

Convert video to different codec (H.264, VP8, VP9, AV1):

```go
result, err := client.VideoTranscode(ctx, &dpf.VideoTranscodeJob{
    Input:      "video.mp4",
    Output:     "output.mp4",
    Codec:      "h264",      // h264, h265, vp8, vp9, av1
    Bitrate:    "2M",
    Preset:     "medium",    // ultrafast, fast, medium, slow, veryslow
    CRF:        func(u uint8) *uint8 { return &u }(23),
    AudioCodec: "aac",
})

// Available codecs:
// - h264: Best compatibility
// - h265: Better compression (HEVC)
// - vp8, vp9: WebM support
// - av1: Best compression, newest
```

### Video Resize

Resize video maintaining aspect ratio:

```go
result, err := client.VideoResize(ctx, &dpf.VideoResizeJob{
    Input:          "video.mp4",
    Output:         "video_720p.mp4",
    Width:          1280,      // optional
    Height:         720,       // required if no width
    MaintainAspect: true,      // default: true
})
```

### Video Trim

Extract a time range from video:

```go
result, err := client.VideoTrim(ctx, &dpf.VideoTrimJob{
    Input:  "video.mp4",
    Output: "clip.mp4",
    Start:  10.5,    // seconds
    End:    60.0,    // seconds
})
```

### Video Thumbnail

Extract a frame as image:

```go
result, err := client.VideoThumbnail(ctx, &dpf.VideoThumbnailJob{
    Input:     "video.mp4",
    Output:    "poster.jpg",
    Timestamp: "25%",    // or "30.5" for seconds
    Format:    "jpeg",   // jpeg, png, webp
    Quality:   func(u uint8) *uint8 { return &u }(85),
})
```

### Video Profile

Apply web-optimized encoding profile:

```go
result, err := client.VideoProfile(ctx, &dpf.VideoProfileJob{
    Input:   "video.mp4",
    Output:  "output.mp4",
    Profile: "web-mid",   // web-low, web-mid, web-high
})

// Available profiles:
// - web-low:  480p @ 1M bitrate
// - web-mid:  720p @ 2.5M bitrate
// - web-high: 1080p @ 5M bitrate
```

### Video Metadata

Extract video metadata:

```go
result, err := client.VideoMetadata(ctx, &dpf.VideoMetadataJob{
    Input: "video.mp4",
})
// result.Metadata contains duration, codec, resolution, etc.
```

---

## 5. Audio Operations

### Audio Transcode

Convert between audio formats:

```go
result, err := client.AudioTranscode(ctx, &dpf.AudioTranscodeJob{
    Input:      "audio.wav",
    Output:     "audio.mp3",
    Codec:      "mp3",       // mp3, aac, opus, vorbis, flac, wav
    Bitrate:    "192k",
    SampleRate: func(u uint32) *uint32 { return &u }(48000),
    Channels:   func(u uint32) *uint32 { return &u }(2),
    Quality:    func(u uint8) *uint8 { return &u }(5),  // 0-10
})

// Available codecs:
// - mp3:    Most compatible
// - aac:    Better quality at same bitrate
// - opus:   Best for speech
// - vorbis: Open source alternative
// - flac:   Lossless
// - wav:    Uncompressed
```

### Audio Trim

Trim audio by timestamps:

```go
result, err := client.AudioTrim(ctx, &dpf.AudioTrimJob{
    Input:  "podcast.mp3",
    Output: "segment.mp3",
    Start:  60.0,    // seconds
    End:    150.0,   // seconds
})
```

### Audio Normalize

Normalize loudness to target LUFS:

```go
result, err := client.AudioNormalize(ctx, &dpf.AudioNormalizeJob{
    Input:       "audio.mp3",
    Output:      "audio_normalized.mp3",
    TargetLUFS:  -14.0,
    Threshold:   func(f float64) *float64 { return &f }(-50.0),
})

// LUFS targets:
// -14 LUFS: YouTube
// -16 LUFS: Spotify
// -23 LUFS: EBU R128 standard
```

### Audio Silence Trim

Remove leading/trailing silence:

```go
result, err := client.AudioSilenceTrim(ctx, &dpf.AudioSilenceTrimJob{
    Input:       "audio_with_silence.mp3",
    Output:      "audio_clean.mp3",
    ThresholdDB: func(f float64) *float64 { return &f }(-40.0),  // dB
    MinDuration: func(f float64) *float64 { return &f }(0.5),    // seconds
})
```

---

## 6. Streaming Client (Recommended for Servers)

The same helper exists on the persistent client:

```go
result, err := sc.MarkdownToPDF(&dpf.MarkdownToPDFJob{
    MarkdownText: func(s string) *string { return &s }("# Streamed PDF"),
    Inline:       true,
})
```

For high-throughput scenarios, use `StreamClient` to reuse the Rust process:

```go
// Initialize once (e.g., at server startup)
sc, err := dpf.NewStreamClient("./bin/dpf")
if err != nil {
    log.Fatal(err)
}
defer sc.Close()  // Clean up on shutdown

// Video transcode (concurrent-safe)
result1, _ := sc.VideoTranscode(&dpf.VideoTranscodeJob{...})

// Audio normalize
result2, _ := sc.AudioNormalize(&dpf.AudioNormalizeJob{...})

// Mix operations
result3, _ := sc.Execute(&dpf.ResizeJob{
    Operation: "resize",
    Input:     "image.jpg",
    OutputDir: "output",
    Widths:    []uint32{320, 640, 1024},
})
```

---

## 6. Batch Operations

Execute multiple operations in parallel:

```go
result, err := sc.Execute(&dpf.BatchJob{
    Operation: "batch",
    Jobs: []any{
        dpf.VideoTranscodeJob{
            Input:  "video.mp4",
            Output: "video_720p.mp4",
            Height: 720,
        },
        dpf.VideoThumbnailJob{
            Input:     "video.mp4",
            Output:    "poster.jpg",
            Timestamp: "25%",
        },
        dpf.AudioNormalizeJob{
            Input:      "audio.mp3",
            Output:     "audio_normalized.mp3",
            TargetLUFS: -14.0,
        },
    },
})
```

---

## 7. MCP Server Integration

Example MCP server handlers:

```go
type MCPServer struct {
    imgClient *dpf.StreamClient
}

func NewMCPServer(binaryPath string) (*MCPServer, error) {
    sc, err := dpf.NewStreamClient(binaryPath)
    if err != nil {
        return nil, err
    }
    return &MCPServer{imgClient: sc}, nil
}

func (s *MCPServer) Shutdown() {
    s.imgClient.Close()
}

// Video transcode handler
func (s *MCPServer) handleVideoTranscode(ctx context.Context, params json.RawMessage) (any, error) {
    var req struct {
        Input   string `json:"input"`
        Output  string `json:"output"`
        Codec   string `json:"codec"`
        Bitrate string `json:"bitrate,omitempty"`
    }
    if err := json.Unmarshal(params, &req); err != nil {
        return nil, err
    }
    return s.imgClient.VideoTranscode(&dpf.VideoTranscodeJob{
        Input:   req.Input,
        Output:  req.Output,
        Codec:   req.Codec,
        Bitrate: req.Bitrate,
    })
}

// Audio normalize handler
func (s *MCPServer) handleAudioNormalize(ctx context.Context, params json.RawMessage) (any, error) {
    var req struct {
        Input      string  `json:"input"`
        Output     string  `json:"output"`
        TargetLUFS float64 `json:"target_lufs"`
    }
    if err := json.Unmarshal(params, &req); err != nil {
        return nil, err
    }
    return s.imgClient.AudioNormalize(&dpf.AudioNormalizeJob{
        Input:      req.Input,
        Output:     req.Output,
        TargetLUFS: req.TargetLUFS,
    })
}
```

---

## 8. System Requirements

### FFmpeg (Required for Video/Audio)

dpf uses FFmpeg CLI for video and audio processing:

```bash
# Linux
sudo apt install ffmpeg

# macOS
brew install ffmpeg

# Verify
ffmpeg -version
```

**Minimum version:** FFmpeg 6.0+

---

## 9. Direct JSON Protocol

You can also use the JSON protocol directly without the Go client:

```bash
# Video transcode
dpf process --job '{
  "operation": "video",
  "transcode": {
    "input": "in.mp4",
    "output": "out.mp4",
    "codec": "h264"
  }
}'

# Audio normalize
dpf process --job '{
  "operation": "audio",
  "normalize": {
    "input": "in.mp3",
    "output": "out.mp3",
    "target_lufs": -14
  }
}'
```

---

## 10. Job Types Reference

| Type | Operation | Description |
|------|-----------|-------------|
| `VideoTranscodeJob` | `video` + `transcode` | Convert video codec |
| `VideoResizeJob` | `video` + `resize` | Resize video dimensions |
| `VideoTrimJob` | `video` + `trim` | Extract time range |
| `VideoThumbnailJob` | `video` + `thumbnail` | Extract frame as image |
| `VideoProfileJob` | `video` + `profile` | Apply web profile |
| `VideoMetadataJob` | `video` + `metadata` | Extract metadata |
| `AudioTranscodeJob` | `audio` + `transcode` | Convert audio format |
| `AudioTrimJob` | `audio` + `trim` | Trim by time range |
| `AudioNormalizeJob` | `audio` + `normalize` | Normalize loudness |
| `AudioSilenceTrimJob` | `audio` + `silence_trim` | Remove silence |

---

## 11. Error Handling

```go
result, err := client.VideoTranscode(ctx, &dpf.VideoTranscodeJob{
    Input:  "video.mp4",
    Output: "output.mp4",
    Codec:  "h264",
})

if err != nil {
    // Handle error
    log.Printf("Error: %v", err)
    return
}

// Check result
if !result.Success {
    log.Printf("Operation failed: %s", result.Operation)
    return
}

log.Printf("Success in %dms: %+v", result.ElapsedMs, result.Outputs)
```

---

## 12. Timeout Configuration

```go
client := dpf.NewClient("./bin/dpf")

// Default: 30 seconds
client.SetTimeout(30 * time.Second)

// For video processing (can be slow)
client.SetTimeout(5 * time.Minute)

// For quick operations
client.SetTimeout(10 * time.Second)
```
