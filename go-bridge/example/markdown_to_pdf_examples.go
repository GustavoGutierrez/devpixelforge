package main

import (
	"context"
	"encoding/base64"
	"fmt"
	"log"
	"path/filepath"
	"time"

	dpf "github.com/GustavoGutierrez/devpixelforge-bridge"
)

// markdownToPDFExamples shows both MCP-friendly inline conversion and file-based conversion.
func markdownToPDFExamples() {
	client := dpf.NewClient("./dpf")
	client.SetTimeout(60 * time.Second)
	ctx := context.Background()

	// ── Inline PDF with theme customization (v0.1.6+) ──
	inlineMarkdown := "# Inline PDF\n\nGenerated without caller-managed temp files.\n\n![Sample](sample.png)"
	inlineResult, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
		MarkdownText: &inlineMarkdown,
		Inline:       true,
		Theme:        strPtr("professional"),
		ThemeOverride: &dpf.ThemeOverride{
			BodyFontSize: float64Ptr(11.0),
			MarginMM:     float64Ptr(15.0),
		},
		ResourceFiles: map[string]string{
			"sample.png": "../dpf/test_fixtures/sample.png",
		},
	})
	if err != nil {
		log.Printf("inline markdown_to_pdf failed: %v", err)
	} else if len(inlineResult.Outputs) > 0 && inlineResult.Outputs[0].DataBase64 != nil {
		decoded, decodeErr := base64.StdEncoding.DecodeString(*inlineResult.Outputs[0].DataBase64)
		if decodeErr != nil {
			log.Printf("inline PDF decode failed: %v", decodeErr)
		} else {
			fmt.Printf("Inline markdown_to_pdf produced %d PDF bytes (custom theme)\n", len(decoded))
		}
	}

	// ── File-based PDF ──
	outputPath := filepath.Join("output", "report.pdf")
	fileResult, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
		Input:    "assets/report.md",
		Output:   outputPath,
		PageSize: strPtr("letter"),
		Theme:    strPtr("engineering"),
	})
	if err != nil {
		log.Printf("file markdown_to_pdf failed: %v", err)
	} else {
		printResult("Markdown to PDF", fileResult)
	}

	// ── Raw JSON theme_config (alternative to ThemeOverride) ──
	rawResult, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
		MarkdownText: strPtr("# Raw Theme Config\n\nWith explicit JSON overrides."),
		Inline:       true,
		Theme:        strPtr("scientific_article"),
		ThemeConfig:  []byte(`{"body_font_size_pt": 10.5, "code_font_size_pt": 9.0}`),
	})
	if err != nil {
		log.Printf("raw theme_config markdown_to_pdf failed: %v", err)
	} else if inlineResult.Outputs[0].DataBase64 != nil {
		fmt.Printf("Raw theme_config PDF: %d bytes\n", rawResult.Outputs[0].SizeBytes)
	}
}
