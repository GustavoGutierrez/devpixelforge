//! DevForge Image Processor — Motor de procesamiento de imágenes de alto rendimiento.
//!
//! Diseñado para ser invocado desde Go (o cualquier proceso padre) vía stdin/stdout JSON.
//! Soporta dos modos:
//!   1. CLI con argumentos (para invocaciones simples)
//!   2. Modo streaming por stdin (para batch y pipeline)
//!
//! Autor: Ing. Gustavo Gutiérrez

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

mod operations;
mod pipeline;

use operations::{
    favicon::FaviconParams,
    optimize::OptimizeParams,
    resize::ResizeParams,
    convert::ConvertParams,
    sprite::SpriteParams,
    placeholder::PlaceholderParams,
    batch::BatchJob,
};

// ─── CLI Interface ───────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "devforge-imgproc", version, about = "Image processing engine for DevForge MCP")]
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
    /// Generar favicon multi-tamaño desde imagen o SVG
    Favicon(FaviconParams),
    /// Generar sprite sheet desde múltiples imágenes
    Sprite(SpriteParams),
    /// Generar placeholder (blur hash, dominant color, LQIP)
    Placeholder(PlaceholderParams),
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
            let jobs: Vec<ImageJob> = serde_json::from_str(&content)
                .context("Invalid batch JSON")?;
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

        let result = process_job_json(trimmed)
            .unwrap_or_else(|e| {
                serde_json::to_string(&JobError {
                    success: false,
                    operation: "unknown".into(),
                    error: e.to_string(),
                    elapsed_ms: 0,
                }).unwrap_or_default()
            });

        writeln!(out, "{}", result)?;
        out.flush()?;
    }

    Ok(())
}

/// Procesa un JSON de trabajo y devuelve el resultado como JSON string.
fn process_job_json(json: &str) -> Result<String> {
    let job: ImageJob = serde_json::from_str(json)
        .context("Invalid job JSON")?;

    let start = Instant::now();
    let op_name = job.operation_name();

    match pipeline::execute(job) {
        Ok(mut result) => {
            result.elapsed_ms = start.elapsed().as_millis() as u64;
            result.operation = op_name;
            Ok(serde_json::to_string(&result)?)
        }
        Err(e) => {
            Ok(serde_json::to_string(&JobError {
                success: false,
                operation: op_name,
                error: e.to_string(),
                elapsed_ms: start.elapsed().as_millis() as u64,
            })?)
        }
    }
}

fn print_capabilities() {
    let caps = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "operations": [
            "resize", "optimize", "convert", "favicon",
            "sprite", "placeholder", "batch"
        ],
        "input_formats": ["png", "jpeg", "gif", "webp", "bmp", "tiff", "svg", "ico"],
        "output_formats": ["png", "jpeg", "webp", "ico", "avif"],
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
            "sprite_generation": true
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
            ImageJob::Favicon(_) => "favicon",
            ImageJob::Sprite(_) => "sprite",
            ImageJob::Placeholder(_) => "placeholder",
            ImageJob::Batch(_) => "batch",
        }.to_string()
    }
}
