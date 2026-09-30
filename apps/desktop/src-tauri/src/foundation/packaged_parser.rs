//! Native authority for the development package's fixed installed parser.
//! No caller can supply an executable, arguments, PATH entry or file root.

use fruitboard_flp_parser::ExpectedFingerprint;
use fruitboard_flp_parser::supervisor::{
    CancellationToken, ParseRequest, ParserRequest, ParserSupervisor, ProtocolReply,
    SupervisorLimits,
};
use fruitboard_flp_parser::validation::validate_descriptor;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

const PARSER_NAME: &str = "fruitboard-flp-parser.exe";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const CRASH_ACK_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, PartialEq, Eq)]
enum PackageError {
    Missing,
    Unsafe,
}

fn installed_executable(application: &Path) -> Result<PathBuf, PackageError> {
    if !application.is_absolute() {
        return Err(PackageError::Unsafe);
    }
    let directory = application.parent().ok_or(PackageError::Unsafe)?;
    let executable = directory.join(PARSER_NAME);
    let metadata = match fs::symlink_metadata(&executable) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(PackageError::Missing);
        }
        Err(_) => return Err(PackageError::Unsafe),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(PackageError::Unsafe);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(PackageError::Unsafe);
        }
    }
    Ok(executable)
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ParserEvidence {
    fixed_installed_sibling: bool,
    health_validated: bool,
    descriptor_validated: bool,
    child_reused: bool,
    rejection_contained: bool,
    abrupt_exit_recovered: bool,
    explicit_shutdown: bool,
    restart_after_shutdown: bool,
    missing_binary_contained: bool,
    child_before_crash: Option<u32>,
    child_after_crash: Option<u32>,
}

fn health(
    supervisor: &mut ParserSupervisor,
    token: &CancellationToken,
) -> Result<(), &'static str> {
    match supervisor.request(ParserRequest::HealthCheck, token) {
        Ok(ProtocolReply::Result(value))
            if value.as_object().is_some_and(|object| object.len() == 1)
                && value["status"] == "ok" =>
        {
            Ok(())
        }
        _ => Err("parser_health_failed"),
    }
}

fn lifecycle(
    supervisor: &mut ParserSupervisor,
    executable: &Path,
    output: &Path,
) -> Result<ParserEvidence, &'static str> {
    let token = CancellationToken::default();
    health(supervisor, &token)?;
    let before = supervisor.process_id().ok_or("parser_child_missing")?;
    let descriptor = match supervisor.request(ParserRequest::Describe, &token) {
        Ok(ProtocolReply::Result(value)) => value,
        _ => return Err("parser_descriptor_failed"),
    };
    validate_descriptor(&descriptor).map_err(|_| "parser_descriptor_invalid")?;
    // Empty authority rejects the request before any file bytes can be read.
    let denied = ParserRequest::Parse(ParseRequest {
        path: executable.with_file_name("never-authorized.flp"),
        expected: ExpectedFingerprint {
            size: 1,
            modified_at_ms: 1,
        },
        allowed_roots: Vec::new(),
    });
    if !matches!(supervisor.request(denied, &token), Ok(ProtocolReply::Rejected { code }) if code == "INVALID_PATH")
    {
        return Err("parser_rejection_failed");
    }
    health(supervisor, &token)?;
    if supervisor.process_id() != Some(before) {
        return Err("parser_reuse_failed");
    }

    // The external harness checks parent PID, executable path and start time
    // before killing this exact child. Its acknowledgement follows confirmed
    // process exit; PID alone is never termination authority.
    let handshake = serde_json::to_vec(&serde_json::json!({ "childPid": before }))
        .map_err(|_| "parser_handshake_failed")?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output.with_extension("parser-crash.json"))
        .map_err(|_| "parser_handshake_failed")?;
    file.write_all(&handshake)
        .and_then(|()| file.sync_all())
        .map_err(|_| "parser_handshake_failed")?;
    drop(file);
    let started = Instant::now();
    while !output.with_extension("parser-crashed").is_file() {
        if started.elapsed() >= CRASH_ACK_TIMEOUT {
            return Err("parser_crash_ack_timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
    health(supervisor, &token)?;
    let after = supervisor.process_id().ok_or("parser_child_missing")?;
    if after == before {
        return Err("parser_crash_recovery_failed");
    }
    supervisor
        .shutdown()
        .map_err(|_| "parser_shutdown_failed")?;
    if supervisor.process_id().is_some() {
        return Err("parser_shutdown_failed");
    }
    health(supervisor, &token)?;
    supervisor
        .shutdown()
        .map_err(|_| "parser_shutdown_failed")?;
    Ok(ParserEvidence {
        fixed_installed_sibling: true,
        health_validated: true,
        descriptor_validated: true,
        child_reused: true,
        rejection_contained: true,
        abrupt_exit_recovered: true,
        explicit_shutdown: supervisor.process_id().is_none(),
        restart_after_shutdown: true,
        child_before_crash: Some(before),
        child_after_crash: Some(after),
        ..ParserEvidence::default()
    })
}

pub(super) fn parser_evidence(
    output: &Path,
    expect_missing: bool,
) -> Result<ParserEvidence, &'static str> {
    let application = std::env::current_exe().map_err(|_| "parser_location_failed")?;
    let location = installed_executable(&application);
    if expect_missing {
        return if location == Err(PackageError::Missing) {
            Ok(ParserEvidence {
                fixed_installed_sibling: true,
                missing_binary_contained: true,
                ..ParserEvidence::default()
            })
        } else {
            Err("parser_missing_not_observed")
        };
    }
    let executable = location.map_err(|_| "parser_package_unavailable")?;
    let mut supervisor = ParserSupervisor::new(
        executable.clone(),
        SupervisorLimits {
            request_timeout: REQUEST_TIMEOUT,
            max_requests_per_process: 16,
        },
    )
    .map_err(|_| "parser_supervisor_failed")?;
    let result = lifecycle(&mut supervisor, &executable, output);
    // Cleanup runs on every result before the native application exits.
    supervisor
        .shutdown()
        .map_err(|_| "parser_shutdown_failed")?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_sibling_is_required_even_when_other_executables_exist() {
        let directory =
            std::env::temp_dir().join(format!("fruitboard-package-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&directory).unwrap();
        let application = directory.join("fruitboard-desktop.exe");
        fs::write(&application, b"synthetic").unwrap();
        assert_eq!(
            installed_executable(&application),
            Err(PackageError::Missing)
        );
        let parser = directory.join(PARSER_NAME);
        fs::create_dir(&parser).unwrap();
        assert_eq!(
            installed_executable(&application),
            Err(PackageError::Unsafe)
        );
        fs::remove_dir(&parser).unwrap();
        fs::write(&parser, b"synthetic").unwrap();
        assert_eq!(installed_executable(&application).unwrap(), parser);
        assert_eq!(
            installed_executable(Path::new("relative.exe")),
            Err(PackageError::Unsafe)
        );
        fs::remove_file(parser).unwrap();
        fs::remove_file(application).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
