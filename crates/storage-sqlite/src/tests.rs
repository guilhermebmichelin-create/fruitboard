use super::*;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("fruitboard-storage-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn database(&self) -> PathBuf {
        self.0.join("storage/fruitboard.db")
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn upgraded_migrations(sql: &'static str) -> Vec<Migration> {
    vec![
        Migration {
            name: MIGRATIONS[0].name,
            sql: MIGRATIONS[0].sql,
        },
        Migration {
            name: MIGRATIONS[1].name,
            sql: MIGRATIONS[1].sql,
        },
        Migration {
            name: "003_test_only",
            sql,
        },
    ]
}

#[test]
fn embedded_runtime_passes_the_shared_policy() {
    println!("Embedded SQLite runtime: {}", sqlite_version());
    verify_embedded_version().unwrap();
}

#[test]
fn creates_latest_and_reopens_without_reseeding_settings() {
    let directory = TestDirectory::new();
    {
        let mut database = Database::open(directory.path()).unwrap();
        assert_eq!(database.schema_version().unwrap(), 2);
        assert_eq!(database.startup_view().unwrap(), StartupView::Home);
        database.set_startup_view(StartupView::Library).unwrap();
    }
    let database = Database::open(directory.path()).unwrap();
    assert_eq!(database.startup_view().unwrap(), StartupView::Library);
    assert!(!directory.path().join("storage/backups").exists());
}

#[test]
fn upgrades_every_supported_fixture_and_preserves_existing_rows() {
    // v0 (empty), v1 (settings), and v2 (scan roots) are the supported set.
    // Fixtures are generated from committed SQL, never private binary databases.
    for version in 0..=MIGRATIONS.len() {
        let directory = TestDirectory::new();
        if version == 0 {
            fs::create_dir(directory.path().join("storage")).unwrap();
            Connection::open(directory.database())
                .unwrap()
                .close()
                .unwrap();
        } else {
            let mut fixture =
                Database::open_with_migrations(directory.path(), &MIGRATIONS[..version]).unwrap();
            fixture.set_startup_view(StartupView::Board).unwrap();
        }
        let latest = Database::open(directory.path()).unwrap();
        assert_eq!(latest.schema_version().unwrap(), MIGRATIONS.len());
        assert_eq!(
            latest.startup_view().unwrap(),
            if version == 0 {
                StartupView::Home
            } else {
                StartupView::Board
            }
        );
    }
}

#[test]
fn future_upgrade_creates_a_restorable_pre_migration_backup() {
    let directory = TestDirectory::new();
    {
        let mut database = Database::open(directory.path()).unwrap();
        database.set_startup_view(StartupView::Preferences).unwrap();
    }
    let migrations = upgraded_migrations("ALTER TABLE app_settings ADD COLUMN test_value TEXT;");
    let database = Database::open_with_migrations(directory.path(), &migrations).unwrap();
    assert_eq!(
        migrations::version(&database.connection, &migrations).unwrap(),
        3
    );
    let backups: Vec<_> = fs::read_dir(directory.path().join("storage/backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let recovered_directory = TestDirectory::new();
    let recovered = Database::recover_to(&backups[0], recovered_directory.path()).unwrap();
    assert_eq!(recovered.schema_version().unwrap(), 2);
    assert_eq!(recovered.startup_view().unwrap(), StartupView::Preferences);
}

#[test]
fn failed_migration_rolls_back_schema_data_and_ledger() {
    let directory = TestDirectory::new();
    {
        let mut database = Database::open(directory.path()).unwrap();
        database.set_startup_view(StartupView::Library).unwrap();
    }
    let before = fs::read(directory.database()).unwrap();
    let migrations = upgraded_migrations(
        "UPDATE app_settings SET startup_view = 'board';
         CREATE TABLE should_rollback (id INTEGER);
         INSERT INTO missing_table VALUES (1);",
    );
    assert!(matches!(
        Database::open_with_migrations(directory.path(), &migrations),
        Err(StorageError::MigrationFailed)
    ));
    assert_eq!(fs::read(directory.database()).unwrap(), before);
    let database = Database::open(directory.path()).unwrap();
    assert_eq!(database.schema_version().unwrap(), 2);
    assert_eq!(database.startup_view().unwrap(), StartupView::Library);
    let count: i64 = database
        .connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name = 'should_rollback'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn foreign_keys_and_repository_transactions_enforce_atomic_changes() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    database
        .connection
        .execute_batch(
            "CREATE TABLE test_parent (id INTEGER PRIMARY KEY);
         CREATE TABLE test_child (parent INTEGER REFERENCES test_parent(id));",
        )
        .unwrap();
    let result: Result<()> = database.transaction(|transaction| {
        transaction.execute("UPDATE app_settings SET startup_view = ?1", ["board"])?;
        transaction.execute("INSERT INTO test_child VALUES (?1)", [99])?;
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(database.startup_view().unwrap(), StartupView::Home);
    database
        .transaction(|transaction| {
            transaction.execute("INSERT INTO test_parent VALUES (?1)", [99])?;
            transaction.execute("INSERT INTO test_child VALUES (?1)", [99])?;
            Ok(())
        })
        .unwrap();
    assert!(
        database
            .connection
            .execute("DELETE FROM test_parent", [])
            .is_err()
    );
}

#[test]
fn schema_rejects_invalid_settings_and_duplicates() {
    let directory = TestDirectory::new();
    let database = Database::open(directory.path()).unwrap();
    assert!(
        database
            .connection
            .execute(
                "UPDATE app_settings SET startup_view = ?1",
                ["private/path"]
            )
            .is_err()
    );
    assert!(
        database
            .connection
            .execute("INSERT INTO app_settings VALUES (2, 'home')", [])
            .is_err()
    );
    assert_eq!(database.startup_view().unwrap(), StartupView::Home);
}

#[test]
fn unknown_newer_and_tampered_schemas_fail_without_rewriting() {
    for sql in [
        "PRAGMA user_version = 99;",
        "UPDATE schema_migration SET sql = 'changed';",
        "DELETE FROM schema_migration;",
        "PRAGMA application_id = 0;",
        "DROP TABLE app_settings;",
    ] {
        let directory = TestDirectory::new();
        {
            let database = Database::open(directory.path()).unwrap();
            database.connection.execute_batch(sql).unwrap();
        }
        let before = fs::read(directory.database()).unwrap();
        assert!(Database::open(directory.path()).is_err());
        assert_eq!(fs::read(directory.database()).unwrap(), before);
    }
}

#[test]
fn refuses_unrecognized_unversioned_databases() {
    let directory = TestDirectory::new();
    fs::create_dir(directory.path().join("storage")).unwrap();
    {
        let connection = Connection::open(directory.database()).unwrap();
        connection
            .execute_batch("CREATE TABLE unrelated (id INTEGER);")
            .unwrap();
    }
    let before = fs::read(directory.database()).unwrap();
    assert!(matches!(
        Database::open(directory.path()),
        Err(StorageError::InvalidSchema)
    ));
    assert_eq!(fs::read(directory.database()).unwrap(), before);
}

#[test]
fn uses_delete_journaling_full_durability_and_one_native_owner() {
    let directory = TestDirectory::new();
    let database = Database::open(directory.path()).unwrap();
    let journal: String = database
        .connection
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    let durability: i64 = database
        .connection
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .unwrap();
    assert_eq!(journal, "delete");
    assert_eq!(durability, 2);
    assert!(matches!(
        Database::open(directory.path()),
        Err(StorageError::Busy)
    ));
    drop(database);
    let connection = Connection::open(directory.database()).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    drop(connection);
    assert!(matches!(
        Database::open(directory.path()),
        Err(StorageError::UnsupportedJournal)
    ));
}

#[test]
fn backup_recovers_committed_state_after_original_corruption_without_overwriting_it() {
    let directory = TestDirectory::new();
    let backup = {
        let mut database = Database::open(directory.path()).unwrap();
        database.set_startup_view(StartupView::Library).unwrap();
        database.create_backup().unwrap()
    };
    fs::write(directory.database(), b"synthetic corruption").unwrap();
    assert!(Database::open(directory.path()).is_err());
    let recovered_directory = TestDirectory::new();
    {
        let recovered = Database::recover_to(&backup, recovered_directory.path()).unwrap();
        assert_eq!(recovered.startup_view().unwrap(), StartupView::Library);
    }
    assert_eq!(
        fs::read(directory.database()).unwrap(),
        b"synthetic corruption"
    );
    let reopened = Database::open(recovered_directory.path()).unwrap();
    assert_eq!(reopened.startup_view().unwrap(), StartupView::Library);
    assert!(backup.exists());
}

#[test]
fn invalid_or_incomplete_backups_and_existing_destinations_are_preserved() {
    let directory = TestDirectory::new();
    let backup = Database::open(directory.path())
        .unwrap()
        .create_backup()
        .unwrap();
    let before = fs::read(&backup).unwrap();
    assert!(Database::recover_to(&backup, directory.path()).is_err());
    assert_eq!(fs::read(&backup).unwrap(), before);
    let target = TestDirectory::new();
    let incomplete = directory.path().join("interrupted.pending.db");
    fs::write(&incomplete, &before).unwrap();
    assert!(matches!(
        Database::recover_to(&incomplete, target.path()),
        Err(StorageError::InvalidBackup)
    ));
    let invalid = directory.path().join("invalid.backup.db");
    fs::write(&invalid, b"not sqlite").unwrap();
    assert!(matches!(
        Database::recover_to(&invalid, target.path()),
        Err(StorageError::InvalidBackup)
    ));
    assert!(!target.database().exists());
}

#[test]
fn raw_sqlite_errors_never_escape_in_storage_diagnostics() {
    let source = rusqlite::Error::InvalidParameterName("private project.flp".into());
    let safe = StorageError::from(source);
    assert_eq!(safe.to_string(), "storage_database_failed");
    assert!(!format!("{safe:?}").contains("private"));
    assert!(std::error::Error::source(&safe).is_none());
}

fn assert_recovery_rejects_existing_artifact(name: &str) {
    let source_directory = TestDirectory::new();
    let backup = {
        let mut database = Database::open(source_directory.path()).unwrap();
        database.set_startup_view(StartupView::Library).unwrap();
        database.create_backup().unwrap()
    };
    let backup_before = fs::read(&backup).unwrap();
    let source_before = fs::read(source_directory.database()).unwrap();
    for contents in [b"".as_slice(), b"existing recovery evidence".as_slice()] {
        let destination = TestDirectory::new();
        let storage = destination.path().join("storage");
        fs::create_dir(&storage).unwrap();
        fs::write(storage.join("owner.lock"), b"existing owner lock").unwrap();
        fs::write(storage.join(name), contents).unwrap();
        assert!(
            matches!(
                Database::recover_to(&backup, destination.path()),
                Err(StorageError::UnsafeLocation)
            ),
            "recovery must refuse {name}, including when empty"
        );
        assert_eq!(fs::read(storage.join(name)).unwrap(), contents);
        assert_eq!(
            fs::read(storage.join("owner.lock")).unwrap(),
            b"existing owner lock"
        );
        assert_eq!(
            fs::read_dir(&storage).unwrap().count(),
            2,
            "rejection must not create a staged or published database"
        );
        assert_eq!(fs::read(&backup).unwrap(), backup_before);
        assert_eq!(
            fs::read(source_directory.database()).unwrap(),
            source_before
        );
    }
}

#[test]
fn recovery_rejects_an_existing_database_without_changing_files() {
    assert_recovery_rejects_existing_artifact("fruitboard.db");
}

#[test]
fn recovery_rejects_an_orphan_journal_without_changing_files() {
    assert_recovery_rejects_existing_artifact("fruitboard.db-journal");
}

#[test]
fn recovery_rejects_an_orphan_wal_without_changing_files() {
    assert_recovery_rejects_existing_artifact("fruitboard.db-wal");
}

#[test]
fn recovery_rejects_an_orphan_shm_without_changing_files() {
    assert_recovery_rejects_existing_artifact("fruitboard.db-shm");
}

#[test]
fn recovery_rejects_a_hot_journal_that_would_replace_library_with_home() {
    let source_directory = TestDirectory::new();
    let (backup, journal) = {
        let mut database = Database::open(source_directory.path()).unwrap();
        let transaction = database.connection.transaction().unwrap();
        transaction
            .execute("UPDATE app_settings SET startup_view = 'board'", [])
            .unwrap();
        transaction.cache_flush().unwrap();
        // Capture a valid rollback journal containing Home before rolling back.
        // Only generated test files are copied; no live product database is used.
        let journal = fs::read(
            source_directory
                .path()
                .join("storage/fruitboard.db-journal"),
        )
        .unwrap();
        transaction.rollback().unwrap();
        database.set_startup_view(StartupView::Library).unwrap();
        (database.create_backup().unwrap(), journal)
    };
    let backup_before = fs::read(&backup).unwrap();
    let source_before = fs::read(source_directory.database()).unwrap();

    // Independently prove this fixture is dangerous: SQLite replays the journal
    // against the offline Library backup and silently returns Home.
    let replay_control = TestDirectory::new();
    fs::create_dir(replay_control.path().join("storage")).unwrap();
    fs::copy(&backup, replay_control.database()).unwrap();
    let control_journal = replay_control.path().join("storage/fruitboard.db-journal");
    fs::write(&control_journal, &journal).unwrap();
    let control = Connection::open(replay_control.database()).unwrap();
    assert_eq!(read_startup_view(&control).unwrap(), StartupView::Home);
    assert!(!control_journal.exists());

    let destination = TestDirectory::new();
    let storage = destination.path().join("storage");
    fs::create_dir(&storage).unwrap();
    fs::write(storage.join("owner.lock"), b"existing owner lock").unwrap();
    let orphan = storage.join("fruitboard.db-journal");
    fs::write(&orphan, &journal).unwrap();
    assert!(matches!(
        Database::recover_to(&backup, destination.path()),
        Err(StorageError::UnsafeLocation)
    ));
    assert_eq!(fs::read(&orphan).unwrap(), journal);
    assert_eq!(fs::read_dir(&storage).unwrap().count(), 2);
    assert!(!destination.database().exists());
    assert_eq!(fs::read(&backup).unwrap(), backup_before);
    assert_eq!(
        fs::read(source_directory.database()).unwrap(),
        source_before
    );
}

#[test]
fn backup_failure_prevents_migration_from_touching_original_data() {
    let directory = TestDirectory::new();
    Database::open(directory.path()).unwrap();
    let before = fs::read(directory.database()).unwrap();
    fs::write(
        directory.path().join("storage/backups"),
        b"blocked destination",
    )
    .unwrap();
    let migrations = upgraded_migrations("UPDATE app_settings SET startup_view = 'board';");
    assert!(Database::open_with_migrations(directory.path(), &migrations).is_err());
    assert_eq!(fs::read(directory.database()).unwrap(), before);
}

#[test]
fn a_migration_that_removes_required_settings_rolls_back_before_commit() {
    let directory = TestDirectory::new();
    Database::open(directory.path()).unwrap();
    let before = fs::read(directory.database()).unwrap();
    let migrations = upgraded_migrations("DELETE FROM app_settings;");
    assert!(matches!(
        Database::open_with_migrations(directory.path(), &migrations),
        Err(StorageError::MigrationFailed)
    ));
    assert_eq!(fs::read(directory.database()).unwrap(), before);
}

#[test]
fn unicode_application_data_locations_round_trip() {
    let directory = TestDirectory::new();
    let nested = directory.path().join("Espaço 音楽");
    let mut database = Database::open(&nested).unwrap();
    database.set_startup_view(StartupView::Board).unwrap();
    let backup = database.create_backup().unwrap();
    let restored = Database::recover_to(&backup, &directory.path().join("Recuperação")).unwrap();
    assert_eq!(restored.startup_view().unwrap(), StartupView::Board);
}

#[test]
fn rejects_relative_locations_and_preexisting_links() {
    assert!(matches!(
        Database::open(Path::new("relative")),
        Err(StorageError::UnsafeLocation)
    ));
    #[cfg(unix)]
    {
        let directory = TestDirectory::new();
        let outside = TestDirectory::new();
        std::os::unix::fs::symlink(outside.path(), directory.path().join("storage")).unwrap();
        assert!(matches!(
            Database::open(directory.path()),
            Err(StorageError::UnsafeLocation)
        ));
        assert!(!outside.path().join("fruitboard.db").exists());
    }
}

#[cfg(unix)]
#[test]
fn unix_database_and_backup_permissions_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let backup = database.create_backup().unwrap();
    assert_eq!(
        fs::metadata(directory.path().join("storage"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    for path in [directory.database(), backup] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn killed_migration_recovers_the_original_committed_database() {
    const ENVIRONMENT: &str = "FRUITBOARD_MIGRATION_CRASH_TEST";
    if let Some(directory) = std::env::var_os(ENVIRONMENT) {
        let mut database = Database::open(Path::new(&directory)).unwrap();
        database.create_backup().unwrap();
        // Enough pages to spill dirty pages into the database before COMMIT,
        // requiring a real hot-journal recovery when the process is terminated.
        database
            .connection
            .pragma_update(None, "cache_size", 5)
            .unwrap();
        let migrations = upgraded_migrations(
            "UPDATE app_settings SET startup_view = 'board';
             CREATE TABLE interrupted (id INTEGER PRIMARY KEY, value BLOB);
             WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<100)
             INSERT INTO interrupted SELECT x, zeroblob(4096) FROM n;",
        );
        migrations::apply_observed(&mut database.connection, &migrations, || {
            println!("MIGRATION_READY_TO_TERMINATE");
            std::io::stdout().flush().unwrap();
            loop {
                std::thread::park();
            }
        })
        .unwrap();
        unreachable!();
    }
    let directory = TestDirectory::new();
    {
        let mut database = Database::open(directory.path()).unwrap();
        database.set_startup_view(StartupView::Library).unwrap();
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "tests::killed_migration_recovers_the_original_committed_database",
            "--nocapture",
        ])
        .env(ENVIRONMENT, directory.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if line.unwrap().contains("MIGRATION_READY_TO_TERMINATE") {
                let _ = send.send(());
                break;
            }
        }
    });
    let ready = receive.recv_timeout(Duration::from_secs(30));
    let _ = child.kill();
    let status = child.wait().unwrap();
    reader.join().unwrap();
    ready.expect("child must reach the uncommitted migration");
    assert!(!status.success());
    assert!(
        directory
            .path()
            .join("storage/fruitboard.db-journal")
            .exists()
    );
    let recovered = Database::open(directory.path()).unwrap();
    assert_eq!(recovered.schema_version().unwrap(), 2);
    assert_eq!(recovered.startup_view().unwrap(), StartupView::Library);
    migrations::validate_integrity(&recovered.connection).unwrap();
    assert!(
        recovered
            .connection
            .prepare("SELECT * FROM interrupted")
            .is_err()
    );
}

#[test]
fn scan_roots_add_list_remove_and_persist_across_restart() {
    let directory = TestDirectory::new();
    let first_id;
    let second_id;
    {
        let mut database = Database::open(directory.path()).unwrap();
        assert!(database.list_scan_roots().unwrap().is_empty());

        let first = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        let second = database
            .add_scan_root("Loops", "D:\\Loops")
            .unwrap();
        assert_ne!(first.id, second.id);
        for root in [&first, &second] {
            assert!(root.enabled);
            assert_eq!(root.availability, ScanRootAvailability::Available);
            assert_eq!(root.last_error_code, None);
        }
        first_id = first.id.clone();
        second_id = second.id.clone();

        let listed = database.list_scan_roots().unwrap();
        assert_eq!(
            listed.iter().map(|root| root.id.clone()).collect::<Vec<_>>(),
            vec![first_id.clone(), second_id.clone()]
        );

        database.remove_scan_root(&first_id).unwrap();
        assert_eq!(database.list_scan_roots().unwrap().len(), 1);
    }
    let database = Database::open(directory.path()).unwrap();
    let listed = database.list_scan_roots().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, second_id);
    assert_eq!(listed[0].display_name, "Loops");
    assert_eq!(listed[0].canonical_path, "D:\\Loops");
}

#[test]
fn scan_roots_reject_duplicates_blank_inputs_and_unknown_removals() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();

    assert!(matches!(
        database.add_scan_root("Again", "C:\\Music\\Projects"),
        Err(StorageError::Conflict)
    ));
    assert!(database.list_scan_roots().unwrap().len() == 1);
    for (name, path) in [("", "C:\\Other"), ("Name", ""), ("", "")] {
        assert!(matches!(
            database.add_scan_root(name, path),
            Err(StorageError::InvalidSchema)
        ));
    }
    assert!(matches!(
        database.remove_scan_root("missing-root-id"),
        Err(StorageError::NotFound)
    ));
    assert!(database.list_scan_roots().unwrap().len() == 1);
}

#[test]
fn scan_root_schema_enforces_bounds_and_survives_backup_recovery() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    assert!(
        database
            .connection
            .execute(
                "INSERT INTO scan_root
                 (id, display_name, canonical_path, enabled, availability, last_error_code)
                 VALUES ('x', 'Bad', 'C:\\Bad', 2, 'available', NULL)",
                []
            )
            .is_err()
    );
    assert!(
        database
            .connection
            .execute(
                "INSERT INTO scan_root
                 (id, display_name, canonical_path, enabled, availability, last_error_code)
                 VALUES ('y', 'Bad', 'C:\\Bad', 1, 'scanning', NULL)",
                []
            )
            .is_err()
    );
    assert!(
        database
            .connection
            .execute(
                "INSERT INTO scan_root
                 (id, display_name, canonical_path, enabled, availability, last_error_code)
                 VALUES ('z', 'Bad', 'C:\\Music\\Projects', 1, 'available', NULL)",
                []
            )
            .is_err()
    );

    let backup = database.create_backup().unwrap();
    let recovered_directory = TestDirectory::new();
    let recovered = Database::recover_to(&backup, recovered_directory.path()).unwrap();
    let listed = recovered.list_scan_roots().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].display_name, "Projects");
    assert_eq!(listed[0].canonical_path, "C:\\Music\\Projects");
}

#[test]
fn upgrade_from_v1_preserves_preferences_and_starts_empty_roots() {
    let directory = TestDirectory::new();
    {
        let mut fixture =
            Database::open_with_migrations(directory.path(), &MIGRATIONS[..1]).unwrap();
        fixture.set_startup_view(StartupView::Board).unwrap();
    }
    let database = Database::open(directory.path()).unwrap();
    assert_eq!(database.schema_version().unwrap(), 2);
    assert_eq!(database.startup_view().unwrap(), StartupView::Board);
    assert!(database.list_scan_roots().unwrap().is_empty());
}
