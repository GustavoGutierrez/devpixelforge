# Documentación de DevPixelForge

Documentación técnica del proyecto DevPixelForge.

---

## 📁 Directorios

| Directorio | Descripción |
|------------|-------------|
| [`testing/`](testing/) | Documentación completa de la suite de tests |
| [`examples/`](examples/) | Ejemplos JSON para cada operación |

---

## 📚 Testing

La documentación de testing está organizada en:

1. **[Arquitectura de Tests](testing/01-architecture.md)** - Estructura y diseño de la suite de tests
2. **[Guía de Tests Rust](testing/02-rust-tests.md)** - Tests unitarios e integración en Rust
3. **[Guía de Tests Go](testing/03-go-tests.md)** - Tests del bridge de Go
4. **[Ejecución y CI/CD](testing/04-running-and-ci.md)** - Cómo ejecutar tests y configurar CI

### Resumen Rápido

```bash
# Tests Rust
cd dpf && cargo test          # Todos los tests
cargo test --lib              # Solo unitarios
cargo test --test integration_tests  # Solo integración

# Tests Go
cd go-bridge && go test -v
```

**Total: 316+ tests** (280 Rust unit + 20 Rust integration + 16 Go)

---

## 🎬 Video Processing

Operaciones de video implementadas:

| Operación | Descripción | Archivo |
|-----------|-------------|---------|
| `transcode` | Transcodificar a H.264, VP8/9, AV1 | `dpf/src/operations/video/transcode.rs` |
| `resize` | Redimensionar manteniendo aspect ratio | `dpf/src/operations/video/resize.rs` |
| `trim` | Recortar por timestamps (HH:MM:SS) | `dpf/src/operations/video/trim.rs` |
| `thumbnail` | Extraer frames en timestamps | `dpf/src/operations/video/thumbnail.rs` |
| `profile` | Aplicar presets web (low/mid/high) | `dpf/src/operations/video/profiles.rs` |

**Códecs**: H.264, VP8, VP9, AV1

**Formatos**: mp4, webm, mkv, avi, mov

### Perfiles Web

| Perfil | Resolución | Bitrate |
|--------|------------|---------|
| web-low | 480p | 1M |
| web-mid | 720p | 2.5M |
| web-high | 1080p | 5M |

---

## 🎵 Audio Processing

Operaciones de audio implementadas:

| Operación | Descripción | Archivo |
|-----------|-------------|---------|
| `transcode` | Transcodificar a AAC, MP3, Opus | `dpf/src/operations/audio/transcode.rs` |
| `trim` | Recortar por timestamps | `dpf/src/operations/audio/trim.rs` |
| `normalize` | Normalizar loudness (LUFS) | `dpf/src/operations/audio/normalize.rs` |
| `silence_trim` | Eliminar silencio inicio/fin | `dpf/src/operations/audio/silence_trim.rs` |

**Códecs**: MP3, AAC, Opus, Vorbis, FLAC, WAV

**Formatos**: mp3, aac, ogg, wav, flac, opus

---

## 🔗 Enlaces Útiles

- [README principal](../README.md)
- [AGENTS.md](../AGENTS.md) - Configuración de agentes
- [INTEGRATION.md](../INTEGRATION.md) - Protocolo de integración

---

## 📝 Convenciones

- Documentación en español
- Código y ejemplos comentados
- Ejemplos de comandos listos para copiar
