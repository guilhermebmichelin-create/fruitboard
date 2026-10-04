use super::*;
use crate::MIGRATIONS;
use crate::metadata::tests::{capabilities, reply, setup};
use crate::tests::TestDirectory;

#[test]
fn explorer_uses_current_projection_and_keeps_source_gaps_and_root_scope() {
    let directory = TestDirectory::new();
    let (mut db, input) = setup(&directory, MIGRATIONS.len());
    let other = db.add_scan_root("Other", "C:\\Synthetic\\Other").unwrap();
    match db.read_plugin_explorer(&input.root_id).unwrap() {
        ExplorerRead::Ready { entries, .. } => {
            assert_eq!(entries.len(), 1);
            assert!(matches!(entries[0].metadata, ExplorerMetadata::NoAnalysis));
        }
        _ => panic!("expected complete root read"),
    }
    db.publish_metadata_snapshot(&input, &capabilities(), reply(), 100)
        .unwrap();
    match db.read_plugin_explorer(&input.root_id).unwrap() {
        ExplorerRead::Ready { entries, .. } => {
            assert!(matches!(entries[0].metadata, ExplorerMetadata::Current(_)))
        }
        _ => panic!("expected current read"),
    }
    match db.read_plugin_explorer(&other.id).unwrap() {
        ExplorerRead::Ready { entries, .. } => assert!(entries.is_empty()),
        _ => panic!("expected other root"),
    }
    db.connection
        .execute(
            "UPDATE file_location SET presence='missing', metadata_revision=metadata_revision+1",
            [],
        )
        .unwrap();
    match db.read_plugin_explorer(&input.root_id).unwrap() {
        ExplorerRead::Ready { entries, .. } => {
            assert!(matches!(entries[0].metadata, ExplorerMetadata::Missing))
        }
        _ => panic!("expected missing coverage"),
    }
    db.connection
        .execute("UPDATE file_location SET presence='present'", [])
        .unwrap();
    match db.read_plugin_explorer(&input.root_id).unwrap() {
        ExplorerRead::Ready { entries, .. } => {
            assert!(matches!(entries[0].metadata, ExplorerMetadata::Stale))
        }
        _ => panic!("expected stale coverage"),
    }
    db.set_scan_root_enabled(&input.root_id, false).unwrap();
    assert!(matches!(
        db.read_plugin_explorer(&input.root_id).unwrap(),
        ExplorerRead::Unavailable
    ));
    assert!(matches!(
        db.read_plugin_explorer("unknown"),
        Err(StorageError::NotFound)
    ));
}

#[test]
fn explorer_entry_budget_returns_no_partial_totals_or_payloads() {
    let directory = TestDirectory::new();
    let (db, input) = setup(&directory, MIGRATIONS.len());
    for index in 1..=MAX_EXPLORER_ENTRIES {
        let id = format!("location-{index}");
        let locator = format!("v1:i:fixture-{index}.flp");
        db.connection.execute(
            "INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms)
             SELECT ?1,project_file_id,scan_root_id,?2,?2,?2,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms FROM file_location WHERE id='location'",
            rusqlite::params![id, locator],
        ).unwrap();
    }
    assert!(matches!(
        db.read_plugin_explorer(&input.root_id).unwrap(),
        ExplorerRead::Limited
    ));
    db.connection
        .execute("DELETE FROM file_location WHERE id='location-2000'", [])
        .unwrap();
    match db.read_plugin_explorer(&input.root_id).unwrap() {
        ExplorerRead::Ready { entries, .. } => assert_eq!(entries.len(), MAX_EXPLORER_ENTRIES),
        _ => panic!("exact boundary should fit"),
    }
}
