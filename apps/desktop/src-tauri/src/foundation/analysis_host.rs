//! Explicit development-only analysis composition. One worker and one parser;
//! source/transport operations release the native DB mutex.
use fruitboard_analysis_execution::{
    AnalysisClock, AnalysisWorker, ParserPort, SystemClock, native::WindowsAuthority,
};
use fruitboard_flp_parser::supervisor::{
    CancellationToken, ParserRequest, ParserSupervisor, ProtocolReply, SupervisorError,
    SupervisorLimits,
};
use fruitboard_flp_parser::{AuthorizedSource, authorize_source};
use fruitboard_storage::{Database, MAX_ANALYSIS_BATCH, StorageError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Fixed native sibling only. Hold its authorized file/ancestor handles across
/// restarts, preventing Windows replacement between selection and launch.
struct TrustedParser {
    application: PathBuf,
    supervisor: Option<ParserSupervisor>,
    held: Option<AuthorizedSource>,
}
impl ParserPort for TrustedParser {
    fn request(
        &mut self,
        request: ParserRequest,
        cancellation: &CancellationToken,
    ) -> Result<ProtocolReply, SupervisorError> {
        if cancellation.is_cancelled() {
            return Err(SupervisorError::Cancelled);
        }
        if self.supervisor.is_none() {
            let executable = super::packaged_parser::installed_executable(&self.application)
                .map_err(|_| SupervisorError::StartFailed)?;
            let held = authorize_source(
                &executable,
                executable.parent().ok_or(SupervisorError::StartFailed)?,
            )
            .map_err(|_| SupervisorError::StartFailed)?;
            let supervisor = ParserSupervisor::new(executable, SupervisorLimits::default())?;
            self.held = Some(held);
            self.supervisor = Some(supervisor);
        }
        self.supervisor
            .as_mut()
            .ok_or(SupervisorError::StartFailed)?
            .request(request, cancellation)
    }
    fn shutdown(&mut self) -> Result<(), SupervisorError> {
        if let Some(mut supervisor) = self.supervisor.take() {
            supervisor.shutdown()?;
        }
        self.held.take();
        Ok(())
    }
}

struct Running {
    stop: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
    join: JoinHandle<()>,
}
pub(crate) struct AnalysisHost {
    database: Arc<Mutex<Database>>,
    running: Mutex<Option<Running>>,
}
impl AnalysisHost {
    pub(crate) fn is_available(&self) -> bool {
        self.running.lock().ok().is_some_and(|running| {
            running.as_ref().is_some_and(|active| {
                !active.stop.load(Ordering::Acquire) && !active.join.is_finished()
            })
        })
    }
    pub(crate) fn new(database: Arc<Mutex<Database>>) -> Self {
        Self {
            database,
            running: Mutex::new(None),
        }
    }
    pub(crate) fn start(&self, application: &Path) -> Result<(), StorageError> {
        let mut running = self.running.lock().map_err(|_| StorageError::Io)?;
        if running.is_some() {
            return Err(StorageError::Conflict);
        }
        let clock = SystemClock;
        let session = self
            .database
            .lock()
            .map_err(|_| StorageError::Io)?
            .start_analysis_session(clock.now_ms())?;
        let database = self.database.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (wake, receiver) = mpsc::channel();
        let application = application.to_owned();
        let join = thread::Builder::new()
            .name("fruitboard-analysis".into())
            .spawn(move || {
                let mut worker = AnalysisWorker::new(
                    TrustedParser {
                        application,
                        supervisor: None,
                        held: None,
                    },
                    WindowsAuthority,
                );
                let mut cursor = None;
                while !stopped.load(Ordering::Acquire) {
                    let claimed = (|| {
                        let mut db = database.lock().map_err(|_| StorageError::Io)?;
                        cursor = db.discover_analysis_jobs(
                            cursor.as_ref(),
                            MAX_ANALYSIS_BATCH,
                            clock.now_ms(),
                        )?;
                        db.claim_analysis_job(&session, clock.now_ms())
                    })();
                    let lease = match claimed {
                        Ok(lease) => lease,
                        Err(_) => break,
                    };
                    if let Some(lease) = lease {
                        let cancellation = CancellationToken::default();
                        // Bounded, scoped monitor: durable cancel/root/source/session
                        // revocation interrupts a blocked parser request promptly.
                        thread::scope(|scope| {
                            let token = cancellation.clone();
                            let (done, done_rx) = mpsc::channel();
                            let monitor_database = database.clone();
                            let monitor_stop = stopped.clone();
                            let monitor_lease = &lease;
                            let monitor_clock = &clock;
                            let monitor = scope.spawn(move || {
                                loop {
                                    if monitor_stop.load(Ordering::Acquire) {
                                        token.cancel();
                                        break;
                                    }
                                    match monitor_database.try_lock() {
                                        Ok(db) => {
                                            if !db
                                                .analysis_lease_current(
                                                    monitor_lease,
                                                    monitor_clock.now_ms(),
                                                )
                                                .unwrap_or(false)
                                            {
                                                token.cancel();
                                                break;
                                            }
                                        }
                                        Err(std::sync::TryLockError::Poisoned(_)) => {
                                            token.cancel();
                                            break;
                                        }
                                        Err(std::sync::TryLockError::WouldBlock) => {}
                                    }
                                    match done_rx.recv_timeout(Duration::from_millis(25)) {
                                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                                    }
                                }
                            });
                            let result = worker.execute(&database, &lease, &clock, &cancellation);
                            let _ = done.send(());
                            let _ = monitor.join();
                            if result.is_err() {
                                stopped.store(true, Ordering::Release);
                            }
                        });
                    }
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
                let _ = worker.shutdown();
            })
            .map_err(|_| StorageError::Io)?;
        *running = Some(Running { stop, wake, join });
        Ok(())
    }
    pub(crate) fn shutdown(&self) {
        let mut running = self
            .running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Keep lifecycle ownership through join: a concurrent start must wait
        // until this worker and its owned parser have retired.
        if let Some(active) = running.take() {
            active.stop.store(true, Ordering::Release);
            let _ = active.wake.send(());
            let _ = active.join.join();
        }
    }
}
impl Drop for AnalysisHost {
    fn drop(&mut self) {
        self.shutdown();
    }
}

const ANALYSIS_SMOKE_TIMEOUT: Duration = Duration::from_secs(20);
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AnalysisEvidence {
    parsed_and_persisted: bool,
    snapshot_id: String,
    source_sha256: String,
    payload_sha256: String,
}
/// Installed development evidence only. The native harness independently
/// copies an approved fixture into this fixed sibling of its evidence output.
/// Normal launches never run this fixture admission/probe.
pub(super) fn smoke_evidence(
    app: &tauri::AppHandle,
    output: &Path,
) -> Result<AnalysisEvidence, &'static str> {
    use tauri::Manager;
    let folder = output
        .parent()
        .and_then(Path::parent)
        .ok_or("analysis_fixture_missing")?
        .join("Analysis Fixture");
    let canonical = crate::canonical_scan_root(folder.to_str().ok_or("analysis_fixture_missing")?)
        .ok_or("analysis_fixture_missing")?;
    let foundation = app.state::<crate::NativeFoundation>();
    let root = {
        let db = foundation
            .preferences
            .database
            .lock()
            .map_err(|_| "analysis_storage_failed")?;
        db.list_scan_roots()
            .map_err(|_| "analysis_storage_failed")?
            .into_iter()
            .find(|root| root.canonical_path == canonical)
    };
    let root = match root {
        Some(root) => root,
        None => foundation
            .scan_roots
            .add_scan_root("Foundation analysis".into(), canonical, None)
            .map_err(|_| "analysis_root_failed")?,
    };
    foundation
        .scan_console
        .scan_now(root.id.clone())
        .map_err(|_| "analysis_scan_failed")?;
    let deadline = std::time::Instant::now() + ANALYSIS_SMOKE_TIMEOUT;
    loop {
        {
            let db = foundation
                .preferences
                .database
                .lock()
                .map_err(|_| "analysis_storage_failed")?;
            let page = db
                .query_library(&fruitboard_storage::LibraryQuery {
                    scan_root_id: root.id.clone(),
                    page_size: 10,
                    cursor: None,
                    snapshot: None,
                })
                .map_err(|_| "analysis_storage_failed")?;
            if let Some(location) = page
                .locations
                .iter()
                .find(|l| l.relative_path == "sample.flp")
                && let Some(snapshot) = db
                    .current_metadata_snapshot(&location.project_file_id)
                    .map_err(|_| "analysis_storage_failed")?
            {
                if snapshot.header().outcome != fruitboard_storage::MetadataOutcome::Complete {
                    return Err("analysis_result_failed");
                }
                let payload = snapshot.payload_json().ok_or("analysis_result_failed")?;
                return Ok(AnalysisEvidence {
                    parsed_and_persisted: true,
                    snapshot_id: snapshot.header().id.clone(),
                    source_sha256: snapshot
                        .header()
                        .input_content_sha256
                        .clone()
                        .ok_or("analysis_result_failed")?,
                    payload_sha256: fruitboard_flp_parser::sha256_hex(payload.as_bytes()),
                });
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err("analysis_result_timeout");
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analysis_host_starts_one_owner_and_shuts_down_without_a_parser_or_source() {
        let directory =
            std::env::temp_dir().join(format!("fruitboard-analysis-host-{}", uuid::Uuid::now_v7()));
        let database = Arc::new(Mutex::new(Database::open(&directory).unwrap()));
        let host = AnalysisHost::new(database.clone());
        assert!(!host.is_available());
        let application = directory.join("application.exe");
        host.start(&application).unwrap();
        assert!(host.is_available());
        assert!(matches!(
            host.start(&application),
            Err(StorageError::Conflict)
        ));
        host.shutdown();
        assert!(!host.is_available());
        host.shutdown();
        host.start(&application).unwrap();
        host.shutdown();
        drop(host);
        drop(database);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
