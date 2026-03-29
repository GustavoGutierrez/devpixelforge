# DevPixelForge Documentation

Complete technical documentation for DevPixelForge (dpf).

---

## 📚 Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [JSON Protocol](#json-protocol)
- [Usage Modes](#usage-modes)
- [Project Structure](#project-structure)
- [Building](#building)
- [Testing](#testing)

---

## Overview

DevPixelForge is a **high-performance multimedia processing engine** written in Rust with Go bindings. It provides:

| Category | Operations |
|----------|------------|
| **Images** | resize, crop, rotate, watermark, adjust, optimize, convert, palette, favicon, sprite, placeholder, srcset, EXIF |
| **Video** | transcode, resize, trim, thumbnail, web profiles |
| **Audio** | transcode, trim, normalize (LUFS), silence trim |

### Key Features

| Feature | Description |
|---------|-------------|
| **Multi-format** | PNG, JPEG, WebP, GIF, SVG, ICO, AVIF, MP4, WebM, MP3, AAC, Opus |
| **High Performance** | Rust-powered with parallel processing via Rayon |
| **Streaming Mode** | Persistent process for low-latency operations |
| **FFI Bridge** | Native Go bindings for seamless integration |
| **Smart Operations** | Focal point cropping, auto-quality optimization, entropy-based selection |
| **Static Binary** | musl-compiled binary for portability |

---

## Architecture

```
┌─────────────────┐      JSON/stdio      ┌──────────────────┐
│   Go Bridge     │◄────────────────────►│   dpf (Rust)     │
│  (Client)       │   stdin/stdout       │  (Engine)       │
└─────────────────┘                      └──────────────────┘
```

The Go bridge communicates with the Rust engine via **JSON over stdin/stdout**.

---

## Quick Start

```bash
# Build
make build

# Verify capabilities
./dpf/target/release/dpf caps

# Example resize
./dpf/target/release/dpf process \
  --job '{"operation":"resize","input":"image.png","output_dir":"out","widths":[320,640]}'
```

---

## CLI Reference

| Command | Description |
|---------|-------------|
| `dpf caps` | Show supported capabilities |
| `dpf process --job <json>` | Execute single operation |
| `dpf batch --file <path>` | Process batch from JSON file |
| `dpf --stream` | Start streaming mode |

---

## Usage Modes

| Mode | Command | Use Case |
|------|---------|----------|
| One-shot | `dpf process --job '{...}'` | Single operation |
| Stdin | `echo '{...}' \| dpf` | Pipes and scripts |
| Streaming | `dpf --stream` | Multiple operations |
| Batch | `dpf batch --file jobs.json` | Parallel jobs |

---

## Project Structure

```
devpixelforge/
├── dpf/                    # Rust Engine
│   └── src/operations/
│       ├── image/          # 14 image operations
│       ├── video/          # 5 video operations
│       └── audio/          # 4 audio operations
├── go-bridge/             # Go FFI Bindings
├── docs/                  # Documentation
│   ├── schema.md          # JSON schema reference
│   ├── examples.md        # Working examples
│   └── testing/           # Testing docs
├── Makefile
└── README.md              # Main README
```

---

## Building

| Dependency | Version |
|------------|---------|
| Rust | ≥ 1.74 |
| Go | ≥ 1.21 |

```bash
make build        # Full build
make build-rust  # Rust only
make build-go    # Go only
```

---

## Testing

| Component | Tests |
|-----------|-------|
| Rust Operations | 280+ |
| Integration | 20+ |
| Go Bridge | 16+ |
| **Total** | **316+** |

```bash
make test
```

---

## Documentation Links

| Document | Description |
|----------|-------------|
| [📋 JSON Schema](schema.md) | Full JSON protocol reference |
| [💡 Examples](examples.md) | Working examples for all operations |
| [🧪 Testing](testing/README.md) | Testing architecture and guides |

---

## License

GNU General Public License v3.0 (GPL-3.0) - see [../LICENSE](../LICENSE) for details.
