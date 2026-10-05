use crate::{MAX_CHANNELS, MAX_REPORT_BYTES};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UncheckedReason {
    OutsideRoot,
    UnsupportedPathSyntax,
    RelativeReference,
    UnresolvedPlaceholder,
    UnqualifiedFilesystem,
    UnsupportedCaseMode,
    ReparseOrOffline,
    AccessDenied,
    NotRegularFile,
    LimitReached,
    Deadline,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestFailure {
    InvalidInput,
    ReferencesUnavailable,
    Unavailable(UncheckedReason),
    Stale,
    Cancelled,
    Deadline,
    WorkerRetired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Present,
    NotFound,
    NotChecked { reason: UncheckedReason },
    NoSavedReference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ChannelResult {
    pub position: usize,
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// Native-captured correlation, not a renderer command or access grant.
/// No Debug: even opaque identifiers must not be interpolated into logs.
#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct Context {
    request_id: String,
    root_id: String,
    location_id: String,
    snapshot_id: String,
    session_id: String,
}

impl Context {
    pub fn new(
        request: &str,
        root: &str,
        location: &str,
        snapshot: &str,
        session: &str,
    ) -> Result<Self, RequestFailure> {
        for id in [request, root, location, snapshot, session] {
            if id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            {
                return Err(RequestFailure::InvalidInput);
            }
        }
        Ok(Self {
            request_id: request.into(),
            root_id: root.into(),
            location_id: location.into(),
            snapshot_id: snapshot.into(),
            session_id: session.into(),
        })
    }
}

/// Ephemeral, ordered, path-free output. Created only after the final fence.
#[derive(Serialize)]
pub struct Report {
    version: u8,
    context: Context,
    checked_at_unix_ms: u64,
    channels: Vec<ChannelResult>,
}

impl Report {
    pub(crate) fn new(
        context: Context,
        checked_at_unix_ms: u64,
        outcomes: Vec<Outcome>,
    ) -> Result<Self, RequestFailure> {
        if checked_at_unix_ms > 253_402_300_799_999 || outcomes.len() > MAX_CHANNELS {
            return Err(RequestFailure::InvalidInput);
        }
        Ok(Self {
            version: 1,
            context,
            checked_at_unix_ms,
            channels: outcomes
                .into_iter()
                .enumerate()
                .map(|(i, outcome)| ChannelResult {
                    position: i + 1,
                    outcome,
                })
                .collect(),
        })
    }
    pub fn context(&self) -> &Context {
        &self.context
    }
    pub fn channels(&self) -> &[ChannelResult] {
        &self.channels
    }
    pub fn to_json(&self) -> Result<Vec<u8>, RequestFailure> {
        let encoded = serde_json::to_vec(self).map_err(|_| RequestFailure::InvalidInput)?;
        if encoded.len() > MAX_REPORT_BYTES {
            return Err(RequestFailure::Unavailable(UncheckedReason::LimitReached));
        }
        Ok(encoded)
    }
}
