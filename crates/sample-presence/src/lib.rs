//! ADR-007 policy foundation. No filesystem adapter or application capability.
//!
//! Stored paths are untrusted. Only an injected, reviewed metadata port can
//! qualify objects, compare filesystem names and attest exact-name absence.
//! This crate never reads content, enumerates directories or mutates files.
#![forbid(unsafe_code)]

mod execution;
mod lifecycle;
mod path;
mod report;

pub use execution::{
    AuthorityFence, CapturedInput, Child, HandleLease, MetadataPort, ObjectIdentity, Operations,
    PortError, Qualified, SourceFingerprint, check,
};
pub use lifecycle::{AdmissionError, MonotonicClock, RequestControl, WorkerGate, WorkerLease};
pub use path::AbsolutePath;
pub use report::{ChannelResult, Context, Outcome, Report, RequestFailure, UncheckedReason};

pub const MAX_CHANNELS: usize = 256;
pub const MAX_REFERENCE_UTF16: usize = 4095;
pub const MAX_REFERENCE_BYTES: usize = 12285;
pub const MAX_REFERENCE_TOTAL_BYTES: usize = 256 * 1024;
pub const MAX_COMPONENTS: usize = 64;
pub const MAX_OPERATIONS: usize = 2048;
pub const MAX_HANDLES: usize = 132;
pub const MAX_REPORT_BYTES: usize = 64 * 1024;
pub const DEADLINE_MS: u64 = 2000;
