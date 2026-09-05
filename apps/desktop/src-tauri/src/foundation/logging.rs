use super::clock::Clock;
use super::errors::ErrorCode;
use super::identifiers::OpaqueId;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::UNIX_EPOCH;

const ACTIVE_LOG_NAME: &str = "fruitboard.log";
const LOG_PREFIX: &str = "fruitboard.";
const LOG_SUFFIX: &str = ".log";
const MAX_DIAGNOSTIC_CHARS: usize = 512;
const MAX_START_RECORD_BYTES: u64 = 4 * 1024;

pub const DEFAULT_MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const DEFAULT_MAX_FILES: usize = 5;
pub const DEFAULT_MAX_AGE_MILLIS: u64 = 14 * 24 * 60 * 60 * 1000;

static BEARER_CREDENTIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer\s+[a-z0-9._~+/=-]+")
        .expect("the static bearer credential pattern must compile")
});
static SECRET_PAIR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b(access[_-]?token|refresh[_-]?token|id[_-]?token|authorization[_-]?code|authorization|client[_-]?secret|oauth[_-]?(?:code|state|token)|code[_-]?(?:verifier|challenge)|code)\b(?:\s*["']?\s*[:=]\s*["']?)[^&,\s"'}]+"#,
    )
    .expect("the static secret pair pattern must compile")
});
static PATH_SIGNAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)^/|[\s=('"]/|[a-z]:[\\/]|\\\\|\bfile:///|\.flp\b"#)
        .expect("the static path signal pattern must compile")
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) enum LogEventKind {
    #[serde(rename = "command_completed")]
    Completed,
    #[serde(rename = "command_rejected")]
    Rejected,
    #[serde(rename = "command_failed")]
    Failed,
    #[serde(rename = "command_panicked")]
    Panicked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OperationalLogEvent {
    pub(crate) timestamp_millis: u64,
    pub(crate) level: LogLevel,
    pub(crate) kind: LogEventKind,
    pub(crate) operation: &'static str,
    pub(crate) correlation_id: OpaqueId,
    pub(crate) job_id: Option<OpaqueId>,
    pub(crate) error_code: Option<ErrorCode>,
    pub(crate) diagnostic: Option<SafeDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SafeDiagnostic(String);

impl SafeDiagnostic {
    pub(crate) fn new(value: &str) -> Self {
        Self(redact_sensitive(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredLogRecord<'a> {
    timestamp_millis: u64,
    level: LogLevel,
    subsystem: &'static str,
    version: &'static str,
    event: LogEventKind,
    operation: &'static str,
    correlation_id: &'a OpaqueId,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_id: Option<&'a OpaqueId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<ErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveLogStart {
    timestamp_millis: u64,
}

pub(crate) trait LogSink: Send + Sync {
    fn write(&self, event: &OperationalLogEvent) -> io::Result<()>;
}

#[derive(Debug, Default)]
struct DisabledLogSink;

impl LogSink for DisabledLogSink {
    fn write(&self, _event: &OperationalLogEvent) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RetentionPolicy {
    pub(crate) max_file_bytes: u64,
    pub(crate) max_files: usize,
    pub(crate) max_age_millis: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_files: DEFAULT_MAX_FILES,
            max_age_millis: DEFAULT_MAX_AGE_MILLIS,
        }
    }
}

#[derive(Debug, Default)]
struct WriterState {
    active_started_at: Option<u64>,
    rotation_sequence: u64,
}

struct LocalLogSink {
    directory: PathBuf,
    retention: RetentionPolicy,
    clock: Arc<dyn Clock>,
    state: Mutex<WriterState>,
}

impl LocalLogSink {
    fn new(
        directory: PathBuf,
        retention: RetentionPolicy,
        clock: Arc<dyn Clock>,
    ) -> io::Result<Self> {
        if retention.max_file_bytes == 0
            || retention.max_files == 0
            || retention.max_age_millis == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "log retention limits must be non-zero",
            ));
        }

        fs::create_dir_all(&directory)?;

        Ok(Self {
            directory,
            retention,
            clock,
            state: Mutex::new(WriterState::default()),
        })
    }

    fn active_path(&self) -> PathBuf {
        self.directory.join(ACTIVE_LOG_NAME)
    }

    fn active_start(&self, path: &Path, now: u64) -> u64 {
        let recorded_start = fs::File::open(path).ok().and_then(|file| {
            let mut reader = BufReader::new(file).take(MAX_START_RECORD_BYTES);
            let mut first_line = String::new();
            reader.read_line(&mut first_line).ok()?;
            serde_json::from_str::<ActiveLogStart>(&first_line)
                .ok()
                .map(|record| record.timestamp_millis)
        });
        let metadata_start = path.metadata().ok().and_then(|metadata| {
            metadata
                .created()
                .or_else(|_| metadata.modified())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        });

        recorded_start
            .or(metadata_start)
            .map_or(now, |started_at| started_at.min(now))
    }

    fn rotate_if_needed(
        &self,
        next_line_bytes: u64,
        now: u64,
        state: &mut WriterState,
    ) -> io::Result<()> {
        let active = self.active_path();
        let metadata = match active.metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };

        let started_at = *state
            .active_started_at
            .get_or_insert_with(|| self.active_start(&active, now));
        let exceeds_size = metadata.len() > 0
            && metadata
                .len()
                .saturating_add(next_line_bytes)
                .saturating_add(1)
                > self.retention.max_file_bytes;
        let exceeds_age =
            metadata.len() > 0 && now.saturating_sub(started_at) >= self.retention.max_age_millis;

        if !exceeds_size && !exceeds_age {
            return Ok(());
        }

        let rotated = loop {
            let sequence = state.rotation_sequence;
            state.rotation_sequence = state.rotation_sequence.saturating_add(1);
            let candidate = self
                .directory
                .join(format!("fruitboard.{started_at:020}.{sequence:06}.log"));
            if !candidate.exists() {
                break candidate;
            }
        };

        fs::rename(active, rotated)?;
        state.active_started_at = None;
        Ok(())
    }

    fn prune_rotated(&self, now: u64) -> io::Result<()> {
        let mut retained = Vec::new();

        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(timestamp) = rotated_timestamp(&name) else {
                continue;
            };

            if now.saturating_sub(timestamp) >= self.retention.max_age_millis {
                fs::remove_file(entry.path())?;
            } else {
                retained.push((timestamp, entry.path()));
            }
        }

        retained.sort_by(|left, right| right.cmp(left));
        let rotated_limit = self.retention.max_files.saturating_sub(1);
        for (_, path) in retained.into_iter().skip(rotated_limit) {
            fs::remove_file(path)?;
        }

        Ok(())
    }
}

impl LogSink for LocalLogSink {
    fn write(&self, event: &OperationalLogEvent) -> io::Result<()> {
        let record = StoredLogRecord {
            timestamp_millis: event.timestamp_millis,
            level: event.level,
            subsystem: "command",
            version: env!("CARGO_PKG_VERSION"),
            event: event.kind,
            operation: event.operation,
            correlation_id: &event.correlation_id,
            job_id: event.job_id.as_ref(),
            error_code: event.error_code,
            diagnostic: event.diagnostic.as_ref().map(SafeDiagnostic::as_str),
        };
        let line = serde_json::to_string(&record).map_err(io::Error::other)?;
        let line_bytes = u64::try_from(line.len()).unwrap_or(u64::MAX);
        let now = self.clock.now_millis();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);

        self.rotate_if_needed(line_bytes, now, &mut state)?;

        let active = self.active_path();
        let is_new = !active.exists();
        let mut file = OpenOptions::new().create(true).append(true).open(active)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        if is_new {
            state.active_started_at = Some(now);
        }

        self.prune_rotated(now)
    }
}

pub(crate) fn default_log_sink(
    directory: Option<PathBuf>,
    clock: Arc<dyn Clock>,
) -> Arc<dyn LogSink> {
    directory
        .and_then(|directory| {
            LocalLogSink::new(directory, RetentionPolicy::default(), clock)
                .ok()
                .map(|logger| Arc::new(logger) as Arc<dyn LogSink>)
        })
        .unwrap_or_else(|| Arc::new(DisabledLogSink))
}

fn redact_sensitive(value: &str) -> String {
    if value.chars().any(|character| {
        character == '\u{fffd}'
            || (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    }) {
        return "[REDACTED_BINARY_PAYLOAD]".to_owned();
    }

    let normalized: String = value
        .chars()
        .map(|character| {
            if matches!(character, '\n' | '\r' | '\t') {
                ' '
            } else {
                character
            }
        })
        .collect();

    // Paths may legally contain whitespace and punctuation that cannot be used as
    // reliable free-text boundaries. Drop the complete diagnostic when one is
    // present so a suffix can never survive a partial regex replacement.
    if PATH_SIGNAL.is_match(&normalized) {
        return "[REDACTED_PATH]".to_owned();
    }

    let redacted = BEARER_CREDENTIAL.replace_all(&normalized, "Bearer [REDACTED]");
    let redacted = SECRET_PAIR.replace_all(&redacted, "$1=[REDACTED]");
    let mut characters = redacted.chars();
    let mut bounded: String = characters.by_ref().take(MAX_DIAGNOSTIC_CHARS).collect();
    if characters.next().is_some() {
        bounded.pop();
        bounded.push('…');
    }
    bounded
}

fn rotated_timestamp(name: &str) -> Option<u64> {
    let body = name.strip_prefix(LOG_PREFIX)?.strip_suffix(LOG_SUFFIX)?;
    let (timestamp, sequence) = body.split_once('.')?;
    if sequence.is_empty() || sequence.contains('.') {
        return None;
    }
    timestamp.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::IdGenerator;
    use crate::foundation::identifiers::IdKind;
    use crate::foundation::test_support::{FakeClock, FakeIdGenerator};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "fruitboard-log-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test log directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn event(
        ids: &FakeIdGenerator,
        diagnostic: &str,
        timestamp_millis: u64,
    ) -> OperationalLogEvent {
        OperationalLogEvent {
            timestamp_millis,
            level: LogLevel::Error,
            kind: LogEventKind::Failed,
            operation: "get_app_health",
            correlation_id: ids.next_id(IdKind::Correlation),
            job_id: None,
            error_code: Some(ErrorCode::Internal),
            diagnostic: Some(SafeDiagnostic::new(diagnostic)),
        }
    }

    fn log_contents(directory: &Path) -> String {
        let mut contents = String::new();
        for entry in fs::read_dir(directory).expect("test logs should be readable") {
            let entry = entry.expect("test log entry should be readable");
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "log")
            {
                contents.push_str(
                    &fs::read_to_string(entry.path()).expect("test log should contain UTF-8"),
                );
            }
        }
        contents
    }

    #[test]
    fn redacts_oauth_material_without_relying_on_path_redaction() {
        let value = concat!(
            "Bearer header.payload.signature ",
            "access_token=access-secret refresh_token='refresh-secret' ",
            "code_verifier=pkce-secret oauth_state=state-secret ",
            "authorization_code=authorization-secret"
        );
        let diagnostic = SafeDiagnostic::new(value);
        let redacted = diagnostic.as_str();

        for forbidden in [
            "header.payload.signature",
            "access-secret",
            "refresh-secret",
            "pkce-secret",
            "state-secret",
            "authorization-secret",
        ] {
            assert!(!redacted.contains(forbidden), "leaked {forbidden}");
        }
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_a_quoted_unix_path_as_one_complete_value() {
        assert_eq!(
            SafeDiagnostic::new(r#"{"path":"/Users/example/My Music/private-master.wav"}"#)
                .as_str(),
            "[REDACTED_PATH]"
        );
    }

    #[test]
    fn redacts_a_windows_path_with_semicolons_as_one_complete_value() {
        assert_eq!(
            SafeDiagnostic::new(r#"C:\Users\example\Sets;Archive\private-master.wav"#).as_str(),
            "[REDACTED_PATH]"
        );
    }

    #[test]
    fn redacts_an_flp_name_with_spaces_as_one_complete_value() {
        assert_eq!(
            SafeDiagnostic::new("failed to open Private Client Project.flp").as_str(),
            "[REDACTED_PATH]"
        );
    }

    #[test]
    fn redacts_other_supported_path_forms_as_complete_values() {
        for value in [
            r#"D:/Users/artist/Exports/master.wav"#,
            r#"{"path":"/home/producer/private/project"}"#,
            "file:///home/producer/My Music/export.wav",
            "GET /oauth/callback?code=authorization-secret",
        ] {
            assert_eq!(
                SafeDiagnostic::new(value).as_str(),
                "[REDACTED_PATH]",
                "failed to redact {value:?}"
            );
        }
    }

    #[test]
    fn replaces_untrusted_binary_diagnostics() {
        assert_eq!(
            redact_sensitive("parser payload: \0\u{1}binary"),
            "[REDACTED_BINARY_PAYLOAD]"
        );
        assert_eq!(
            redact_sensitive("parser payload: \u{b}binary"),
            "[REDACTED_BINARY_PAYLOAD]"
        );
        assert_eq!(
            redact_sensitive("parser payload: \u{fffd}binary"),
            "[REDACTED_BINARY_PAYLOAD]"
        );
    }

    #[test]
    fn bounds_diagnostics_without_splitting_unicode() {
        let redacted = redact_sensitive(&"🍎".repeat(MAX_DIAGNOSTIC_CHARS + 10));

        assert_eq!(redacted.chars().count(), MAX_DIAGNOSTIC_CHARS);
        assert!(redacted.ends_with('…'));
    }

    #[test]
    fn writes_allowlisted_json_and_enforces_size_and_count_retention() {
        let directory = TestDirectory::new();
        let clock = Arc::new(FakeClock::new(1_000));
        let logger = LocalLogSink::new(
            directory.path().to_path_buf(),
            RetentionPolicy {
                max_file_bytes: 1,
                max_files: 3,
                max_age_millis: 10_000,
            },
            clock.clone(),
        )
        .expect("test logger should initialize");
        let ids = FakeIdGenerator::new(1);

        for offset in 0..6 {
            clock.set(1_000 + offset);
            logger
                .write(&event(
                    &ids,
                    "access_token=secret C:\\Users\\person\\song.flp",
                    1_000 + offset,
                ))
                .expect("test event should be written");
        }

        let files = fs::read_dir(directory.path())
            .expect("test logs should be readable")
            .count();
        assert!(files <= 3, "retention kept {files} files");

        let contents = log_contents(directory.path());
        assert!(!contents.contains("secret"));
        assert!(!contents.contains("person"));
        assert!(!contents.contains("song.flp"));
        assert!(contents.lines().all(|line| {
            let record: serde_json::Value =
                serde_json::from_str(line).expect("each log line should be JSON");
            record["subsystem"] == "command"
                && record["event"] == "command_failed"
                && record.get("correlationId").is_some()
        }));
    }

    #[test]
    fn persisted_logs_do_not_retain_fragments_of_sensitive_paths() {
        let directory = TestDirectory::new();
        let clock = Arc::new(FakeClock::new(1_000));
        let logger = LocalLogSink::new(
            directory.path().to_path_buf(),
            RetentionPolicy::default(),
            clock,
        )
        .expect("test logger should initialize");
        let ids = FakeIdGenerator::new(1);
        let diagnostics = [
            r#"{"path":"/Users/example/My Music/private-master.wav"}"#,
            r#"C:\Users\example\Sets;Archive\private-master.wav"#,
            "failed to open Private Client Project.flp",
        ];

        for (offset, diagnostic) in diagnostics.iter().enumerate() {
            let timestamp = 1_000 + u64::try_from(offset).expect("test offset should fit");
            logger
                .write(&event(&ids, diagnostic, timestamp))
                .expect("path-bearing event should be written");
        }

        let contents = log_contents(directory.path());
        for forbidden in [
            "Users",
            "example",
            "My Music",
            "private-master.wav",
            "Sets;Archive",
            "Private Client Project.flp",
        ] {
            assert!(!contents.contains(forbidden), "persisted {forbidden}");
        }

        let stored_diagnostics: Vec<_> = contents
            .lines()
            .map(|line| {
                let record: serde_json::Value =
                    serde_json::from_str(line).expect("each log line should be JSON");
                record["diagnostic"]
                    .as_str()
                    .expect("diagnostic should be stored")
                    .to_owned()
            })
            .collect();
        assert_eq!(stored_diagnostics.len(), diagnostics.len());
        assert!(
            stored_diagnostics
                .iter()
                .all(|diagnostic| diagnostic == "[REDACTED_PATH]")
        );
    }

    #[test]
    fn rotates_and_removes_logs_that_exceed_the_age_limit() {
        let directory = TestDirectory::new();
        let clock = Arc::new(FakeClock::new(100));
        let logger = LocalLogSink::new(
            directory.path().to_path_buf(),
            RetentionPolicy {
                max_file_bytes: u64::MAX,
                max_files: 5,
                max_age_millis: 10,
            },
            clock.clone(),
        )
        .expect("test logger should initialize");
        let ids = FakeIdGenerator::new(1);

        logger
            .write(&event(&ids, "first", 100))
            .expect("first event should be written");
        clock.set(111);
        logger
            .write(&event(&ids, "second", 111))
            .expect("second event should be written");

        let names: Vec<_> = fs::read_dir(directory.path())
            .expect("test logs should be readable")
            .map(|entry| entry.expect("test entry should be readable").file_name())
            .collect();
        assert_eq!(names, [ACTIVE_LOG_NAME]);
        assert!(log_contents(directory.path()).contains("second"));
        assert!(!log_contents(directory.path()).contains("first"));
    }

    #[test]
    fn preserves_the_active_log_start_time_across_restarts() {
        let directory = TestDirectory::new();
        let clock = Arc::new(FakeClock::new(100));
        let retention = RetentionPolicy {
            max_file_bytes: u64::MAX,
            max_files: 5,
            max_age_millis: 10,
        };
        let ids = FakeIdGenerator::new(1);

        {
            let logger =
                LocalLogSink::new(directory.path().to_path_buf(), retention, clock.clone())
                    .expect("first logger should initialize");
            logger
                .write(&event(&ids, "first session", 100))
                .expect("first session should be written");
        }

        clock.set(109);
        {
            let logger =
                LocalLogSink::new(directory.path().to_path_buf(), retention, clock.clone())
                    .expect("second logger should initialize");
            logger
                .write(&event(&ids, "second session", 109))
                .expect("second session should be written");
        }
        assert!(log_contents(directory.path()).contains("first session"));
        assert!(log_contents(directory.path()).contains("second session"));

        clock.set(111);
        {
            let logger =
                LocalLogSink::new(directory.path().to_path_buf(), retention, clock.clone())
                    .expect("third logger should initialize");
            logger
                .write(&event(&ids, "third session", 111))
                .expect("third session should be written");
        }

        let contents = log_contents(directory.path());
        assert!(contents.contains("third session"));
        assert!(!contents.contains("first session"));
        assert!(!contents.contains("second session"));
    }
}
