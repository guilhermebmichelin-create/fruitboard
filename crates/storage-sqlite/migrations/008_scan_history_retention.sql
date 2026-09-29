-- Keyset maintenance selects at most 64 old terminal candidates per ledger.
-- Per-root ranking seeks only the bounded keep window, never all history.
CREATE INDEX scan_run_retention_age ON scan_run (started_at_ms, id)
    WHERE state IN ('completed', 'failed', 'cancelled', 'interrupted');
CREATE INDEX scan_run_retention_root
    ON scan_run (scan_root_id, started_at_ms DESC, id DESC)
    WHERE state IN ('completed', 'failed', 'cancelled', 'interrupted');
CREATE INDEX scan_job_retention_age ON scan_job (updated_at_ms, id)
    WHERE state IN ('completed', 'failed', 'cancelled', 'interrupted');
CREATE INDEX scan_job_history_root_order
    ON scan_job (scan_root_id, created_at_ms DESC, id DESC);

-- Retention checks references without scanning the library or root catalog.
CREATE INDEX file_location_last_seen_run ON file_location (last_seen_scan_run_id)
    WHERE last_seen_scan_run_id IS NOT NULL;
CREATE INDEX scan_root_last_successful_run ON scan_root (last_successful_run_id)
    WHERE last_successful_run_id IS NOT NULL;
