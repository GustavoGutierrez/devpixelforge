# Guía de integración — DevForge Image Processor en un proyecto Go

Esta guía explica qué archivos debes copiar y cómo usar el motor de imágenes
Rust desde cualquier proyecto Go.

---

## 1. Qué necesitas llevarte

### Binario Rust (obligatorio)

```
rust-imgproc/target/release/devforge-imgproc
```

Este es el motor que hace el procesamiento real. Debe:
- Estar compilado para la plataforma destino (`make build-rust` en Linux/macOS).
- Ser accesible por el proceso Go en tiempo de ejecución.

> Para distribuir sin dependencias del sistema usa el binario estático:
> `make build-rust-static` → `rust-imgproc/target/x86_64-unknown-linux-musl/release/devforge-imgproc`

### Cliente Go (obligatorio)

```
go-bridge/imgproc.go
```

Un solo archivo `.go` sin dependencias externas (solo stdlib). Contiene:
- Todos los tipos de job (`ResizeJob`, `OptimizeJob`, `ConvertJob`, etc.)
- `Client` — cliente one-shot (un proceso por operación)
- `StreamClient` — cliente streaming (proceso Rust persistente, recomendado para servidores)
- Métodos de conveniencia: `Resize`, `Optimize`, `Convert`, `Favicon`, `Placeholder`

---

## 2. Cómo integrarlo en tu proyecto Go

### Paso 1 — Copiar los archivos

```bash
# En tu proyecto Go
cp /ruta/a/devforge-imgproc/rust-imgproc/target/release/devforge-imgproc ./bin/
cp /ruta/a/devforge-imgproc/go-bridge/imgproc.go ./internal/imgproc/imgproc.go
```

Estructura recomendada en tu proyecto:

```
mi-proyecto/
├── bin/
│   └── devforge-imgproc        # binario Rust
├── internal/
│   └── imgproc/
│       └── imgproc.go          # cliente Go (copiado de go-bridge/)
└── ...
```

### Paso 2 — Ajustar el package name

Abre `internal/imgproc/imgproc.go` y cambia la primera línea:

```go
// Antes:
package imgproc

// Después (si lo colocas en internal/imgproc/):
package imgproc   // sin cambios si respetas la ruta
```

### Paso 3 — Usar en tu código

#### Opción A: Cliente one-shot (simple, para pocas operaciones)

```go
import "mi-proyecto/internal/imgproc"

client := imgproc.NewClient("./bin/devforge-imgproc")
client.SetTimeout(60 * time.Second)

// Resize responsivo
result, err := client.Resize(ctx, "uploads/foto.jpg", "public/img", []uint32{320, 640, 1280})

// Optimizar + generar WebP
result, err = client.Optimize(ctx, []string{"public/img/foto.jpg"}, "public/img/opt")

// Convertir SVG a WebP
result, err = client.Convert(ctx, "assets/logo.svg", "public/img/logo.webp", "webp")

// Generar favicons desde SVG
result, err = client.Favicon(ctx, "assets/logo.svg", "public/favicons")

// LQIP para lazy loading
result, err = client.Placeholder(ctx, "uploads/hero.jpg")
if result.Metadata != nil {
    // result contiene "data_uri" con el base64 del placeholder
}
```

#### Opción B: StreamClient (recomendado para servidores MCP o alta carga)

El `StreamClient` arranca el proceso Rust **una sola vez** y reutiliza el
canal stdin/stdout para todas las operaciones. Elimina ~5ms de overhead por
operación.

```go
import "mi-proyecto/internal/imgproc"

// Inicializar una vez (p.ej. al arrancar el servidor)
sc, err := imgproc.NewStreamClient("./bin/devforge-imgproc")
if err != nil {
    log.Fatal(err)
}
defer sc.Close()

// Enviar trabajos concurrentemente (StreamClient es thread-safe)
result, err := sc.Execute(&imgproc.ResizeJob{
    Operation: "resize",
    Input:     "uploads/foto.jpg",
    OutputDir: "public/img",
    Widths:    []uint32{320, 640, 1280},
})

result, err = sc.Execute(&imgproc.ConvertJob{
    Operation: "convert",
    Input:     "assets/icon.svg",
    Output:    "public/img/icon.webp",
    Format:    "webp",
})
```

#### Opción C: Batch (múltiples operaciones en paralelo)

```go
result, err := sc.Execute(&imgproc.BatchJob{
    Operation: "batch",
    Jobs: []any{
        imgproc.ResizeJob{
            Operation: "resize",
            Input:     "uploads/hero.jpg",
            OutputDir: "public/img/hero",
            Widths:    []uint32{320, 640, 1280},
        },
        imgproc.FaviconJob{
            Operation:   "favicon",
            Input:       "assets/logo.svg",
            OutputDir:   "public/favicons",
            GenerateICO: true,
        },
    },
})
```

---

## 3. Integración en un MCP server Go

Patrón recomendado para un servidor MCP:

```go
type MCPServer struct {
    imgClient *imgproc.StreamClient
    // ... otros campos
}

func NewMCPServer(binaryPath string) (*MCPServer, error) {
    sc, err := imgproc.NewStreamClient(binaryPath)
    if err != nil {
        return nil, fmt.Errorf("failed to start image processor: %w", err)
    }
    return &MCPServer{imgClient: sc}, nil
}

func (s *MCPServer) Shutdown() {
    s.imgClient.Close()
}

// Handler para la tool "optimize_images"
func (s *MCPServer) handleOptimizeImages(ctx context.Context, params json.RawMessage) (any, error) {
    var req struct {
        Paths     []string `json:"paths"`
        OutputDir string   `json:"output_dir"`
    }
    if err := json.Unmarshal(params, &req); err != nil {
        return nil, err
    }

    result, err := s.imgClient.Execute(&imgproc.OptimizeJob{
        Operation: "optimize",
        Inputs:    req.Paths,
        OutputDir: &req.OutputDir,
        AlsoWebp:  true,
    })
    if err != nil {
        return nil, fmt.Errorf("image optimization failed: %w", err)
    }

    return result, nil
}

// Handler para la tool "generate_favicon"
func (s *MCPServer) handleGenerateFavicon(ctx context.Context, params json.RawMessage) (any, error) {
    var req struct {
        Input     string `json:"input"`
        OutputDir string `json:"output_dir"`
    }
    if err := json.Unmarshal(params, &req); err != nil {
        return nil, err
    }

    return s.imgClient.Execute(&imgproc.FaviconJob{
        Operation:        "favicon",
        Input:            req.Input,
        OutputDir:        req.OutputDir,
        GenerateICO:      true,
        GenerateManifest: true,
    })
}
```

---

## 4. Checklist de integración

- [ ] Binario `devforge-imgproc` copiado y con permisos de ejecución (`chmod +x`)
- [ ] `imgproc.go` copiado al package correcto de tu proyecto
- [ ] Ruta al binario configurada correctamente (absoluta o relativa al CWD del proceso)
- [ ] `StreamClient` inicializado al arrancar el servidor y cerrado al apagar (`defer sc.Close()`)
- [ ] Timeout adecuado para operaciones pesadas (`client.SetTimeout(120 * time.Second)`)

---

## 5. Resumen de tipos de job disponibles

| Tipo | Campo `operation` | Cuándo usarlo |
|------|------------------|---------------|
| `ResizeJob` | `"resize"` | Generar variantes responsivas |
| `OptimizeJob` | `"optimize"` | Comprimir PNG/JPEG + generar WebP |
| `ConvertJob` | `"convert"` | Cambiar formato (SVG→PNG, PNG→WebP, etc.) |
| `FaviconJob` | `"favicon"` | Generar pack de favicons desde SVG/PNG |
| `SpriteJob` | `"sprite"` | Crear sprite sheet + CSS |
| `PlaceholderJob` | `"placeholder"` | LQIP, color dominante, gradiente CSS |
| `BatchJob` | `"batch"` | Ejecutar múltiples operaciones en paralelo |
