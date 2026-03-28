# DevPixelForge JSON API Schema

Complete reference for the JSON protocol used by dpf for image processing operations.

## Table of Contents

1. [JobResult](#jobresult) - Response structure
2. [ImageJob](#imagejob) - Union of all operations
3. [ResizeParams](#resizeparams)
4. [CropParams](#cropparams)
5. [RotateParams](#rotateparams)
6. [WatermarkParams](#watermarkparams)
7. [AdjustParams](#adjustparams)
8. [QualityParams](#qualityparams)
9. [SrcsetParams](#srcsetparams)
10. [ExifParams](#exifparams)
11. [OptimizeParams](#optimizeparams)
12. [ConvertParams](#convertparams)
13. [PaletteParams](#paletteparams)
14. [FaviconParams](#faviconparams)
15. [SpriteParams](#spriteparams)
16. [PlaceholderParams](#placeholderparams)
17. [BatchParams](#batchparams)
18. [JobError](#joberror)

---

## JobResult

Response structure returned by all operations.

```rust
pub struct JobResult {
    pub success: bool,
    pub operation: String,
    pub outputs: Vec<OutputFile>,
    pub elapsed_ms: u64,
    pub metadata: Option<serde_json::Value>,
}
```

### OutputFile

```rust
pub struct OutputFile {
    pub path: String,          // Output file path
    pub format: String,        // Image format (png, jpeg, webp, avif)
    pub width: u32,            // Output width in pixels
    pub height: u32,           // Output height in pixels
    pub size_bytes: u64,       // File size in bytes
    pub data_base64: Option<String>,  // Base64-encoded image (if inline=true)
}
```

---

## ImageJob

Top-level job object. The `operation` field determines which params struct is used.

```json
{
  "operation": "resize|crop|rotate|watermark|adjust|quality|srcset|exif|optimize|convert|palette|favicon|sprite|placeholder|batch",
  ...
}
```

---

## ResizeParams

Resize images to multiple widths or by percentage.

```json
{
  "operation": "resize",
  "input": "string",           // Required: Source image path
  "output_dir": "string",      // Required: Output directory
  "widths": [320, 640, 1024], // Array of target widths (or use scale_percent)
  "scale_percent": 50,         // Alternative: Scale by percentage
  "max_height": 800,           // Optional: Max height constraint
  "format": "png",             // Optional: Output format (png, jpeg, webp, avif)
  "quality": 85,               // Optional: JPEG/WebP quality (1-100, default 85)
  "filter": "lanczos3",        // Optional: Resize filter (default, lanczos3, catmullrom)
  "linear_rgb": false,         // Optional: Use linear RGB space (default false)
  "inline": false              // Optional: Return base64-encoded output
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output_dir` | String | Yes | - | Output directory |
| `widths` | `Vec<u32>` | No* | - | Target widths in pixels |
| `scale_percent` | `f32` | No* | - | Scale percentage (50 = half size) |
| `max_height` | `u32` | No | - | Maximum height constraint |
| `format` | String | No | same as input | Output format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |
| `filter` | String | No | lanczos3 | Resize filter |
| `linear_rgb` | bool | No | false | Linear RGB processing |
| `inline` | bool | No | false | Return base64 output |

*Either `widths` or `scale_percent` is required.

---

## CropParams

Crop images with manual rectangle or smart crop modes.

```json
{
  "operation": "crop",
  "input": "string",
  "output": "string",
  "rect": { "x": 0, "y": 0, "width": 100, "height": 100 },
  "gravity": "center",
  "focal_x": 0.5,
  "focal_y": 0.5,
  "width": 800,
  "height": 600,
  "format": "png",
  "quality": 85,
  "inline": false
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output` | String | Yes | - | Output file path |
| `rect` | `CropRect` | No* | - | Manual crop rectangle |
| `gravity` | String | No* | - | Smart crop mode |
| `focal_x` | `f32` | No | 0.5 | Focal point X (0.0-1.0) |
| `focal_y` | `f32` | No | 0.5 | Focal point Y (0.0-1.0) |
| `width` | `u32` | No** | - | Target width for smart crop |
| `height` | `u32` | No** | - | Target height for smart crop |
| `format` | String | No | same as input | Output format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |
| `inline` | bool | No | false | Return base64 output |

*Either `rect` or `gravity` is required.
**Width and height required when using `gravity`.

### Gravity Modes

- `center` - Crop from center of image
- `focal_point` - Crop around focal point (use `focal_x`, `focal_y`)
- `entropy` - Crop region with highest brightness variance

### CropRect

```rust
pub struct CropRect {
    pub x: u32,       // Left edge
    pub y: u32,       // Top edge
    pub width: u32,    // Width in pixels
    pub height: u32,   // Height in pixels
}
```

---

## RotateParams

Rotate images by fixed or arbitrary angles, with optional flip.

```json
{
  "operation": "rotate",
  "input": "string",
  "output": "string",
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

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output` | String | Yes | - | Output file path |
| `angle` | `u16` | No* | - | Fixed angle: 0, 90, 180, 270 |
| `angle_f` | `f32` | No* | - | Arbitrary angle (-360 to 360) |
| `flip` | String | No | - | "horizontal" or "vertical" |
| `auto_orient` | bool | No | false | Auto-orient from EXIF |
| `background` | String | No | transparent | Background color (#RRGGBB) |
| `format` | String | No | same as input | Output format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |
| `inline` | bool | No | false | Return base64 output |

*Either `angle` or `angle_f` is required.

---

## WatermarkParams

Add text or image watermarks with positioning and opacity control.

```json
{
  "operation": "watermark",
  "input": "string",
  "output": "string",
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

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output` | String | Yes | - | Output file path |
| `text` | String | No* | - | Text watermark content |
| `image` | String | No* | - | Image watermark path |
| `position` | String | No | bottom-right | Position on image |
| `opacity` | `f32` | No | 1.0 | Opacity (0.0-1.0) |
| `font_size` | `u32` | No | 24 | Font size for text (pixels) |
| `color` | String | No | #FFFFFF | Text color (hex) |
| `offset_x` | `u32` | No | 10 | Horizontal offset from edge |
| `offset_y` | `u32` | No | 10 | Vertical offset from edge |
| `format` | String | No | same as input | Output format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |
| `inline` | bool | No | false | Return base64 output |

*Either `text` or `image` is required.

### Position Values

3x3 grid positions:
- `top-left`, `top-center`, `top-right`
- `center-left`, `center`, `center-right`
- `bottom-left`, `bottom-center`, `bottom-right`

Aliases: `topleft`, `top`, `center`, `middle`, `bottom`, `left`, `right`, etc.

---

## AdjustParams

Adjust image properties: brightness, contrast, saturation, blur, sharpen.

```json
{
  "operation": "adjust",
  "input": "string",
  "output": "string",
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

### Fields

| Field | Type | Required | Default | Range | Description |
|-------|------|----------|---------|-------|-------------|
| `input` | String | Yes | - | - | Source image path |
| `output` | String | Yes | - | - | Output file path |
| `brightness` | `f32` | No | 0.0 | -1.0 to 1.0 | Brightness adjustment |
| `contrast` | `f32` | No | 0.0 | -1.0 to 1.0 | Contrast adjustment |
| `saturation` | `f32` | No | 0.0 | -1.0 to 1.0 | Saturation (-1 = grayscale) |
| `blur` | `f32` | No | - | 0.0 to 50.0 | Gaussian blur sigma |
| `sharpen` | `f32` | No | - | 0.0 to 10.0 | Sharpen amount |
| `linear_rgb` | bool | No | true | - | Use linear RGB for adjustments |
| `format` | String | No | same | - | Output format |
| `quality` | `u8` | No | 85 | - | JPEG/WebP quality |
| `inline` | bool | No | false | - | Return base64 output |

All adjustment values can be combined. Values of 0 or null mean no change.

---

## QualityParams

Auto-quality optimization using binary search to achieve target file size.

```json
{
  "operation": "quality",
  "input": "string",
  "output": "string",
  "target_size": 50000,
  "tolerance_percent": 5.0,
  "max_iterations": 10,
  "min_quality": 30,
  "max_quality": 95,
  "format": "jpeg",
  "inline": false
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output` | String | Yes | - | Output file path |
| `target_size` | `u64` | Yes | - | Target file size in bytes |
| `tolerance_percent` | `f32` | No | 5.0 | Acceptable deviation (%) |
| `max_iterations` | `u8` | No | 10 | Max binary search iterations |
| `min_quality` | `u8` | No | 30 | Minimum quality to try |
| `max_quality` | `u8` | No | 95 | Maximum quality to try |
| `format` | String | Yes | - | Output format (jpeg, webp, avif) |
| `inline` | bool | No | false | Return base64 output |

### Response Metadata

```json
{
  "target_size": 50000,
  "final_quality": 72,
  "final_size": 48500,
  "deviation_percent": 3.0,
  "iterations": 6,
  "converged": true
}
```

---

## SrcsetParams

Generate responsive image variants with srcset and optional HTML output.

```json
{
  "operation": "srcset",
  "input": "string",
  "output_dir": "string",
  "widths": [320, 640, 960, 1280, 1920],
  "densities": [1.0, 2.0],
  "format": "webp",
  "quality": 85,
  "generate_html": true,
  "linear_rgb": true
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output_dir` | String | Yes | - | Output directory |
| `widths` | `Vec<u32>` | Yes | - | Target widths in pixels |
| `densities` | `Vec<f32>` | No | [1.0, 2.0] | Density multipliers |
| `format` | String | No | same | Output format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |
| `generate_html` | bool | No | false | Generate HTML img tag |
| `linear_rgb` | bool | No | false | Use linear RGB for resize |

### Output Files

Generated filenames follow pattern:
- `{name}-{width}w.{ext}` (1x density)
- `{name}-{width}w-{density}x.{ext}` (other densities)

Example: `hero-640w.webp`, `hero-640w-2x.webp`

### HTML Output (when `generate_html: true`)

```html
<img src="hero-1920w.webp" srcset="hero-320w.webp 320w, hero-640w.webp 640w, ..." sizes="(max-width: 1920px) 100vw, 1920px" alt="">
```

---

## ExifParams

EXIF metadata operations: strip, preserve, extract, auto-orient.

```json
{
  "operation": "exif",
  "input": "string",
  "output": "string",
  "exif_op": "strip",
  "mode": "all",
  "keep": null,
  "return_metadata": true,
  "format": "jpeg",
  "quality": 85,
  "inline": false
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output` | String | No* | - | Output file path |
| `exif_op` | String | Yes | - | Operation type |
| `mode` | String | No | all | Strip mode |
| `keep` | `Vec<String>` | No | [] | Tags to keep (preserve) |
| `return_metadata` | bool | No | true | Include metadata in response |
| `format` | String | No | same | Output format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |
| `inline` | bool | No | false | Return base64 output |

*Required for `strip`, `preserve`, and `auto_orient` operations.

### Operation Types

| Operation | Description |
|-----------|-------------|
| `strip` | Remove EXIF data |
| `preserve` | Remove all EXIF except specified tags |
| `extract` | Read and return EXIF metadata (no output required) |
| `auto_orient` | Apply EXIF orientation transformation |

### Strip Modes

| Mode | Description |
|------|-------------|
| `all` | Remove all EXIF data |
| `gps` | Remove GPS information only |
| `thumbnail` | Remove embedded thumbnail |
| `camera` | Remove camera make/model info |

### Extracted Metadata Fields

```json
{
  "has_exif": true,
  "make": "Canon",
  "model": "EOS 5D",
  "orientation": 1,
  "datetime": "2024:01:15 10:30:00",
  "exposure_time": "1/250",
  "f_number": "f/2.8",
  "iso": 400,
  "gps": {
    "latitude": 40.7128,
    "longitude": -74.0060,
    "altitude": 10.5
  }
}
```

### Orientation Values

| Value | Transformation |
|-------|----------------|
| 1 | Normal |
| 2 | Flipped horizontally |
| 3 | Rotated 180° |
| 4 | Flipped vertically |
| 5 | Rotated 90° CW, flipped |
| 6 | Rotated 90° CW |
| 7 | Rotated 90° CCW, flipped |
| 8 | Rotated 90° CCW |

---

## OptimizeParams

Lossless and lossy compression optimization.

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

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `inputs` | `Vec<String>` | Yes | - | Input file paths |
| `output_dir` | String | Yes | - | Output directory |
| `level` | String | No | lossless | Optimization level |
| `quality` | `u8` | No | 80 | Lossy quality (if level is lossy) |
| `also_webp` | bool | No | false | Also generate WebP versions |

### Optimization Levels

| Level | Description |
|-------|-------------|
| `lossless` | Maximum quality, moderate compression |
| `lossy` | Lower quality, smaller files |

---

## ConvertParams

Convert images between formats.

```json
{
  "operation": "convert",
  "input": "image.png",
  "output": "image.webp",
  "format": "webp",
  "quality": 85
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output` | String | Yes | - | Output file path |
| `format` | String | Yes | - | Target format |
| `quality` | `u8` | No | 85 | JPEG/WebP quality |

### Supported Formats

- PNG: `png`
- JPEG: `jpeg`, `jpg`
- WebP: `webp`
- AVIF: `avif`
- ICO: `ico`
- GIF: `gif`

---

## PaletteParams

Reduce image color palette with optional dithering.

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

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `output_dir` | String | Yes | - | Output directory |
| `max_colors` | `u32` | No | 256 | Maximum colors in palette |
| `dithering` | `f32` | No | 0.0 | Dithering amount (0.0-1.0) |
| `format` | String | No | same | Output format |

---

## FaviconParams

Generate multi-size favicon from SVG or raster image.

```json
{
  "operation": "favicon",
  "input": "logo.svg",
  "output_dir": "out",
  "sizes": [16, 32, 48, 180, 192, 512]
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path (SVG or raster) |
| `output_dir` | String | Yes | - | Output directory |
| `sizes` | `Vec<u32>` | No | [16,32,48,180,192,512] | Target sizes |

### Output Files

Generates:
- `favicon.ico` - Multi-size ICO
- `favicon-{size}.png` - Individual sizes

---

## SpriteParams

Generate sprite sheet from multiple images.

```json
{
  "operation": "sprite",
  "inputs": ["icon1.png", "icon2.png", "icon3.png"],
  "output": "sprites.png",
  "columns": 4,
  "padding": 2
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `inputs` | `Vec<String>` | Yes | - | Input image paths |
| `output` | String | Yes | - | Output sprite path |
| `columns` | `u32` | No | 4 | Columns in sprite grid |
| `padding` | `u32` | No | 0 | Padding between sprites |

---

## PlaceholderParams

Generate low-quality image placeholders (LQIP) or extract dominant color.

```json
{
  "operation": "placeholder",
  "input": "photo.jpg",
  "kind": "lqip",
  "width": 20,
  "format": "webp",
  "quality": 30
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `input` | String | Yes | - | Source image path |
| `kind` | String | No | lqip | Placeholder type |
| `width` | `u32` | No | 20 | LQIP width (for lqip) |
| `format` | String | No | webp | Output format |
| `quality` | `u8` | No | 30 | Quality setting |

### Placeholder Types

| Kind | Description |
|------|-------------|
| `lqip` | Low-quality blurred placeholder |
| `dominant` | Solid color placeholder |

### Response for Dominant Color

```json
{
  "metadata": {
    "dominant_color": "#3498db"
  }
}
```

---

## BatchParams

Execute multiple jobs in parallel.

```json
{
  "operation": "batch",
  "jobs": [
    { "operation": "resize", "input": "a.png", ... },
    { "operation": "optimize", "inputs": ["b.jpg"], ... }
  ],
  "parallel": true
}
```

### Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `jobs` | `Vec<Job>` | Yes | - | Array of job objects |
| `parallel` | bool | No | true | Execute in parallel |

Jobs array contains any valid ImageJob objects.

---

## JobError

Error response when an operation fails.

```rust
pub struct JobError {
    pub success: bool,       // Always false
    pub operation: String,    // Failed operation name
    pub error: String,        // Error message
}
```

### Example

```json
{
  "success": false,
  "operation": "resize",
  "error": "Input file not found: missing.png"
}
```

---

## Common Error Codes

| Error | Cause |
|-------|-------|
| `Input file not found` | Source image doesn't exist |
| `Invalid image format` | Unsupported or corrupted image |
| `width is required` | Missing required width parameter |
| `Must specify either 'rect' or 'gravity'` | Crop mode unspecified |
| `Invalid rotation angle` | Angle not in {0, 90, 180, 270} |
| `Opacity must be between 0.0 and 1.0` | Invalid opacity value |
| `Output path required` | Missing output path for operation |
| `At least one width must be specified` | Empty widths array |

---

## Inline Output

When `inline: true` is set, the output image is base64-encoded in the response:

```json
{
  "success": true,
  "operation": "resize",
  "outputs": [{
    "path": "out/320-image.png",
    "format": "png",
    "width": 320,
    "height": 240,
    "size_bytes": 12345,
    "data_base64": "iVBORw0KGgoAAAANSUhEUgAAAAE..."
  }]
}
```

---

## CLI Usage Examples

```bash
# Single operation
dpf process --job '{"operation":"resize","input":"img.png","output_dir":"out","widths":[320,640]}'

# Batch file
dpf batch --file jobs.json

# Streaming mode
dpf --stream

# With stdin
echo '{"operation":"resize","input":"img.png","output_dir":"out","widths":[320]}' | dpf
```
