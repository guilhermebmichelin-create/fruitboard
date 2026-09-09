//! Diagnostic-only native-operation profile for the filesystem boundary.
//!
//! This companion isolates the native-facing pieces timed by PR #93's
//! `profile-fs-calls` example. It runs the enumeration crate directly so the
//! profile can distinguish ancestor validation, directory queries, entry
//! opens/metadata, child-directory opens/metadata, case-mode queries, and
//! root work. It does not change the production scanner or storage path.

use fruitboard_filesystem_enumeration::{
    BatchSink, EnumerationReport, NativeOperationCall, NativeOperationProfile, NeverCancelled,
    NoProgress, Outcome, SinkError, WindowsFilesystemPort, enumerate,
};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Instant;

#[derive(Default)]
struct RecordingSink;

impl BatchSink for RecordingSink {
    fn accept(
        &mut self,
        _batch: fruitboard_filesystem_enumeration::ObservationBatch,
    ) -> Result<(), SinkError> {
        Ok(())
    }

    fn discard(&mut self) {}
}

struct Arguments {
    root: String,
    runs: usize,
    out: Option<String>,
    warm: bool,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut root = None;
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

fn call_json(call: NativeOperationCall) -> String {
    format!(
        "{{\"calls\":{},\"errors\":{},\"nanos\":{}}}",
        call.calls, call.errors, call.nanos
    )
}

fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Complete => "Complete",
        Outcome::Partial => "Partial",
        Outcome::Denied => "Denied",
        Outcome::RootUnavailable => "RootUnavailable",
        Outcome::Cancelled => "Cancelled",
        Outcome::ResourceLimit => "ResourceLimit",
        Outcome::SinkFailed => "SinkFailed",
        Outcome::Invalid => "Invalid",
        Outcome::UnsupportedFilesystem => "UnsupportedFilesystem",
    }
}

fn profile_json(profile: &NativeOperationProfile) -> String {
    format!(
        "{{\"ancestor_validation\":{},\"ancestor_links\":{},\"directory_query\":{},\"entry_open\":{},\"entry_metadata\":{},\"directory_open\":{},\"directory_metadata\":{},\"directory_case_sensitivity\":{},\"root_open\":{},\"root_metadata\":{},\"root_case_sensitivity\":{}}}",
        call_json(profile.ancestor_validation),
        profile.ancestor_links,
        call_json(profile.directory_query),
        call_json(profile.entry_open),
        call_json(profile.entry_metadata),
        call_json(profile.directory_open),
        call_json(profile.directory_metadata),
        call_json(profile.directory_case_sensitivity),
        call_json(profile.root_open),
        call_json(profile.root_metadata),
        call_json(profile.root_case_sensitivity),
    )
}

fn run_once(root: &Path, run_index: usize, warm: bool) -> Result<String, String> {
    let profile = Rc::new(RefCell::new(NativeOperationProfile::default()));
    let mut port = WindowsFilesystemPort::new_with_diagnostics(Rc::clone(&profile));
    let mut sink = RecordingSink;
    let started = Instant::now();
    let report: EnumerationReport = enumerate(
        &mut port,
        root,
        &Default::default(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );
    let enumeration_nanos = started.elapsed().as_nanos();
    let measured = profile.borrow();
    let native_nanos = measured.filesystem_nanos();
    let residual_nanos = enumeration_nanos.saturating_sub(native_nanos);
    Ok(format!(
        "{{\"tool\":\"profile-native-ops-v1\",\"run\":{},\"label\":{},\"enumeration_ms\":{},\"outcome\":{},\"authoritative\":{},\"observations\":{},\"batches\":{},\"native_call_sum_ms\":{},\"residual_ms\":{},\"native\":{}}}",
        run_index,
        json_string(if warm || run_index > 0 {
            "warm"
        } else {
            "first"
        }),
        enumeration_nanos / 1_000_000,
        json_string(outcome_name(report.outcome)),
        if report.authoritative {
            "true"
        } else {
            "false"
        },
        report.observations_discovered,
        report.batches_delivered,
        native_nanos / 1_000_000,
        residual_nanos / 1_000_000,
        profile_json(&measured),
    ))
}

fn usage() -> &'static str {
    "Usage: profile-native-ops --root <fixture-root> [--runs <n>] [--warm] [--out <path>]"
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
    if arguments.root.is_empty() {
        eprintln!("--root must be non-empty\n{}", usage());
        return ExitCode::FAILURE;
    }
    if !Path::new(&arguments.root).is_dir() {
        eprintln!("--root must name an existing directory\n{}", usage());
        return ExitCode::FAILURE;
    }
    let mut lines = Vec::with_capacity(arguments.runs);
    for run_index in 0..arguments.runs {
        match run_once(Path::new(&arguments.root), run_index, arguments.warm) {
            Ok(line) => lines.push(line),
            Err(error) => {
                eprintln!("profile failed: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    let output = format!("{}\n", lines.join("\n"));
    if let Some(path) = arguments.out {
        if let Err(error) = fs::write(path, output.as_bytes()) {
            eprintln!("failed to write profile output: {error}");
            return ExitCode::FAILURE;
        }
    } else {
        print!("{output}");
    }
    ExitCode::SUCCESS
}
