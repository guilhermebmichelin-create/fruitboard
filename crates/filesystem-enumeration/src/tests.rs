use super::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Default)]
struct RecordingSink {
    batches: Vec<ObservationBatch>,
    error: Option<SinkError>,
    discarded: bool,
}

impl BatchSink for RecordingSink {
    fn accept(&mut self, batch: ObservationBatch) -> Result<(), SinkError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.batches.push(batch);
        Ok(())
    }

    fn discard(&mut self) {
        self.batches.clear();
        self.discarded = true;
    }
}

#[derive(Default)]
struct RecordingRunStorage {
    staged: BTreeMap<String, Vec<ObservationBatch>>,
    invalidated: Vec<String>,
    error: Option<SinkError>,
}

impl RunScopedStorage for RecordingRunStorage {
    fn stage_batch(&mut self, run_id: &str, batch: ObservationBatch) -> Result<(), SinkError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.staged
            .entry(run_id.to_owned())
            .or_default()
            .push(batch);
        Ok(())
    }

    fn invalidate_run(&mut self, run_id: &str) {
        self.staged.remove(run_id);
        self.invalidated.push(run_id.to_owned());
    }
}

#[derive(Default)]
struct RecordingProgress {
    updates: Vec<ProgressUpdate>,
}

impl ProgressSink for RecordingProgress {
    fn report(&mut self, update: ProgressUpdate) {
        self.updates.push(update);
    }
}

struct FakeCursor {
    state: Rc<RefCell<FakeState>>,
    key: String,
    links: Vec<FakeLink>,
    entries: VecDeque<Result<DirectoryEntry, PortError>>,
}

#[derive(Clone)]
struct FakeLink {
    key: String,
    generation: u64,
}

impl DirectoryCursor for FakeCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
        {
            let mut state = self.state.borrow_mut();
            if state.replace_before_next_entry.remove(&self.key) {
                replace_with_junction(&mut state, &self.key);
            }
        }
        self.ensure_current()?;
        self.entries.pop_front().transpose()
    }

    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
        self.ensure_current()?;
        let key = child_key(&self.key, &entry.name);
        let state = self.state.borrow_mut();
        if let Some((cancel_key, token)) = &state.cancel_on_metadata
            && cancel_key == &key
        {
            token.cancel();
        }
        state
            .metadata
            .get(&key)
            .cloned()
            .unwrap_or(Err(PortError::NotFound))
    }

    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
        self.ensure_current()?;
        let key = child_key(&self.key, &entry.name);
        let mut state = self.state.borrow_mut();
        if state.replace_before_open.remove(&key) {
            replace_with_junction(&mut state, &key);
            return Err(PortError::ReparsePoint);
        }
        let metadata = state
            .metadata
            .get(&key)
            .cloned()
            .unwrap_or(Err(PortError::NotFound))?;
        if metadata.reparse_point || state.reparse.contains(&key) {
            return Err(PortError::ReparsePoint);
        }
        if metadata.kind != EntryKind::Directory {
            return Err(PortError::Other);
        }
        let entries = state
            .directories
            .get(&key)
            .cloned()
            .ok_or(PortError::NotFound)?;
        state.opened.push(key.clone());
        let generation = state.generations.get(&key).copied().unwrap_or(0);
        let mut links = self.links.clone();
        links.push(FakeLink {
            key: key.clone(),
            generation,
        });
        let case_sensitivity = state
            .case_sensitivity
            .get(&key)
            .copied()
            .unwrap_or_default();
        let cursor = FakeCursor {
            state: Rc::clone(&self.state),
            key,
            links,
            entries: entries.into_iter().collect(),
        };
        Ok(OpenedDirectory {
            metadata,
            case_sensitivity,
            cursor: Box::new(cursor),
        })
    }
}

impl FakeCursor {
    fn ensure_current(&self) -> Result<(), PortError> {
        let state = self.state.borrow();
        if self.links.iter().any(|link| {
            state.reparse.contains(&link.key)
                || state.generations.get(&link.key).copied().unwrap_or(0) != link.generation
        }) {
            Err(PortError::Changed)
        } else {
            Ok(())
        }
    }
}

struct FakePort {
    state: Rc<RefCell<FakeState>>,
}

struct FakeState {
    root_results: VecDeque<Result<RootMetadata, PortError>>,
    directories: BTreeMap<String, Vec<Result<DirectoryEntry, PortError>>>,
    metadata: BTreeMap<String, Result<FileMetadata, PortError>>,
    case_sensitivity: BTreeMap<String, DirectoryCaseSensitivity>,
    generations: BTreeMap<String, u64>,
    reparse: BTreeSet<String>,
    replace_before_open: BTreeSet<String>,
    replace_before_next_entry: BTreeSet<String>,
    replace_root_before_open: bool,
    opened: Vec<String>,
    outside_accesses: usize,
    cancel_on_open: Option<CancellationToken>,
    cancel_on_metadata: Option<(String, CancellationToken)>,
}

impl FakePort {
    fn new(_root: &Path) -> Self {
        let root_metadata = RootMetadata {
            metadata: directory_metadata(900),
            qualification: FilesystemQualification::LocalNtfs,
        };
        Self {
            state: Rc::new(RefCell::new(FakeState {
                root_results: VecDeque::from([Ok(root_metadata)]),
                directories: BTreeMap::new(),
                metadata: BTreeMap::new(),
                case_sensitivity: BTreeMap::new(),
                generations: BTreeMap::new(),
                reparse: BTreeSet::new(),
                replace_before_open: BTreeSet::new(),
                replace_before_next_entry: BTreeSet::new(),
                replace_root_before_open: false,
                opened: Vec::new(),
                outside_accesses: 0,
                cancel_on_open: None,
                cancel_on_metadata: None,
            })),
        }
    }

    fn root_metadata(&self) -> RootMetadata {
        RootMetadata {
            metadata: directory_metadata(900),
            qualification: FilesystemQualification::LocalNtfs,
        }
    }

    fn add_directory(&mut self, relative: &str, entries: Vec<Result<DirectoryEntry, PortError>>) {
        self.state
            .borrow_mut()
            .directories
            .insert(relative.to_owned(), entries);
    }

    fn add_file(&mut self, relative: &str, metadata: Result<FileMetadata, PortError>) {
        self.state
            .borrow_mut()
            .metadata
            .insert(relative.to_owned(), metadata);
    }

    fn add_root_result(&mut self, result: Result<RootMetadata, PortError>) {
        self.state.borrow_mut().root_results.push_back(result);
    }

    fn set_root_results(&mut self, results: Vec<Result<RootMetadata, PortError>>) {
        self.state.borrow_mut().root_results = results.into_iter().collect();
    }

    fn deny_directory(&mut self, relative: &str) {
        self.state
            .borrow_mut()
            .directories
            .insert(relative.to_owned(), vec![Err(PortError::AccessDenied)]);
    }

    fn cancel_on_open(&mut self, token: CancellationToken) {
        self.state.borrow_mut().cancel_on_open = Some(token);
    }

    fn replace_before_open(&mut self, relative: &str) {
        self.state
            .borrow_mut()
            .replace_before_open
            .insert(relative.to_owned());
    }

    fn replace_before_next_entry(&mut self, relative: &str) {
        self.state
            .borrow_mut()
            .replace_before_next_entry
            .insert(relative.to_owned());
    }

    fn replace_root_before_open(&mut self) {
        self.state.borrow_mut().replace_root_before_open = true;
    }

    fn set_case_sensitivity(&mut self, relative: &str, sensitivity: DirectoryCaseSensitivity) {
        self.state
            .borrow_mut()
            .case_sensitivity
            .insert(relative.to_owned(), sensitivity);
    }

    fn outside_accesses(&self) -> usize {
        self.state.borrow().outside_accesses
    }

    fn opened(&self) -> Vec<String> {
        self.state.borrow().opened.clone()
    }
}

impl FilesystemPort for FakePort {
    fn inspect_root(&mut self, _root: &Path) -> Result<RootMetadata, PortError> {
        self.state
            .borrow_mut()
            .root_results
            .pop_front()
            .unwrap_or_else(|| Ok(self.root_metadata()))
    }

    fn open_root(&mut self, _root: &Path) -> Result<OpenedDirectory, PortError> {
        let mut state = self.state.borrow_mut();
        if state.replace_root_before_open {
            state.replace_root_before_open = false;
            replace_with_junction(&mut state, ".");
            return Err(PortError::ReparsePoint);
        }
        if let Some(token) = &state.cancel_on_open {
            token.cancel();
        }
        if state.reparse.contains(".") {
            return Err(PortError::ReparsePoint);
        }
        let entries = state
            .directories
            .get(".")
            .cloned()
            .ok_or(PortError::NotFound)?;
        let key = ".".to_owned();
        let generation = state.generations.get(&key).copied().unwrap_or(0);
        let case_sensitivity = state
            .case_sensitivity
            .get(&key)
            .copied()
            .unwrap_or_default();
        let metadata = self.root_metadata().metadata;
        let cursor = FakeCursor {
            state: Rc::clone(&self.state),
            key: key.clone(),
            links: vec![FakeLink { key, generation }],
            entries: entries.into_iter().collect(),
        };
        Ok(OpenedDirectory {
            metadata,
            case_sensitivity,
            cursor: Box::new(cursor),
        })
    }
}

fn child_key(parent: &str, name: &std::ffi::OsStr) -> String {
    let name = name.to_string_lossy();
    if parent == "." {
        name.into_owned()
    } else {
        format!("{parent}/{name}")
    }
}

fn replace_with_junction(state: &mut FakeState, key: &str) {
    let generation = state.generations.entry(key.to_owned()).or_default();
    *generation = generation.saturating_add(1);
    state.reparse.insert(key.to_owned());
}

fn identity(value: u128) -> QualifiedIdentity {
    QualifiedIdentity {
        volume_serial: 7,
        file_id: value,
        qualification: IdentityQualification::LocalNtfs,
    }
}

fn directory_metadata(file_id: u128) -> FileMetadata {
    FileMetadata {
        kind: EntryKind::Directory,
        byte_size: 0,
        modified_unix_ns: 100,
        identity: Some(identity(file_id)),
        reparse_point: false,
        recall_or_offline: false,
    }
}

fn file_metadata(file_id: u128, byte_size: u64) -> FileMetadata {
    FileMetadata {
        kind: EntryKind::File,
        byte_size,
        modified_unix_ns: 100,
        identity: Some(identity(file_id)),
        reparse_point: false,
        recall_or_offline: false,
    }
}

fn reparse_file_metadata() -> FileMetadata {
    FileMetadata {
        kind: EntryKind::File,
        byte_size: 0,
        modified_unix_ns: 100,
        identity: None,
        reparse_point: true,
        recall_or_offline: false,
    }
}

fn reparse_directory_metadata() -> FileMetadata {
    FileMetadata {
        kind: EntryKind::Directory,
        byte_size: 0,
        modified_unix_ns: 100,
        identity: None,
        reparse_point: true,
        recall_or_offline: false,
    }
}

fn limits() -> EnumerationLimits {
    EnumerationLimits {
        progress_interval: std::time::Duration::ZERO,
        ..EnumerationLimits::default()
    }
}

fn root_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\synthetic-root")
    } else {
        PathBuf::from("/synthetic-root")
    }
}

fn file_entry(name: &str) -> Result<DirectoryEntry, PortError> {
    Ok(DirectoryEntry::new(name))
}

fn simple_port() -> (FakePort, PathBuf) {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(
        ".",
        vec![file_entry("Mixed.FLP"), file_entry("ignored.txt")],
    );
    port.add_file("Mixed.FLP", Ok(file_metadata(1, 42)));
    port.add_file("ignored.txt", Ok(file_metadata(2, 9)));
    (port, root)
}

#[test]
fn successful_empty_root_is_explicitly_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", Vec::new());
    let mut sink = RecordingSink::default();
    let mut progress = RecordingProgress::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut progress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert_eq!(report.batches_delivered, 0);
    assert!(sink.batches.is_empty());
    assert!(
        progress
            .updates
            .last()
            .is_some_and(|update| update.final_update && update.outcome == Some(Outcome::Complete))
    );
}

#[test]
fn extension_filter_is_case_insensitive_and_metadata_stays_separate_from_content() {
    let (mut port, root) = simple_port();
    let mut sink = RecordingSink::default();
    let mut progress = NoProgress;
    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut progress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    let observations = sink
        .batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].display_path.as_str(), "Mixed.FLP");
    assert_eq!(observations[0].byte_size, 42);
}

#[test]
fn batches_are_bounded_and_keep_sequence_order() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    let mut entries = Vec::new();
    for index in 0..5 {
        let name = format!("file-{index}.flp");
        entries.push(file_entry(&name));
        port.add_file(&name, Ok(file_metadata(index + 1, index as u64)));
    }
    port.add_directory(".", entries);
    let mut configured = limits();
    configured.max_batch_records = 2;
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert_eq!(report.observations_discovered, 5);
    assert_eq!(report.batches_delivered, 3);
    assert_eq!(
        sink.batches
            .iter()
            .map(|batch| batch.records.len())
            .collect::<Vec<_>>(),
        vec![2, 2, 1]
    );
    assert_eq!(
        sink.batches
            .iter()
            .map(|batch| batch.sequence)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn resource_limit_is_non_authoritative_and_never_implies_missing() {
    let (mut port, root) = simple_port();
    let mut configured = limits();
    configured.max_entries = 1;
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::ResourceLimit);
    assert!(!report.authoritative);
    assert!(report.batches_delivered <= 1);
    assert!(sink.discarded);
}

#[test]
fn pending_directory_work_is_bounded() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("one"), file_entry("two")]);
    port.add_file(
        "one",
        Ok(FileMetadata {
            kind: EntryKind::Directory,
            ..directory_metadata(1)
        }),
    );
    port.add_file(
        "two",
        Ok(FileMetadata {
            kind: EntryKind::Directory,
            ..directory_metadata(2)
        }),
    );
    port.add_directory("one", Vec::new());
    port.add_directory("two", Vec::new());
    let mut configured = limits();
    configured.max_pending_directories = 1;
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::ResourceLimit);
    assert!(!report.authoritative);
}

#[test]
fn denied_directory_is_a_coverage_failure_not_a_policy_exclusion() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("denied")]);
    port.add_file(
        "denied",
        Ok(FileMetadata {
            kind: EntryKind::Directory,
            ..directory_metadata(2)
        }),
    );
    port.deny_directory("denied");
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Denied);
    assert!(!report.authoritative);
    assert!(report.exclusions.is_empty());
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryDenied)
    );
}

#[test]
fn disappearing_file_is_partial_and_does_not_authorize_absence() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("vanished.flp")]);
    port.add_file("vanished.flp", Err(PortError::NotFound));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::EntryDisappeared)
    );
}

#[test]
fn cursor_failure_is_isolated_but_completion_is_not_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("one.flp"), Err(PortError::Other)]);
    port.add_file("one.flp", Ok(file_metadata(1, 1)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 1);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryRead)
    );
}

#[test]
fn cancellation_is_cooperative_and_drops_the_pending_batch() {
    let (mut port, root) = simple_port();
    let token = CancellationToken::new();
    port.cancel_on_open(token.clone());
    let mut sink = RecordingSink::default();
    let mut progress = RecordingProgress::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &token,
        &mut sink,
        &mut progress,
    );

    assert_eq!(report.outcome, Outcome::Cancelled);
    assert!(!report.authoritative);
    assert!(sink.batches.is_empty());
    assert!(sink.discarded);
    assert!(progress
        .updates
        .last()
        .is_some_and(|update| update.final_update && update.outcome == Some(Outcome::Cancelled)));
}

#[test]
fn reparse_points_are_excluded_without_being_traversed() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("alias")]);
    port.add_file("alias", Ok(reparse_file_metadata()));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    assert!(
        report
            .exclusions
            .iter()
            .any(|exclusion| exclusion.reason == ExclusionReason::ReparsePoint)
    );
    assert!(report.coverage_failures.is_empty());
}

#[test]
fn a_directory_that_turns_into_a_junction_at_metadata_time_stays_non_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("sub")]);
    port.add_file("sub", Ok(reparse_directory_metadata()));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert!(report.exclusions.is_empty());
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryChanged)
    );
}

#[test]
fn hardlink_like_aliases_keep_two_observations_with_one_identity() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("one.flp"), file_entry("two.FLP")]);
    port.add_file("one.flp", Ok(file_metadata(99, 12)));
    port.add_file("two.FLP", Ok(file_metadata(99, 12)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    let observations = sink
        .batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].identity, observations[1].identity);
    assert_ne!(observations[0].locator, observations[1].locator);
}

#[test]
fn case_only_duplicate_locators_invalidate_the_run() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("Same.flp"), file_entry("same.FLP")]);
    port.add_file("Same.flp", Ok(file_metadata(1, 1)));
    port.add_file("same.FLP", Ok(file_metadata(2, 1)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Invalid);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DuplicateLocator)
    );
}

#[test]
fn root_loss_after_traversal_is_not_complete() {
    let (mut port, root) = simple_port();
    port.add_root_result(Err(PortError::NotFound));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::RootUnavailable);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::RootNotFound)
    );
}

#[test]
fn unsupported_filesystem_is_explicitly_non_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.set_root_results(vec![Ok(RootMetadata {
        metadata: directory_metadata(900),
        qualification: FilesystemQualification::Unqualified,
    })]);
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::UnsupportedFilesystem);
    assert!(!report.authoritative);
}

#[test]
fn sink_failure_is_non_authoritative() {
    let (mut port, root) = simple_port();
    let mut sink = RecordingSink {
        batches: Vec::new(),
        error: Some(SinkError::Unavailable),
        discarded: false,
    };

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::SinkFailed);
    assert!(!report.authoritative);
    assert!(sink.discarded);
}

#[test]
fn run_scoped_storage_invalidates_provisional_batches_on_failure() {
    let (mut port, root) = simple_port();
    let mut configured = limits();
    configured.max_entries = 1;
    configured.max_batch_records = 1;
    let mut storage = RecordingRunStorage::default();

    let report = enumerate_into_run(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut storage,
        "run-resource-limit",
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::ResourceLimit);
    assert!(!report.authoritative);
    assert!(!storage.staged.contains_key("run-resource-limit"));
    assert_eq!(storage.invalidated, vec!["run-resource-limit"]);
}

#[test]
fn run_scoped_adapter_invalidates_in_flight_staging_when_dropped_armed() {
    let mut storage = RecordingRunStorage::default();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut sink = RunScopedSinkAdapter::new(&mut storage, "run-panic");
        let _ = sink.accept(ObservationBatch {
            sequence: 0,
            records: Vec::new(),
            estimated_bytes: 0,
        });
        panic!("simulated traversal panic after stage_batch");
    }));

    assert!(result.is_err());
    assert!(!storage.staged.contains_key("run-panic"));
    assert_eq!(storage.invalidated, vec!["run-panic"]);
}

#[test]
fn run_scoped_adapter_release_keeps_a_complete_run_staged_without_invalidation() {
    let mut storage = RecordingRunStorage::default();
    {
        let mut sink = RunScopedSinkAdapter::new(&mut storage, "run-complete");
        let _ = sink.accept(ObservationBatch {
            sequence: 0,
            records: Vec::new(),
            estimated_bytes: 0,
        });
        sink.release();
    }

    assert!(storage.invalidated.is_empty());
    assert!(storage.staged.contains_key("run-complete"));
}

#[test]
fn run_scoped_storage_retains_a_successful_empty_root_for_publication() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", Vec::new());
    let mut storage = RecordingRunStorage::default();

    let report = enumerate_into_run(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut storage,
        "run-empty",
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    assert!(storage.invalidated.is_empty());
    assert!(!storage.staged.contains_key("run-empty"));
}

#[test]
fn case_sensitive_directory_keeps_case_distinct_serialized_locators() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("case-tree")]);
    port.add_file("case-tree", Ok(directory_metadata(2)));
    port.add_directory("case-tree", vec![file_entry("A.flp"), file_entry("a.flp")]);
    port.add_file("case-tree/A.flp", Ok(file_metadata(3, 1)));
    port.add_file("case-tree/a.flp", Ok(file_metadata(4, 2)));
    port.set_case_sensitivity("case-tree", DirectoryCaseSensitivity::Sensitive);
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    let observations = sink
        .batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect::<Vec<_>>();
    let separator = if cfg!(windows) { "\\" } else { "/" };
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.locator.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec![
            format!("case-tree{separator}A.flp"),
            format!("case-tree{separator}a.flp")
        ]
    );
}

#[test]
fn windows_timestamp_and_identity_conversions_are_checked() {
    // Unix epoch: the offset constant cancels exactly.
    assert_eq!(
        windows_filetime_100ns_to_unix_ns_checked(WINDOWS_EPOCH_OFFSET_100NS as i64),
        Ok(0)
    );
    // Single-tick edges around the epoch.
    assert_eq!(
        windows_filetime_100ns_to_unix_ns_checked(WINDOWS_EPOCH_OFFSET_100NS as i64 - 1),
        Ok(-100)
    );
    assert_eq!(
        windows_filetime_100ns_to_unix_ns_checked(WINDOWS_EPOCH_OFFSET_100NS as i64 + 1),
        Ok(100)
    );
    // FILETIME extremes fall outside the durable i64-nanosecond range and
    // must reject, never saturate.
    assert_eq!(
        windows_filetime_100ns_to_unix_ns_checked(i64::MIN),
        Err(TimestampError::OutOfRange)
    );
    assert_eq!(
        windows_filetime_100ns_to_unix_ns_checked(i64::MAX),
        Err(TimestampError::OutOfRange)
    );

    assert_eq!(windows_file_id_to_u128([0; 16]), 0);
    assert_eq!(
        windows_file_id_to_u128([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        1
    );
    assert_eq!(
        windows_file_id_to_u128([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
        1u128 << 120
    );
    assert_eq!(windows_file_id_to_u128([u8::MAX; 16]), u128::MAX);
}

#[test]
fn root_replacement_between_inspection_and_open_is_refused_before_traversal() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("outside.flp")]);
    port.add_file("outside.flp", Ok(file_metadata(4, 1)));
    port.replace_root_before_open();
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::RootUnavailable);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert!(sink.batches.is_empty());
    assert!(port.opened().is_empty());
    assert_eq!(port.outside_accesses(), 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::RootChanged)
    );
}

#[test]
fn child_replacement_between_metadata_and_open_is_refused_before_cursor_escape() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("ancestor")]);
    port.add_file("ancestor", Ok(directory_metadata(2)));
    port.add_directory("ancestor", vec![file_entry("outside.flp")]);
    port.add_file("ancestor/outside.flp", Ok(file_metadata(4, 1)));
    port.replace_before_open("ancestor");
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert!(port.opened().is_empty());
    assert_eq!(port.outside_accesses(), 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryChanged)
    );
}

#[test]
fn ancestor_replacement_invalidates_bound_cursor_before_next_entry() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("ancestor")]);
    port.add_file("ancestor", Ok(directory_metadata(2)));
    port.add_directory("ancestor", vec![file_entry("outside.flp")]);
    port.add_file("ancestor/outside.flp", Ok(file_metadata(4, 1)));
    port.replace_before_next_entry("ancestor");
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert_eq!(port.opened(), vec!["ancestor"]);
    assert_eq!(port.outside_accesses(), 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryChanged)
    );
}

#[test]
fn path_normalization_rejects_absolute_parent_and_ads_components() {
    let absolute = if cfg!(windows) {
        Path::new(r"C:\absolute\file.flp")
    } else {
        Path::new("/absolute/file.flp")
    };
    assert_eq!(normalize_relative_path(absolute), Err(PathError::Absolute));
    assert_eq!(
        normalize_relative_path(Path::new("folder/../file.flp")),
        Err(PathError::ParentTraversal)
    );
    assert_eq!(
        normalize_relative_path(Path::new("folder:ads.flp")),
        Err(PathError::AlternateDataStream)
    );
}

fn file_metadata_ns(file_id: u128, byte_size: u64, modified_unix_ns: i128) -> FileMetadata {
    FileMetadata {
        kind: EntryKind::File,
        byte_size,
        modified_unix_ns,
        identity: Some(identity(file_id)),
        reparse_point: false,
        recall_or_offline: false,
    }
}

fn observations_of(sink: &RecordingSink) -> Vec<&Observation> {
    sink.batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect()
}

fn run_with_entries(
    entries: Vec<Result<DirectoryEntry, PortError>>,
    files: &[(&str, FileMetadata)],
) -> (EnumerationReport, RecordingSink) {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", entries);
    for (name, metadata) in files {
        port.add_file(name, Ok(metadata.clone()));
    }
    let mut sink = RecordingSink::default();
    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );
    (report, sink)
}

// This is the storage validator's syntactic shape from publication.rs:417-438
// on the Contract-Revision:2 branch. It is deliberately test-only so this
// isolated crate can prove the handoff without importing storage-sqlite.
fn agent1_valid_pct_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit);
            let low = bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit);
            if !high || !low {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

fn agent1_is_valid_locator_key_v1(key: &str) -> bool {
    if key.len() > MAX_OBSERVATION_KEY_BYTES || !key.is_ascii() || key.contains('\0') {
        return false;
    }
    let Some(body) = key.strip_prefix("v1:") else {
        return false;
    };
    if body.is_empty() {
        return false;
    }
    body.split('/').all(|segment| {
        let Some((mode, encoded)) = segment.split_once(':') else {
            return false;
        };
        (mode == "i" || mode == "s")
            && !encoded.is_empty()
            && encoded.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'%')
            })
            && agent1_valid_pct_encoding(encoded)
    })
}

fn scan_observation_for_key(locator_key: &str) -> ScanObservation {
    ScanObservation {
        locator_key: locator_key.to_owned(),
        relative_path: "a.flp".to_owned(),
        byte_size: 1,
        modified_at_ns: 1,
        identity: None,
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agent1ScanObservationFixture {
    locator_key: String,
    relative_path: String,
    byte_size: String,
    modified_at_ns: String,
    identity: Option<Agent1IdentityFixture>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agent1IdentityFixture {
    volume_serial: String,
    file_id: String,
}

#[test]
fn scan_observation_json_fixture_pins_agent1_serde_contract() {
    // Vendored because this crate intentionally remains outside the root
    // workspace. Contract-Revision:2 sections 2-3 require camelCase field
    // names and exact decimal strings for u64, i128, u64 identity, and u128
    // identity.
    let fixture_text = include_str!("../testdata/scan_observation_max.json");
    let fixture: Agent1ScanObservationFixture =
        serde_json::from_str(fixture_text).expect("pinned ScanObservation fixture parses");
    let value: serde_json::Value =
        serde_json::from_str(fixture_text).expect("pinned fixture is valid JSON");
    let object = value.as_object().expect("fixture object");
    assert_eq!(
        object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "locatorKey",
            "relativePath",
            "byteSize",
            "modifiedAtNs",
            "identity"
        ])
    );
    assert!(object["byteSize"].is_string());
    assert!(object["modifiedAtNs"].is_string());
    assert_eq!(fixture.locator_key, "v1:i:max.flp");
    assert_eq!(fixture.relative_path, "Max.FLp");
    assert_eq!(fixture.byte_size, u64::MAX.to_string());
    assert_eq!(fixture.modified_at_ns, i128::MIN.to_string());
    let identity = fixture.identity.expect("maximum identity present");
    assert_eq!(identity.volume_serial, u64::MAX.to_string());
    assert_eq!(identity.file_id, u128::MAX.to_string());

    // The wire DTO preserves the full source range; the storage staging guard
    // separately rejects u64 values above SQLite's signed i64 range.
    let observation = ScanObservation {
        locator_key: fixture.locator_key,
        relative_path: fixture.relative_path,
        byte_size: fixture.byte_size.parse().expect("u64 decimal"),
        modified_at_ns: fixture.modified_at_ns.parse().expect("i128 decimal"),
        identity: Some(EncodedIdentity {
            volume_serial: identity.volume_serial,
            file_id: identity.file_id,
        }),
    };
    assert_eq!(
        validate_scan_observation(&observation),
        Err(ScanConversionError::ByteSizeOutOfRange)
    );
}

#[test]
fn locator_key_v1_matches_agent1_shape_for_contract_vectors() {
    let insensitive = DirectoryCaseSensitivity::Insensitive;
    let sensitive = DirectoryCaseSensitivity::Sensitive;
    let mixed = LocatorKeyV1::root()
        .child("Shared", insensitive)
        .expect("valid component")
        .child("BuildOutput", sensitive)
        .expect("valid component")
        .child("artifact.flp", insensitive)
        .expect("valid component");
    let generated = [
        LocatorKeyV1::root()
            .child("Foo.flp", insensitive)
            .expect("valid component"),
        mixed,
        LocatorKeyV1::root()
            .child("café.flp", insensitive)
            .expect("valid component"),
    ];
    for key in generated.iter().map(LocatorKeyV1::as_str) {
        assert!(agent1_is_valid_locator_key_v1(key), "Agent 1 rejects {key}");
        assert_eq!(
            validate_scan_observation(&scan_observation_for_key(key)).is_ok(),
            agent1_is_valid_locator_key_v1(key),
            "validator disagreement for generated key {key}"
        );
    }

    // Copied from Agent 1's publication test at tests.rs:4144-4157. These
    // vectors must remain rejected by both boundaries.
    let invalid_keys = [
        "Projects/foo.flp",
        "ci:projects:foo.flp",
        "v1:",
        "v1:i:",
        "v1:x:foo.flp",
        "v1:i:foo/bar",
        "v1:i:a//i:b",
        "v1:i:a\\b",
        "v1:i:café.flp",
        "v1:i:bad%2.flp",
        "v1:i:bad%zz.flp",
        "v1:i:ok.flp%",
    ];
    for key in invalid_keys {
        assert!(
            !agent1_is_valid_locator_key_v1(key),
            "Agent 1 accepts {key}"
        );
        assert_eq!(
            validate_scan_observation(&scan_observation_for_key(key)).is_ok(),
            agent1_is_valid_locator_key_v1(key),
            "validator disagreement for invalid key {key}"
        );
    }
}

#[test]
fn locator_key_lowercase_hex_policy_is_explicit() {
    // Agent 1's syntactic guard intentionally accepts either hex case, while
    // rev2 §1.1 ABNF and this producer require canonical uppercase escapes.
    // The producer emits uppercase and rejects a non-canonical input rather
    // than silently changing the opaque bytes handed to storage.
    let lowercase = "v1:i:caf%c3%a9.flp";
    assert!(agent1_is_valid_locator_key_v1(lowercase));
    assert_eq!(
        validate_scan_observation(&scan_observation_for_key(lowercase)),
        Err(ScanConversionError::LocatorKey)
    );
}

#[test]
fn scan_observation_locator_key_boundaries_match_storage() {
    let prefix_len = "v1:i:".len();
    let maximum = format!(
        "v1:i:{}",
        "a".repeat(MAX_OBSERVATION_KEY_BYTES - prefix_len)
    );
    assert_eq!(maximum.len(), MAX_OBSERVATION_KEY_BYTES);
    assert!(agent1_is_valid_locator_key_v1(&maximum));
    assert!(validate_scan_observation(&scan_observation_for_key(&maximum)).is_ok());

    let over = format!("{}a", maximum);
    assert_eq!(over.len(), MAX_OBSERVATION_KEY_BYTES + 1);
    assert!(!agent1_is_valid_locator_key_v1(&over));
    assert_eq!(
        validate_scan_observation(&scan_observation_for_key(&over)),
        Err(ScanConversionError::LocatorKey)
    );

    for key in ["v1:i:a\0b", "Projects/foo.flp"] {
        assert!(!agent1_is_valid_locator_key_v1(key));
        assert_eq!(
            validate_scan_observation(&scan_observation_for_key(key)),
            Err(ScanConversionError::LocatorKey)
        );
    }
}

#[test]
fn traversal_rejects_overlength_locator_key_before_metadata() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    let name = format!(
        "{}.flp",
        "a".repeat(MAX_OBSERVATION_KEY_BYTES - "v1:i:".len() - ".flp".len() + 1)
    );
    assert_eq!(format!("v1:i:{name}").len(), MAX_OBSERVATION_KEY_BYTES + 1);
    port.add_directory(".", vec![file_entry(&name)]);
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| { failure.kind == CoverageFailureKind::LocatorKeyTooLong })
    );
}

#[test]
fn locator_key_v1_case_only_rename_keeps_key_and_updates_display_only() {
    let root = root_path();

    // First spelling observed.
    let mut first = FakePort::new(&root);
    first.add_directory(".", vec![file_entry("A.flp")]);
    first.add_file("A.flp", Ok(file_metadata(1, 10)));
    let mut first_sink = RecordingSink::default();
    let first_report = enumerate(
        &mut first,
        &root,
        &limits(),
        &NeverCancelled,
        &mut first_sink,
        &mut NoProgress,
    );
    assert_eq!(first_report.outcome, Outcome::Complete);

    // Case-only rename observed later.
    let mut second = FakePort::new(&root);
    second.add_directory(".", vec![file_entry("a.flp")]);
    second.add_file("a.flp", Ok(file_metadata(1, 10)));
    let mut second_sink = RecordingSink::default();
    let second_report = enumerate(
        &mut second,
        &root,
        &limits(),
        &NeverCancelled,
        &mut second_sink,
        &mut NoProgress,
    );
    assert_eq!(second_report.outcome, Outcome::Complete);

    let first_observation = &observations_of(&first_sink)[0];
    let second_observation = &observations_of(&second_sink)[0];
    // Same durable key: publication updates the display spelling and retains
    // the location and physical association.
    assert_eq!(
        first_observation.locator_key_v1,
        second_observation.locator_key_v1
    );
    assert_eq!(first_observation.locator_key_v1.as_str(), "v1:i:a.flp");
    assert_ne!(
        first_observation.display_path.as_str(),
        second_observation.display_path.as_str()
    );

    // Unit-level: the insensitive fold is total for ASCII case pairs.
    assert_eq!(
        LocatorKeyV1::root()
            .child("A.flp", DirectoryCaseSensitivity::Insensitive)
            .expect("valid component"),
        LocatorKeyV1::root()
            .child("a.flp", DirectoryCaseSensitivity::Insensitive)
            .expect("valid component"),
    );
}

#[test]
fn locator_key_v1_sensitive_directory_keeps_case_distinct_aliases_in_binary_order() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.set_case_sensitivity(".", DirectoryCaseSensitivity::Sensitive);
    port.add_directory(".", vec![file_entry("foo.flp"), file_entry("Foo.flp")]);
    port.add_file("foo.flp", Ok(file_metadata(1, 1)));
    port.add_file("Foo.flp", Ok(file_metadata(2, 2)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    let mut keys = observations_of(&sink)
        .iter()
        .map(|observation| observation.locator_key_v1.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(keys.len(), 2);
    assert_ne!(keys[0], keys[1]);
    // Deterministic storage order is the byte-wise order of the key strings.
    keys.sort();
    assert_eq!(keys, vec!["v1:s:Foo.flp", "v1:s:foo.flp"]);
    let upper = LocatorKeyV1::decode("v1:s:Foo.flp").expect("valid key");
    let lower = LocatorKeyV1::decode("v1:s:foo.flp").expect("valid key");
    assert!(upper < lower);
}

#[test]
fn locator_key_v1_mixed_directory_modes_share_one_tree() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    // Root stays insensitive; `sub` is case-sensitive.
    port.add_directory(".", vec![file_entry("Root.flp"), file_entry("sub")]);
    port.add_file("Root.flp", Ok(file_metadata(1, 1)));
    port.add_file("sub", Ok(directory_metadata(2)));
    port.add_directory("sub", vec![file_entry("Leaf.flp"), file_entry("leaf.flp")]);
    port.add_file("sub/Leaf.flp", Ok(file_metadata(3, 1)));
    port.add_file("sub/leaf.flp", Ok(file_metadata(4, 1)));
    port.set_case_sensitivity("sub", DirectoryCaseSensitivity::Sensitive);
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    let mut keys = observations_of(&sink)
        .iter()
        .map(|observation| observation.locator_key_v1.as_str().to_owned())
        .collect::<Vec<_>>();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "v1:i:root.flp",
            "v1:i:sub/s:Leaf.flp",
            "v1:i:sub/s:leaf.flp",
        ]
    );
    // Every segment records its own directory mode.
    let nested = LocatorKeyV1::decode("v1:i:sub/s:Leaf.flp").expect("valid key");
    assert_eq!(
        nested
            .components()
            .expect("valid components")
            .iter()
            .map(|component| component.sensitivity)
            .collect::<Vec<_>>(),
        vec![
            DirectoryCaseSensitivity::Insensitive,
            DirectoryCaseSensitivity::Sensitive,
        ]
    );
    assert_eq!(
        nested.components().expect("valid components")[1].spelling,
        "Leaf.flp"
    );
}

#[test]
fn locator_key_v1_unicode_long_paths_and_serialization_are_stable() {
    let insensitive = DirectoryCaseSensitivity::Insensitive;
    let sensitive = DirectoryCaseSensitivity::Sensitive;

    // NFC and NFD spellings normalize to the same durable key; display paths
    // remain responsible for retaining the spelling returned by the filesystem.
    let nfc = LocatorKeyV1::root()
        .child("\u{e9}.flp", sensitive)
        .expect("valid");
    let nfd = LocatorKeyV1::root()
        .child("e\u{301}.flp", sensitive)
        .expect("valid");
    assert_eq!(nfc, nfd);
    assert_eq!(nfc.as_str(), "v1:s:%C3%A9.flp");
    assert!(nfc.as_str().is_ascii() && nfd.as_str().is_ascii());

    // Non-ASCII case folds in insensitive directories (Ä -> ä).
    assert_eq!(
        LocatorKeyV1::root()
            .child("\u{c4}.flp", insensitive)
            .expect("valid"),
        LocatorKeyV1::root()
            .child("\u{e4}.flp", insensitive)
            .expect("valid"),
    );
    // The generated simple-fold table is not Unicode lowercase expansion:
    // common/special mappings fold, while Turkic dotted I stays distinct.
    assert_eq!(
        LocatorKeyV1::root()
            .child("\u{b5}.flp", insensitive)
            .expect("valid"),
        LocatorKeyV1::root()
            .child("\u{3bc}.flp", insensitive)
            .expect("valid"),
    );
    assert_eq!(
        LocatorKeyV1::root()
            .child("\u{3c2}.flp", insensitive)
            .expect("valid"),
        LocatorKeyV1::root()
            .child("\u{3c3}.flp", insensitive)
            .expect("valid"),
    );
    assert_ne!(
        LocatorKeyV1::root()
            .child("\u{130}.flp", insensitive)
            .expect("valid"),
        LocatorKeyV1::root()
            .child("i\u{307}.flp", insensitive)
            .expect("valid"),
    );

    // Mixed-case extensions fold with the rest of the component.
    assert_eq!(
        LocatorKeyV1::root()
            .child("Song.FLP", insensitive)
            .expect("valid")
            .as_str(),
        "v1:i:song.flp",
    );

    // Long paths are stable: encode -> decode -> encode is byte-identical.
    let long_name = format!("{}.flp", "a".repeat(200));
    let long_key = LocatorKeyV1::root()
        .child(&long_name, insensitive)
        .expect("valid");
    assert!(long_key.as_str().is_ascii());
    let decoded = LocatorKeyV1::decode(long_key.as_str()).expect("valid key");
    assert_eq!(
        LocatorKeyV1::decode(decoded.as_str())
            .expect("valid key")
            .as_str(),
        long_key.as_str()
    );

    // Byte-stable across independent constructions (two runs agree exactly).
    let again = LocatorKeyV1::root()
        .child(&long_name, insensitive)
        .expect("valid");
    assert_eq!(again.as_str(), long_key.as_str());

    // Non-canonical and malformed strings reject.
    assert_eq!(
        LocatorKeyV1::decode("").unwrap_err(),
        LocatorKeyError::Empty
    );
    assert_eq!(
        LocatorKeyV1::decode("a.flp").unwrap_err(),
        LocatorKeyError::MissingVersion
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:x:a").unwrap_err(),
        LocatorKeyError::BadMode
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:i:%zz").unwrap_err(),
        LocatorKeyError::BadEscape
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:i:%61").unwrap_err(),
        LocatorKeyError::NonCanonical
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:i:a%7eb").unwrap_err(),
        LocatorKeyError::NonCanonical
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:i:a%2fb").unwrap_err(),
        LocatorKeyError::InvalidComponent
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:i:é").unwrap_err(),
        LocatorKeyError::NonAscii
    );
    assert_eq!(
        LocatorKeyV1::decode("v1").unwrap_err(),
        LocatorKeyError::MissingVersion
    );
    assert_eq!(
        LocatorKeyV1::decode("v1:s:%FF").unwrap_err(),
        LocatorKeyError::InvalidUnicode
    );
}

#[test]
fn locator_key_v1_duplicates_reject_the_run_regardless_of_order() {
    for entries in [
        vec![file_entry("Same.flp"), file_entry("same.FLP")],
        vec![file_entry("same.FLP"), file_entry("Same.flp")],
    ] {
        let root = root_path();
        let mut port = FakePort::new(&root);
        port.add_directory(".", entries);
        port.add_file("Same.flp", Ok(file_metadata(1, 1)));
        port.add_file("same.FLP", Ok(file_metadata(2, 1)));
        let mut sink = RecordingSink::default();

        let report = enumerate(
            &mut port,
            &root,
            &limits(),
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );

        assert_eq!(report.outcome, Outcome::Invalid);
        assert!(!report.authoritative);
        assert!(
            report
                .coverage_failures
                .iter()
                .any(|failure| failure.kind == CoverageFailureKind::DuplicateLocator)
        );
        // No winner: provisional batches are discarded, never staged.
        assert!(sink.batches.is_empty());
        assert!(sink.discarded);
    }
}

#[test]
fn checked_timestamp_edges_reject_out_of_range_without_saturating() {
    assert_eq!(unix_ns_to_i64_checked(i64::MIN as i128), Ok(i64::MIN));
    assert_eq!(unix_ns_to_i64_checked(i64::MAX as i128), Ok(i64::MAX));
    assert_eq!(
        unix_ns_to_i64_checked(i64::MIN as i128 - 1),
        Err(TimestampError::OutOfRange)
    );
    assert_eq!(
        unix_ns_to_i64_checked(i64::MAX as i128 + 1),
        Err(TimestampError::OutOfRange)
    );
    assert_eq!(system_time_to_unix_ns_checked(std::time::UNIX_EPOCH), Ok(0));
    assert_eq!(
        system_time_to_unix_ns_checked(std::time::UNIX_EPOCH - std::time::Duration::from_secs(1)),
        Err(TimestampError::BeforeEpoch)
    );

    // A fake-port timestamp outside the durable range fails the file with an
    // explicit coverage failure instead of saturating the run.
    let (report, _) = run_with_entries(
        vec![file_entry("future.flp")],
        &[("future.flp", file_metadata_ns(1, 8, i64::MAX as i128 + 1))],
    );
    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::TimestampOutOfRange)
    );

    // A byte size beyond the durable 0..=i64::MAX range fails the same way.
    let (report, _) = run_with_entries(
        vec![file_entry("huge.flp")],
        &[("huge.flp", file_metadata_ns(1, i64::MAX as u64 + 1, 100))],
    );
    assert_eq!(report.outcome, Outcome::Partial);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::ByteSizeOutOfRange)
    );

    // Boundary values still pass.
    let (report, sink) = run_with_entries(
        vec![file_entry("edge.flp")],
        &[(
            "edge.flp",
            file_metadata_ns(1, i64::MAX as u64, i64::MIN as i128),
        )],
    );
    assert_eq!(report.outcome, Outcome::Complete);
    assert_eq!(observations_of(&sink).len(), 1);
}

#[test]
fn file_id_byte_order_is_little_endian_and_round_trips() {
    // Byte edges.
    assert_eq!(windows_file_id_to_u128([0; 16]), 0);
    assert_eq!(windows_file_id_to_u128([u8::MAX; 16]), u128::MAX);

    // Fixed byte-order vector: bytes 0x01..=0x10 read little-endian.
    let vector: [u8; 16] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10,
    ];
    assert_eq!(
        windows_file_id_to_u128(vector),
        0x100F_0E0D_0C0B_0A09_0807_0605_0403_0201u128
    );

    // The inverse helper is exact on edges and the vector.
    for value in [
        0u128,
        1,
        1u128 << 120,
        u128::MAX,
        windows_file_id_to_u128(vector),
    ] {
        assert_eq!(
            windows_file_id_to_u128(u128_to_windows_file_id_bytes(value)),
            value
        );
    }

    // Identity DTO decimals are exact at the numeric edges.
    let encoded = encode_identity(&QualifiedIdentity {
        volume_serial: u64::MAX,
        file_id: u128::MAX,
        qualification: IdentityQualification::LocalNtfs,
    })
    .expect("local NTFS identity encodes");
    assert_eq!(encoded.volume_serial, "18446744073709551615");
    assert_eq!(encoded.file_id, "340282366920938463463374607431768211455");
    let zero = encode_identity(&identity(0)).expect("zero identity encodes");
    assert_eq!(zero.volume_serial, "7");
    assert_eq!(zero.file_id, "0");
}

#[test]
fn scan_observation_conversion_enforces_storage_staging_guards() {
    // A real enumeration observation converts exactly.
    let (_, sink) = run_with_entries(
        vec![file_entry("Mix.FLP")],
        &[("Mix.FLP", file_metadata(9, 42))],
    );
    let observation = observations_of(&sink)[0];
    let scan = observation
        .to_scan_observation()
        .expect("valid observation converts");
    // The key is the durable LocatorKeyV1 bytes, never the display spelling.
    assert_eq!(scan.locator_key, observation.locator_key_v1.as_str());
    assert!(scan.locator_key.starts_with("v1:"));
    assert_ne!(scan.locator_key, observation.locator.as_str());
    assert_eq!(scan.relative_path, observation.display_path.as_str());
    assert_eq!(scan.byte_size, 42);
    assert_eq!(scan.modified_at_ns, 100);
    let identity = scan.identity.as_ref().expect("identity present");
    assert_eq!(identity.volume_serial, "7");
    assert_eq!(identity.file_id, "9");
    validate_scan_observation(&scan).expect("converted observation validates");

    // Decimal strings are exact at the numeric edges (JSON-safe, no numbers).
    assert_eq!(u64::MAX.to_string(), "18446744073709551615");
    assert_eq!(
        u128::MAX.to_string(),
        "340282366920938463463374607431768211455"
    );
    let edge = ScanObservation {
        locator_key: "v1:i:edge.flp".to_owned(),
        relative_path: "edge.flp".to_owned(),
        byte_size: i64::MAX as u64,
        modified_at_ns: i64::MIN as i128,
        identity: Some(EncodedIdentity {
            volume_serial: u64::MAX.to_string(),
            file_id: u128::MAX.to_string(),
        }),
    };
    validate_scan_observation(&edge).expect("edge values validate");
    let top = ScanObservation {
        modified_at_ns: i64::MAX as i128,
        ..edge.clone()
    };
    validate_scan_observation(&top).expect("maximum mtime validates");

    // Out-of-range numerics reject.
    assert_eq!(
        validate_scan_observation(&ScanObservation {
            byte_size: i64::MAX as u64 + 1,
            ..edge.clone()
        }),
        Err(ScanConversionError::ByteSizeOutOfRange)
    );
    assert_eq!(
        validate_scan_observation(&ScanObservation {
            modified_at_ns: i64::MAX as i128 + 1,
            ..edge.clone()
        }),
        Err(ScanConversionError::TimestampOutOfRange)
    );
    assert_eq!(
        validate_scan_observation(&ScanObservation {
            modified_at_ns: i64::MIN as i128 - 1,
            ..edge.clone()
        }),
        Err(ScanConversionError::TimestampOutOfRange)
    );

    // Non-ASCII keys reject per the storage is_ascii guard.
    assert_eq!(
        validate_scan_observation(&ScanObservation {
            locator_key: "v1:i:é.flp".to_owned(),
            ..edge.clone()
        }),
        Err(ScanConversionError::LocatorKey)
    );

    // Identity encodings: negative, signed, nondecimal, leading-zero,
    // overflow, and partial pairs all reject.
    for (volume_serial, file_id) in [
        (Some("-1"), Some("1")),
        (Some("+1"), Some("1")),
        (Some("0x1"), Some("1")),
        (Some("01"), Some("1")),
        (Some("1"), Some("007")),
        (Some("18446744073709551616"), Some("1")),
        (Some("1"), Some("340282366920938463463374607431768211456")),
        (Some(""), Some("1")),
        (Some("1"), Some("")),
        (Some("1"), None),
        (None, Some("1")),
    ] {
        assert!(
            validate_identity_pair(volume_serial, file_id).is_err(),
            "invalid identity must reject: {volume_serial:?}/{file_id:?}"
        );
    }
    assert_eq!(
        validate_identity_pair(None, None).expect("empty pair"),
        None
    );
    assert_eq!(
        validate_identity_pair(Some("0"), Some("0")).expect("zero pair"),
        Some(EncodedIdentity {
            volume_serial: "0".to_owned(),
            file_id: "0".to_owned(),
        })
    );
}

#[cfg(windows)]
mod windows_fixtures {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        marker_path: PathBuf,
        marker: Vec<u8>,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let tick = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before epoch")
                .as_nanos();
            let ordinal = NEXT.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "fruitboard-enumeration-{label}-{}-{tick}-{ordinal}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("create disposable fixture root");
            let marker_path = root.join("source-marker.FLP");
            let marker = vec![0, 1, 2, 0xF1, 0x00, 0xFF, 0x7E, 0x42];
            fs::write(&marker_path, &marker).expect("write disposable marker");
            Self {
                root,
                marker_path,
                marker,
            }
        }

        fn long_io_path(path: &Path) -> PathBuf {
            let text = path.to_string_lossy();
            if text.starts_with(r"\\?\") {
                return path.to_owned();
            }
            if let Some(unc) = text.strip_prefix(r"\\") {
                return PathBuf::from(format!(r"\\?\UNC\{unc}"));
            }
            PathBuf::from(format!(r"\\?\{text}"))
        }

        fn create_long_unicode_file(&self) -> PathBuf {
            let mut directory = self.root.join("ユニコード");
            for index in 0..18 {
                directory.push(format!("segment-{index:02}-long-name"));
            }
            let file = directory.join("long-project-name-with-spaces.FlP");
            fs::create_dir_all(Self::long_io_path(&directory)).expect("create long fixture path");
            fs::write(Self::long_io_path(&file), &self.marker).expect("write long FLP marker");
            file
        }

        fn create_junction(&self) -> PathBuf {
            let target = self.root.join("real-target");
            let junction = self.root.join("reparse-alias");
            fs::create_dir_all(&target).expect("create junction target");
            fs::write(target.join("outside-looking.FLP"), &self.marker)
                .expect("write junction target marker");
            let result = Command::new("cmd.exe")
                .args([
                    "/C",
                    "mklink",
                    "/J",
                    &junction.to_string_lossy(),
                    &target.to_string_lossy(),
                ])
                .output()
                .expect("launch mklink for disposable junction");
            assert!(
                result.status.success(),
                "junction fixture unavailable: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            junction
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn ntfs_fixture_discovers_unicode_long_mixed_case_and_preserves_markers() {
        let fixture = Fixture::new("metadata");
        let long_file = fixture.create_long_unicode_file();
        fs::write(fixture.root.join("mixed-case.fLp"), &fixture.marker)
            .expect("write mixed-case marker");
        fs::write(fixture.root.join("not-an-flp.txt"), &fixture.marker)
            .expect("write filter marker");
        let before = fs::read(&fixture.marker_path).expect("read marker before scan");
        let mut port = WindowsFilesystemPort::new();
        let mut sink = RecordingSink::default();
        let report = enumerate(
            &mut port,
            &fixture.root,
            &EnumerationLimits {
                progress_interval: std::time::Duration::ZERO,
                ..EnumerationLimits::default()
            },
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );
        let after = fs::read(&fixture.marker_path).expect("read marker after scan");

        assert_eq!(report.outcome, Outcome::Complete);
        assert!(report.authoritative);
        assert_eq!(before, after);
        let observations = sink
            .batches
            .iter()
            .flat_map(|batch| batch.records.iter())
            .collect::<Vec<_>>();
        assert!(
            observations
                .iter()
                .any(|observation| observation.display_path.as_str() == "mixed-case.fLp")
        );
        assert!(observations.iter().any(|observation| {
            observation
                .display_path
                .as_str()
                .ends_with("long-project-name-with-spaces.FlP")
        }));
        assert!(!observations.iter().any(|observation| {
            observation
                .display_path
                .as_str()
                .ends_with("not-an-flp.txt")
        }));
        assert!(observations.iter().any(|observation| {
            observation.display_path.as_str()
                == long_file
                    .strip_prefix(&fixture.root)
                    .expect("long file under fixture")
                    .to_string_lossy()
                    .replace('/', "\\")
        }));
    }

    #[test]
    fn ntfs_fixture_keeps_hardlink_aliases_per_path() {
        let fixture = Fixture::new("aliases");
        let alias_source = fixture.root.join("alias-source.FLP");
        let alias = fixture.root.join("alias-second.flp");
        fs::hard_link(&fixture.marker_path, &alias_source).expect("create hardlink source");
        fs::hard_link(&alias_source, &alias).expect("create hardlink alias");
        let mut port = WindowsFilesystemPort::new();
        let mut sink = RecordingSink::default();
        let report = enumerate(
            &mut port,
            &fixture.root,
            &EnumerationLimits {
                progress_interval: std::time::Duration::ZERO,
                ..EnumerationLimits::default()
            },
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );

        assert_eq!(report.outcome, Outcome::Complete);
        assert!(report.authoritative);
        let observations = sink
            .batches
            .iter()
            .flat_map(|batch| batch.records.iter())
            .collect::<Vec<_>>();
        let aliases = observations
            .iter()
            .filter(|observation| {
                observation.display_path.as_str() == "alias-source.FLP"
                    || observation.display_path.as_str() == "alias-second.flp"
            })
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].identity, aliases[1].identity);
    }

    #[test]
    fn ntfs_fixture_metadata_marks_a_junction_as_reparse() {
        let fixture = Fixture::new("reparse-metadata");
        let junction = fixture.create_junction();
        let expected_name = junction
            .file_name()
            .expect("junction has a file name")
            .to_owned();
        let mut port = WindowsFilesystemPort::new();
        let _ = port
            .inspect_root(&fixture.root)
            .expect("fixture root is inspectable");
        let opened = port.open_root(&fixture.root).expect("fixture root opens");
        let mut cursor = opened.cursor;
        let mut found = None;
        while let Some(entry) = cursor.next_entry().expect("directory query succeeds") {
            if entry.name == expected_name {
                found = Some(
                    cursor
                        .read_metadata(&entry)
                        .expect("junction metadata succeeds"),
                );
                break;
            }
        }
        let metadata = found.expect("junction appears in root enumeration");
        assert_eq!(metadata.kind, EntryKind::Directory);
        assert!(metadata.reparse_point);
    }

    #[test]
    fn ntfs_fixture_junction_subtree_is_non_authoritative() {
        let fixture = Fixture::new("junction");
        let junction = fixture.create_junction();
        let mut port = WindowsFilesystemPort::new();
        let mut sink = RecordingSink::default();
        let report = enumerate(
            &mut port,
            &fixture.root,
            &EnumerationLimits {
                progress_interval: std::time::Duration::ZERO,
                ..EnumerationLimits::default()
            },
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );

        assert_eq!(report.outcome, Outcome::Partial);
        assert!(!report.authoritative);
        let junction_relative = junction
            .strip_prefix(&fixture.root)
            .expect("junction under fixture")
            .to_string_lossy()
            .replace('/', "\\");
        // The junction itself is a metadata-time directory swap, not a policy
        // exclusion: the skipped subtree must never be covered authoritatively.
        assert!(report.coverage_failures.iter().any(|failure| {
            failure.display_path.as_ref().map(|path| path.as_str())
                == Some(junction_relative.as_str())
                && failure.kind == CoverageFailureKind::DirectoryChanged
        }));
        assert!(!report.exclusions.iter().any(|exclusion| {
            exclusion.display_path.as_str() == junction_relative
                && exclusion.reason == ExclusionReason::ReparsePoint
        }));
    }

    #[test]
    fn ntfs_fixture_locator_keys_are_ascii_decode_and_convert() {
        let fixture = Fixture::new("keys");
        fixture.create_long_unicode_file();
        fs::write(fixture.root.join("Mixed-Case.FlP"), &fixture.marker)
            .expect("write mixed-case marker");
        let mut port = WindowsFilesystemPort::new();
        let mut sink = RecordingSink::default();
        let report = enumerate(
            &mut port,
            &fixture.root,
            &EnumerationLimits {
                progress_interval: std::time::Duration::ZERO,
                ..EnumerationLimits::default()
            },
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );

        assert_eq!(report.outcome, Outcome::Complete);
        let observations = sink
            .batches
            .iter()
            .flat_map(|batch| batch.records.iter())
            .collect::<Vec<_>>();
        assert!(!observations.is_empty());
        let mut keys = Vec::new();
        for observation in &observations {
            let key = observation.locator_key_v1.as_str();
            assert!(key.is_ascii(), "key must be ASCII: {key}");
            let decoded = LocatorKeyV1::decode(key).expect("key decodes");
            assert_eq!(decoded.as_str(), key, "decode round-trips");
            observation
                .to_scan_observation()
                .expect("observation converts to the storage DTO");
            keys.push(key.to_owned());
        }
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), observations.len(), "no duplicate keys");
    }

    #[test]
    #[ignore = "DriveFS/streamed placeholders are unavailable in this environment; #47 remains open"]
    fn drivefs_placeholder_fixture_requires_a_qualified_host() {}

    #[test]
    #[ignore = "ACL-denied fixture requires a disposable Windows security-policy setup"]
    fn acl_denied_fixture_requires_an_explicit_windows_acl_environment() {}
}
