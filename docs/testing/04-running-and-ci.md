# Ejecución de Tests y CI/CD

Guía completa para ejecutar tests localmente y configurar CI/CD para el proyecto devpixelforge.

---

## Ejecución Local

### Tests en Rust

```bash
# Todos los tests
cd dpf && cargo test

# Tests con output detallado
cargo test -- --nocapture

# Tests específicos por nombre
cargo test test_resize

# Tests con optimizaciones de release (más rápido para tests de rendimiento)
cargo test --release

# Tests de integración únicamente
cargo test --test '*'

# Tests unitarios únicamente
cargo test --lib
```

### Tests en Go

```bash
cd go-bridge

# Todos los tests
go test ./...

# Tests con verbose
go test -v ./...

# Tests de un paquete específico
go test ./pkg/bridge

# Tests con cobertura
go test -cover ./...
go test -coverprofile=coverage.out ./...
go tool cover -html=coverage.out

# Tests con timeout extendido (para operaciones de imagen)
go test -v -timeout 5m ./...
```

### Tests de integración End-to-End

```bash
# Construir primero el binario Rust
make build-rust

# Ejecutar test de capacidades
./dpf/target/release/dpf caps

# Test de resize manual
echo '{"operation":"resize","input":"test.png","output_dir":"/tmp/test","widths":[320,640]}' | \
    ./dpf/target/release/dpf process --job -
```

---

## Scripts Útiles

### Script: Ejecutar Todo (`scripts/test-all.sh`)

```bash
#!/bin/bash
set -e

echo "🧪 Ejecutando todos los tests..."

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Tests de Rust
echo -e "${YELLOW}▶ Tests de Rust...${NC}"
cd dpf
cargo test --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Rust tests PASSED${NC}"
else
    echo -e "${RED}❌ Rust tests FAILED${NC}"
    exit 1
fi
cd ..

# Construir binario para tests de Go
echo -e "${YELLOW}▶ Construyendo binario Rust...${NC}"
make build-rust

# Tests de Go
echo -e "${YELLOW}▶ Tests de Go...${NC}"
cd go-bridge
go test -v ./...
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Go tests PASSED${NC}"
else
    echo -e "${RED}❌ Go tests FAILED${NC}"
    exit 1
fi
cd ..

# Test de integración
echo -e "${YELLOW}▶ Test de integración (caps)...${NC}"
./dpf/target/release/dpf caps
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Integration test PASSED${NC}"
else
    echo -e "${RED}❌ Integration test FAILED${NC}"
    exit 1
fi

echo -e "${GREEN}🎉 Todos los tests pasaron!${NC}"
```

### Script: Tests Rápidos (`scripts/test-quick.sh`)

```bash
#!/bin/bash
# Tests rápidos - útiles para pre-commit

echo "⚡ Tests rápidos..."

# Solo unit tests de Rust (sin tests de integración que necesitan imágenes)
cd dpf
cargo test --lib --release

# Verificación de formato
cargo fmt -- --check

# Clippy
cargo clippy -- -D warnings

cd ../go-bridge

# Format check
gofmt -d .

# Build check
go build ./...

echo "✅ Tests rápidos completados"
```

### Script: Regenerar Fixtures (`scripts/regenerate-fixtures.sh`)

```bash
#!/bin/bash
# Regenera las imágenes de prueba/fixtures

FIXTURES_DIR="dpf/tests/fixtures"
mkdir -p "$FIXTURES_DIR"

echo "🔄 Regenerando fixtures..."

# Crear imágenes de prueba con ImageMagick (si está disponible)
if command -v convert &> /dev/null; then
    # Imagen PNG de prueba
    convert -size 100x100 xc:blue "$FIXTURES_DIR/test_100x100.png"
    convert -size 800x600 xc:red "$FIXTURES_DIR/test_800x600.png"
    convert -size 1920x1080 xc:green "$FIXTURES_DIR/test_1920x1080.png"
    
    # Con transparencia
    convert -size 200x200 xc:none -pointsize 30 -fill black \
        -gravity center -annotate +0+0 "TEST" \
        "$FIXTURES_DIR/test_transparent.png"
    
    # JPEG
    convert -size 640x480 xc:orange "$FIXTURES_DIR/test_640x480.jpg"
    
    # WebP
    convert -size 400x300 xc:purple "$FIXTURES_DIR/test_400x300.webp"
    
    echo "✅ Fixtures regenerados en $FIXTURES_DIR"
else
    echo "⚠️  ImageMagick no encontrado. Instálalo para regenerar fixtures."
    exit 1
fi

# Listar fixtures generados
ls -la "$FIXTURES_DIR"
```

---

## CI/CD (GitHub Actions)

### Workflow Completo para Rust

Crear `.github/workflows/rust.yml`:

```yaml
name: Rust CI

on:
  push:
    branches: [ main, develop ]
    paths:
      - 'dpf/**'
      - '.github/workflows/rust.yml'
  pull_request:
    branches: [ main ]
    paths:
      - 'dpf/**'

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust toolchain
      uses: dtolnay/rust-action@stable
      with:
        toolchain: stable
    
    - name: Cache cargo dependencies
      uses: Swatinem/rust-cache@v2
      with:
        workspaces: dpf
        cache-directories: |
          ~/.cargo/registry
          ~/.cargo/git
          dpf/target
    
    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libjpeg-dev libpng-dev libwebp-dev
    
    - name: Check formatting
      run: |
        cd dpf
        cargo fmt -- --check
    
    - name: Run clippy
      run: |
        cd dpf
        cargo clippy -- -D warnings
    
    - name: Run tests
      run: |
        cd dpf
        cargo test --release --verbose
    
    - name: Build release
      run: |
        cd dpf
        cargo build --release --verbose
    
    - name: Upload binary artifact
      uses: actions/upload-artifact@v4
      with:
        name: dpf-binary-linux
        path: dpf/target/release/dpf
        retention-days: 7

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Rust toolchain
      uses: dtolnay/rust-action@stable
    
    - name: Install cargo-tarpaulin
      run: cargo install cargo-tarpaulin
    
    - name: Cache cargo
      uses: Swatinem/rust-cache@v2
      with:
        workspaces: dpf
    
    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libjpeg-dev libpng-dev libwebp-dev
    
    - name: Generate coverage
      run: |
        cd dpf
        cargo tarpaulin --out xml --out html --release
    
    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        files: dpf/cobertura.xml
        fail_ci_if_error: false
    
    - name: Upload HTML report
      uses: actions/upload-artifact@v4
      with:
        name: coverage-report
        path: dpf/tarpaulin-report.html
```

### Workflow para Go

Crear `.github/workflows/go.yml`:

```yaml
name: Go CI

on:
  push:
    branches: [ main, develop ]
    paths:
      - 'go-bridge/**'
      - 'dpf/target/**'
      - '.github/workflows/go.yml'
  pull_request:
    branches: [ main ]
    paths:
      - 'go-bridge/**'

jobs:
  test:
    name: Go Tests
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up Go
      uses: actions/setup-go@v5
      with:
        go-version: '1.21'
        cache: true
        cache-dependency-path: go-bridge/go.sum
    
    - name: Download binary artifact
      uses: actions/download-artifact@v4
      with:
        name: dpf-binary-linux
        path: dpf/target/release/
    
    - name: Make binary executable
      run: chmod +x dpf/target/release/dpf
    
    - name: Get dependencies
      run: |
        cd go-bridge
        go mod download
    
    - name: Build
      run: |
        cd go-bridge
        go build -v ./...
    
    - name: Test
      run: |
        cd go-bridge
        go test -v -race -coverprofile=coverage.out ./...
      env:
        DPF_BINARY_PATH: ${{ github.workspace }}/dpf/target/release/dpf
    
    - name: Upload coverage
      uses: actions/upload-artifact@v4
      with:
        name: go-coverage
        path: go-bridge/coverage.out
```

### Workflow Combinado

Ver archivo adjunto: [`github-workflow-example.yml`](github-workflow-example.yml)

### Cómo Cachear Dependencias

**Rust:**

```yaml
- name: Cache Rust
  uses: Swatinem/rust-cache@v2
  with:
    workspaces: dpf
    shared-key: "rust-cache-${{ runner.os }}"
```

**Go:**

```yaml
- name: Set up Go
  uses: actions/setup-go@v5
  with:
    go-version: '1.21'
    cache: true
    cache-dependency-path: go-bridge/go.sum
```

### Cómo Guardar Resultados de Tests

```yaml
- name: Test with JUnit output
  run: |
    cd dpf
    cargo test --release -- --format=junit > test-results.xml
  continue-on-error: true

- name: Upload test results
  uses: actions/upload-artifact@v4
  if: always()
  with:
    name: test-results
    path: dpf/test-results.xml

- name: Publish Test Results
  uses: EnricoMi/publish-unit-test-result-action@v2
  if: always()
  with:
    files: dpf/test-results.xml
```

---

## Cobertura de Código

### Rust con cargo-tarpaulin

**Instalación:**

```bash
cargo install cargo-tarpaulin
```

**Generar reportes:**

```bash
cd dpf

# HTML report
cargo tarpaulin --out html

# XML para CI (cobertura)
cargo tarpaulin --out xml

# LCOV para integración con otras herramientas
cargo tarpaulin --out lcov

# Todos los formatos
cargo tarpaulin --out html --out xml --out lcov

# Ver reporte HTML
open tarpaulin-report.html  # macOS
xdg-open tarpaulin-report.html  # Linux
```

**Configuración en `.tarpaulin.toml`:**

```toml
[default]
exclude-files = ["tests/*", "examples/*"]
exclude = ["test_*"]
run-types = ["Tests", "Doctests"]
out = ["Html", "Xml"]
release = true
```

### Go con cobertura nativa

```bash
cd go-bridge

# Cobertura por paquete
go test -cover ./...

# Reporte detallado
go test -coverprofile=coverage.out ./...

# Ver en HTML
go tool cover -html=coverage.out -o coverage.html

# Ver por función
go tool cover -func=coverage.out

# Cobertura con threshold (falla si < 80%)
go test -coverprofile=coverage.out ./... && \
    go tool cover -func=coverage.out | \
    awk '/total:/ {print $3}' | \
    sed 's/%//' | \
    awk '{if ($1 < 80) exit 1}'
```

**Integración con Codecov:**

```yaml
- name: Upload to Codecov
  uses: codecov/codecov-action@v3
  with:
    files: ./dpf/cobertura.xml,./go-bridge/coverage.out
    flags: unittests
    name: codecov-umbrella
```

---

## Solución de Problemas

### "Binary not found" en tests de Go

**Problema:** Los tests de Go no encuentran el binario Rust.

**Soluciones:**

```bash
# 1. Construir primero
make build-rust

# 2. Establecer variable de entorno
export DPF_BINARY_PATH="$(pwd)/dpf/target/release/dpf"

# 3. O modificar el test para buscar en múltiples ubicaciones:
func findBinary() string {
    locations := []string{
        os.Getenv("DPF_BINARY_PATH"),
        "../dpf/target/release/dpf",
        "./dpf/target/release/dpf",
        "./target/release/dpf",
    }
    for _, loc := range locations {
        if loc != "" && fileExists(loc) {
            return loc
        }
    }
    log.Fatal("Binary not found")
    return ""
}
```

### Tests lentos (AVIF)

**Problema:** Los tests con formato AVIF son extremadamente lentos.

**Soluciones:**

1. **Usar `--release` en tests:**
   ```bash
   cargo test --release
   ```

2. **Etiquetar tests lentos:**
   ```rust
   #[test]
   #[ignore = "slow - AVIF encoding"]
   fn test_avif_encoding() { ... }
   ```

3. **Ejecutar tests ignorados solo en CI completo:**
   ```bash
   # Local (rápido)
   cargo test
   
   # CI completo
   cargo test -- --ignored
   ```

4. **Timeout en tests:**
   ```rust
   #[test]
   #[timeout(Duration::from_secs(30))]
   fn test_slow_operation() { ... }
   ```

### Fixtures faltantes

**Problema:** Tests fallan porque no encuentran imágenes de prueba.

**Soluciones:**

```bash
# 1. Ejecutar script de regeneración
./scripts/regenerate-fixtures.sh

# 2. O crear fixtures mínimos manualmente:
mkdir -p dpf/tests/fixtures

# Con ImageMagick
convert -size 100x100 xc:blue dpf/tests/fixtures/test.png

# O descargar fixtures de ejemplo
# (agregar a setup de tests)
```

**En CI:**

```yaml
- name: Generate test fixtures
  run: |
    sudo apt-get install -y imagemagick
    ./scripts/regenerate-fixtures.sh
```

---

## Pre-commit Hooks (Opcional)

Instalar [pre-commit](https://pre-commit.com/):

```bash
pip install pre-commit
pre-commit install
```

Crear `.pre-commit-config.yaml`:

```yaml
repos:
  # Rust
  - repo: local
    hooks:
      - id: cargo-fmt
        name: Rust fmt
        entry: bash -c 'cd dpf && cargo fmt -- --check'
        language: system
        files: \.rs$
        pass_filenames: false
      
      - id: cargo-clippy
        name: Rust clippy
        entry: bash -c 'cd dpf && cargo clippy -- -D warnings'
        language: system
        files: \.rs$
        pass_filenames: false
      
      - id: cargo-test-quick
        name: Rust tests (quick)
        entry: bash -c 'cd dpf && cargo test --lib --release'
        language: system
        files: \.rs$
        pass_filenames: false
  
  # Go
  - repo: local
    hooks:
      - id: go-fmt
        name: Go fmt
        entry: bash -c 'cd go-bridge && gofmt -d .'
        language: system
        files: \.go$
        pass_filenames: false
      
      - id: go-build
        name: Go build
        entry: bash -c 'cd go-bridge && go build ./...'
        language: system
        files: \.go$
        pass_filenames: false
      
      - id: go-test-quick
        name: Go tests (quick)
        entry: bash -c 'cd go-bridge && go test -short ./...'
        language: system
        files: \.go$
        pass_filenames: false

  # General
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.5.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-added-large-files
```

**Configuración opcional para skips:**

```bash
# Saltar tests lentos en commits rápidos
SKIP=cargo-test-quick git commit -m "WIP"

# Forzar todos los checks
pre-commit run --all-files
```

---

## Referencias

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo-tarpaulin Documentation](https://github.com/xd009642/tarpaulin)
- [Go Testing](https://golang.org/pkg/testing/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
