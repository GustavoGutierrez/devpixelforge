package dpf

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestMarkdownToPDFJobJSON(t *testing.T) {
	job := &MarkdownToPDFJob{
		Operation:    "markdown_to_pdf",
		MarkdownText: strPtr("# JSON contract"),
		Inline:       true,
		Theme:        strPtr("engineering"),
		ResourceFiles: map[string]string{
			"sample.png": "fixtures/sample.png",
		},
	}

	data, err := json.Marshal(job)
	if err != nil {
		t.Fatalf("JSON marshal failed: %v", err)
	}

	jsonStr := string(data)
	if !contains(jsonStr, "markdown_to_pdf") {
		t.Fatal("JSON missing markdown_to_pdf operation")
	}
	if !contains(jsonStr, "markdown_text") {
		t.Fatal("JSON missing markdown_text field")
	}
	if !contains(jsonStr, "inline") {
		t.Fatal("JSON missing inline field")
	}
	if !contains(jsonStr, "resource_files") {
		t.Fatal("JSON missing resource_files field")
	}
}

func TestClientMarkdownToPDFInlineWithResourceFiles(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	result, err := client.MarkdownToPDF(context.Background(), &MarkdownToPDFJob{
		MarkdownText: strPtr("# Inline Assets\n\n![Logo](sample.png)"),
		Inline:       true,
		Theme:        strPtr("informational"),
		ResourceFiles: map[string]string{
			"sample.png": getFixturePath("sample.png"),
		},
	})
	if err != nil {
		t.Fatalf("MarkdownToPDF failed: %v", err)
	}
	if !result.Success {
		t.Fatal("expected success")
	}
	if result.Outputs[0].DataBase64 == nil {
		t.Fatal("expected inline PDF bytes")
	}
}

func TestClientMarkdownToPDFInline(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	result, err := client.MarkdownToPDF(context.Background(), &MarkdownToPDFJob{
		MarkdownText: strPtr("# Inline PDF\n\nGenerated from Go."),
		Inline:       true,
		Theme:        strPtr("professional"),
	})
	if err != nil {
		t.Fatalf("MarkdownToPDF failed: %v", err)
	}
	if !result.Success {
		t.Fatal("expected success")
	}
	if len(result.Outputs) != 1 {
		t.Fatalf("expected one output, got %d", len(result.Outputs))
	}
	if result.Outputs[0].Format != "pdf" {
		t.Fatalf("expected pdf output, got %s", result.Outputs[0].Format)
	}
	if result.Outputs[0].DataBase64 == nil {
		t.Fatal("expected inline PDF bytes")
	}

	decoded, err := base64.StdEncoding.DecodeString(*result.Outputs[0].DataBase64)
	if err != nil {
		t.Fatalf("inline PDF should decode: %v", err)
	}
	if len(decoded) < 4 || string(decoded[:4]) != "%PDF" {
		t.Fatal("decoded inline bytes do not look like a PDF")
	}
	assertMetadataBackend(t, result, "typst")
}

func TestClientMarkdownToPDFFileOutput(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	tempDir := t.TempDir()
	outputPath := filepath.Join(tempDir, "report.pdf")

	result, err := client.MarkdownToPDF(context.Background(), &MarkdownToPDFJob{
		Input:    getFixturePath("sample.md"),
		Output:   outputPath,
		PageSize: strPtr("letter"),
	})
	if err != nil {
		t.Fatalf("MarkdownToPDF failed: %v", err)
	}
	if !result.Success {
		t.Fatal("expected success")
	}
	if _, err := os.Stat(outputPath); err != nil {
		t.Fatalf("expected PDF output file: %v", err)
	}
	if result.Outputs[0].Format != "pdf" {
		t.Fatalf("expected pdf output, got %s", result.Outputs[0].Format)
	}
	assertMetadataBackend(t, result, "typst")
}

func TestStreamClientMarkdownToPDFInline(t *testing.T) {
	binaryPath := setupBinary(t)
	client, err := NewStreamClient(binaryPath)
	if err != nil {
		t.Fatalf("NewStreamClient failed: %v", err)
	}
	defer client.Close()

	result, err := client.MarkdownToPDF(&MarkdownToPDFJob{
		MarkdownText: strPtr("# Stream PDF\n\nGenerated from StreamClient."),
		Inline:       true,
	})
	if err != nil {
		t.Fatalf("Stream MarkdownToPDF failed: %v", err)
	}
	if !result.Success {
		t.Fatal("expected success")
	}
	if result.Outputs[0].DataBase64 == nil {
		t.Fatal("expected inline PDF bytes")
	}
}

func TestClientMarkdownToPDFValidationFailure(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	result, err := client.MarkdownToPDF(context.Background(), &MarkdownToPDFJob{
		MarkdownText:   strPtr("# Invalid"),
		MarkdownBase64: strPtr("IyBJbnZhbGlk"),
		Inline:         true,
	})
	if err != nil {
		t.Fatalf("expected structured failure result, got error: %v", err)
	}
	if result.Success {
		t.Fatal("expected failure result")
	}
	if result.Operation != "markdown_to_pdf" {
		t.Fatalf("expected markdown_to_pdf operation, got %s", result.Operation)
	}
}

func assertMetadataBackend(t *testing.T, result *JobResult, expected string) {
	t.Helper()
	if result.Metadata == nil {
		t.Fatal("expected metadata")
	}

	var payload map[string]any
	if err := json.Unmarshal(*result.Metadata, &payload); err != nil {
		t.Fatalf("metadata should decode: %v", err)
	}

	if payload["backend"] != expected {
		t.Fatalf("expected backend %q, got %#v", expected, payload["backend"])
	}
}

func TestThemeOverrideSerialization(t *testing.T) {
	job := &MarkdownToPDFJob{
		Operation:    "markdown_to_pdf",
		MarkdownText: strPtr("# Custom Theme"),
		Inline:       true,
		Theme:        strPtr("engineering"),
		ThemeOverride: &ThemeOverride{
			BodyFontSize: float64Ptr(11.5),
			CodeFontSize: float64Ptr(9.5),
			HeadingScale: float64Ptr(1.4),
			MarginMM:     float64Ptr(14.0),
		},
	}

	job.applyThemeOverride()

	if job.ThemeConfig == nil {
		t.Fatal("expected ThemeConfig to be populated from ThemeOverride")
	}

	var override ThemeOverride
	if err := json.Unmarshal(job.ThemeConfig, &override); err != nil {
		t.Fatalf("ThemeConfig should unmarshal to ThemeOverride: %v", err)
	}

	if override.BodyFontSize == nil || *override.BodyFontSize != 11.5 {
		t.Fatalf("expected body_font_size_pt=11.5, got %v", override.BodyFontSize)
	}
	if override.CodeFontSize == nil || *override.CodeFontSize != 9.5 {
		t.Fatalf("expected code_font_size_pt=9.5, got %v", override.CodeFontSize)
	}
	if override.HeadingScale == nil || *override.HeadingScale != 1.4 {
		t.Fatalf("expected heading_scale=1.4, got %v", override.HeadingScale)
	}
	if override.MarginMM == nil || *override.MarginMM != 14.0 {
		t.Fatalf("expected margin_mm=14.0, got %v", override.MarginMM)
	}
}

func TestClientMarkdownToPDFWithThemeOverride(t *testing.T) {
	binaryPath := setupBinary(t)
	client := NewClient(binaryPath)

	result, err := client.MarkdownToPDF(context.Background(), &MarkdownToPDFJob{
		MarkdownText: strPtr("# Theme Override\n\nCustom font sizes and margins."),
		Inline:       true,
		Theme:        strPtr("professional"),
		ThemeOverride: &ThemeOverride{
			BodyFontSize: float64Ptr(12.0),
			MarginMM:     float64Ptr(20.0),
		},
	})
	if err != nil {
		t.Fatalf("MarkdownToPDF with ThemeOverride failed: %v", err)
	}
	if !result.Success {
		t.Fatal("expected success")
	}
	if result.Outputs[0].DataBase64 == nil {
		t.Fatal("expected inline PDF bytes")
	}

	decoded, err := base64.StdEncoding.DecodeString(*result.Outputs[0].DataBase64)
	if err != nil {
		t.Fatalf("inline PDF should decode: %v", err)
	}
	if len(decoded) < 4 || string(decoded[:4]) != "%PDF" {
		t.Fatal("decoded inline bytes do not look like a PDF")
	}
	assertMetadataBackend(t, result, "typst")
}
