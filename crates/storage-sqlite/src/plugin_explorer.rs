//! Bounded root-scoped saved metadata read. No filesystem or parser authority.
use crate::metadata::{
    CURRENT_SNAPSHOT_FROM, HEADER_COLUMNS, bounded_payload_column, read_snapshot,
};
use crate::publication::{LOCATION_COLUMNS, published_location_from_row};
use crate::{
    Database, FilePresence, MetadataSnapshot, PublishedLocation, Result, ScanRoot, ScanRootMode,
    StorageError,
};

pub const MAX_EXPLORER_ENTRIES: usize = 2_000;
pub const MAX_EXPLORER_INPUT_BYTES: usize = 8 * 1024 * 1024;

// Private saved values intentionally have no Debug or Serialize implementation.
pub enum ExplorerRead {
    Unavailable,
    Limited,
    Ready {
        root: ScanRoot,
        entries: Vec<ExplorerEntry>,
    },
}
pub struct ExplorerEntry {
    pub location: PublishedLocation,
    pub metadata: ExplorerMetadata,
}
pub enum ExplorerMetadata {
    Current(Box<MetadataSnapshot>),
    Missing,
    Stale,
    NoAnalysis,
}

impl Database {
    /// Read one consistent database snapshot. No partial result is returned
    /// when the entry or accumulated metadata-input budget is exceeded.
    pub fn read_plugin_explorer(&self, root_id: &str) -> Result<ExplorerRead> {
        if root_id.is_empty() || root_id.len() > 128 || root_id.contains('\0') {
            return Err(StorageError::InvalidSchema);
        }
        let tx = self.connection.unchecked_transaction()?;
        let root = Self::select_scan_root(&tx, root_id)?;
        if !root.enabled || root.mode != ScanRootMode::LocalNtfs {
            return Ok(ExplorerRead::Unavailable);
        }
        let locations = {
            let mut statement = tx.prepare(&format!(
                "{LOCATION_COLUMNS} WHERE scan_root_id = ?1
                 ORDER BY locator_key COLLATE BINARY, id LIMIT ?2"
            ))?;
            statement
                .query_map(
                    rusqlite::params![root_id, (MAX_EXPLORER_ENTRIES + 1) as i64],
                    published_location_from_row,
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        if locations.len() > MAX_EXPLORER_ENTRIES {
            return Ok(ExplorerRead::Limited);
        }
        let entries = {
            let payload = bounded_payload_column();
            // The scalar lookup uses the same origin fences as single-project
            // reads. Starting from a bounded, ordered root lets SQLite resolve
            // all present locations without per-location client queries.
            let mut statement = tx.prepare(&format!(
                "SELECT {HEADER_COLUMNS}, {payload},
                    requested.id AS requested_id, candidate.id IS NOT NULL AS has_candidate,
                    snapshot.id IS NOT NULL AS has_snapshot,
                    requested_root.configuration_revision AS root_revision,
                    requested.metadata_revision AS location_revision,
                    candidate.metadata_revision AS file_revision,
                    candidate.metadata_publication_revision AS publication_revision
                 FROM file_location AS requested
                 JOIN scan_root AS requested_root ON requested_root.id = requested.scan_root_id
                 LEFT JOIN project_file AS candidate ON candidate.id = requested.project_file_id
                    AND requested.presence = 'present'
                    AND requested.byte_size = candidate.byte_size
                    AND requested.modified_at_ns = candidate.modified_at_ns
                    AND requested.modified_at_ns >= 0
                 LEFT JOIN metadata_snapshot AS snapshot ON snapshot.id = (
                    SELECT snapshot.id {CURRENT_SNAPSHOT_FROM} AND file.id = candidate.id
                 )
                 WHERE requested.scan_root_id = ?1
                 ORDER BY requested.locator_key COLLATE BINARY, requested.id LIMIT ?2"
            ))?;
            let mut rows =
                statement.query(rusqlite::params![root_id, MAX_EXPLORER_ENTRIES as i64])?;
            let mut input_bytes = 0_usize;
            let mut entries = Vec::with_capacity(locations.len());
            for location in locations {
                let row = rows.next()?.ok_or(StorageError::InvalidSchema)?;
                if row.get::<_, String>("requested_id")? != location.id {
                    return Err(StorageError::InvalidSchema);
                }
                let metadata = if location.presence == FilePresence::Missing {
                    ExplorerMetadata::Missing
                } else if !row.get::<_, bool>("has_candidate")? {
                    ExplorerMetadata::Stale
                } else {
                    for column in [
                        "root_revision",
                        "location_revision",
                        "file_revision",
                        "publication_revision",
                    ] {
                        if row.get::<_, i64>(column)? < 0 {
                            return Err(StorageError::InvalidSchema);
                        }
                    }
                    if row.get::<_, bool>("has_snapshot")? {
                        // Both the requested location and the snapshot already
                        // match the file fingerprint via the two joins above.
                        let snapshot = read_snapshot(row)?;
                        snapshot.validate_payload()?;
                        input_bytes += snapshot.payload_json().map_or(0, str::len);
                        if input_bytes > MAX_EXPLORER_INPUT_BYTES {
                            return Ok(ExplorerRead::Limited);
                        }
                        ExplorerMetadata::Current(Box::new(snapshot))
                    } else if self
                        .metadata_snapshot_page(&location.project_file_id, None, 1)?
                        .items
                        .is_empty()
                    {
                        ExplorerMetadata::NoAnalysis
                    } else {
                        ExplorerMetadata::Stale
                    }
                };
                entries.push(ExplorerEntry { location, metadata });
            }
            entries
        };
        tx.rollback()?;
        Ok(ExplorerRead::Ready { root, entries })
    }
}

#[cfg(test)]
mod input_budget_tests;
#[cfg(test)]
mod tests;
