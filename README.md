# DevForge Image Processor — Motor Rust para Go MCP Server

Componente de procesamiento de imágenes de alto rendimiento en Rust,
diseñado para ser invocado desde el MCP server de Go (DevForge MCP).

## Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│                    DevForge MCP (Go)                        │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │ analyze  │  │ suggest  │  │ manage   │  │ generate  │  │
│  │ _layout  │  │ _layout  │  │ _tokens  │  │ _ui_image │  │
│  └──────────┘  └──────────┘  └──────────┘  └─────┬─────┘  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐        │        │
│  │ optimize │  │ generate │  │ suggest  │        │        │
│  │ _images  │  │ _favicon │  │ _colors  │        │        │
│  └────┬─────┘  └────┬─────┘  └──────────┘        │        │
│       │              │                             │        │
│       └──────────────┼─────────────────────────────┘        │
│                      │                                      │
│              ┌───────▼───────┐                              │
│              │  Go Bridge    │  imgproc.Client               │
│              │  (JSON/stdio) │  imgproc.StreamClient          │
│              └───────┬───────┘                              │
└──────────────────────┼──────────────────────────────────────┘
                       │ JSON sobre stdin/stdout
                       │
┌──────────────────────▼──────────────────────────────────────┐
│            devforge-imgproc (Rust Binary)                   │
│                                                             │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌─────────────┐  │
│  │ resize  │  │ optimize │  │ convert │  │ favicon     │  │
│  │ (rayon) │  │ (oxipng  │  │ (webp,  │  │ (ico,       │  │
│  │         │  │  mozjpeg) │  │  resvg) │  │  multi-size)│  │
│  └─────────┘  └──────────┘  └─────────┘  └─────────────┘  │
│  ┌─────────┐  ┌──────────┐  ┌──────────────────────────┐  │
│  │ sprite  │  │ place-   │  │ pipeline (rayon batch)   │  │
│  │ sheet   │  │ holder   │  │ parallel job execution   │  │
│  └─────────┘  └──────────┘  └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Protocolo de comunicación Go ↔ Rust

El binario Rust acepta trabajos como JSON. Cada trabajo tiene un campo
`operation` que determina la operación:

```json
{"operation": "resize", "input": "hero.png", "output_dir": "out", "widths": [320, 640, 1024]}
```

Responde con:

```json
{
  "success": true,
  "operation": "resize",
  "outputs": [
    {"path": "out/hero_320w.png", "format": "png", "width": 320, "height": 180, "size_bytes": 24000}
  ],
  "elapsed_ms": 45,
  "metadata": {"source_width": 1920, "source_height": 1080, "variants_generated": 3}
}
```

### Modos de invocación

| Modo | Comando | Uso ideal |
|------|---------|-----------|
| **One-shot** | `devforge-imgproc process --job '{...}'` | Operaciones individuales |
| **Stdin** | `echo '{...}' \| devforge-imgproc` | Scripts y pipes |
| **Streaming** | `devforge-imgproc --stream` | MCP server (proceso persistente) |
| **Batch file** | `devforge-imgproc batch --file jobs.json` | CI/CD, bulk processing |

### Modo streaming (recomendado para MCP)

El modo `--stream` mantiene el proceso Rust vivo. Go envía un JSON por línea
a stdin y lee la respuesta (un JSON por línea) de stdout:

```
Go stdin  →  {"operation":"resize",...}\n
Rust stdout ← {"success":true,"outputs":[...]}\n
Go stdin  →  {"operation":"favicon",...}\n
Rust stdout ← {"success":true,"outputs":[...]}\n
```

Esto elimina el overhead de ~5ms de crear un proceso nuevo por cada operación.

## Operaciones soportadas

| Operación | Descripción | Paralelismo |
|-----------|-------------|-------------|
| `resize` | Responsive images, múltiples anchos | ✅ rayon (por variante) |
| `optimize` | Compresión lossless/lossy (oxipng, mozjpeg) | ✅ rayon (por archivo) |
| `convert` | Cambio de formato (svg→png, png→webp, etc.) | Single |
| `favicon` | Multi-size + .ico + manifest.json | ✅ rayon (por tamaño) |
| `sprite` | Sprite sheet + CSS coordinates | Single (I/O bound) |
| `placeholder` | LQIP, dominant color, CSS gradient | Single |
| `batch` | Múltiples operaciones en paralelo | ✅ rayon (por job) |

## Requisitos del sistema

### Compilación estándar (`make build-rust`, `make build`)

- **Rust toolchain** ≥ 1.74 via [rustup](https://rustup.rs/)
- **Go** ≥ 1.21 (solo para `make build` / `make build-go`)

### Compilación estática musl (`make build-rust-static`)

Genera un binario Linux completamente estático (sin dependencias de glibc).
Requiere dos dependencias adicionales:

**1. Target Rust para musl:**
```bash
rustup target add x86_64-unknown-linux-musl
```

**2. Linker musl (`musl-gcc`) — Ubuntu/Debian:**
```bash
sudo apt-get install musl-tools
```

> En otras distribuciones: `sudo dnf install musl-gcc` (Fedora/RHEL) o
> `sudo pacman -S musl` (Arch).

Sin estas dos dependencias el compilador falla con `E0463: can't find crate for 'core'`.

## Build

```bash
# Compilar el motor Rust (release optimizado)
make build-rust

# Compilar todo (Rust + Go example)
make build

# Ver capacidades
make test

# Binario estático (Linux, para distribuir sin dependencias)
make build-rust-static
```

## Ejemplo completo: `devforge-imgproc-example`

El binario `devforge-imgproc-example` (generado con `make build`) demuestra
todas las operaciones principales usando los archivos de `assets/` como entrada
y genera resultados en `output/`.

```bash
./devforge-imgproc-example
```

### Qué demuestra

#### One-Shot Mode — operaciones individuales

| Operación | Entrada | Salida | Descripción |
|-----------|---------|--------|-------------|
| **Responsive Resize** | `assets/hero.png` | `output/hero/hero_{320,640,1024,1440,1920}w.png` | 5 variantes responsivas en paralelo (~95ms) |
| **Batch Optimize + WebP** | `assets/photo{1,2}.jpg`, `assets/photo3.png` | `output/optimized/` | Optimización lossless + variante WebP por cada imagen (~880ms) |
| **Favicon Generation** | `assets/logo.svg` | `output/favicons/` | 8 tamaños PNG (16–512px) + `favicon.ico` + `manifest.json` (~20ms) |
| **LQIP Placeholder** | `assets/hero.png` | *(inline base64)* | Miniatura 20×10px en base64 para lazy loading (~25ms) |

#### Streaming Mode — proceso persistente (recomendado para MCP)

Demuestra el cliente `StreamClient` que mantiene el proceso Rust vivo y
procesa múltiples trabajos sin overhead de spawn:

| Job | Operación | Tiempo IPC |
|-----|-----------|-----------|
| 0 | `resize` card.png → 3 variantes | ~11ms |
| 1 | `convert` icon.svg → icon.webp | ~15ms |
| 2 | `placeholder` css_gradient | ~69ms |
| 3 | `placeholder` dominant_color | ~39ms |

> **Nota:** el overhead de IPC (Go ↔ Rust por stdin/stdout) es de ~2ms por
> operación, frente a los ~5ms de crear un proceso nuevo cada vez.

### Salida esperada

```
=== One-Shot Mode ===

Responsive Resize (took 95ms):
  Success: true
  Outputs: 5 files
    - output/hero/hero_320w.png  (png, 320x174,   129114 bytes)
    - output/hero/hero_640w.png  (png, 640x349,   452715 bytes)
    - output/hero/hero_1024w.png (png, 1024x558, 1088238 bytes)
    - output/hero/hero_1440w.png (png, 1440x785, 2070397 bytes)
    - output/hero/hero_1920w.png (png, 1920x1047,3500538 bytes)

Batch Optimize + WebP (took 882ms):
  Success: true
  Outputs: 6 files
    - output/optimized/photo1.jpg  (jpg,  2816x1536, 670144 bytes)
    - output/optimized/photo1.webp (webp, 2816x1536, 587750 bytes)
    ...

Favicon Generation (took 20ms):
  Success: true
  Outputs: 9 files
    - output/favicons/favicon-16x16.png  ...
    - output/favicons/favicon.ico        (48x48, 3709 bytes)

LQIP Placeholder (took 25ms):
  Metadata: { "data_uri": "data:image/png;base64,...", "lqip_width": 20, ... }

=== Streaming Mode ===
Job 0 (resize):      9ms  (IPC: 11ms)
Job 1 (convert):    14ms  (IPC: 15ms)
Job 2 (placeholder): 68ms (IPC: 69ms)
Job 3 (placeholder): 38ms (IPC: 39ms)
```

---

## Ejemplos de uso

### Convertir PNG a WebP

**Stdin (pipe):**
```bash
echo '{"operation":"convert","input":"sample.png","output":"sample.webp","format":"webp","quality":85}' \
  | ./rust-imgproc/target/release/devforge-imgproc
```

**One-shot (argumento):**
```bash
./rust-imgproc/target/release/devforge-imgproc process \
  --job '{"operation":"convert","input":"sample.png","output":"sample.webp","format":"webp","quality":85}'
```

**Modo streaming (proceso persistente):**
```bash
# Iniciar el proceso una vez y enviar múltiples trabajos
./rust-imgproc/target/release/devforge-imgproc --stream <<EOF
{"operation":"convert","input":"sample.png","output":"sample.webp","format":"webp","quality":85}
{"operation":"convert","input":"banner.png","output":"banner.webp","format":"webp","quality":90}
EOF
```

**Batch file (múltiples conversiones en paralelo):**
```json
// jobs.json
[
  {"operation":"convert","input":"sample.png","output":"sample.webp","format":"webp","quality":85},
  {"operation":"convert","input":"hero.png","output":"hero.webp","format":"webp","quality":90}
]
```
```bash
./rust-imgproc/target/release/devforge-imgproc batch --file jobs.json
```

> **Parámetros `convert`:** `input` (ruta origen), `output` (ruta destino), `format` (`png`, `jpeg`, `webp`, `ico`), `quality` (1–100, solo para jpeg/webp).

---

## Estructura del proyecto

```
devforge-imgproc/
├── Makefile
├── rust-imgproc/              # Motor de imágenes Rust
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # CLI + JSON protocol + streaming mode
│       ├── pipeline.rs        # Job execution + rayon parallel batch
│       └── operations/
│           ├── mod.rs
│           ├── utils.rs       # Load image, SVG rasterize, fit dimensions
│           ├── resize.rs      # Responsive resize (parallel per width)
│           ├── optimize.rs    # PNG (oxipng) + JPEG (mozjpeg) optimization
│           ├── convert.rs     # Format conversion inc. SVG→raster
│           ├── favicon.rs     # Multi-size favicons + ICO + manifest
│           ├── sprite.rs      # Sprite sheet + CSS generation
│           ├── placeholder.rs # LQIP, dominant color, CSS gradient
│           └── batch.rs       # Batch job definition
│
└── go-bridge/                 # Cliente Go para integrar en MCP server
    ├── go.mod
    ├── imgproc.go             # Client (one-shot) + StreamClient (persistent)
    └── example/
        └── main.go            # Ejemplos de uso desde Go
```
