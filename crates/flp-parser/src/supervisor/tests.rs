use super::*;
use std::fs;
use std::sync::OnceLock;

fn fake_binary() -> &'static PathBuf {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let directory = std::env::temp_dir().join(format!(
            "fruitboard-supervisor-test-{}-{} space Ω",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let executable = directory.join(format!("child{}", std::env::consts::EXE_SUFFIX));
        let result = Command::new("rustc")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/supervisor_child.rs"))
            .args(["--edition=2024", "-C", "debuginfo=0", "-o"])
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        executable
    })
}

fn supervisor(mode: &str) -> ParserSupervisor {
    let mut supervisor = ParserSupervisor::new(
        fake_binary().clone(),
        SupervisorLimits {
            request_timeout: Duration::from_secs(2),
            max_requests_per_process: 10,
        },
    )
    .unwrap();
    supervisor.arguments.push(mode.into());
    supervisor
}

fn health(supervisor: &mut ParserSupervisor) -> Result<ProtocolReply, SupervisorError> {
    supervisor.request(ParserRequest::HealthCheck, &CancellationToken::default())
}

fn result(reply: ProtocolReply) -> Value {
    match reply {
        ProtocolReply::Result(value) => value,
        ProtocolReply::Rejected { .. } => panic!("expected result"),
    }
}

#[test]
fn healthy_process_is_reused_recycled_and_explicitly_reaped() {
    let mut supervisor = supervisor("healthy");
    supervisor.limits.max_requests_per_process = 2;
    let first = result(health(&mut supervisor).unwrap());
    let second = result(health(&mut supervisor).unwrap());
    assert_eq!(first["pid"], second["pid"]);
    let next = result(health(&mut supervisor).unwrap());
    assert_ne!(first["pid"], next["pid"]);
    supervisor.shutdown().unwrap();
    assert!(supervisor.running.is_none());
    supervisor.shutdown().unwrap();
    assert_eq!(result(health(&mut supervisor).unwrap())["status"], "ok");
}

#[test]
fn protocol_faults_retire_process_and_later_work_recovers_without_replay() {
    for (mode, expected) in [
        ("crash", SupervisorError::ProcessStopped),
        ("truncated", SupervisorError::ProcessStopped),
        ("oversized", SupervisorError::ResponseLimit),
        ("malformed", SupervisorError::InvalidProtocol),
        ("wrong-id", SupervisorError::InvalidProtocol),
        ("wrong-version", SupervisorError::InvalidProtocol),
        ("wrong-schema", SupervisorError::InvalidProtocol),
        ("ambiguous", SupervisorError::InvalidProtocol),
        ("missing-result", SupervisorError::InvalidProtocol),
        ("null-result", SupervisorError::InvalidProtocol),
        ("private-error", SupervisorError::InvalidProtocol),
    ] {
        let mut supervisor = supervisor(mode);
        assert!(
            matches!(health(&mut supervisor), Err(error) if error == expected),
            "{mode}"
        );
        assert!(supervisor.running.is_none(), "{mode}");
        assert_eq!(supervisor.sequence, 1, "failed requests are never replayed");
        supervisor.arguments = vec!["healthy".into()];
        assert_eq!(result(health(&mut supervisor).unwrap())["status"], "ok");
        assert_eq!(supervisor.sequence, 2);
    }
}

#[test]
fn valid_file_rejection_keeps_healthy_process_and_stderr_cannot_block_replies() {
    let mut supervisor = supervisor("rejection");
    assert!(
        matches!(health(&mut supervisor), Ok(ProtocolReply::Rejected { code }) if code == "INPUT_CHANGED")
    );
    let process = supervisor.running.as_ref().unwrap().child.id();
    assert!(matches!(
        health(&mut supervisor),
        Ok(ProtocolReply::Rejected { .. })
    ));
    assert_eq!(supervisor.running.as_ref().unwrap().child.id(), process);
    supervisor.shutdown().unwrap();
    supervisor.arguments = vec!["stderr-flood".into()];
    assert_eq!(result(health(&mut supervisor).unwrap())["status"], "ok");
}

#[test]
fn timeout_retires_even_a_process_that_never_reads_its_input() {
    let mut supervisor = supervisor("blocked-input");
    supervisor.limits.request_timeout = Duration::from_millis(150);
    let root = fake_binary().parent().unwrap().to_path_buf();
    let request = ParserRequest::Parse(ParseRequest {
        path: root.join("synthetic.flp"),
        expected: ExpectedFingerprint {
            size: 1,
            modified_at_ms: 1,
        },
        allowed_roots: vec![root.join("a".repeat(50_000))],
    });
    let started = Instant::now();
    assert!(matches!(
        supervisor.request(request, &CancellationToken::default()),
        Err(SupervisorError::TimedOut)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(supervisor.running.is_none());
}

#[test]
fn cancellation_retires_an_inflight_process_and_precancel_does_not_spawn() {
    let mut supervisor = supervisor("stall");
    let marker = fake_binary()
        .parent()
        .unwrap()
        .join("cancellation-request-observed");
    supervisor.arguments.push(marker.clone().into_os_string());
    let cancellation = CancellationToken::default();
    let observed_token = cancellation.clone();
    let signal = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() {
            assert!(
                Instant::now() < deadline,
                "child did not observe the request"
            );
            thread::sleep(Duration::from_millis(5));
        }
        observed_token.cancel();
        fs::remove_file(marker).unwrap();
    });
    assert!(matches!(
        supervisor.request(ParserRequest::HealthCheck, &cancellation),
        Err(SupervisorError::Cancelled)
    ));
    signal.join().unwrap();
    assert!(supervisor.running.is_none());
    assert!(matches!(
        supervisor.request(ParserRequest::HealthCheck, &cancellation),
        Err(SupervisorError::Cancelled)
    ));
    assert!(supervisor.running.is_none());
}

#[test]
fn drop_terminates_and_reaps_the_owned_child() {
    let mut supervisor = supervisor("healthy");
    health(&mut supervisor).unwrap();
    let pid = supervisor.running.as_ref().unwrap().child.id();
    drop(supervisor);
    #[cfg(target_os = "linux")]
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    #[cfg(windows)]
    {
        let output = Command::new("tasklist.exe")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let csv = String::from_utf8(output.stdout).unwrap();
        assert!(!csv.lines().any(|line| {
            line.split(',')
                .nth(1)
                .is_some_and(|value| value.trim_matches('"') == pid.to_string())
        }));
    }
}

#[test]
fn request_limits_invalid_paths_and_spawn_failures_are_fixed_errors() {
    assert!(matches!(
        ParserSupervisor::new(
            PathBuf::from("relative-parser"),
            SupervisorLimits::default()
        ),
        Err(SupervisorError::InvalidConfiguration)
    ));
    let mut supervisor = supervisor("healthy");
    let root = fake_binary().parent().unwrap().to_path_buf();
    let request = ParserRequest::Parse(ParseRequest {
        path: PathBuf::from("relative.flp"),
        expected: ExpectedFingerprint {
            size: 1,
            modified_at_ms: 1,
        },
        allowed_roots: vec![root.clone()],
    });
    assert!(matches!(
        supervisor.request(request, &CancellationToken::default()),
        Err(SupervisorError::InvalidRequest)
    ));
    let request = ParserRequest::Parse(ParseRequest {
        path: root.join("synthetic.flp"),
        expected: ExpectedFingerprint {
            size: 1,
            modified_at_ms: 1,
        },
        allowed_roots: vec![root.join("x".repeat(MAX_REQUEST_BYTES))],
    });
    assert!(matches!(
        supervisor.request(request, &CancellationToken::default()),
        Err(SupervisorError::RequestLimit)
    ));
    assert!(supervisor.running.is_none());
    let request = ParserRequest::Parse(ParseRequest {
        path: root.join("synthetic.flp"),
        expected: ExpectedFingerprint {
            size: 1,
            modified_at_ms: 1,
        },
        allowed_roots: vec![root.clone(); MAX_ALLOWED_ROOTS + 1],
    });
    assert!(matches!(
        supervisor.request(request, &CancellationToken::default()),
        Err(SupervisorError::RequestLimit)
    ));
    assert!(supervisor.running.is_none());
    let mut missing = ParserSupervisor::new(
        root.join("missing-private-executable"),
        SupervisorLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        health(&mut missing),
        Err(SupervisorError::StartFailed)
    ));
}
