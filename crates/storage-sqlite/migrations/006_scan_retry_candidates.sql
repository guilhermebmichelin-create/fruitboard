-- Keep the retry sweep on the bounded, durable candidate subset. Exhausted
-- and cancelled terminal chains are not retry candidates, while the existing
-- root/active indexes continue to fence disabled and occupied roots.
CREATE INDEX scan_job_retry_order
    ON scan_job (created_at_ms, id, updated_at_ms)
    WHERE state = 'failed'
      AND cancellation_requested = 0
      AND attempt < max_attempts;

CREATE INDEX scan_job_status_active_order
    ON scan_job (scan_root_id, created_at_ms DESC, id DESC)
    WHERE state IN ('queued', 'running');

CREATE INDEX scan_job_status_terminal_order
    ON scan_job (scan_root_id, created_at_ms DESC, id DESC)
    WHERE state NOT IN ('queued', 'running');

CREATE INDEX scan_run_status_order
    ON scan_run (scan_job_id, started_at_ms DESC, id DESC);
