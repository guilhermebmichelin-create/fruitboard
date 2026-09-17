use std::fmt;

/// Path-free classification retained for the redacted `Database` failure.
///
/// Every value is a stable, closed enum. The raw backend error, its SQL text,
/// bound parameters, and any filesystem path or message are intentionally
/// discarded so only a fixed diagnostic can reach a log or the IPC boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseDetail {
    /// SQLite reported a failure; its primary result code is preserved when
    /// one is available.
    Sqlite(Option<rusqlite::ErrorCode>),
    /// A filesystem operation for the storage backend failed; the OS error
    /// kind is preserved without the path or message.
    Io(std::io::ErrorKind),
}

/// Fixed diagnostics only: SQLite errors may contain SQL, values, and paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    /// A storage I/O failure without a preserved OS kind, for example an
    /// adapter-reported failure or an incomplete backend copy.
    Io,
    Busy,
    UnsafeLocation,
    UnsupportedSqlite,
    UnsupportedJournal,
    NewerSchema,
    InvalidSchema,
    MigrationFailed,
    InvalidBackup,
    Conflict,
    /// The worker supplied terminally invalid staging input. The run has
    /// already been durably failed and its stage discarded.
    StagingRejected,
    NotFound,
    InvalidCursor,
    StaleCursor,
    Database(DatabaseDetail),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Io => "storage_io_failed",
            Self::Busy => "storage_busy",
            Self::UnsafeLocation => "storage_unsafe_location",
            Self::UnsupportedSqlite => "storage_unsupported_sqlite",
            Self::UnsupportedJournal => "storage_unsupported_journal",
            Self::NewerSchema => "storage_newer_schema",
            Self::InvalidSchema => "storage_invalid_schema",
            Self::MigrationFailed => "storage_migration_failed",
            Self::InvalidBackup => "storage_invalid_backup",
            Self::Conflict => "storage_conflict",
            Self::StagingRejected => "storage_staging_rejected",
            Self::NotFound => "storage_not_found",
            Self::InvalidCursor => "storage_invalid_cursor",
            Self::StaleCursor => "storage_stale_cursor",
            Self::Database(_) => "storage_database_failed",
        })
    }
}

/// The closed diagnostic is the whole error: no raw source is exposed.
impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    /// Preserve the OS error kind behind the redacted `Database` variant; the
    /// message, path, and raw error value are discarded.
    fn from(error: std::io::Error) -> Self {
        Self::Database(DatabaseDetail::Io(error.kind()))
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
                Self::Busy
            }
            code => Self::Database(DatabaseDetail::Sqlite(code)),
        }
    }
}

pub type Result<T> = std::result::Result<T, StorageError>;
