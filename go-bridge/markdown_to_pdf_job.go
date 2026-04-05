package dpf

import "encoding/json"

// MarkdownToPDFJob defines the Markdown-to-PDF operation contract.
type MarkdownToPDFJob struct {
	Operation      string            `json:"operation"`
	Input          string            `json:"input,omitempty"`
	MarkdownText   *string           `json:"markdown_text,omitempty"`
	MarkdownBase64 *string           `json:"markdown_base64,omitempty"`
	Output         string            `json:"output,omitempty"`
	OutputDir      string            `json:"output_dir,omitempty"`
	FileName       string            `json:"file_name,omitempty"`
	Inline         bool              `json:"inline,omitempty"`
	PageSize       *string           `json:"page_size,omitempty"`
	PageWidthMM    *float64          `json:"page_width_mm,omitempty"`
	PageHeightMM   *float64          `json:"page_height_mm,omitempty"`
	LayoutMode     *string           `json:"layout_mode,omitempty"`
	Theme          *string           `json:"theme,omitempty"`
	ThemeConfig    json.RawMessage   `json:"theme_config,omitempty"`
	ResourceFiles  map[string]string `json:"resource_files,omitempty"`
}
