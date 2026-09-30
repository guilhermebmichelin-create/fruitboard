//! Optional native-only parser publication. No paths, file I/O or IPC authority.
use crate::{Database, Result, StorageError};
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::{
    ParseContext, ParserCapabilities, ValidatedProjectReply, ValidationError,
    validate_project_reply,
};
use fruitboard_flp_parser::{
    ADAPTER_ID, ADAPTER_VERSION, ExpectedFingerprint, PROTOCOL_VERSION, SCHEMA_VERSION,
};
use rusqlite::{Connection, OptionalExtension, params};

mod payload;
#[cfg(test)]
mod tests;

pub const MAX_METADATA_JSON_BYTES: usize = 256 * 1024;
pub const MAX_METADATA_PAGE_SIZE: usize = 100;

/// Sealed database observation. Only capture_metadata_input can create it;
/// neither renderer nor parser fields can choose its source fences.
#[derive(Clone, PartialEq, Eq)]
pub struct MetadataInput {
    root_id: String,
    location_id: String,
    project_file_id: String,
    root_revision: i64,
    file_revision: i64,
    location_revision: i64,
    publication_revision: i64,
    byte_size: u64,
    modified_at_ns: i64,
}

impl MetadataInput {
    pub fn project_file_id(&self) -> &str {
        &self.project_file_id
    }
    pub fn location_id(&self) -> &str {
        &self.location_id
    }
    pub fn parse_context(&self) -> ParseContext {
        ParseContext {
            root_id: self.root_id.clone(),
            file_id: self.project_file_id.clone(),
            root_revision: self.root_revision as u64,
            file_revision: self.location_revision as u64,
            root_enabled: true,
            expected: ExpectedFingerprint {
                size: self.byte_size,
                modified_at_ms: (self.modified_at_ns / 1_000_000) as u64,
            },
            // The stored parser digest is a claim, not an independent native
            // hash observation, so it must never become validation authority.
            content_sha256: None,
        }
    }
}

const SELECT_INPUT: &str = "SELECT root.id, location.id, file.id,
    root.configuration_revision, file.metadata_revision, location.metadata_revision,
    file.metadata_publication_revision, location.byte_size, location.modified_at_ns
    FROM file_location AS location
    JOIN scan_root AS root ON root.id = location.scan_root_id
    JOIN project_file AS file ON file.id = location.project_file_id
    WHERE root.id = ?1 AND location.id = ?2
      AND root.enabled = 1 AND root.mode = 'local_ntfs'
      AND location.presence = 'present'
      AND location.byte_size = file.byte_size
      AND location.modified_at_ns = file.modified_at_ns
      AND location.modified_at_ns >= 0";

fn capture(connection: &Connection, root_id: &str, location_id: &str) -> Result<MetadataInput> {
    let input = connection
        .query_row(SELECT_INPUT, params![root_id, location_id], |row| {
            Ok(MetadataInput {
                root_id: row.get(0)?,
                location_id: row.get(1)?,
                project_file_id: row.get(2)?,
                root_revision: row.get(3)?,
                file_revision: row.get(4)?,
                location_revision: row.get(5)?,
                publication_revision: row.get(6)?,
                byte_size: unsigned(row, 7)?,
                modified_at_ns: row.get(8)?,
            })
        })
        .optional()?
        .ok_or(StorageError::NotFound)?;
    if input.root_revision < 0
        || input.file_revision < 0
        || input.location_revision < 0
        || input.publication_revision < 0
    {
        return Err(StorageError::InvalidSchema);
    }
    Ok(input)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataOutcome {
    Complete,
    Partial,
    Unsupported,
    Failed,
    Rejected,
}
impl MetadataOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
            Self::Rejected => "rejected",
        }
    }
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "complete" => Self::Complete,
            "partial" => Self::Partial,
            "unsupported" => Self::Unsupported,
            "failed" => Self::Failed,
            "rejected" => Self::Rejected,
            _ => return Err(StorageError::InvalidSchema),
        })
    }
}

/// Native read model. No automatic Debug/Serialize because saved metadata can
/// contain private names/locators. A future UI needs an explicit typed adapter.
pub struct MetadataSnapshot {
    header: MetadataSnapshotHeader,
    payload_json: Option<String>,
}
impl MetadataSnapshot {
    pub fn header(&self) -> &MetadataSnapshotHeader {
        &self.header
    }
    /// Versioned, allowlisted local projection, never the raw parser reply.
    /// This opaque persisted text is not a new validated parser capability.
    pub fn payload_json(&self) -> Option<&str> {
        self.payload_json.as_deref()
    }
}

#[derive(Clone)]
pub struct MetadataSnapshotHeader {
    pub id: String,
    pub project_file_id: String,
    pub input_root_id: String,
    pub input_location_id: String,
    pub input_root_revision: i64,
    pub input_file_revision: i64,
    pub input_location_revision: i64,
    pub input_byte_size: u64,
    pub input_modified_at_ns: i64,
    pub input_content_sha256: Option<String>,
    pub adapter_id: String,
    pub adapter_version: String,
    pub protocol_version: u64,
    pub parser_schema_version: u64,
    pub projection_version: u64,
    pub outcome: MetadataOutcome,
    pub parser_code: Option<String>,
    pub unsupported_saved_version: Option<String>,
    pub parsed_at_ms: i64,
}

#[derive(Clone)]
pub struct MetadataCursor {
    project_file_id: String,
    parsed_at_ms: i64,
    id: String,
}
pub struct MetadataPage {
    pub items: Vec<MetadataSnapshotHeader>,
    pub next: Option<MetadataCursor>,
}

const HEADER_COLUMNS: &str = "snapshot.id, snapshot.project_file_id, snapshot.input_root_id,
    snapshot.input_location_id, snapshot.input_root_revision, snapshot.input_file_revision,
    snapshot.input_location_revision, snapshot.input_byte_size, snapshot.input_modified_at_ns,
    snapshot.input_content_sha256, snapshot.adapter_id, snapshot.adapter_version,
    snapshot.protocol_version, snapshot.parser_schema_version, snapshot.projection_version,
    snapshot.outcome, snapshot.parser_code, snapshot.unsupported_saved_version, snapshot.parsed_at_ms";

fn unsigned(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
}

fn read_header(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetadataSnapshotHeader> {
    let outcome: String = row.get(15)?;
    let outcome = MetadataOutcome::parse(&outcome).map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(MetadataSnapshotHeader {
        id: row.get(0)?,
        project_file_id: row.get(1)?,
        input_root_id: row.get(2)?,
        input_location_id: row.get(3)?,
        input_root_revision: row.get(4)?,
        input_file_revision: row.get(5)?,
        input_location_revision: row.get(6)?,
        input_byte_size: unsigned(row, 7)?,
        input_modified_at_ns: row.get(8)?,
        input_content_sha256: row.get(9)?,
        adapter_id: row.get(10)?,
        adapter_version: row.get(11)?,
        protocol_version: unsigned(row, 12)?,
        parser_schema_version: unsigned(row, 13)?,
        projection_version: unsigned(row, 14)?,
        outcome,
        parser_code: row.get(16)?,
        unsupported_saved_version: row.get(17)?,
        parsed_at_ms: row.get(18)?,
    })
}

impl Database {
    pub fn capture_metadata_input(
        &self,
        root_id: &str,
        location_id: &str,
    ) -> Result<MetadataInput> {
        capture(&self.connection, root_id, location_id)
    }

    /// Validate and publish under the same IMMEDIATE transaction as freshness
    /// checks and the latest pointer. Stale/invalid replies produce no rows.
    /// The future worker must also re-observe the authorized source handle:
    /// database observations alone cannot prove the filesystem has not changed.
    pub fn publish_metadata_snapshot(
        &mut self,
        input: &MetadataInput,
        capabilities: &ParserCapabilities,
        reply: ProtocolReply,
        parsed_at_ms: i64,
    ) -> Result<MetadataSnapshotHeader> {
        self.publish_metadata_observed(input, capabilities, reply, parsed_at_ms, || {})
    }

    fn publish_metadata_observed(
        &mut self,
        input: &MetadataInput,
        capabilities: &ParserCapabilities,
        reply: ProtocolReply,
        parsed_at_ms: i64,
        before_commit: impl FnOnce(),
    ) -> Result<MetadataSnapshotHeader> {
        if parsed_at_ms < 0 {
            return Err(StorageError::InvalidSchema);
        }
        if let ProtocolReply::Result(value) = &reply {
            payload::check_reply_size(value)?;
        }
        self.transaction(|transaction| {
            let current = capture(transaction, &input.root_id, &input.location_id).map_err(|error| {
                if error == StorageError::NotFound { StorageError::Conflict } else { error }
            })?;
            if &current != input { return Err(StorageError::Conflict); }
            let validated = validate_project_reply(reply, capabilities, &input.parse_context(), &current.parse_context())
                .map_err(|error| if error == ValidationError::StaleInput { StorageError::Conflict } else { StorageError::InvalidSchema })?;
            let (outcome, parser_code, unsupported_version, content_hash, payload_json) = match &validated {
                ValidatedProjectReply::Metadata(metadata) => {
                    let outcome = match metadata.initial().outcome() {
                        fruitboard_flp_parser::validation::MetadataOutcome::Complete => MetadataOutcome::Complete,
                        fruitboard_flp_parser::validation::MetadataOutcome::Partial => MetadataOutcome::Partial,
                    };
                    (outcome, None, None, Some(metadata.initial().content_sha256()), Some(payload::encode(metadata)?))
                }
                ValidatedProjectReply::Failed(code) => (MetadataOutcome::Failed, Some(code.as_str()), None, None, None),
                ValidatedProjectReply::Rejected(code) => (MetadataOutcome::Rejected, Some(code.as_str()), None, None, None),
                ValidatedProjectReply::UnsupportedSavedVersion(version) => (MetadataOutcome::Unsupported, None, Some(version.as_str()), None, None),
            };
            let id = uuid::Uuid::now_v7().to_string();
            transaction.execute("INSERT INTO metadata_snapshot
                (id, project_file_id, input_root_id, input_location_id, input_root_revision,
                 input_file_revision, input_location_revision, input_byte_size, input_modified_at_ns,
                 input_content_sha256, adapter_id, adapter_version, protocol_version, parser_schema_version,
                 projection_version, outcome, parser_code, unsupported_saved_version, payload_json, parsed_at_ms)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 1, ?15, ?16, ?17, ?18, ?19)",
                params![id, input.project_file_id, input.root_id, input.location_id, input.root_revision,
                    input.file_revision, input.location_revision, i64::try_from(input.byte_size).map_err(|_| StorageError::InvalidSchema)?, input.modified_at_ns,
                    content_hash, ADAPTER_ID, ADAPTER_VERSION, i64::try_from(PROTOCOL_VERSION).map_err(|_| StorageError::InvalidSchema)?, i64::try_from(SCHEMA_VERSION).map_err(|_| StorageError::InvalidSchema)?,
                    outcome.as_str(), parser_code, unsupported_version, payload_json, parsed_at_ms])?;
            let next = input.publication_revision.checked_add(1).ok_or(StorageError::InvalidSchema)?;
            let changed = transaction.execute("UPDATE project_file
                SET current_metadata_snapshot_id = ?1, metadata_publication_revision = ?2
                WHERE id = ?3 AND metadata_publication_revision = ?4", params![id, next, input.project_file_id, input.publication_revision])?;
            if changed != 1 { return Err(StorageError::Conflict); }
            before_commit();
            transaction.query_row(&format!("SELECT {HEADER_COLUMNS} FROM metadata_snapshot AS snapshot WHERE snapshot.id = ?1"), [&id], read_header).map_err(Into::into)
        })
    }

    pub fn metadata_snapshot(&self, id: &str) -> Result<Option<MetadataSnapshot>> {
        let selected = self.connection.query_row(
            &format!("SELECT {HEADER_COLUMNS}, CASE WHEN length(CAST(snapshot.payload_json AS BLOB)) <= {MAX_METADATA_JSON_BYTES} THEN snapshot.payload_json ELSE NULL END FROM metadata_snapshot AS snapshot WHERE snapshot.id = ?1"),
            [id], |row| Ok(MetadataSnapshot { header: read_header(row)?, payload_json: row.get(19)? }),
        ).optional()?;
        if let Some(snapshot) = &selected {
            if snapshot.header.projection_version != 1 {
                return Err(StorageError::InvalidSchema);
            }
            let expects_payload = matches!(
                snapshot.header.outcome,
                MetadataOutcome::Complete | MetadataOutcome::Partial
            );
            if expects_payload != snapshot.payload_json.is_some() {
                return Err(StorageError::InvalidSchema);
            }
            if let Some(payload) = &snapshot.payload_json {
                let value: serde_json::Value =
                    serde_json::from_str(payload).map_err(|_| StorageError::InvalidSchema)?;
                if value["projectionVersion"] != 1
                    || value["contentSha256"].as_str()
                        != snapshot.header.input_content_sha256.as_deref()
                {
                    return Err(StorageError::InvalidSchema);
                }
            }
        }
        Ok(selected)
    }

    /// Missing/changed/detached/disabled sources retain history but cannot
    /// present an earlier snapshot as current, even after values are restored.
    pub fn current_metadata_snapshot(
        &self,
        project_file_id: &str,
    ) -> Result<Option<MetadataSnapshot>> {
        let id: Option<String> = self.connection.query_row("SELECT snapshot.id
            FROM project_file AS file
            JOIN metadata_snapshot AS snapshot ON snapshot.id = file.current_metadata_snapshot_id
            JOIN file_location AS location ON location.id = snapshot.input_location_id
            JOIN scan_root AS root ON root.id = snapshot.input_root_id
            WHERE file.id = ?1 AND snapshot.project_file_id = file.id
              AND location.project_file_id = file.id AND location.scan_root_id = root.id
              AND root.enabled = 1 AND root.mode = 'local_ntfs' AND location.presence = 'present'
              AND root.configuration_revision = snapshot.input_root_revision
              AND file.metadata_revision = snapshot.input_file_revision
              AND location.metadata_revision = snapshot.input_location_revision
              AND file.byte_size = snapshot.input_byte_size AND location.byte_size = snapshot.input_byte_size
              AND file.modified_at_ns = snapshot.input_modified_at_ns AND location.modified_at_ns = snapshot.input_modified_at_ns", [project_file_id], |row| row.get(0)).optional()?;
        id.map(|id| self.metadata_snapshot(&id))
            .transpose()
            .map(Option::flatten)
    }

    /// Bounded indexed header pagination; payloads are loaded individually.
    pub fn metadata_snapshot_page(
        &self,
        project_file_id: &str,
        after: Option<&MetadataCursor>,
        limit: usize,
    ) -> Result<MetadataPage> {
        if !(1..=MAX_METADATA_PAGE_SIZE).contains(&limit) {
            return Err(StorageError::InvalidCursor);
        }
        if after.is_some_and(|cursor| cursor.project_file_id != project_file_id) {
            return Err(StorageError::InvalidCursor);
        }
        let (time, id) = after.map_or((i64::MAX, "\u{10ffff}"), |cursor| {
            (cursor.parsed_at_ms, cursor.id.as_str())
        });
        let mut statement = self.connection.prepare(&format!("SELECT {HEADER_COLUMNS} FROM metadata_snapshot AS snapshot
            WHERE snapshot.project_file_id = ?1 AND (snapshot.parsed_at_ms, snapshot.id COLLATE BINARY) < (?2, ?3)
            ORDER BY snapshot.parsed_at_ms DESC, snapshot.id COLLATE BINARY DESC LIMIT ?4"))?;
        let mut items = statement
            .query_map(
                params![project_file_id, time, id, (limit + 1) as i64],
                read_header,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next = if has_more {
            items.last().map(|last| MetadataCursor {
                project_file_id: project_file_id.to_owned(),
                parsed_at_ms: last.parsed_at_ms,
                id: last.id.clone(),
            })
        } else {
            None
        };
        Ok(MetadataPage { items, next })
    }
}
