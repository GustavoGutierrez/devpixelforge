package dpf

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

// getBinaryPath retorna la ruta al binario dpf para tests
func getBinaryPath() string {
	// Buscar en el directorio del proyecto
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)

	// Subir al directorio raíz del proyecto y buscar el binario
	binaryPath := filepath.Join(dir, "..", "..", "dpf", "target", "debug", "dpf")

	// Si no existe en debug, intentar con release
	if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
		binaryPath = filepath.Join(dir, "..", "..", "dpf", "target", "release", "dpf")
	}

	return binaryPath
}

// getFixturesDir retorna la ruta al directorio de fixtures
func getFixturesDir() string {
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)
	return filepath.Join(dir, "..", "..", "dpf", "test_fixtures")
}

// getFixturePath retorna la ruta a un archivo de fixture
func getFixturePath(name string) string {
	return filepath.Join(getFixturesDir(), name)
}

// setupBinary compila el binario si es necesario
func setupBinary(t *testing.T) string {
	binaryPath := getBinaryPath()

	// Verificar si el binario existe
	if _, err := os.Stat(binaryPath); os.IsNotExist(err) {
		t.Skip("Binary not found. Run 'cargo build' in dpf/ directory first")
	}

	return binaryPath
}

// ============================================================================
// Tests de NewClient
// ============================================================================

func TestNewClient(t *testing.T) {
	binaryPath := getBinaryPath()
	client := NewClient(binaryPath)

	if client == nil {
		t.Fatal("NewClient returned nil")
	}

	if client.binaryPath != binaryPath {
		t.Errorf("Expected binaryPath %s, got %s", binaryPath, client.binaryPath)
	}

	if client.timeout != 30*time.Second {
		t.Errorf("Expected timeout 30s, got %v", client.timeout)
	}
}

func TestClientSetTimeout(t *testing.T) {
	binaryPath := getBinaryPath()
	client := NewClient(binaryPath)

	newTimeout := 60 * time.Second
	client.SetTimeout(newTimeout)

	if client.timeout != newTimeout {
		t.Errorf("Expected timeout %v, got %v", newTimeout, client.timeout)
	}
}

// ============================================================================
// Tests de Execute
// ============================================================================

func TestExecuteResize(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	// Crear directorio temporal para salida
	tempDir := t.TempDir()

	job := &ResizeJob{
		Operation: "resize",
		Input:     getFixturePath("sample.png"),
		OutputDir: tempDir,
		Widths:    []uint32{50, 100},
		Format:    strPtr("png"),
		Quality:   uint8Ptr(85),
	}

	ctx := context.Background()
	result, err := client.Execute(ctx, job)

	if err != nil {
		t.Fatalf("Execute failed: %v", err)
	}

	if result == nil {
		t.Fatal("Result is nil")
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if result.Operation != "resize" {
		t.Errorf("Expected operation 'resize', got '%s'", result.Operation)
	}

	if len(result.Outputs) != 2 {
		t.Errorf("Expected 2 outputs, got %d", len(result.Outputs))
	}

	// Verificar que los archivos existen
	for _, output := range result.Outputs {
		if _, err := os.Stat(output.Path); os.IsNotExist(err) {
			t.Errorf("Output file not found: %s", output.Path)
		}
	}
}

func TestExecuteConvert(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "output.jpg")

	job := &ConvertJob{
		Operation: "convert",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}

	ctx := context.Background()
	result, err := client.Execute(ctx, job)

	if err != nil {
		t.Fatalf("Execute failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if _, err := os.Stat(outputPath); os.IsNotExist(err) {
		t.Errorf("Output file not found: %s", outputPath)
	}
}

func TestExecuteInvalidJob(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	// Job inválido - falta input
	job := &ResizeJob{
		Operation: "resize",
		OutputDir: "/tmp",
		Widths:    []uint32{100},
	}

	ctx := context.Background()
	result, err := client.Execute(ctx, job)

	// Debe retornar error o un resultado con Success = false
	if err == nil && result != nil && result.Success {
		t.Error("Expected error or failure for invalid job")
	}
}

func TestExecuteMissingFile(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	job := &ResizeJob{
		Operation: "resize",
		Input:     "/nonexistent/file.png",
		OutputDir: "/tmp",
		Widths:    []uint32{100},
	}

	ctx := context.Background()
	_, err := client.Execute(ctx, job)

	if err == nil {
		t.Error("Expected error for missing file")
	}
}

func TestExecuteTimeout(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)
	client.SetTimeout(1 * time.Millisecond) // Timeout muy corto

	job := &ResizeJob{
		Operation: "resize",
		Input:     getFixturePath("sample.png"),
		OutputDir: t.TempDir(),
		Widths:    []uint32{50},
	}

	ctx := context.Background()
	_, err := client.Execute(ctx, job)

	// Debe retornar error por timeout
	if err == nil {
		t.Error("Expected timeout error")
	}
}

// ============================================================================
// Tests de métodos de conveniencia
// ============================================================================

func TestResize(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()

	ctx := context.Background()
	result, err := client.Resize(ctx, getFixturePath("sample.png"), tempDir, []uint32{50})

	if err != nil {
		t.Fatalf("Resize failed: %v", err)
	}

	if !result.Success {
		t.Error("Expected success")
	}
}

func TestConvert(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "output.webp")

	ctx := context.Background()
	result, err := client.Convert(ctx, getFixturePath("sample.png"), outputPath, "webp")

	if err != nil {
		t.Fatalf("Convert failed: %v", err)
	}

	if !result.Success {
		t.Error("Expected success")
	}
}

func TestResizePercent(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()

	ctx := context.Background()
	result, err := client.ResizePercent(ctx, getFixturePath("sample.png"), tempDir, 50.0)

	if err != nil {
		t.Fatalf("ResizePercent failed: %v", err)
	}

	if !result.Success {
		t.Error("Expected success")
	}

	// Verificar que el resultado tiene el tamaño correcto
	if len(result.Outputs) > 0 {
		output := result.Outputs[0]
		if output.Width != 50 { // 100 * 0.5 = 50
			t.Errorf("Expected width 50, got %d", output.Width)
		}
	}
}

func TestFavicon(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()

	ctx := context.Background()
	result, err := client.Favicon(ctx, getFixturePath("sample.png"), tempDir)

	if err != nil {
		t.Fatalf("Favicon failed: %v", err)
	}

	if !result.Success {
		t.Error("Expected success")
	}
}

func TestOptimize(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()

	ctx := context.Background()
	result, err := client.Optimize(ctx, []string{getFixturePath("sample.png")}, tempDir)

	// Puede fallar si oxipng no está disponible
	if err != nil {
		t.Skipf("Optimize not available: %v", err)
	}

	if !result.Success {
		t.Logf("Optimize returned failure: %v", result)
	}
}

// ============================================================================
// Tests de StreamClient
// ============================================================================

func TestNewStreamClient(t *testing.T) {
	binaryPath := setupBinary(t)

	client, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer client.Close()

	if client.cmd == nil {
		t.Error("StreamClient.cmd is nil")
	}

	if client.stdin == nil {
		t.Error("StreamClient.stdin is nil")
	}
}

func TestStreamClientExecute(t *testing.T) {
	binaryPath := setupBinary(t)

	client, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer client.Close()

	tempDir := t.TempDir()

	job := &ResizeJob{
		Operation: "resize",
		Input:     getFixturePath("sample.png"),
		OutputDir: tempDir,
		Widths:    []uint32{50},
	}

	result, err := client.Execute(job)

	if err != nil {
		t.Fatalf("StreamClient.Execute failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

func TestStreamClientMultipleJobs(t *testing.T) {
	binaryPath := setupBinary(t)

	client, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer client.Close()

	tempDir := t.TempDir()

	// Ejecutar múltiples jobs
	for i := 0; i < 3; i++ {
		job := &ResizeJob{
			Operation: "resize",
			Input:     getFixturePath("sample.png"),
			OutputDir: tempDir,
			Widths:    []uint32{50},
		}

		result, err := client.Execute(job)
		if err != nil {
			t.Fatalf("StreamClient.Execute failed on iteration %d: %v", i, err)
		}

		if !result.Success {
			t.Errorf("Expected success on iteration %d", i)
		}
	}
}

// ============================================================================
// Tests de serialización JSON
// ============================================================================

func TestResizeJobJSON(t *testing.T) {
	job := &ResizeJob{
		Operation: "resize",
		Input:     "test.png",
		OutputDir: "/tmp",
		Widths:    []uint32{100, 200},
		Format:    strPtr("webp"),
		Quality:   uint8Ptr(90),
	}

	data, err := json.Marshal(job)
	if err != nil {
		t.Fatalf("JSON marshal failed: %v", err)
	}

	// Verificar que contiene los campos esperados
	jsonStr := string(data)
	if !contains(jsonStr, "operation") {
		t.Error("JSON missing 'operation' field")
	}
	if !contains(jsonStr, "resize") {
		t.Error("JSON missing operation value")
	}
	if !contains(jsonStr, "widths") {
		t.Error("JSON missing 'widths' field")
	}
}

func TestJobResultUnmarshal(t *testing.T) {
	jsonData := `{
		"success": true,
		"operation": "resize",
		"outputs": [
			{
				"path": "/tmp/test_50w.png",
				"format": "png",
				"width": 50,
				"height": 50,
				"size_bytes": 1024
			}
		],
		"elapsed_ms": 150
	}`

	var result JobResult
	if err := json.Unmarshal([]byte(jsonData), &result); err != nil {
		t.Fatalf("JSON unmarshal failed: %v", err)
	}

	if !result.Success {
		t.Error("Expected Success to be true")
	}

	if result.Operation != "resize" {
		t.Errorf("Expected operation 'resize', got '%s'", result.Operation)
	}

	if len(result.Outputs) != 1 {
		t.Errorf("Expected 1 output, got %d", len(result.Outputs))
	}

	if result.ElapsedMs != 150 {
		t.Errorf("Expected elapsed_ms 150, got %d", result.ElapsedMs)
	}
}

// ============================================================================
// Helpers
// ============================================================================

func strPtr(s string) *string {
	return &s
}

func uint8Ptr(u uint8) *uint8 {
	return &u
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsInternal(s, substr))
}

func containsInternal(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

// ============================================================================
// Tests de Crop Operation
// ============================================================================

func TestCropCenter(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "crop_center.jpg")

	job := &CropJob{
		Operation: "crop",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Gravity:   "center",
		Width:     uint32Ptr(100),
		Height:    uint32Ptr(100),
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}

	ctx := context.Background()
	result, err := client.Crop(ctx, job)

	if err != nil {
		t.Fatalf("Crop failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if _, err := os.Stat(outputPath); os.IsNotExist(err) {
		t.Errorf("Output file not found: %s", outputPath)
	}
}

func TestCropManual(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "crop_manual.jpg")

	job := &CropJob{
		Operation: "crop",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Rect: &CropRect{
			X:      10,
			Y:      10,
			Width:  100,
			Height: 100,
		},
		Format:  "png",
		Quality: uint8Ptr(85),
	}

	ctx := context.Background()
	result, err := client.Crop(ctx, job)

	if err != nil {
		t.Fatalf("Crop failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

func TestCropFocalPoint(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "crop_focal.jpg")

	job := &CropJob{
		Operation: "crop",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Gravity:   "focal_point",
		FocalX:    float64Ptr(0.75),
		FocalY:    float64Ptr(0.25),
		Width:     uint32Ptr(200),
		Height:    uint32Ptr(150),
		Format:    "png",
	}

	ctx := context.Background()
	result, err := client.Crop(ctx, job)

	if err != nil {
		t.Fatalf("Crop failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

// ============================================================================
// Tests de Rotate Operation
// ============================================================================

func TestRotate90(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "rotate_90.jpg")

	job := &RotateJob{
		Operation: "rotate",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Angle:     uint16Ptr(90),
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}

	ctx := context.Background()
	result, err := client.Rotate(ctx, job)

	if err != nil {
		t.Fatalf("Rotate failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if _, err := os.Stat(outputPath); os.IsNotExist(err) {
		t.Errorf("Output file not found: %s", outputPath)
	}
}

func TestRotateFlipHorizontal(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "flip_h.jpg")

	job := &RotateJob{
		Operation: "rotate",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Flip:      "horizontal",
		Format:    "png",
	}

	ctx := context.Background()
	result, err := client.Rotate(ctx, job)

	if err != nil {
		t.Fatalf("Rotate failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

func TestRotateAutoOrient(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "auto_orient.jpg")

	job := &RotateJob{
		Operation:  "rotate",
		Input:      getFixturePath("sample.png"),
		Output:     outputPath,
		AutoOrient: true,
		Format:     "png",
	}

	ctx := context.Background()
	result, err := client.Rotate(ctx, job)

	if err != nil {
		t.Fatalf("Rotate failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

// ============================================================================
// Tests de Watermark Operation
// ============================================================================

func TestWatermarkText(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "watermark_text.jpg")

	job := &WatermarkJob{
		Operation: "watermark",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Text:      "© Test",
		Position:  "bottom-right",
		Opacity:   float64Ptr(0.8),
		FontSize:  uint32Ptr(24),
		Color:     "#FFFFFF",
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}

	ctx := context.Background()
	result, err := client.Watermark(ctx, job)

	if err != nil {
		t.Fatalf("Watermark failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if _, err := os.Stat(outputPath); os.IsNotExist(err) {
		t.Errorf("Output file not found: %s", outputPath)
	}
}

// ============================================================================
// Tests de Adjust Operation
// ============================================================================

func TestAdjustBrightnessContrast(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "adjusted.jpg")

	job := &AdjustJob{
		Operation:  "adjust",
		Input:      getFixturePath("sample.png"),
		Output:     outputPath,
		Brightness: float64Ptr(0.1),
		Contrast:   float64Ptr(0.15),
		Format:     "jpeg",
		Quality:    uint8Ptr(90),
	}

	ctx := context.Background()
	result, err := client.Adjust(ctx, job)

	if err != nil {
		t.Fatalf("Adjust failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if _, err := os.Stat(outputPath); os.IsNotExist(err) {
		t.Errorf("Output file not found: %s", outputPath)
	}
}

func TestAdjustBlurSharpen(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "blur_sharpen.jpg")

	job := &AdjustJob{
		Operation: "adjust",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Blur:      float64Ptr(1.0),
		Sharpen:   float64Ptr(0.5),
		Format:    "png",
	}

	ctx := context.Background()
	result, err := client.Adjust(ctx, job)

	if err != nil {
		t.Fatalf("Adjust failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

// ============================================================================
// Tests de Srcset Operation
// ============================================================================

func TestSrcset(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()

	job := &SrcsetJob{
		Operation:    "srcset",
		Input:        getFixturePath("sample.png"),
		OutputDir:    tempDir,
		Widths:       []uint32{320, 640, 960},
		Densities:    []float64{1.0, 2.0},
		Format:       "jpeg",
		Quality:      uint8Ptr(85),
		GenerateHTML: true,
		LinearRGB:    true,
	}

	ctx := context.Background()
	result, err := client.Srcset(ctx, job)

	if err != nil {
		t.Fatalf("Srcset failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}

	if len(result.Outputs) < 3 {
		t.Errorf("Expected at least 3 outputs, got %d", len(result.Outputs))
	}
}

// ============================================================================
// Tests de StreamClient con nuevas operaciones
// ============================================================================

func TestStreamClientCrop(t *testing.T) {
	binaryPath := setupBinary(t)

	sc, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer sc.Close()

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "stream_crop.jpg")

	job := &CropJob{
		Operation: "crop",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Gravity:   "center",
		Width:     uint32Ptr(50),
		Height:    uint32Ptr(50),
		Format:    "png",
	}

	result, err := sc.Crop(job)

	if err != nil {
		t.Fatalf("StreamClient.Crop failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

func TestStreamClientRotate(t *testing.T) {
	binaryPath := setupBinary(t)

	sc, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer sc.Close()

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "stream_rotate.jpg")

	job := &RotateJob{
		Operation: "rotate",
		Input:     getFixturePath("sample.png"),
		Output:    outputPath,
		Angle:     uint16Ptr(180),
		Format:    "png",
	}

	result, err := sc.Rotate(job)

	if err != nil {
		t.Fatalf("StreamClient.Rotate failed: %v", err)
	}

	if !result.Success {
		t.Errorf("Expected success, got failure")
	}
}

func TestStreamClientMultipleNewOperations(t *testing.T) {
	binaryPath := setupBinary(t)

	sc, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer sc.Close()

	tempDir := t.TempDir()

	// Crop
	cropJob := &CropJob{
		Operation: "crop",
		Input:     getFixturePath("sample.png"),
		Output:    filepath.Join(tempDir, "sc_crop.jpg"),
		Gravity:   "center",
		Width:     uint32Ptr(50),
		Height:    uint32Ptr(50),
		Format:    "png",
	}
	result, err := sc.Crop(cropJob)
	if err != nil {
		t.Fatalf("StreamClient.Crop failed: %v", err)
	}
	if !result.Success {
		t.Errorf("Crop: Expected success")
	}

	// Rotate
	rotateJob := &RotateJob{
		Operation: "rotate",
		Input:     getFixturePath("sample.png"),
		Output:    filepath.Join(tempDir, "sc_rotate.jpg"),
		Angle:     uint16Ptr(90),
		Format:    "png",
	}
	result, err = sc.Rotate(rotateJob)
	if err != nil {
		t.Fatalf("StreamClient.Rotate failed: %v", err)
	}
	if !result.Success {
		t.Errorf("Rotate: Expected success")
	}

	// Adjust
	adjustJob := &AdjustJob{
		Operation:  "adjust",
		Input:      getFixturePath("sample.png"),
		Output:     filepath.Join(tempDir, "sc_adjust.jpg"),
		Brightness: float64Ptr(0.1),
		Format:     "png",
	}
	result, err = sc.Adjust(adjustJob)
	if err != nil {
		t.Fatalf("StreamClient.Adjust failed: %v", err)
	}
	if !result.Success {
		t.Errorf("Adjust: Expected success")
	}
}

// ============================================================================
// Helper functions for new types
// ============================================================================

func uint32Ptr(v uint32) *uint32 {
	return &v
}

func uint16Ptr(v uint16) *uint16 {
	return &v
}

func float64Ptr(v float64) *float64 {
	return &v
}
