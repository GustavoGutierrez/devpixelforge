# DevPixelForge integration guide

This document describes the current public integration surface for `dpf 0.4.2`,
with special focus on the `markdown_to_pdf` operation.

## Important scope note

`dpf` is integrated as a compiled binary that speaks JSON over CLI/stdin/stdout.
For external consumers, that JSON protocol is the stable integration surface.

`glyphweaveforge` is still the underlying Markdown-to-PDF renderer, but it is an
implementation detail of `dpf`, not the main integration entry point documented
here.

## Version validated

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

Validate that in your environment with:

```bash
./dpf/target/release/dpf caps

# Or the static binary
./dpf/target/x86_64-unknown-linux-musl/release/dpf caps
```

## Integration modes

`dpf` supports four integration modes:

1. `process --job '{...}'`
2. stdin one-shot: `echo '{...}' | dpf`
3. `--stream` for persistent low-latency workers
4. `batch --file jobs.json` for multiple jobs

## Build and runtime

Build from the repository root:

```bash
make build
```

Useful binaries:

- `./dpf/target/release/dpf`
- `./dpf/target/x86_64-unknown-linux-musl/release/dpf`

## Recommended usage

For Markdown-to-PDF integration, send JSON jobs to `dpf` instead of calling the
renderer crate directly.

The simplest file-to-file flow is:

```bash
./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"README.md","output":"README.pdf","theme":"scientific_article"}'
```

## Markdown-to-PDF API

`markdown_to_pdf` accepts exactly one source from `input`, `markdown_text`, or
`markdown_base64`.

It also requires at least one output mode from `output`, `output_dir`, or
`inline=true`.

### Request shape

```json
{
  "operation": "markdown_to_pdf",
  "input": "docs/report.md",
  "markdown_text": "# Report",
  "markdown_base64": "IyBSZXBvcnQ=",
  "output": "/tmp/report.pdf",
  "output_dir": "/tmp/reports",
  "file_name": "report.pdf",
  "inline": false,
  "page_size": "a4",
  "page_width_mm": null,
  "page_height_mm": null,
  "layout_mode": "paged",
  "theme": "engineering",
  "theme_config": {
    "margin_mm": 14.0
  },
  "resource_files": {
    "logo.png": "./assets/logo.png"
  }
}
```

### Fields

| Field | Required | Notes |
|-------|----------|-------|
| `operation` | Yes | Must be `markdown_to_pdf`. |
| `input` | No* | Markdown file path. Best option when the document references local assets. |
| `markdown_text` | No* | Inline UTF-8 Markdown source. |
| `markdown_base64` | No* | Base64-encoded UTF-8 Markdown source. |
| `output` | No** | Explicit output PDF path. |
| `output_dir` | No** | Directory output mode. For file input, the source stem is used by default. |
| `file_name` | No | Optional override when `output_dir` is used. Required for inline input with `output_dir`. |
| `inline` | No | When `true`, returns a base64 PDF in `outputs[*].data_base64`. |
| `page_size` | No | Supported presets: `a4`, `letter`, `legal`. Default is `a4`. |
| `page_width_mm` | No*** | Custom width in millimeters. |
| `page_height_mm` | No*** | Custom height in millimeters. |
| `layout_mode` | No | Supported values: `paged`, `single_page`. Default is `paged`. |
| `theme` | No | Built-in theme preset. See the theme list below. If omitted, renderer defaults are used. |
| `theme_config` | No | JSON overrides forwarded to GlyphWeaveForge `ThemeConfig`. |
| `resource_files` | No | Optional href-to-file mapping for inline Markdown assets. |

`*` Exactly one input source is required.

`**` At least one output mode is required.

`***` Custom page size requires both dimensions and both must be positive.

## Built-in themes

The current `dpf 0.4.2` build accepts these theme strings:

- `invoice`
- `scientific_article`
- `professional`
- `engineering`
- `informational`

Example using the scientific article theme:

```bash
./dpf/target/x86_64-unknown-linux-musl/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"integration-guide.md","output":"integration-guide-scientific-article.pdf","theme":"scientific_article"}'
```

## Common integration flows

### File input to file output

Use file input when the Markdown contains relative image paths.

```json
{
  "operation": "markdown_to_pdf",
  "input": "docs/report.md",
  "output": "out/report.pdf",
  "theme": "engineering",
  "page_size": "letter",
  "layout_mode": "paged"
}
```

### Inline Markdown to inline PDF

```json
{
  "operation": "markdown_to_pdf",
  "markdown_text": "# Inline report\n\nGenerated from memory.",
  "inline": true,
  "theme": "professional"
}
```

### Inline Markdown with injected assets

```json
{
  "operation": "markdown_to_pdf",
  "markdown_text": "# Inline Assets\n\n![Logo](logo.png)",
  "inline": true,
  "theme": "informational",
  "resource_files": {
    "logo.png": "./assets/logo.png"
  }
}
```

### Inline Markdown to output directory

When using `output_dir` with inline Markdown, `file_name` is required.

```json
{
  "operation": "markdown_to_pdf",
  "markdown_base64": "IyBSZXBvcnQKClJlbmRlcmVkIGZyb20gYmFzZTY0Lg==",
  "output_dir": "out/reports",
  "file_name": "report.pdf",
  "theme": "invoice"
}
```

### Custom page size

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

## Go bridge usage

For Go consumers, the repository ships a dedicated bridge module at:

`github.com/GustavoGutierrez/devpixelforge-bridge`

The current bridge API is built around:

- `dpf.NewClient(binaryPath)` for one-shot process execution
- `dpf.NewStreamClient(binaryPath)` for a persistent worker process
- `dpf.MarkdownToPDFJob` for the request payload
- `(*Client).MarkdownToPDF(...)` and `(*StreamClient).MarkdownToPDF(...)`

### Recommended one-shot usage

Use `Client` when calls are infrequent or stateless.

```go
package main

import (
	"context"
	"log"
	"time"

	dpf "github.com/GustavoGutierrez/devpixelforge-bridge"
)

func strPtr(s string) *string { return &s }

func main() {
	client := dpf.NewClient("./bin/dpf")
	client.SetTimeout(60 * time.Second)

	result, err := client.MarkdownToPDF(context.Background(), &dpf.MarkdownToPDFJob{
		Input:    "docs/report.md",
		Output:   "out/report.pdf",
		PageSize: strPtr("letter"),
		Theme:    strPtr("engineering"),
	})
	if err != nil {
		log.Fatal(err)
	}
	if !result.Success {
		log.Fatal("markdown_to_pdf returned success=false")
	}

	log.Printf("generated %s", result.Outputs[0].Path)
}
```

### Inline PDF from Go

Use `Inline: true` when the caller wants PDF bytes in memory.

```go
package main

import (
	"context"
	"encoding/base64"
	"log"
	"time"

	dpf "github.com/GustavoGutierrez/devpixelforge-bridge"
)

func strPtr(s string) *string { return &s }

func main() {
	client := dpf.NewClient("./bin/dpf")
	client.SetTimeout(60 * time.Second)

	markdown := "# Inline Report\n\nGenerated from Go memory."
	result, err := client.MarkdownToPDF(context.Background(), &dpf.MarkdownToPDFJob{
		MarkdownText: &markdown,
		Inline:       true,
		Theme:        strPtr("professional"),
	})
	if err != nil {
		log.Fatal(err)
	}
	if !result.Success {
		log.Fatal("markdown_to_pdf returned success=false")
	}

	pdfBytes, err := base64.StdEncoding.DecodeString(*result.Outputs[0].DataBase64)
	if err != nil {
		log.Fatal(err)
	}

	log.Printf("received %d PDF bytes", len(pdfBytes))
}
```

### Inline assets from Go

When Markdown is in memory and references local assets, pass `ResourceFiles`.

```go
markdown := "# Inline Assets\n\n![Logo](logo.png)"

result, err := client.MarkdownToPDF(context.Background(), &dpf.MarkdownToPDFJob{
	MarkdownText: &markdown,
	Inline:       true,
	Theme:        strPtr("informational"),
	ResourceFiles: map[string]string{
		"logo.png": "./assets/logo.png",
	},
})
```

### Stream client usage

Use `StreamClient` when your Go service sends many jobs and you want to avoid
process spawn overhead.

```go
sc, err := dpf.NewStreamClient("./bin/dpf")
if err != nil {
	log.Fatal(err)
}
defer sc.Close()

markdown := "# Streamed PDF\n\nGenerated through StreamClient."
result, err := sc.MarkdownToPDF(&dpf.MarkdownToPDFJob{
	MarkdownText: &markdown,
	Inline:       true,
	Theme:        strPtr("scientific_article"),
})
if err != nil {
	log.Fatal(err)
}
if !result.Success {
	log.Fatal("stream markdown_to_pdf returned success=false")
}
```

### Go bridge contract notes

- `MarkdownToPDFJob` mirrors the JSON API field names used by `dpf`.
- `Operation` is set automatically to `markdown_to_pdf` if left empty when you call
  `Client.MarkdownToPDF(...)` or `StreamClient.MarkdownToPDF(...)`.
- `Client.MarkdownToPDF(...)` requires a `context.Context`; `StreamClient.MarkdownToPDF(...)`
  does not because the process is already running.
- `Client.SetTimeout(...)` controls the one-shot command timeout.
- Structured validation failures can return `err == nil` with `result.Success == false`,
  so production code should check both values.
- The current Go bridge `JobResult` type does not expose the JSON `error` field from
  failed Rust responses, so callers only see `Success == false` unless the bridge is
  extended.
- Inline PDF bytes are returned in `result.Outputs[0].DataBase64`.
- Runtime metadata remains in `result.Metadata` and includes fields such as
  `backend`, `page_size`, `layout_mode`, `theme`, and `resource_resolver`.

### When to use each Go client

- Use `Client` for CLI-style execution, occasional jobs, or simpler call sites.
- Use `StreamClient` for MCP servers, worker pools, or high-throughput backends.
- Prefer file input from Go when Markdown references relative assets already on disk.
- Prefer `ResourceFiles` when the Markdown source is assembled in memory.

## Response shape

Successful responses use this structure:

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

### Metadata notes

- `metadata.backend` is always `typst` for `markdown_to_pdf`.
- `metadata.resource_resolver` is `filesystem`, `custom`, or `none`.
- File outputs always report `format="pdf"`, `width=0`, and `height=0`.
- Inline outputs return the PDF bytes in `outputs[*].data_base64`.

## Streaming and batch usage

### Stream mode

Start the worker:

```bash
./dpf/target/release/dpf --stream
```

Then send one JSON job per line:

```json
{"operation":"markdown_to_pdf","markdown_text":"# Streamed PDF\n\nGenerated through stream mode.","inline":true}
```

### Batch mode

Create a JSON array file:

```json
[
  {
    "operation": "markdown_to_pdf",
    "input": "docs/report.md",
    "output": "out/report.pdf",
    "theme": "engineering"
  }
]
```

Run it with:

```bash
./dpf/target/release/dpf batch --file jobs.json
```

## Supported behavior validated in the current build

The current repository validation covers:

- UTF-8 text rendering
- headings
- ordered and unordered lists
- block quotes
- fenced code blocks
- inline code
- Markdown links
- standard Markdown images
- basic HTML `<img>` extraction
- standard Markdown tables
- all built-in themes rendering non-blank PDFs
- file output, inline output, batch mode, and stream mode

## Current limitations and notes

- Raw HTML is not a general layout system. `dpf` only sanitizes a narrow subset of
  wrapper tags and converts basic HTML `<img>` tags into Markdown-compatible image
  content before rendering.
- Footnotes, Mermaid fences, and math fences should still be treated as limited or
  fallback content unless you validate your exact documents against the current build.
- When your input comes from memory, local assets are not auto-discovered unless you
  provide `resource_files`.

## Best practices

- Prefer `input` over inline Markdown when the document references local files.
- Use `resource_files` for inline Markdown with images or other local assets.
- Use `theme` for stable built-in styling and `theme_config` only when you need
  explicit overrides.
- Use `caps` at startup if your integration must assert supported features or the
  binary version.
- Keep generated PDFs in separate output paths when comparing themes.

## Validation commands used in this repository

```bash
./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"README.md","output":"docs/validation/markdown-to-pdf/readme.pdf","theme":"engineering"}'

./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"AGENTS.md","output":"docs/validation/markdown-to-pdf/agents.pdf","theme":"engineering"}'

for theme in invoice scientific_article professional engineering informational; do
  ./dpf/target/release/dpf process \
    --job "{\"operation\":\"markdown_to_pdf\",\"input\":\"dpf/test_fixtures/sample.md\",\"output\":\"docs/validation/markdown-to-pdf/themes/${theme//_/-}.pdf\",\"theme\":\"${theme}\"}"
done
```

## Summary

If you are integrating the current project version, target `dpf 0.4.2` and its
JSON-based `markdown_to_pdf` API.

Use `theme` values from the current built-in list, prefer file input for
asset-heavy Markdown, and rely on `caps` plus repository validation artifacts to
confirm runtime support in your environment.
