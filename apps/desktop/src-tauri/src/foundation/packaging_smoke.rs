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
struct SmokeEvidence {
    schema_version: u64,
    status: &'static str,
    storage: StorageEvidence,
    sidecar: SidecarEvidence,
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

fn sidecar_evidence(app: &AppHandle) -> Result<SidecarEvidence, &'static str> {
    let (mut response_events, mut response_child) = spawn(app, "respond")?;
    response_child
        .write(b"ping\n")
        .map_err(|_| "sidecar_input_failed")?;
    let response = collect(&mut response_events, Duration::from_secs(3))?;
    if response.terminated_code != Some(0)
        || response.stdout != ["pong:path-accepted"]
        || !response.stderr.is_empty()
    {
        return Err("sidecar_response_failed");
    }

    let (mut failure_events, _failure_child) = spawn(app, "fail")?;
    let failure = collect(&mut failure_events, Duration::from_secs(3))?;
    if failure.terminated_code != Some(17)
        || failure.stderr != ["controlled_failure"]
        || !failure.stdout.is_empty()
    {
        return Err("sidecar_failure_contract_failed");
    }

    let (mut wait_events, wait_child) = spawn(app, "wait")?;
    let waiting = collect(&mut wait_events, Duration::from_millis(250))?;
    if waiting.terminated_code.is_some() || waiting.stdout != ["ready:path-accepted"] {
        return Err("sidecar_timeout_contract_failed");
    }
    wait_child
        .kill()
        .map_err(|_| "sidecar_termination_failed")?;
    let terminated = collect(&mut wait_events, Duration::from_secs(3))?;
    if terminated.terminated_code.is_none() {
        return Err("sidecar_termination_unconfirmed");
    }

    Ok(SidecarEvidence {
        controlled_failure_exit_code: 17,
        failure_contained: true,
        responded: true,
        spaces_and_unicode_argument: true,
        started: true,
        terminated: true,
        timeout_observed: true,
    })
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
        let result = storage.and_then(|(startup_view_before, startup_view_after)| {
            let evidence = SmokeEvidence {
                schema_version: 1,
                status: "ok",
                storage: StorageEvidence {
                    startup_view_before,
                    startup_view_after,
                },
                sidecar: sidecar_evidence(&app)?,
            };
            write_json(&request.output, &evidence)
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
        app.exit(exit_code);
    });
}

#[cfg(test)]
mod tests {
    use super::normalized_event_line;

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
}
