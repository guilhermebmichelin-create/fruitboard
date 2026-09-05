use std::fmt;

/// Fixed diagnostics only: SQLite errors may contain SQL, values, and paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    Io,
    Busy,
    UnsafeLocation,
    UnsupportedSqlite,
    UnsupportedJournal,
    NewerSchema,
    InvalidSchema,
    MigrationFailed,
    InvalidBackup,
    Database,
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
            Self::Database => "storage_database_failed",
        })
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(_: std::io::Error) -> Self {
        Self::Io
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
                Self::Busy
            }
            _ => Self::Database,
        }
    }
}

pub type Result<T> = std::result::Result<T, StorageError>;
