#![allow(clippy::unwrap_used, clippy::expect_used)]
//! SC-09: a reader that closes the pipe early is a clean exit, not a panic.
use std::process::{Command, Stdio};

fn closed_pipe(args: &[&str]) {
    let db = tempfile::TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mabi-ah"))
        .env_remove("NEXON_OPEN_API_KEY")
        .arg("--db")
        .arg(db.path().join("t.db"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Drop the read end before the child has started writing.
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "args {args:?}, stderr: {stderr}"
    );
    assert!(!stderr.contains("panicked"), "stderr: {stderr}");
    assert!(!stderr.contains("Broken pipe"), "stderr: {stderr}");
}

#[test]
fn categories_survives_closed_stdout() {
    closed_pipe(&["categories"]);
}

#[test]
fn table_format_survives_closed_stdout() {
    closed_pipe(&["--format", "table", "categories"]);
}
