# JSON Examples

Working JSON examples for all DevPixelForge operations.

---

## Image Operations

## Document Operations

### Markdown to PDF

**Markdown file to PDF file:**
```json
{
  "operation": "markdown_to_pdf",
  "input": "docs/report.md",
  "output": "/tmp/report.pdf",
  "page_size": "letter",
  "layout_mode": "paged",
  "theme": "engineering"
}
```

**Inline Markdown to inline PDF:**
```json
{
  "operation": "markdown_to_pdf",
  "markdown_text": "# Inline Report\n\nRendered from memory.",
  "inline": true,
  "theme": "professional",
  "resource_files": {
    "logo.png": "./assets/logo.png"
  },
  "theme_config": {
    "margin_mm": 14.0
  }
}
```

**Inline Markdown to output directory:**
```json
{
  "operation": "markdown_to_pdf",
  "markdown_base64": "IyBSZXBvcnQKClJlbmRlcmVkIGZyb20gYmFzZTY0Lg==",
  "output_dir": "/tmp/reports",
  "file_name": "report.pdf",
  "theme": "invoice"
}
```

The operation accepts exactly one source from `input`, `markdown_text`, or `markdown_base64`. At least one output mode is required: `output`, `output_dir`, or `inline=true`.

Prefer file-based input when Markdown references local images. If inline Markdown must resolve assets, provide `resource_files` so dpf can inject them through GlyphWeaveForge's public resource resolver.

For a real repository validation flow that converts `README.md`, `AGENTS.md`, and all five built-in themes into PDFs, see [`validation/markdown-to-pdf/README.md`](validation/markdown-to-pdf/README.md).

---

### Resize

**Resize to multiple widths:**
```json
{
  "operation": "resize",
  "input": "photo.jpg",
  "output_dir": "dist/images",
  "widths": [320, 640, 1024, 1920],
  "format": "webp",
  "quality": 85,
  "linear_rgb": true
}
```

**Resize by percentage:**
```json
{
  "operation": "resize",
  "input": "photo.jpg",
  "output_dir": "dist/images",
  "scale_percent": 50,
  "format": "png"
}
```

**Resize with max dimensions:**
```json
{
  "operation": "resize",
  "input": "photo.jpg",
  "output_dir": "dist/images",
  "widths": [1200],
  "max_height": 800,
  "format": "jpeg",
  "quality": 90
}
```

### Crop

**Manual crop:**
```json
{
  "operation": "crop",
  "input": "photo.jpg",
  "output": "cropped.jpg",
  "rect": {
    "x": 100,
    "y": 50,
    "width": 800,
    "height": 600
  },
  "format": "jpeg",
  "quality": 90
}
```

**Center crop:**
```json
{
  "operation": "crop",
  "input": "photo.jpg",
  "output": "center-crop.jpg",
  "width": 800,
  "height": 600,
  "gravity": "center"
}
```

**Focal point crop:**
```json
{
  "operation": "crop",
  "input": "photo.jpg",
  "output": "focal-crop.jpg",
  "width": 400,
  "height": 400,
  "gravity": "focal_point",
  "focal_x": 0.75,
  "focal_y": 0.25
}
```

**Entropy-based crop:**
```json
{
  "operation": "crop",
  "input": "photo.jpg",
  "output": "entropy-crop.jpg",
  "width": 800,
  "height": 400,
  "gravity": "entropy"
}
```

### Rotate

**Rotate 90 degrees:**
```json
{
  "operation": "rotate",
  "input": "photo.jpg",
  "output": "rotated.jpg",
  "angle": 90,
  "format": "jpeg",
  "quality": 90
}
```

**Rotate with fill:**
```json
{
  "operation": "rotate",
  "input": "photo.jpg",
  "output": "rotated-filled.jpg",
  "angle": 45,
  "background": "#FFFFFF",
  "format": "png"
}
```

**Flip horizontal:**
```json
{
  "operation": "rotate",
  "input": "photo.jpg",
  "output": "flipped.jpg",
  "flip": "horizontal"
}
```

**Auto-orient (from EXIF):**
```json
{
  "operation": "rotate",
  "input": "photo.jpg",
  "output": "oriented.jpg",
  "auto_orient": true
}
```

### Watermark

**Text watermark:**
```json
{
  "operation": "watermark",
  "input": "photo.jpg",
  "output": "watermarked.jpg",
  "text": "© 2024 DevPixelForge",
  "position": "bottom-right",
  "opacity": 0.7,
  "font_size": 24,
  "color": "#FFFFFF",
  "offset_x": 20,
  "offset_y": 20
}
```

**Image watermark:**
```json
{
  "operation": "watermark",
  "input": "photo.jpg",
  "output": "watermarked.jpg",
  "image": "logo.png",
  "position": "top-right",
  "opacity": 0.5,
  "offset_x": 10,
  "offset_y": 10
}
```

### Adjust

**Brightness and contrast:**
```json
{
  "operation": "adjust",
  "input": "photo.jpg",
  "output": "adjusted.jpg",
  "brightness": 0.15,
  "contrast": 0.1,
  "format": "jpeg",
  "quality": 90
}
```

**Color adjustments:**
```json
{
  "operation": "adjust",
  "input": "photo.jpg",
  "output": "adjusted.jpg",
  "saturation": 0.3,
  "brightness": 0.05
}
```

**Blur and sharpen:**
```json
{
  "operation": "adjust",
  "input": "photo.jpg",
  "output": "processed.jpg",
  "blur": 2.0,
  "sharpen": 1.5,
  "linear_rgb": true
}
```

### Quality (Auto-Optimization)

**Target file size:**
```json
{
  "operation": "quality",
  "input": "photo.jpg",
  "output": "optimized.jpg",
  "target_size": 50000,
  "tolerance_percent": 5,
  "max_iterations": 10,
  "format": "jpeg",
  "inline": false
}
```

**With quality bounds:**
```json
{
  "operation": "quality",
  "input": "photo.jpg",
  "output": "optimized.jpg",
  "target_size": 30000,
  "min_quality": 40,
  "max_quality": 95,
  "format": "jpeg"
}
```

### Srcset

**Responsive images with HTML:**
```json
{
  "operation": "srcset",
  "input": "hero.jpg",
  "output_dir": "dist/images",
  "widths": [320, 640, 960, 1280, 1920],
  "densities": [1.0, 2.0],
  "format": "webp",
  "quality": 85,
  "generate_html": true,
  "linear_rgb": true
}
```

### EXIF

**Strip all EXIF:**
```json
{
  "operation": "exif",
  "input": "photo.jpg",
  "output": "cleaned.jpg",
  "exif_op": "strip",
  "mode": "all",
  "format": "jpeg",
  "quality": 90
}
```

**Strip GPS only:**
```json
{
  "operation": "exif",
  "input": "photo.jpg",
  "output": "gps-removed.jpg",
  "exif_op": "strip",
  "mode": "gps",
  "format": "jpeg"
}
```

**Extract metadata:**
```json
{
  "operation": "exif",
  "input": "photo.jpg",
  "exif_op": "extract",
  "return_metadata": true
}
```

**Auto-orient:**
```json
{
  "operation": "exif",
  "input": "photo.jpg",
  "output": "oriented.jpg",
  "exif_op": "auto_orient"
}
```

### Optimize

**Lossless PNG optimization:**
```json
{
  "operation": "optimize",
  "inputs": ["a.png", "b.png"],
  "output_dir": "dist/images",
  "level": "lossless"
}
```

**Lossy JPEG optimization:**
```json
{
  "operation": "optimize",
  "inputs": ["a.jpg", "b.jpg"],
  "output_dir": "dist/images",
  "level": "lossy",
  "quality": 85
}
```

**Generate WebP variants:**
```json
{
  "operation": "optimize",
  "inputs": ["a.png", "b.png"],
  "output_dir": "dist/images",
  "level": "lossy",
  "also_webp": true,
  "quality": 80
}
```

### Convert

**PNG to WebP:**
```json
{
  "operation": "convert",
  "input": "image.png",
  "output": "image.webp",
  "format": "webp",
  "quality": 85
}
```

**JPEG to AVIF:**
```json
{
  "operation": "convert",
  "input": "image.jpg",
  "output": "image.avif",
  "format": "avif",
  "quality": 80
}
```

### Palette

**Reduce to 32 colors:**
```json
{
  "operation": "palette",
  "input": "icon.png",
  "output_dir": "dist/icons",
  "max_colors": 32,
  "format": "png"
}
```

**With dithering:**
```json
{
  "operation": "palette",
  "input": "icon.png",
  "output_dir": "dist/icons",
  "max_colors": 16,
  "dithering": 0.8,
  "format": "png"
}
```

### Favicon

**Generate from logo:**
```json
{
  "operation": "favicon",
  "input": "logo.svg",
  "output_dir": "dist/favicon",
  "sizes": [16, 32, 48, 180, 192, 512]
}
```

### Sprite

**Generate sprite sheet:**
```json
{
  "operation": "sprite",
  "inputs": ["icon1.png", "icon2.png", "icon3.png", "icon4.png"],
  "output": "sprites.png",
  "columns": 2,
  "padding": 5
}
```

### Placeholder

**LQIP (Low Quality Image Placeholder):**
```json
{
  "operation": "placeholder",
  "input": "photo.jpg",
  "output": "lqip.txt",
  "kind": "lqip",
  "width": 10,
  "format": "jpeg",
  "quality": 30
}
```

**Dominant color:**
```json
{
  "operation": "placeholder",
  "input": "photo.jpg",
  "output": "color.txt",
  "kind": "dominant_color"
}
```

---

## Video Operations

### Video Transcode

**H.264:**
```json
{
  "operation": "video",
  "transcode": {
    "input": "input.mkv",
    "output": "output.mp4",
    "codec": "h264"
  }
}
```

**VP9:**
```json
{
  "operation": "video",
  "transcode": {
    "input": "input.mp4",
    "output": "output.webm",
    "codec": "vp9"
  }
}
```

**AV1:**
```json
{
  "operation": "video",
  "transcode": {
    "input": "input.mp4",
    "output": "output.mkv",
    "codec": "av1",
    "preset": "medium"
  }
}
```

### Video Resize

**720p:**
```json
{
  "operation": "video",
  "resize": {
    "input": "input.mp4",
    "output": "output.mp4",
    "height": 720
  }
}
```

**Specific dimensions:**
```json
{
  "operation": "video",
  "resize": {
    "input": "input.mp4",
    "output": "output.mp4",
    "width": 1280,
    "height": 720,
    "maintain_aspect": true
  }
}
```

### Video Trim

**By seconds:**
```json
{
  "operation": "video",
  "trim": {
    "input": "input.mp4",
    "output": "clip.mp4",
    "start": 30,
    "end": 90
  }
}
```

### Video Thumbnail

**At 25%:**
```json
{
  "operation": "video",
  "thumbnail": {
    "input": "video.mp4",
    "output": "thumb.jpg",
    "timestamp": "25%",
    "format": "jpeg",
    "quality": 85
  }
}
```

**At 10 seconds:**
```json
{
  "operation": "video",
  "thumbnail": {
    "input": "video.mp4",
    "output": "thumb.png",
    "timestamp": "10",
    "format": "png"
  }
}
```

### Video Profile

**Web Low (480p):**
```json
{
  "operation": "video",
  "profile": {
    "input": "input.mp4",
    "output": "output-480p.mp4",
    "profile": "web-low"
  }
}
```

**Web Mid (720p):**
```json
{
  "operation": "video",
  "profile": {
    "input": "input.mp4",
    "output": "output-720p.mp4",
    "profile": "web-mid"
  }
}
```

**Web High (1080p):**
```json
{
  "operation": "video",
  "profile": {
    "input": "input.mp4",
    "output": "output-1080p.mp4",
    "profile": "web-high"
  }
}
```

---

## Audio Operations

### Audio Transcode

**MP3 to AAC:**
```json
{
  "operation": "audio",
  "transcode": {
    "input": "input.mp3",
    "output": "output.aac",
    "codec": "aac",
    "bitrate": "192k"
  }
}
```

**To Opus:**
```json
{
  "operation": "audio",
  "transcode": {
    "input": "input.mp3",
    "output": "output.opus",
    "codec": "opus",
    "bitrate": "128k"
  }
}
```

### Audio Trim

```json
{
  "operation": "audio",
  "trim": {
    "input": "podcast.mp3",
    "output": "intro.mp3",
    "start": 0,
    "end": 60
  }
}
```

### Audio Normalize

**YouTube standard (-14 LUFS):**
```json
{
  "operation": "audio",
  "normalize": {
    "input": "podcast.mp3",
    "output": "normalized.mp3",
    "target_lufs": -14
  }
}
```

**Spotify standard (-16 LUFS):**
```json
{
  "operation": "audio",
  "normalize": {
    "input": "music.mp3",
    "output": "normalized.mp3",
    "target_lufs": -16
  }
}
```

**EBU R128 standard (-23 LUFS):**
```json
{
  "operation": "audio",
  "normalize": {
    "input": "interview.mp3",
    "output": "normalized.mp3",
    "target_lufs": -23
  }
}
```

### Audio Silence Trim

**Default threshold:**
```json
{
  "operation": "audio",
  "silence_trim": {
    "input": "recording.mp3",
    "output": "trimmed.mp3"
  }
}
```

**Custom threshold:**
```json
{
  "operation": "audio",
  "silence_trim": {
    "input": "recording.mp3",
    "output": "trimmed.mp3",
    "threshold_db": -50,
    "min_duration": 0.3
  }
}
```

---

## Batch Operations

**Multiple image operations:**
```json
{
  "operation": "batch",
  "jobs": [
    {
      "operation": "resize",
      "input": "hero.jpg",
      "output_dir": "dist",
      "widths": [320, 640, 1024]
    },
    {
      "operation": "optimize",
      "inputs": ["a.png", "b.png"],
      "output_dir": "dist"
    },
    {
      "operation": "watermark",
      "input": "photo.jpg",
      "output": "watermarked.jpg",
      "text": "© 2024"
    }
  ]
}
```

**Video processing batch:**
```json
{
  "operation": "batch",
  "jobs": [
    {
      "operation": "video",
      "profile": {
        "input": "lecture.mp4",
        "output": "480p.mp4",
        "profile": "web-low"
      }
    },
    {
      "operation": "video",
      "profile": {
        "input": "lecture.mp4",
        "output": "720p.mp4",
        "profile": "web-mid"
      }
    },
    {
      "operation": "video",
      "thumbnail": {
        "input": "lecture.mp4",
        "output": "thumb.jpg",
        "timestamp": "10%"
      }
    }
  ]
}
```
