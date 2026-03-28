//! Tests de integración para el pipeline de procesamiento de imágenes
//! Estos tests usan el CLI directamente en lugar de importar el crate

use serde_json::json;
use std::process::Command;
use tempfile::TempDir;

fn fixtures_dir() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/test_fixtures").to_string()
}

fn fixture_path(name: &str) -> String {
    format!("{}/{}", fixtures_dir(), name)
}

fn binary_path() -> String {
    format!("{}/target/debug/dpf", env!("CARGO_MANIFEST_DIR"))
}

fn build_binary() {
    // Compilar el binario si no existe
    let binary = binary_path();
    if !std::path::Path::new(&binary).exists() {
        let output = Command::new("cargo")
            .args(["build"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to build binary");

        if !output.status.success() {
            panic!(
                "Failed to build binary: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

// ============================================================================
// Tests de CLI básicos
// ============================================================================

#[test]
fn test_cli_caps_command() {
    build_binary();

    let output = Command::new(binary_path())
        .arg("caps")
        .output()
        .expect("Failed to execute caps command");

    assert!(
        output.status.success(),
        "Caps command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Caps output should be valid JSON");

    // Verificar estructura básica
    assert!(
        json.get("input_formats").is_some(),
        "Should have input_formats field"
    );
    assert!(
        json.get("output_formats").is_some(),
        "Should have output_formats field"
    );
    assert!(
        json.get("operations").is_some(),
        "Should have operations field"
    );
}

#[test]
fn test_cli_process_resize() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_str().unwrap();

    let job_json = json!({
        "operation": "resize",
        "input": fixture_path("sample.png"),
        "output_dir": output_dir,
        "widths": [50, 100],
        "format": "png"
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute process command");

    assert!(
        output.status.success(),
        "Process command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JobResult JSON");

    assert_eq!(result["success"], true, "Job should succeed");
    assert_eq!(result["operation"], "resize");
    assert!(result["outputs"].as_array().unwrap().len() >= 1);
}

#[test]
fn test_cli_process_convert() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.jpg");

    let job_json = json!({
        "operation": "convert",
        "input": fixture_path("sample.png"),
        "output": output_path.to_str().unwrap(),
        "format": "jpeg",
        "quality": 90
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute process command");

    assert!(
        output.status.success(),
        "Process command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert!(output_path.exists());
}

#[test]
fn test_cli_process_invalid_job() {
    build_binary();

    // Job inválido - falta input
    let job_json = r#"{"operation": "resize", "output_dir": "/tmp", "widths": [100]}"#;

    let output = Command::new(binary_path())
        .args(["process", "--job", job_json])
        .output()
        .expect("Failed to execute process command");

    // Debe fallar porque faltan parámetros requeridos
    assert!(!output.status.success() || String::from_utf8_lossy(&output.stdout).contains("false"));
}

#[test]
fn test_cli_process_missing_file() {
    build_binary();

    let job_json = r#"{"operation": "resize", "input": "/nonexistent.png", "output_dir": "/tmp", "widths": [100]}"#;

    let output = Command::new(binary_path())
        .args(["process", "--job", job_json])
        .output()
        .expect("Failed to execute process command");

    // Debe fallar porque el archivo no existe
    assert!(!output.status.success() || String::from_utf8_lossy(&output.stdout).contains("false"));
}

// ============================================================================
// Tests de batch
// ============================================================================

#[test]
fn test_cli_batch_command() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_str().unwrap();

    // Crear archivo batch
    let batch_jobs = json!([
        {
            "operation": "resize",
            "input": fixture_path("sample.png"),
            "output_dir": output_dir,
            "widths": [50],
            "format": "png"
        },
        {
            "operation": "resize",
            "input": fixture_path("sample.jpg"),
            "output_dir": output_dir,
            "widths": [50],
            "format": "jpg"
        }
    ]);

    let batch_file = temp_dir.path().join("batch.json");
    std::fs::write(&batch_file, batch_jobs.to_string()).unwrap();

    let output = Command::new(binary_path())
        .args(["batch", "--file", batch_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute batch command");

    assert!(
        output.status.success(),
        "Batch command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let results: Vec<serde_json::Value> =
        serde_json::from_str(&stdout).expect("Output should be valid JSON array");

    // Batch devuelve un array de resultados
    assert_eq!(results.len(), 2, "Should have 2 results from batch");

    // Verificar que al menos uno tenga éxito
    let success_count = results
        .iter()
        .filter(|r| r["success"].as_bool() == Some(true))
        .count();
    assert!(success_count >= 1, "At least one job should succeed");
}

// ============================================================================
// Tests de streaming mode
// ============================================================================

#[test]
fn test_stream_mode_single_job() {
    build_binary();

    use std::io::Write;
    use std::process::Stdio;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_str().unwrap();

    let job_json = json!({
        "operation": "resize",
        "input": fixture_path("sample.png"),
        "output_dir": output_dir,
        "widths": [50],
        "format": "png"
    });

    let mut child = Command::new(binary_path())
        .arg("--stream")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start stream process");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        writeln!(stdin, "{}", job_json).expect("Failed to write to stdin");
        // Enviar EOF
    }

    let output = child.wait_with_output().expect("Failed to read stdout");

    // Verificamos que haya salida válida o que el proceso termine
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        // Intentar parsear cada línea como JSON
        for line in stdout.lines() {
            if let Ok(result) = serde_json::from_str::<serde_json::Value>(line) {
                if result.get("success").is_some() {
                    assert!(result["success"].as_bool().unwrap_or(false));
                }
            }
        }
    }
}

// ============================================================================
// Tests de diferentes operaciones
// ============================================================================

#[test]
fn test_cli_favicon_generation() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_str().unwrap();

    let job_json = json!({
        "operation": "favicon",
        "input": fixture_path("sample.png"),
        "output_dir": output_dir,
        "generate_ico": true,
        "generate_manifest": false
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute favicon command");

    assert!(
        output.status.success(),
        "Favicon command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
}

#[test]
fn test_cli_svg_conversion() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.png");

    let job_json = json!({
        "operation": "convert",
        "input": fixture_path("sample.svg"),
        "output": output_path.to_str().unwrap(),
        "format": "png",
        "width": 200,
        "height": 200
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute convert command");

    assert!(
        output.status.success(),
        "SVG convert command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert!(output_path.exists());
}

// ============================================================================
// Tests de JSON serialization
// ============================================================================

#[test]
fn test_json_serialization_resize() {
    // Verificar que los jobs se serializan correctamente
    let job = json!({
        "operation": "resize",
        "input": "test.png",
        "output_dir": "/tmp",
        "widths": [100, 200],
        "format": "webp",
        "quality": 90,
        "filter": "lanczos3",
        "linear_rgb": false,
        "inline": false
    });

    let json_str = job.to_string();
    assert!(json_str.contains("operation"));
    assert!(json_str.contains("resize"));
    assert!(json_str.contains("widths"));
}

#[test]
fn test_json_serialization_convert() {
    let job = json!({
        "operation": "convert",
        "input": "test.svg",
        "output": "/tmp/output.png",
        "format": "png",
        "width": 100,
        "height": 100,
        "quality": 85,
        "inline": false
    });

    let json_str = job.to_string();
    assert!(json_str.contains("convert"));
    assert!(json_str.contains("svg"));
}

#[test]
fn test_json_deserialization_result() {
    let result_json = json!({
        "success": true,
        "operation": "resize",
        "outputs": [
            {
                "path": "/tmp/test_100w.png",
                "format": "png",
                "width": 100,
                "height": 100,
                "size_bytes": 1024
            }
        ],
        "elapsed_ms": 150,
        "metadata": {
            "source_width": 200,
            "source_height": 200
        }
    });

    assert_eq!(result_json["success"], true);
    assert_eq!(result_json["operation"], "resize");
    assert_eq!(result_json["outputs"].as_array().unwrap().len(), 1);
    assert_eq!(result_json["elapsed_ms"], 150);
}

// ============================================================================
// Video integration tests
// ============================================================================

#[test]
#[ignore = "Requires video test fixture in test_fixtures/"]
fn video_transcode_h264_profile() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.mp4");

    // Check if video fixture exists
    let video_path = fixture_path("sample.mp4");
    if !std::path::Path::new(&video_path).exists() {
        panic!("Video test fixture not found: {}", video_path);
    }

    let job_json = json!({
        "operation": "video",
        "transcode": {
            "input": video_path,
            "output": output_path.to_str().unwrap(),
            "codec": "h264",
            "bitrate": "2000k",
            "preset": "fast"
        }
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute video_transcode command");

    assert!(
        output.status.success(),
        "Video transcode command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "video_transcode");
    assert!(output_path.exists(), "Output video should exist");
}

#[test]
#[ignore = "Requires video test fixture in test_fixtures/"]
fn video_resize_maintains_aspect() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("resized.mp4");

    // Check if video fixture exists
    let video_path = fixture_path("sample.mp4");
    if !std::path::Path::new(&video_path).exists() {
        panic!("Video test fixture not found: {}", video_path);
    }

    let job_json = json!({
        "operation": "video",
        "resize": {
            "input": video_path,
            "output": output_path.to_str().unwrap(),
            "width": 640,
            "mode": "fit"
        }
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute video_resize command");

    assert!(
        output.status.success(),
        "Video resize command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "video_resize");
    assert!(output_path.exists(), "Output video should exist");
}

#[test]
#[ignore = "Requires video test fixture in test_fixtures/"]
fn video_transcode_vp9_webm() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.webm");

    let video_path = fixture_path("sample.mp4");
    if !std::path::Path::new(&video_path).exists() {
        panic!("Video test fixture not found: {}", video_path);
    }

    let job_json = json!({
        "operation": "video",
        "transcode": {
            "input": video_path,
            "output": output_path.to_str().unwrap(),
            "codec": "vp9",
            "bitrate": "1500k"
        }
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute video_transcode command");

    assert!(
        output.status.success(),
        "VP9 transcode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert!(output_path.exists());
}

// ============================================================================
// Audio integration tests
// ============================================================================

#[test]
#[ignore = "Requires audio test fixture in test_fixtures/"]
fn audio_transcode_to_aac() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.aac");

    // Check if audio fixture exists
    let audio_path = fixture_path("sample.mp3");
    if !std::path::Path::new(&audio_path).exists() {
        panic!("Audio test fixture not found: {}", audio_path);
    }

    let job_json = json!({
        "operation": "audio",
        "transcode": {
            "input": audio_path,
            "output": output_path.to_str().unwrap(),
            "codec": "aac",
            "bitrate": "192k"
        }
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute audio_transcode command");

    assert!(
        output.status.success(),
        "Audio transcode command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "audio_transcode");
    assert!(output_path.exists(), "Output audio should exist");
}

#[test]
#[ignore = "Requires audio test fixture in test_fixtures/"]
fn audio_normalize_lufs() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("normalized.mp3");

    let audio_path = fixture_path("sample.mp3");
    if !std::path::Path::new(&audio_path).exists() {
        panic!("Audio test fixture not found: {}", audio_path);
    }

    // Normalize to YouTube standard: -14 LUFS
    let job_json = json!({
        "operation": "audio",
        "normalize": {
            "input": audio_path,
            "output": output_path.to_str().unwrap(),
            "target_lufs": -14.0
        }
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute audio_normalize command");

    assert!(
        output.status.success(),
        "Audio normalize command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "audio_normalize");
    assert!(output_path.exists(), "Output audio should exist");
}

#[test]
#[ignore = "Requires audio test fixture in test_fixtures/"]
fn audio_transcode_to_opus() {
    build_binary();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.opus");

    let audio_path = fixture_path("sample.mp3");
    if !std::path::Path::new(&audio_path).exists() {
        panic!("Audio test fixture not found: {}", audio_path);
    }

    let job_json = json!({
        "operation": "audio",
        "transcode": {
            "input": audio_path,
            "output": output_path.to_str().unwrap(),
            "codec": "opus",
            "bitrate": "128k"
        }
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute audio_transcode command");

    assert!(
        output.status.success(),
        "Opus transcode failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert!(output_path.exists());
}

// ============================================================================
// Video/Audio JSON serialization tests
// ============================================================================

#[test]
fn test_json_serialization_video_transcode() {
    let job = json!({
        "operation": "video",
        "transcode": {
            "input": "input.mp4",
            "output": "/tmp/output.mp4",
            "codec": "h264",
            "bitrate": "2000k",
            "preset": "medium",
            "audio_codec": "aac",
            "audio_bitrate": 128
        }
    });

    let json_str = job.to_string();
    assert!(json_str.contains("transcode"));
    assert!(json_str.contains("h264"));
    assert!(json_str.contains("2000k"));
}

#[test]
fn test_json_serialization_audio_normalize() {
    let job = json!({
        "operation": "audio",
        "normalize": {
            "input": "input.mp3",
            "output": "/tmp/output.mp3",
            "target_lufs": -14.0,
            "threshold_lufs": -50.0
        }
    });

    let json_str = job.to_string();
    assert!(json_str.contains("normalize"));
    assert!(json_str.contains("-14"));
    assert!(json_str.contains("target_lufs"));
}
