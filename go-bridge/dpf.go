// Package dpf provides a Go client for the devpixelforge (dpf) Rust media processor.
//
// Supports two operation modes:
//   - One-shot: executes the Rust binary for each job (simple, stateless)
//   - Streaming: keeps a persistent Rust process and sends jobs via stdin
//     (faster for multiple operations, avoids spawn overhead)
//
// Author: Ing. Gustavo Gutiérrez
package dpf

import (
	"bufio"
	"bytes"
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
	Operation    string   `json:"operation"`
	Input        string   `json:"input"`
	OutputDir    string   `json:"output_dir"`
	Widths       []uint32 `json:"widths,omitempty"`
	ScalePercent *float32 `json:"scale_percent,omitempty"`
	MaxHeight    *uint32  `json:"max_height,omitempty"`
	Format       *string  `json:"format,omitempty"`
	Quality      *uint8   `json:"quality,omitempty"`
	Filter       *string  `json:"filter,omitempty"`
	LinearRGB    bool     `json:"linear_rgb,omitempty"`
	Inline       bool     `json:"inline,omitempty"`
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

// PaletteJob reduce la paleta de colores (útil para PNG/GIF).
type PaletteJob struct {
	Operation string   `json:"operation"`
	Input     string   `json:"input"`
	OutputDir string   `json:"output_dir"`
	MaxColors *uint32  `json:"max_colors,omitempty"`
	Dithering *float32 `json:"dithering,omitempty"`
	Format    *string  `json:"format,omitempty"`
	Inline    bool     `json:"inline,omitempty"`
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
	// Ruta al binario dpf (devpixelforge)
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
	var stdout bytes.Buffer
	var stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	err = cmd.Run()
	if err != nil {
		if result, parseErr := parseJobResult(stdout.Bytes()); parseErr == nil {
			return result, nil
		}
		if exitErr, ok := err.(*exec.ExitError); ok {
			message := stderr.String()
			if message == "" {
				message = exitErr.Error()
			}
			return nil, fmt.Errorf("imgproc: process failed: %s", message)
		}
		return nil, fmt.Errorf("imgproc: execution failed: %w", err)
	}

	return parseJobResult(stdout.Bytes())
}

func parseJobResult(output []byte) (*JobResult, error) {
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

// ResizePercent redimensiona una imagen por porcentaje (ej: 50.0 para mitad).
func (c *Client) ResizePercent(ctx context.Context, input, outputDir string, percent float32) (*JobResult, error) {
	return c.Execute(ctx, &ResizeJob{
		Operation:    "resize",
		Input:        input,
		OutputDir:    outputDir,
		ScalePercent: &percent,
	})
}

// ResizeLinear redimensiona usando espacio de color lineal (mejor calidad).
func (c *Client) ResizeLinear(ctx context.Context, input, outputDir string, widths []uint32) (*JobResult, error) {
	return c.Execute(ctx, &ResizeJob{
		Operation: "resize",
		Input:     input,
		OutputDir: outputDir,
		Widths:    widths,
		LinearRGB: true,
	})
}

// Palette reduce la paleta de colores de una imagen.
func (c *Client) Palette(ctx context.Context, input, outputDir string, maxColors uint32) (*JobResult, error) {
	return c.Execute(ctx, &PaletteJob{
		Operation: "palette",
		Input:     input,
		OutputDir: outputDir,
		MaxColors: &maxColors,
	})
}

// PaletteWithDithering reduce la paleta con dithering para suavizar bandas.
func (c *Client) PaletteWithDithering(ctx context.Context, input, outputDir string, maxColors uint32, dithering float32) (*JobResult, error) {
	return c.Execute(ctx, &PaletteJob{
		Operation: "palette",
		Input:     input,
		OutputDir: outputDir,
		MaxColors: &maxColors,
		Dithering: &dithering,
	})
}

// ─── Image Suite Operations ────────────────────────────────────────

// Crop performs a crop operation on an image.
func (c *Client) Crop(ctx context.Context, job *CropJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "crop"
	}
	return c.Execute(ctx, job)
}

// Rotate performs rotation and/or flip on an image.
func (c *Client) Rotate(ctx context.Context, job *RotateJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "rotate"
	}
	return c.Execute(ctx, job)
}

// Watermark adds a watermark (text or image) to an image.
func (c *Client) Watermark(ctx context.Context, job *WatermarkJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "watermark"
	}
	return c.Execute(ctx, job)
}

// Adjust applies image adjustments (brightness, contrast, saturation, blur, sharpen).
func (c *Client) Adjust(ctx context.Context, job *AdjustJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "adjust"
	}
	return c.Execute(ctx, job)
}

// AutoQuality optimizes image quality to target file size using binary search.
func (c *Client) AutoQuality(ctx context.Context, job *QualityJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "quality"
	}
	return c.Execute(ctx, job)
}

// Srcset generates responsive image variants for srcset attribute.
func (c *Client) Srcset(ctx context.Context, job *SrcsetJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "srcset"
	}
	return c.Execute(ctx, job)
}

// Exif performs EXIF operations (strip, preserve, extract, auto_orient).
func (c *Client) Exif(ctx context.Context, job *ExifJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "exif"
	}
	// Use ExifOp field, the JSON tag maps to "exif_op"
	return c.Execute(ctx, job)
}

// MarkdownToPDF converts Markdown into PDF using the Rust Typst-backed renderer.
func (c *Client) MarkdownToPDF(ctx context.Context, job *MarkdownToPDFJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "markdown_to_pdf"
	}
	return c.Execute(ctx, job)
}

// ─── Video Operations ──────────────────────────────────────────────

func (c *Client) VideoTranscode(ctx context.Context, job *VideoTranscodeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_transcode"
	}
	return c.Execute(ctx, job)
}

func (c *Client) VideoResize(ctx context.Context, job *VideoResizeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_resize"
	}
	return c.Execute(ctx, job)
}

func (c *Client) VideoTrim(ctx context.Context, job *VideoTrimJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_trim"
	}
	return c.Execute(ctx, job)
}

func (c *Client) VideoThumbnail(ctx context.Context, job *VideoThumbnailJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_thumbnail"
	}
	return c.Execute(ctx, job)
}

func (c *Client) VideoProfile(ctx context.Context, job *VideoProfileJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_profile"
	}
	return c.Execute(ctx, job)
}

func (c *Client) VideoMetadata(ctx context.Context, job *VideoMetadataJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_metadata"
	}
	return c.Execute(ctx, job)
}

// ─── Audio Operations ──────────────────────────────────────────────

func (c *Client) AudioTranscode(ctx context.Context, job *AudioTranscodeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_transcode"
	}
	return c.Execute(ctx, job)
}

func (c *Client) AudioTrim(ctx context.Context, job *AudioTrimJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_trim"
	}
	return c.Execute(ctx, job)
}

func (c *Client) AudioNormalize(ctx context.Context, job *AudioNormalizeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_normalize"
	}
	return c.Execute(ctx, job)
}

func (c *Client) AudioSilenceTrim(ctx context.Context, job *AudioSilenceTrimJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_silence_trim"
	}
	return c.Execute(ctx, job)
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

// ─── StreamClient: Image Suite Operations ─────────────────────────

// Crop performs a crop operation on an image.
func (sc *StreamClient) Crop(job *CropJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "crop"
	}
	return sc.Execute(job)
}

// Rotate performs rotation and/or flip on an image.
func (sc *StreamClient) Rotate(job *RotateJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "rotate"
	}
	return sc.Execute(job)
}

// Watermark adds a watermark (text or image) to an image.
func (sc *StreamClient) Watermark(job *WatermarkJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "watermark"
	}
	return sc.Execute(job)
}

// Adjust applies image adjustments (brightness, contrast, saturation, blur, sharpen).
func (sc *StreamClient) Adjust(job *AdjustJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "adjust"
	}
	return sc.Execute(job)
}

// AutoQuality optimizes image quality to target file size using binary search.
func (sc *StreamClient) AutoQuality(job *QualityJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "quality"
	}
	return sc.Execute(job)
}

// Srcset generates responsive image variants for srcset attribute.
func (sc *StreamClient) Srcset(job *SrcsetJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "srcset"
	}
	return sc.Execute(job)
}

// Exif performs EXIF operations (strip, preserve, extract, auto_orient).
func (sc *StreamClient) Exif(job *ExifJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "exif"
	}
	// Use ExifOp field, the JSON tag maps to "exif_op"
	return sc.Execute(job)
}

// MarkdownToPDF converts Markdown into PDF using the persistent stream client.
func (sc *StreamClient) MarkdownToPDF(job *MarkdownToPDFJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "markdown_to_pdf"
	}
	return sc.Execute(job)
}

// ─── Video Operations ──────────────────────────────────────────────

func (sc *StreamClient) VideoTranscode(job *VideoTranscodeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_transcode"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) VideoResize(job *VideoResizeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_resize"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) VideoTrim(job *VideoTrimJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_trim"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) VideoThumbnail(job *VideoThumbnailJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_thumbnail"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) VideoProfile(job *VideoProfileJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_profile"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) VideoMetadata(job *VideoMetadataJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "video_metadata"
	}
	return sc.Execute(job)
}

// ─── Audio Operations ──────────────────────────────────────────────

func (sc *StreamClient) AudioTranscode(job *AudioTranscodeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_transcode"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) AudioTrim(job *AudioTrimJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_trim"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) AudioNormalize(job *AudioNormalizeJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_normalize"
	}
	return sc.Execute(job)
}

func (sc *StreamClient) AudioSilenceTrim(job *AudioSilenceTrimJob) (*JobResult, error) {
	if job.Operation == "" {
		job.Operation = "audio_silence_trim"
	}
	return sc.Execute(job)
}
