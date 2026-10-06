//! One bounded metadata worker, with a separate revocation/deadline waiter.
//! Timed-out/cancelled calls retain admission until their kernel work retires.
use super::{AppError, Prepared, Request, Response};
use fruitboard_sample_presence::{
    AdmissionError, AuthorityFence, CapturedInput, Context, MonotonicClock, ObjectIdentity, Report,
    RequestControl, RequestFailure, SourceFingerprint, UncheckedReason, WorkerGate, WorkerLease,
};
use fruitboard_storage::{Database, MetadataSource, StorageError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

struct Clock(Instant);
impl MonotonicClock for Clock {
    fn now_ms(&self) -> u64 {
        self.0.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
    }
}
struct Active {
    request: Request,
    control: RequestControl,
    delivery: Arc<Delivery>,
}
#[derive(Default)]
struct Delivery {
    cancelled: AtomicBool,
    worker_done: AtomicBool,
    waiter_done: AtomicBool,
}
pub(super) struct Host {
    database: Arc<Mutex<Database>>,
    gate: WorkerGate,
    session: String,
    stopped: AtomicBool,
    active: Mutex<Option<Active>>,
}
struct Capture {
    source: MetadataSource,
    input: CapturedInput,
}
pub(crate) struct Ticket {
    host: Arc<Host>,
    request: Request,
    source: MetadataSource,
    control: RequestControl,
    receiver: mpsc::Receiver<Result<Report, RequestFailure>>,
    delivery: Arc<Delivery>,
}
impl Host {
    pub(super) fn new(database: Arc<Mutex<Database>>) -> Arc<Self> {
        Arc::new(Self {
            database,
            gate: WorkerGate::default(),
            session: uuid::Uuid::now_v7().to_string(),
            stopped: AtomicBool::new(false),
            active: Mutex::new(None),
        })
    }
    fn capture(&self, request: &Request) -> Result<Capture, RequestFailure> {
        let db = self
            .database
            .try_lock()
            .map_err(|_| RequestFailure::Unavailable(UncheckedReason::LimitReached))?;
        let input = db
            .capture_metadata_input(&request.root_id, &request.location_id)
            .map_err(|_| RequestFailure::Stale)?;
        if input.byte_size().to_string() != request.expected_byte_size
            || super::super::scan_console::unix_ns_to_rfc3339(input.modified_at_ns())
                != request.expected_modified_at
        {
            return Err(RequestFailure::Stale);
        }
        let snapshot = db
            .current_metadata_snapshot(input.project_file_id())
            .map_err(|_| RequestFailure::Stale)?
            .ok_or(RequestFailure::Stale)?;
        if snapshot.header().id != request.snapshot_id {
            return Err(RequestFailure::Stale);
        }
        let source = db
            .capture_metadata_source(&input)
            .map_err(|error| match error {
                StorageError::NotFound => {
                    RequestFailure::Unavailable(UncheckedReason::UnqualifiedFilesystem)
                }
                _ => RequestFailure::Stale,
            })?;
        let references = super::super::project_details::saved_sample_values(
            snapshot
                .payload_json()
                .ok_or(RequestFailure::ReferencesUnavailable)?,
        )
        .map_err(|_| RequestFailure::InvalidInput)?;
        let context = Context::new(
            &request.request_id,
            &request.root_id,
            &request.location_id,
            &request.snapshot_id,
            &self.session,
        )?;
        let revisions = input.parse_context();
        let (volume, file) = source.identity();
        let captured = CapturedInput::new(
            context,
            source.root(),
            source.path(),
            AuthorityFence {
                root_revision: revisions.root_revision,
                location_revision: revisions.file_revision,
                // The root grant is stored as a locator/revision, not a directory ID.
                // The native adapter captures its actual ID under pinned ancestors.
                root_identity: None,
                source: SourceFingerprint {
                    identity: ObjectIdentity { volume, file },
                    byte_size: input.byte_size(),
                    modified_at_ns: input.modified_at_ns(),
                },
            },
            references,
        )?;
        Ok(Capture {
            source,
            input: captured,
        })
    }
    pub(super) fn begin(
        self: &Arc<Self>,
        request: Request,
        now: u64,
    ) -> Result<Prepared, AppError> {
        self.begin_with(request, now, |capture, lease, authorization, now| {
            #[cfg(windows)]
            {
                fruitboard_sample_presence::check(
                    &capture.input,
                    lease,
                    &mut fruitboard_sample_presence::native::WindowsPort::new(authorization),
                    now,
                )
            }
            #[cfg(not(windows))]
            {
                let _ = (capture, lease, authorization, now);
                Err(RequestFailure::Unavailable(
                    UncheckedReason::UnqualifiedFilesystem,
                ))
            }
        })
    }
    fn begin_with(
        self: &Arc<Self>,
        request: Request,
        now: u64,
        run: impl FnOnce(
            &Capture,
            &mut WorkerLease,
            Authorization,
            u64,
        ) -> Result<Report, RequestFailure>
        + Send
        + 'static,
    ) -> Result<Prepared, AppError> {
        let mut active = self.active.lock().map_err(|_| crate::storage_failed())?;
        if self.stopped.load(Ordering::Acquire) {
            return Ok(Prepared::Immediate(Response::state(request, "stale")));
        }
        if active.is_some() {
            return Ok(Prepared::Immediate(Response::state(request, "busy")));
        }
        let mut lease = match self.gate.try_start(Arc::new(Clock(Instant::now()))) {
            Ok(lease) => lease,
            Err(AdmissionError::Busy) => {
                return Ok(Prepared::Immediate(Response::state(request, "busy")));
            }
            Err(AdmissionError::Closed) => {
                return Ok(Prepared::Immediate(Response::state(request, "stale")));
            }
        };
        let capture = match self.capture(&request) {
            Ok(capture) => capture,
            Err(error) => return Ok(Prepared::Immediate(failure(request, error))),
        };
        // Empty/old projections never start a worker or qualify any object.
        if !capture.input.has_saved_references() {
            return Ok(Prepared::Immediate(Response::state(
                request,
                "no_references",
            )));
        }
        let control = lease.control();
        let delivery = Arc::new(Delivery::default());
        *active = Some(Active {
            request: request.clone(),
            control: control.clone(),
            delivery: delivery.clone(),
        });
        let authorization = Authorization {
            host: self.clone(),
            source: capture.source.clone(),
            snapshot: request.snapshot_id.clone(),
            context: capture.input.context().clone(),
            fence: capture.input.fence().clone(),
            control: control.clone(),
        };
        let source = capture.source.clone();
        let (sender, receiver) = mpsc::channel();
        let host = self.clone();
        let worker_request = request.clone();
        let worker_delivery = delivery.clone();
        // Admission precedes spawning; no replacement worker can be launched
        // while an earlier syscall is blocked, even after its waiter returns.
        let spawned = std::thread::Builder::new()
            .name("fruitboard-samples".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(&capture, &mut lease, authorization, now)
                }))
                .unwrap_or(Err(RequestFailure::InvalidInput));
                // Retire authority/handles before releasing admission. Do not hold
                // the active mutex across dropping native objects or sending output.
                drop(capture);
                if let Ok(mut active) = host.active.lock() {
                    worker_delivery.worker_done.store(true, Ordering::Release);
                    if worker_delivery.waiter_done.load(Ordering::Acquire)
                        && active
                            .as_ref()
                            .is_some_and(|entry| entry.request == worker_request)
                    {
                        *active = None;
                    }
                    // Admission and active identity change under the same host lock.
                    drop(lease);
                } else {
                    drop(lease);
                }
                let _ = sender.send(result);
            });
        if spawned.is_err() {
            *active = None;
            return Err(crate::storage_failed());
        }
        Ok(Prepared::Worker(Ticket {
            host: self.clone(),
            request,
            source,
            control,
            receiver,
            delivery,
        }))
    }
    pub(super) fn cancel(&self, request: &Request) {
        if let Ok(active) = self.active.lock()
            && let Some(active) = active.as_ref().filter(|a| a.request == *request)
        {
            active.delivery.cancelled.store(true, Ordering::Release);
            active.control.cancel();
        }
    }
    pub(super) fn shutdown(&self) {
        self.stopped.store(true, Ordering::Release);
        self.gate.shutdown();
    }
    fn current(
        &self,
        source: &MetadataSource,
        snapshot: &str,
    ) -> Result<Option<bool>, RequestFailure> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(RequestFailure::Stale);
        }
        match self.database.try_lock() {
            Ok(db) => db
                .metadata_source_current(source, snapshot)
                .map(Some)
                .map_err(|_| RequestFailure::Stale),
            Err(std::sync::TryLockError::WouldBlock) => Ok(None),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(RequestFailure::Stale),
        }
    }
}
struct Authorization {
    host: Arc<Host>,
    source: MetadataSource,
    snapshot: String,
    context: Context,
    fence: AuthorityFence,
    control: RequestControl,
}
#[cfg(windows)]
impl fruitboard_sample_presence::native::CurrentAuthorization for Authorization {
    fn current(&mut self, context: &Context, fence: &AuthorityFence) -> Result<(), RequestFailure> {
        if *context != self.context || *fence != self.fence {
            return Err(RequestFailure::Stale);
        }
        loop {
            self.control.poll_deadline()?;
            match self.host.current(&self.source, &self.snapshot)? {
                Some(true) => return Ok(()),
                Some(false) => return Err(RequestFailure::Stale),
                // A concurrent short DB read is not evidence of a change.
                None => std::thread::park_timeout(Duration::from_millis(1)),
            }
        }
    }
}
impl Ticket {
    pub(crate) fn wait(self) -> Response {
        loop {
            if self.delivery.cancelled.load(Ordering::Acquire) {
                return failure(self.request.clone(), RequestFailure::Cancelled);
            }
            if let Err(error) = self.control.poll_deadline() {
                // Publication/retirement can precede the channel send. Its
                // actual outcome still comes exclusively from this receiver.
                if error != RequestFailure::WorkerRetired {
                    return failure(self.request.clone(), error);
                }
            }
            match self.host.current(&self.source, &self.request.snapshot_id) {
                Ok(Some(true) | None) => {}
                _ => {
                    self.control.invalidate();
                    return failure(self.request.clone(), RequestFailure::Stale);
                }
            }
            match self.receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(Ok(report)) => {
                    if self.delivery.cancelled.load(Ordering::Acquire) {
                        return failure(self.request.clone(), RequestFailure::Cancelled);
                    }
                    // The library worker's successful publish retires its control;
                    // correlate and recheck the durable fence at transport return.
                    if self.host.current(&self.source, &self.request.snapshot_id) != Ok(Some(true))
                    {
                        return failure(self.request.clone(), RequestFailure::Stale);
                    }
                    if report.to_json().is_err() {
                        return failure(self.request.clone(), RequestFailure::InvalidInput);
                    }
                    return Response {
                        request: self.request.clone(),
                        state: "complete",
                        reason: None,
                        report: Some(report),
                    };
                }
                Ok(Err(error)) => return failure(self.request.clone(), error),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return failure(self.request.clone(), RequestFailure::InvalidInput);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.delivery.waiter_done.store(true, Ordering::Release);
        if let Ok(mut active) = self.host.active.lock()
            && self.delivery.worker_done.load(Ordering::Acquire)
            && active
                .as_ref()
                .is_some_and(|entry| entry.request == self.request)
        {
            *active = None;
        }
    }
}
fn failure(request: Request, error: RequestFailure) -> Response {
    let (state, reason) = match error {
        RequestFailure::ReferencesUnavailable => ("references_unavailable", None),
        RequestFailure::Stale | RequestFailure::WorkerRetired => ("stale", None),
        RequestFailure::Cancelled => ("cancelled", None),
        RequestFailure::Deadline => ("deadline", None),
        RequestFailure::Unavailable(reason) => ("unavailable", Some(reason)),
        RequestFailure::InvalidInput => ("failed", None),
    };
    Response {
        request,
        state,
        reason,
        report: None,
    }
}

#[cfg(test)]
mod tests;
