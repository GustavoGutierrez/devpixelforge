# Testing Documentation

Documentación completa de la suite de tests para **devpixelforge**.

---

## 📚 Índice de Documentos

| Documento | Descripción | Público Objetivo |
|-----------|-------------|------------------|
| [`01-architecture.md`](01-architecture.md) | Arquitectura y estructura de tests | Arquitectos, Tech Leads |
| [`02-rust-tests.md`](02-rust-tests.md) | Guía de tests Rust (unitarios e integración) | Desarrolladores Rust |
| [`03-go-tests.md`](03-go-tests.md) | Guía de tests del bridge Go | Desarrolladores Go |
| [`04-running-and-ci.md`](04-running-and-ci.md) | Ejecución, CI/CD y solución de problemas | DevOps, CI Engineers |

---

## 🚀 Guía Rápida

### Ejecutar Todos los Tests

```bash
# En la raíz del proyecto
make test              # Test básico (solo Rust caps)

# Tests completos Rust
cd dpf && cargo test   # 57 tests (45 unit + 12 integration)

# Tests Go
cd go-bridge && go test -v

# Script completo (ver docs/testing/04-running-and-ci.md)
./scripts/test-all.sh
```

### Tests por Componente

```bash
# Solo tests unitarios Rust
cd dpf && cargo test --lib

# Solo tests de integración Rust
cd dpf && cargo test --test integration_tests

# Tests Go (requiere binario Rust compilado)
cd go-bridge && go test -v -run TestExecute
```

---

## 📊 Resumen de Tests

| Lenguaje | Tipo | Cantidad | Ubicación |
|----------|------|----------|-----------|
| **Rust** | Unitarios | 45 | `src/*/tests` (inline) |
| **Rust** | Integración | 12 | `tests/integration_tests.rs` |
| **Go** | Unitarios + Integración | 17 | `dpf_test.go` |
| **Total** | - | **74** | - |

---

## 🏗️ Arquitectura de Tests

```
docs/testing/
├── 01-architecture.md          # Arquitectura y estructura
├── 02-rust-tests.md            # Guía Rust completa
├── 03-go-tests.md              # Guía Go completa
├── 04-running-and-ci.md        # CI/CD y operación
├── github-workflow-example.yml # Workflow de ejemplo
└── README.md                   # Este archivo

dpf/
├── src/
│   └── operations/
│       ├── utils.rs            # #[cfg(test)] mod tests
│       ├── resize.rs           # #[cfg(test)] mod tests
│       └── convert.rs          # #[cfg(test)] mod tests
├── tests/
│   └── integration_tests.rs    # Tests CLI/integración
├── examples/
│   └── gen_fixtures.rs         # Generador de fixtures
└── test_fixtures/              # Imágenes de prueba
    ├── sample.png
    ├── sample.jpg
    ├── sample.svg
    └── corrupt/bad.png

go-bridge/
└── dpf_test.go                 # Tests del cliente Go
```

---

## 🔧 Fixtures de Prueba

Las imágenes de prueba se generan automáticamente:

```bash
cd dpf && cargo run --example gen_fixtures
```

**Fixtures disponibles:**
- `sample.png` - PNG RGBA 100x100 con gradiente
- `sample.jpg` - JPEG 100x100
- `sample.svg` - SVG vector 100x100
- `sample_transparent.png` - PNG con canal alpha
- `large.png` - PNG grande 1000x1000
- `solid_red.png` / `solid_blue.png` - Colores sólidos
- `corrupt/bad.png` - Archivo corrupto para tests de error

---

## 📝 Convenciones

### Rust
- Tests inline con `#[cfg(test)] mod tests`
- Nomenclatura: `test_<funcion>_<caso>`
- Usar `tempfile::TempDir` para directorios temporales

### Go
- Archivo `*_test.go` separado
- Nomenclatura: `Test<Componente><Caso>`
- Usar `t.TempDir()` para directorios temporales
- Tests de integración hacen `t.Skip()` si no hay binario

---

## 🐛 Solución Rápida de Problemas

### "Binary not found" en tests Go
```bash
cd dpf && cargo build  # Compilar binario debug
cd go-bridge && go test -v
```

### Tests lentos
```bash
# Excluir tests de AVIF (lentos)
cargo test -- --skip avif

# Ejecutar tests en paralelo (por defecto)
cargo test -- --test-threads=8
```

### Fixtures faltantes
```bash
cd dpf && cargo run --example gen_fixtures
```

---

## 📖 Ejemplos de Uso

### Agregar un test unitario en Rust

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mi_funcion_caso_exitoso() {
        // Arrange
        let input = "test.png";
        
        // Act
        let result = mi_funcion(input);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

### Agregar un test en Go

```go
func TestMiFuncion(t *testing.T) {
    // Arrange
    client := NewClient(getBinaryPath())
    
    // Act
    result, err := client.MiFuncion(ctx, args)
    
    // Assert
    if err != nil {
        t.Fatalf("unexpected error: %v", err)
    }
    if !result.Success {
        t.Error("expected success")
    }
}
```

---

## 🔗 Referencias

- [Documentación oficial de testing en Rust](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Documentación oficial de testing en Go](https://golang.org/pkg/testing/)
- [Cargo Test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)

---

## 🤝 Contribuir

Al agregar nuevos tests:

1. **Seguir las convenciones** de nomenclatura
2. **Documentar el propósito** del test en comentarios
3. **Usar fixtures existentes** cuando sea posible
4. **Limpiar recursos** (temp files, etc.)
5. **Verificar en CI** antes de hacer push

---

## 📊 Estado de Tests

```
Rust Unitarios:    45 ✅
Rust Integración:  12 ✅
Go Tests:          17 ✅
────────────────────────────
Total:             74 ✅
```

Última actualización: Marzo 2025
