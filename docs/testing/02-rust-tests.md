# Guía de Tests Rust para devpixelforge

Este documento describe el sistema de testing del componente Rust (`dpf/`) de devpixelforge.

---

## 1. Tests Unitarios

### Organización

Los tests unitarios en Rust se organizan **inline** en cada módulo, dentro de un bloque `#[cfg(test)]` al final del archivo fuente:

```rust
// src/operations/utils.rs

pub fn fit_dimensions(...) -> (u32, u32) {
    // implementación...
}

#[cfg(test)]
mod tests {
    use super::*;
    // tests aquí...
}
```

Esta convención permite:
- Acceso a funciones privadas del módulo (`use super::*`)
- Compilación condicional (solo en modo test)
- Cohesión: tests junto al código que prueban

### Convenciones de Nomenclatura

Todos los tests siguen el prefijo `test_`:

```rust
#[test]
fn test_fit_dimensions_both_dimensions() { }

#[test]
fn test_load_image_png() { }

#[test]
fn test_save_image_jpeg() { }
```

Para tests que están temporalmente deshabilitados:

```rust
#[test]
#[ignore = "linear_rgb feature has implementation issues - needs investigation"]
fn test_resize_with_linear_rgb() { }
```

### Ejecución

```bash
# Todos los tests unitarios
cargo test --lib

# Tests de un módulo específico
cargo test --lib utils::tests
cargo test --lib resize::tests

# Un test específico
cargo test test_fit_dimensions_both_dimensions

# Tests ignorados incluidos
cargo test --lib -- --ignored

# Output detallado
cargo test --lib -- --nocapture
```

---

## 2. Tests por Módulo

### 2.1 `utils.rs`

Ubicación: `dpf/src/operations/utils.rs`

Este módulo contiene funciones de utilidad para carga/guardado de imágenes y cálculos de dimensiones.

**Funciones testeadas:**

| Función | Tests | Descripción |
|---------|-------|-------------|
| `fit_dimensions` | 5 tests | Cálculo de dimensiones manteniendo aspect ratio |
| `load_image` | 4 tests | Carga de PNG, JPEG, SVG; manejo de errores |
| `save_image` | 6 tests | Guardado en PNG, JPEG, WebP, ICO, AVIF |
| `file_size` | 2 tests | Obtención de tamaño de archivo |
| `ensure_parent_dir` | 3 tests | Creación de directorios anidados |

**Casos de prueba principales:**

```rust
// fit_dimensions - mantener aspect ratio
#[test]
fn test_fit_dimensions_both_dimensions() {
    let (w, h) = fit_dimensions(1000, 500, Some(500), Some(200));
    // Ratio w: 0.5, ratio h: 0.4 → usa el menor
    assert_eq!(w, 400);
    assert_eq!(h, 200);
}

// Carga de diferentes formatos
#[test]
fn test_load_image_png() {
    let path = fixture_path("sample.png");
    let img = load_image(&path).expect("Failed to load PNG");
    assert_eq!(img.width(), 100);
    assert_eq!(img.height(), 100);
}

// Manejo de errores
#[test]
fn test_load_image_not_found() {
    let result = load_image("/nonexistent/path/image.png");
    assert!(result.is_err());
}

#[test]
fn test_load_image_corrupt() {
    let path = fixture_path("corrupt/bad.png");
    let result = load_image(&path);
    assert!(result.is_err());
}
```

### 2.2 `resize.rs`

Ubicación: `dpf/src/operations/resize.rs`

Operación de redimensionamiento de imágenes con soporte para múltiples variantes.

**Casos de prueba principales:**

**Tests básicos de resize:**

```rust
#[test]
fn test_resize_single_width() {
    let params = ResizeParams {
        input: fixture_path("sample.png"),
        output_dir: temp_dir.path().to_str().unwrap().to_string(),
        widths: Some(vec![50]),
        scale_percent: None,
        max_height: None,
        format: Some("png".to_string()),
        quality: Some(85),
        filter: None,
        linear_rgb: false,
        inline: false,
    };

    let result = execute(params).expect("Resize failed");
    assert!(result.success);
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.outputs[0].width, 50);
}
```

**Tests de validación de parámetros:**

```rust
#[test]
fn test_resize_invalid_scale_percent_zero() {
    let params = ResizeParams {
        scale_percent: Some(0.0), // Inválido
        // ...
    };
    let result = execute(params);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("scale_percent"));
}

#[test]
fn test_resize_missing_widths_and_scale() {
    let params = ResizeParams {
        widths: None,
        scale_percent: None, // Ambos faltan → error
        // ...
    };
    let result = execute(params);
    assert!(result.is_err());
}
```

**Tests de filtros de resize:**

```rust
#[test]
fn test_resize_with_filter_lanczos3() { }
#[test]
fn test_resize_with_filter_nearest() { }
#[test]
fn test_resize_with_filter_triangle() { }
#[test]
fn test_resize_with_filter_catmullrom() { }
```

**Tests de formatos de salida:**

```rust
#[test]
fn test_resize_output_jpeg() { }
#[test]
fn test_resize_output_webp() { }
```

**Tests de opciones avanzadas:**

```rust
#[test]
fn test_resize_with_max_height() {
    // Verifica que max_height limite las dimensiones
}

#[test]
fn test_resize_with_inline() {
    // Verifica que data_base64 se genere correctamente
}
```

### 2.3 `convert.rs`

Ubicación: `dpf/src/operations/convert.rs`

Operación de conversión entre formatos de imagen.

**Casos de prueba principales:**

**Tests de conversión básica:**

```rust
#[test]
fn test_convert_png_to_jpeg() {
    let params = ConvertParams {
        input: fixture_path("sample.png"),
        output: output_path.to_str().unwrap().to_string(),
        format: "jpeg".to_string(),
        quality: Some(90),
        // ...
    };

    let result = execute(params).expect("Convert PNG to JPEG failed");
    assert!(result.success);
    assert_eq!(result.outputs[0].format, "jpeg");
    assert!(output_path.exists());
}
```

**Tests de conversión entre formatos:**

```rust
#[test]
fn test_convert_png_to_webp() { }
#[test]
fn test_convert_png_to_ico() {      // ICO con múltiples tamaños
#[test]
fn test_convert_jpeg_to_png() { }
#[test]
fn test_convert_to_avif() { }
```

**Tests de conversión SVG:**

```rust
#[test]
fn test_convert_svg_to_png() {
    // Conversión de vector a raster
}

#[test]
fn test_convert_svg_to_png_with_custom_size() {
    // SVG rasterizado a tamaño específico
    let params = ConvertParams {
        width: Some(200),
        height: Some(200),
        // ...
    };
    assert_eq!(result.outputs[0].width, 200);
    assert_eq!(result.outputs[0].height, 200);
}
```

**Tests de inline base64:**

```rust
#[test]
fn test_convert_with_inline() {
    let params = ConvertParams { inline: true, ... };
    let result = execute(params).expect("Convert with inline failed");
    
    assert!(result.outputs[0].data_base64.is_some());
    let b64 = result.outputs[0].data_base64.as_ref().unwrap();
    assert!(!b64.is_empty());
    
    // Verificar que es base64 válido
    let decoded = base64::engine::general_purpose::STANDARD.decode(b64);
    assert!(decoded.is_ok());
}
```

---

## 3. Tests de Integración

Ubicación: `dpf/tests/integration_tests.rs`

Los tests de integración prueban el CLI compilado en lugar de importar el crate como librería. Esto valida el pipeline completo desde la línea de comandos.

### Qué se testea

| Área | Tests | Descripción |
|------|-------|-------------|
| CLI básico | 4 tests | Comando `caps`, `process` con resize/convert |
| Batch | 1 test | Procesamiento múltiple jobs |
| Streaming | 1 test | Modo `--stream` con stdin |
| Operaciones | 2 tests | Favicon generation, SVG conversion |
| JSON | 3 tests | Serialización/deserialización |

### Cómo usan el binario compilado

```rust
fn binary_path() -> String {
    format!("{}/target/debug/dpf", env!("CARGO_MANIFEST_DIR"))
}

fn build_binary() {
    let binary = binary_path();
    if !std::path::Path::new(&binary).exists() {
        let output = Command::new("cargo")
            .args(["build"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to build binary");
        // ...
    }
}

#[test]
fn test_cli_caps_command() {
    build_binary();

    let output = Command::new(binary_path())
        .arg("caps")
        .output()
        .expect("Failed to execute caps command");

    assert!(output.status.success());
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Caps output should be valid JSON");
    
    assert!(json.get("input_formats").is_some());
    assert!(json.get("output_formats").is_some());
}
```

### Ejemplo de test de CLI con JSON

```rust
#[test]
fn test_cli_process_resize() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_str().unwrap();

    let job_json = json!({
        "operation": "resize",
        "input": fixture_path("sample.png"),
        "output_dir": output_dir,
        "widths": [50, 100],
        "format": "png"
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute process command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JobResult JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "resize");
}
```

### Test de streaming mode

```rust
#[test]
fn test_stream_mode_single_job() {
    build_binary();

    use std::io::Write;
    use std::process::Stdio;

    let job_json = json!({
        "operation": "resize",
        "input": fixture_path("sample.png"),
        "output_dir": output_dir,
        "widths": [50],
        "format": "png"
    });

    let mut child = Command::new(binary_path())
        .arg("--stream")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start stream process");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        writeln!(stdin, "{}", job_json).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    // Verificar output...
}
```

### Ejecución de tests de integración

```bash
# Todos los tests de integración
cargo test --test integration_tests

# Un test específico
cargo test test_cli_process_resize --test integration_tests

# Con output visible
cargo test --test integration_tests -- --nocapture
```

---

## 4. Fixtures

### Ubicación

```
dpf/
├── test_fixtures/
│   ├── sample.png              # 100x100 RGBA
│   ├── sample.jpg              # 100x100 JPEG
│   ├── sample_transparent.png  # 100x100 con alpha
│   ├── sample.svg              # Vector 100x100
│   ├── large.png               # 1000x1000 para tests de rendimiento
│   ├── solid_red.png           # Color sólido para tests
│   ├── solid_blue.png          # Color sólido para tests
│   └── corrupt/
│       └── bad.png             # Archivo corrupto intencionalmente
```

### Generación de fixtures

Las fixtures se generan mediante un ejemplo ejecutable:

```bash
cd dpf
cargo run --example gen_fixtures
```

Código en: `dpf/examples/gen_fixtures.rs`

Este script genera:
- Imágenes con gradientes (para tests de resize quality)
- Imágenes con transparencia
- SVG con elementos geométricos
- Archivos corruptos (para tests de error handling)

### Acceso desde tests

```rust
fn fixtures_dir() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
}

fn fixture_path(name: &str) -> String {
    format!("{}/{}", fixtures_dir(), name)
}

// Uso en tests
#[test]
fn test_load_image_png() {
    let path = fixture_path("sample.png");
    let img = load_image(&path).expect("Failed to load PNG");
    // ...
}
```

`CARGO_MANIFEST_DIR` apunta al directorio que contiene `Cargo.toml` del crate.

---

## 5. Helpers de Test

### `tempfile::TempDir`

Todos los tests que escriben archivos usan directorios temporales que se auto-limpian:

```rust
use tempfile::TempDir;

#[test]
fn test_save_image_png() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test_output.png");

    let img = image::DynamicImage::new_rgba8(50, 50);
    save_image(&img, output_path.to_str().unwrap(), "png", 85)
        .expect("Failed to save PNG");

    assert!(output_path.exists());
    // temp_dir se limpia automáticamente al salir del scope
}
```

### Funciones auxiliares estándar

```rust
// En cada módulo de tests:

fn fixtures_dir() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
}

fn fixture_path(name: &str) -> String {
    format!("{}/{}", fixtures_dir(), name)
}
```

### Dependencias de test

En `Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3.8"
```

---

## 6. Cómo Agregar Nuevos Tests

### 6.1 Test unitario para función existente

```rust
// En el archivo del módulo (ej: src/operations/my_module.rs)

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixtures_dir() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/{}", fixtures_dir(), name)
    }

    #[test]
    fn test_my_new_function_success() {
        // Arrange
        let input = fixture_path("sample.png");
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("result.png");

        // Act
        let result = my_new_function(&input, output.to_str().unwrap());

        // Assert
        assert!(result.is_ok());
        assert!(output.exists());
    }

    #[test]
    fn test_my_new_function_error_handling() {
        let result = my_new_function("/nonexistent/file.png", "/tmp/out.png");
        assert!(result.is_err());
    }
}
```

### 6.2 Test de integración para CLI

```rust
// En tests/integration_tests.rs

#[test]
fn test_cli_my_new_operation() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_str().unwrap();

    let job_json = json!({
        "operation": "my_new_operation",
        "input": fixture_path("sample.png"),
        "output_dir": output_dir,
        "param1": "value1"
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute process command");

    assert!(
        output.status.success(),
        "Process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "my_new_operation");
}
```

### 6.3 Agregar nueva fixture

```rust
// En examples/gen_fixtures.rs

fn main() {
    let fixtures_dir = "test_fixtures";
    
    // Tu nueva fixture
    let my_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(200, 200);
    // ... configurar píxeles ...
    my_img
        .save(format!("{}/my_fixture.png", fixtures_dir))
        .expect("Failed to save my_fixture.png");
    println!("✓ Created my_fixture.png (200x200)");
}
```

Luego ejecutar:

```bash
cargo run --example gen_fixtures
```

### 6.4 Patrones recomendados

**AAA Pattern (Arrange-Act-Assert):**

```rust
#[test]
fn test_resize_with_quality() {
    // Arrange
    let temp_dir = TempDir::new().unwrap();
    let params = ResizeParams {
        input: fixture_path("large.png"),
        output_dir: temp_dir.path().to_str().unwrap().to_string(),
        widths: Some(vec![500]),
        quality: Some(95), // Alta calidad
        ..Default::default()
    };

    // Act
    let result = execute(params);

    // Assert
    assert!(result.is_ok());
    let output = &result.unwrap().outputs[0];
    assert_eq!(output.width, 500);
    assert!(output.size_bytes > 0);
}
```

**Tests parametrizados (tabla):**

```rust
#[test]
fn test_save_image_various_formats() {
    let formats = vec![
        ("png", 85),
        ("jpeg", 90),
        ("webp", 80),
    ];

    for (format, quality) in formats {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join(format!("output.{}", format));
        
        let img = image::DynamicImage::new_rgba8(50, 50);
        let result = save_image(&img, output_path.to_str().unwrap(), format, quality);
        
        assert!(result.is_ok(), "Failed to save as {}", format);
        assert!(output_path.exists());
    }
}
```

---

## 7. Ejecución Completa

```bash
# Todos los tests (unitarios + integración)
cargo test

# Solo tests unitarios
cargo test --lib

# Solo tests de integración
cargo test --test integration_tests

# Con coverage (requiere cargo-tarpaulin)
cargo tarpaulin --lib --tests

# Tests en release mode (más rápido para tests de integración)
cargo test --release --test integration_tests
```

---

## Referencias

- [Rust Testing Documentation](https://doc.rust-lang.org/rust-by-example/testing.html)
- [Tempfile Crate](https://docs.rs/tempfile/)
- Archivos de ejemplo:
  - `dpf/src/operations/utils.rs` - Tests de funciones de utilidad
  - `dpf/src/operations/resize.rs` - Tests de operación de resize
  - `dpf/src/operations/convert.rs` - Tests de conversión de formatos
  - `dpf/tests/integration_tests.rs` - Tests de integración CLI
  - `dpf/examples/gen_fixtures.rs` - Generador de fixtures
