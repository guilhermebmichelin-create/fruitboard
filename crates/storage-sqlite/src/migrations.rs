use crate::{Result, StorageError};
use rusqlite::{Connection, TransactionBehavior};

pub(crate) const APPLICATION_ID: i64 = 0x46524244;
pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "001_local_settings",
        sql: include_str!("../migrations/001_local_settings.sql"),
    },
    Migration {
        name: "002_scan_roots",
        sql: include_str!("../migrations/002_scan_roots.sql"),
    },
    Migration {
        name: "003_scan_execution",
        sql: include_str!("../migrations/003_scan_execution.sql"),
    },
    Migration {
        name: "004_scan_publication",
        sql: include_str!("../migrations/004_scan_publication.sql"),
    },
    Migration {
        name: "005_integration_contract",
        sql: include_str!("../migrations/005_integration_contract.sql"),
    },
];

#[derive(Clone, Copy)]
pub(crate) struct Migration {
    pub(crate) name: &'static str,
    pub(crate) sql: &'static str,
}

pub(crate) fn version(connection: &Connection, migrations: &[Migration]) -> Result<usize> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > migrations.len() as i64 {
        return Err(StorageError::NewerSchema);
    }
    let application: i64 =
        connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if version == 0 {
        let tables: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )?;
        if tables != 0 || application != 0 {
            return Err(StorageError::InvalidSchema);
        }
        return Ok(0);
    }
    if version < 0 || application != APPLICATION_ID {
        return Err(StorageError::InvalidSchema);
    }
    let mut statement = connection
        .prepare("SELECT version, name, sql FROM schema_migration ORDER BY version")
        .map_err(|_| StorageError::InvalidSchema)?;
    let records = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if records.len() != version as usize {
        return Err(StorageError::InvalidSchema);
    }
    for (index, (record_version, name, sql)) in records.iter().enumerate() {
        let expected = &migrations[index];
        if *record_version != (index + 1) as i64 || name != expected.name || sql != expected.sql {
            return Err(StorageError::InvalidSchema);
        }
    }
    Ok(version as usize)
}

pub(crate) fn validate_integrity(connection: &Connection) -> Result<()> {
    let mut check = connection.prepare("PRAGMA integrity_check")?;
    let checks = check
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let foreign_key_violation = connection
        .prepare("PRAGMA foreign_key_check")?
        .query([])?
        .next()?
        .is_some();
    if checks != ["ok"] || foreign_key_violation {
        return Err(StorageError::InvalidSchema);
    }
    Ok(())
}

pub(crate) fn apply(connection: &mut Connection, migrations: &[Migration]) -> Result<()> {
    apply_observed(connection, migrations, || {})
}

pub(crate) fn apply_observed(
    connection: &mut Connection,
    migrations: &[Migration],
    before_commit: impl FnOnce(),
) -> Result<()> {
    // The schema changes, seeds, ledger, and version are one commit, including
    // when several future migrations are pending. Drop rolls back on any error.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current = version(&transaction, migrations)?;
    if current == 0 {
        transaction.execute_batch(
            "CREATE TABLE schema_migration (
                version INTEGER PRIMARY KEY CHECK (version > 0),
                name TEXT NOT NULL UNIQUE,
                sql TEXT NOT NULL
            ) STRICT;",
        )?;
        transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
    }
    for (index, migration) in migrations.iter().enumerate().skip(current) {
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migration (version, name, sql) VALUES (?1, ?2, ?3)",
            rusqlite::params![(index + 1) as i64, migration.name, migration.sql],
        )?;
    }
    transaction.pragma_update(None, "user_version", migrations.len() as i64)?;
    validate_integrity(&transaction)?;
    crate::read_startup_view(&transaction)?;
    before_commit();
    transaction.commit()?;
    Ok(())
}
