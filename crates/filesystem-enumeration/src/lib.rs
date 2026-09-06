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

/// A root-relative, boundary-normalized locator.
///
/// On Windows equality and ordering use ordinal case-insensitive comparison,
/// matching the filesystem policy. The first observed case-preserving spelling
/// is retained for serialization and display hand-off; callers must not use
/// spelling alone as physical identity evidence.
#[derive(Clone, Debug)]
pub struct NormalizedLocator(String);

impl NormalizedLocator {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl PartialEq for NormalizedLocator {
    fn eq(&self, other: &Self) -> bool {
        compare_locator(&self.0, &other.0) == Ordering::Equal
    }
}

impl Eq for NormalizedLocator {}

impl PartialOrd for NormalizedLocator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NormalizedLocator {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_locator(&self.0, &other.0)
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

#[cfg(windows)]
fn compare_locator(left: &str, right: &str) -> Ordering {
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
fn compare_locator(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}

/// Physical identity qualified only for the researched local NTFS scope.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QualifiedIdentity {
    pub volume_serial: u64,
    pub file_id: u128,
    pub qualification: IdentityQualification,
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
    Other,
}

/// A streaming directory cursor. It returns names only; metadata is a separate
/// call so disappearing-entry races can be isolated and tested.
pub trait DirectoryCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError>;
}

/// The only filesystem authority needed by the enumerator.
pub trait FilesystemPort {
    fn inspect_root(&mut self, root: &Path) -> Result<RootMetadata, PortError>;
    fn open_directory(&mut self, directory: &Path) -> Result<Box<dyn DirectoryCursor>, PortError>;
    fn read_metadata(&mut self, path: &Path) -> Result<FileMetadata, PortError>;
}

/// The storage/publication owner implements this with run-scoped staging.
pub trait BatchSink {
    fn accept(&mut self, batch: ObservationBatch) -> Result<(), SinkError>;
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
    engine.report.outcome = outcome;
    engine.report.authoritative = outcome.is_authoritative();
    engine.emit_progress(true, Some(outcome));
    engine.report
}

struct DirectoryWork {
    absolute: PathBuf,
    relative: PathBuf,
    path_bytes: usize,
}

struct Engine<'a, P, S, C, R> {
    port: &'a mut P,
    root: &'a Path,
    limits: EnumerationLimits,
    cancellation: &'a C,
    sink: &'a mut S,
    progress: &'a mut R,
    report: EnumerationReport,
    seen: BTreeSet<NormalizedLocator>,
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
        self.stack.push(DirectoryWork {
            absolute: self.root.to_owned(),
            relative: PathBuf::new(),
            path_bytes: 0,
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
            let is_root = work.relative.as_os_str().is_empty();
            let mut cursor = match self.port.open_directory(&work.absolute) {
                Ok(cursor) => cursor,
                Err(error) => {
                    self.record_directory_port_error(&work.relative, is_root, error);
                    if error == PortError::ResourceLimit {
                        return Outcome::ResourceLimit;
                    }
                    if is_root {
                        return self.status.unwrap_or(Outcome::RootUnavailable);
                    }
                    continue;
                }
            };
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
                if !self.reserve_path_bytes(display_path.as_str().len()) {
                    self.status = Some(Outcome::ResourceLimit);
                    return Outcome::ResourceLimit;
                }
                let child_absolute = work.absolute.join(&entry.name);
                let metadata = match self.port.read_metadata(&child_absolute) {
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
                        self.pending_path_bytes =
                            self.pending_path_bytes.saturating_add(pending_path_bytes);
                        self.stack.push(DirectoryWork {
                            absolute: child_absolute,
                            relative: child_relative,
                            path_bytes: pending_path_bytes,
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
                        if !self.seen.insert(locator.clone()) {
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
            PortError::Other => self.add_failure(
                None,
                CoverageFailureKind::RootChanged,
                Outcome::RootUnavailable,
            ),
        }
    }

    fn record_directory_port_error(&mut self, relative: &Path, is_root: bool, error: PortError) {
        let display = display_path_or_none(relative);
        match error {
            PortError::AccessDenied => self.add_failure(
                display,
                if is_root {
                    CoverageFailureKind::RootDenied
                } else {
                    CoverageFailureKind::DirectoryDenied
                },
                Outcome::Denied,
            ),
            PortError::NotFound => self.add_failure(
                display,
                if is_root {
                    CoverageFailureKind::RootNotFound
                } else {
                    CoverageFailureKind::DirectoryNotFound
                },
                if is_root {
                    Outcome::RootUnavailable
                } else {
                    Outcome::Partial
                },
            ),
            PortError::ResourceLimit => self.status = Some(Outcome::ResourceLimit),
            PortError::ReparsePoint => {
                if is_root {
                    self.add_failure(
                        None,
                        CoverageFailureKind::RootChanged,
                        Outcome::RootUnavailable,
                    );
                } else if let Some(display) = display {
                    self.add_exclusion(display, ExclusionReason::ReparsePoint);
                }
            }
            PortError::Unsupported | PortError::Other => self.add_failure(
                display,
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
            PortError::ReparsePoint => {
                if let Some(display) = display {
                    self.add_exclusion(display, ExclusionReason::ReparsePoint);
                } else {
                    self.add_failure(
                        None,
                        CoverageFailureKind::RootChanged,
                        Outcome::RootUnavailable,
                    );
                }
            }
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

    fn open_directory(&mut self, _directory: &Path) -> Result<Box<dyn DirectoryCursor>, PortError> {
        Err(PortError::Unsupported)
    }

    fn read_metadata(&mut self, _path: &Path) -> Result<FileMetadata, PortError> {
        Err(PortError::Unsupported)
    }
}

#[cfg(windows)]
mod windows_port {
    use super::*;
    use std::ffi::c_void;
    use std::fs;
    use std::mem::size_of;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::fs::MetadataExt;
    use std::ptr::{null, null_mut};
    use std::time::{SystemTime, UNIX_EPOCH};

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
    const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
    const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
    const FILE_READ_ATTRIBUTES: u32 = 0x0080;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ID_INFO_CLASS: i32 = 18;
    const INVALID_HANDLE_VALUE: *mut c_void = -1isize as *mut c_void;
    const ERROR_FILE_NOT_FOUND: u32 = 2;
    const ERROR_PATH_NOT_FOUND: u32 = 3;
    const ERROR_ACCESS_DENIED: u32 = 5;
    const ERROR_INVALID_NAME: u32 = 123;
    const ERROR_SHARING_VIOLATION: u32 = 32;
    const DRIVE_FIXED: u32 = 3;

    #[repr(C)]
    struct FileIdInfo {
        volume_serial_number: u64,
        file_id: [u8; 16],
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
    }

    pub(super) struct WindowsDirectoryCursor {
        entries: fs::ReadDir,
    }

    impl DirectoryCursor for WindowsDirectoryCursor {
        fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
            match self.entries.next() {
                Some(Ok(entry)) => Ok(Some(DirectoryEntry::new(entry.file_name()))),
                Some(Err(error)) => Err(map_io_error(error)),
                None => Ok(None),
            }
        }
    }

    impl FilesystemPort for WindowsFilesystemPort {
        fn inspect_root(&mut self, root: &Path) -> Result<RootMetadata, PortError> {
            let qualification = filesystem_qualification(root)?;
            let metadata = read_metadata(root, qualification)?;
            self.qualification = Some(qualification);
            Ok(RootMetadata {
                metadata,
                qualification,
            })
        }

        fn open_directory(
            &mut self,
            directory: &Path,
        ) -> Result<Box<dyn DirectoryCursor>, PortError> {
            let io_path = extended_path(directory);
            let metadata = fs::symlink_metadata(&io_path).map_err(map_io_error)?;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(PortError::ReparsePoint);
            }
            if !metadata.is_dir() {
                return Err(PortError::Other);
            }
            let entries = fs::read_dir(io_path).map_err(map_io_error)?;
            Ok(Box::new(WindowsDirectoryCursor { entries }))
        }

        fn read_metadata(&mut self, path: &Path) -> Result<FileMetadata, PortError> {
            read_metadata(
                path,
                self.qualification
                    .unwrap_or(FilesystemQualification::Unqualified),
            )
        }
    }

    fn read_metadata(
        path: &Path,
        qualification: FilesystemQualification,
    ) -> Result<FileMetadata, PortError> {
        let io_path = extended_path(path);
        let metadata = fs::symlink_metadata(&io_path).map_err(map_io_error)?;
        let attributes = metadata.file_attributes();
        let reparse_point = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        let recall_or_offline = attributes
            & (FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0;
        let kind = if metadata.is_dir() {
            EntryKind::Directory
        } else if metadata.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        let modified_unix_ns = system_time_to_unix_ns(metadata.modified().map_err(map_io_error)?)?;
        let identity = if qualification == FilesystemQualification::LocalNtfs
            && !reparse_point
            && !recall_or_offline
        {
            query_file_id(&io_path)?
        } else {
            None
        };
        Ok(FileMetadata {
            kind,
            byte_size: metadata.len(),
            modified_unix_ns,
            identity,
            reparse_point,
            recall_or_offline,
        })
    }

    fn query_file_id(path: &Path) -> Result<Option<QualifiedIdentity>, PortError> {
        let wide = wide_null(path);
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let error = map_win_error(last_error());
            return match error {
                PortError::NotFound => Err(error),
                PortError::AccessDenied | PortError::Other => Ok(None),
                other => Err(other),
            };
        }
        let mut info = FileIdInfo {
            volume_serial_number: 0,
            file_id: [0; 16],
        };
        let succeeded = unsafe {
            GetFileInformationByHandleEx(
                handle,
                FILE_ID_INFO_CLASS,
                (&mut info as *mut FileIdInfo).cast::<c_void>(),
                size_of::<FileIdInfo>() as u32,
            ) != 0
        };
        let close_succeeded = unsafe { CloseHandle(handle) != 0 };
        if !close_succeeded {
            return Ok(None);
        }
        if !succeeded {
            return match map_win_error(last_error()) {
                PortError::NotFound => Err(PortError::NotFound),
                PortError::ResourceLimit => Err(PortError::ResourceLimit),
                _ => Ok(None),
            };
        }
        Ok(Some(QualifiedIdentity {
            volume_serial: info.volume_serial_number,
            file_id: u128::from_le_bytes(info.file_id),
            qualification: IdentityQualification::LocalNtfs,
        }))
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

    fn system_time_to_unix_ns(value: SystemTime) -> Result<i128, PortError> {
        match value.duration_since(UNIX_EPOCH) {
            Ok(duration) => Ok((duration.as_secs() as i128)
                .saturating_mul(1_000_000_000)
                .saturating_add(duration.subsec_nanos() as i128)),
            Err(error) => {
                let duration = error.duration();
                Ok(-((duration.as_secs() as i128)
                    .saturating_mul(1_000_000_000)
                    .saturating_add(duration.subsec_nanos() as i128)))
            }
        }
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

    fn map_io_error(error: std::io::Error) -> PortError {
        if let Some(code) = error.raw_os_error() {
            return map_win_error(code as u32);
        }
        match error.kind() {
            std::io::ErrorKind::PermissionDenied => PortError::AccessDenied,
            std::io::ErrorKind::NotFound => PortError::NotFound,
            _ => PortError::Other,
        }
    }

    fn map_win_error(error: u32) -> PortError {
        match error {
            ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND | ERROR_INVALID_NAME => PortError::NotFound,
            ERROR_ACCESS_DENIED | ERROR_SHARING_VIOLATION => PortError::AccessDenied,
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
