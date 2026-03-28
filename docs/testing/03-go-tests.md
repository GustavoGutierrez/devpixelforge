# Guía de Tests para el Bridge de Go

Esta guía documenta cómo testear el bridge de Go (`go-bridge/`) que comunica Go con el motor Rust de procesamiento de imágenes.

---

## 1. Estructura de Tests

### Archivo Principal

Los tests están en el archivo **`go-bridge/dpf_test.go`**.

Go organiza los tests usando funciones que comienzan con `Test`:

```go
// Función de test básica
func TestAlgo(t *testing.T) {
    // código del test
}

// Test con sub-tests
func TestAlgo(t *testing.T) {
    t.Run("subtest 1", func(t *testing.T) {
        // código
    })
    
    t.Run("subtest 2", func(t *testing.T) {
        // código
    })
}
```

**Convenciones del archivo:**
- Tests agrupados por funcionalidad con separadores de comentarios
- Helpers al final del archivo
- Cada test debe ser independiente (no depender de otros tests)

---

## 2. Tipos de Tests

### 2.1 Tests Unitarios (Sin Binario)

Estos tests no requieren el binario Rust compilado. Verifican:

- **Serialización JSON**: Que las estructuras se serializan correctamente
- **Deserialización**: Que los responses JSON se parsean bien
- **Estructuras de datos**: Valores por defecto, punteros, etc.

```go
func TestResizeJobJSON(t *testing.T) {
    job := &ResizeJob{
        Operation: "resize",
        Input:     "test.png",
        Widths:    []uint32{100, 200},
    }
    
    data, err := json.Marshal(job)
    if err != nil {
        t.Fatalf("JSON marshal failed: %v", err)
    }
    
    // Verificaciones...
}
```

**Ventajas:**
- Rápidos (no compilan Rust)
- No requieren fixtures
- Ideales para CI cuando el binario no está disponible

### 2.2 Tests de Integración (Requieren Binario)

Estos tests ejecutan el binario Rust real. Requieren:

1. El binario compilado en `dpf/target/debug/dpf` o `dpf/target/release/dpf`
2. Fixtures de imágenes en `dpf/test_fixtures/`

```go
func TestExecuteResize(t *testing.T) {
    binaryPath := setupBinary(t)  // Skip si no existe binario
    client := NewClient(binaryPath)
    
    job := &ResizeJob{
        Input:  getFixturePath("sample.png"),
        Widths: []uint32{50, 100},
    }
    
    result, err := client.Execute(context.Background(), job)
    // Verificar resultado...
}
```

### 2.3 Tests de StreamClient

Verifican el modo streaming (proceso persistente):

```go
func TestStreamClientExecute(t *testing.T) {
    client, err := NewStreamClient(binaryPath)
    if err != nil {
        t.Fatalf("NewStreamClient failed: %v", err)
    }
    defer client.Close()  // ¡Siempre cerrar!
    
    result, err := client.Execute(job)
    // Verificar resultado...
}
```

**Tests específicos de StreamClient:**
- `TestNewStreamClient`: Inicialización correcta
- `TestStreamClientExecute`: Ejecutar un job
- `TestStreamClientMultipleJobs`: Múltiples jobs en secuencia

---

## 3. Helpers

### `getBinaryPath()`

Encuentra la ruta al binario `dpf` automáticamente:

```go
func getBinaryPath() string {
    // Usa runtime.Caller para obtener el directorio del test
    _, filename, _, _ := runtime.Caller(0)
    dir := filepath.Dir(filename)
    
    // Busca en debug primero, luego en release
    binaryPath := filepath.Join(dir, "..", "..", "dpf", "target", "debug", "dpf")
    if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
        binaryPath = filepath.Join(dir, "..", "..", "dpf", "target", "release", "dpf")
    }
    
    return binaryPath
}
```

**Lógica:**
1. Obtiene el directorio del archivo de test actual
2. Navega hacia arriba para encontrar el proyecto
3. Prioriza `debug/`, fallback a `release/`

### `getFixturePath(name)`

Retorna la ruta completa a un archivo de fixture:

```go
func getFixturePath(name string) string {
    return filepath.Join(getFixturesDir(), name)
}

// Uso:
path := getFixturePath("sample.png")
// → /ruta/al/proyecto/dpf/test_fixtures/sample.png
```

### `setupBinary(t)`

Verifica que el binario exista o hace skip:

```go
func setupBinary(t *testing.T) string {
    binaryPath := getBinaryPath()
    
    if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
        t.Skip("Binary not found. Run 'cargo build' in dpf/ directory first")
    }
    
    return binaryPath
}
```

**Importante:** Usar `t.Skip()` permite que los tests unitarios pasen incluso sin el binario.

---

## 4. Métodos Testeados

### 4.1 `NewClient` y `SetTimeout`

```go
func TestNewClient(t *testing.T) {
    client := NewClient(binaryPath)
    
    if client == nil {
        t.Fatal("NewClient returned nil")
    }
    
    if client.timeout != 30*time.Second {
        t.Errorf("Expected timeout 30s, got %v", client.timeout)
    }
}

func TestClientSetTimeout(t *testing.T) {
    client := NewClient(binaryPath)
    client.SetTimeout(60 * time.Second)
    
    if client.timeout != 60*time.Second {
        t.Errorf("Expected timeout 60s, got %v", client.timeout)
    }
}
```

### 4.2 `Execute` con Diferentes Jobs

**Resize:**
```go
func TestExecuteResize(t *testing.T) {
    job := &ResizeJob{
        Operation: "resize",
        Input:     getFixturePath("sample.png"),
        OutputDir: tempDir,
        Widths:    []uint32{50, 100},
    }
    
    result, err := client.Execute(ctx, job)
    // Verificar: success, outputs count, files existen
}
```

**Convert:**
```go
func TestExecuteConvert(t *testing.T) {
    job := &ConvertJob{
        Operation: "convert",
        Input:     getFixturePath("sample.png"),
        Output:    outputPath,
        Format:    "jpeg",
    }
    // ...
}
```

**Casos de error:**
- `TestExecuteInvalidJob`: Job con campos faltantes
- `TestExecuteMissingFile`: Input que no existe
- `TestExecuteTimeout`: Timeout muy corto

### 4.3 Métodos de Conveniencia

| Método | Test | Descripción |
|--------|------|-------------|
| `Resize()` | `TestResize` | Resize por anchos |
| `Convert()` | `TestConvert` | Conversión de formato |
| `ResizePercent()` | `TestResizePercent` | Resize por porcentaje |
| `Favicon()` | `TestFavicon` | Generación de favicons |
| `Optimize()` | `TestOptimize` | Optimización con oxipng |

Ejemplo de test de conveniencia:
```go
func TestResize(t *testing.T) {
    result, err := client.Resize(ctx, 
        getFixturePath("sample.png"), 
        tempDir, 
        []uint32{50},
    )
    
    if err != nil {
        t.Fatalf("Resize failed: %v", err)
    }
    
    if !result.Success {
        t.Error("Expected success")
    }
}
```

### 4.4 `StreamClient`

```go
func TestNewStreamClient(t *testing.T) {
    client, err := NewStreamClient(binaryPath)
    if err != nil {
        t.Fatalf("NewStreamClient failed: %v", err)
    }
    defer client.Close()
    
    // Verificar que stdin y cmd estén inicializados
    if client.cmd == nil || client.stdin == nil {
        t.Error("Client not properly initialized")
    }
}

func TestStreamClientMultipleJobs(t *testing.T) {
    client, _ := NewStreamClient(binaryPath)
    defer client.Close()
    
    // Ejecutar múltiples jobs secuencialmente
    for i := 0; i < 3; i++ {
        result, err := client.Execute(job)
        // Verificar cada resultado...
    }
}
```

---

## 5. Ejecutar Tests

### Comando Básico

```bash
cd go-bridge
go test -v
```

**Flags útiles:**
- `-v`: Verbose (muestra output de todos los tests)
- `-run TestNombre`: Ejecutar solo tests que coincidan
- `-count=1`: Deshabilitar cache
- `-timeout 5m`: Timeout total

### Ejecutar Solo Tests Unitarios

```bash
go test -v -run "Test.*JSON|Test.*Unmarshal|TestNewClient|TestSetTimeout"
```

### Ejecutar Solo Tests de Integración

```bash
# Primero asegurar que el binario existe
cd dpf && cargo build --release

cd go-bridge
go test -v -run "TestExecute|TestResize|TestConvert|TestStream"
```

### Evitar Skips en CI

Para forzar ejecución de tests de integración (fallar si no hay binario):

```bash
# Opción 1: Variable de entorno
REQUIRE_BINARY=1 go test -v

# Opción 2: Modificar setupBinary para leer la variable
func setupBinary(t *testing.T) string {
    binaryPath := getBinaryPath()
    
    if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
        if os.Getenv("REQUIRE_BINARY") == "1" {
            t.Fatalf("Binary required but not found at %s", binaryPath)
        }
        t.Skip("Binary not found")
    }
    
    return binaryPath
}
```

---

## 6. Ejemplo: Agregar un Nuevo Test

Supongamos que queremos testear el método `Palette()`:

### Paso 1: Crear la función de test

```go
func TestPalette(t *testing.T) {
    binaryPath := setupBinary(t)  // Requiere binario
    client := NewClient(binaryPath)
    
    tempDir := t.TempDir()  // Directorio temporal automático
    
    ctx := context.Background()
    result, err := client.Palette(ctx, 
        getFixturePath("sample.png"),
        tempDir,
        256,  // maxColors
    )
    
    if err != nil {
        t.Fatalf("Palette failed: %v", err)
    }
    
    if !result.Success {
        t.Error("Expected success")
    }
    
    // Verificar que se generó al menos un archivo
    if len(result.Outputs) == 0 {
        t.Error("Expected at least one output file")
    }
}
```

### Paso 2: Ejecutar el test

```bash
cd go-bridge
go test -v -run TestPalette
```

### Paso 3: Test de caso de error (opcional)

```go
func TestPaletteInvalidColors(t *testing.T) {
    binaryPath := setupBinary(t)
    client := NewClient(binaryPath)
    
    // Probar con 0 colores (inválido)
    result, err := client.Palette(ctx, 
        getFixturePath("sample.png"),
        t.TempDir(),
        0,
    )
    
    // Debería fallar o retornar Success=false
    if err == nil && result != nil && result.Success {
        t.Error("Expected error for invalid maxColors")
    }
}
```

---

## Checklist para Nuevos Tests

- [ ] ¿Usa `setupBinary(t)` si requiere el binario?
- [ ] ¿Usa `t.TempDir()` para directorios de salida?
- [ ] ¿Usa `getFixturePath()` para acceder a fixtures?
- [ ] ¿Tiene `defer client.Close()` para StreamClient?
- [ ] ¿Verifica tanto `err` como `result.Success`?
- [ ] ¿Verifica que los archivos de salida existen?
- [ ] ¿El test es independiente (no depende de otros)?
- [ ] ¿El nombre sigue el patrón `TestAlgo`?

---

## Solución de Problemas

### "Binary not found"

```bash
cd dpf
cargo build --release
# o
cargo build  # para debug
```

### "Fixture not found"

Verificar que exista en `dpf/test_fixtures/`:
```bash
ls dpf/test_fixtures/
```

### Tests fallan intermitentemente

- Verificar que se usa `defer client.Close()` en StreamClient
- Aumentar timeout: `client.SetTimeout(60 * time.Second)`
- Verificar que no hay race conditions (StreamClient es thread-safe)

### Timeout en CI

```bash
go test -v -timeout 10m -run "TestExecute"
```
