use fruitboard_storage::StartupView;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

const MODE_VARIABLE: &str = "FRUITBOARD_FOUNDATION_SMOKE_MODE";
const OUTPUT_VARIABLE: &str = "FRUITBOARD_FOUNDATION_SMOKE_OUTPUT";
const PROBE_NAME: &str = "fruitboard-sidecar-smoke";
const PROBE_PATH: &str = r"C:\Fruitboard Smoke\音";

/// Bounded per-stage sidecar budgets. The PowerShell harness enforces a
/// single outer deadline for the whole installed launch (see
/// `Invoke-AppSmokeMode`); that outer deadline must cover the sum below plus
/// Tauri startup, storage setup, evidence sync, and process exit. Keep the
/// packaging-policy budget test in sync when any value changes.
const RESPOND_TIMEOUT: Duration = Duration::from_secs(3);
const FAIL_TIMEOUT: Duration = Duration::from_secs(3);
const WAIT_READY_TIMEOUT: Duration = Duration::from_millis(250);
const WAIT_TERMINATE_TIMEOUT: Duration = Duration::from_secs(3);

/// Fixed progress-stage names only. The progress file carries stage names and
/// elapsed milliseconds; it never carries paths, arguments, output text, or
/// exit codes beyond the fixed evidence contract.
const STAGE_SCHEDULE_START: &str = "schedule_start";
const STAGE_RESPOND_DONE: &str = "sidecar_respond_done";
const STAGE_FAIL_DONE: &str = "sidecar_fail_done";
const STAGE_WAIT_READY_DONE: &str = "sidecar_wait_ready_done";
const STAGE_WAIT_TERMINATE_DONE: &str = "sidecar_wait_terminate_done";
const STAGE_EVIDENCE_WRITTEN: &str = "evidence_written";
const STAGE_EXIT_REQUESTED: &str = "exit_requested";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SmokeMode {
    Seed,
    Verify,
}

pub(crate) struct SmokeRequest {
    pub(crate) mode: SmokeMode,
    output: PathBuf,
}

impl SmokeRequest {
    pub(crate) fn from_environment() -> Option<Self> {
        let mode = match std::env::var(MODE_VARIABLE).ok()?.as_str() {
            "seed" => SmokeMode::Seed,
            "verify" => SmokeMode::Verify,
            _ => return None,
        };
        let output = PathBuf::from(std::env::var_os(OUTPUT_VARIABLE)?);
        if !output.is_absolute()
            || output.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return None;
        }
        Some(Self { mode, output })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageEvidence {
    startup_view_before: StartupView,
    startup_view_after: StartupView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SidecarEvidence {
    controlled_failure_exit_code: i32,
    failure_contained: bool,
    responded: bool,
    spaces_and_unicode_argument: bool,
    started: bool,
    terminated: bool,
    timeout_observed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TimingEvidence {
    respond_ms: u64,
    fail_ms: u64,
    wait_ready_ms: u64,
    wait_terminate_ms: u64,
    total_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SmokeEvidence {
    schema_version: u64,
    status: &'static str,
    storage: StorageEvidence,
    sidecar: SidecarEvidence,
    timing: TimingEvidence,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SmokeFailure {
    schema_version: u64,
    status: &'static str,
    code: &'static str,
}

struct ProcessEvents {
    stderr: Vec<String>,
    stdout: Vec<String>,
    terminated_code: Option<i32>,
}

fn normalized_event_line(bytes: Vec<u8>) -> Result<Option<String>, &'static str> {
    let line = String::from_utf8(bytes).map_err(|_| "sidecar_output_invalid")?;
    let line = line.trim_matches(['\r', '\n']);
    Ok((!line.is_empty()).then(|| line.to_owned()))
}

fn spawn(
    app: &AppHandle,
    mode: &str,
) -> Result<(tauri::async_runtime::Receiver<CommandEvent>, CommandChild), &'static str> {
    app.shell()
        .sidecar(PROBE_NAME)
        .map_err(|_| "sidecar_configuration_failed")?
        .args(["--mode", mode, "--probe-path", PROBE_PATH])
        .spawn()
        .map_err(|_| "sidecar_start_failed")
}

fn collect(
    receiver: &mut tauri::async_runtime::Receiver<CommandEvent>,
    timeout: Duration,
) -> Result<ProcessEvents, &'static str> {
    let deadline = Instant::now() + timeout;
    let mut events = ProcessEvents {
        stderr: Vec::new(),
        stdout: Vec::new(),
        terminated_code: None,
    };

    while Instant::now() < deadline {
        match receiver.try_recv() {
            Ok(CommandEvent::Stdout(bytes)) => {
                if let Some(line) = normalized_event_line(bytes)? {
                    events.stdout.push(line);
                }
            }
            Ok(CommandEvent::Stderr(bytes)) => {
                if let Some(line) = normalized_event_line(bytes)? {
                    events.stderr.push(line);
                }
            }
            Ok(CommandEvent::Terminated(payload)) => {
                events.terminated_code = payload.code;
                return Ok(events);
            }
            Ok(CommandEvent::Error(_)) => return Err("sidecar_runtime_failed"),
            Ok(_) => {}
            Err(_) => {
                if receiver.is_closed() {
                    return Err("sidecar_event_stream_closed");
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    Ok(events)
}

fn sidecar_evidence(
    app: &AppHandle,
    progress_output: &std::path::Path,
    schedule_start: &Instant,
) -> Result<(SidecarEvidence, TimingEvidence), &'static str> {
    let stage_start = Instant::now();
    let (mut response_events, mut response_child) = spawn(app, "respond")?;
    response_child
        .write(b"ping\n")
        .map_err(|_| "sidecar_input_failed")?;
    let response = collect(&mut response_events, RESPOND_TIMEOUT)?;
    if response.terminated_code != Some(0)
        || response.stdout != ["pong:path-accepted"]
        || !response.stderr.is_empty()
    {
        return Err("sidecar_response_failed");
    }
    let respond_ms = stage_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    record_stage(progress_output, schedule_start, STAGE_RESPOND_DONE);

    let stage_start = Instant::now();
    let (mut failure_events, _failure_child) = spawn(app, "fail")?;
    let failure = collect(&mut failure_events, FAIL_TIMEOUT)?;
    if failure.terminated_code != Some(17)
        || failure.stderr != ["controlled_failure"]
        || !failure.stdout.is_empty()
    {
        return Err("sidecar_failure_contract_failed");
    }
    let fail_ms = stage_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    record_stage(progress_output, schedule_start, STAGE_FAIL_DONE);

    let stage_start = Instant::now();
    let (mut wait_events, wait_child) = spawn(app, "wait")?;
    let waiting = collect(&mut wait_events, WAIT_READY_TIMEOUT)?;
    if waiting.terminated_code.is_some() || waiting.stdout != ["ready:path-accepted"] {
        return Err("sidecar_timeout_contract_failed");
    }
    let wait_ready_ms = stage_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    record_stage(progress_output, schedule_start, STAGE_WAIT_READY_DONE);
    wait_child
        .kill()
        .map_err(|_| "sidecar_termination_failed")?;
    let stage_start = Instant::now();
    let terminated = collect(&mut wait_events, WAIT_TERMINATE_TIMEOUT)?;
    if terminated.terminated_code.is_none() {
        return Err("sidecar_termination_unconfirmed");
    }
    let wait_terminate_ms = stage_start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    record_stage(progress_output, schedule_start, STAGE_WAIT_TERMINATE_DONE);

    Ok((
        SidecarEvidence {
            controlled_failure_exit_code: 17,
            failure_contained: true,
            responded: true,
            spaces_and_unicode_argument: true,
            started: true,
            terminated: true,
            timeout_observed: true,
        },
        TimingEvidence {
            respond_ms,
            fail_ms,
            wait_ready_ms,
            wait_terminate_ms,
            total_ms: 0,
        },
    ))
}

/// Sibling progress file for the evidence output (for example,
/// `seed.stages.jsonl` next to `seed.json`). Each line is one fixed stage
/// name plus elapsed milliseconds since schedule start. Best-effort only:
/// progress failures never change the smoke exit code or evidence assertions.
fn progress_path(output: &std::path::Path) -> PathBuf {
    output.with_extension("stages.jsonl")
}

fn record_stage(output: &std::path::Path, schedule_start: &Instant, stage: &str) {
    let elapsed_ms = schedule_start
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let line = serde_json::json!({"stage": stage, "elapsedMs": elapsed_ms});
    let mut line = serde_json::to_string(&line).unwrap_or_default();
    line.push('\n');
    let _ = OpenOptions::new()
        .create(true)
        .append(true)
        .open(progress_path(output))
        .and_then(|mut file| file.write_all(line.as_bytes()).and_then(|()| file.flush()));
}

fn write_json(path: &PathBuf, value: &impl Serialize) -> Result<(), &'static str> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "evidence_serialization_failed")?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|_| "evidence_create_failed")?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "evidence_write_failed")
}

pub(crate) fn schedule(
    app: AppHandle,
    request: SmokeRequest,
    storage: Result<(StartupView, StartupView), &'static str>,
) {
    tauri::async_runtime::spawn_blocking(move || {
        let schedule_start = Instant::now();
        record_stage(&request.output, &schedule_start, STAGE_SCHEDULE_START);
        let result = storage.and_then(|(startup_view_before, startup_view_after)| {
            let (sidecar, mut timing) = sidecar_evidence(&app, &request.output, &schedule_start)?;
            timing.total_ms = schedule_start
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let evidence = SmokeEvidence {
                schema_version: 1,
                status: "ok",
                storage: StorageEvidence {
                    startup_view_before,
                    startup_view_after,
                },
                sidecar,
                timing,
            };
            write_json(&request.output, &evidence)?;
            record_stage(&request.output, &schedule_start, STAGE_EVIDENCE_WRITTEN);
            Ok(())
        });

        let exit_code = match result {
            Ok(()) => 0,
            Err(code) => {
                let failure = SmokeFailure {
                    schema_version: 1,
                    status: "error",
                    code,
                };
                let _ = write_json(&request.output, &failure);
                1
            }
        };
        record_stage(&request.output, &schedule_start, STAGE_EXIT_REQUESTED);
        app.exit(exit_code);
    });
}

#[cfg(test)]
mod tests {
    use super::{normalized_event_line, progress_path};
    use std::path::PathBuf;

    #[test]
    fn normalizes_windows_and_unix_process_line_framing() {
        assert_eq!(
            normalized_event_line(b"pong:path-accepted\r".to_vec()).unwrap(),
            Some("pong:path-accepted".to_owned())
        );
        assert_eq!(
            normalized_event_line(b"pong:path-accepted\n".to_vec()).unwrap(),
            Some("pong:path-accepted".to_owned())
        );
        assert_eq!(normalized_event_line(b"\n".to_vec()).unwrap(), None);
    }

    #[test]
    fn progress_file_is_a_bounded_sibling_without_paths() {
        let output = PathBuf::from(r"C:\smoke\raw\seed.json");
        let progress = progress_path(&output);
        assert_eq!(progress, PathBuf::from(r"C:\smoke\raw\seed.stages.jsonl"));
        assert_ne!(progress, output);
    }
}
