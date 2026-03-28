# DevPixelForge (dpf)

High-performance image processing engine in Rust, with Go client for MCP integration.

## Architecture

```
┌─────────────────┐      JSON/stdio      ┌──────────────────┐
│   Go Bridge     │◄────────────────────►│   dpf (Rust)     │
│  (Client)       │   stdin/stdout       │  (Image Engine)  │
└─────────────────┘                      └──────────────────┘
```

## Operations

| Operation | Description | CLI Example |
|-----------|-------------|-------------|
| `resize` | Resize to multiple widths or by percentage | `{"operation":"resize","input":"x.png","output_dir":"out","widths":[320,640]}` |
| `resize` (%) | Scale by percentage | `{"operation":"resize","input":"x.png","output_dir":"out","scale_percent":50}` |
| `optimize` | Lossless/lossy compression | `{"operation":"optimize","inputs":["x.png"],"level":"lossy"}` |
| `convert` | Change format | `{"operation":"convert","input":"x.png","output":"x.webp","format":"webp"}` |
| `palette` | Reduce color palette | `{"operation":"palette","input":"x.png","output_dir":"out","max_colors":32}` |
| `favicon` | Generate multi-size favicons | `{"operation":"favicon","input":"logo.svg","output_dir":"out"}` |
| `sprite` | Generate sprite sheet | `{"operation":"sprite","inputs":["a.png","b.png"],"output":"sprite.png"}` |
| `placeholder` | LQIP, dominant color | `{"operation":"placeholder","input":"x.png","kind":"lqip"}` |
| `crop` | Manual or smart crop (center, focal_point, entropy) | `{"operation":"crop","input":"x.png","output":"out.png","rect":{"x":0,"y":0,"width":100,"height":100}}` |
| `rotate` | Rotate 90/180/270 or arbitrary angle, flip | `{"operation":"rotate","input":"x.png","output":"out.png","angle":90}` |
| `watermark` | Text or image overlay with opacity | `{"operation":"watermark","input":"x.png","output":"out.png","text":"© 2024"}` |
| `adjust` | Brightness, contrast, saturation, blur, sharpen | `{"operation":"adjust","input":"x.png","output":"out.png","brightness":0.2}` |
| `quality` | Auto-quality optimization via binary search | `{"operation":"quality","input":"x.png","output":"out.jpg","target_size":10000,"format":"jpeg"}` |
| `srcset` | Responsive image variants with HTML generation | `{"operation":"srcset","input":"x.png","output_dir":"out","widths":[320,640,1024]}` |
| `exif` | Strip, preserve, extract, or auto-orient EXIF | `{"operation":"exif","input":"x.jpg","exif_op":"extract"}` |
| `batch` | Multiple jobs in parallel | `{"operation":"batch","jobs":[...]}` |

## Quick Start

```bash
# Build
make build

# Verify capabilities
./dpf/target/release/dpf caps

# Example resize
./dpf/target/release/dpf process \
  --job '{"operation":"resize","input":"img.png","output_dir":"out","widths":[320,640]}'

# Streaming mode (persistent process)
./dpf/target/release/dpf --stream
```

## Usage Modes

| Mode | Usage | Example |
|------|-------|---------|
| One-shot | Single operation | `dpf process --job '{...}'` |
| Stdin | Pipes/scripts | `echo '{...}' | dpf` |
| Streaming | Persistent process | `dpf --stream` |
| Batch | Multiple parallel jobs | `dpf batch --file jobs.json` |

## Parameters by Operation

### Resize
```json
{
  "operation": "resize",
  "input": "image.png",
  "output_dir": "out",
  "widths": [320, 640, 1024],
  "scale_percent": null,
  "max_height": null,
  "format": "png",
  "quality": 85,
  "filter": "lanczos3",
  "linear_rgb": false,
  "inline": false
}
```

### Crop
```json
{
  "operation": "crop",
  "input": "image.png",
  "output": "cropped.png",
  "rect": { "x": 100, "y": 100, "width": 200, "height": 200 },
  "gravity": null,
  "focal_x": null,
  "focal_y": null,
  "width": null,
  "height": null,
  "format": "png",
  "quality": 85,
  "inline": false
}
```

Gravity modes for smart crop:
- `center` - Crop from center
- `focal_point` - Crop around focal point (use `focal_x`, `focal_y` 0.0-1.0)
- `entropy` - Crop region with highest brightness variance

### Rotate
```json
{
  "operation": "rotate",
  "input": "image.png",
  "output": "rotated.png",
  "angle": 90,
  "angle_f": null,
  "flip": null,
  "auto_orient": false,
  "background": "#FFFFFF",
  "format": "png",
  "quality": 85,
  "inline": false
}
```

- `angle`: 0, 90, 180, 270 degrees
- `angle_f`: Arbitrary rotation (-360 to 360)
- `flip`: "horizontal" or "vertical"

### Watermark
```json
{
  "operation": "watermark",
  "input": "image.png",
  "output": "watermarked.png",
  "text": "© 2024",
  "image": null,
  "position": "bottom-right",
  "opacity": 0.8,
  "font_size": 24,
  "color": "#FFFFFF",
  "offset_x": 10,
  "offset_y": 10,
  "format": "png",
  "quality": 85,
  "inline": false
}
```

Positions: `top-left`, `top-center`, `top-right`, `center-left`, `center`, `center-right`, `bottom-left`, `bottom-center`, `bottom-right`

### Adjust
```json
{
  "operation": "adjust",
  "input": "image.png",
  "output": "adjusted.png",
  "brightness": 0.2,
  "contrast": -0.1,
  "saturation": 0.5,
  "blur": null,
  "sharpen": null,
  "linear_rgb": true,
  "format": "png",
  "quality": 85,
  "inline": false
}
```

Ranges:
- `brightness`, `contrast`, `saturation`: -1.0 to 1.0
- `blur`: 0.0 to 50.0 (sigma)
- `sharpen`: 0.0 to 10.0 (amount)

### Quality (Auto-Optimization)
```json
{
  "operation": "quality",
  "input": "image.png",
  "output": "optimized.jpg",
  "target_size": 10000,
  "tolerance_percent": 5.0,
  "max_iterations": 10,
  "min_quality": 30,
  "max_quality": 95,
  "format": "jpeg",
  "inline": false
}
```

### Srcset
```json
{
  "operation": "srcset",
  "input": "hero.jpg",
  "output_dir": "out",
  "widths": [320, 640, 960, 1280, 1920],
  "densities": [1.0, 2.0],
  "format": "webp",
  "quality": 85,
  "generate_html": true,
  "linear_rgb": true
}
```

### EXIF
```json
{
  "operation": "exif",
  "input": "photo.jpg",
  "output": "cleaned.jpg",
  "exif_op": "strip",
  "mode": "all",
  "keep": null,
  "return_metadata": true,
  "format": "jpeg",
  "quality": 85,
  "inline": false
}
```

Operations:
- `strip` - Remove EXIF data (modes: all, gps, thumbnail, camera)
- `preserve` - Keep only specified tags
- `extract` - Read and return EXIF metadata
- `auto_orient` - Apply EXIF orientation transformation

### Optimize
```json
{
  "operation": "optimize",
  "inputs": ["a.png", "b.jpg"],
  "output_dir": "out",
  "level": "lossless",
  "quality": 80,
  "also_webp": true
}
```

### Palette
```json
{
  "operation": "palette",
  "input": "icon.png",
  "output_dir": "out",
  "max_colors": 32,
  "dithering": 0.5,
  "format": "png"
}
```

## Structure

```
devpixelforge/
├── dpf/                    # Rust Engine
│   └── src/operations/
│       ├── resize.rs       # Resize (with % and linear RGB)
│       ├── crop.rs         # Manual and smart crop
│       ├── rotate.rs       # Rotation and flip
│       ├── watermark.rs    # Text and image overlay
│       ├── adjust.rs       # Brightness, contrast, blur, sharpen
│       ├── quality.rs      # Auto-quality binary search
│       ├── srcset.rs       # Responsive image variants
│       ├── exif_ops.rs     # EXIF strip, preserve, extract
│       ├── optimize.rs     # oxipng/mozjpeg compression
│       ├── convert.rs      # Format conversion
│       ├── palette.rs      # Palette reduction + dithering
│       ├── favicon.rs      # Multi-size favicons
│       ├── sprite.rs       # Sprite sheets
│       └── placeholder.rs  # LQIP, dominant color
├── go-bridge/
│   └── dpf.go             # Go client + StreamClient
├── docs/
│   └── examples/          # JSON examples for each operation
└── Makefile
```

## Requirements

- **Rust** ≥ 1.74
- **Go** ≥ 1.21 (for the bridge)
- For static binary: `musl-tools` (Ubuntu/Debian)

## Video Processing

dpf supports video transcoding, resizing, trimming, and thumbnail extraction:

```bash
# Transcode to H.264
dpf process --job '{"operation":"video","transcode":{"input":"video.mp4","output":"out.mp4","codec":"h264"}}'

# Resize to 720p
dpf process --job '{"operation":"video","resize":{"input":"video.mp4","output":"out.mp4","height":720}}'

# Generate thumbnail
dpf process --job '{"operation":"video","thumbnail":{"input":"video.mp4","output":"thumb.jpg","timestamp":"25%"}}'

# Apply web profile (720p, 2.5M bitrate)
dpf process --job '{"operation":"video","profile":{"input":"video.mp4","output":"out.mp4","profile":"web-mid"}}'
```

## Audio Processing

Audio transcoding, trimming, normalization, and silence removal:

```bash
# Transcode to AAC
dpf process --job '{"operation":"audio","transcode":{"input":"audio.mp3","output":"out.aac","codec":"aac"}}'

# Normalize loudness (YouTube standard: -14 LUFS)
dpf process --job '{"operation":"audio","normalize":{"input":"audio.mp3","output":"out.mp3","target_lufs":-14}}'

# Trim silence from start/end
dpf process --job '{"operation":"audio","silence_trim":{"input":"audio.mp3","output":"out.mp3"}}'
```

### Supported Formats

| Type | Input | Output |
|------|-------|--------|
| Video | mp4, webm, mkv, avi, mov | mp4, webm, mkv |
| Audio | mp3, aac, ogg, wav, flac, opus | mp3, aac, ogg, wav |

### Video Codecs
H.264, VP8, VP9, AV1

### Audio Codecs
MP3, AAC, Opus, Vorbis, FLAC, WAV

### Video Profiles
| Profile | Resolution | Bitrate |
|---------|------------|---------|
| web-low | 480p | 1M |
| web-mid | 720p | 2.5M |
| web-high | 1080p | 5M |

## Features

- ✅ Multi-format: PNG, JPEG, WebP, GIF, SVG, ICO, AVIF
- ✅ Video processing: transcode, resize, trim, thumbnail, profile
- ✅ Audio processing: transcode, trim, normalize (LUFS), silence trim
- ✅ Video codecs: H.264, VP8, VP9, AV1
- ✅ Audio codecs: MP3, AAC, Opus, Vorbis, FLAC, WAV
- ✅ Parallel processing with rayon
- ✅ Streaming mode (persistent process)
- ✅ Palette reduction with dithering
- ✅ Resize by percentage and linear RGB
- ✅ Lossless/lossy optimization
- ✅ SVG→raster via resvg
- ✅ Smart crop (center, focal point, entropy)
- ✅ Text and image watermarks with opacity
- ✅ Brightness/contrast/saturation adjustments
- ✅ Gaussian blur and sharpen
- ✅ Auto-quality optimization via binary search
- ✅ Srcset generation with HTML output
- ✅ EXIF metadata handling

## Test Coverage

| Component | Tests |
|-----------|-------|
| Operations (Rust) | 280+ |
| Integration Tests | 20+ |
| Go Bridge | 16+ |

Run tests:
```bash
# Rust
cargo test

# Go
go test ./...

# Full verification
make test
```

## License

MIT - Ing. Gustavo Gutiérrez
