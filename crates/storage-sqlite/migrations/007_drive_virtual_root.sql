ALTER TABLE scan_root
    ADD COLUMN mode TEXT NOT NULL DEFAULT 'local_ntfs'
        CHECK (mode IN ('local_ntfs', 'drive_virtual'));
