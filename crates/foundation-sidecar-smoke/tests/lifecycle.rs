use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "Fruitboard Smoke 音 {} {sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("Unicode test directory should be created");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copied_probe(directory: &Path) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_BIN_EXE_fruitboard-sidecar-smoke"));
    let extension = source.extension().and_then(|value| value.to_str());
    let filename = match extension {
        Some(extension) => format!("sidecar probe 音.{extension}"),
        None => "sidecar probe 音".to_owned(),
    };
    let destination = directory.join(filename);
    fs::copy(&source, &destination).expect("probe should copy into Unicode path");
    destination
}

fn command(probe: &Path, mode: &str, path: &Path) -> Command {
    let mut command = Command::new(probe);
    command.args(["--mode", mode, "--probe-path"]);
    command.arg(path);
    command
}

#[test]
fn starts_and_responds_without_echoing_the_probe_path() {
    let directory = TestDirectory::new();
    let probe = copied_probe(&directory.0);
    let mut child = command(&probe, "respond", &directory.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("probe should start from a path containing spaces and Unicode");
    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(b"ping\n")
        .expect("ping should be written");
    let output = child.wait_with_output().expect("probe should finish");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "pong:path-accepted\n"
    );
    assert!(output.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&output.stdout).contains(&directory.0.to_string_lossy()[..]));
}

#[test]
fn reports_a_controlled_failure_without_crashing_the_parent() {
    let directory = TestDirectory::new();
    let probe = copied_probe(&directory.0);
    let output = command(&probe, "fail", &directory.0)
        .output()
        .expect("failure probe should run");

    assert_eq!(output.status.code(), Some(17));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "controlled_failure\n"
    );
}

#[test]
fn observes_a_timeout_and_terminates_the_child() {
    let directory = TestDirectory::new();
    let probe = copied_probe(&directory.0);
    let mut child = command(&probe, "wait", &directory.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("wait probe should start");
    let mut ready = String::new();
    BufReader::new(child.stdout.take().expect("stdout should be piped"))
        .read_line(&mut ready)
        .expect("ready marker should be readable");
    assert_eq!(ready, "ready:path-accepted\n");

    thread::sleep(Duration::from_millis(150));
    assert!(
        child
            .try_wait()
            .expect("child state should be readable")
            .is_none()
    );
    child.kill().expect("timed-out child should terminate");
    let status = child.wait().expect("terminated child should be reaped");
    assert!(!status.success());
}
