# DevPixelForge (dpf)

## Presentación del Proyecto

---

## ¿Qué es DevPixelForge?

**DevPixelForge** es un motor de procesamiento multimedia de alto rendimiento desarrollado en **Rust** con integración nativa en **Go**. Su propósito principal es transformar, convertir y optimizar medios digitales (imágenes, videos, audio y documentos) mediante una CLI simple basada en JSON.

> *"Transforma pixeles a la velocidad de Rust."*

---

## ¿Para qué sirve?

DevPixelForge permite automatizar tareas de procesamiento de medios sin necesidad de instalar herramientas separadas. Algunas operaciones incluyen:

| Categoría | Operaciones |
|-----------|-------------|
| **Imágenes** | Redimensionar, recortar, rotar, marcar agua, ajustar brillo/contraste, optimizar calidad, convertir formatos (PNG, JPEG, WebP, AVIF, GIF), generar favicons, sprites, placeholders, srcset |
| **Video** | Transcodificar (H.264, H.265, VP8, VP9, AV1), redimensionar, recortar, extraer thumbnails, perfiles web |
| **Audio** | Transcodificar, recortar, normalizar LUFS, eliminar silencio |
| **Documentos** | Convertir Markdown a PDF conThemes profesionales |

---

## ¿Cómo funciona?

La arquitectura es simple y eficiente:

```
┌─────────────────┐      JSON/stdio      ┌──────────────────┐
│   Go Bridge     │◄────────────────────►│   dpf (Rust)     │
│  (Cliente)      │   stdin/stdout       │  (Motor)         │
└─────────────────┘                      └──────────────────┘
```

1. **Envías un job** — Definición de la operación en JSON
2. **El motor Rust procesa** — Operaciones paralelas con Rayon
3. **Recibes el resultado** — JSON con rutas, metadatos, o bytes inline

### Modos de uso

- **One-shot**: `dpf process --job '{...}'`
- **Stdin**: `echo '{...}' | dpf`
- **Streaming**: `dpf --stream` (proceso persistente de baja latencia)
- **Batch**: `dpf batch --file jobs.json` (múltiples jobs)

---

## ¿Cómo se usa?

### Instalación

```bash
git clone https://github.com/your-org/devpixelforge.git
cd devpixelforge
make build
```

### Ejemplos básicos

```bash
# Redimensionar imagen a múltiples anchos
./dpf/target/release/dpf process \
  --job '{"operation":"resize","input":"foto.jpg","output_dir":"out","widths":[320,640,1024]}'

# Convertir Markdown a PDF
./dpf/target/release/dpf process \
  --job '{"operation":"markdown_to_pdf","input":"README.md","output":"doc.pdf","theme":"engineering"}'

# Normalizar audio a LUFS -14
./dpf/target/release/dpf process \
  --job '{"operation":"normalize","input":"audio.mp3","output":"audio-normalizado.mp3","target_lufs":-14}'
```

### Integración con Go

```go
client := dpf.NewClient("./dpf")
result, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
    Input:  "docs/report.md",
    Output: "out/report.pdf",
    Theme:  strPtr("professional"),
})
```

---

## ¿Cómo te ayuda?

### ✅ Ventajas principales

1. **Alto rendimiento** — Rust + procesamiento paralelo (Rayon)
2. **Portabilidad** — Binario estático (musl) que corre en cualquier Linux
3. **Flexibilidad** — Múltiples formatos de entrada/salida
4. **Integración sencilla** — Protocolo JSON sobre CLI
5. **Themes profesionales** — Markdown a PDF con estilos predefinidos

### 📊 Comparativa rápida

| Herramienta | DevPixelForge | ImageMagick | FFmpeg |
|-------------|---------------|-------------|--------|
| Velocidad   | Muy alta      | Media       | Alta   |
| Binario único| Sí          | No          | No     |
| Markdown→PDF| Sí            | No          | No     |
| Go bindings | Sí            | No          | No     |

---

## Casos de uso

### 1. Generación automática de assets web

```bash
# Generar srcset responsive para una imagen
./dpf process --job '{"operation":"srcset","input":"hero.jpg","widths":[320,640,960,1280]}'
```

### 2. Pipeline de contenido editorial

```bash
# Convertir artículos Markdown a PDF con theme científico
./dpf process --job '{"operation":"markdown_to_pdf","input":"articulo.md","output":"articulo.pdf","theme":"scientific_article"}'
```

### 3. Normalización de audio para podcasts

```bash
# Normalizar a LUFS -14 (estándar YouTube)
./dpf process --job '{"operation":"normalize","input":"podcast.mp3","output":"podcast-normalizado.mp3","target_lufs":-14}'
```

### 4. Optimización de imágenes para e-commerce

```bash
# Redimensionar + optimizar para web
./dpf process --job '{"operation":"resize","input":"producto.jpg","output_dir":"out","widths":[400,800],"quality":85}'
```

### 5. Extracción de thumbnails de video

```bash
# Generar thumbnail al 25% del video
./dpf process --job '{"operation":"thumbnail","input":"video.mp4","output":"thumb.jpg","timestamp":"25%"}'
```

### 6. Integración en servidores Go

```go
// Servidor Go que genera PDFs al vuelo
func generatePDF(w http.ResponseWriter, r *http.Request) {
    client := dpf.NewClient("./bin/dpf")
    result, _ := client.MarkdownToPDF(r.Context(), &dpf.MarkdownToPDFJob{
        MarkdownText: strPtr("# Reporte\n\nContenido dinámico"),
        Inline:       true,
        Theme:        strPtr("professional"),
    })
    pdfBytes, _ := base64.StdEncoding.DecodeString(*result.Outputs[0].DataBase64)
    w.Header().Set("Content-Type", "application/pdf")
    w.Write(pdfBytes)
}
```

---

## Especificaciones técnicas

| Aspecto | Detalle |
|---------|---------|
| **Lenguaje** | Rust (motor) + Go (bridge) |
| **Versión actual** | 0.4.4 |
| **Plataformas** | Linux (x86_64 musl), macOS (ARM64, Intel) |
| **Video codecs** | H.264, H.265, VP8, VP9, AV1 |
| **Image formats** | PNG, JPEG, WebP, GIF, AVIF, BMP, TIFF, ICO, SVG |
| **Audio codecs** | MP3, AAC, Opus, FLAC, WAV |
| **Documentos** | Markdown → PDF (via Typst) |

---

## ¿Quién lo usa?

DevPixelForge es ideal para:

- **Desarrolladores** que necesitan automatización de medios en pipelines CI/CD
- **Equipos de contenido** que generan documentos PDF desde Markdown
- **Plataformas web** que procesan imágenes subidas por usuarios
- **Podcasters** que normalizan audio para múltiples plataformas
- **Servicios de video** que transcodifican contenido

---

## Próximos pasos

1. **Prueba local**: `make build && ./dpf/target/release/dpf caps`
2. **Explora operaciones**: Revisa `docs/examples.md`
3. **Integra en tu proyecto**: Usa el Go bridge o el protocolo JSON

---

*DevPixelForge — Transformando medios a la velocidad de Rust.*