//! Bounded root-scoped saved metadata read. No filesystem or parser authority.
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
        let mut input_bytes = 0_usize;
        let mut entries = Vec::with_capacity(locations.len());
        for location in locations {
            let metadata = if location.presence == FilePresence::Missing {
                ExplorerMetadata::Missing
            } else {
                match self.capture_metadata_input(root_id, &location.id) {
                    Err(StorageError::NotFound) => ExplorerMetadata::Stale,
                    Err(error) => return Err(error),
                    Ok(input) => match self.current_metadata_snapshot(input.project_file_id())? {
                        Some(snapshot) => {
                            let header = snapshot.header();
                            if header.input_byte_size != location.byte_size
                                || header.input_modified_at_ns != location.modified_at_ns
                            {
                                ExplorerMetadata::Stale
                            } else {
                                input_bytes = input_bytes
                                    .saturating_add(snapshot.payload_json().map_or(0, str::len));
                                if input_bytes > MAX_EXPLORER_INPUT_BYTES {
                                    return Ok(ExplorerRead::Limited);
                                }
                                ExplorerMetadata::Current(Box::new(snapshot))
                            }
                        }
                        None => {
                            if self
                                .metadata_snapshot_page(input.project_file_id(), None, 1)?
                                .items
                                .is_empty()
                            {
                                ExplorerMetadata::NoAnalysis
                            } else {
                                ExplorerMetadata::Stale
                            }
                        }
                    },
                }
            };
            entries.push(ExplorerEntry { location, metadata });
        }
        tx.rollback()?;
        Ok(ExplorerRead::Ready { root, entries })
    }
}

#[cfg(test)]
mod input_budget_tests;
#[cfg(test)]
mod tests;
