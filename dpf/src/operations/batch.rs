use serde::{Deserialize, Serialize};

use crate::ImageJob;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BatchJob {
    /// Lista de trabajos a ejecutar en paralelo
    pub jobs: Vec<ImageJob>,
    /// Máximo de hilos para este batch (default: todos los disponibles)
    pub max_threads: Option<usize>,
}
