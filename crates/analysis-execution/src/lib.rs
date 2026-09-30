//! One native parser request at a time. Filesystem/parser work never holds the
//! shared database mutex; sealed lease and fresh source guard authorize commit.
use fruitboard_flp_parser::ExpectedFingerprint;
use fruitboard_flp_parser::supervisor::{
    CancellationToken, ParseRequest, ParserRequest, ParserSupervisor, ProtocolReply,
    SupervisorError,
};
use fruitboard_flp_parser::validation::{ParserCapabilities, validate_descriptor};
use fruitboard_storage::{
    AnalysisFailure, AnalysisLease, AnalysisSource, AnalysisState, Database, StorageError,
};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
pub mod native;

#[derive(Clone, PartialEq, Eq)]
pub struct SourceObservation {
    pub byte_size: u64,
    pub modified_at_ns: i64,
    pub identity: Option<(u64, u128)>,
    pub sha256: String,
}
pub trait OpenSource {
    fn observe(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<SourceObservation, AnalysisFailure>;
}
pub trait SourceAuthority {
    type Guard: OpenSource;
    fn open(
        &self,
        source: &AnalysisSource,
        cancellation: &CancellationToken,
    ) -> Result<Self::Guard, AnalysisFailure>;
}
/// Trusted native injection seam, never renderer-selected adapter/arguments.
pub trait ParserPort {
    fn request(
        &mut self,
        request: ParserRequest,
        cancellation: &CancellationToken,
    ) -> Result<ProtocolReply, SupervisorError>;
    fn shutdown(&mut self) -> Result<(), SupervisorError>;
}
impl ParserPort for ParserSupervisor {
    fn request(
        &mut self,
        request: ParserRequest,
        cancellation: &CancellationToken,
    ) -> Result<ProtocolReply, SupervisorError> {
        ParserSupervisor::request(self, request, cancellation)
    }
    fn shutdown(&mut self) -> Result<(), SupervisorError> {
        ParserSupervisor::shutdown(self)
    }
}
pub trait AnalysisClock: Send + Sync {
    fn now_ms(&self) -> i64;
}
pub struct SystemClock;
impl AnalysisClock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Complete,
    Unsupported,
    Failed,
    Cancelled,
    Stale,
    RetryQueued,
}
fn lock(database: &Mutex<Database>) -> Result<std::sync::MutexGuard<'_, Database>, StorageError> {
    database.lock().map_err(|_| StorageError::Io)
}
fn outcome(state: AnalysisState) -> ExecutionOutcome {
    match state {
        AnalysisState::Complete => ExecutionOutcome::Complete,
        AnalysisState::Unsupported => ExecutionOutcome::Unsupported,
        AnalysisState::Cancelled => ExecutionOutcome::Cancelled,
        AnalysisState::Stale => ExecutionOutcome::Stale,
        AnalysisState::Queued => ExecutionOutcome::RetryQueued,
        _ => ExecutionOutcome::Failed,
    }
}
fn failure(
    database: &Mutex<Database>,
    lease: &AnalysisLease,
    error: AnalysisFailure,
    now: i64,
) -> Result<ExecutionOutcome, StorageError> {
    let mut db = lock(database)?;
    match db.fail_analysis_job(lease, error, now) {
        Ok(()) => {}
        Err(StorageError::Conflict) => {
            return Ok(
                if db.analysis_status(lease.location_id())?.is_some_and(|s| {
                    s.job_id == lease.job_id() && s.state == AnalysisState::Cancelled
                }) {
                    ExecutionOutcome::Cancelled
                } else {
                    ExecutionOutcome::Stale
                },
            );
        }
        Err(e) => return Err(e),
    }
    // Status carries only opaque IDs and safe codes; no private parser payload.
    let state = db
        .analysis_status(lease.location_id())?
        .ok_or(StorageError::NotFound)?
        .state;
    Ok(outcome(state))
}

pub struct AnalysisWorker<P, A> {
    parser: P,
    authority: A,
}
impl<P: ParserPort, A: SourceAuthority> AnalysisWorker<P, A> {
    pub fn new(parser: P, authority: A) -> Self {
        Self { parser, authority }
    }
    pub fn shutdown(&mut self) -> Result<(), SupervisorError> {
        self.parser.shutdown()
    }

    /// The caller's scoped cancellation token is polled by the supervisor and
    /// source reader. The desktop monitor cancels it when durable authority is
    /// revoked; the writer independently checks authority again at commit.
    pub fn execute(
        &mut self,
        database: &Arc<Mutex<Database>>,
        lease: &AnalysisLease,
        clock: &dyn AnalysisClock,
        cancellation: &CancellationToken,
    ) -> Result<ExecutionOutcome, StorageError> {
        let result = self.parse(database, lease, clock, cancellation);
        match result {
            Ok((guard, capabilities, reply, observation)) => {
                let mut db = lock(database)?;
                if cancellation.is_cancelled() {
                    drop(db);
                    return failure(
                        database,
                        lease,
                        AnalysisFailure::Interrupted,
                        clock.now_ms(),
                    );
                }
                let result = db.complete_analysis_job(
                    lease,
                    &capabilities,
                    reply,
                    &observation.sha256,
                    clock.now_ms(),
                );
                // Keep the write/delete-denying Windows source and ancestor
                // handles alive until the transaction has committed.
                drop(guard);
                match result {
                    Ok(header) => Ok(match header.outcome {
                        fruitboard_storage::MetadataOutcome::Complete
                        | fruitboard_storage::MetadataOutcome::Partial => {
                            ExecutionOutcome::Complete
                        }
                        fruitboard_storage::MetadataOutcome::Unsupported => {
                            ExecutionOutcome::Unsupported
                        }
                        _ => ExecutionOutcome::Failed,
                    }),
                    Err(StorageError::Conflict) => {
                        drop(db);
                        failure(
                            database,
                            lease,
                            AnalysisFailure::SourceChanged,
                            clock.now_ms(),
                        )
                    }
                    Err(StorageError::InvalidSchema) => {
                        drop(db);
                        failure(
                            database,
                            lease,
                            AnalysisFailure::InvalidReply,
                            clock.now_ms(),
                        )
                    }
                    Err(e) => Err(e),
                }
            }
            Err(error) => failure(database, lease, error, clock.now_ms()),
        }
    }

    fn parse(
        &mut self,
        database: &Mutex<Database>,
        lease: &AnalysisLease,
        clock: &dyn AnalysisClock,
        cancellation: &CancellationToken,
    ) -> Result<
        (
            A::Guard,
            ParserCapabilities,
            ProtocolReply,
            SourceObservation,
        ),
        AnalysisFailure,
    > {
        if cancellation.is_cancelled() {
            return Err(AnalysisFailure::Interrupted);
        }
        if !lock(database)
            .map_err(|_| AnalysisFailure::Interrupted)?
            .analysis_lease_current(lease, clock.now_ms())
            .map_err(|_| AnalysisFailure::Interrupted)?
        {
            return Err(AnalysisFailure::SourceChanged);
        }
        let mut guard = self.authority.open(lease.source(), cancellation)?;
        let before = guard.observe(cancellation)?;
        let source = lease.source();
        if before.byte_size != source.byte_size()
            || before.modified_at_ns != source.modified_at_ns()
            || source
                .identity()
                .is_some_and(|id| before.identity != Some(id))
        {
            return Err(AnalysisFailure::SourceChanged);
        }
        let transport = |error: SupervisorError| {
            if error == SupervisorError::Cancelled {
                AnalysisFailure::Interrupted
            } else {
                AnalysisFailure::ParserTransport
            }
        };
        // Validate health and selected capabilities after any process restart.
        // Transport owns recycling, cancellation, deadlines and retirement.
        match self
            .parser
            .request(ParserRequest::HealthCheck, cancellation)
            .map_err(transport)?
        {
            ProtocolReply::Result(value)
                if value.as_object().is_some_and(|o| o.len() == 1) && value["status"] == "ok" => {}
            _ => return Err(AnalysisFailure::InvalidReply),
        }
        let capabilities = match self
            .parser
            .request(ParserRequest::Describe, cancellation)
            .map_err(transport)?
        {
            ProtocolReply::Result(value) => {
                validate_descriptor(&value).map_err(|_| AnalysisFailure::InvalidReply)?
            }
            _ => return Err(AnalysisFailure::InvalidReply),
        };
        let request = ParseRequest {
            path: source.path().to_owned(),
            allowed_roots: vec![source.root().to_owned()],
            expected: ExpectedFingerprint {
                size: before.byte_size,
                modified_at_ms: (before.modified_at_ns / 1_000_000) as u64,
            },
        };
        let reply = self
            .parser
            .request(ParserRequest::Parse(request), cancellation)
            .map_err(transport)?;
        let after = guard.observe(cancellation)?;
        if before != after {
            return Err(AnalysisFailure::SourceChanged);
        }
        if cancellation.is_cancelled() {
            return Err(AnalysisFailure::Interrupted);
        }
        Ok((guard, capabilities, reply, after))
    }
}

#[cfg(test)]
mod tests;
