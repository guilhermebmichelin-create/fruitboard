-- Publication resolves live identity candidates only within the root being
-- published. Keeping the root and presence columns first prevents aliases in
-- unrelated roots, or retained missing paths, from expanding that lookup.
CREATE INDEX file_location_root_live_identity
    ON file_location (
        scan_root_id,
        presence,
        identity_volume_serial,
        identity_file_id,
        project_file_id,
        locator_key COLLATE BINARY
    )
    WHERE scan_root_id IS NOT NULL
      AND presence = 'present'
      AND identity_volume_serial IS NOT NULL
      AND identity_file_id IS NOT NULL;
