//! Benchmark driver for the integrated Phase 2 scanner (#41 / P2-11).
//!
//! Drives the REAL production stack against a fixture root: the handle-bound
//! Windows [`WindowsFilesystemPort`], a durable [`Database`] in a
//! harness-supplied app-data directory, and [`ScanWorker`] with the system
//! clock (`start_session`, `request_manual_scan`, `poll` until terminal).
//! There are no shortcuts: enumerate + stage + publish run through the
//! worker's full production path, including lease renewal, batch fencing and
//! the fenced atomic publication.
//!
//! The driver emits typed JSON lines on stdout for
//! `scripts/run-benchmark.mjs`:
//!
//! - `{"phase":"ready", ...}` once the database, session and root exist;
//! - `{"phase":"scan_started", ...}` immediately before the manual scan is
//!   requested (this is the start of the measured operation);
//! - `{"phase":"cancellation_requested", ...}` only with `--cancel-after-ms`,
//!   when the cooperative cancellation token fires;
//! - `{"phase":"scan_finished", ...}` with the terminal status, outcome and
//!   counters, or `{"phase":"error", ...}` when the run could not reach a
//!   terminal execution.
//!
//! This driver never prints paths, usernames or machine identities: the
//! harness records fixture identity by seed and manifest hash only.
use fruitboard_filesystem_enumeration::{CancellationToken, WindowsFilesystemPort};
use fruitboard_scan_execution::{
    ScanExecution, ScanExecutionStatus, ScanWorker, SystemClock, WorkerConfig,
};
use fruitboard_storage::Database;
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum Field<'a> {
    Text(&'a str),
    Signed(i64),
    Count(usize),
    Flag(bool),
    Null,
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
    out
}

fn emit(phase: &str, elapsed_ms: i64, fields: &[(&str, Field<'_>)]) {
    let mut line = String::from("{\"phase\":");
    line.push_str(&json_string(phase));
    line.push_str(",\"elapsed_ms\":");
    line.push_str(&elapsed_ms.to_string());
    for (name, field) in fields {
        line.push_str(",\"");
        line.push_str(name);
        line.push_str("\":");
        match field {
            Field::Text(value) => line.push_str(&json_string(value)),
            Field::Signed(value) => line.push_str(&value.to_string()),
            Field::Count(value) => line.push_str(&value.to_string()),
            Field::Flag(value) => line.push_str(if *value { "true" } else { "false" }),
            Field::Null => line.push_str("null"),
        }
    }
    line.push('}');
    println!("{line}");
}

fn outcome_name(outcome: Option<&fruitboard_filesystem_enumeration::Outcome>) -> Field<'_> {
    match outcome {
        None => Field::Null,
        Some(outcome) => Field::Text(match outcome {
            fruitboard_filesystem_enumeration::Outcome::Complete => "Complete",
            fruitboard_filesystem_enumeration::Outcome::Partial => "Partial",
            fruitboard_filesystem_enumeration::Outcome::Denied => "Denied",
            fruitboard_filesystem_enumeration::Outcome::RootUnavailable => "RootUnavailable",
            fruitboard_filesystem_enumeration::Outcome::Cancelled => "Cancelled",
            fruitboard_filesystem_enumeration::Outcome::ResourceLimit => "ResourceLimit",
            fruitboard_filesystem_enumeration::Outcome::SinkFailed => "SinkFailed",
            fruitboard_filesystem_enumeration::Outcome::Invalid => "Invalid",
            fruitboard_filesystem_enumeration::Outcome::UnsupportedFilesystem => {
                "UnsupportedFilesystem"
            }
        }),
    }
}

fn status_name(status: ScanExecutionStatus) -> &'static str {
    match status {
        ScanExecutionStatus::Published => "Published",
        ScanExecutionStatus::Failed => "Failed",
        ScanExecutionStatus::Cancelled => "Cancelled",
        ScanExecutionStatus::Interrupted => "Interrupted",
        ScanExecutionStatus::Fenced => "Fenced",
    }
}

fn usage() -> &'static str {
    "Usage: benchmark --root <fixture-root> --db <app-data-dir> [--cancel-after-ms <ms>] [--settle-ms <ms>]"
}

struct Arguments {
    root: String,
    db: String,
    cancel_after_ms: Option<u64>,
    settle_ms: u64,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut root = None;
    let mut db = None;
    let mut cancel_after_ms = None;
    let mut settle_ms = 300;
    let mut index = 1;
    let args: Vec<String> = std::env::args().collect();
    while index < args.len() {
        let flag = args[index].as_str();
        let mut value = || -> Result<String, String> {
            index += 1;
            args.get(index)
                .cloned()
                .ok_or_else(|| format!("missing value for {flag}"))
        };
        match flag {
            "--root" => root = Some(value()?),
            "--db" => db = Some(value()?),
            "--cancel-after-ms" => {
                let raw = value()?;
                cancel_after_ms = Some(
                    raw.parse::<u64>()
                        .map_err(|_| format!("--cancel-after-ms must be a number: {raw}"))?,
                );
            }
            "--settle-ms" => {
                let raw = value()?;
                settle_ms = raw
                    .parse::<u64>()
                    .map_err(|_| format!("--settle-ms must be a number: {raw}"))?;
            }
            "--help" | "-h" => return Err(String::new()),
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }
    let root = root.ok_or_else(|| "--root is required".to_string())?;
    let db = db.ok_or_else(|| "--db is required".to_string())?;
    if root.is_empty() || db.is_empty() {
        return Err("--root and --db must be non-empty".to_string());
    }
    Ok(Arguments {
        root,
        db,
        cancel_after_ms,
        settle_ms,
    })
}

fn terminal(status: ScanExecutionStatus) -> bool {
    matches!(
        status,
        ScanExecutionStatus::Published | ScanExecutionStatus::Cancelled
    )
}

fn elapsed_ms(started_at: Instant) -> i64 {
    started_at.elapsed().as_millis().min(i64::MAX as u128) as i64
}

fn main() -> ExitCode {
    let started_at = Instant::now();

    let arguments = match parse_arguments() {
        Ok(arguments) => arguments,
        Err(help) if help.is_empty() => {
            println!("{}", usage());
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("{message}\n{}", usage());
            return ExitCode::FAILURE;
        }
    };

    let mut db = match Database::open(Path::new(&arguments.db)) {
        Ok(db) => db,
        Err(error) => {
            emit(
                "error",
                elapsed_ms(started_at),
                &[("message", Field::Text(&error.to_string()))],
            );
            return ExitCode::FAILURE;
        }
    };
    let worker = match ScanWorker::new(WorkerConfig::default()) {
        Ok(worker) => worker,
        Err(error) => {
            emit(
                "error",
                elapsed_ms(started_at),
                &[("message", Field::Text(&format!("{error:?}")))],
            );
            return ExitCode::FAILURE;
        }
    };
    let clock = SystemClock;
    let session = match worker.start_session(&mut db, &clock) {
        Ok(session) => session,
        Err(error) => {
            emit(
                "error",
                elapsed_ms(started_at),
                &[("message", Field::Text(&error.to_string()))],
            );
            return ExitCode::FAILURE;
        }
    };
    // Reuse the root when the durable database already tracks this fixture
    // path (warm iterations share one committed state); add it otherwise.
    let root = match db.list_scan_roots() {
        Ok(roots) => match roots.into_iter().find(|root| root.canonical_path == arguments.root) {
            Some(existing) => existing,
            None => match db.add_scan_root("benchmark-fixture", &arguments.root) {
                Ok(root) => root,
                Err(error) => {
                    emit(
                        "error",
                        elapsed_ms(started_at),
                        &[("message", Field::Text(&error.to_string()))],
                    );
                    return ExitCode::FAILURE;
                }
            },
        },
        Err(error) => {
            emit(
                "error",
                elapsed_ms(started_at),
                &[("message", Field::Text(&error.to_string()))],
            );
            return ExitCode::FAILURE;
        }
    };
    emit(
        "ready",
        elapsed_ms(started_at),
        &[
            ("root_id", Field::Text(&root.id)),
            ("session_id", Field::Text(&session.id)),
            ("worker_config", Field::Text("default")),
        ],
    );

    let token = CancellationToken::new();
    if let Some(delay_ms) = arguments.cancel_after_ms {
        let token = token.clone();
        thread::spawn(move || {
            // The settle window runs right after this thread is spawned, so
            // this deadline fires `delay_ms` into the measured scan window.
            let cancel_at = Instant::now()
                .checked_add(Duration::from_millis(arguments.settle_ms + delay_ms))
                .expect("cancel deadline fits in Instant range");
            let now = Instant::now();
            if cancel_at > now {
                thread::sleep(cancel_at - now);
            }
            token.cancel();
            emit("cancellation_requested", elapsed_ms(started_at), &[]);
        });
    }

    // Settle window so the harness memory sampler can observe the idle
    // working set before the measured operation starts. It does not affect
    // the measured scan window (which starts at `scan_started`).
    if arguments.settle_ms > 0 {
        thread::sleep(Duration::from_millis(arguments.settle_ms));
    }

    let scan_started_at = elapsed_ms(started_at);
    emit("scan_started", scan_started_at, &[]);
    let mut port = WindowsFilesystemPort::new();
    let execution = match worker.request_manual_scan(&mut db, &root.id, &clock) {
        Ok(_) => match worker.poll(&mut db, &session.id, &mut port, &token, &clock) {
            Ok(Some(execution)) => execution,
            Ok(None) => {
                emit(
                    "error",
                    elapsed_ms(started_at),
                    &[("message", Field::Text("no execution"))],
                );
                return ExitCode::FAILURE;
            }
            Err(error) => {
                emit(
                    "error",
                    elapsed_ms(started_at),
                    &[("message", Field::Text(&error.to_string()))],
                );
                return ExitCode::FAILURE;
            }
        },
        Err(error) => {
            emit(
                "error",
                elapsed_ms(started_at),
                &[("message", Field::Text(&error.to_string()))],
            );
            return ExitCode::FAILURE;
        }
    };

    let finished_at = elapsed_ms(started_at);
    emit_finished(finished_at, &execution, scan_started_at);
    if terminal(execution.status) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit_finished(finished_at: i64, execution: &ScanExecution, scan_started_at: i64) {
    let scan_ms = finished_at.saturating_sub(scan_started_at);
    let mut fields: Vec<(&str, Field<'_>)> = vec![
        ("scan_ms", Field::Signed(scan_ms)),
        ("status", Field::Text(status_name(execution.status))),
        ("authoritative", Field::Flag(execution.authoritative)),
        ("outcome", outcome_name(execution.enumeration_outcome.as_ref())),
        ("run_id", Field::Text(&execution.run_id)),
        ("job_id", Field::Text(&execution.job_id)),
        ("error_code", field_from_option(execution.error_code.as_deref())),
    ];
    match &execution.publication {
        Some(publication) => {
            fields.push(("location_count", Field::Signed(publication.location_count)));
            fields.push(("generation", Field::Signed(publication.generation)));
        }
        None => fields.push(("location_count", Field::Null)),
    }
    match &execution.changes {
        Some(changes) => {
            fields.push(("changes_computed", Field::Flag(true)));
            fields.push(("changes_added", Field::Count(changes.added)));
            fields.push(("changes_modified", Field::Count(changes.modified)));
            fields.push(("changes_replaced", Field::Count(changes.replaced)));
            fields.push(("changes_identity_uncertain", Field::Count(changes.identity_uncertain)));
            fields.push(("changes_missing", Field::Count(changes.missing)));
            fields.push(("changes_restored", Field::Count(changes.restored)));
            fields.push(("changes_renames", Field::Count(changes.renames)));
        }
        None => fields.push(("changes_computed", Field::Flag(false))),
    }
    emit("scan_finished", finished_at, &fields);
}

fn field_from_option(value: Option<&str>) -> Field<'_> {
    match value {
        Some(value) => Field::Text(value),
        None => Field::Null,
    }
}