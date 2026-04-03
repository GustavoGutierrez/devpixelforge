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
