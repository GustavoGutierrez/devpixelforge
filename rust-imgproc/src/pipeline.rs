use anyhow::Result;
use rayon::prelude::*;

use crate::{ImageJob, JobResult};
use crate::operations::{resize, optimize, convert, favicon, sprite, placeholder};

/// Ejecuta un solo trabajo de imagen.
pub fn execute(job: ImageJob) -> Result<JobResult> {
    match job {
        ImageJob::Resize(params) => resize::execute(params),
        ImageJob::Optimize(params) => optimize::execute(params),
        ImageJob::Convert(params) => convert::execute(params),
        ImageJob::Favicon(params) => favicon::execute(params),
        ImageJob::Sprite(params) => sprite::execute(params),
        ImageJob::Placeholder(params) => placeholder::execute(params),
        ImageJob::Batch(batch) => {
            // Ejecutar batch como paralelo
            let results = run_parallel(batch.jobs);
            let total_outputs: Vec<_> = results.iter()
                .flat_map(|r| r.outputs.clone())
                .collect();

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
