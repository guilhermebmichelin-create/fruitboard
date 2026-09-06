//! Bounded, metadata-only Windows filesystem enumeration for Issue #36.
//!
//! This crate is an isolated producer. It has no parser, SQLite connection,
//! watcher, renderer command, or source-file mutation capability. The caller
//! owns staging and must discard every provisional batch unless the returned
//! report is authoritative and the durable publication fence succeeds.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

/// The provisional Phase 2 maximum observation batch.
pub const HARD_MAX_BATCH_RECORDS: usize = 512;
/// The provisional Phase 2 maximum staging batch size.
pub const HARD_MAX_BATCH_BYTES: usize = 256 * 1024 * 1024;

/// A root-relative path retained for display/listing. It is not an identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DisplayRelativePath(String);

impl DisplayRelativePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A root-relative locator serialized with the spelling returned by the
/// directory handle.
///
/// Serialization is deliberately case-preserving. Equality on this value is
/// also exact: the Windows boundary owns case-equivalence, including
/// per-directory case-sensitive mode, and performs duplicate detection before
/// delivery. Downstream consumers must treat this string as an opaque locator
/// and must not apply a second normalization rule.
#[derive(Clone, Debug)]
pub struct NormalizedLocator(String);

impl NormalizedLocator {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Eq for NormalizedLocator {}

impl PartialEq for NormalizedLocator {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Ord for NormalizedLocator {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for NormalizedLocator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Reasons a relative path cannot be accepted by the Windows boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathError {
    Empty,
    Absolute,
    ParentTraversal,
    CurrentComponent,
    InvalidUnicode,
    Nul,
    AlternateDataStream,
    InvalidComponent,
}

/// Normalize a root-relative path at the boundary.
///
/// The returned display path and locator use `\\` separators. No filesystem
/// lookup occurs here; the caller is responsible for constructing the input
/// only from an entry name returned by its port.
pub fn normalize_relative_path(
    relative: &Path,
) -> Result<(DisplayRelativePath, NormalizedLocator), PathError> {
    let mut components = Vec::new();
    for component in relative.components() {
        let Component::Normal(value) = component else {
            return Err(match component {
                Component::RootDir | Component::Prefix(_) => PathError::Absolute,
                Component::ParentDir => PathError::ParentTraversal,
                Component::CurDir => PathError::CurrentComponent,
                Component::Normal(_) => PathError::InvalidComponent,
            });
        };
        let value = value.to_str().ok_or(PathError::InvalidUnicode)?;
        validate_component(value)?;
        components.push(value);
    }
    if components.is_empty() {
        return Err(PathError::Empty);
    }
    let value = components.join("\\");
    Ok((DisplayRelativePath(value.clone()), NormalizedLocator(value)))
}

fn validate_component(value: &str) -> Result<(), PathError> {
    if value.is_empty() {
        return Err(PathError::InvalidComponent);
    }
    if value.contains('\0') {
        return Err(PathError::Nul);
    }
    if value.contains(':') {
        return Err(PathError::AlternateDataStream);
    }
    if value.contains(['/', '\\']) {
        return Err(PathError::InvalidComponent);
    }
    if value
        .chars()
        .any(|character| matches!(character, '*' | '?' | '<' | '>' | '"' | '|'))
    {
        return Err(PathError::InvalidComponent);
    }
    if value.chars().any(char::is_control) {
        return Err(PathError::InvalidComponent);
    }
    Ok(())
}

/// Case behavior reported by an opened directory handle.
///
/// This is a directory property, not a global lowercase-normalization rule.
/// The Windows implementation reads `FileCaseSensitiveInfo`; a fake port can
/// provide the same decision deterministically.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum DirectoryCaseSensitivity {
    Sensitive,
    #[default]
    Insensitive,
}

#[derive(Clone, Debug)]
struct LocatorKey(Vec<LocatorComponent>);

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocatorComponent {
    value: String,
    sensitivity: DirectoryCaseSensitivity,
}

impl PartialEq for LocatorKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for LocatorKey {}

impl LocatorKey {
    fn child(&self, value: &str, sensitivity: DirectoryCaseSensitivity) -> Self {
        let mut components = self.0.clone();
        components.push(LocatorComponent {
            value: value.to_owned(),
            sensitivity,
        });
        Self(components)
    }
}

impl Ord for LocatorKey {
    fn cmp(&self, other: &Self) -> Ordering {
        for (left, right) in self.0.iter().zip(&other.0) {
            let sensitivity = left.sensitivity.cmp(&right.sensitivity);
            if sensitivity != Ordering::Equal {
                return sensitivity;
            }
            let value = match left.sensitivity {
                DirectoryCaseSensitivity::Sensitive => left.value.cmp(&right.value),
                DirectoryCaseSensitivity::Insensitive => {
                    compare_case_insensitive(&left.value, &right.value)
                }
            };
            if value != Ordering::Equal {
                return value;
            }
        }
        self.0.len().cmp(&other.0.len())
    }
}

impl PartialOrd for LocatorKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(windows)]
fn compare_case_insensitive(left: &str, right: &str) -> Ordering {
    use std::os::windows::ffi::OsStrExt;

    let left: Vec<u16> = OsStr::new(left).encode_wide().collect();
    let right: Vec<u16> = OsStr::new(right).encode_wide().collect();
    let result = unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        )
    };
    match result {
        1 => Ordering::Less,
        2 => Ordering::Equal,
        3 => Ordering::Greater,
        _ => left.cmp(&right),
    }
}

#[cfg(not(windows))]
fn compare_case_insensitive(left: &str, right: &str) -> Ordering {
    // The production implementation is Windows-only. This keeps portable
    // synthetic tests deterministic without changing the serialized spelling.
    fn ascii_case_key(value: char) -> char {
        if value.is_ascii_uppercase() {
            char::from_u32(value as u32 + ('a' as u32 - 'A' as u32))
                .expect("ASCII case mapping is a valid scalar")
        } else {
            value
        }
    }

    let mut left_chars = left.chars();
    let mut right_chars = right.chars();
    loop {
        match (left_chars.next(), right_chars.next()) {
            (Some(left), Some(right)) if left.eq_ignore_ascii_case(&right) => continue,
            (Some(left), Some(right)) => {
                let ordering = ascii_case_key(left).cmp(&ascii_case_key(right));
                if ordering == Ordering::Equal {
                    left.cmp(&right)
                } else {
                    ordering
                }
            }
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

/// Physical identity qualified only for the researched local NTFS scope.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QualifiedIdentity {
    pub volume_serial: u64,
    pub file_id: u128,
    pub qualification: IdentityQualification,
}

/// Number of 100-nanosecond intervals between the Windows and Unix epochs.
/// The conversion is kept at the source precision; storage may deliberately
/// reduce precision later, but the enumeration boundary does not.
pub const WINDOWS_EPOCH_OFFSET_100NS: i128 = 116_444_736_000_000_000;

/// Convert a signed Windows FILETIME tick count to signed Unix nanoseconds.
pub const fn windows_filetime_100ns_to_unix_ns(ticks: i64) -> i128 {
    (ticks as i128 - WINDOWS_EPOCH_OFFSET_100NS) * 100
}

/// Preserve the opaque Windows 128-bit file-ID bytes in the agreed u128 DTO.
pub const fn windows_file_id_to_u128(bytes: [u8; 16]) -> u128 {
    u128::from_le_bytes(bytes)
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IdentityQualification {
    LocalNtfs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    File,
    Directory,
    Other,
}

/// Metadata returned by a filesystem port. No content is present here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    pub kind: EntryKind,
    pub byte_size: u64,
    pub modified_unix_ns: i128,
    pub identity: Option<QualifiedIdentity>,
    pub reparse_point: bool,
    pub recall_or_offline: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilesystemQualification {
    LocalNtfs,
    Unqualified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootMetadata {
    pub metadata: FileMetadata,
    pub qualification: FilesystemQualification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    pub name: OsString,
}

impl DirectoryEntry {
    pub fn new(name: impl Into<OsString>) -> Self {
        Self { name: name.into() }
    }
}

/// Coarse, path-free errors a port may inject or observe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortError {
    AccessDenied,
    NotFound,
    ResourceLimit,
    Unsupported,
    ReparsePoint,
    Changed,
    Other,
}

/// An opened directory capability. All operations are relative to the same
/// opened directory handle; callers never turn an entry name back into an
/// independently resolved path.
pub trait DirectoryCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError>;
    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError>;
    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError>;
}

/// A directory handle plus its metadata and case-mode decision.
pub struct OpenedDirectory {
    pub metadata: FileMetadata,
    pub case_sensitivity: DirectoryCaseSensitivity,
    pub cursor: Box<dyn DirectoryCursor>,
}

/// The only filesystem authority needed by the enumerator.
pub trait FilesystemPort {
    fn inspect_root(&mut self, root: &Path) -> Result<RootMetadata, PortError>;
    fn open_root(&mut self, root: &Path) -> Result<OpenedDirectory, PortError>;
}

/// The storage/publication owner implements this with run-scoped staging.
pub trait BatchSink {
    fn accept(&mut self, batch: ObservationBatch) -> Result<(), SinkError>;

    /// Discard all batches accepted for the current run. The enumerator calls
    /// this exactly once for every non-authoritative outcome.
    fn discard(&mut self);
}

/// Provisional storage for one enumeration run. A complete run remains staged
/// for the publication owner; every other outcome is invalidated by the
/// run-scoped adapter before the report is returned.
pub trait RunScopedStorage {
    fn stage_batch(&mut self, run_id: &str, batch: ObservationBatch) -> Result<(), SinkError>;
    fn invalidate_run(&mut self, run_id: &str);
}

/// Adapts the agreed run-ID staging/invalidation interface to the enumerator's
/// bounded batch sink. It intentionally has no commit method: the storage
/// publication transaction owns the final generation/lease/cancellation fence.
pub struct RunScopedSinkAdapter<'a, S: ?Sized> {
    storage: &'a mut S,
    run_id: String,
}

impl<'a, S: ?Sized> RunScopedSinkAdapter<'a, S> {
    pub fn new(storage: &'a mut S, run_id: impl Into<String>) -> Self {
        Self {
            storage,
            run_id: run_id.into(),
        }
    }
}

impl<S: RunScopedStorage + ?Sized> BatchSink for RunScopedSinkAdapter<'_, S> {
    fn accept(&mut self, batch: ObservationBatch) -> Result<(), SinkError> {
        self.storage.stage_batch(&self.run_id, batch)
    }

    fn discard(&mut self) {
        self.storage.invalidate_run(&self.run_id);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SinkError {
    ResourceLimit,
    Unavailable,
    Rejected,
}

/// Cooperative cancellation. Implementations must not block in this method.
pub trait Cancellation {
    fn is_cancelled(&self) -> bool;
}

#[derive(Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, AtomicOrdering::Release);
    }
}

impl Cancellation for CancellationToken {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(AtomicOrdering::Acquire)
    }
}

pub struct NeverCancelled;

impl Cancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressUpdate {
    pub directories_visited: usize,
    pub entries_examined: usize,
    pub observations_discovered: usize,
    pub final_update: bool,
    pub outcome: Option<Outcome>,
}

pub trait ProgressSink {
    fn report(&mut self, update: ProgressUpdate);
}

pub struct NoProgress;

impl ProgressSink for NoProgress {
    fn report(&mut self, _update: ProgressUpdate) {}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub locator: NormalizedLocator,
    pub display_path: DisplayRelativePath,
    pub byte_size: u64,
    pub modified_unix_ns: i128,
    pub identity: Option<QualifiedIdentity>,
}

impl Observation {
    fn estimated_bytes(&self) -> usize {
        self.locator
            .as_str()
            .len()
            .saturating_add(self.display_path.as_str().len())
            .saturating_add(std::mem::size_of::<u64>())
            .saturating_add(std::mem::size_of::<i128>())
            .saturating_add(self.identity.as_ref().map_or(0, |_| {
                std::mem::size_of::<u64>() + std::mem::size_of::<u128>()
            }))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationBatch {
    pub sequence: u64,
    pub records: Vec<Observation>,
    pub estimated_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExclusionReason {
    ReparsePoint,
    UnsupportedEntryType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyExclusion {
    pub display_path: DisplayRelativePath,
    pub reason: ExclusionReason,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageFailureKind {
    InvalidRoot,
    InvalidLimits,
    UnsupportedFilesystem,
    RootNotFound,
    RootDenied,
    RootChanged,
    RootIdentityUnavailable,
    DirectoryNotFound,
    DirectoryDenied,
    DirectoryRead,
    DirectoryChanged,
    DirectoryIdentityUnavailable,
    EntryDisappeared,
    EntryDenied,
    MetadataRead,
    InvalidEntryName,
    DuplicateLocator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageFailure {
    pub display_path: Option<DisplayRelativePath>,
    pub kind: CoverageFailureKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Complete,
    Partial,
    Denied,
    RootUnavailable,
    Cancelled,
    ResourceLimit,
    SinkFailed,
    Invalid,
    UnsupportedFilesystem,
}

impl Outcome {
    pub fn is_authoritative(self) -> bool {
        self == Self::Complete
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumerationReport {
    pub outcome: Outcome,
    pub authoritative: bool,
    pub root_qualification: Option<FilesystemQualification>,
    pub directories_visited: usize,
    pub entries_examined: usize,
    pub observations_discovered: usize,
    pub batches_delivered: usize,
    pub identity_unavailable: usize,
    pub exclusions: Vec<PolicyExclusion>,
    pub omitted_exclusions: usize,
    pub coverage_failures: Vec<CoverageFailure>,
    pub omitted_coverage_failures: usize,
}

impl EnumerationReport {
    fn new() -> Self {
        Self {
            outcome: Outcome::Invalid,
            authoritative: false,
            root_qualification: None,
            directories_visited: 0,
            entries_examined: 0,
            observations_discovered: 0,
            batches_delivered: 0,
            identity_unavailable: 0,
            exclusions: Vec::new(),
            omitted_exclusions: 0,
            coverage_failures: Vec::new(),
            omitted_coverage_failures: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumerationLimits {
    pub max_directories: usize,
    pub max_pending_directories: usize,
    pub max_pending_path_bytes: usize,
    pub max_entries: usize,
    pub max_observations: usize,
    pub max_path_bytes: usize,
    pub max_total_path_bytes: usize,
    pub max_diagnostics: usize,
    pub max_exclusions: usize,
    pub max_batch_records: usize,
    pub max_batch_bytes: usize,
    pub max_batches: usize,
    pub progress_interval: Duration,
}

impl Default for EnumerationLimits {
    fn default() -> Self {
        Self {
            max_directories: 100_000,
            max_pending_directories: 4_096,
            max_pending_path_bytes: 4 * 1024 * 1024,
            max_entries: 1_000_000,
            max_observations: 100_000,
            max_path_bytes: 32 * 1024,
            max_total_path_bytes: 64 * 1024 * 1024,
            max_diagnostics: 256,
            max_exclusions: 256,
            max_batch_records: HARD_MAX_BATCH_RECORDS,
            max_batch_bytes: HARD_MAX_BATCH_BYTES,
            max_batches: 200_000,
            progress_interval: Duration::from_millis(250),
        }
    }
}

impl EnumerationLimits {
    fn validate(&self) -> Result<(), ()> {
        if self.max_directories == 0
            || self.max_pending_directories == 0
            || self.max_pending_path_bytes == 0
            || self.max_entries == 0
            || self.max_observations == 0
            || self.max_path_bytes == 0
            || self.max_total_path_bytes == 0
            || self.max_batch_records == 0
            || self.max_batch_records > HARD_MAX_BATCH_RECORDS
            || self.max_batch_bytes == 0
            || self.max_batch_bytes > HARD_MAX_BATCH_BYTES
            || self.max_batches == 0
        {
            return Err(());
        }
        Ok(())
    }
}

/// Enumerate one already-selected root into bounded, provisional batches.
pub fn enumerate<P, S, C, R>(
    port: &mut P,
    root: &Path,
    limits: &EnumerationLimits,
    cancellation: &C,
    sink: &mut S,
    progress: &mut R,
) -> EnumerationReport
where
    P: FilesystemPort,
    S: BatchSink,
    C: Cancellation,
    R: ProgressSink,
{
    let mut engine = Engine {
        port,
        root,
        limits: limits.clone(),
        cancellation,
        sink,
        progress,
        report: EnumerationReport::new(),
        seen: BTreeSet::new(),
        stack: Vec::new(),
        pending: Vec::new(),
        pending_bytes: 0,
        pending_path_bytes: 0,
        next_batch_sequence: 0,
        path_bytes: 0,
        root_identity: None,
        last_progress: None,
        status: None,
    };
    let outcome = engine.run();
    if !outcome.is_authoritative() {
        engine.sink.discard();
    }
    engine.report.outcome = outcome;
    engine.report.authoritative = outcome.is_authoritative();
    engine.emit_progress(true, Some(outcome));
    engine.report
}

/// Enumerate into a run-scoped staging sink.
///
/// The adapter invalidates the run before this function returns for every
/// non-authoritative outcome, including cancellation, a rejected batch, root
/// replacement, and a late final-root failure. A `Complete` report leaves
/// accepted batches staged for the publication owner to fence and commit.
pub fn enumerate_into_run<P, S, C, R>(
    port: &mut P,
    root: &Path,
    limits: &EnumerationLimits,
    cancellation: &C,
    storage: &mut S,
    run_id: impl Into<String>,
    progress: &mut R,
) -> EnumerationReport
where
    P: FilesystemPort,
    S: RunScopedStorage + ?Sized,
    C: Cancellation,
    R: ProgressSink,
{
    let mut sink = RunScopedSinkAdapter::new(storage, run_id);
    enumerate(port, root, limits, cancellation, &mut sink, progress)
}

struct DirectoryWork {
    relative: PathBuf,
    path_bytes: usize,
    locator_key: LocatorKey,
    opened: OpenedDirectory,
}

struct Engine<'a, P, S, C, R> {
    port: &'a mut P,
    root: &'a Path,
    limits: EnumerationLimits,
    cancellation: &'a C,
    sink: &'a mut S,
    progress: &'a mut R,
    report: EnumerationReport,
    seen: BTreeSet<LocatorKey>,
    stack: Vec<DirectoryWork>,
    pending: Vec<Observation>,
    pending_bytes: usize,
    pending_path_bytes: usize,
    next_batch_sequence: u64,
    path_bytes: usize,
    root_identity: Option<QualifiedIdentity>,
    last_progress: Option<Instant>,
    status: Option<Outcome>,
}

impl<'a, P, S, C, R> Engine<'a, P, S, C, R>
where
    P: FilesystemPort,
    S: BatchSink,
    C: Cancellation,
    R: ProgressSink,
{
    fn run(&mut self) -> Outcome {
        if self.limits.validate().is_err() {
            self.add_failure(None, CoverageFailureKind::InvalidLimits, Outcome::Invalid);
            return Outcome::Invalid;
        }
        if self.validate_root_path().is_err() {
            self.add_failure(None, CoverageFailureKind::InvalidRoot, Outcome::Invalid);
            return Outcome::Invalid;
        }
        if self.cancellation.is_cancelled() {
            return Outcome::Cancelled;
        }

        let initial_root = match self.port.inspect_root(self.root) {
            Ok(value) => value,
            Err(error) => {
                self.record_root_port_error(error);
                return self.status.unwrap_or(Outcome::RootUnavailable);
            }
        };
        self.report.root_qualification = Some(initial_root.qualification);
        if let Some(outcome) = self.validate_root_metadata(&initial_root) {
            return outcome;
        }
        self.root_identity = initial_root.metadata.identity.clone();
        let opened_root = match self.port.open_root(self.root) {
            Ok(opened) => opened,
            Err(error) => {
                self.record_root_port_error(error);
                return self.status.unwrap_or(Outcome::RootUnavailable);
            }
        };
        if let Some(outcome) = self.validate_opened_root(&initial_root, &opened_root) {
            return outcome;
        }
        self.stack.push(DirectoryWork {
            relative: PathBuf::new(),
            path_bytes: 0,
            locator_key: LocatorKey(Vec::new()),
            opened: opened_root,
        });

        while let Some(work) = self.stack.pop() {
            self.pending_path_bytes = self.pending_path_bytes.saturating_sub(work.path_bytes);
            if self.cancellation.is_cancelled() {
                return Outcome::Cancelled;
            }
            if self.report.directories_visited >= self.limits.max_directories {
                self.status = Some(Outcome::ResourceLimit);
                return Outcome::ResourceLimit;
            }
            let parent_case_sensitivity = work.opened.case_sensitivity;
            let parent_locator_key = work.locator_key;
            let mut cursor = work.opened.cursor;
            self.report.directories_visited += 1;
            loop {
                if self.cancellation.is_cancelled() {
                    return Outcome::Cancelled;
                }
                let entry = match cursor.next_entry() {
                    Ok(Some(entry)) => entry,
                    Ok(None) => break,
                    Err(error) => {
                        self.record_directory_cursor_error(&work.relative, error);
                        if error == PortError::ResourceLimit {
                            return Outcome::ResourceLimit;
                        }
                        break;
                    }
                };
                self.report.entries_examined = self.report.entries_examined.saturating_add(1);
                if self.report.entries_examined > self.limits.max_entries {
                    self.status = Some(Outcome::ResourceLimit);
                    return Outcome::ResourceLimit;
                }
                if self.cancellation.is_cancelled() {
                    return Outcome::Cancelled;
                }
                if validate_entry_name(&entry.name).is_err() {
                    self.add_failure(
                        None,
                        CoverageFailureKind::InvalidEntryName,
                        Outcome::Invalid,
                    );
                    self.emit_progress(false, None);
                    continue;
                }
                let child_relative = work.relative.join(&entry.name);
                let (display_path, locator) = match normalize_relative_path(&child_relative) {
                    Ok(value) => value,
                    Err(_) => {
                        self.add_failure(
                            None,
                            CoverageFailureKind::InvalidEntryName,
                            Outcome::Invalid,
                        );
                        self.emit_progress(false, None);
                        continue;
                    }
                };
                let entry_text = entry
                    .name
                    .to_str()
                    .expect("validated directory entry name is Unicode");
                let locator_key = parent_locator_key.child(entry_text, parent_case_sensitivity);
                if !self.reserve_path_bytes(display_path.as_str().len()) {
                    self.status = Some(Outcome::ResourceLimit);
                    return Outcome::ResourceLimit;
                }
                let metadata = match cursor.read_metadata(&entry) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        self.record_metadata_error(&display_path, error);
                        if error == PortError::ResourceLimit {
                            return Outcome::ResourceLimit;
                        }
                        self.emit_progress(false, None);
                        continue;
                    }
                };
                if metadata.reparse_point {
                    self.add_exclusion(display_path, ExclusionReason::ReparsePoint);
                    self.emit_progress(false, None);
                    continue;
                }
                match metadata.kind {
                    EntryKind::Directory => {
                        let pending_path_bytes = display_path.as_str().len();
                        if self.stack.len() >= self.limits.max_pending_directories
                            || self.pending_path_bytes.saturating_add(pending_path_bytes)
                                > self.limits.max_pending_path_bytes
                        {
                            self.status = Some(Outcome::ResourceLimit);
                            return Outcome::ResourceLimit;
                        }
                        let opened = match cursor.open_directory(&entry) {
                            Ok(opened) => opened,
                            Err(error) => {
                                self.record_directory_open_error(&display_path, error);
                                if error == PortError::ResourceLimit {
                                    return Outcome::ResourceLimit;
                                }
                                continue;
                            }
                        };
                        if !self.validate_opened_child(&display_path, &metadata, &opened) {
                            continue;
                        }
                        self.pending_path_bytes =
                            self.pending_path_bytes.saturating_add(pending_path_bytes);
                        self.stack.push(DirectoryWork {
                            relative: child_relative,
                            path_bytes: pending_path_bytes,
                            locator_key,
                            opened,
                        });
                    }
                    EntryKind::Other => {
                        self.add_exclusion(display_path, ExclusionReason::UnsupportedEntryType);
                    }
                    EntryKind::File => {
                        if !is_flp_name(&entry.name) {
                            self.emit_progress(false, None);
                            continue;
                        }
                        if self.report.observations_discovered >= self.limits.max_observations {
                            self.status = Some(Outcome::ResourceLimit);
                            return Outcome::ResourceLimit;
                        }
                        if !self.seen.insert(locator_key) {
                            self.add_failure(
                                Some(display_path),
                                CoverageFailureKind::DuplicateLocator,
                                Outcome::Invalid,
                            );
                            return Outcome::Invalid;
                        }
                        if metadata.identity.is_none() {
                            self.report.identity_unavailable =
                                self.report.identity_unavailable.saturating_add(1);
                        }
                        let observation = Observation {
                            locator,
                            display_path,
                            byte_size: metadata.byte_size,
                            modified_unix_ns: metadata.modified_unix_ns,
                            identity: metadata.identity,
                        };
                        if !self.append_observation(observation) {
                            return self.append_failure_outcome();
                        }
                    }
                }
                self.emit_progress(false, None);
            }
        }

        if self.cancellation.is_cancelled() {
            return Outcome::Cancelled;
        }
        if !self.flush_pending() {
            return self.status.unwrap_or(Outcome::SinkFailed);
        }
        if self.cancellation.is_cancelled() {
            return Outcome::Cancelled;
        }

        let final_root = match self.port.inspect_root(self.root) {
            Ok(value) => value,
            Err(error) => {
                self.record_root_port_error(error);
                return self.status.unwrap_or(Outcome::RootUnavailable);
            }
        };
        if let Some(outcome) = self.validate_final_root(&final_root) {
            return outcome;
        }
        self.status.unwrap_or(Outcome::Complete)
    }

    fn validate_root_path(&self) -> Result<(), PathError> {
        if !self.root.is_absolute() {
            return Err(PathError::Absolute);
        }
        if self.root.as_os_str().is_empty() {
            return Err(PathError::Empty);
        }
        if self.root.to_str().is_none() {
            return Err(PathError::InvalidUnicode);
        }
        if self
            .root
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
        {
            return Err(PathError::ParentTraversal);
        }
        if self.root.to_string_lossy().len() > self.limits.max_path_bytes {
            return Err(PathError::InvalidComponent);
        }
        Ok(())
    }

    fn validate_root_metadata(&mut self, root: &RootMetadata) -> Option<Outcome> {
        if root.qualification != FilesystemQualification::LocalNtfs {
            self.add_failure(
                None,
                CoverageFailureKind::UnsupportedFilesystem,
                Outcome::UnsupportedFilesystem,
            );
            return Some(Outcome::UnsupportedFilesystem);
        }
        if root.metadata.reparse_point || root.metadata.kind != EntryKind::Directory {
            self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            );
            return Some(Outcome::RootUnavailable);
        }
        if root.metadata.identity.is_none() {
            self.add_failure(
                None,
                CoverageFailureKind::RootIdentityUnavailable,
                Outcome::Partial,
            );
            return Some(Outcome::Partial);
        }
        None
    }

    fn validate_opened_root(
        &mut self,
        inspected: &RootMetadata,
        opened: &OpenedDirectory,
    ) -> Option<Outcome> {
        if opened.metadata.reparse_point || opened.metadata.kind != EntryKind::Directory {
            self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            );
            return Some(Outcome::RootUnavailable);
        }
        match (&inspected.metadata.identity, &opened.metadata.identity) {
            (Some(before), Some(after)) if before == after => None,
            (Some(_), Some(_)) => {
                self.add_failure(
                    None,
                    CoverageFailureKind::RootChanged,
                    Outcome::RootUnavailable,
                );
                Some(Outcome::RootUnavailable)
            }
            _ => {
                self.add_failure(
                    None,
                    CoverageFailureKind::RootIdentityUnavailable,
                    Outcome::Partial,
                );
                Some(Outcome::Partial)
            }
        }
    }

    fn validate_opened_child(
        &mut self,
        display_path: &DisplayRelativePath,
        inspected: &FileMetadata,
        opened: &OpenedDirectory,
    ) -> bool {
        if opened.metadata.reparse_point || opened.metadata.kind != EntryKind::Directory {
            self.add_failure(
                Some(display_path.clone()),
                CoverageFailureKind::DirectoryChanged,
                Outcome::Partial,
            );
            return false;
        }
        match (&inspected.identity, &opened.metadata.identity) {
            (Some(before), Some(after)) if before == after => true,
            (Some(_), Some(_)) => {
                self.add_failure(
                    Some(display_path.clone()),
                    CoverageFailureKind::DirectoryChanged,
                    Outcome::Partial,
                );
                false
            }
            _ => {
                self.add_failure(
                    Some(display_path.clone()),
                    CoverageFailureKind::DirectoryIdentityUnavailable,
                    Outcome::Partial,
                );
                false
            }
        }
    }

    fn validate_final_root(&mut self, root: &RootMetadata) -> Option<Outcome> {
        if root.qualification != FilesystemQualification::LocalNtfs {
            self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            );
            return Some(Outcome::RootUnavailable);
        }
        if root.metadata.reparse_point || root.metadata.kind != EntryKind::Directory {
            self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            );
            return Some(Outcome::RootUnavailable);
        }
        match (&self.root_identity, &root.metadata.identity) {
            (Some(before), Some(after)) if before == after => None,
            (Some(_), Some(_)) => {
                self.add_failure(
                    None,
                    CoverageFailureKind::RootChanged,
                    Outcome::RootUnavailable,
                );
                Some(Outcome::RootUnavailable)
            }
            _ => {
                self.add_failure(
                    None,
                    CoverageFailureKind::RootIdentityUnavailable,
                    Outcome::Partial,
                );
                Some(Outcome::Partial)
            }
        }
    }

    fn reserve_path_bytes(&mut self, bytes: usize) -> bool {
        let next = self.path_bytes.saturating_add(bytes);
        if bytes > self.limits.max_path_bytes || next > self.limits.max_total_path_bytes {
            return false;
        }
        self.path_bytes = next;
        true
    }

    fn append_observation(&mut self, observation: Observation) -> bool {
        let bytes = observation.estimated_bytes();
        if bytes > self.limits.max_batch_bytes {
            return false;
        }
        if !self.pending.is_empty()
            && (self.pending.len() >= self.limits.max_batch_records
                || self.pending_bytes.saturating_add(bytes) > self.limits.max_batch_bytes)
            && !self.flush_pending()
        {
            return false;
        }
        self.pending_bytes = self.pending_bytes.saturating_add(bytes);
        self.pending.push(observation);
        self.report.observations_discovered = self.report.observations_discovered.saturating_add(1);
        if self.pending.len() >= self.limits.max_batch_records {
            return self.flush_pending();
        }
        true
    }

    fn append_failure_outcome(&mut self) -> Outcome {
        if self.cancellation.is_cancelled() {
            self.status = Some(Outcome::Cancelled);
            Outcome::Cancelled
        } else {
            let outcome = self.status.unwrap_or(Outcome::ResourceLimit);
            self.status = Some(outcome);
            outcome
        }
    }

    fn flush_pending(&mut self) -> bool {
        if self.pending.is_empty() {
            return true;
        }
        if self.cancellation.is_cancelled() {
            self.pending.clear();
            self.pending_bytes = 0;
            return false;
        }
        if self.report.batches_delivered >= self.limits.max_batches {
            self.pending.clear();
            self.pending_bytes = 0;
            self.status = Some(Outcome::ResourceLimit);
            return false;
        }
        let batch = ObservationBatch {
            sequence: self.next_batch_sequence,
            records: std::mem::take(&mut self.pending),
            estimated_bytes: self.pending_bytes,
        };
        self.pending_bytes = 0;
        match self.sink.accept(batch) {
            Ok(()) => {
                self.next_batch_sequence = self.next_batch_sequence.saturating_add(1);
                self.report.batches_delivered = self.report.batches_delivered.saturating_add(1);
                true
            }
            Err(SinkError::ResourceLimit) => {
                self.status = Some(Outcome::ResourceLimit);
                false
            }
            Err(SinkError::Unavailable | SinkError::Rejected) => {
                self.status = Some(Outcome::SinkFailed);
                false
            }
        }
    }

    fn record_root_port_error(&mut self, error: PortError) {
        match error {
            PortError::AccessDenied => {
                self.add_failure(None, CoverageFailureKind::RootDenied, Outcome::Denied)
            }
            PortError::NotFound => self.add_failure(
                None,
                CoverageFailureKind::RootNotFound,
                Outcome::RootUnavailable,
            ),
            PortError::ResourceLimit => self.status = Some(Outcome::ResourceLimit),
            PortError::Unsupported => self.add_failure(
                None,
                CoverageFailureKind::UnsupportedFilesystem,
                Outcome::UnsupportedFilesystem,
            ),
            PortError::ReparsePoint => self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            ),
            PortError::Changed => self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            ),
            PortError::Other => self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            ),
        }
    }

    fn record_directory_open_error(&mut self, display: &DisplayRelativePath, error: PortError) {
        match error {
            PortError::AccessDenied => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::DirectoryDenied,
                Outcome::Denied,
            ),
            PortError::NotFound => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::DirectoryNotFound,
                Outcome::Partial,
            ),
            PortError::ResourceLimit => self.status = Some(Outcome::ResourceLimit),
            PortError::ReparsePoint | PortError::Changed => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::DirectoryChanged,
                Outcome::Partial,
            ),
            PortError::Unsupported | PortError::Other => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::DirectoryRead,
                Outcome::Partial,
            ),
        }
    }

    fn record_directory_cursor_error(&mut self, relative: &Path, error: PortError) {
        let display = display_path_or_none(relative);
        match error {
            PortError::AccessDenied => self.add_failure(
                display,
                CoverageFailureKind::DirectoryDenied,
                Outcome::Denied,
            ),
            PortError::NotFound => self.add_failure(
                display,
                CoverageFailureKind::DirectoryNotFound,
                Outcome::Partial,
            ),
            PortError::ResourceLimit => self.status = Some(Outcome::ResourceLimit),
            PortError::ReparsePoint | PortError::Changed => self.add_failure(
                display,
                CoverageFailureKind::DirectoryChanged,
                Outcome::Partial,
            ),
            PortError::Unsupported | PortError::Other => self.add_failure(
                display,
                CoverageFailureKind::DirectoryRead,
                Outcome::Partial,
            ),
        }
    }

    fn record_metadata_error(&mut self, display: &DisplayRelativePath, error: PortError) {
        match error {
            PortError::AccessDenied => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::EntryDenied,
                Outcome::Denied,
            ),
            PortError::NotFound => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::EntryDisappeared,
                Outcome::Partial,
            ),
            PortError::ResourceLimit => self.status = Some(Outcome::ResourceLimit),
            PortError::ReparsePoint => {
                self.add_exclusion(display.clone(), ExclusionReason::ReparsePoint)
            }
            PortError::Changed => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::DirectoryChanged,
                Outcome::Partial,
            ),
            PortError::Unsupported | PortError::Other => self.add_failure(
                Some(display.clone()),
                CoverageFailureKind::MetadataRead,
                Outcome::Partial,
            ),
        }
    }

    fn add_failure(
        &mut self,
        display_path: Option<DisplayRelativePath>,
        kind: CoverageFailureKind,
        outcome: Outcome,
    ) {
        if self.report.coverage_failures.len() < self.limits.max_diagnostics {
            self.report
                .coverage_failures
                .push(CoverageFailure { display_path, kind });
        } else {
            self.report.omitted_coverage_failures =
                self.report.omitted_coverage_failures.saturating_add(1);
        }
        self.status = Some(combine_outcomes(self.status, outcome));
    }

    fn add_exclusion(&mut self, display_path: DisplayRelativePath, reason: ExclusionReason) {
        if self.report.exclusions.len() < self.limits.max_exclusions {
            self.report.exclusions.push(PolicyExclusion {
                display_path,
                reason,
            });
        } else {
            self.report.omitted_exclusions = self.report.omitted_exclusions.saturating_add(1);
        }
    }

    fn emit_progress(&mut self, force: bool, outcome: Option<Outcome>) {
        let now = Instant::now();
        let should_emit = force
            || self
                .last_progress
                .is_none_or(|last| now.duration_since(last) >= self.limits.progress_interval);
        if should_emit {
            self.last_progress = Some(now);
            self.progress.report(ProgressUpdate {
                directories_visited: self.report.directories_visited,
                entries_examined: self.report.entries_examined,
                observations_discovered: self.report.observations_discovered,
                final_update: force,
                outcome,
            });
        }
    }
}

fn combine_outcomes(current: Option<Outcome>, next: Outcome) -> Outcome {
    let rank = |outcome: Outcome| match outcome {
        Outcome::Complete => 0,
        Outcome::Partial => 1,
        Outcome::Denied => 2,
        Outcome::UnsupportedFilesystem => 3,
        Outcome::RootUnavailable => 4,
        Outcome::Invalid => 5,
        Outcome::SinkFailed => 6,
        Outcome::ResourceLimit => 7,
        Outcome::Cancelled => 8,
    };
    current.map_or(next, |current| {
        if rank(next) > rank(current) {
            next
        } else {
            current
        }
    })
}

fn display_path_or_none(relative: &Path) -> Option<DisplayRelativePath> {
    if relative.as_os_str().is_empty() {
        None
    } else {
        normalize_relative_path(relative)
            .ok()
            .map(|(display, _)| display)
    }
}

fn validate_entry_name(name: &OsStr) -> Result<(), PathError> {
    let mut components = Path::new(name).components();
    let Some(Component::Normal(component)) = components.next() else {
        return Err(PathError::InvalidComponent);
    };
    if components.next().is_some() {
        return Err(PathError::InvalidComponent);
    }
    let value = component.to_str().ok_or(PathError::InvalidUnicode)?;
    validate_component(value)
}

fn is_flp_name(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("flp"))
}

/// The production Windows port. On non-Windows hosts it reports
/// `Unsupported`; fake ports keep deterministic tests portable.
pub struct WindowsFilesystemPort {
    qualification: Option<FilesystemQualification>,
}

impl WindowsFilesystemPort {
    pub fn new() -> Self {
        Self {
            qualification: None,
        }
    }
}

impl Default for WindowsFilesystemPort {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(windows))]
impl FilesystemPort for WindowsFilesystemPort {
    fn inspect_root(&mut self, _root: &Path) -> Result<RootMetadata, PortError> {
        Err(PortError::Unsupported)
    }

    fn open_root(&mut self, _root: &Path) -> Result<OpenedDirectory, PortError> {
        Err(PortError::Unsupported)
    }
}

#[cfg(windows)]
mod windows_port {
    use super::*;
    use std::ffi::c_void;
    use std::mem::{MaybeUninit, size_of};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::ptr::{null, null_mut};
    use std::rc::Rc;

    const FILE_LIST_DIRECTORY: u32 = 0x0001;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0010;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
    const FILE_SYNCHRONOUS_IO_NONALERT: u32 = 0x0000_0020;
    const FILE_OPEN_REPARSE_POINT_OPTION: u32 = 0x0020_0000;
    const FILE_NAMES_INFORMATION_CLASS: i32 = 12;
    const FILE_BASIC_INFORMATION_CLASS: i32 = 0;
    const FILE_STANDARD_INFORMATION_CLASS: i32 = 1;
    const FILE_ATTRIBUTE_TAG_INFORMATION_CLASS: i32 = 9;
    const FILE_ID_INFO_CLASS: i32 = 18;
    const FILE_CASE_SENSITIVE_INFORMATION_CLASS: i32 = 23;
    const FILE_CS_FLAG_CASE_SENSITIVE_DIR: u32 = 0x0000_0001;
    const DIRECTORY_BUFFER_BYTES: usize = 64 * 1024;
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
    const ERROR_FILE_NOT_FOUND: u32 = 2;
    const ERROR_PATH_NOT_FOUND: u32 = 3;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_INVALID_FUNCTION: u32 = 1;
    const ERROR_INVALID_NAME: u32 = 123;
    const ERROR_INVALID_PARAMETER: u32 = 87;
    const ERROR_NOT_SUPPORTED: u32 = 50;
    const ERROR_SHARING_VIOLATION: u32 = 32;
    const DRIVE_FIXED: u32 = 3;
    const STATUS_NO_MORE_FILES: u32 = 0x8000_0006;
    const STATUS_BUFFER_OVERFLOW: u32 = 0x8000_0005;
    const STATUS_OBJECT_NAME_NOT_FOUND: u32 = 0xC000_0034;
    const STATUS_OBJECT_PATH_NOT_FOUND: u32 = 0xC000_003A;
    const STATUS_OBJECT_NAME_INVALID: u32 = 0xC000_0033;
    const STATUS_NOT_A_DIRECTORY: u32 = 0xC000_0103;
    const STATUS_ACCESS_DENIED: u32 = 0xC000_0022;
    const STATUS_SHARING_VIOLATION: u32 = 0xC000_0043;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileIdInfo {
        volume_serial_number: u64,
        file_id: [u8; 16],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileBasicInfo {
        creation_time: i64,
        last_access_time: i64,
        last_write_time: i64,
        change_time: i64,
        file_attributes: u32,
        reserved: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileStandardInfo {
        allocation_size: i64,
        end_of_file: i64,
        number_of_links: u32,
        delete_pending: u8,
        directory: u8,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileAttributeTagInfo {
        file_attributes: u32,
        reparse_tag: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FileCaseSensitiveInfo {
        flags: u32,
    }

    #[repr(C)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *mut u16,
    }

    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: *mut c_void,
        object_name: *mut UnicodeString,
        attributes: u32,
        security_descriptor: *mut c_void,
        security_quality_of_service: *mut c_void,
    }

    #[repr(C)]
    struct IoStatusBlock {
        status: i32,
        information: usize,
    }

    unsafe extern "system" {
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: *mut c_void,
        ) -> *mut c_void;
        fn GetFileInformationByHandleEx(
            handle: *mut c_void,
            file_information_class: i32,
            file_information: *mut c_void,
            buffer_size: u32,
        ) -> i32;
        fn GetLastError() -> u32;
        fn GetDriveTypeW(root_path_name: *const u16) -> u32;
        fn GetVolumeInformationW(
            root_path_name: *const u16,
            volume_name_buffer: *mut u16,
            volume_name_size: u32,
            volume_serial_number: *mut u32,
            maximum_component_length: *mut u32,
            file_system_flags: *mut u32,
            file_system_name_buffer: *mut u16,
            file_system_name_size: u32,
        ) -> i32;
        fn GetVolumePathNameW(
            file_name: *const u16,
            volume_path_name: *mut u16,
            buffer_length: u32,
        ) -> i32;
        fn NtCreateFile(
            file_handle: *mut *mut c_void,
            desired_access: u32,
            object_attributes: *mut ObjectAttributes,
            io_status_block: *mut IoStatusBlock,
            allocation_size: *mut i64,
            file_attributes: u32,
            share_access: u32,
            create_disposition: u32,
            create_options: u32,
            ea_buffer: *mut c_void,
            ea_length: u32,
        ) -> i32;
        fn NtQueryDirectoryFile(
            file_handle: *mut c_void,
            event: *mut c_void,
            apc_routine: *mut c_void,
            apc_context: *mut c_void,
            io_status_block: *mut IoStatusBlock,
            file_information: *mut c_void,
            length: u32,
            file_information_class: i32,
            return_single_entry: u8,
            file_name: *mut UnicodeString,
            restart_scan: u8,
        ) -> i32;
    }

    struct WindowsHandle(*mut c_void);

    impl WindowsHandle {
        fn raw(&self) -> *mut c_void {
            self.0
        }
    }

    impl Drop for WindowsHandle {
        fn drop(&mut self) {
            if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    struct ValidationChain {
        parent: Option<Rc<ValidationChain>>,
        parent_handle: Rc<WindowsHandle>,
        name: Vec<u16>,
        identity: QualifiedIdentity,
    }

    pub(super) struct WindowsDirectoryCursor {
        handle: Rc<WindowsHandle>,
        qualification: FilesystemQualification,
        validation_chain: Option<Rc<ValidationChain>>,
        restart_scan: bool,
        buffer: [u8; DIRECTORY_BUFFER_BYTES],
    }

    impl DirectoryCursor for WindowsDirectoryCursor {
        fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
            loop {
                self.validate_ancestors()?;
                let mut status = IoStatusBlock {
                    status: 0,
                    information: 0,
                };
                let restart_scan = u8::from(!self.restart_scan);
                let result = unsafe {
                    NtQueryDirectoryFile(
                        self.handle.raw(),
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        &mut status,
                        self.buffer.as_mut_ptr().cast::<c_void>(),
                        self.buffer.len() as u32,
                        FILE_NAMES_INFORMATION_CLASS,
                        1,
                        null_mut(),
                        restart_scan,
                    )
                };
                self.restart_scan = true;
                match result as u32 {
                    0 => {
                        let information = status.information.min(self.buffer.len());
                        let name = parse_file_names_information(&self.buffer, information)?;
                        if name == OsStr::new(".") || name == OsStr::new("..") {
                            continue;
                        }
                        return Ok(Some(DirectoryEntry::new(name)));
                    }
                    STATUS_BUFFER_OVERFLOW => {
                        let information = status.information.min(self.buffer.len());
                        let name = parse_file_names_information(&self.buffer, information)?;
                        if name == OsStr::new(".") || name == OsStr::new("..") {
                            continue;
                        }
                        return Ok(Some(DirectoryEntry::new(name)));
                    }
                    STATUS_NO_MORE_FILES => return Ok(None),
                    _ => return Err(map_nt_status(result)),
                }
            }
        }

        fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
            self.validate_ancestors()?;
            let handle = open_relative(
                &self.handle,
                &entry.name,
                FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                FILE_OPEN_REPARSE_POINT_OPTION | FILE_SYNCHRONOUS_IO_NONALERT,
            )?;
            read_handle_metadata(&handle, self.qualification)
        }

        fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
            self.validate_ancestors()?;
            let handle = open_relative(
                &self.handle,
                &entry.name,
                FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT_OPTION | FILE_SYNCHRONOUS_IO_NONALERT,
            )?;
            let metadata = read_handle_metadata(&handle, self.qualification)?;
            if metadata.reparse_point {
                return Err(PortError::ReparsePoint);
            }
            if metadata.kind != EntryKind::Directory {
                return Err(PortError::Other);
            }
            let identity = metadata.identity.clone().ok_or(PortError::Changed)?;
            let handle = Rc::new(handle);
            let validation_chain = Some(Rc::new(ValidationChain {
                parent: self.validation_chain.clone(),
                parent_handle: Rc::clone(&self.handle),
                name: wide_name(&entry.name)?,
                identity,
            }));
            let case_sensitivity = query_case_sensitivity(&handle)?;
            let cursor = WindowsDirectoryCursor {
                handle: Rc::clone(&handle),
                qualification: self.qualification,
                validation_chain,
                restart_scan: false,
                buffer: [0; DIRECTORY_BUFFER_BYTES],
            };
            Ok(OpenedDirectory {
                metadata,
                case_sensitivity,
                cursor: Box::new(cursor),
            })
        }
    }

    impl WindowsDirectoryCursor {
        fn root(handle: Rc<WindowsHandle>, qualification: FilesystemQualification) -> Self {
            Self {
                handle,
                qualification,
                validation_chain: None,
                restart_scan: false,
                buffer: [0; DIRECTORY_BUFFER_BYTES],
            }
        }

        fn validate_ancestors(&self) -> Result<(), PortError> {
            let mut chain = self.validation_chain.as_deref();
            while let Some(link) = chain {
                let handle = open_relative(
                    &link.parent_handle,
                    &OsString::from_wide(&link.name),
                    FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                    FILE_OPEN_REPARSE_POINT_OPTION | FILE_SYNCHRONOUS_IO_NONALERT,
                )?;
                let metadata = read_handle_metadata(&handle, self.qualification)?;
                if metadata.reparse_point || metadata.kind != EntryKind::Directory {
                    return Err(PortError::Changed);
                }
                if metadata.identity.as_ref() != Some(&link.identity) {
                    return Err(PortError::Changed);
                }
                chain = link.parent.as_deref();
            }
            Ok(())
        }
    }

    impl FilesystemPort for WindowsFilesystemPort {
        fn inspect_root(&mut self, root: &Path) -> Result<RootMetadata, PortError> {
            let qualification = filesystem_qualification(root)?;
            let handle = open_path(root)?;
            let metadata = read_handle_metadata(&handle, qualification)?;
            self.qualification = Some(qualification);
            Ok(RootMetadata {
                metadata,
                qualification,
            })
        }

        fn open_root(&mut self, root: &Path) -> Result<OpenedDirectory, PortError> {
            let qualification = self.qualification.ok_or(PortError::Unsupported)?;
            let handle = Rc::new(open_path(root)?);
            let metadata = read_handle_metadata(&handle, qualification)?;
            if metadata.reparse_point {
                return Err(PortError::ReparsePoint);
            }
            if metadata.kind != EntryKind::Directory {
                return Err(PortError::Other);
            }
            let case_sensitivity = query_case_sensitivity(&handle)?;
            let cursor = WindowsDirectoryCursor::root(Rc::clone(&handle), qualification);
            Ok(OpenedDirectory {
                metadata,
                case_sensitivity,
                cursor: Box::new(cursor),
            })
        }
    }

    fn open_path(path: &Path) -> Result<WindowsHandle, PortError> {
        let wide = wide_null(&extended_path(path));
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(map_win_error(last_error()));
        }
        Ok(WindowsHandle(handle))
    }

    fn open_relative(
        parent: &WindowsHandle,
        name: &OsStr,
        desired_access: u32,
        create_options: u32,
    ) -> Result<WindowsHandle, PortError> {
        let mut name = wide_name(name)?;
        let mut unicode_name = UnicodeString {
            length: (name.len() * size_of::<u16>()) as u16,
            maximum_length: (name.len() * size_of::<u16>()) as u16,
            buffer: name.as_mut_ptr(),
        };
        let mut attributes = ObjectAttributes {
            length: size_of::<ObjectAttributes>() as u32,
            root_directory: parent.raw(),
            object_name: &mut unicode_name,
            attributes: 0,
            security_descriptor: null_mut(),
            security_quality_of_service: null_mut(),
        };
        let mut io_status = IoStatusBlock {
            status: 0,
            information: 0,
        };
        let mut handle = null_mut();
        let status = unsafe {
            NtCreateFile(
                &mut handle,
                desired_access,
                &mut attributes,
                &mut io_status,
                null_mut(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                OPEN_EXISTING,
                create_options,
                null_mut(),
                0,
            )
        };
        if status < 0 {
            return Err(map_nt_status(status));
        }
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(PortError::Other);
        }
        Ok(WindowsHandle(handle))
    }

    fn wide_name(name: &OsStr) -> Result<Vec<u16>, PortError> {
        let name: Vec<u16> = name.encode_wide().collect();
        if name.is_empty() || name.len() > (u16::MAX as usize / size_of::<u16>()) {
            return Err(PortError::Other);
        }
        Ok(name)
    }

    fn read_handle_metadata(
        handle: &WindowsHandle,
        qualification: FilesystemQualification,
    ) -> Result<FileMetadata, PortError> {
        let basic = query_handle_info::<FileBasicInfo>(handle, FILE_BASIC_INFORMATION_CLASS)?;
        let standard =
            query_handle_info::<FileStandardInfo>(handle, FILE_STANDARD_INFORMATION_CLASS)?;
        let tag = query_handle_info::<FileAttributeTagInfo>(
            handle,
            FILE_ATTRIBUTE_TAG_INFORMATION_CLASS,
        )?;
        if standard.end_of_file < 0 {
            return Err(PortError::Other);
        }
        let attributes = basic.file_attributes | tag.file_attributes;
        let reparse_point = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        let recall_or_offline = attributes
            & (FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0;
        let kind = if standard.directory != 0 || attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            EntryKind::Directory
        } else if attributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        let modified_unix_ns = windows_filetime_100ns_to_unix_ns(basic.last_write_time);
        let identity = if qualification == FilesystemQualification::LocalNtfs
            && !reparse_point
            && !recall_or_offline
        {
            query_file_id(handle)?
        } else {
            None
        };
        Ok(FileMetadata {
            kind,
            byte_size: standard.end_of_file as u64,
            modified_unix_ns,
            identity,
            reparse_point,
            recall_or_offline,
        })
    }

    fn query_file_id(handle: &WindowsHandle) -> Result<Option<QualifiedIdentity>, PortError> {
        match query_handle_info::<FileIdInfo>(handle, FILE_ID_INFO_CLASS) {
            Ok(info) => Ok(Some(QualifiedIdentity {
                volume_serial: info.volume_serial_number,
                file_id: windows_file_id_to_u128(info.file_id),
                qualification: IdentityQualification::LocalNtfs,
            })),
            Err(PortError::NotFound) => Err(PortError::NotFound),
            Err(PortError::ResourceLimit) => Err(PortError::ResourceLimit),
            Err(PortError::AccessDenied | PortError::Unsupported | PortError::Other) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn query_case_sensitivity(
        handle: &WindowsHandle,
    ) -> Result<DirectoryCaseSensitivity, PortError> {
        let info = query_handle_info::<FileCaseSensitiveInfo>(
            handle,
            FILE_CASE_SENSITIVE_INFORMATION_CLASS,
        )?;
        if info.flags & FILE_CS_FLAG_CASE_SENSITIVE_DIR != 0 {
            Ok(DirectoryCaseSensitivity::Sensitive)
        } else {
            Ok(DirectoryCaseSensitivity::Insensitive)
        }
    }

    fn query_handle_info<T: Copy>(
        handle: &WindowsHandle,
        information_class: i32,
    ) -> Result<T, PortError> {
        let mut info = MaybeUninit::<T>::zeroed();
        let succeeded = unsafe {
            GetFileInformationByHandleEx(
                handle.raw(),
                information_class,
                info.as_mut_ptr().cast::<c_void>(),
                size_of::<T>() as u32,
            ) != 0
        };
        if !succeeded {
            return Err(map_win_error(last_error()));
        }
        Ok(unsafe { info.assume_init() })
    }

    fn filesystem_qualification(path: &Path) -> Result<FilesystemQualification, PortError> {
        let volume_path = volume_path(path)?;
        let wide = wide_null(&volume_path);
        if unsafe { GetDriveTypeW(wide.as_ptr()) } != DRIVE_FIXED {
            return Ok(FilesystemQualification::Unqualified);
        }
        let mut file_system_name = [0u16; 256];
        let mut serial = 0u32;
        let mut maximum_component_length = 0u32;
        let mut flags = 0u32;
        let succeeded = unsafe {
            GetVolumeInformationW(
                wide.as_ptr(),
                null_mut(),
                0,
                &mut serial,
                &mut maximum_component_length,
                &mut flags,
                file_system_name.as_mut_ptr(),
                file_system_name.len() as u32,
            ) != 0
        };
        if !succeeded {
            return Err(map_win_error(last_error()));
        }
        let name = String::from_utf16_lossy(&file_system_name);
        if name.trim_end_matches('\0').eq_ignore_ascii_case("NTFS") {
            Ok(FilesystemQualification::LocalNtfs)
        } else {
            Ok(FilesystemQualification::Unqualified)
        }
    }

    fn volume_path(path: &Path) -> Result<PathBuf, PortError> {
        let wide = wide_null(&extended_path(path));
        let mut output = vec![0u16; 32_768];
        let succeeded = unsafe {
            GetVolumePathNameW(wide.as_ptr(), output.as_mut_ptr(), output.len() as u32) != 0
        };
        if !succeeded {
            return Err(map_win_error(last_error()));
        }
        let nul = output
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(output.len());
        Ok(PathBuf::from(OsString::from_wide(&output[..nul])))
    }

    fn extended_path(path: &Path) -> PathBuf {
        let text = path.to_string_lossy();
        if text.starts_with(r"\\?\") || text.starts_with(r"\\.\") {
            return path.to_owned();
        }
        if let Some(unc) = text.strip_prefix(r"\\") {
            return PathBuf::from(format!(r"\\?\UNC\{unc}"));
        }
        if text.as_bytes().get(1) == Some(&b':') {
            return PathBuf::from(format!(r"\\?\{text}"));
        }
        path.to_owned()
    }

    fn wide_null(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain([0]).collect()
    }

    fn last_error() -> u32 {
        unsafe { GetLastError() }
    }

    fn parse_file_names_information(
        buffer: &[u8],
        information: usize,
    ) -> Result<OsString, PortError> {
        const HEADER_BYTES: usize = 12;
        if information < HEADER_BYTES || buffer.len() < HEADER_BYTES {
            return Err(PortError::Other);
        }
        let name_length = u32::from_ne_bytes(buffer[8..12].try_into().unwrap()) as usize;
        if name_length == 0 || !name_length.is_multiple_of(size_of::<u16>()) {
            return Err(PortError::Other);
        }
        let end = HEADER_BYTES.saturating_add(name_length);
        if end > information || end > buffer.len() {
            return Err(PortError::Other);
        }
        let words = &buffer[HEADER_BYTES..end];
        let words = words
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_ne_bytes(*bytes))
            .collect::<Vec<_>>();
        Ok(OsString::from_wide(&words))
    }

    fn map_nt_status(status: i32) -> PortError {
        match status as u32 {
            STATUS_OBJECT_NAME_NOT_FOUND | STATUS_OBJECT_PATH_NOT_FOUND => PortError::NotFound,
            STATUS_OBJECT_NAME_INVALID => PortError::NotFound,
            STATUS_NOT_A_DIRECTORY => PortError::Other,
            STATUS_ACCESS_DENIED | STATUS_SHARING_VIOLATION => PortError::AccessDenied,
            _ => PortError::Other,
        }
    }

    fn map_win_error(error: u32) -> PortError {
        match error {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND | ERROR_INVALID_NAME => PortError::NotFound,
            ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION => PortError::AccessDenied,
            ERROR_INVALID_FUNCTION | ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED => {
                PortError::Unsupported
            }
            _ => PortError::Other,
        }
    }
}

#[cfg(windows)]
unsafe extern "system" {
    fn CompareStringOrdinal(
        lp_string1: *const u16,
        cch_count1: i32,
        lp_string2: *const u16,
        cch_count2: i32,
        b_ignore_case: i32,
    ) -> i32;
}

#[cfg(test)]
mod tests;
