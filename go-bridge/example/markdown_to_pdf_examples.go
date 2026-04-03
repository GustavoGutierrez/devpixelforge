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

	inlineMarkdown := "# Inline PDF\n\nGenerated without caller-managed temp files."
	inlineResult, err := client.MarkdownToPDF(ctx, &dpf.MarkdownToPDFJob{
		MarkdownText: &inlineMarkdown,
		Inline:       true,
		Theme:        strPtr("professional"),
	})
	if err != nil {
		log.Printf("inline markdown_to_pdf failed: %v", err)
	} else if len(inlineResult.Outputs) > 0 && inlineResult.Outputs[0].DataBase64 != nil {
		decoded, decodeErr := base64.StdEncoding.DecodeString(*inlineResult.Outputs[0].DataBase64)
		if decodeErr != nil {
			log.Printf("inline PDF decode failed: %v", decodeErr)
		} else {
			fmt.Printf("Inline markdown_to_pdf produced %d PDF bytes\n", len(decoded))
		}
	}

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
}
