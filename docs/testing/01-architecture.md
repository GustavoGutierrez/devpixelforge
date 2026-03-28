# Arquitectura de la Suite de Tests

## 1. Visión General

La suite de tests de **devpixelforge** tiene como propósito garantizar la correcta funcionalidad del motor de procesamiento de imágenes en Rust y su bridge FFI en Go. Los tests cubren tres niveles de abstracción:

| Nivel | Propósito | Cobertura |
|-------|-----------|-----------|
| **Unitarios** | Validar funciones individuales | Módulos internos de Rust |
| **Integración** | Validar CLI y protocolo JSON | Comandos `process`, `batch`, `caps` |
| **Bridge** | Validar integración Go ↔ Rust | Cliente Go y comunicación FFI |

### Objetivos Principales

1. **Correctitud**: Verificar que cada operación (resize, convert, optimize, favicon, etc.) produce resultados esperados
2. **Protocolo**: Asegurar que el JSON de entrada/salida cumple el contrato establecido
3. **Robustez**: Manejar casos de error (archivos corruptos, parámetros inválidos, timeouts)
4. **FFI**: Validar que el bridge Go puede comunicarse correctamente con el binario Rust

---

## 2. Estructura de Directorios

```
devpixelforge/
├── dpf/                              # Rust core
│   ├── src/
│   │   ├── operations/               # Módulos de operaciones
│   │   │   ├── resize.rs
│   │   │   ├── convert.rs
│   │   │   ├── optimize.rs
│   │   │   └── ...
│   │   ├── pipeline.rs               # Pipeline principal
│   │   └── main.rs                   # Entry point CLI
│   ├── tests/                        # ← Tests de integración Rust
│   │   └── integration_tests.rs
│   └── test_fixtures/                # ← Datos de prueba
│       ├── sample.png
│       ├── sample.jpg
│       ├── sample.svg
│       ├── sample_transparent.png
│       ├── solid_red.png
│       ├── solid_blue.png
│       ├── large.png
│       └── corrupt/
│           └── bad.png
├── go-bridge/                        # Go FFI bridge
│   ├── dpf.go                        # Implementación del cliente
│   ├── dpf_test.go                   # ← Tests del bridge Go
│   └── example/
│       └── main.go                   # Ejemplo de uso
└── docs/testing/
    └── 01-architecture.md            # ← Este documento
```

### Convenciones de Ubicación

| Tipo de Test | Ubicación | Naming |
|--------------|-----------|--------|
| Unitarios Rust | `src/**/*.rs` (inline `#[cfg(test)]`) | `fn test_*` |
| Integración Rust | `tests/*.rs` | `tests/integration_*.rs` |
| Bridge Go | `go-bridge/*_test.go` | `func Test*` |

---

## 3. Tipos de Tests

### 3.1 Tests Unitarios (Rust)

**Ubicación**: Módulos inline con `#[cfg(test)]` en `src/`

**Propósito**: Validar funciones individuales sin dependencias externas.

**Ejemplo**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dimensions() {
        let result = calculate_dimensions(100, 200, 50, None);
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 100);
    }
}
```

**Características**:
- No requieren binario compilado
- Ejecutan en memoria
- Ideal para lógica pura (matemáticas de resize, parsing, etc.)

**Comando**:
```bash
cd dpf && cargo test --lib
```

---

### 3.2 Tests de Integración (Rust)

**Ubicación**: `dpf/tests/integration_tests.rs`

**Propósito**: Validar el CLI como caja negra, probando el protocolo JSON completo.

**Cobertura**:

| Categoría | Tests |
|-----------|-------|
| **CLI Básico** | `test_cli_caps_command`, `test_cli_process_resize`, `test_cli_process_convert` |
| **Error Handling** | `test_cli_process_invalid_job`, `test_cli_process_missing_file` |
| **Batch** | `test_cli_batch_command` |
| **Streaming** | `test_stream_mode_single_job` |
| **Operaciones** | `test_cli_favicon_generation`, `test_cli_svg_conversion` |
| **Serialización** | `test_json_serialization_resize`, `test_json_deserialization_result` |

**Características**:
- Compilan el binario automáticamente si no existe
- Usan `std::process::Command` para ejecutar el CLI
- Usan `tempfile::TempDir` para aislamiento
- Usan fixtures del directorio `test_fixtures/`

**Comando**:
```bash
cd dpf && cargo test --test integration_tests
```

---

### 3.3 Tests de Bridge (Go)

**Ubicación**: `go-bridge/dpf_test.go`

**Propósito**: Validar que el cliente Go puede comunicarse correctamente con el binario Rust vía FFI (stdin/stdout).

**Cobertura**:

| Categoría | Tests |
|-----------|-------|
| **Cliente** | `TestNewClient`, `TestClientSetTimeout` |
| **Ejecución** | `TestExecuteResize`, `TestExecuteConvert`, `TestExecuteInvalidJob`, `TestExecuteMissingFile`, `TestExecuteTimeout` |
| **Métodos Conveniencia** | `TestResize`, `TestConvert`, `TestResizePercent`, `TestFavicon`, `TestOptimize` |
| **Streaming** | `TestNewStreamClient`, `TestStreamClientExecute`, `TestStreamClientMultipleJobs` |
| **Serialización** | `TestResizeJobJSON`, `TestJobResultUnmarshal` |

**Características**:
- Requieren binario Rust pre-compilado (hacen skip si no existe)
- Usan `t.TempDir()` para aislamiento
- Validan tanto one-shot como modo streaming
- Testean timeouts y manejo de errores

**Comando**:
```bash
cd go-bridge && go test -v
```

---

## 4. Fixtures y Datos de Prueba

### ¿Qué son los Fixtures?

Los fixtures son archivos de imagen estáticos usados como entrada para los tests. Permiten reproducir escenarios consistentes sin generar imágenes dinámicamente.

### Inventario de Fixtures

| Archivo | Formato | Tamaño | Propósito |
|---------|---------|--------|-----------|
| `sample.png` | PNG | 100x100 | Caso base, formato estándar |
| `sample.jpg` | JPEG | 100x100 | Testing conversión JPEG |
| `sample.svg` | SVG | Vector | Testing renderizado vectorial |
| `sample_transparent.png` | PNG | 100x100 | Canal alpha, testing transparencia |
| `solid_red.png` | PNG | 100x100 | Color sólido, testing paleta |
| `solid_blue.png` | PNG | 100x100 | Color sólido, testing paleta |
| `large.png` | PNG | 2000x2000 | Testing imágenes grandes |
| `corrupt/bad.png` | PNG | - | Testing manejo de errores |

### Generación de Fixtures

Los fixtures son **estáticos** (versionados en git). No se generan dinámicamente durante los tests porque:

1. **Determinismo**: Los tests deben ser reproducibles
2. **Velocidad**: Evitar overhead de generación
3. **Confianza**: Las imágenes de referencia son "la verdad"

Para agregar nuevos fixtures:

```bash
# Crear imagen de prueba con ImageMagick (opcional)
convert -size 100x100 xc:blue dpf/test_fixtures/solid_blue.png

# O usar cualquier herramienta de preferencia
```

### Uso en Tests

**Rust**:
```rust
fn fixture_path(name: &str) -> String {
    format!("{}/{}", fixtures_dir(), name)
}

// Uso
let input = fixture_path("sample.png");
```

**Go**:
```go
func getFixturePath(name string) string {
    return filepath.Join(getFixturesDir(), name)
}

// Uso
input := getFixturePath("sample.png")
```

---

## 5. Dependencias de Test

### Rust (Cargo.toml)

```toml
[dev-dependencies]
tempfile = "3"      # Directorios temporales para tests
assert_fs = "1"     # Assertions sobre filesystem
```

| Crate | Versión | Propósito |
|-------|---------|-----------|
| `tempfile` | 3.x | Crear directorios/archivos temporales que se auto-limpian |
| `assert_fs` | 1.x | Assertions específicas para filesystem (presets, exists, etc.) |

### Go (go.mod)

```go
module github.com/GustavoGutierrez/devpixelforge-bridge

go 1.23
```

Go solo usa la **librería estándar** para tests:

| Paquete | Propósito |
|---------|-----------|
| `testing` | Framework de tests nativo |
| `context` | Timeouts y cancelación |
| `os` | Filesystem operations |
| `path/filepath` | Manipulación de paths |
| `encoding/json` | Serialización/deserialización |
| `time` | Timeouts |

---

## 6. Diagrama de Flujo de Ejecución

### Flujo Completo de Testing

```
┌─────────────────────────────────────────────────────────────────┐
│                     Developer ejecuta tests                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  make test (o cargo test / go test)                             │
└─────────────────────────────────────────────────────────────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
                ▼               ▼               ▼
┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐
│   Rust Unit       │ │   Rust Integ      │ │   Go Bridge       │
│   (cargo test)    │ │   (cargo test)    │ │   (go test)       │
└───────────────────┘ └───────────────────┘ └───────────────────┘
        │                     │                     │
        ▼                     ▼                     ▼
┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐
│  Compila lib      │ │  Compila binario  │ │  Busca binario    │
│  En memoria       │ │  target/debug/dpf │ │  en debug/release │
│  Ejecuta tests    │ │                   │ │                   │
│  #[cfg(test)]     │ │                   │ │  Si no existe:    │
│                   │ │                   │ │  SKIP test        │
└───────────────────┘ └───────────────────┘ └───────────────────┘
                              │                     │
                              ▼                     ▼
                    ┌───────────────────┐ ┌───────────────────┐
                    │  Crea temp dirs   │ │  Crea t.TempDir() │
                    │  con tempfile     │ │                   │
                    └───────────────────┘ └───────────────────┘
                              │                     │
                              ▼                     ▼
                    ┌───────────────────┐ ┌───────────────────┐
                    │  Ejecuta CLI      │ │  Inicia Client    │
                    │  process/batch/   │ │  NewClient()      │
                    │  --stream         │ │                   │
                    │                   │ │  o StreamClient() │
                    └───────────────────┘ └───────────────────┘
                              │                     │
                              ▼                     ▼
                    ┌───────────────────┐ ┌───────────────────┐
                    │  Lee stdout JSON  │ │  stdin JSON job   │
                    │  Parsea resultado │ │  stdout result    │
                    └───────────────────┘ └───────────────────┘
                              │                     │
                              ▼                     ▼
                    ┌───────────────────┐ ┌───────────────────┐
                    │  Assertions       │ │  Assertions       │
                    │  - success: true  │ │  - Success: true  │
                    │  - outputs exist  │ │  - archivos exist │
                    │  - formato OK     │ │  - JSON parse OK  │
                    └───────────────────┘ └───────────────────┘
                              │                     │
                              └───────────┬─────────┘
                                          ▼
                    ┌───────────────────┐
                    │  Cleanup          │
                    │  - temp dirs      │
                    │  - Drop Client    │
                    │  - Close Stream   │
                    └───────────────────┘
                                          │
                                          ▼
                    ┌───────────────────┐
                    │  Reporte          │
                    │  passed/failed    │
                    └───────────────────┘
```

### Flujo de Datos en un Test de Integración

```
Test Code
    │
    ▼
┌─────────────────┐
│  build_binary() │  ← Compila dpf si no existe
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  TempDir::new() │  ← Directorio temporal único
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  job JSON       │  ← {"operation": "resize", "input": "...", ...}
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  Command::new   │  ← Ejecuta: dpf process --job '{...}'
│  (binary_path)  │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  dpf binario    │  ← Procesa imagen
│  (Rust)         │
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  stdout JSON    │  ← {"success": true, "outputs": [...]}
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  parse JSON     │  ← serde_json::from_str
└─────────────────┘
    │
    ▼
┌─────────────────┐
│  assert!()      │  ← Validaciones
└─────────────────┘
```

---

## Referencias Rápidas

### Comandos de Test

```bash
# Rust - Todos los tests
cd dpf && cargo test

# Rust - Solo unitarios
cd dpf && cargo test --lib

# Rust - Solo integración
cd dpf && cargo test --test integration_tests

# Rust - Un test específico
cd dpf && cargo test test_cli_resize -- --nocapture

# Go - Todos los tests
cd go-bridge && go test -v

# Go - Un test específico
cd go-bridge && go test -v -run TestExecuteResize

# Todos los tests del proyecto
make test
```

### Convenciones de Nomenclatura

| Lenguaje | Convención | Ejemplo |
|----------|------------|---------|
| Rust | `test_<modulo>_<escenario>` | `test_cli_process_resize` |
| Go | `Test<Componente><Accion>` | `TestExecuteResize` |

---

## Notas de Mantenimiento

1. **Agregar un nuevo tipo de fixture**: Copiar archivo a `dpf/test_fixtures/` y documentar en esta guía
2. **Agregar test de integración**: Añadir a `dpf/tests/integration_tests.rs`
3. **Agregar test de bridge**: Añadir a `go-bridge/dpf_test.go`
4. **Tests que requieren binario**: Siempre verificar existencia con `setupBinary()` o `build_binary()`
5. **Aislamiento**: Siempre usar directorios temporales, nunca escribir en `test_fixtures/`
