//! devpixelforge (dpf) — Motor de procesamiento de imágenes de alto rendimiento.
//!
//! Diseñado para ser invocado desde Go (o cualquier proceso padre) vía stdin/stdout JSON.
//! Soporta dos modos:
//!   1. CLI con argumentos (para invocaciones simples)
//!   2. Modo streaming por stdin (para batch y pipeline)
//!
//! Autor: Ing. Gustavo Gutiérrez

#![recursion_limit = "512"]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

pub mod operations;
pub mod pipeline;

use audio::{AudioNormalizeParams, AudioSilenceTrimParams, AudioTranscodeParams, AudioTrimParams};
use operations::{
    adjust::AdjustParams, audio, batch::BatchJob, convert::ConvertParams, crop::CropParams,
    exif_ops::ExifParams, favicon::FaviconParams, markdown_to_pdf::MarkdownToPdfParams,
    optimize::OptimizeParams, palette::PaletteParams, placeholder::PlaceholderParams,
    quality::QualityParams, resize::ResizeParams, rotate::RotateParams, sprite::SpriteParams,
    srcset::SrcsetParams, video, watermark::WatermarkParams,
};

// ─── CLI Interface ───────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "dpf",
    version,
    about = "devpixelforge - Image processing engine"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Modo streaming: lee trabajos JSON de stdin línea por línea
    #[arg(long, short = 's')]
    stream: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Procesar un solo trabajo desde JSON en argumento
    Process {
        /// JSON del trabajo a procesar
        #[arg(long)]
        job: String,
    },
    /// Procesar un archivo JSON con múltiples trabajos (batch)
    Batch {
        /// Ruta al archivo JSON con array de trabajos
        #[arg(long)]
        file: PathBuf,
    },
    /// Info sobre formatos y capacidades soportadas
    Caps,
}

// ─── Job Protocol (lo que Go envía / recibe) ─────────────────────

/// Video job parameters - supports multiple video operations.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum VideoJob {
    /// Transcode video with codec and bitrate control
    Transcode(video::VideoTranscodeParams),
    /// Resize video maintaining aspect ratio
    Resize(video::VideoResizeParams),
    /// Trim video by start/end timestamps
    Trim(video::VideoTrimParams),
    /// Extract thumbnails at timestamps
    Thumbnail(video::VideoThumbnailParams),
    /// Apply web-optimized encoding profile
    Profile(video::VideoProfileParams),
    /// Extract video metadata
    Metadata(video::VideoMetadataParams),
}

/// Audio job parameters - supports multiple audio operations.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum AudioJob {
    /// Audio transcode operation
    Transcode(AudioTranscodeParams),
    /// Audio trim operation
    Trim(AudioTrimParams),
    /// Audio loudness normalization
    Normalize(AudioNormalizeParams),
    /// Audio silence trimming
    SilenceTrim(AudioSilenceTrimParams),
}

/// Trabajo de procesamiento — el contrato entre Go y Rust.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum ImageJob {
    /// Redimensionar imagen (thumbnail, responsive sizes)
    Resize(ResizeParams),
    /// Optimizar imagen (lossless/lossy compression)
    Optimize(OptimizeParams),
    /// Convertir formato (png→webp, svg→png, etc.)
    Convert(ConvertParams),
    /// Recortar imagen (manual o smart crop)
    Crop(CropParams),
    /// Rotar y/o voltear imagen
    Rotate(RotateParams),
    /// Añadir watermark (texto o imagen)
    Watermark(WatermarkParams),
    /// Ajustar imagen (brillo, contraste, saturación, blur, sharpen)
    Adjust(AdjustParams),
    /// Generar favicon multi-tamaño desde imagen o SVG
    Favicon(FaviconParams),
    /// Generar sprite sheet desde múltiples imágenes
    Sprite(SpriteParams),
    /// Generar placeholder (blur hash, dominant color, LQIP)
    Placeholder(PlaceholderParams),
    /// Reducir paleta de colores (útil para PNG/GIF)
    Palette(PaletteParams),
    /// Auto-optimize quality for target file size
    Quality(QualityParams),
    /// Generate responsive srcset variants
    Srcset(SrcsetParams),
    /// EXIF operations (strip, preserve, extract, auto_orient)
    Exif(ExifParams),
    /// Convert Markdown into PDF using the Typst renderer backend
    MarkdownToPdf(MarkdownToPdfParams),
    /// Video processing operations
    Video(VideoJob),
    /// Audio processing operations
    Audio(AudioJob),
    /// Batch: múltiples operaciones en paralelo
    Batch(BatchJob),
}

/// Resultado exitoso de una operación.
#[derive(Debug, Serialize)]
pub struct JobResult {
    pub success: bool,
    pub operation: String,
    pub outputs: Vec<OutputFile>,
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Archivo de salida producido.
#[derive(Debug, Serialize, Clone)]
pub struct OutputFile {
    pub path: String,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    /// Base64 del contenido si se solicitó inline output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
}

/// Respuesta de error.
#[derive(Debug, Serialize)]
pub struct JobError {
    pub success: bool,
    pub operation: String,
    pub error: String,
    pub elapsed_ms: u64,
}

// ─── Main ────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Inicializar rayon con todos los cores disponibles
    rayon::ThreadPoolBuilder::new()
        .num_threads(0) // 0 = detectar automáticamente
        .build_global()
        .context("Failed to initialize thread pool")?;

    if cli.stream {
        return run_stream_mode();
    }

    match cli.command {
        Some(Commands::Process { job }) => {
            let result = process_job_json(&job)?;
            println!("{}", result);
        }
        Some(Commands::Batch { file }) => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Cannot read batch file: {}", file.display()))?;
            let jobs: Vec<ImageJob> =
                serde_json::from_str(&content).context("Invalid batch JSON")?;
            let results = pipeline::run_parallel(jobs);
            println!("{}", serde_json::to_string(&results)?);
        }
        Some(Commands::Caps) => {
            print_capabilities();
        }
        None => {
            // Sin subcomando ni --stream: leer un solo JSON de stdin
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().is_empty() {
                let result = process_job_json(input.trim())?;
                println!("{}", result);
            }
        }
    }

    Ok(())
}

/// Modo streaming: lee JSON línea por línea de stdin, responde en stdout.
/// Ideal para Go manteniendo un proceso Rust persistente.
fn run_stream_mode() -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = line.context("Failed to read stdin")?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let result = process_job_json(trimmed).unwrap_or_else(|e| {
            serde_json::to_string(&JobError {
                success: false,
                operation: "unknown".into(),
                error: e.to_string(),
                elapsed_ms: 0,
            })
            .unwrap_or_default()
        });

        writeln!(out, "{}", result)?;
        out.flush()?;
    }

    Ok(())
}

/// Procesa un JSON de trabajo y devuelve el resultado como JSON string.
fn process_job_json(json: &str) -> Result<String> {
    let job: ImageJob = serde_json::from_str(json).context("Invalid job JSON")?;

    let start = Instant::now();
    let op_name = job.operation_name();

    match pipeline::execute(job) {
        Ok(mut result) => {
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            result.operation = op_name;
            Ok(serde_json::to_string(&result)?)
        }
        Err(e) => Ok(serde_json::to_string(&JobError {
            success: false,
            operation: op_name,
            error: e.to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })?),
    }
}

fn print_capabilities() {
    let caps = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "operations": [
            "resize", "optimize", "convert", "crop", "rotate", "watermark", "adjust",
            "favicon", "sprite", "placeholder", "palette", "quality", "srcset", "exif",
            "markdown_to_pdf",
            "video_transcode", "video_resize", "video_trim", "video_thumbnail",
            "video_profile", "video_metadata",
            "audio_transcode", "audio_trim", "audio_normalize", "audio_silence_trim",
            "batch"
        ],
        "input_formats": {
            "image": ["png", "jpeg", "gif", "webp", "bmp", "tiff", "svg", "ico"],
            "video": ["mp4", "webm", "mkv", "avi", "mov", "m4v"],
            "audio": ["mp3", "aac", "ogg", "wav", "flac", "opus", "m4a"]
        },
        "output_formats": {
            "image": ["png", "jpeg", "webp", "ico", "avif", "gif"],
            "video": ["mp4", "webm", "mkv", "avi", "mov"],
            "audio": ["mp3", "aac", "ogg", "wav", "flac", "opus"],
            "document": ["pdf"]
        },
        "video_profiles": ["web-low", "web-mid", "web-high"],
        "video_codecs": ["h264", "vp8", "vp9", "av1"],
        "audio_codecs": ["aac", "mp3", "opus", "vorbis", "flac"],
        "features": {
            "parallel_batch": true,
            "streaming_mode": true,
            "inline_base64_output": true,
            "svg_to_raster": true,
            "lossless_optimization": true,
            "lossy_optimization": true,
            "responsive_sizes": true,
            "favicon_multi_size": true,
            "placeholder_generation": true,
            "sprite_generation": true,
            "palette_reduction": true,
            "resize_by_percent": true,
            "linear_rgb_resize": true,
            "crop_manual": true,
            "crop_smart": true,
            "rotate_90_180_270": true,
            "rotate_arbitrary": true,
            "flip_horizontal_vertical": true,
            "auto_orient_exif": true,
            "watermark_text": true,
            "watermark_image": true,
            "watermark_9position": true,
            "adjust_brightness": true,
            "adjust_contrast": true,
            "adjust_saturation": true,
            "adjust_blur": true,
            "adjust_sharpen": true,
            "linear_rgb_adjustments": true,
            "auto_quality_binary_search": true,
            "srcset_generation": true,
            "responsive_images": true,
            "exif_strip": true,
            "exif_preserve": true,
            "exif_extract": true,
            "exif_auto_orient": true,
            "video_transcode": true,
            "video_resize": true,
            "video_trim": true,
            "video_thumbnail": true,
            "video_profile": true,
            "video_metadata": true,
            "audio_transcode": true,
            "audio_trim": true,
            "audio_normalize": true,
            "audio_silence_trim": true,
            "markdown_to_pdf": true,
            "markdown_to_pdf_typst": true,
            "pdf_inline_output": true
        },
        "threads": rayon::current_num_threads()
    });
    println!("{}", serde_json::to_string_pretty(&caps).unwrap());
}

impl ImageJob {
    fn operation_name(&self) -> String {
        match self {
            ImageJob::Resize(_) => "resize",
            ImageJob::Optimize(_) => "optimize",
            ImageJob::Convert(_) => "convert",
            ImageJob::Crop(_) => "crop",
            ImageJob::Rotate(_) => "rotate",
            ImageJob::Watermark(_) => "watermark",
            ImageJob::Adjust(_) => "adjust",
            ImageJob::Favicon(_) => "favicon",
            ImageJob::Sprite(_) => "sprite",
            ImageJob::Placeholder(_) => "placeholder",
            ImageJob::Palette(_) => "palette",
            ImageJob::Quality(_) => "quality",
            ImageJob::Srcset(_) => "srcset",
            ImageJob::Exif(_) => "exif",
            ImageJob::MarkdownToPdf(_) => "markdown_to_pdf",
            ImageJob::Video(v) => match v {
                VideoJob::Transcode(_) => "video_transcode",
                VideoJob::Resize(_) => "video_resize",
                VideoJob::Trim(_) => "video_trim",
                VideoJob::Thumbnail(_) => "video_thumbnail",
                VideoJob::Profile(_) => "video_profile",
                VideoJob::Metadata(_) => "video_metadata",
            },
            ImageJob::Audio(a) => match a {
                AudioJob::Transcode(_) => "audio_transcode",
                AudioJob::Trim(_) => "audio_trim",
                AudioJob::Normalize(_) => "audio_normalize",
                AudioJob::SilenceTrim(_) => "audio_silence_trim",
            },
            ImageJob::Batch(_) => "batch",
        }
        .to_string()
    }
}
