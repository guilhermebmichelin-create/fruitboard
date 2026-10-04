use super::*;
use crate::MIGRATIONS;
use crate::metadata::tests::{capabilities, reply, setup};
use crate::tests::TestDirectory;
use fruitboard_flp_parser::supervisor::ProtocolReply;
use serde_json::json;

#[test]
fn explorer_input_budget_keeps_shared_file_locations_distinct_and_fails_closed() {
    let directory = TestDirectory::new();
    let (mut db, input) = setup(&directory, MIGRATIONS.len());
    let ProtocolReply::Result(mut raw) = reply() else {
        panic!("result fixture required")
    };
    let name = "Synthetic".repeat(12);
    let plugin = json!({"className":{"status":"extracted","value":name},"name":{"status":"extracted","value":name},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}});
    raw["eventCount"] = json!(512);
    raw["pluginReferences"] = json!({"status":"extracted","coverage":"top-level-saved-references","value":vec![plugin;512]});
    db.publish_metadata_snapshot(&input, &capabilities(), ProtocolReply::Result(raw), 100)
        .unwrap();
    let snapshot = db
        .current_metadata_snapshot(input.project_file_id())
        .unwrap()
        .unwrap();
    let bytes = snapshot.payload_json().unwrap().len();
    let allowed = MAX_EXPLORER_INPUT_BYTES / bytes;
    assert!(allowed > 1 && allowed < MAX_EXPLORER_ENTRIES);
    for index in 1..=allowed {
        let id = format!("copy-{index}");
        let key = format!("v1:i:copy-{index}.flp");
        db.connection.execute(
            "INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms)
             SELECT ?1,project_file_id,scan_root_id,?2,?2,?2,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms FROM file_location WHERE id='location'",
            rusqlite::params![id, key],
        ).unwrap();
        if index == allowed - 1 {
            match db.read_plugin_explorer(&input.root_id).unwrap() {
                ExplorerRead::Ready { entries, .. } => {
                    assert_eq!(entries.len(), allowed);
                    assert!(
                        entries
                            .iter()
                            .all(|entry| matches!(entry.metadata, ExplorerMetadata::Current(_)))
                    );
                }
                _ => panic!("bounded locations should fit"),
            }
        }
    }
    assert!(matches!(
        db.read_plugin_explorer(&input.root_id).unwrap(),
        ExplorerRead::Limited
    ));
}
