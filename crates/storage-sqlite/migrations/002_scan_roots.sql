CREATE TABLE scan_root (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (length(display_name) > 0),
    canonical_path TEXT NOT NULL UNIQUE CHECK (length(canonical_path) > 0),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    availability TEXT NOT NULL DEFAULT 'unknown'
        CHECK (availability IN ('available', 'unavailable', 'unknown')),
    last_error_code TEXT CHECK (
        last_error_code IS NULL
        OR last_error_code IN ('unavailable_path', 'permission_denied')
    )
) STRICT;
