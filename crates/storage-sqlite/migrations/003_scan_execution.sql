ALTER TABLE scan_root ADD COLUMN configuration_revision INTEGER NOT NULL DEFAULT 0
    CHECK (configuration_revision >= 0);
ALTER TABLE scan_root ADD COLUMN generation INTEGER NOT NULL DEFAULT 0
    CHECK (generation >= 0);

CREATE TABLE scan_session (
    id TEXT PRIMARY KEY CHECK (length(id) > 0),
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER CHECK (
        ended_at_ms IS NULL OR ended_at_ms >= started_at_ms
    )
) STRICT;

CREATE TABLE scan_job (
    id TEXT PRIMARY KEY,
    scan_root_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('initial', 'manual', 'periodic', 'recovery')),
    state TEXT NOT NULL DEFAULT 'queued'
        CHECK (state IN ('queued', 'running', 'completed', 'failed', 'cancelled', 'interrupted')),
    retry_chain_id TEXT NOT NULL,
    attempt INTEGER NOT NULL DEFAULT 0 CHECK (attempt >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 4 CHECK (max_attempts > 0),
    not_before_ms INTEGER NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    follow_up_requested INTEGER NOT NULL DEFAULT 0
        CHECK (follow_up_requested IN (0, 1)),
    cancellation_requested INTEGER NOT NULL DEFAULT 0
        CHECK (cancellation_requested IN (0, 1)),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    last_error_code TEXT
) STRICT;

CREATE UNIQUE INDEX scan_job_active_root
    ON scan_job (scan_root_id)
    WHERE state IN ('queued', 'running');
CREATE INDEX scan_job_due
    ON scan_job (state, not_before_ms, priority DESC, created_at_ms);
CREATE INDEX scan_job_root
    ON scan_job (scan_root_id, created_at_ms);

CREATE TABLE scan_run (
    id TEXT PRIMARY KEY,
    scan_job_id TEXT NOT NULL,
    scan_root_id TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    configuration_revision INTEGER NOT NULL CHECK (configuration_revision >= 0),
    retry_chain_id TEXT NOT NULL,
    attempt INTEGER NOT NULL CHECK (attempt > 0),
    session_id TEXT NOT NULL,
    lease_token TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'running'
        CHECK (state IN ('running', 'completed', 'failed', 'cancelled', 'interrupted')),
    cancellation_requested INTEGER NOT NULL DEFAULT 0
        CHECK (cancellation_requested IN (0, 1)),
    started_at_ms INTEGER NOT NULL,
    finished_at_ms INTEGER,
    lease_expires_at_ms INTEGER NOT NULL,
    outcome TEXT CHECK (
        outcome IS NULL OR outcome IN ('completed', 'failed', 'cancelled', 'interrupted')
    ),
    error_code TEXT,
    UNIQUE (scan_job_id, attempt)
) STRICT;

CREATE INDEX scan_run_job ON scan_run (scan_job_id, attempt);
CREATE INDEX scan_run_active_lease ON scan_run (state, lease_expires_at_ms);
CREATE INDEX scan_run_root ON scan_run (scan_root_id, started_at_ms);
