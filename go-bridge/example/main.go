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

	imgproc "github.com/GustavoGutierrez/devforge-imgproc-bridge"
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
	client := imgproc.NewClient("./devforge-imgproc")
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
}

func streaming() {
	// El StreamClient mantiene el proceso Rust vivo.
	// Ideal para el MCP server que recibe muchas peticiones.
	sc, err := imgproc.NewStreamClient("./devforge-imgproc")
	if err != nil {
		log.Fatal(err)
	}
	defer sc.Close()

	// Simular múltiples peticiones MCP en secuencia rápida
	jobs := []any{
		&imgproc.ResizeJob{
			Operation: "resize",
			Input:     "assets/card.png",
			OutputDir: "output/cards",
			Widths:    []uint32{200, 400, 800},
		},
		&imgproc.ConvertJob{
			Operation: "convert",
			Input:     "assets/icon.svg",
			Output:    "output/icon.webp",
			Format:    "webp",
		},
		&imgproc.PlaceholderJob{
			Operation: "placeholder",
			Input:     "assets/banner.jpg",
			Kind:      strPtr("css_gradient"),
			Inline:    true,
		},
		&imgproc.PlaceholderJob{
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
    result, err := s.imgClient.Execute(&imgproc.OptimizeJob{
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
    result, err := s.imgClient.Execute(&imgproc.BatchJob{
        Operation: "batch",
        Jobs: []any{
            imgproc.ResizeJob{
                Operation: "resize",
                Input:     req.SVGPath,
                OutputDir: req.OutputDir + "/responsive",
                Widths:    req.Sizes,
            },
            imgproc.FaviconJob{
                Operation:   "favicon",
                Input:       req.SVGPath,
                OutputDir:   req.OutputDir + "/favicons",
                GenerateICO: true,
            },
            imgproc.PlaceholderJob{
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

func printResult(label string, result *imgproc.JobResult) {
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

func strPtr(s string) *string { return &s }
