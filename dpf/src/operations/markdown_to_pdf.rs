use anyhow::{Context, Result};
use base64::Engine;
use glyphweaveforge::{
    BuiltInTheme, Forge, ForgeError, LayoutMode, PageSize, RenderBackendSelection, ThemeConfig,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

use crate::{JobResult, OutputFile};

const INLINE_PDF_PATH: &str = "inline://markdown-to-pdf.pdf";

/// JSON contract for the `markdown_to_pdf` operation.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct MarkdownToPdfParams {
    /// Source markdown file path.
    pub input: Option<String>,
    /// Inline UTF-8 markdown text.
    pub markdown_text: Option<String>,
    /// Base64-encoded UTF-8 markdown bytes.
    pub markdown_base64: Option<String>,
    /// Explicit output PDF path.
    pub output: Option<String>,
    /// Output directory for generated PDFs.
    pub output_dir: Option<String>,
    /// Optional file name override when using `output_dir`.
    pub file_name: Option<String>,
    /// Include an inline base64 PDF payload in the response.
    #[serde(default)]
    pub inline: bool,
    /// Page size preset (`a4`, `letter`, `legal`).
    pub page_size: Option<String>,
    /// Custom page width in millimeters.
    pub page_width_mm: Option<f32>,
    /// Custom page height in millimeters.
    pub page_height_mm: Option<f32>,
    /// Layout mode (`paged`, `single_page`).
    pub layout_mode: Option<String>,
    /// Theme preset (`professional`, `engineering`, `invoice`, `scientific_article`, `informational`).
    pub theme: Option<String>,
    /// Optional theme overrides passed through to GlyphWeaveForge.
    pub theme_config: Option<Value>,
    /// Optional href-to-file mapping used to resolve assets for in-memory markdown sources.
    pub resource_files: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone)]
enum MarkdownInput {
    File(PathBuf),
    Text(String),
    SanitizedText {
        markdown: String,
        source_dir: Option<PathBuf>,
    },
}

#[derive(Debug, Clone)]
struct ValidatedParams {
    input: MarkdownInput,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    file_name: Option<String>,
    inline: bool,
    page_size: PageSize,
    layout_mode: LayoutMode,
    theme: Option<BuiltInTheme>,
    theme_config: Option<Value>,
    resource_files: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Error)]
pub enum MarkdownToPdfError {
    #[error("exactly one markdown input source must be provided: input, markdown_text, or markdown_base64")]
    InvalidInputSelection,
    #[error("at least one output mode must be provided: output, output_dir, or inline=true")]
    MissingOutputMode,
    #[error("file_name can only be used together with output_dir")]
    FileNameRequiresOutputDir,
    #[error("file_name is required when output_dir is used with inline markdown input")]
    MissingFileNameForInlineDirectoryOutput,
    #[error("page_width_mm and page_height_mm must both be provided for a custom page size")]
    CustomPageSizeRequiresBothDimensions,
    #[error("page_width_mm and page_height_mm must be positive values")]
    InvalidCustomPageSize,
    #[error("unsupported page_size: {0}")]
    UnsupportedPageSize(String),
    #[error("unsupported layout_mode: {0}")]
    UnsupportedLayoutMode(String),
    #[error("unsupported theme: {0}")]
    UnsupportedTheme(String),
    #[error("markdown_base64 is not valid base64")]
    InvalidMarkdownBase64,
    #[error("inline markdown bytes are not valid UTF-8")]
    InvalidMarkdownUtf8,
    #[error("input markdown file not found: {0}")]
    InputNotFound(String),
    #[error("failed to render markdown to PDF: {0}")]
    Render(#[from] ForgeError),
    #[error("failed to write PDF output to {path}: {source}")]
    OutputWrite {
        path: String,
        source: std::io::Error,
    },
}

/// Executes the `markdown_to_pdf` operation.
pub fn execute(params: MarkdownToPdfParams) -> Result<JobResult> {
    let validated = validate_params(params)?;
    let metadata = build_metadata(&validated);

    let mut outputs = Vec::new();
    let must_render_to_memory =
        validated.inline || validated.output.is_some() && validated.output_dir.is_some();

    if must_render_to_memory {
        let pdf_bytes = render_to_memory(&validated)?;

        if let Some(output_path) = validated.output.as_ref() {
            write_pdf(output_path, &pdf_bytes)?;
            outputs.push(file_output(output_path, None)?);
        }

        if let Some(output_dir) = validated.output_dir.as_ref() {
            let output_path = resolve_output_dir_path(&validated, output_dir)?;
            write_pdf(&output_path, &pdf_bytes)?;
            outputs.push(file_output(&output_path, None)?);
        }

        if validated.inline {
            outputs.push(inline_output(&pdf_bytes));
        }
    } else if let Some(output_path) = validated.output.as_ref() {
        let written_path = render_to_file(&validated, output_path)?;
        outputs.push(file_output(&written_path, None)?);
    } else if let Some(output_dir) = validated.output_dir.as_ref() {
        let written_path = render_to_directory(&validated, output_dir)?;
        outputs.push(file_output(&written_path, None)?);
    }

    Ok(JobResult {
        success: true,
        operation: "markdown_to_pdf".into(),
        outputs,
        elapsed_ms: 0,
        metadata: Some(metadata),
    })
}

fn validate_params(params: MarkdownToPdfParams) -> Result<ValidatedParams> {
    let mut input_count = 0;
    if params.input.is_some() {
        input_count += 1;
    }
    if params.markdown_text.is_some() {
        input_count += 1;
    }
    if params.markdown_base64.is_some() {
        input_count += 1;
    }

    if input_count != 1 {
        return Err(MarkdownToPdfError::InvalidInputSelection.into());
    }

    if params.output.is_none() && params.output_dir.is_none() && !params.inline {
        return Err(MarkdownToPdfError::MissingOutputMode.into());
    }

    if params.file_name.is_some() && params.output_dir.is_none() {
        return Err(MarkdownToPdfError::FileNameRequiresOutputDir.into());
    }

    let input = if let Some(input) = params.input {
        let input_path = PathBuf::from(&input);
        if !input_path.exists() {
            return Err(MarkdownToPdfError::InputNotFound(input).into());
        }
        MarkdownInput::File(input_path)
    } else if let Some(markdown_text) = params.markdown_text {
        MarkdownInput::Text(markdown_text)
    } else if let Some(markdown_base64) = params.markdown_base64 {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(markdown_base64)
            .map_err(|_| MarkdownToPdfError::InvalidMarkdownBase64)?;
        std::str::from_utf8(&decoded).map_err(|_| MarkdownToPdfError::InvalidMarkdownUtf8)?;
        MarkdownInput::SanitizedText {
            markdown: String::from_utf8(decoded)
                .map_err(|_| MarkdownToPdfError::InvalidMarkdownUtf8)?,
            source_dir: None,
        }
    } else {
        return Err(MarkdownToPdfError::InvalidInputSelection.into());
    };

    let input = sanitize_input(input)?;

    if params.output_dir.is_some()
        && params.file_name.is_none()
        && !matches!(input, MarkdownInput::File(_))
    {
        return Err(MarkdownToPdfError::MissingFileNameForInlineDirectoryOutput.into());
    }

    Ok(ValidatedParams {
        input,
        output: params.output.map(PathBuf::from),
        output_dir: params.output_dir.map(PathBuf::from),
        file_name: params.file_name,
        inline: params.inline,
        page_size: resolve_page_size(
            params.page_size,
            params.page_width_mm,
            params.page_height_mm,
        )?,
        layout_mode: resolve_layout_mode(params.layout_mode)?,
        theme: resolve_theme(params.theme)?,
        theme_config: params.theme_config,
        resource_files: params
            .resource_files
            .unwrap_or_default()
            .into_iter()
            .map(|(href, path)| (href, PathBuf::from(path)))
            .collect(),
    })
}

fn resolve_page_size(
    page_size: Option<String>,
    page_width_mm: Option<f32>,
    page_height_mm: Option<f32>,
) -> Result<PageSize> {
    match (page_width_mm, page_height_mm) {
        (Some(width_mm), Some(height_mm)) => {
            if width_mm <= 0.0 || height_mm <= 0.0 {
                return Err(MarkdownToPdfError::InvalidCustomPageSize.into());
            }
            Ok(PageSize::Custom {
                width_mm,
                height_mm,
            })
        }
        (Some(_), None) | (None, Some(_)) => {
            Err(MarkdownToPdfError::CustomPageSizeRequiresBothDimensions.into())
        }
        (None, None) => match page_size.as_deref().unwrap_or("a4") {
            "a4" => Ok(PageSize::A4),
            "letter" => Ok(PageSize::Letter),
            "legal" => Ok(PageSize::Legal),
            other => Err(MarkdownToPdfError::UnsupportedPageSize(other.to_string()).into()),
        },
    }
}

fn resolve_layout_mode(layout_mode: Option<String>) -> Result<LayoutMode> {
    match layout_mode.as_deref().unwrap_or("paged") {
        "paged" => Ok(LayoutMode::Paged),
        "single_page" => Ok(LayoutMode::SinglePage),
        other => Err(MarkdownToPdfError::UnsupportedLayoutMode(other.to_string()).into()),
    }
}

fn resolve_theme(theme: Option<String>) -> Result<Option<BuiltInTheme>> {
    match theme.as_deref() {
        None => Ok(None),
        Some("professional") => Ok(Some(BuiltInTheme::Professional)),
        Some("engineering") => Ok(Some(BuiltInTheme::Engineering)),
        Some("invoice") => Ok(Some(BuiltInTheme::Invoice)),
        Some("scientific_article") => Ok(Some(BuiltInTheme::ScientificArticle)),
        Some("informational") => Ok(Some(BuiltInTheme::Informational)),
        Some(other) => Err(MarkdownToPdfError::UnsupportedTheme(other.to_string()).into()),
    }
}

fn render_to_memory(params: &ValidatedParams) -> Result<Vec<u8>> {
    let forge = build_forge(params).to_memory();
    let output = forge.convert().map_err(MarkdownToPdfError::from)?;
    output
        .bytes
        .context("markdown_to_pdf memory output did not contain bytes")
}

fn render_to_file(params: &ValidatedParams, output_path: &Path) -> Result<PathBuf> {
    let forge = build_forge(params).to_file(output_path);
    let output = forge.convert().map_err(MarkdownToPdfError::from)?;
    output
        .written_path
        .context("markdown_to_pdf file output did not report a written path")
}

fn render_to_directory(params: &ValidatedParams, output_dir: &Path) -> Result<PathBuf> {
    let mut forge = build_forge(params).to_directory(output_dir);
    if let Some(file_name) = params.file_name.as_ref() {
        forge = forge.with_output_file_name(file_name);
    }
    let output = forge.convert().map_err(MarkdownToPdfError::from)?;
    output
        .written_path
        .context("markdown_to_pdf directory output did not report a written path")
}

fn build_forge(params: &ValidatedParams) -> Forge<'_> {
    let forge = match &params.input {
        MarkdownInput::File(path) => Forge::new().from_path(path),
        MarkdownInput::Text(markdown) => Forge::new().from_text(markdown),
        MarkdownInput::SanitizedText { markdown, .. } => Forge::new().from_text(markdown),
    };

    let mut forge = forge
        .with_backend(RenderBackendSelection::Typst)
        .with_page_size(params.page_size)
        .with_layout_mode(params.layout_mode);

    if let (Some(theme), None) = (params.theme.as_ref(), params.theme_config.as_ref()) {
        forge = forge.with_theme(*theme);
    } else if params.theme.is_some() || params.theme_config.is_some() {
        forge = forge.with_theme_config(ThemeConfig {
            built_in: params.theme,
            custom_theme_json: params.theme_config.clone(),
        });
    }

    if should_attach_resource_resolver(params) {
        let resource_files = Arc::new(params.resource_files.clone());
        let source_dir = sanitized_source_dir(params).map(PathBuf::from);
        forge.with_resource_resolver(move |href| {
            resolve_resource_file(resource_files.as_ref(), source_dir.as_deref(), href)
        })
    } else {
        forge
    }
}

fn resolve_resource_file(
    resource_files: &BTreeMap<String, PathBuf>,
    source_dir: Option<&Path>,
    href: &str,
) -> io::Result<Vec<u8>> {
    if let Some(path) = resource_files.get(href) {
        return std::fs::read(path);
    }

    if let Some(source_dir) = source_dir {
        return std::fs::read(source_dir.join(href));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("resource not found in resource_files: {href}"),
    ))
}

fn resolve_output_dir_path(params: &ValidatedParams, output_dir: &Path) -> Result<PathBuf> {
    if let Some(file_name) = params.file_name.as_ref() {
        return Ok(output_dir.join(ensure_pdf_suffix(file_name)));
    }

    match &params.input {
        MarkdownInput::File(path) => {
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("document");
            Ok(output_dir.join(format!("{}.pdf", stem)))
        }
        MarkdownInput::Text(_)
        | MarkdownInput::SanitizedText {
            source_dir: None, ..
        } => Err(MarkdownToPdfError::MissingFileNameForInlineDirectoryOutput.into()),
        MarkdownInput::SanitizedText {
            source_dir: Some(_),
            ..
        } => Err(MarkdownToPdfError::MissingFileNameForInlineDirectoryOutput.into()),
    }
}

fn ensure_pdf_suffix(file_name: &str) -> String {
    if file_name.to_ascii_lowercase().ends_with(".pdf") {
        file_name.to_string()
    } else {
        format!("{}.pdf", file_name)
    }
}

fn write_pdf(output_path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create PDF output directory: {}",
                parent.display()
            )
        })?;
    }

    std::fs::write(output_path, bytes).map_err(|source| MarkdownToPdfError::OutputWrite {
        path: output_path.display().to_string(),
        source,
    })?;
    Ok(())
}

fn file_output(path: &Path, data_base64: Option<String>) -> Result<OutputFile> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("failed to inspect rendered PDF output: {}", path.display()))?;

    Ok(OutputFile {
        path: path.display().to_string(),
        format: "pdf".into(),
        width: 0,
        height: 0,
        size_bytes: metadata.len(),
        data_base64,
    })
}

fn inline_output(bytes: &[u8]) -> OutputFile {
    OutputFile {
        path: INLINE_PDF_PATH.into(),
        format: "pdf".into(),
        width: 0,
        height: 0,
        size_bytes: bytes.len() as u64,
        data_base64: Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
    }
}

fn build_metadata(params: &ValidatedParams) -> Value {
    json!({
        "backend": "typst",
        "page_size": page_size_label(&params.page_size),
        "layout_mode": layout_mode_label(&params.layout_mode),
        "theme": theme_label(params.theme.as_ref()),
        "inline": params.inline,
        "has_file_output": params.output.is_some() || params.output_dir.is_some(),
        "resource_resolver": resource_resolver_label(params),
        "resource_files": params.resource_files.len(),
    })
}

fn resource_resolver_label(params: &ValidatedParams) -> &'static str {
    if !params.resource_files.is_empty() {
        "custom"
    } else if matches!(params.input, MarkdownInput::File(_))
        || sanitized_source_dir(params).is_some()
    {
        "filesystem"
    } else {
        "none"
    }
}

fn should_attach_resource_resolver(params: &ValidatedParams) -> bool {
    !params.resource_files.is_empty() || sanitized_source_dir(params).is_some()
}

fn sanitized_source_dir(params: &ValidatedParams) -> Option<&Path> {
    match &params.input {
        MarkdownInput::SanitizedText {
            source_dir: Some(source_dir),
            ..
        } => Some(source_dir.as_path()),
        _ => None,
    }
}

fn sanitize_input(input: MarkdownInput) -> Result<MarkdownInput> {
    match input {
        MarkdownInput::File(path) => {
            let markdown = std::fs::read_to_string(&path).with_context(|| {
                format!(
                    "failed to read markdown input for sanitization: {}",
                    path.display()
                )
            })?;
            let sanitized = sanitize_markdown(&markdown);
            if sanitized.changed {
                Ok(MarkdownInput::SanitizedText {
                    markdown: sanitized.markdown,
                    source_dir: path.parent().map(Path::to_path_buf),
                })
            } else {
                Ok(MarkdownInput::File(path))
            }
        }
        MarkdownInput::Text(markdown) => {
            Ok(MarkdownInput::Text(sanitize_markdown(&markdown).markdown))
        }
        MarkdownInput::SanitizedText {
            markdown,
            source_dir,
        } => Ok(MarkdownInput::SanitizedText {
            markdown: sanitize_markdown(&markdown).markdown,
            source_dir,
        }),
    }
}

#[derive(Debug)]
struct SanitizedMarkdown {
    markdown: String,
    changed: bool,
}

fn sanitize_markdown(markdown: &str) -> SanitizedMarkdown {
    let block_wrapper_re =
        Regex::new(r#"(?i)^\s*</?(?:p|div|span|section|article|header|footer|center)\b[^>]*>\s*$"#)
            .expect("wrapper regex should compile");
    let img_tag_re = Regex::new(r#"(?i)<img\b[^>]*>"#).expect("img regex should compile");

    let mut changed = false;
    let mut lines = Vec::new();
    let mut in_code_fence = false;

    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            lines.push(line.to_string());
            continue;
        }

        if in_code_fence {
            lines.push(line.to_string());
            continue;
        }

        if block_wrapper_re.is_match(line.trim()) {
            changed = true;
            continue;
        }

        let replaced = img_tag_re.replace_all(line, |captures: &regex::Captures<'_>| {
            let tag = captures
                .get(0)
                .map(|value| value.as_str())
                .unwrap_or_default();
            convert_html_img_tag(tag)
        });
        if replaced.as_ref() != line {
            changed = true;
            lines.push(replaced.into_owned());
        } else {
            lines.push(line.to_string());
        }
    }

    SanitizedMarkdown {
        markdown: lines.join("\n"),
        changed,
    }
}

fn convert_html_img_tag(tag: &str) -> String {
    let src = extract_html_attribute(tag, "src");
    let alt = extract_html_attribute(tag, "alt").unwrap_or_else(|| "image".to_string());

    match src {
        Some(src) => format!("![{}]({})", alt, src),
        None => tag.to_string(),
    }
}

fn extract_html_attribute(tag: &str, attribute: &str) -> Option<String> {
    let pattern = format!(
        r#"(?i)\b{}\s*=\s*(?:\"([^\"]*)\"|'([^']*)')"#,
        regex::escape(attribute)
    );
    let regex = Regex::new(&pattern).expect("attribute regex should compile");
    let captures = regex.captures(tag)?;
    captures
        .get(1)
        .or_else(|| captures.get(2))
        .map(|value| value.as_str().to_string())
}

fn page_size_label(page_size: &PageSize) -> Value {
    match page_size {
        PageSize::A4 => json!("a4"),
        PageSize::Letter => json!("letter"),
        PageSize::Legal => json!("legal"),
        PageSize::Custom {
            width_mm,
            height_mm,
        } => json!({
            "name": "custom",
            "width_mm": width_mm,
            "height_mm": height_mm,
        }),
    }
}

fn layout_mode_label(layout_mode: &LayoutMode) -> &'static str {
    match layout_mode {
        LayoutMode::Paged => "paged",
        LayoutMode::SinglePage => "single_page",
    }
}

fn theme_label(theme: Option<&BuiltInTheme>) -> &'static str {
    match theme {
        Some(BuiltInTheme::Professional) | None => "professional",
        Some(BuiltInTheme::Engineering) => "engineering",
        Some(BuiltInTheme::Invoice) => "invoice",
        Some(BuiltInTheme::ScientificArticle) => "scientific_article",
        Some(BuiltInTheme::Informational) => "informational",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fixtures_dir() -> String {
        concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
    }

    fn fixture_path(name: &str) -> String {
        format!("{}/{}", fixtures_dir(), name)
    }

    #[test]
    fn rejects_multiple_input_sources() {
        let params = MarkdownToPdfParams {
            input: Some(fixture_path("sample.md")),
            markdown_text: Some("# Duplicate".into()),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: None,
            theme_config: None,
            resource_files: None,
        };

        let error = validate_params(params).expect_err("validation should fail");
        assert!(error
            .to_string()
            .contains("exactly one markdown input source"));
    }

    #[test]
    fn rejects_missing_output_mode() {
        let params = MarkdownToPdfParams {
            input: None,
            markdown_text: Some("# Missing output".into()),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: false,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: None,
            theme_config: None,
            resource_files: None,
        };

        let error = validate_params(params).expect_err("validation should fail");
        assert!(error.to_string().contains("at least one output mode"));
    }

    #[test]
    fn rejects_invalid_base64() {
        let params = MarkdownToPdfParams {
            input: None,
            markdown_text: None,
            markdown_base64: Some("%%%".into()),
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: None,
            theme_config: None,
            resource_files: None,
        };

        let error = validate_params(params).expect_err("validation should fail");
        assert!(error.to_string().contains("not valid base64"));
    }

    #[test]
    fn requires_file_name_for_inline_directory_output() {
        let params = MarkdownToPdfParams {
            input: None,
            markdown_text: Some("# Inline directory".into()),
            markdown_base64: None,
            output: None,
            output_dir: Some("/tmp/out".into()),
            file_name: None,
            inline: false,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: None,
            theme_config: None,
            resource_files: None,
        };

        let error = validate_params(params).expect_err("validation should fail");
        assert!(error.to_string().contains("file_name is required"));
    }

    #[test]
    fn shapes_inline_pdf_output() {
        let output = inline_output(b"%PDF-test");
        assert_eq!(output.path, INLINE_PDF_PATH);
        assert_eq!(output.format, "pdf");
        assert_eq!(output.width, 0);
        assert_eq!(output.height, 0);
        assert!(output.data_base64.is_some());
    }

    #[test]
    fn sanitizes_raw_html_wrappers_and_images() {
        let sanitized =
            sanitize_markdown("<p align=\"center\">\n<img src=\"logo.png\" alt=\"Logo\">\n</p>\n");

        assert!(sanitized.changed);
        assert_eq!(sanitized.markdown, "![Logo](logo.png)");
    }

    #[test]
    fn keeps_raw_html_inside_code_fences() {
        let source = "```html\n<div align=\"center\">\n</div>\n```";
        let sanitized = sanitize_markdown(source);

        assert!(!sanitized.changed);
        assert_eq!(sanitized.markdown, source);
    }

    #[test]
    fn renders_markdown_text_to_inline_pdf() {
        let result = execute(MarkdownToPdfParams {
            input: None,
            markdown_text: Some("# Inline\n\nHello".into()),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: Some("letter".into()),
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: Some("paged".into()),
            theme: Some("engineering".into()),
            theme_config: None,
            resource_files: None,
        })
        .expect("conversion should succeed");

        assert!(result.success);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].format, "pdf");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(
                result.outputs[0]
                    .data_base64
                    .as_ref()
                    .expect("inline PDF expected"),
            )
            .expect("base64 should decode");
        assert!(decoded.starts_with(b"%PDF"));
    }

    #[test]
    fn writes_pdf_into_output_directory_with_source_stem() {
        let temp_dir = TempDir::new().expect("temp dir should exist");
        let result = execute(MarkdownToPdfParams {
            input: Some(fixture_path("sample.md")),
            markdown_text: None,
            markdown_base64: None,
            output: None,
            output_dir: Some(temp_dir.path().display().to_string()),
            file_name: None,
            inline: false,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: None,
            theme_config: None,
            resource_files: None,
        })
        .expect("conversion should succeed");

        assert_eq!(result.outputs.len(), 1);
        assert!(result.outputs[0].path.ends_with("sample.pdf"));
        assert!(Path::new(&result.outputs[0].path).exists());
    }

    #[test]
    fn renders_shell_comment_code_blocks_to_pdf() {
        let result = execute(MarkdownToPdfParams {
            input: None,
            markdown_text: Some("```bash\n# Build\nmake build\n```".into()),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: Some("engineering".into()),
            theme_config: None,
            resource_files: None,
        })
        .expect("conversion should succeed");

        assert!(result.success);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].format, "pdf");
    }

    #[test]
    fn renders_generic_angle_brackets_inside_code_blocks() {
        let result = execute(MarkdownToPdfParams {
            input: None,
            markdown_text: Some("```rust\nfn generic<T, U>() {}\n```".into()),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: Some("engineering".into()),
            theme_config: None,
            resource_files: None,
        })
        .expect("conversion should succeed");

        assert!(result.success);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].format, "pdf");
    }

    #[test]
    fn renders_slash_prefixed_comments_inside_code_blocks() {
        let result = execute(MarkdownToPdfParams {
            input: None,
            markdown_text: Some("```rust\n// processing logic\n```".into()),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: Some("engineering".into()),
            theme_config: None,
            resource_files: None,
        })
        .expect("conversion should succeed");

        assert!(result.success);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].format, "pdf");
    }

    #[test]
    fn renders_pointer_type_syntax_inside_code_blocks() {
        let result = execute(MarkdownToPdfParams {
            input: None,
            markdown_text: Some(
                "```go\nfunc NewImageProcessor(input string) *ImageProcessor {\n}\n```".into(),
            ),
            markdown_base64: None,
            output: None,
            output_dir: None,
            file_name: None,
            inline: true,
            page_size: None,
            page_width_mm: None,
            page_height_mm: None,
            layout_mode: None,
            theme: Some("engineering".into()),
            theme_config: None,
            resource_files: None,
        })
        .expect("conversion should succeed");

        assert!(result.success);
        assert_eq!(result.outputs.len(), 1);
        assert_eq!(result.outputs[0].format, "pdf");
    }
}
