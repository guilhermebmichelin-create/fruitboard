//! Relative-path and notification-action types for the watcher boundary.
//!
//! OS notification payloads are reduced to validated root-relative path bytes
//! plus an action code before they can enter any public type. Absolute-shaped,
//! traversal-shaped, or otherwise invalid path bytes are never surfaced: the
//! record is counted as obscured activity and dropped. No public type can
//! carry an absolute path.

use std::fmt;

/// Windows `FILE_ACTION_*` notification action codes (`winnt.h`). Kept local
/// so the pure parser below stays cross-platform and dependency-free.
pub(crate) const FILE_ACTION_ADDED: u32 = 0x0000_0001;
pub(crate) const FILE_ACTION_REMOVED: u32 = 0x0000_0002;
pub(crate) const FILE_ACTION_MODIFIED: u32 = 0x0000_0003;
pub(crate) const FILE_ACTION_RENAMED_OLD_NAME: u32 = 0x0000_0004;
pub(crate) const FILE_ACTION_RENAMED_NEW_NAME: u32 = 0x0000_0005;

/// Maximum validated relative-path byte length. Matches the reconciliation
/// core's per-path bound so hints can never exceed downstream limits.
pub const MAX_RELATIVE_PATH_BYTES: usize = 32_768;

/// Why a root-relative path was rejected. Never echoes the rejected bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativePathRejected {
    Empty,
    TooLong,
    NotUtf8,
    RootedOrAbsolute,
    ColonOrNul,
    DotComponent,
}

/// A validated, root-relative notification path.
///
/// The bytes are the root-relative name as reported by the OS boundary,
/// converted to UTF-8 (invalid surrogates become U+FFFD, never a panic).
/// Validation rejects anything that could read as absolute or escape the
/// root, so no public type in this crate can carry an absolute path. The
/// [`fmt::Debug`] implementation redacts the bytes: hints must never leak
/// user-visible names into logs.
#[derive(Clone, PartialEq, Eq)]
pub struct RelativePath {
    bytes: Vec<u8>,
}

impl RelativePath {
    /// Validates and wraps root-relative path bytes. Mirrors the
    /// reconciliation core's defensive rejection rules.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, RelativePathRejected> {
        if bytes.is_empty() {
            return Err(RelativePathRejected::Empty);
        }
        if bytes.len() > MAX_RELATIVE_PATH_BYTES {
            return Err(RelativePathRejected::TooLong);
        }
        if matches!(bytes.first(), Some(b'/' | b'\\')) {
            return Err(RelativePathRejected::RootedOrAbsolute);
        }
        if bytes.contains(&b':') || bytes.contains(&0) {
            return Err(RelativePathRejected::ColonOrNul);
        }
        if std::str::from_utf8(&bytes).is_err() {
            return Err(RelativePathRejected::NotUtf8);
        }
        if bytes
            .split(|&byte| byte == b'/' || byte == b'\\')
            .any(|part| matches!(part, b"" | b"." | b".."))
        {
            return Err(RelativePathRejected::DotComponent);
        }
        Ok(Self { bytes })
    }

    /// The validated root-relative path bytes. Opaque by contract: consumers
    /// must not interpret them; they are hints, not state.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for RelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RelativePath({} bytes)", self.bytes.len())
    }
}

/// A Windows notification action code, mapped from `FILE_ACTION_*`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotifyAction {
    Added,
    Removed,
    Modified,
    RenamedOldName,
    RenamedNewName,
}

impl NotifyAction {
    pub(crate) fn from_code(code: u32) -> Option<Self> {
        match code {
            FILE_ACTION_ADDED => Some(Self::Added),
            FILE_ACTION_REMOVED => Some(Self::Removed),
            FILE_ACTION_MODIFIED => Some(Self::Modified),
            FILE_ACTION_RENAMED_OLD_NAME => Some(Self::RenamedOldName),
            FILE_ACTION_RENAMED_NEW_NAME => Some(Self::RenamedNewName),
            _ => None,
        }
    }
}

/// One raw watcher observation: only relative-path bytes plus an action code.
/// `Obscured` records activity that could not be surfaced (unknown action
/// code or invalid path bytes); it never carries path bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawEvent {
    Change {
        action: NotifyAction,
        relative_path: RelativePath,
    },
    Obscured,
}

/// Result of parsing one ReadDirectoryChangesW result buffer.
pub(crate) struct ParsedBatch {
    /// One queued item per visited record; every record becomes activity.
    pub events: Vec<RawEvent>,
    /// Records surfaced as [`RawEvent::Obscured`].
    pub obscured: u64,
    /// The chain was malformed or overran the buffer; some records may not
    /// have been visited, so coverage is uncertain.
    pub truncated: bool,
}

/// Parses a ReadDirectoryChangesW `FILE_NOTIFY_INFORMATION` chain (DWORD-
/// chained records) defensively. Malformed offsets stop the walk and mark the
/// batch truncated rather than trusting unverified lengths.
pub(crate) fn parse_notify_buffer(buffer: &[u8]) -> ParsedBatch {
    if buffer.is_empty() {
        return ParsedBatch {
            events: Vec::new(),
            obscured: 0,
            truncated: false,
        };
    }
    let mut events = Vec::new();
    let mut obscured = 0;
    let mut truncated = false;
    let mut offset = 0usize;
    loop {
        if buffer.len() - offset < 12 {
            truncated = true;
            break;
        }
        let next_entry_offset =
            u32::from_le_bytes(buffer[offset..offset + 4].try_into().expect("4-byte slice"));
        let action = u32::from_le_bytes(
            buffer[offset + 4..offset + 8]
                .try_into()
                .expect("4-byte slice"),
        );
        let name_length = u32::from_le_bytes(
            buffer[offset + 8..offset + 12]
                .try_into()
                .expect("4-byte slice"),
        ) as usize;
        let name_start = offset + 12;
        let Some(name_end) = name_start.checked_add(name_length) else {
            truncated = true;
            break;
        };
        if name_end > buffer.len() {
            truncated = true;
            break;
        }
        match parse_record(action, &buffer[name_start..name_end]) {
            Some(event) => events.push(event),
            None => {
                obscured += 1;
                events.push(RawEvent::Obscured);
            }
        }

        if next_entry_offset == 0 {
            break;
        }
        let step = next_entry_offset as usize;
        if step < 12
            || !step.is_multiple_of(4)
            || offset
                .checked_add(step)
                .is_none_or(|end| end > buffer.len())
        {
            truncated = true;
            break;
        }
        offset += step;
    }
    ParsedBatch {
        events,
        obscured,
        truncated,
    }
}

fn parse_record(action: u32, name_bytes: &[u8]) -> Option<RawEvent> {
    let (pairs, remainder) = name_bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    let action = NotifyAction::from_code(action)?;
    // Lossy conversion keeps the pipeline panic-free; unpaired surrogates
    // become U+FFFD. These bytes are hints, never state.
    let text = String::from_utf16_lossy(
        &pairs
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect::<Vec<u16>>(),
    );
    match RelativePath::from_bytes(text.into_bytes()) {
        Ok(relative_path) => Some(RawEvent::Change {
            action,
            relative_path,
        }),
        Err(_) => None,
    }
}
