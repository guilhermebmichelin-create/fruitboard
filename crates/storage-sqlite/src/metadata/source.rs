//! Sealed, read-only source selection. No filesystem operation or renderer path.
use super::MetadataInput;
use crate::{Database, Result, StorageError};
use rusqlite::params;

#[derive(Clone, PartialEq, Eq)]
pub struct MetadataSource {
    input: MetadataInput,
    root: String,
    path: String,
    identity: (u64, u128),
}

impl MetadataSource {
    pub fn input(&self) -> &MetadataInput {
        &self.input
    }
    pub fn root(&self) -> &str {
        &self.root
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn identity(&self) -> (u64, u128) {
        self.identity
    }
}
impl Database {
    /// Only enabled local NTFS / present matching sources with qualified IDs.
    /// The host must still qualify the actual held objects before any probes.
    pub fn capture_metadata_source(&self, input: &MetadataInput) -> Result<MetadataSource> {
        if self.capture_metadata_input(&input.root_id, input.location_id())? != *input {
            return Err(StorageError::Conflict);
        }
        let (root, relative, volume, file): (String, String, Option<String>, Option<String>) =
            self.connection.prepare_cached("SELECT r.canonical_path,l.relative_path,l.identity_volume_serial,l.identity_file_id
                FROM file_location l JOIN scan_root r ON r.id=l.scan_root_id
                WHERE r.id=?1 AND l.id=?2")?.query_row(params![input.root_id,input.location_id],
                    |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)))?;
        // Parse Windows components as text even in portable tests. Never allow
        // a stored relative locator to replace the configured root grant.
        let parts: Vec<_> = relative.split(['\\', '/']).collect();
        if root.len() > 32767
            || relative.len() > 32767
            || parts.is_empty()
            || parts
                .iter()
                .any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains([':', '\0']))
        {
            return Err(StorageError::InvalidSchema);
        }
        let (volume, file) = match (volume, file) {
            (Some(v), Some(f)) => (v, f),
            (None, None) => return Err(StorageError::NotFound),
            _ => return Err(StorageError::InvalidSchema),
        };
        let identity = (
            volume
                .parse::<u64>()
                .map_err(|_| StorageError::InvalidSchema)?,
            file.parse::<u128>()
                .map_err(|_| StorageError::InvalidSchema)?,
        );
        if identity.0.to_string() != volume || identity.1.to_string() != file {
            return Err(StorageError::InvalidSchema);
        }
        let path = format!("{}\\{}", root.trim_end_matches('\\'), parts.join("\\"));
        Ok(MetadataSource {
            input: input.clone(),
            root,
            path,
            identity,
        })
    }

    /// Includes every root/file/location/publication revision, actual stored
    /// locator and qualified identity, plus the current snapshot's origin fence.
    pub fn metadata_source_current(&self, source: &MetadataSource, snapshot: &str) -> Result<bool> {
        let current = match self.capture_metadata_source(&source.input) {
            Ok(current) => current,
            Err(StorageError::NotFound | StorageError::Conflict) => return Ok(false),
            Err(error) => return Err(error),
        };
        Ok(current == *source
            && self
                .current_metadata_snapshot_id(source.input.project_file_id())?
                .as_deref()
                == Some(snapshot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::tests::{capabilities, reply, setup};
    use crate::{MIGRATIONS, tests::TestDirectory};
    #[test]
    fn source_is_sealed_by_revisions_qualified_identity_locator_and_current_snapshot() {
        for mutation in [
            "UPDATE scan_root SET configuration_revision=configuration_revision+1",
            "UPDATE project_file SET metadata_revision=metadata_revision+1",
            "UPDATE project_file SET metadata_publication_revision=metadata_publication_revision+1",
            "UPDATE file_location SET metadata_revision=metadata_revision+1",
            "UPDATE file_location SET identity_file_id='9'",
            "UPDATE file_location SET relative_path='other.flp'",
            "UPDATE scan_root SET canonical_path='C:\\Synthetic\\Other'",
            "UPDATE project_file SET current_metadata_snapshot_id=NULL",
            "UPDATE scan_root SET enabled=0",
            "UPDATE file_location SET presence='missing'",
        ] {
            let dir = TestDirectory::new();
            let (mut db, input) = setup(&dir, MIGRATIONS.len());
            db.connection
                .execute(
                    "UPDATE file_location SET identity_volume_serial='7',identity_file_id='3'",
                    [],
                )
                .unwrap();
            let input = db
                .capture_metadata_input(&input.root_id, input.location_id())
                .unwrap();
            let saved = db
                .publish_metadata_snapshot(&input, &capabilities(), reply(), 100)
                .unwrap();
            let input = db
                .capture_metadata_input(&input.root_id, input.location_id())
                .unwrap();
            let source = db.capture_metadata_source(&input).unwrap();
            assert_eq!(source.identity(), (7, 3));
            assert_eq!(source.path(), "C:\\Synthetic\\Projects\\fixture.flp");
            assert!(db.metadata_source_current(&source, &saved.id).unwrap());
            db.connection.execute(mutation, []).unwrap();
            assert!(
                !db.metadata_source_current(&source, &saved.id).unwrap(),
                "{mutation}"
            );
        }
    }
    #[test]
    fn unqualified_or_escaping_stored_locators_never_become_native_input() {
        for relative in [
            "../private.flp",
            "..\\private.flp",
            "C:\\private.flp",
            "\\private.flp",
            "folder//private.flp",
            "folder\\.\\private.flp",
        ] {
            let dir = TestDirectory::new();
            let (db, input) = setup(&dir, MIGRATIONS.len());
            db.connection.execute("UPDATE file_location SET identity_volume_serial='7',identity_file_id='3',relative_path=?1",[relative]).unwrap();
            let input = db
                .capture_metadata_input(&input.root_id, input.location_id())
                .unwrap();
            assert!(db.capture_metadata_source(&input).is_err(), "{relative}");
        }
        let dir = TestDirectory::new();
        let (db, input) = setup(&dir, MIGRATIONS.len());
        assert!(matches!(
            db.capture_metadata_source(&input),
            Err(StorageError::NotFound)
        ));
    }
}
