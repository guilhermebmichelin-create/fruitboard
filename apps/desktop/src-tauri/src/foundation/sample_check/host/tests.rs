use super::*;
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
use fruitboard_storage::{EncodedIdentity, LibraryQuery, ScanKind, ScanObservation};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

struct Fixture {
    base: PathBuf,
    host: Option<Arc<Host>>,
    request: Request,
    raw: Value,
    source: PathBuf,
    target: PathBuf,
    bytes: Vec<u8>,
    database_file: PathBuf,
    retained: bool,
}
impl Fixture {
    fn new() -> Self {
        Self::with_profile(None, None)
    }
    fn for_review() -> Self {
        match (
            std::env::var_os("FRUITBOARD_SAMPLE_CHECK_REVIEW_PROFILE"),
            std::env::var_os("FRUITBOARD_SAMPLE_CHECK_REVIEW_ROOT"),
        ) {
            (None, None) => Self::new(),
            (Some(profile), Some(root)) => {
                let profile = PathBuf::from(profile);
                let root = PathBuf::from(root);
                assert!(
                    profile.is_absolute()
                        && !profile.exists()
                        && root.is_absolute()
                        && !root.exists(),
                    "require fresh absolute owned paths"
                );
                let local_app_data = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
                assert_eq!(profile.parent(), Some(local_app_data.as_path()));
                assert!(
                    profile
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("com.fruitboard.desktop.sample-review-"),
                    "refuse owner profile identity"
                );
                Self::with_profile(Some(profile), Some(root))
            }
            _ => panic!("both explicit review paths are required"),
        }
    }
    fn with_profile(profile: Option<PathBuf>, root: Option<PathBuf>) -> Self {
        let path = root.unwrap_or_else(|| {
            std::env::temp_dir().join(format!("fruitboard-sample-host-{}", uuid::Uuid::now_v7()))
        });
        std::fs::create_dir(&path).unwrap();
        let full = std::fs::canonicalize(&path).unwrap();
        let full = full.to_str().unwrap();
        let base = PathBuf::from(full.strip_prefix(r"\\?\").unwrap_or(full));
        let root_path = base.join("projects");
        std::fs::create_dir(&root_path).unwrap();
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../fixtures/parser-corpus/FIX-FL2026-MULTISAMPLER.flp"),
        )
        .unwrap();
        let source = root_path.join("project.flp");
        std::fs::write(&source, &bytes).unwrap();
        let target = root_path.join("metadata-target.wav");
        std::fs::write(&target, b"private non-audio metadata sentinel").unwrap();
        let metadata = std::fs::metadata(&source).unwrap();
        let ns = i64::try_from(
            metadata
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();
        #[cfg(windows)]
        let identity = identity(&source);
        #[cfg(not(windows))]
        let identity = (7, 3);
        let retained = profile.is_some();
        let app_data = profile.unwrap_or_else(|| base.join("storage"));
        let database_file = app_data.join("storage/fruitboard.db");
        let mut db = Database::open(&app_data).unwrap();
        let root = db
            .add_scan_root("Private sample check", root_path.to_str().unwrap())
            .unwrap();
        let session = db.start_scan_session(1).unwrap();
        db.enqueue_scan(&root.id, ScanKind::Manual, 2).unwrap();
        let scan = db.lease_next_scan(&session.id, 3, 1000).unwrap().unwrap();
        db.stage_scan_observations(
            &scan.run.id,
            &session.id,
            &scan.run.lease_token,
            4,
            &[ScanObservation {
                locator_key: "v1:i:project.flp".into(),
                relative_path: "project.flp".into(),
                byte_size: bytes.len() as u64,
                modified_at_ns: i128::from(ns),
                identity: Some(EncodedIdentity {
                    volume_serial: identity.0.to_string(),
                    file_id: identity.1.to_string(),
                }),
            }],
        )
        .unwrap();
        db.publish_scan_run(&scan.run.id, &session.id, &scan.run.lease_token, 5)
            .unwrap();
        let location = db
            .query_library(&LibraryQuery {
                scan_root_id: root.id.clone(),
                page_size: 10,
                cursor: None,
                snapshot: None,
            })
            .unwrap()
            .locations
            .remove(0);
        let mut raw = parse_bytes(&bytes);
        assert_eq!(raw["channelCount"]["value"], 3);
        raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":ns / 1_000_000,"hash":{"algorithm":"sha256","value":sha256_hex(&bytes)}});
        raw["filesystemCreatedAtMs"] =
            json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
        // Constructed host projection, not a claim that these paths came from FL.
        raw["sampleReferences"] = json!({"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED","items":[
            {"status":"extracted","value":target.to_str().unwrap()},
            {"status":"extracted","value":root_path.join("absent.wav").to_str().unwrap()},
            {"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED"}]});
        let input = db.capture_metadata_input(&root.id, &location.id).unwrap();
        let snapshot = db
            .publish_metadata_snapshot(
                &input,
                &capabilities(),
                ProtocolReply::Result(raw.clone()),
                100,
            )
            .unwrap();
        let request = Request {
            schema_version: 1,
            root_id: root.id,
            location_id: location.id,
            snapshot_id: snapshot.id,
            request_id: uuid::Uuid::now_v7().to_string(),
            expected_byte_size: bytes.len().to_string(),
            expected_modified_at: super::super::super::scan_console::unix_ns_to_rfc3339(ns),
        };
        Self {
            base,
            host: Some(Host::new(Arc::new(Mutex::new(db)))),
            request,
            raw,
            source,
            target,
            bytes,
            database_file,
            retained,
        }
    }
    fn host(&self) -> &Arc<Host> {
        self.host.as_ref().unwrap()
    }
    fn publish(&self, raw: Value) -> String {
        let mut db = self.host().database.lock().unwrap();
        let input = db
            .capture_metadata_input(&self.request.root_id, &self.request.location_id)
            .unwrap();
        db.publish_metadata_snapshot(&input, &capabilities(), ProtocolReply::Result(raw), 101)
            .unwrap()
            .id
    }
    fn result(&self, request: Request) -> Response {
        match self.host().begin(request, 1000).unwrap() {
            Prepared::Immediate(response) => response,
            Prepared::Worker(ticket) => ticket.wait(),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let host = self.host.take().unwrap();
        host.shutdown();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Arc::strong_count(&host) != 1 {
            assert!(Instant::now() < deadline, "worker still owns fixture");
            std::thread::yield_now();
        }
        fn ordinary(path: &Path) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                assert_eq!(
                    std::fs::symlink_metadata(path).unwrap().file_attributes() & 0x400,
                    0
                );
            }
            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                let kind = entry.file_type().unwrap();
                assert!(!kind.is_symlink());
                if kind.is_dir() {
                    ordinary(&entry.path());
                }
            }
        }
        ordinary(&self.base);
        drop(host);
        if !self.retained {
            std::fs::remove_dir_all(&self.base).unwrap();
        }
    }
}
fn capabilities() -> fruitboard_flp_parser::validation::ParserCapabilities {
    validate_descriptor(&json!({"adapter":ADAPTER_ID,"adapterVersion":ADAPTER_VERSION,
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024})).unwrap()
}
#[cfg(windows)]
fn identity(path: &Path) -> (u64, u128) {
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    struct Id {
        volume: u64,
        bytes: [u8; 16],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandleEx(
            handle: *mut std::ffi::c_void,
            class: u32,
            data: *mut std::ffi::c_void,
            size: u32,
        ) -> i32;
    }
    let file = std::fs::OpenOptions::new()
        .access_mode(0x80 | 0x100000)
        .share_mode(7)
        .custom_flags(0x200000)
        .open(path)
        .unwrap();
    let mut id = Id {
        volume: 0,
        bytes: [0; 16],
    };
    assert_ne!(
        unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                18,
                (&mut id as *mut Id).cast(),
                std::mem::size_of::<Id>() as u32,
            )
        },
        0
    );
    (id.volume, u128::from_le_bytes(id.bytes))
}
fn response(prepared: Prepared) -> Response {
    match prepared {
        Prepared::Immediate(value) => value,
        Prepared::Worker(ticket) => ticket.wait(),
    }
}

#[test]
#[cfg(windows)]
fn current_authorized_command_checks_metadata_without_mutating_source_target_or_database() {
    let fixture = Fixture::for_review();
    let database = &fixture.database_file;
    let before_db = std::fs::read(database).unwrap();
    let before_target = std::fs::read(&fixture.target).unwrap();
    let result = serde_json::to_value(fixture.result(fixture.request.clone())).unwrap();
    assert_eq!(result["state"], "complete");
    assert_eq!(
        result["report"]["channels"],
        json!([{ "position":1,"status":"present" },{ "position":2,"status":"not_found" },{ "position":3,"status":"no_saved_reference" }])
    );
    assert!(!result.to_string().contains("metadata-target"));
    assert_eq!(std::fs::read(&fixture.source).unwrap(), fixture.bytes);
    assert_eq!(std::fs::read(&fixture.target).unwrap(), before_target);
    assert_eq!(std::fs::read(database).unwrap(), before_db);
    assert!(result.to_string().len() < 64 * 1024);
    if fixture.retained {
        std::fs::write(fixture.base.join("review-profile.json"),serde_json::to_vec(&json!({
            "request":fixture.request,"database":fixture.database_file.to_str(),"source":fixture.source.to_str(),"target":fixture.target.to_str(),
            "sourceSha256":sha256_hex(&fixture.bytes),"targetSha256":sha256_hex(&before_target),"constructedProjection":true,"report":result
        })).unwrap()).unwrap();
    }
}

#[test]
fn invalid_root_snapshot_and_displayed_fingerprints_start_no_worker() {
    let fixture = Fixture::new();
    for field in ["root", "snapshot", "size", "mtime", "location"] {
        let mut request = fixture.request.clone();
        match field {
            "root" => request.root_id = "foreign-root".into(),
            "snapshot" => request.snapshot_id = "old-snapshot".into(),
            "size" => request.expected_byte_size = "1".into(),
            "mtime" => request.expected_modified_at = "2026-01-01T00:00:00Z".into(),
            _ => request.location_id = "foreign-location".into(),
        }
        let prepared = fixture
            .host()
            .begin_with(request, 1000, |_, _, _, _| {
                panic!("stale input reached worker")
            })
            .unwrap();
        assert_eq!(response(prepared).state, "stale");
    }
}

#[test]
fn empty_references_admit_no_filesystem_operations_and_later_snapshot_invalidates_old_request() {
    let fixture = Fixture::new();
    let mut raw = fixture.raw.clone();
    raw["sampleReferences"] = json!({"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED","items":vec![json!({"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED"});3]});
    let snapshot = fixture.publish(raw);
    assert_eq!(fixture.result(fixture.request.clone()).state, "stale");
    let mut request = fixture.request.clone();
    request.snapshot_id = snapshot;
    assert_eq!(
        response(
            fixture
                .host()
                .begin_with(request, 1000, |_, _, _, _| panic!(
                    "empty references reached filesystem"
                ))
                .unwrap()
        )
        .state,
        "no_references"
    );
}

fn blocked(fixture: &Fixture) -> (Ticket, mpsc::Sender<()>, mpsc::Receiver<()>) {
    let (release, wait) = mpsc::channel();
    let (started, ready) = mpsc::channel();
    let (done, retired) = mpsc::channel();
    let prepared = fixture
        .host()
        .begin_with(fixture.request.clone(), 1000, move |_, lease, _, _| {
            started.send(()).unwrap();
            wait.recv().unwrap();
            let result = lease.control().poll_deadline();
            done.send(()).unwrap();
            Err(result.err().unwrap_or(RequestFailure::InvalidInput))
        })
        .unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let Prepared::Worker(ticket) = prepared else {
        panic!("worker was not admitted")
    };
    (ticket, release, retired)
}
fn await_retirement(fixture: &Fixture, retired: mpsc::Receiver<()>) {
    retired.recv_timeout(Duration::from_secs(2)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while fixture.host().active.lock().unwrap().is_some() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}
#[test]
fn cancellation_returns_while_blocked_keeps_gate_busy_and_discards_late_result() {
    let fixture = Fixture::new();
    let (ticket, release, retired) = blocked(&fixture);
    let mut wrong = fixture.request.clone();
    wrong.request_id = "other-request".into();
    fixture.host().cancel(&wrong);
    assert!(ticket.control.poll_deadline().is_ok());
    fixture.host().cancel(&fixture.request);
    let started = Instant::now();
    assert_eq!(ticket.wait().state, "cancelled");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(
        response(fixture.host().begin(wrong, 1000).unwrap()).state,
        "busy"
    );
    release.send(()).unwrap();
    await_retirement(&fixture, retired);
}

#[test]
#[cfg(windows)]
fn native_cancel_after_worker_publication_fences_the_pending_transport_reply() {
    let fixture = Fixture::new();
    let Prepared::Worker(ticket) = fixture.host().begin(fixture.request.clone(), 1000).unwrap()
    else {
        panic!("expected worker")
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ticket.delivery.worker_done.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    let mut next = fixture.request.clone();
    next.request_id = "next-request".into();
    assert_eq!(
        response(fixture.host().begin(next, 1000).unwrap()).state,
        "busy"
    );
    fixture.host().cancel(&fixture.request);
    assert_eq!(ticket.wait().state, "cancelled");
    assert!(fixture.host().active.lock().unwrap().is_none());
}

#[test]
#[cfg(windows)]
fn source_save_before_watcher_publication_discards_all_observations() {
    let fixture = Fixture::new();
    std::fs::File::options()
        .write(true)
        .open(&fixture.source)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(1_000_000_000)),
        )
        .unwrap();
    assert_eq!(fixture.result(fixture.request.clone()).state, "stale");
    assert_eq!(std::fs::read(&fixture.source).unwrap(), fixture.bytes);
}
#[test]
fn deadline_returns_while_blocked_without_admitting_a_replacement_worker() {
    let fixture = Fixture::new();
    let (ticket, release, retired) = blocked(&fixture);
    let started = Instant::now();
    assert_eq!(ticket.wait().state, "deadline");
    assert!(started.elapsed() < Duration::from_secs(3));
    let mut next = fixture.request.clone();
    next.request_id = "next-request".into();
    assert_eq!(
        response(fixture.host().begin(next, 1000).unwrap()).state,
        "busy"
    );
    release.send(()).unwrap();
    await_retirement(&fixture, retired);
}
#[test]
fn root_revocation_and_runtime_shutdown_signal_blocked_controls() {
    for shutdown in [false, true] {
        let fixture = Fixture::new();
        let (ticket, release, retired) = blocked(&fixture);
        if shutdown {
            fixture.host().shutdown();
        } else {
            fixture
                .host()
                .database
                .lock()
                .unwrap()
                .set_scan_root_enabled(&fixture.request.root_id, false)
                .unwrap();
        }
        assert_eq!(ticket.wait().state, "stale");
        release.send(()).unwrap();
        await_retirement(&fixture, retired);
    }
}
#[test]
#[cfg(windows)]
fn database_guard_is_released_before_native_qualification_and_superseded_projection_cannot_publish()
{
    let fixture = Fixture::new();
    let host = fixture.host().clone();
    let raw = fixture.raw.clone();
    let request = fixture.request.clone();
    let (observed, ready) = mpsc::channel();
    let prepared = fixture
        .host()
        .begin_with(
            request.clone(),
            1000,
            move |capture, lease, authorization, now| {
                // Actual held metadata qualification occurs only after this DB guard is released.
                {
                    let mut db = host
                        .database
                        .try_lock()
                        .expect("DB held across filesystem worker");
                    let input = db
                        .capture_metadata_input(&request.root_id, &request.location_id)
                        .unwrap();
                    db.publish_metadata_snapshot(
                        &input,
                        &capabilities(),
                        ProtocolReply::Result(raw),
                        102,
                    )
                    .unwrap();
                }
                observed.send(()).unwrap();
                fruitboard_sample_presence::check(
                    &capture.input,
                    lease,
                    &mut fruitboard_sample_presence::native::WindowsPort::new(authorization),
                    now,
                )
            },
        )
        .unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(response(prepared).state, "stale");
}
