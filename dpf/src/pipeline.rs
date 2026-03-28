use anyhow::Result;
use rayon::prelude::*;

use crate::operations::{
    adjust, audio, convert, crop, exif_ops, favicon, optimize, palette, placeholder, quality,
    resize, rotate, sprite, srcset, video, watermark,
};
use crate::{AudioJob, ImageJob, JobResult, VideoJob};

pub use crate::operations;

/// Ejecuta un solo trabajo de imagen.
pub fn execute(job: ImageJob) -> Result<JobResult> {
    match job {
        ImageJob::Resize(params) => resize::execute(params),
        ImageJob::Optimize(params) => optimize::execute(params),
        ImageJob::Convert(params) => convert::execute(params),
        ImageJob::Crop(params) => crop::execute(params),
        ImageJob::Rotate(params) => rotate::execute(params),
        ImageJob::Watermark(params) => watermark::execute(params),
        ImageJob::Adjust(params) => adjust::execute(params),
        ImageJob::Favicon(params) => favicon::execute(params),
        ImageJob::Sprite(params) => sprite::execute(params),
        ImageJob::Placeholder(params) => placeholder::execute(params),
        ImageJob::Palette(params) => palette::execute(params),
        ImageJob::Quality(params) => quality::execute(params),
        ImageJob::Srcset(params) => srcset::execute(params),
        ImageJob::Exif(params) => exif_ops::execute(params),
        ImageJob::Video(vjob) => execute_video_job(vjob),
        ImageJob::Audio(ajob) => execute_audio_job(ajob),
        ImageJob::Batch(batch) => {
            // Ejecutar batch como paralelo
            let results = run_parallel(batch.jobs);
            let total_outputs: Vec<_> = results.iter().flat_map(|r| r.outputs.clone()).collect();

            Ok(JobResult {
                success: true,
                operation: "batch".into(),
                outputs: total_outputs,
                elapsed_ms: 0,
                metadata: Some(serde_json::json!({
                    "jobs_total": results.len(),
                    "jobs_succeeded": results.iter().filter(|r| r.success).count(),
                })),
            })
        }
    }
}

/// Execute a video job based on its operation type.
fn execute_video_job(job: VideoJob) -> Result<JobResult> {
    match job {
        VideoJob::Transcode(params) => video::transcode::execute(params),
        VideoJob::Resize(params) => video::resize::execute(params),
        VideoJob::Trim(params) => video::trim::execute(params),
        VideoJob::Thumbnail(params) => video::thumbnail::execute(params),
        VideoJob::Profile(params) => video::profiles::execute(params),
        VideoJob::Metadata(params) => video::metadata::execute(params),
    }
}

/// Execute an audio job based on its operation type.
fn execute_audio_job(job: AudioJob) -> Result<JobResult> {
    match job {
        AudioJob::Transcode(params) => audio::transcode::execute(params),
        AudioJob::Trim(params) => audio::trim::execute(params),
        AudioJob::Normalize(params) => audio::normalize::execute(params),
        AudioJob::SilenceTrim(params) => audio::silence_trim::execute(params),
    }
}

/// Ejecuta múltiples trabajos en paralelo usando rayon.
pub fn run_parallel(jobs: Vec<ImageJob>) -> Vec<JobResult> {
    jobs.into_par_iter()
        .map(|job| {
            let op_name = job.operation_name();
            let start = std::time::Instant::now();
            match execute(job) {
                Ok(mut result) => {
                    result.elapsed_ms = start.elapsed().as_millis() as u64;
                    result
                }
                Err(e) => {
                    // Convertir error a JobResult fallido para no romper el batch
                    JobResult {
                        success: false,
                        operation: op_name,
                        outputs: vec![],
                        elapsed_ms: start.elapsed().as_millis() as u64,
                        metadata: Some(serde_json::json!({
                            "error": e.to_string(),
                        })),
                    }
                }
            }
        })
        .collect()
}
