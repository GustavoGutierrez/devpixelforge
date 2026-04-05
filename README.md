<p align="center">
  <img src="devpixelforge.png" width="300" alt="DevPixelForge">
</p>

<div align="center">

![Rust](https://img.shields.io/badge/Rust-1.74+-dea584?style=flat-square&logo=rust&logoColor=white)
![Go](https://img.shields.io/badge/Go-1.21+-00ADD8?style=flat-square&logo=go&logoColor=white)
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-blue?style=flat-square)

</div>

# DevPixelForge (dpf)

High-performance multimedia processing engine in Rust with Go client for seamless integration.

**"Transform pixels at the speed of Rust."**

---

## What is DevPixelForge?

DevPixelForge (dpf) is a **high-performance multimedia processing engine** that provides:

| Category | Operations |
|----------|------------|
| **Images** | Resize, crop, rotate, watermark, adjust, optimize, convert, palette, favicon, sprite, placeholder, srcset, EXIF |
| **Documents** | Markdown to PDF (GlyphWeaveForge + Typst) |
| **Video** | Transcode, resize, trim, thumbnail, web profiles |
| **Audio** | Transcode, trim, normalize (LUFS), silence removal |

### Key Features

- 🚀 **High Performance** — Rust-powered with parallel processing (Rayon)
- 🔄 **Multiple Formats** — PNG, JPEG, WebP, GIF, SVG, ICO, AVIF, MP4, WebM, MP3, AAC, Opus
- 📡 **Streaming Mode** — Persistent process for low-latency operations
- 🔗 **Go Integration** — Native FFI bindings via StreamClient
- 📄 **Markdown to PDF** — Typst-backed document rendering for file and inline flows
- 🎯 **Smart Operations** — Focal point cropping, auto-quality, entropy-based selection
- 📦 **Static Binary** — musl-compiled for portability

### Markdown-to-PDF Highlights

- Uses GlyphWeaveForge via dpf `0.4.2` with the Typst backend selected explicitly.
- Supports file input, inline text, or base64-encoded Markdown.
- Supports file output, directory output, and inline base64 PDF responses.
- Supports `resource_files` for in-memory Markdown inputs that reference local assets.
- Supports built-in themes: `invoice`, `scientific_article`, `professional`, `engineering`, `informational`.
- Available themes are validated against the current build. See [`integration-guide.md`](integration-guide.md) for complete API details.

---

## How It Works

```
┌─────────────────┐      JSON/stdio      ┌──────────────────┐
│   Go Bridge     │◄────────────────────►│   dpf (Rust)     │
│  (Client)       │   stdin/stdout       │  (Engine)       │
└─────────────────┘                      └──────────────────┘
```

1. **Send Job** — JSON job definition via stdin
2. **Process** — Rust engine handles the operation
3. **Receive Result** — JSON response via stdout

---

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/your-org/devpixelforge.git
cd devpixelforge
make build

# Verify capabilities (reports version, operations, features)
./dpf/target/release/dpf caps

# Or use the static musl binary
./dpf/target/x86_64-unknown-linux-musl/release/dpf caps
```

### Basic Usage

```bash
# Single operation (one-shot)
./dpf/target/release/dpf process \
  --job '{"operation":"resize","input":"image.png","output_dir":"out","widths":[320,640]}'

# Batch processing
./dpf/target/release/dpf batch --file jobs.json

# Markdown to PDF
./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","markdown_text":"# Hello\n\nPDF","inline":true}'

# Streaming mode (persistent process)
./dpf/target/release/dpf --stream
```

---

## Usage Modes

| Mode | Command | Use Case |
|------|---------|----------|
| One-shot | `dpf process --job '{...}'` | Single operation |
| Stdin | `echo '{...}' \| dpf` | Pipes and scripts |
| Streaming | `dpf --stream` | Multiple operations, low latency |
| Batch | `dpf batch --file jobs.json` | Parallel job processing |

### Streaming Mode Example

```bash
# Start streaming process
./dpf/target/release/dpf --stream

# Send multiple jobs (one per line)
{"operation":"resize","input":"a.png","output_dir":"out","widths":[320]}
{"operation":"optimize","inputs":["b.png"],"output_dir":"out"}
{"operation":"watermark","input":"c.png","output":"out/c.png","text":"© 2024"}
```

---

## Operations Reference

### Image Operations

| Operation | Description | Example |
|-----------|-------------|---------|
| `resize` | Resize to multiple widths | `{"widths":[320,640,1024]}` |
| `crop` | Manual or smart crop | `{"gravity":"focal_point","focal_x":0.75}` |
| `rotate` | Rotate/flip | `{"angle":90}` |
| `watermark` | Text/image overlay | `{"text":"© 2024","position":"bottom-right"}` |
| `adjust` | Brightness, contrast, blur | `{"brightness":0.2,"blur":2.0}` |
| `quality` | Auto-quality optimization | `{"target_size":50000}` |
| `srcset` | Responsive images + HTML | `{"widths":[320,640],"generate_html":true}` |
| `exif` | Strip/extract EXIF | `{"exif_op":"strip","mode":"all"}` |
| `optimize` | Lossless/lossy compression | `{"level":"lossless"}` |
| `convert` | Format conversion | `{"format":"webp"}` |
| `palette` | Color reduction | `{"max_colors":32,"dithering":0.5}` |
| `favicon` | Multi-size favicons | `{"sizes":[16,32,180]}` |
| `sprite` | Sprite sheets | `{"inputs":["a.png","b.png"],"columns":2}` |
| `placeholder` | LQIP, dominant color | `{"kind":"lqip","width":20}` |
| `markdown_to_pdf` | Render Markdown as PDF | `{"markdown_text":"# Report","inline":true}` |

### Document Operation

`markdown_to_pdf` accepts exactly one source from `input`, `markdown_text`, or `markdown_base64` and at least one output mode from `output`, `output_dir`, or `inline`.

- File input + `output` / `output_dir` keeps filesystem-relative asset resolution.
- Inline input + `inline=true` returns base64 PDF bytes in `outputs[0].data_base64`.
- Inline input + `resource_files` injects href-to-file asset mappings through the resource resolver.
- Inline input + `output_dir` requires `file_name`.
- Supported themes: `invoice`, `scientific_article`, `professional`, `engineering`, `informational`.

Example:

```json
{
  "operation": "markdown_to_pdf",
  "markdown_text": "# Report\n\nRendered by Typst.",
  "inline": true,
  "theme": "professional",
  "resource_files": {
    "logo.png": "./assets/logo.png"
  }
}
```

---

## Go Bridge Quick Start

The Go bridge module is `github.com/GustavoGutierrez/devpixelforge-bridge`.

### One-shot client

```go
client := dpf.NewClient("./dpf")
client.SetTimeout(60 * time.Second)

result, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
    Input:    "docs/report.md",
    Output:   "out/report.pdf",
    PageSize: strPtr("letter"),
    Theme:    strPtr("professional"),
})

if !result.Success {
    log.Fatal("markdown_to_pdf returned success=false")
}
log.Printf("generated %s", result.Outputs[0].Path)
```

**Contract notes:**
- `Client.MarkdownToPDF(...)` requires a `context.Context`.
- `Client.SetTimeout(...)` controls the one-shot command timeout.
- Structured validation failures can return `err == nil` with `result.Success == false`, so production code should check both values.

### Stream client (persistent process)

```go
sc, err := dpf.NewStreamClient("./dpf")
defer sc.Close()

result, err := sc.MarkdownToPDF(&dpf.MarkdownToPDFJob{
    MarkdownText: &markdown,
    Inline:       true,
    Theme:        strPtr("engineering"),
})

if !result.Success {
    log.Fatal("stream markdown_to_pdf returned success=false")
}
```

**StreamClient notes:**
- Does not require a `context.Context` because the process is already running.
- Ideal for MCP servers, worker pools, or high-throughput backends.

### Inline PDF from Go

```go
markdown := "# Inline Report\n\nGenerated from Go memory."
result, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
    MarkdownText: &markdown,
    Inline:       true,
    Theme:        strPtr("professional"),
})

pdfBytes, err := base64.StdEncoding.DecodeString(*result.Outputs[0].DataBase64)
```

### Custom Page Size

```json
{
  "operation": "markdown_to_pdf",
  "input": "docs/report.md",
  "output": "out/report-custom.pdf",
  "page_width_mm": 210.0,
  "page_height_mm": 297.0,
  "layout_mode": "single_page",
  "theme": "scientific_article"
}
```

Custom page size requires both dimensions and both must be positive.

See [`integration-guide.md`](integration-guide.md) for complete Go bridge documentation.

---

### CLI Validation Artifacts

The repository includes a reproducible CLI validation guide and generated sample PDFs for this feature:

- Guide: [`docs/validation/markdown-to-pdf/README.md`](docs/validation/markdown-to-pdf/README.md)
- Generated PDFs:
  - [`docs/validation/markdown-to-pdf/readme.pdf`](docs/validation/markdown-to-pdf/readme.pdf)
  - [`docs/validation/markdown-to-pdf/agents.pdf`](docs/validation/markdown-to-pdf/agents.pdf)
  - [`docs/validation/markdown-to-pdf/themes/`](docs/validation/markdown-to-pdf/themes/)

### Video Operations

| Operation | Description | Example |
|-----------|-------------|---------|
| `transcode` | Codec conversion | `{"codec":"h264"}` |
| `resize` | Scale video | `{"height":720}` |
| `trim` | Cut by timestamps | `{"start":30,"end":90}` |
| `thumbnail` | Extract frame | `{"timestamp":"25%"}` |
| `profile` | Web-optimized | `{"profile":"web-mid"}` |

### Audio Operations

| Operation | Description | Example |
|-----------|-------------|---------|
| `transcode` | Format conversion | `{"codec":"aac","bitrate":"192k"}` |
| `trim` | Cut by timestamps | `{"start":0,"end":60}` |
| `normalize` | LUFS normalization | `{"target_lufs":-14}` |
| `silence_trim` | Remove silence | `{"threshold_db":-40}` |

---

## Project Structure

```
devpixelforge/
├── dpf/                           # Rust Engine
│   └── src/
│       ├── lib.rs                 # Main entry
│       ├── cli.rs                 # CLI arguments
│       ├── processor.rs            # Job processing
│       └── operations/
│           ├── image/              # 14 image operations
│           ├── video/              # 5 video operations
│           └── audio/              # 4 audio operations
├── go-bridge/                     # Go FFI Bindings
│   └── pkg/dpf/
│       ├── client.go              # ProcessClient
│       └── stream.go              # StreamClient
├── docs/                          # Documentation
│   ├── README.md                  # This file (index)
│   ├── schema.md                  # JSON schema reference
│   ├── examples.md                # Working examples
│   └── testing/                   # Testing docs
└── Makefile
```

---

## Building

### Requirements

| Dependency | Version | Purpose |
|------------|---------|---------|
| Rust | ≥ 1.74 | Core engine |
| Go | ≥ 1.21 | FFI bridge |
| musl-tools | (Debian/Ubuntu) | Static binary |

### Build Commands

```bash
# Full build (Rust + Go)
make build

# Rust only
make build-rust

# Go only
make build-go

# Static binary (musl)
make build-rust-static
```

---

## Version and Capabilities

The current binary reports:

```json
{
  "version": "0.4.2",
  "operations": ["markdown_to_pdf"],
  "output_formats": {
    "document": ["pdf"]
  },
  "features": {
    "markdown_to_pdf": true,
    "markdown_to_pdf_typst": true,
    "pdf_inline_output": true,
    "streaming_mode": true,
    "parallel_batch": true
  }
}
```

Validate in your environment:

```bash
./dpf/target/release/dpf caps

# Or the static binary
./dpf/target/x86_64-unknown-linux-musl/release/dpf caps
```

---

## Validation Commands

Reproducible validation commands used in this repository:

```bash
# Validate README to PDF
./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"README.md","output":"docs/validation/markdown-to-pdf/readme.pdf","theme":"engineering"}'

# Validate AGENTS.md to PDF
./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"AGENTS.md","output":"docs/validation/markdown-to-pdf/agents.pdf","theme":"engineering"}'

# Validate all themes
for theme in invoice scientific_article professional engineering informational; do
  ./dpf/target/release/dpf process \
    --job "{\"operation\":\"markdown_to_pdf\",\"input\":\"dpf/test_fixtures/sample.md\",\"output\":\"docs/validation/markdown-to-pdf/themes/${theme//_/-}.pdf\",\"theme\":\"${theme}\"}"
done
```

---

## Supported Behavior

Current repository validation covers:

- UTF-8 text rendering
- Headings
- Ordered and unordered lists
- Block quotes
- Fenced code blocks
- Inline code
- Markdown links
- Standard Markdown images
- Basic HTML `<img>` extraction
- Standard Markdown tables
- All built-in themes rendering non-blank PDFs
- File output, inline output, batch mode, and stream mode

---

## Limitations and Notes

- Raw HTML is not a general layout system. `dpf` only sanitizes a narrow subset of wrapper tags and converts basic HTML `<img>` tags into Markdown-compatible image content before rendering.
- Footnotes, Mermaid fences, and math fences should be treated as limited or fallback content unless you validate your exact documents against the current build.
- When your input comes from memory, local assets are not auto-discovered unless you provide `resource_files`.

---

## Best Practices

- Prefer `input` over inline Markdown when the document references local files.
- Use `resource_files` for inline Markdown with images or other local assets.
- Use `theme` for stable built-in styling and `theme_config` only when you need explicit overrides.
- Use `caps` at startup if your integration must assert supported features or the binary version.
- Keep generated PDFs in separate output paths when comparing themes.

---

## Response Structure

Successful responses include:

```json
{
  "success": true,
  "operation": "markdown_to_pdf",
  "outputs": [
    {
      "path": "out/report.pdf",
      "format": "pdf",
      "width": 0,
      "height": 0,
      "size_bytes": 122623,
      "data_base64": null
    }
  ],
  "elapsed_ms": 169,
  "metadata": {
    "backend": "typst",
    "page_size": "a4",
    "layout_mode": "paged",
    "theme": "engineering",
    "inline": false,
    "has_file_output": true,
    "resource_resolver": "filesystem",
    "resource_files": 0
  }
}
```

Metadata notes:
- `metadata.backend` is always `typst` for `markdown_to_pdf`.
- `metadata.resource_resolver` is `filesystem`, `custom`, or `none`.
- File outputs always report `format="pdf"`, `width=0`, and `height=0`.
- Inline outputs return the PDF bytes in `outputs[*].data_base64`.

---

## Testing

```bash
# All tests
make test

# Rust tests
cd dpf && cargo test

# Go tests
cd go-bridge && go test -v
```

| Component | Tests |
|-----------|-------|
| Rust Operations | 280+ |
| Integration | 20+ |
| Go Bridge | 16+ |
| **Total** | **316+** |

---

## Documentation

| Document | Description |
|----------|-------------|
| [📖 Main Docs](docs/README.md) | Complete project documentation |
| [📋 JSON Schema](docs/schema.md) | Full JSON protocol reference |
| [💡 Examples](docs/examples.md) | Working examples for all operations |
| [🧪 Testing](docs/testing/README.md) | Testing architecture and guides |
| [✅ Markdown-to-PDF Validation](docs/validation/markdown-to-pdf/README.md) | Reproduction steps and committed validation artifacts |
| [🔗 Integration Guide](integration-guide.md) | Complete API reference for dpf 0.4.2 and Go bridge usage |

---

## License

GNU General Public License v3.0 (GPL-3.0)

Copyright (c) 2024 Ing. Gustavo Gutiérrez

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
