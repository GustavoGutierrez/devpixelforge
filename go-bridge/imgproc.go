// Package imgproc proporciona un cliente Go para el motor de procesamiento
// de imágenes en Rust (devforge-imgproc).
//
// Soporta dos modos de operación:
//   - One-shot: ejecuta el binario Rust por cada trabajo (simple, sin estado)
//   - Streaming: mantiene un proceso Rust persistente y envía trabajos por stdin
//     (más rápido para múltiples operaciones, evita el overhead de spawn)
//
// Autor: Ing. Gustavo Gutiérrez
package imgproc

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os/exec"
	"sync"
	"time"
)

// ─── Job Types (espejo del protocolo Rust) ───────────────────────

// ResizeJob define una operación de resize.
type ResizeJob struct {
	Operation string   `json:"operation"`
	Input     string   `json:"input"`
	OutputDir string   `json:"output_dir"`
	Widths    []uint32 `json:"widths"`
	MaxHeight *uint32  `json:"max_height,omitempty"`
	Format    *string  `json:"format,omitempty"`
	Quality   *uint8   `json:"quality,omitempty"`
	Filter    *string  `json:"filter,omitempty"`
	Inline    bool     `json:"inline,omitempty"`
}

// OptimizeJob define una operación de optimización.
type OptimizeJob struct {
	Operation string   `json:"operation"`
	Inputs    []string `json:"inputs"`
	OutputDir *string  `json:"output_dir,omitempty"`
	Level     *string  `json:"level,omitempty"`
	Quality   *uint8   `json:"quality,omitempty"`
	AlsoWebp  bool     `json:"also_webp,omitempty"`
}

// ConvertJob define una conversión de formato.
type ConvertJob struct {
	Operation string  `json:"operation"`
	Input     string  `json:"input"`
	Output    string  `json:"output"`
	Format    string  `json:"format"`
	Quality   *uint8  `json:"quality,omitempty"`
	Width     *uint32 `json:"width,omitempty"`
	Height    *uint32 `json:"height,omitempty"`
	Inline    bool    `json:"inline,omitempty"`
}

// FaviconJob genera favicons multi-tamaño.
type FaviconJob struct {
	Operation        string   `json:"operation"`
	Input            string   `json:"input"`
	OutputDir        string   `json:"output_dir"`
	Sizes            []uint32 `json:"sizes,omitempty"`
	GenerateICO      bool     `json:"generate_ico"`
	GenerateManifest bool     `json:"generate_manifest,omitempty"`
	Prefix           *string  `json:"prefix,omitempty"`
}

// SpriteJob genera un sprite sheet.
type SpriteJob struct {
	Operation   string   `json:"operation"`
	Inputs      []string `json:"inputs"`
	Output      string   `json:"output"`
	CellSize    *uint32  `json:"cell_size,omitempty"`
	Columns     *uint32  `json:"columns,omitempty"`
	Padding     *uint32  `json:"padding,omitempty"`
	GenerateCSS bool     `json:"generate_css,omitempty"`
}

// PlaceholderJob genera placeholders (LQIP, color dominante, gradiente CSS).
type PlaceholderJob struct {
	Operation string  `json:"operation"`
	Input     string  `json:"input"`
	Output    *string `json:"output,omitempty"`
	Kind      *string `json:"kind,omitempty"`
	LQIPWidth *uint32 `json:"lqip_width,omitempty"`
	Inline    bool    `json:"inline"`
}

// ─── Response Types ──────────────────────────────────────────────

// JobResult es la respuesta del motor Rust.
type JobResult struct {
	Success   bool             `json:"success"`
	Operation string           `json:"operation"`
	Outputs   []OutputFile     `json:"outputs"`
	ElapsedMs uint64           `json:"elapsed_ms"`
	Metadata  *json.RawMessage `json:"metadata,omitempty"`
}

// OutputFile describe un archivo producido.
type OutputFile struct {
	Path       string  `json:"path"`
	Format     string  `json:"format"`
	Width      uint32  `json:"width"`
	Height     uint32  `json:"height"`
	SizeBytes  uint64  `json:"size_bytes"`
	DataBase64 *string `json:"data_base64,omitempty"`
}

// ─── Client ──────────────────────────────────────────────────────

// Client es el cliente principal para el motor de imágenes Rust.
type Client struct {
	// Ruta al binario devforge-imgproc
	binaryPath string
	// Timeout por defecto para operaciones
	timeout time.Duration
}

// NewClient crea un nuevo cliente.
// binaryPath es la ruta al binario compilado de Rust.
func NewClient(binaryPath string) *Client {
	return &Client{
		binaryPath: binaryPath,
		timeout:    30 * time.Second,
	}
}

// SetTimeout configura el timeout para operaciones.
func (c *Client) SetTimeout(d time.Duration) {
	c.timeout = d
}

// Execute ejecuta un trabajo y devuelve el resultado.
// El job puede ser cualquiera de los tipos *Job definidos arriba.
func (c *Client) Execute(ctx context.Context, job any) (*JobResult, error) {
	data, err := json.Marshal(job)
	if err != nil {
		return nil, fmt.Errorf("imgproc: failed to marshal job: %w", err)
	}

	ctx, cancel := context.WithTimeout(ctx, c.timeout)
	defer cancel()

	cmd := exec.CommandContext(ctx, c.binaryPath, "process", "--job", string(data))

	output, err := cmd.Output()
	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			return nil, fmt.Errorf("imgproc: process failed: %s", string(exitErr.Stderr))
		}
		return nil, fmt.Errorf("imgproc: execution failed: %w", err)
	}

	var result JobResult
	if err := json.Unmarshal(output, &result); err != nil {
		return nil, fmt.Errorf("imgproc: invalid response: %w", err)
	}

	return &result, nil
}

// ─── Convenience Methods ─────────────────────────────────────────

// Resize redimensiona una imagen a múltiples anchos.
func (c *Client) Resize(ctx context.Context, input, outputDir string, widths []uint32) (*JobResult, error) {
	return c.Execute(ctx, &ResizeJob{
		Operation: "resize",
		Input:     input,
		OutputDir: outputDir,
		Widths:    widths,
	})
}

// Optimize optimiza una o más imágenes.
func (c *Client) Optimize(ctx context.Context, inputs []string, outputDir string) (*JobResult, error) {
	return c.Execute(ctx, &OptimizeJob{
		Operation: "optimize",
		Inputs:    inputs,
		OutputDir: &outputDir,
		AlsoWebp:  true,
	})
}

// Convert convierte una imagen a otro formato.
func (c *Client) Convert(ctx context.Context, input, output, format string) (*JobResult, error) {
	return c.Execute(ctx, &ConvertJob{
		Operation: "convert",
		Input:     input,
		Output:    output,
		Format:    format,
	})
}

// Favicon genera favicons desde una imagen.
func (c *Client) Favicon(ctx context.Context, input, outputDir string) (*JobResult, error) {
	return c.Execute(ctx, &FaviconJob{
		Operation:        "favicon",
		Input:            input,
		OutputDir:        outputDir,
		GenerateICO:      true,
		GenerateManifest: true,
	})
}

// Placeholder genera un placeholder LQIP inline.
func (c *Client) Placeholder(ctx context.Context, input string) (*JobResult, error) {
	return c.Execute(ctx, &PlaceholderJob{
		Operation: "placeholder",
		Input:     input,
		Inline:    true,
	})
}

// ─── Streaming Client (proceso persistente) ──────────────────────

// StreamClient mantiene un proceso Rust vivo para enviar múltiples trabajos
// sin el overhead de crear un nuevo proceso cada vez.
type StreamClient struct {
	cmd    *exec.Cmd
	stdin  io.WriteCloser
	reader *bufio.Reader
	mu     sync.Mutex
}

// NewStreamClient inicia el motor Rust en modo streaming.
func NewStreamClient(binaryPath string) (*StreamClient, error) {
	cmd := exec.Command(binaryPath, "--stream")

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return nil, fmt.Errorf("imgproc: failed to get stdin pipe: %w", err)
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return nil, fmt.Errorf("imgproc: failed to get stdout pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return nil, fmt.Errorf("imgproc: failed to start process: %w", err)
	}

	return &StreamClient{
		cmd:    cmd,
		stdin:  stdin,
		reader: bufio.NewReaderSize(stdout, 4*1024*1024), // 4MB buffer
	}, nil
}

// Execute envía un trabajo al proceso Rust persistente.
// Thread-safe: se puede llamar desde múltiples goroutines.
func (sc *StreamClient) Execute(job any) (*JobResult, error) {
	sc.mu.Lock()
	defer sc.mu.Unlock()

	data, err := json.Marshal(job)
	if err != nil {
		return nil, fmt.Errorf("imgproc: marshal failed: %w", err)
	}

	// Enviar JSON + newline
	if _, err := sc.stdin.Write(append(data, '\n')); err != nil {
		return nil, fmt.Errorf("imgproc: write failed: %w", err)
	}

	// Leer respuesta (una línea JSON)
	line, err := sc.reader.ReadBytes('\n')
	if err != nil {
		return nil, fmt.Errorf("imgproc: read failed: %w", err)
	}

	var result JobResult
	if err := json.Unmarshal(line, &result); err != nil {
		return nil, fmt.Errorf("imgproc: invalid response: %w", err)
	}

	return &result, nil
}

// Close cierra el proceso Rust.
func (sc *StreamClient) Close() error {
	sc.stdin.Close()
	return sc.cmd.Wait()
}
