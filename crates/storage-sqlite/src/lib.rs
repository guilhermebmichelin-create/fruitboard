//! Native-only SQLite ownership. No connection, SQL executor, or path is sent to IPC.
mod error;
mod execution;
mod files;
mod migrations;
mod publication;

pub use error::{Result, StorageError};
pub use execution::{
    DEFAULT_SCAN_MAX_ATTEMPTS, EnqueueResult, LeasedScan, ScanJob, ScanJobState, ScanKind,
    ScanRootExecution, ScanRun, ScanRunOutcome, ScanRunState, ScanSession,
};
use files::{Location, check_path, private_directory, private_file};
use migrations::{MIGRATIONS, Migration};
pub use publication::{
    FilePresence, LibraryCursor, LibraryPage, LibraryQuery, LibrarySnapshot, MAX_LIBRARY_PAGE_SIZE,
    MAX_STAGED_BATCH_RECORDS, MAX_STAGED_PATH_BYTES, MAX_STAGED_RECORDS, PublishedLocation,
    ScanObservation, ScanPublication, ScanRootPublication, ScanStageState, ScanStaging,
};
use rusqlite::{
    Connection, OpenFlags, TransactionBehavior,
    backup::{Backup, StepResult},
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn sqlite_version() -> &'static str {
    rusqlite::version()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SqlitePolicy {
    minimum_wal_safe_version: String,
    approved_fixed_backports: Vec<String>,
}
#[derive(Deserialize)]
struct Policy {
    sqlite: SqlitePolicy,
}

fn parse_version(value: &str) -> Option<[u64; 3]> {
    let parts = value
        .split('.')
        .map(str::parse)
        .collect::<std::result::Result<Vec<u64>, _>>()
        .ok()?;
    parts.try_into().ok()
}

/// Reads the same committed policy as the JavaScript verification gate.
pub fn verify_embedded_version() -> Result<()> {
    let policy: Policy = serde_json::from_str(include_str!("../../../tools/toolchain-policy.json"))
        .map_err(|_| StorageError::UnsupportedSqlite)?;
    let current = parse_version(sqlite_version()).ok_or(StorageError::UnsupportedSqlite)?;
    let minimum = parse_version(&policy.sqlite.minimum_wal_safe_version)
        .ok_or(StorageError::UnsupportedSqlite)?;
    if current >= minimum
        || policy
            .sqlite
            .approved_fixed_backports
            .iter()
            .any(|v| parse_version(v) == Some(current))
    {
        Ok(())
    } else {
        Err(StorageError::UnsupportedSqlite)
    }
}

fn connect(path: &Path, read_only: bool) -> Result<Connection> {
    check_path(path)?;
    let flags = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };
    let connection = Connection::open_with_flags(
        path,
        flags | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    connection.busy_timeout(Duration::from_secs(2))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "trusted_schema", false)?;
    let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if journal != "delete" {
        return Err(StorageError::UnsupportedJournal);
    }
    if !read_only {
        connection.pragma_update(None, "synchronous", "FULL")?;
    }
    Ok(connection)
}

fn copy_database(source: &Connection, destination: &mut Connection) -> Result<()> {
    let backup = Backup::new(source, destination)?;
    // This slice owns one connection and tiny data. A bounded single step avoids
    // an unbounded retry loop if another process holds SQLite locks.
    match backup.step(-1)? {
        StepResult::Done => Ok(()),
        _ => Err(StorageError::Busy),
    }
}

fn read_startup_view(connection: &Connection) -> Result<StartupView> {
    let value: String = connection.query_row(
        "SELECT startup_view FROM app_settings WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    StartupView::parse(&value)
}

fn backup_into(connection: &Connection, directory: &Path) -> Result<PathBuf> {
    migrations::validate_integrity(connection)?;
    private_directory(directory)?;
    let stem = uuid::Uuid::now_v7();
    let pending = directory.join(format!("{stem}.pending.db"));
    let finished = directory.join(format!("{stem}.backup.db"));
    private_file(&pending, true)?;
    {
        let mut destination = connect(&pending, false)?;
        copy_database(connection, &mut destination)?;
        migrations::validate_integrity(&destination)?;
    }
    // Publish only a completed, checked backup; interrupted .pending files are
    // never candidates for recovery and are retained for explicit maintenance.
    std::fs::rename(&pending, &finished)?;
    Ok(finished)
}

/// One native owner per application-data directory. Put this behind a Mutex
/// in the desktop host; SQL and connection access remain private to the crate.
pub struct Database {
    connection: Connection,
    location: Location,
}

impl Database {
    /// app_data is resolved by the native platform, never supplied by the renderer.
    pub fn open(app_data: &Path) -> Result<Self> {
        Self::open_with_migrations(app_data, MIGRATIONS)
    }

    fn open_with_migrations(app_data: &Path, migrations: &[Migration]) -> Result<Self> {
        verify_embedded_version()?;
        let location = Location::acquire(app_data)?;
        private_file(&location.database(), false)?;
        let mut connection = connect(&location.database(), false)?;
        let current = migrations::version(&connection, migrations)?;
        if current < migrations.len() {
            if current > 0 {
                backup_into(&connection, &location.directory.join("backups"))?;
            }
            migrations::apply(&mut connection, migrations)
                .map_err(|_| StorageError::MigrationFailed)?;
        }
        let database = Self {
            connection,
            location,
        };
        database.startup_view()?;
        Ok(database)
    }

    pub fn schema_version(&self) -> Result<usize> {
        migrations::version(&self.connection, MIGRATIONS)
    }

    /// A consistent SQLite snapshot; returns a native-only location, not an IPC value.
    pub fn create_backup(&mut self) -> Result<PathBuf> {
        backup_into(&self.connection, &self.location.directory.join("backups"))
    }

    /// Restore into a fresh application-data location. The source database and
    /// backup are preserved, including when the original database is corrupt.
    /// The destination must contain neither the database nor its journal/WAL/SHM.
    /// Selecting/adopting that location belongs to a future native recovery UI.
    pub fn recover_to(backup: &Path, new_app_data: &Path) -> Result<Self> {
        verify_embedded_version()?;
        if !backup
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".backup.db"))
        {
            return Err(StorageError::InvalidBackup);
        }
        let source = connect(backup, true).map_err(|_| StorageError::InvalidBackup)?;
        migrations::validate_integrity(&source).map_err(|_| StorageError::InvalidBackup)?;
        if migrations::version(&source, MIGRATIONS).map_err(|_| StorageError::InvalidBackup)? == 0 {
            return Err(StorageError::InvalidBackup);
        }
        read_startup_view(&source).map_err(|_| StorageError::InvalidBackup)?;
        let location = Location::acquire(new_app_data)?;
        location.ensure_recovery_destination_empty()?;
        let pending = location
            .directory
            .join(format!("{}.recovery.pending.db", uuid::Uuid::now_v7()));
        private_file(&pending, true)?;
        let mut connection = connect(&pending, false)?;
        copy_database(&source, &mut connection)?;
        migrations::validate_integrity(&connection)?;
        if migrations::version(&connection, MIGRATIONS)? < MIGRATIONS.len() {
            migrations::apply(&mut connection, MIGRATIONS)
                .map_err(|_| StorageError::MigrationFailed)?;
        }
        execution::invalidate_inflight_after_recovery(&mut connection, execution::wall_clock_ms())?;
        drop(connection);
        std::fs::rename(&pending, location.database())?;
        let connection = connect(&location.database(), false)?;
        Ok(Self {
            connection,
            location,
        })
    }

    // Repository operations added in #16 use this one atomic writer boundary.
    // No caller outside this crate can execute arbitrary SQL through it.
    fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = operation(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }

    pub fn startup_view(&self) -> Result<StartupView> {
        read_startup_view(&self.connection)
    }

    pub fn set_startup_view(&mut self, view: StartupView) -> Result<()> {
        self.transaction(|transaction| {
            let changed = transaction.execute(
                "UPDATE app_settings SET startup_view = ?1 WHERE singleton = 1",
                [view.as_str()],
            )?;
            if changed != 1 {
                return Err(StorageError::InvalidSchema);
            }
            Ok(())
        })
    }

    pub fn list_scan_roots(&self) -> Result<Vec<ScanRoot>> {
        let mut statement = self.connection.prepare(
            "SELECT id, display_name, canonical_path, enabled, availability, last_error_code
             FROM scan_root ORDER BY rowid",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().map(Self::scan_root_from_tuple).collect()
    }

    pub fn add_scan_root(&mut self, display_name: &str, canonical_path: &str) -> Result<ScanRoot> {
        if display_name.is_empty() || canonical_path.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|transaction| {
            let duplicates: i64 = transaction.query_row(
                "SELECT count(*) FROM scan_root WHERE canonical_path = ?1",
                [canonical_path],
                |row| row.get(0),
            )?;
            if duplicates != 0 {
                return Err(StorageError::Conflict);
            }
            let root = ScanRoot {
                id: uuid::Uuid::now_v7().to_string(),
                display_name: display_name.to_owned(),
                canonical_path: canonical_path.to_owned(),
                enabled: true,
                availability: ScanRootAvailability::Available,
                last_error_code: None,
            };
            transaction.execute(
                "INSERT INTO scan_root
                 (id, display_name, canonical_path, enabled, availability, last_error_code)
                 VALUES (?1, ?2, ?3, 1, ?4, NULL)",
                rusqlite::params![
                    &root.id,
                    &root.display_name,
                    &root.canonical_path,
                    root.availability.as_str()
                ],
            )?;
            Ok(root)
        })
    }
    fn scan_root_from_tuple(
        row: (String, String, String, i64, String, Option<String>),
    ) -> Result<ScanRoot> {
        let (id, display_name, canonical_path, enabled, availability, last_error_code) = row;
        Ok(ScanRoot {
            id,
            display_name,
            canonical_path,
            enabled: enabled == 1,
            availability: ScanRootAvailability::parse(&availability)?,
            last_error_code,
        })
    }

    fn select_scan_root(transaction: &rusqlite::Transaction<'_>, id: &str) -> Result<ScanRoot> {
        let row = transaction
            .query_row(
                "SELECT id, display_name, canonical_path, enabled, availability, last_error_code
                 FROM scan_root WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
                _ => error.into(),
            })?;
        Self::scan_root_from_tuple(row)
    }

    pub fn set_scan_root_display_name(&mut self, id: &str, display_name: &str) -> Result<ScanRoot> {
        if display_name.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|transaction| {
            let changed = transaction.execute(
                "UPDATE scan_root SET display_name = ?1 WHERE id = ?2",
                rusqlite::params![display_name, id],
            )?;
            if changed != 1 {
                return Err(StorageError::NotFound);
            }
            Self::select_scan_root(transaction, id)
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupView {
    Home,
    Library,
    Board,
    Preferences,
}

impl StartupView {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "home" => Ok(Self::Home),
            "library" => Ok(Self::Library),
            "board" => Ok(Self::Board),
            "preferences" => Ok(Self::Preferences),
            _ => Err(StorageError::InvalidSchema),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Library => "library",
            Self::Board => "board",
            Self::Preferences => "preferences",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanRootAvailability {
    Available,
    Unavailable,
    Unknown,
}

impl ScanRootAvailability {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "available" => Ok(Self::Available),
            "unavailable" => Ok(Self::Unavailable),
            "unknown" => Ok(Self::Unknown),
            _ => Err(StorageError::InvalidSchema),
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable => "unavailable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRoot {
    pub id: String,
    pub display_name: String,
    pub canonical_path: String,
    pub enabled: bool,
    pub availability: ScanRootAvailability,
    pub last_error_code: Option<String>,
}

#[cfg(test)]
mod tests;
