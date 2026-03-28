// Package main muestra cómo integrar el motor de imágenes Rust
// en un servidor MCP de Go para DevForge.
//
// Autor: Ing. Gustavo Gutiérrez
package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"time"

	dpf "github.com/GustavoGutierrez/devpixelforge-bridge"
)

func main() {
	// ─── Opción 1: Cliente one-shot (simple, para pocas operaciones) ───
	fmt.Println("=== One-Shot Mode ===")
	oneShot()

	// ─── Opción 2: Cliente streaming (rápido, para MCP server) ─────────
	fmt.Println("\n=== Streaming Mode ===")
	streaming()
}

func oneShot() {
	// El binario se compila con: cargo build --release
	// y se coloca junto al binario de Go o en PATH
	client := dpf.NewClient("./dpf")
	client.SetTimeout(60 * time.Second)
	ctx := context.Background()

	// ── Ejemplo: Generar responsive images para un hero banner ──
	result, err := client.Resize(ctx, "assets/hero.png", "output/hero", []uint32{
		320, 640, 1024, 1440, 1920,
	})
	if err != nil {
		log.Fatal(err)
	}
	printResult("Responsive Resize", result)

	// ── Ejemplo: Optimizar un lote de imágenes + generar WebP ──
	result, err = client.Optimize(ctx, []string{
		"assets/photo1.jpg",
		"assets/photo2.jpg",
		"assets/photo3.png",
	}, "output/optimized")
	if err != nil {
		log.Fatal(err)
	}
	printResult("Batch Optimize + WebP", result)

	// ── Ejemplo: Generar favicons desde un SVG del logo ──
	result, err = client.Favicon(ctx, "assets/logo.svg", "output/favicons")
	if err != nil {
		log.Fatal(err)
	}
	printResult("Favicon Generation", result)

	// ── Ejemplo: Obtener placeholder LQIP para lazy loading ──
	result, err = client.Placeholder(ctx, "assets/hero.png")
	if err != nil {
		log.Fatal(err)
	}
	printResult("LQIP Placeholder", result)

	// ── Ejemplo: Crop con gravity center ──
	cropJob := &dpf.CropJob{
		Operation: "crop",
		Input:     "assets/photo.jpg",
		Output:    "output/cropped.jpg",
		Gravity:   "center",
		Width:     uint32Ptr(400),
		Height:    uint32Ptr(300),
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}
	result, err = client.Crop(ctx, cropJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("Smart Crop (center)", result)

	// ── Ejemplo: Rotar imagen 90 grados ──
	rotateJob := &dpf.RotateJob{
		Operation: "rotate",
		Input:     "assets/photo.jpg",
		Output:    "output/rotated.jpg",
		Angle:     uint16Ptr(90),
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}
	result, err = client.Rotate(ctx, rotateJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("Rotate 90°", result)

	// ── Ejemplo: Watermark de texto ──
	watermarkJob := &dpf.WatermarkJob{
		Operation: "watermark",
		Input:     "assets/photo.jpg",
		Output:    "output/watermarked.jpg",
		Text:      "© 2026 DevForge",
		Position:  "bottom-right",
		Opacity:   float64Ptr(0.7),
		FontSize:  uint32Ptr(32),
		Color:     "#FFFFFF",
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}
	result, err = client.Watermark(ctx, watermarkJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("Text Watermark", result)

	// ── Ejemplo: Ajuste de brillo y contraste ──
	adjustJob := &dpf.AdjustJob{
		Operation:  "adjust",
		Input:      "assets/photo.jpg",
		Output:     "output/adjusted.jpg",
		Brightness: float64Ptr(0.1),
		Contrast:   float64Ptr(0.15),
		Format:     "jpeg",
		Quality:    uint8Ptr(90),
	}
	result, err = client.Adjust(ctx, adjustJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("Adjust Brightness/Contrast", result)

	// ── Ejemplo: Auto-quality optimization ──
	qualityJob := &dpf.QualityJob{
		Operation:  "quality",
		Input:      "assets/photo.jpg",
		Output:     "output/optimized.jpg",
		TargetSize: 50000, // 50KB target
		Format:     "jpeg",
	}
	result, err = client.AutoQuality(ctx, qualityJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("Auto-Quality (50KB target)", result)

	// ── Ejemplo: Generar srcset responsive ──
	srcsetJob := &dpf.SrcsetJob{
		Operation:    "srcset",
		Input:        "assets/hero.jpg",
		OutputDir:    "output/srcset",
		Widths:       []uint32{320, 640, 960, 1280, 1920},
		Densities:    []float64{1.0, 2.0},
		Format:       "jpeg",
		Quality:      uint8Ptr(85),
		GenerateHTML: true,
		LinearRGB:    true,
	}
	result, err = client.Srcset(ctx, srcsetJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("Responsive Srcset", result)

	// ── Ejemplo: Strip EXIF data ──
	exifJob := &dpf.ExifJob{
		Operation: "exif",
		Input:     "assets/photo.jpg",
		Output:    "output/no-exif.jpg",
		ExifOp:    "strip",
		Mode:      "all",
		Format:    "jpeg",
		Quality:   uint8Ptr(90),
	}
	result, err = client.Exif(ctx, exifJob)
	if err != nil {
		log.Fatal(err)
	}
	printResult("EXIF Strip", result)
}

func streaming() {
	// El StreamClient mantiene el proceso Rust vivo.
	// Ideal para el MCP server que recibe muchas peticiones.
	sc, err := dpf.NewStreamClient("./dpf")
	if err != nil {
		log.Fatal(err)
	}
	defer sc.Close()

	// Simular múltiples peticiones MCP en secuencia rápida
	jobs := []any{
		&dpf.ResizeJob{
			Operation: "resize",
			Input:     "assets/card.png",
			OutputDir: "output/cards",
			Widths:    []uint32{200, 400, 800},
		},
		&dpf.ConvertJob{
			Operation: "convert",
			Input:     "assets/icon.svg",
			Output:    "output/icon.webp",
			Format:    "webp",
		},
		&dpf.PlaceholderJob{
			Operation: "placeholder",
			Input:     "assets/banner.jpg",
			Kind:      strPtr("css_gradient"),
			Inline:    true,
		},
		&dpf.PlaceholderJob{
			Operation: "placeholder",
			Input:     "assets/banner.jpg",
			Kind:      strPtr("dominant_color"),
			Inline:    true,
		},
	}

	for i, job := range jobs {
		start := time.Now()
		result, err := sc.Execute(job)
		elapsed := time.Since(start)

		if err != nil {
			log.Printf("Job %d failed: %v", i, err)
			continue
		}
		fmt.Printf("Job %d (%s): %dms (total with IPC: %s)\n",
			i, result.Operation, result.ElapsedMs, elapsed)

		if result.Metadata != nil {
			fmt.Printf("  Metadata: %s\n", string(*result.Metadata))
		}
	}
}

// ── Ejemplo de integración con el MCP server real ────────────────

/*
// En tu handler MCP de Go, usarías algo así:

func (s *MCPServer) handleOptimizeImages(params json.RawMessage) (any, error) {
    var req struct {
        Paths     []string `json:"paths"`
        OutputDir string   `json:"output_dir"`
        AlsoWebp  bool     `json:"also_webp"`
    }
    if err := json.Unmarshal(params, &req); err != nil {
        return nil, err
    }

    // Usar el StreamClient que se inició con el MCP server
    result, err := s.imgClient.Execute(&dpf.OptimizeJob{
        Operation: "optimize",
        Inputs:    req.Paths,
        OutputDir: &req.OutputDir,
        AlsoWebp:  req.AlsoWebp,
    })
    if err != nil {
        return nil, fmt.Errorf("image optimization failed: %w", err)
    }

    return result, nil
}

func (s *MCPServer) handleGenerateUI(params json.RawMessage) (any, error) {
    var req struct {
        SVGPath   string   `json:"svg_path"`
        OutputDir string   `json:"output_dir"`
        Sizes     []uint32 `json:"sizes"`
    }
    if err := json.Unmarshal(params, &req); err != nil {
        return nil, err
    }

    // Generar responsive images + favicon + placeholder en un batch
    result, err := s.imgClient.Execute(&dpf.BatchJob{
        Operation: "batch",
        Jobs: []any{
            dpf.ResizeJob{
                Operation: "resize",
                Input:     req.SVGPath,
                OutputDir: req.OutputDir + "/responsive",
                Widths:    req.Sizes,
            },
            dpf.FaviconJob{
                Operation:   "favicon",
                Input:       req.SVGPath,
                OutputDir:   req.OutputDir + "/favicons",
                GenerateICO: true,
            },
            dpf.PlaceholderJob{
                Operation: "placeholder",
                Input:     req.SVGPath,
                Inline:    true,
            },
        },
    })
    if err != nil {
        return nil, err
    }

    return result, nil
}
*/

// ─── Helpers ─────────────────────────────────────────────────────

func printResult(label string, result *dpf.JobResult) {
	fmt.Printf("\n%s (took %dms):\n", label, result.ElapsedMs)
	fmt.Printf("  Success: %v\n", result.Success)
	fmt.Printf("  Outputs: %d files\n", len(result.Outputs))
	for _, out := range result.Outputs {
		fmt.Printf("    - %s (%s, %dx%d, %d bytes)\n",
			out.Path, out.Format, out.Width, out.Height, out.SizeBytes)
	}
	if result.Metadata != nil {
		var meta map[string]any
		json.Unmarshal(*result.Metadata, &meta)
		prettyMeta, _ := json.MarshalIndent(meta, "    ", "  ")
		fmt.Printf("  Metadata: %s\n", string(prettyMeta))
	}
}

func strPtr(s string) *string       { return &s }
func uint32Ptr(v uint32) *uint32    { return &v }
func uint16Ptr(v uint16) *uint16    { return &v }
func uint8Ptr(v uint8) *uint8       { return &v }
func float64Ptr(v float64) *float64 { return &v }
