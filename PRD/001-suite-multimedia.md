# PRD 001: Suite de Procesamiento Multimedia para Desarrolladores

**Versión:** 1.0  
**Estado:** Draft  
**Fecha:** 2026-03-28  
**Proyecto:** devpixelforge

---

## 1. Resumen Ejecutivo

Extender devpixelforge desde procesamiento de imágenes básico a una suite completa tipo Squoosh para desarrolladores web/backend y agentes de IA. Incluye: imágenes (core), video (esencial) y audio (mínimo útil), expuesto vía CLI y MCP con operaciones composables.

---

## 2. Alcance

### 2.1 In-Scope
- Módulo Imágenes: transformaciones, optimización web, análisis, utilidades dev
- Módulo Video: transcodificación, resize, trim, thumbnails, perfiles predefinidos
- Módulo Audio: transcodificación, trim, normalización, silence trimming
- Integración CLI unificada con JSON declarativo
- Hooks para agentes de IA (metadata, recomendaciones)

### 2.2 Out-of-Scope
- Edición avanzada tipo Photoshop
- Streaming HLS/DASH complejo
- Transcripción de audio (hook externo solo)
- Background removal nativo (hook a modelo externo)

---

## 3. Módulo Imágenes (Core)

### 3.1 Transformaciones Básicas
| Operación | Detalle |
|-----------|---------|
| Resize | Por ancho/alto/porcentaje. Modos: fit, fill, limit, scale |
| Crop | Manual (coordenadas) o inteligente (center, thumb, smart). Soporte focal point |
| Rotate/Flip | 90/180/270, flip H/V, auto-orientación EXIF |
| Formatos | JPEG, PNG, WebP, AVIF, SVG passthrough |
| Compresión | Calidad 0-100, lossy/lossless, esfuerzo compresión |

### 3.2 Optimización Web
- **Auto-format**: Selección automática WebP/AVIF/JPEG según cliente
- **Auto-quality**: Trade-off peso/calidad automático
- **Variantes responsive**: Generar srcset listo para `<img>`
- **Metadata**: Strip EXIF/GPS/thumbnails, opcional preservar campos

### 3.3 Edición y Análisis
| Feature | Descripción |
|---------|-------------|
| Ajustes | Brillo, contraste, saturación, nitidez, blur |
| Paleta | Extracción colores dominantes para theming |
| Color space | sRGB ↔ otros, corrección gamma |
| Watermark | Logo/texto/badges (posicionamiento configurable) |
| LQIP | Low-quality placeholder / blurhash |

### 3.4 Utilidades Dev
- **Size suggester**: Dado layout objetivo, sugerir breakpoints ideales (JSON)
- **Asset validator**: Check peso máx, dimensiones, formato permitido (CI/CD)
- **Sprite atlas**: Combinar iconos en sprite sheet + mapa posiciones

---

## 4. Módulo Video (Esencial)

### 4.1 Operaciones Core
| Operación | Especificación |
|-----------|----------------|
| Transcode | H.264/MP4 (base), WebM VP8/VP9 (opcional), AV1 (futuro) |
| Resize | 1080p, 720p, 480p manteniendo aspect ratio |
| Trim | Recorte por timestamps start/end |
| Concat | Unir clips secuencia con manejo audio |

### 4.2 Helpers Web
- **Thumbnails**: Extraer frame en timestamp (default 25-30%) → poster
- **Audio extract**: Pista MP3/AAC/OGG desde video
- **Perfiles predefinidos**: `web-low`, `web-mid`, `web-high` (resolución + bitrate)

---

## 5. Módulo Audio (Mínimo Útil)

### 5.1 Operaciones Básicas
| Operación | Formatos/Detalles |
|-----------|-------------------|
| Transcode | MP3, AAC, OGG/Opus, WAV. Bitrate: 64/96/128/192 kbps |
| Trim | Recorte por tiempo start/end |
| Concat | Unir múltiples pistas |

### 5.2 Mejora
- **Normalización**: Peak o loudness target (podcasts/clips)
- **Silence trim**: Auto-remover silencios inicio/fin

---

## 6. Interfaz y Agentes

### 6.1 CLI Unificado
```bash
# One-shot
dpf process --job '{"module":"image","operation":"resize",...}'

# Batch
dpf batch --file jobs.json

# Streaming
dpf --stream  # JSON línea por línea

# Video
dpf video transcode --input vid.mov --profile web-mid

# Audio  
dpf audio normalize --input podcast.mp3 --loudness -16
```

### 6.2 Pipeline Declarativo (JSON)
```json
{
  "pipeline": [
    {"op": "resize", "width": 800, "mode": "fit"},
    {"op": "convert", "format": "webp", "quality": 85},
    {"op": "strip_metadata"}
  ]
}
```

### 6.3 Análisis para Agentes
Endpoint/metadata que exponga:
- Dimensiones, formato, peso, colores dominantes
- Duración (video/audio), bitrate actual
- Histograma simple
- Recomendación optimización automática

---

## 7. Arquitectura Técnica

### 7.1 Stack
- **Imágenes**: Rust (image, imageproc, mozjpeg, ravif)
- **Video**: FFmpeg bindings (rust-ffmpeg) o llamadas a ffmpeg CLI
- **Audio**: FFmpeg / symphonia (Rust)
- **Go bridge**: FFI para integración con ecosistema Go

### 7.2 Estructura
```
dpf/
├── src/
│   ├── image/      # Módulo imágenes
│   ├── video/      # Módulo video
│   ├── audio/      # Módulo audio
│   ├── pipeline/   # Orquestación JSON
│   └── cli/        # CLI unificado
├── go-bridge/      # FFI Go bindings
└── examples/       # Ejemplos de uso
```

### 7.3 Performance
- Binario release: LTO + codegen-units=1
- Procesamiento paralelo donde aplique
- Streaming para archivos grandes

---

## 8. Criterios de Aceptación

### 8.1 Imágenes (MVP)
- [ ] Resize fit/fill/limit funcionando
- [ ] Crop manual e inteligente
- [ ] JPEG/PNG/WebP/AVIF output
- [ ] Auto-format y auto-quality
- [ ] Generación srcset responsive
- [ ] Extracción colores dominantes
- [ ] Watermark básico

### 8.2 Video (MVP)
- [ ] Transcode a H.264/MP4
- [ ] Resize manteniendo aspect
- [ ] Trim por timestamps
- [ ] Extract thumbnail en %
- [ ] Perfiles web-low/mid/high

### 8.3 Audio (MVP)
- [ ] Transcode MP3/AAC/OGG/WAV
- [ ] Trim por tiempo
- [ ] Normalización loudness
- [ ] Silence trimming

### 8.4 CLI/Integración
- [ ] Comandos unificados dpf image/video/audio
- [ ] Soporte batch JSON
- [ ] Pipeline declarativo funcional
- [ ] Metadata/análisis para agentes

---

## 9. Roadmap

| Fase | Entregable | Prioridad |
|------|------------|-----------|
| 1 | Imágenes core + CLI base | P0 |
| 2 | Optimización web + análisis | P0 |
| 3 | Video transcode + resize | P1 |
| 4 | Audio básico | P1 |
| 5 | Pipeline declarativo + agentes | P2 |
| 6 | Video audio extract + trim | P2 |
| 7 | Utilidades dev avanzadas | P3 |

---

## 10. Referencias

- Cloudinary Image API patterns
- Bytescale/Filestack transformation APIs
- Squoosh.app functionality
- FFmpeg best practices for web

---

**Autor:** devpixelforge team  
**Reviewers:** TBD  
**Aprobación:** TBD
