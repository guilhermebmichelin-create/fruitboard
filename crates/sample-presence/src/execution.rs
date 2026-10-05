use crate::{
    AbsolutePath, Context, MAX_CHANNELS, MAX_HANDLES, MAX_OPERATIONS, MAX_REFERENCE_BYTES,
    MAX_REFERENCE_TOTAL_BYTES, MAX_REFERENCE_UTF16, Outcome, Report, RequestControl,
    RequestFailure, UncheckedReason, WorkerLease,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// Preserve a bounded final authority window inside the total operation cap.
// This is a resource reservation, not permission to operate after cancellation.
const FINAL_OPERATIONS: usize = 256;

#[derive(Clone, Eq, PartialEq)]
pub struct ObjectIdentity {
    pub volume: u64,
    pub file: u128,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SourceFingerprint {
    pub identity: ObjectIdentity,
    pub byte_size: u64,
    pub modified_at_ns: i64,
}

/// Captured under the future host's durable authorization guard. No Debug.
pub struct AuthorityFence {
    pub root_revision: u64,
    pub location_revision: u64,
    pub root_identity: ObjectIdentity,
    pub source: SourceFingerprint,
}

/// Trusted-host input, not deserializable from an IPC request. Bounded before I/O.
/// Construction validates shape only; the injected port must qualify authority.
pub struct CapturedInput {
    context: Context,
    root: AbsolutePath,
    source: AbsolutePath,
    fence: AuthorityFence,
    references: Vec<Option<String>>,
}

impl CapturedInput {
    pub fn new(
        context: Context,
        root: &str,
        source: &str,
        fence: AuthorityFence,
        references: Option<Vec<Option<String>>>,
    ) -> Result<Self, RequestFailure> {
        let references = references.ok_or(RequestFailure::ReferencesUnavailable)?;
        if references.len() > MAX_CHANNELS {
            return Err(RequestFailure::InvalidInput);
        }
        let mut total = 0usize;
        for value in references.iter().flatten() {
            if value.is_empty()
                || value.len() > MAX_REFERENCE_BYTES
                || value.encode_utf16().count() > MAX_REFERENCE_UTF16
                || value.contains('\0')
            {
                return Err(RequestFailure::InvalidInput);
            }
            total = total
                .checked_add(value.len())
                .ok_or(RequestFailure::InvalidInput)?;
            if total > MAX_REFERENCE_TOTAL_BYTES {
                return Err(RequestFailure::InvalidInput);
            }
        }
        let root = AbsolutePath::parse(root).map_err(RequestFailure::Unavailable)?;
        let source = AbsolutePath::parse(source).map_err(RequestFailure::Unavailable)?;
        Ok(Self {
            context,
            root,
            source,
            fence,
            references,
        })
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn root(&self) -> &AbsolutePath {
        &self.root
    }
    pub fn source(&self) -> &AbsolutePath {
        &self.source
    }
    pub fn fence(&self) -> &AuthorityFence {
        &self.fence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortError {
    Unchecked(UncheckedReason),
    Request(RequestFailure),
}

impl From<RequestFailure> for PortError {
    fn from(value: RequestFailure) -> Self {
        Self::Request(value)
    }
}

/// Every live native handle owns one token, including qualification/revalidation
/// temporaries. Non-cloneable; guards must drop handles before their tokens.
pub struct HandleLease(Arc<AtomicUsize>);
impl Drop for HandleLease {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// All adapter syscalls must pass through `perform`, one syscall per closure.
/// Native implementations are trusted and separately reviewed for this contract.
pub struct Operations {
    control: RequestControl,
    used: usize,
    limit: usize,
    handles: Arc<AtomicUsize>,
}

impl Operations {
    fn new(control: RequestControl) -> Self {
        Self {
            control,
            used: 0,
            limit: MAX_OPERATIONS - FINAL_OPERATIONS,
            handles: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub fn perform<T>(
        &mut self,
        operation: impl FnOnce() -> Result<T, PortError>,
    ) -> Result<T, PortError> {
        self.control.checkpoint()?;
        if self.used >= self.limit {
            return Err(PortError::Unchecked(UncheckedReason::LimitReached));
        }
        self.used += 1;
        let result = operation();
        // Revocation/timeout beats every provisional observation or OS error.
        self.control.checkpoint()?;
        result
    }
    pub fn acquire_handle(&self) -> Result<HandleLease, PortError> {
        self.control.checkpoint()?;
        self.handles
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                (n < MAX_HANDLES).then_some(n + 1)
            })
            .map_err(|_| PortError::Unchecked(UncheckedReason::LimitReached))?;
        Ok(HandleLease(self.handles.clone()))
    }
    pub fn used(&self) -> usize {
        self.used
    }
    pub fn live_handles(&self) -> usize {
        self.handles.load(Ordering::SeqCst)
    }
}

/// Owned held-object guards; source/root namespace bindings belong to authority.
pub struct Qualified<A, D> {
    pub authority: A,
    pub root_directory: D,
}

/// Only qualified, exact-child observations from a held trusted parent.
pub enum Child<D> {
    Directory(D),
    RegularFile,
    Absent,
}

/// Trusted injection seam, never selected by a renderer. No enumeration, content,
/// process, network fallback or mutation methods. No production adapter yet.
pub trait MetadataPort {
    type Authority;
    type Directory;

    /// Qualify enabled current local-NTFS root/source identity and namespace
    /// bindings against the capture, and hold authority. Reject reparses/offline/
    /// unknown attributes before traversal. No write lock on the source.
    fn qualify(
        &mut self,
        input: &CapturedInput,
        operations: &mut Operations,
    ) -> Result<Qualified<Self::Authority, Self::Directory>, PortError>;

    /// Compare the candidate prefix to the corresponding held root component,
    /// using qualified filesystem case mode. No candidate lookup or guessed
    /// Unicode folding. Query case metadata through operations if needed.
    fn root_component_matches(
        &mut self,
        authority: &Self::Authority,
        index: usize,
        saved_component: &str,
        operations: &mut Operations,
    ) -> Result<bool, PortError>;

    /// Exact next child relative to the trusted parent. Directory guards retain
    /// ancestry. Do not follow reparses/hydrate; refuse uncertain operations.
    /// Absent attests authoritative leaf/intermediate absence, never a query
    /// error. Count each query/open, including temporaries, and all live handles.
    fn child(
        &mut self,
        authority: &Self::Authority,
        parent: &Self::Directory,
        name: &str,
        operations: &mut Operations,
    ) -> Result<Child<Self::Directory>, PortError>;

    /// Match held metadata AND trusted parent/name bindings, durable revisions,
    /// current snapshot/session. DB guard is not held during filesystem calls.
    /// Discard every result on detectable replacement/revocation. At most 256
    /// reserved operations; all calls still honor cancellation/deadline.
    fn revalidate(
        &mut self,
        input: &CapturedInput,
        authority: &Self::Authority,
        operations: &mut Operations,
    ) -> Result<(), PortError>;
}

fn request_error(error: PortError) -> RequestFailure {
    match error {
        PortError::Unchecked(reason) => RequestFailure::Unavailable(reason),
        PortError::Request(error) => error,
    }
}

fn probe<P: MetadataPort>(
    input: &CapturedInput,
    qualified: &Qualified<P::Authority, P::Directory>,
    path: &AbsolutePath,
    port: &mut P,
    operations: &mut Operations,
) -> Result<Outcome, PortError> {
    let root = input.root();
    if path.drive() != root.drive() || path.components().len() < root.components().len() {
        return Ok(Outcome::NotChecked {
            reason: UncheckedReason::OutsideRoot,
        });
    }
    for (index, component) in path
        .components()
        .iter()
        .take(root.components().len())
        .enumerate()
    {
        operations.control.checkpoint()?;
        if !port.root_component_matches(&qualified.authority, index, component, operations)? {
            return Ok(Outcome::NotChecked {
                reason: UncheckedReason::OutsideRoot,
            });
        }
        operations.control.checkpoint()?;
    }
    let suffix = &path.components()[root.components().len()..];
    if suffix.is_empty() {
        return Ok(Outcome::NotChecked {
            reason: UncheckedReason::NotRegularFile,
        });
    }
    let mut chain = Vec::new();
    for (index, name) in suffix.iter().enumerate() {
        operations.control.checkpoint()?;
        let parent = chain.last().unwrap_or(&qualified.root_directory);
        let child = port.child(&qualified.authority, parent, name, operations)?;
        operations.control.checkpoint()?;
        match child {
            Child::Absent => return Ok(Outcome::NotFound),
            Child::RegularFile if index + 1 == suffix.len() => return Ok(Outcome::Present),
            Child::RegularFile => {
                return Ok(Outcome::NotChecked {
                    reason: UncheckedReason::NotRegularFile,
                });
            }
            Child::Directory(_) if index + 1 == suffix.len() => {
                return Ok(Outcome::NotChecked {
                    reason: UncheckedReason::NotRegularFile,
                });
            }
            Child::Directory(directory) => chain.push(directory),
        }
    }
    unreachable!("nonempty suffix yields an observation")
}

/// Sole-worker execution. Timestamp is a bounded host UTC observation; deadlines
/// use the independently injected monotonic clock. Results remain provisional
/// until revalidation and publication. Host/renderer must additionally fence
/// context/generation at transport delivery in slice 3.
pub fn check<P: MetadataPort>(
    input: &CapturedInput,
    lease: &mut WorkerLease,
    port: &mut P,
    checked_at_unix_ms: u64,
) -> Result<Report, RequestFailure> {
    lease.begin()?;
    // Validate timestamp before admitting any filesystem observation.
    if checked_at_unix_ms > 253_402_300_799_999 {
        return Err(RequestFailure::InvalidInput);
    }
    let mut operations = Operations::new(lease.control());
    operations.control.checkpoint()?;
    let qualified = port
        .qualify(input, &mut operations)
        .map_err(request_error)?;
    operations.control.checkpoint()?;
    let mut outcomes = Vec::with_capacity(input.references.len());
    let mut memo = BTreeMap::new();
    let mut exhausted = false;
    for reference in &input.references {
        operations.control.checkpoint()?;
        let outcome = match reference {
            None => Outcome::NoSavedReference,
            Some(_) if exhausted => Outcome::NotChecked {
                reason: UncheckedReason::LimitReached,
            },
            Some(value) => {
                if let Some(outcome) = memo.get(value) {
                    *outcome
                } else {
                    let result = AbsolutePath::parse(value)
                        .map_err(PortError::Unchecked)
                        .and_then(|path| probe(input, &qualified, &path, port, &mut operations));
                    let outcome = match result {
                        Ok(outcome) => outcome,
                        Err(PortError::Unchecked(reason)) => {
                            if reason == UncheckedReason::LimitReached
                                && operations.used >= operations.limit
                            {
                                exhausted = true;
                            }
                            Outcome::NotChecked { reason }
                        }
                        Err(PortError::Request(error)) => return Err(error),
                    };
                    memo.insert(value.clone(), outcome);
                    outcome
                }
            }
        };
        outcomes.push(outcome);
    }
    operations.limit = MAX_OPERATIONS;
    operations.control.checkpoint()?;
    port.revalidate(input, &qualified.authority, &mut operations)
        .map_err(request_error)?;
    operations.control.checkpoint()?;
    let report = Report::new(input.context.clone(), checked_at_unix_ms, outcomes)?;
    // Prove encoded bounds before accepting publication, even for 256 slots.
    report.to_json()?;
    lease.publish()?;
    Ok(report)
}
