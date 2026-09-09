//! Diagnostic-only filesystem-call profile for the Phase 2 benchmark follow-up.
//!
//! This example deliberately wraps the public filesystem-port boundary instead
//! of changing the scanner. `ScanWorker::poll` still owns the production
//! enumerate, stage, change-plan, and publication path; the wrapper only times
//! the calls made through `FilesystemPort`/`DirectoryCursor` and emits a
//! path-free JSON-lines record. The residual is the measured scan window less
//! the timed filesystem calls; it includes staging, reconciliation reads,
//! publication, and worker overhead and is not attributed to one sub-phase.
use fruitboard_filesystem_enumeration::{
    DirectoryCursor, DirectoryEntry, FileMetadata, FilesystemPort, NeverCancelled, OpenedDirectory,
    Outcome, PortError, RootMetadata, WindowsFilesystemPort,
};
use fruitboard_scan_execution::{ScanExecutionStatus, ScanWorker, SystemClock, WorkerConfig};
use fruitboard_storage::Database;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Instant;

#[derive(Default, Clone, Copy)]
struct TimedCall {
    calls: u64,
    errors: u64,
    nanos: u128,
}

impl TimedCall {
    fn record(&mut self, elapsed_nanos: u128, succeeded: bool) {
        self.calls = self.calls.saturating_add(1);
        if !succeeded {
            self.errors = self.errors.saturating_add(1);
        }
        self.nanos = self.nanos.saturating_add(elapsed_nanos);
    }
}

#[derive(Default)]
struct Profile {
    inspect_root: TimedCall,
    open_root: TimedCall,
    next_entry: TimedCall,
    read_metadata: TimedCall,
    open_directory: TimedCall,
}

impl Profile {
    fn filesystem_nanos(&self) -> u128 {
        self.inspect_root
            .nanos
            .saturating_add(self.open_root.nanos)
            .saturating_add(self.next_entry.nanos)
            .saturating_add(self.read_metadata.nanos)
            .saturating_add(self.open_directory.nanos)
    }
}

struct TimedFilesystemPort {
    inner: WindowsFilesystemPort,
    profile: Rc<RefCell<Profile>>,
}

impl TimedFilesystemPort {
    fn new(profile: Rc<RefCell<Profile>>) -> Self {
        Self {
            inner: WindowsFilesystemPort::new(),
            profile,
        }
    }

    fn wrap_directory(&self, directory: OpenedDirectory) -> OpenedDirectory {
        OpenedDirectory {
            metadata: directory.metadata,
            case_sensitivity: directory.case_sensitivity,
            cursor: Box::new(TimedDirectoryCursor {
                inner: directory.cursor,
                profile: Rc::clone(&self.profile),
            }),
        }
    }
}

impl FilesystemPort for TimedFilesystemPort {
    fn inspect_root(&mut self, root: &Path) -> Result<RootMetadata, PortError> {
        let started = Instant::now();
        let result = self.inner.inspect_root(root);
        self.profile
            .borrow_mut()
            .inspect_root
            .record(started.elapsed().as_nanos(), result.is_ok());
        result
    }

    fn open_root(&mut self, root: &Path) -> Result<OpenedDirectory, PortError> {
        let started = Instant::now();
        let result = self.inner.open_root(root);
        let succeeded = result.is_ok();
        self.profile
            .borrow_mut()
            .open_root
            .record(started.elapsed().as_nanos(), succeeded);
        result.map(|directory| self.wrap_directory(directory))
    }
}

struct TimedDirectoryCursor {
    inner: Box<dyn DirectoryCursor>,
    profile: Rc<RefCell<Profile>>,
}

impl DirectoryCursor for TimedDirectoryCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
        let started = Instant::now();
        let result = self.inner.next_entry();
        self.profile
            .borrow_mut()
            .next_entry
            .record(started.elapsed().as_nanos(), result.is_ok());
        result
    }

    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
        let started = Instant::now();
        let result = self.inner.read_metadata(entry);
        self.profile
            .borrow_mut()
            .read_metadata
            .record(started.elapsed().as_nanos(), result.is_ok());
        result
    }

    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
        let started = Instant::now();
        let result = self.inner.open_directory(entry);
        let succeeded = result.is_ok();
        self.profile
            .borrow_mut()
            .open_directory
            .record(started.elapsed().as_nanos(), succeeded);
        result.map(|directory| OpenedDirectory {
            metadata: directory.metadata,
            case_sensitivity: directory.case_sensitivity,
            cursor: Box::new(TimedDirectoryCursor {
                inner: directory.cursor,
                profile: Rc::clone(&self.profile),
            }),
        })
    }
}

struct Arguments {
    root: String,
    db: String,
    runs: usize,
    out: Option<String>,
    warm: bool,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut root = None;
    let mut db = None;
    let mut runs = 2;
    let mut out = None;
    let mut warm = false;
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
            "--runs" => {
                runs = value()?
                    .parse::<usize>()
                    .map_err(|_| "--runs must be a positive integer".to_string())?;
                if runs == 0 {
                    return Err("--runs must be a positive integer".to_string());
                }
            }
            "--out" => out = Some(value()?),
            "--warm" => warm = true,
            "--help" | "-h" => return Err(String::new()),
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.ok_or_else(|| "--root is required".to_string())?,
        db: db.ok_or_else(|| "--db is required".to_string())?,
        runs,
        out,
        warm,
    })
}

fn json_string(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 2);
    result.push('"');
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

fn call_json(call: TimedCall) -> String {
    format!(
        "{{\"calls\":{},\"errors\":{},\"nanos\":{}}}",
        call.calls, call.errors, call.nanos
    )
}

fn outcome_name(outcome: Option<Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Complete) => "Complete",
        Some(Outcome::Partial) => "Partial",
        Some(Outcome::Denied) => "Denied",
        Some(Outcome::RootUnavailable) => "RootUnavailable",
        Some(Outcome::Cancelled) => "Cancelled",
        Some(Outcome::ResourceLimit) => "ResourceLimit",
        Some(Outcome::SinkFailed) => "SinkFailed",
        Some(Outcome::Invalid) => "Invalid",
        Some(Outcome::UnsupportedFilesystem) => "UnsupportedFilesystem",
        None => "none",
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

fn run_once(
    worker: &ScanWorker,
    db: &mut Database,
    session_id: &str,
    root_id: &str,
    run_index: usize,
    warm: bool,
) -> Result<String, String> {
    let profile = Rc::new(RefCell::new(Profile::default()));
    let mut port = TimedFilesystemPort::new(Rc::clone(&profile));
    let clock = SystemClock;
    let started = Instant::now();
    worker
        .request_manual_scan(db, root_id, &clock)
        .map_err(|error| error.to_string())?;
    let execution = worker
        .poll(db, session_id, &mut port, &NeverCancelled, &clock)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "worker returned no execution".to_string())?;
    let scan_nanos = started.elapsed().as_nanos();
    let measured = profile.borrow();
    let filesystem_nanos = measured.filesystem_nanos();
    let residual_nanos = scan_nanos.saturating_sub(filesystem_nanos);
    let location_count = execution.publication.as_ref().map_or_else(
        || "null".to_string(),
        |publication| publication.location_count.to_string(),
    );
    Ok(format!(
        "{{\"tool\":\"profile-fs-calls-v1\",\"run\":{},\"label\":{},\"scan_ms\":{},\"status\":{},\"outcome\":{},\"authoritative\":{},\"location_count\":{},\"filesystem_call_sum_ms\":{},\"residual_ms\":{},\"filesystem\":{{\"inspect_root\":{},\"open_root\":{},\"next_entry\":{},\"read_metadata\":{},\"open_directory\":{}}}}}",
        run_index,
        json_string(if warm || run_index > 0 {
            "warm"
        } else {
            "first"
        }),
        scan_nanos / 1_000_000,
        json_string(status_name(execution.status)),
        json_string(outcome_name(execution.enumeration_outcome)),
        if execution.authoritative {
            "true"
        } else {
            "false"
        },
        location_count,
        filesystem_nanos / 1_000_000,
        residual_nanos / 1_000_000,
        call_json(measured.inspect_root),
        call_json(measured.open_root),
        call_json(measured.next_entry),
        call_json(measured.read_metadata),
        call_json(measured.open_directory),
    ))
}

fn run(arguments: Arguments) -> Result<(Vec<String>, Option<String>), String> {
    if arguments.root.is_empty() || arguments.db.is_empty() {
        return Err("--root and --db must be non-empty".to_string());
    }
    let mut db = Database::open(Path::new(&arguments.db)).map_err(|error| error.to_string())?;
    let worker = ScanWorker::new(WorkerConfig::default()).map_err(|error| format!("{error:?}"))?;
    let clock = SystemClock;
    let session = worker
        .start_session(&mut db, &clock)
        .map_err(|error| error.to_string())?;
    let root = db
        .list_scan_roots()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|root| root.canonical_path == arguments.root)
        .map(|root| root.id)
        .unwrap_or_else(|| {
            db.add_scan_root("profile-fixture", &arguments.root)
                .expect("profile root can be created")
                .id
        });
    let mut lines = Vec::with_capacity(arguments.runs);
    for run_index in 0..arguments.runs {
        lines.push(run_once(
            &worker,
            &mut db,
            &session.id,
            &root,
            run_index,
            arguments.warm,
        )?);
    }
    Ok((lines, arguments.out))
}

fn usage() -> &'static str {
    "Usage: profile-fs-calls --root <fixture-root> --db <app-data-dir> [--runs <n>] [--warm] [--out <path>]"
}

fn main() -> ExitCode {
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
    match run(arguments) {
        Ok((lines, out)) => {
            let output = format!("{}\n", lines.join("\n"));
            if let Some(path) = out {
                if let Err(error) = fs::write(path, output.as_bytes()) {
                    eprintln!("failed to write profile output: {error}");
                    return ExitCode::FAILURE;
                }
            } else {
                print!("{output}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("profile failed: {error}");
            ExitCode::FAILURE
        }
    }
}
