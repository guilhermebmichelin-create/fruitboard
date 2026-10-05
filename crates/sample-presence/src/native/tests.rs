use super::*;
use crate::{MonotonicClock, Outcome, WorkerGate, check};
use std::fs::{self, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct Clock;
impl MonotonicClock for Clock {
    fn now_ms(&self) -> u64 {
        0
    }
}
#[derive(Default)]
struct Fence {
    revoked: Arc<AtomicBool>,
}
impl CurrentAuthorization for Fence {
    fn current(
        &mut self,
        _context: &Context,
        _fence: &AuthorityFence,
    ) -> Result<(), RequestFailure> {
        if self.revoked.load(Ordering::SeqCst) {
            Err(RequestFailure::Stale)
        } else {
            Ok(())
        }
    }
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    source: PathBuf,
    links: Arc<Mutex<Vec<PathBuf>>>,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "fruitboard-sample-metadata-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        assert!(!base.exists());
        fs::create_dir(&base).unwrap();
        let root = base.join("projects");
        fs::create_dir(&root).unwrap();
        let source = root.join("project.flp");
        fs::write(
            &source,
            b"private constructed source; not an FL Studio corpus fixture",
        )
        .unwrap();
        Self {
            base,
            root,
            source,
            links: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn sample(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, b"metadata-only target sentinel").unwrap();
        path
    }
    fn input(&self, references: &[Option<&Path>]) -> CapturedInput {
        let gate = WorkerGate::default();
        let lease = gate.try_start(Arc::new(Clock)).unwrap();
        let operations = Operations::new(lease.control());
        let open = |path: &Path, directory: bool, operations: &Operations| {
            let file = OpenOptions::new()
                .access_mode(0x80 | 0x100000)
                .share_mode(7)
                .custom_flags(0x200000 | if directory { 0x2000000 } else { 0 })
                .open(path)
                .unwrap();
            ffi::independent_test_handle(file, operations.acquire_handle().unwrap())
        };
        let root = open(&self.root, true, &operations);
        let source = open(&self.source, false, &operations);
        let root_id = ffi::identity(&root).unwrap();
        let source_id = ffi::identity(&source).unwrap();
        // Independent std observations, rather than the adapter's conversion,
        // register the expected size/high-resolution modification timestamp.
        let recorded = fs::metadata(&self.source).unwrap();
        let modified_ns = i64::try_from(
            recorded
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();
        CapturedInput::new(
            Context::new("req", "root", "location", "snapshot", "session").unwrap(),
            self.root.to_str().unwrap(),
            self.source.to_str().unwrap(),
            AuthorityFence {
                root_revision: 1,
                location_revision: 1,
                root_identity: root_id,
                source: SourceFingerprint {
                    identity: source_id,
                    byte_size: recorded.len(),
                    modified_at_ns: modified_ns,
                },
            },
            Some(
                references
                    .iter()
                    .map(|v| v.map(|p| p.to_str().unwrap().to_owned()))
                    .collect(),
            ),
        )
        .unwrap()
    }
    fn junction(&self, link: &Path, target: &Path) {
        assert!(link.starts_with(&self.base) && target.starts_with(&self.base));
        self.links.lock().unwrap().push(link.to_owned());
        ffi::junction(link, target);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Remove registered junctions as links, never traversing them. Inventory
        // only this uniquely owned tree and refuse unexpected reparses first.
        for link in self.links.lock().unwrap().iter() {
            if link.exists() {
                fs::remove_dir(link).unwrap();
            }
        }
        fn inventory(root: &Path, entries: &mut Vec<(PathBuf, bool)>) {
            let metadata = fs::symlink_metadata(root).unwrap();
            assert_eq!(
                metadata.file_attributes() & 0x400,
                0,
                "unexpected fixture reparse; cleanup refused"
            );
            if metadata.is_dir() {
                for entry in fs::read_dir(root).unwrap() {
                    inventory(&entry.unwrap().path(), entries);
                }
            }
            entries.push((root.to_owned(), metadata.is_dir()));
        }
        assert!(
            self.base.is_absolute()
                && self
                    .base
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("fruitboard-sample-metadata-")
        );
        let mut entries = Vec::new();
        inventory(&self.base, &mut entries);
        for (path, directory) in entries {
            assert!(path.starts_with(&self.base));
            if directory {
                fs::remove_dir(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
    }
}

fn run(
    input: &CapturedInput,
    port: &mut WindowsPort<Fence>,
) -> Result<crate::Report, RequestFailure> {
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock)).unwrap();
    check(input, &mut lease, port, 1_791_216_000_000)
}
fn outcomes(report: &crate::Report) -> Vec<Outcome> {
    report.channels().iter().map(|v| v.outcome).collect()
}
fn unchecked(reason: UncheckedReason) -> Outcome {
    Outcome::NotChecked { reason }
}

#[test]
fn ordinary_present_absent_intermediate_directory_and_duplicate_slots_preserve_bytes() {
    let fixture = Fixture::new();
    let sample = fixture.sample("Kick.wav");
    let missing = fixture.root.join("missing.wav");
    let absent_parent = fixture.root.join("absent").join("x.wav");
    let source_before = fs::read(&fixture.source).unwrap();
    let sample_before = fs::read(&sample).unwrap();
    let input = fixture.input(&[
        Some(&sample),
        Some(&missing),
        Some(&absent_parent),
        None,
        Some(&sample),
        Some(&fixture.root),
    ]);
    let report = run(&input, &mut WindowsPort::new(Fence::default())).unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            Outcome::Present,
            Outcome::NotFound,
            Outcome::NotFound,
            Outcome::NoSavedReference,
            Outcome::Present,
            unchecked(UncheckedReason::NotRegularFile)
        ]
    );
    assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
    assert_eq!(fs::read(&sample).unwrap(), sample_before);
}

#[test]
fn source_handles_cannot_read_contents_and_allow_ordinary_writers() {
    let fixture = Fixture::new();
    let input = fixture.input(&[]);
    let gate = WorkerGate::default();
    let lease = gate.try_start(Arc::new(Clock)).unwrap();
    let mut operations = Operations::new(lease.control());
    let mut port = WindowsPort::new(Fence::default());
    let qualified = port.qualify(&input, &mut operations).unwrap();
    assert!(ffi::content_read_is_denied(&qualified.authority.source));
    fixture.sample("rights.wav");
    let (sample, _) = port
        .open_child(
            &qualified.root_directory.node,
            "rights.wav",
            Some(false),
            &mut operations,
        )
        .unwrap()
        .unwrap();
    assert!(ffi::content_read_is_denied(&sample));
    let writer = OpenOptions::new()
        .write(true)
        .open(&fixture.source)
        .unwrap();
    drop(writer);
    assert!(
        port.revalidate(&input, &qualified.authority, &mut operations)
            .is_ok()
    );
}

#[test]
fn root_replaced_by_a_junction_during_qualification_cannot_grant_target_access() {
    let fixture = Fixture::new();
    let sample = fixture.root.join("x.wav");
    let input = fixture.input(&[Some(&sample)]);
    let root = fixture.root.clone();
    let old = fixture.base.join("old-projects");
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    let links = fixture.links.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && name == "projects" {
            fs::rename(&root, &old).unwrap();
            links.lock().unwrap().push(root.clone());
            ffi::junction(&root, &outside);
        }
    }));
    assert!(matches!(
        run(&input, &mut port),
        Err(RequestFailure::Unavailable(
            UncheckedReason::ReparseOrOffline
        ))
    ));
}

#[test]
fn native_cancellation_between_query_and_open_prevents_the_next_syscall() {
    let fixture = Fixture::new();
    let sample = fixture.sample("cancel.wav");
    let input = fixture.input(&[Some(&sample)]);
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock)).unwrap();
    let control = lease.control();
    let opens = Arc::new(Mutex::new(0));
    let counter = opens.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if name == "cancel.wav" {
            if stage == TestStage::BeforeOpen {
                control.cancel();
            }
            if stage == TestStage::AfterOpen {
                *counter.lock().unwrap() += 1;
            }
        }
    }));
    assert!(matches!(
        check(&input, &mut lease, &mut port, 0),
        Err(RequestFailure::Cancelled)
    ));
    assert_eq!(*opens.lock().unwrap(), 0);
    assert!(matches!(
        gate.try_start(Arc::new(Clock)),
        Err(crate::AdmissionError::Busy)
    ));
}

#[test]
fn maximum_source_depth_and_separate_candidate_chain_fit_reserved_fence_and_handles() {
    let mut fixture = Fixture::new();
    let root_depth = AbsolutePath::parse(fixture.root.to_str().unwrap())
        .unwrap()
        .components()
        .len();
    assert!(root_depth < 62);
    let mut source_parent = fixture.root.clone();
    let mut sample_parent = fixture.root.clone();
    for _ in root_depth..63 {
        source_parent = source_parent.join("s");
        sample_parent = sample_parent.join("t");
        fs::create_dir(&source_parent).unwrap();
        fs::create_dir(&sample_parent).unwrap();
    }
    let source = source_parent.join("project.flp");
    fs::rename(&fixture.source, &source).unwrap();
    fixture.source = source;
    let sample = sample_parent.join("x.wav");
    fs::write(&sample, b"deep target").unwrap();
    let input = fixture.input(&[Some(&sample)]);
    assert_eq!(input.source().components().len(), 64);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![Outcome::Present]
    );
}

#[test]
fn held_ancestor_names_cannot_be_replaced_while_descendant_writes_remain_allowed() {
    let fixture = Fixture::new();
    let input = fixture.input(&[]);
    let gate = WorkerGate::default();
    let lease = gate.try_start(Arc::new(Clock)).unwrap();
    let mut operations = Operations::new(lease.control());
    let qualified = WindowsPort::new(Fence::default())
        .qualify(&input, &mut operations)
        .unwrap();
    assert!(fs::rename(&fixture.root, fixture.base.join("replacement")).is_err());
    fs::write(fixture.root.join("ordinary-new-file"), b"ordinary save").unwrap();
    assert!(ffi::content_read_is_denied(&qualified.authority.source));
}

#[test]
fn same_name_source_replacement_is_stale_even_while_the_old_handle_survives() {
    let fixture = Fixture::new();
    let sample = fixture.sample("replace-trigger.wav");
    let input = fixture.input(&[Some(&sample)]);
    let source = fixture.source.clone();
    let old = fixture.root.join("old-source.flp");
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "replace-trigger.wav" {
            fs::rename(&source, &old).unwrap();
            fs::write(&source, b"replacement has a different object identity").unwrap();
        }
    }));
    assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
}

#[test]
fn normal_in_place_project_save_succeeds_and_invalidates_the_old_fingerprint() {
    let fixture = Fixture::new();
    let sample = fixture.sample("save-trigger.wav");
    let input = fixture.input(&[Some(&sample)]);
    let source = fixture.source.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "save-trigger.wav" {
            fs::write(&source, b"ordinary shorter project save").unwrap();
        }
    }));
    assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
}

#[test]
fn existing_junction_is_rejected_before_open_without_probing_its_target() {
    let fixture = Fixture::new();
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("x.wav"), b"outside sentinel").unwrap();
    let link = fixture.root.join("jump");
    fixture.junction(&link, &outside);
    let candidate = link.join("x.wav");
    let input = fixture.input(&[Some(&candidate)]);
    let opens = Arc::new(Mutex::new(0));
    let counter = opens.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && (name == "jump" || name == "x.wav") {
            *counter.lock().unwrap() += 1;
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![unchecked(UncheckedReason::ReparseOrOffline)]
    );
    assert_eq!(*opens.lock().unwrap(), 0);
    assert_eq!(
        fs::read(outside.join("x.wav")).unwrap(),
        b"outside sentinel"
    );
}

#[test]
fn junction_inserted_between_attributes_and_open_is_never_followed() {
    let fixture = Fixture::new();
    let directory = fixture.root.join("race");
    fs::create_dir(&directory).unwrap();
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    let candidate = directory.join("x.wav");
    let input = fixture.input(&[Some(&candidate)]);
    let old = fixture.root.join("old-race");
    let links = fixture.links.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && name == "race" {
            fs::rename(&directory, &old).unwrap();
            links.lock().unwrap().push(directory.clone());
            ffi::junction(&directory, &outside);
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![unchecked(UncheckedReason::ReparseOrOffline)]
    );
}

#[test]
fn a_data_share_lock_does_not_block_metadata_only_observation() {
    let fixture = Fixture::new();
    let sample = fixture.sample("locked.wav");
    let input = fixture.input(&[Some(&sample)]);
    let _lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&sample)
        .unwrap();
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![Outcome::Present]
    );
}

#[test]
fn metadata_permission_denial_is_unchecked_instead_of_false_absence() {
    let fixture = Fixture::new();
    let sample = fixture.sample("denied.wav");
    let input = fixture.input(&[Some(&sample)]);
    let _restore = ffi::deny_read_attributes(&sample);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![unchecked(UncheckedReason::AccessDenied)]
    );
}

#[test]
fn injected_offline_recall_and_cloud_attributes_stop_before_candidate_open() {
    for bit in [0x400, 0x1000, 0x40000, 0x400000] {
        let fixture = Fixture::new();
        let sample = fixture.sample("excluded.wav");
        let input = fixture.input(&[Some(&sample)]);
        let opens = Arc::new(Mutex::new(0));
        let counter = opens.clone();
        let mut port = WindowsPort::new(Fence::default());
        port.attributes_filter = Some(Box::new(move |name, basic| {
            if name == "excluded.wav" {
                basic.attributes |= bit;
            }
        }));
        port.hook = Some(Box::new(move |stage, name| {
            if stage == TestStage::BeforeOpen && name == "excluded.wav" {
                *counter.lock().unwrap() += 1;
            }
        }));
        assert_eq!(
            outcomes(&run(&input, &mut port).unwrap()),
            vec![unchecked(UncheckedReason::ReparseOrOffline)]
        );
        assert_eq!(*opens.lock().unwrap(), 0);
    }
}

#[test]
fn native_leaf_case_lookup_uses_the_directory_mode_and_root_variants_remain_unproven() {
    let fixture = Fixture::new();
    fixture.sample("Kick.wav");
    let lower = fixture.root.join("kick.wav");
    let alternate_root = fixture.base.join("PROJECTS").join("Kick.wav");
    let input = fixture.input(&[Some(&lower), Some(&alternate_root)]);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![
            Outcome::Present,
            unchecked(UncheckedReason::UnsupportedCaseMode)
        ]
    );
}

#[test]
fn outside_sibling_and_network_references_never_produce_candidate_queries() {
    let fixture = Fixture::new();
    let sibling = fixture.base.join("projects2").join("outside.wav");
    let outside = fixture.base.join("outside.wav");
    let network = Path::new(r"\\never-contact.invalid\share\outside.wav");
    let input = fixture.input(&[Some(&sibling), Some(&outside), Some(network)]);
    let queries = Arc::new(Mutex::new(0));
    let counter = queries.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "outside.wav" {
            *counter.lock().unwrap() += 1;
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![
            unchecked(UncheckedReason::OutsideRoot),
            unchecked(UncheckedReason::OutsideRoot),
            unchecked(UncheckedReason::UnsupportedPathSyntax)
        ]
    );
    assert_eq!(*queries.lock().unwrap(), 0);
}

#[test]
fn source_identity_mismatch_fails_before_any_sample_query() {
    let fixture = Fixture::new();
    let sample = fixture.sample("x.wav");
    let input = fixture.input(&[Some(&sample)]);
    let mut changed = input.fence().clone();
    changed.source.identity.file += 1;
    let input = CapturedInput::new(
        input.context().clone(),
        fixture.root.to_str().unwrap(),
        fixture.source.to_str().unwrap(),
        changed,
        Some(vec![Some(sample.to_str().unwrap().into())]),
    )
    .unwrap();
    assert!(matches!(
        run(&input, &mut WindowsPort::new(Fence::default())),
        Err(RequestFailure::Stale)
    ));
}

#[test]
fn durable_revocation_during_a_probe_discards_results_and_stops_the_batch() {
    let fixture = Fixture::new();
    let sample = fixture.sample("revoke.wav");
    let later = fixture.sample("later.wav");
    let input = fixture.input(&[Some(&sample), Some(&later)]);
    let fence = Fence::default();
    let revoked = fence.revoked.clone();
    let later_queries = Arc::new(Mutex::new(0));
    let counter = later_queries.clone();
    let mut port = WindowsPort::new(fence);
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::AfterOpen && name == "revoke.wav" {
            revoked.store(true, Ordering::SeqCst);
        }
        if stage == TestStage::BeforeQuery && name == "later.wav" {
            *counter.lock().unwrap() += 1;
        }
    }));
    assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
    assert_eq!(*later_queries.lock().unwrap(), 0);
}

#[test]
fn case_sensitive_directories_keep_distinct_literal_leaf_names() {
    let fixture = Fixture::new();
    let folder = fixture.root.join("sensitive");
    fs::create_dir(&folder).unwrap();
    let handle = OpenOptions::new()
        .access_mode(0x100)
        .custom_flags(0x2000000)
        .open(&folder)
        .unwrap();
    assert!(
        ffi::set_case_sensitive(&handle),
        "owned NTFS case-mode fixture setup failed"
    );
    drop(handle);
    fs::write(folder.join("Kick.wav"), b"upper").unwrap();
    let upper = folder.join("Kick.wav");
    let lower = folder.join("kick.wav");
    let input = fixture.input(&[Some(&upper), Some(&lower)]);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![Outcome::Present, Outcome::NotFound]
    );
}

#[test]
fn wrong_size_or_high_resolution_timestamp_rejects_source_qualification() {
    let fixture = Fixture::new();
    let input = fixture.input(&[]);
    for change_size in [true, false] {
        let mut fence = input.fence().clone();
        if change_size {
            fence.source.byte_size += 1;
        } else {
            fence.source.modified_at_ns += 100;
        }
        let changed = CapturedInput::new(
            input.context().clone(),
            fixture.root.to_str().unwrap(),
            fixture.source.to_str().unwrap(),
            fence,
            Some(vec![]),
        )
        .unwrap();
        assert!(matches!(
            run(&changed, &mut WindowsPort::new(Fence::default())),
            Err(RequestFailure::Stale)
        ));
    }
}

#[test]
fn deliberately_preserved_identity_size_and_timestamp_is_not_a_content_freshness_claim() {
    let fixture = Fixture::new();
    let sample = fixture.sample("preserved.wav");
    let input = fixture.input(&[Some(&sample)]);
    let source = fixture.source.clone();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "preserved.wav" {
            let mut bytes = fs::read(&source).unwrap();
            bytes[0] ^= 1;
            fs::write(&source, bytes).unwrap();
            let file = OpenOptions::new().access_mode(0x100).open(&source).unwrap();
            file.set_times(fs::FileTimes::new().set_modified(modified))
                .unwrap();
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![Outcome::Present]
    );
}
