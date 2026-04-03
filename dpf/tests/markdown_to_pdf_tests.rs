use base64::Engine;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
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
    let binary = binary_path();
    if !std::path::Path::new(&binary).exists() {
        let output = Command::new("cargo")
            .args(["build"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("Failed to build binary");

        assert!(
            output.status.success(),
            "Failed to build binary: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn caps_reports_markdown_to_pdf_support() {
    build_binary();

    let output = Command::new(binary_path())
        .arg("caps")
        .output()
        .expect("Failed to execute caps command");

    assert!(output.status.success());

    let caps: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Caps output should be valid JSON");

    let operations = caps["operations"]
        .as_array()
        .expect("operations should be an array");
    assert!(operations.iter().any(|value| value == "markdown_to_pdf"));

    let document_formats = caps["output_formats"]["document"]
        .as_array()
        .expect("document output formats should be an array");
    assert!(document_formats.iter().any(|value| value == "pdf"));
}

#[test]
fn process_markdown_file_to_pdf_output() {
    build_binary();

    let temp_dir = TempDir::new().expect("temp dir should be created");
    let output_path = temp_dir.path().join("report.pdf");

    let job_json = json!({
        "operation": "markdown_to_pdf",
        "input": fixture_path("sample.md"),
        "output": output_path,
        "page_size": "letter",
        "layout_mode": "paged",
        "theme": "engineering"
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute markdown_to_pdf process command");

    assert!(output.status.success());

    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "markdown_to_pdf");
    assert_eq!(result["outputs"][0]["format"], "pdf");
    assert_eq!(result["outputs"][0]["width"], 0);
    assert_eq!(result["outputs"][0]["height"], 0);
    assert_eq!(result["metadata"]["backend"], "typst");
    assert!(output_path.exists());

    let bytes = std::fs::read(output_path).expect("PDF should exist");
    assert!(bytes.starts_with(b"%PDF"));
}

#[test]
fn process_inline_markdown_to_inline_pdf() {
    build_binary();

    let job_json = json!({
        "operation": "markdown_to_pdf",
        "markdown_text": "# Inline Report\n\nHello from memory.",
        "inline": true,
        "theme": "professional"
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute inline markdown_to_pdf process command");

    assert!(output.status.success());

    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], true);
    assert_eq!(result["outputs"][0]["format"], "pdf");

    let data_base64 = result["outputs"][0]["data_base64"]
        .as_str()
        .expect("inline output should include data_base64");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(data_base64)
        .expect("inline PDF should decode from base64");
    assert!(decoded.starts_with(b"%PDF"));
}

#[test]
fn batch_accepts_markdown_to_pdf_jobs() {
    build_binary();

    let temp_dir = TempDir::new().expect("temp dir should be created");
    let batch_file = temp_dir.path().join("jobs.json");
    let output_path = temp_dir.path().join("batch-report.pdf");
    let jobs = json!([
        {
            "operation": "markdown_to_pdf",
            "input": fixture_path("sample.md"),
            "output": output_path
        }
    ]);
    std::fs::write(&batch_file, jobs.to_string()).expect("batch file should be written");

    let output = Command::new(binary_path())
        .args(["batch", "--file", batch_file.to_str().expect("utf-8 path")])
        .output()
        .expect("Failed to execute batch command");

    assert!(output.status.success());

    let results: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Batch output should be valid JSON");
    assert_eq!(results[0]["success"], true);
    assert_eq!(results[0]["operation"], "markdown_to_pdf");
    assert_eq!(results[0]["outputs"][0]["format"], "pdf");
}

#[test]
fn stream_mode_accepts_markdown_to_pdf_jobs() {
    build_binary();

    let mut child = Command::new(binary_path())
        .arg("--stream")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start stream process");

    let stdin = child.stdin.as_mut().expect("stdin should be available");
    writeln!(
        stdin,
        "{}",
        json!({
            "operation": "markdown_to_pdf",
            "markdown_text": "# Streamed PDF\n\nGenerated through stream mode.",
            "inline": true
        })
    )
    .expect("stream job should be written");
    drop(child.stdin.take());

    let stdout = child.stdout.take().expect("stdout should be available");
    let mut lines = BufReader::new(stdout).lines();
    let line = lines
        .next()
        .expect("one response line expected")
        .expect("response line should be readable");
    let status = child.wait().expect("process should exit cleanly");
    assert!(status.success());

    let result: serde_json::Value =
        serde_json::from_str(&line).expect("Stream output should be valid JSON");
    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "markdown_to_pdf");
}

#[test]
fn reject_multiple_markdown_input_sources() {
    build_binary();

    let job_json = json!({
        "operation": "markdown_to_pdf",
        "input": fixture_path("sample.md"),
        "markdown_text": "# Duplicate input",
        "inline": true
    });

    let output = Command::new(binary_path())
        .args(["process", "--job", &job_json.to_string()])
        .output()
        .expect("Failed to execute invalid markdown_to_pdf process command");

    assert!(output.status.success());

    let result: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("Output should be valid JSON");

    assert_eq!(result["success"], false);
    assert!(result["error"]
        .as_str()
        .expect("error message should exist")
        .contains("exactly one markdown input source"));
}
