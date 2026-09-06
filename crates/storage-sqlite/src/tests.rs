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
    let mut migrations = MIGRATIONS.to_vec();
    migrations.push(Migration {
        name: "004_test_only",
        sql,
    });
    migrations
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
        assert_eq!(database.schema_version().unwrap(), 3);
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
        4
    );
    let backups: Vec<_> = fs::read_dir(directory.path().join("storage/backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let recovered_directory = TestDirectory::new();
    let recovered = Database::recover_to(&backups[0], recovered_directory.path()).unwrap();
    assert_eq!(recovered.schema_version().unwrap(), 3);
    assert_eq!(recovered.startup_view().unwrap(), StartupView::Preferences);
}

#[test]
fn failed_migration_rolls_back_schema_data_and_ledger() {
    let directory = TestDirectory::new();
    {
        let mut database = Database::open(directory.path()).unwrap();
        database.set_startup_view(StartupView::Library).unwrap();
        database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
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
    assert_eq!(database.schema_version().unwrap(), 3);
    assert_eq!(database.startup_view().unwrap(), StartupView::Library);
    assert_eq!(database.list_scan_roots().unwrap().len(), 1);
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
fn recovery_fences_inflight_execution_before_publishing_the_destination() {
    let source_directory = TestDirectory::new();
    let (backup, run_id, job_id) = {
        let mut database = Database::open(source_directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database.begin_scan_session("source-session", 1).unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap();
        let lease = database
            .lease_next_scan("source-session", 3, 100)
            .unwrap()
            .unwrap();
        let backup = database.create_backup().unwrap();
        (backup, lease.run.id, lease.job.id)
    };
    let destination = TestDirectory::new();
    let recovered = Database::recover_to(&backup, destination.path()).unwrap();
    assert_eq!(
        recovered.scan_run(&run_id).unwrap().state,
        ScanRunState::Interrupted
    );
    assert_eq!(
        recovered.scan_job(&job_id).unwrap().state,
        ScanJobState::Interrupted
    );
    assert!(
        recovered
            .list_scan_jobs()
            .unwrap()
            .iter()
            .all(|job| job.state != ScanJobState::Running)
    );
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
    assert_eq!(recovered.schema_version().unwrap(), 3);
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
        let second = database.add_scan_root("Loops", "D:\\Loops").unwrap();
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
            listed
                .iter()
                .map(|root| root.id.clone())
                .collect::<Vec<_>>(),
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
    assert_eq!(database.schema_version().unwrap(), 3);
    assert_eq!(database.startup_view().unwrap(), StartupView::Board);
    assert!(database.list_scan_roots().unwrap().is_empty());
}

#[test]
fn scan_root_settings_update_atomically_and_persist_across_restart() {
    let directory = TestDirectory::new();
    let root_id;
    {
        let mut database = Database::open(directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        root_id = root.id.clone();

        let renamed = database
            .set_scan_root_display_name(&root_id, "Released Projects")
            .unwrap();
        assert_eq!(renamed.id, root_id);
        assert_eq!(renamed.display_name, "Released Projects");
        assert_eq!(renamed.canonical_path, "C:\\Music\\Projects");

        let disabled = database.set_scan_root_enabled(&root_id, false).unwrap();
        assert!(!disabled.enabled);
        assert_eq!(disabled.display_name, "Released Projects");
        let enabled = database.set_scan_root_enabled(&root_id, true).unwrap();
        assert!(enabled.enabled);
    }
    let database = Database::open(directory.path()).unwrap();
    let listed = database.list_scan_roots().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].display_name, "Released Projects");
    assert!(listed[0].enabled);
}

#[test]
fn scan_root_settings_reject_unknown_ids_and_blank_names() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();

    assert!(matches!(
        database.set_scan_root_display_name("missing-root-id", "Name"),
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        database.set_scan_root_enabled("missing-root-id", false),
        Err(StorageError::NotFound)
    ));
    assert!(matches!(
        database.set_scan_root_display_name(&root.id, ""),
        Err(StorageError::InvalidSchema)
    ));

    let unchanged = database.list_scan_roots().unwrap();
    assert_eq!(unchanged.len(), 1);
    assert_eq!(unchanged[0].display_name, "Projects");
    assert!(unchanged[0].enabled);
}

#[test]
fn durable_scan_jobs_coalesce_and_schedule_one_follow_up() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("session-1", 10).unwrap();

    let first = database
        .enqueue_scan(&root.id, ScanKind::Manual, 20)
        .unwrap();
    assert!(!first.coalesced);
    let lease = database
        .lease_next_scan("session-1", 21, 100)
        .unwrap()
        .unwrap();
    let second = database
        .enqueue_scan(&root.id, ScanKind::Periodic, 21)
        .unwrap();
    assert!(second.coalesced);
    assert!(second.follow_up_requested);
    assert_eq!(first.job_id, second.job_id);

    assert_eq!(lease.job.attempt, 1);
    assert_eq!(
        database
            .finish_scan_run(
                &lease.run.id,
                "session-1",
                &lease.run.lease_token,
                23,
                ScanRunOutcome::Completed,
            )
            .unwrap(),
        ScanRunState::Completed
    );

    let jobs = database.list_scan_jobs().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].state, ScanJobState::Completed);
    assert_eq!(jobs[1].state, ScanJobState::Queued);
    assert_eq!(jobs[1].kind, ScanKind::Manual);
}

#[test]
fn durable_lease_policy_starts_with_one_global_worker() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let first = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    let second = database.add_scan_root("Loops", "C:\\Music\\Loops").unwrap();
    database.begin_scan_session("session-1", 1).unwrap();
    database
        .enqueue_scan(&first.id, ScanKind::Manual, 2)
        .unwrap();
    database
        .enqueue_scan(&second.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("session-1", 3, 100)
        .unwrap()
        .unwrap();
    assert!(
        database
            .lease_next_scan("session-1", 4, 100)
            .unwrap()
            .is_none()
    );
    database
        .finish_scan_run(
            &lease.run.id,
            "session-1",
            &lease.run.lease_token,
            5,
            ScanRunOutcome::Completed,
        )
        .unwrap();
    let next = database
        .lease_next_scan("session-1", 6, 100)
        .unwrap()
        .unwrap();
    assert_eq!(next.root.id, second.id);
}

#[test]
fn failed_attempt_can_be_explicitly_retried_with_a_fresh_run() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("session-1", 1).unwrap();
    let job_id = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap()
        .job_id;
    let first = database
        .lease_next_scan("session-1", 3, 100)
        .unwrap()
        .unwrap();
    let chain = first.job.retry_chain_id.clone();
    assert_eq!(
        database
            .finish_scan_run(
                &first.run.id,
                "session-1",
                &first.run.lease_token,
                4,
                ScanRunOutcome::Failed,
            )
            .unwrap(),
        ScanRunState::Failed
    );
    assert!(database.retry_failed_scan_job(&job_id, 5).unwrap());
    let second = database
        .lease_next_scan("session-1", 6, 100)
        .unwrap()
        .unwrap();
    assert_ne!(first.run.id, second.run.id);
    assert_eq!(second.job.retry_chain_id, chain);
    assert_eq!(second.job.attempt, 2);
}

#[test]
fn expired_and_restarted_workers_are_fenced_and_retry_attempts_persist() {
    let directory = TestDirectory::new();
    let root = {
        let mut database = Database::open(directory.path()).unwrap();
        database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap()
    };
    let (first_run_id, first_token, first_job_id) = {
        let mut database = Database::open(directory.path()).unwrap();
        database.begin_scan_session("session-1", 10).unwrap();
        let job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 11)
            .unwrap();
        let lease = database
            .lease_next_scan("session-1", 12, 3)
            .unwrap()
            .unwrap();
        (lease.run.id, lease.run.lease_token, job.job_id)
    };
    {
        let mut database = Database::open(directory.path()).unwrap();
        assert_eq!(database.reap_expired_scan_leases(15).unwrap(), 1);
        assert_eq!(
            database.scan_run(&first_run_id).unwrap().state,
            ScanRunState::Interrupted
        );
        assert!(matches!(
            database.finish_scan_run(
                &first_run_id,
                "session-1",
                &first_token,
                16,
                ScanRunOutcome::Completed,
            ),
            Err(StorageError::Conflict)
        ));
        assert_eq!(database.scan_job(&first_job_id).unwrap().attempt, 1);
    }
    {
        let mut database = Database::open(directory.path()).unwrap();
        database.begin_scan_session("session-2", 20).unwrap();
        let lease = database
            .lease_next_scan("session-2", 1_020, 3)
            .unwrap()
            .unwrap();
        assert_eq!(lease.job.id, first_job_id);
        assert_eq!(lease.job.attempt, 2);
        assert_eq!(database.scan_job(&first_job_id).unwrap().attempt, 2);
    }
}

#[test]
fn retry_budget_is_exhausted_without_resetting_after_reopen() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("session-1", 1).unwrap();
    let job_id = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap()
        .job_id;

    let first = database
        .lease_next_scan("session-1", 3, 1)
        .unwrap()
        .unwrap();
    assert_eq!(first.job.attempt, 1);
    database.reap_expired_scan_leases(4).unwrap();
    let second = database
        .lease_next_scan("session-1", 1_004, 1)
        .unwrap()
        .unwrap();
    assert_eq!(second.job.attempt, 2);
    database.reap_expired_scan_leases(1_005).unwrap();
    let third = database
        .lease_next_scan("session-1", 3_005, 1)
        .unwrap()
        .unwrap();
    assert_eq!(third.job.attempt, 3);
    database.reap_expired_scan_leases(3_006).unwrap();
    let fourth = database
        .lease_next_scan("session-1", 7_006, 1)
        .unwrap()
        .unwrap();
    assert_eq!(fourth.job.attempt, 4);
    database.reap_expired_scan_leases(7_007).unwrap();
    assert_eq!(
        database.scan_job(&job_id).unwrap().state,
        ScanJobState::Failed
    );
    assert!(!database.retry_failed_scan_job(&job_id, 3_007).unwrap());
    drop(database);

    let database = Database::open(directory.path()).unwrap();
    let job = database.scan_job(&job_id).unwrap();
    assert_eq!(job.attempt, 4);
    assert_eq!(job.max_attempts, DEFAULT_SCAN_MAX_ATTEMPTS);
    assert_eq!(job.state, ScanJobState::Failed);
}

#[test]
fn restart_invalidates_old_session_and_enqueues_recovery() {
    let directory = TestDirectory::new();
    let root = {
        let mut database = Database::open(directory.path()).unwrap();
        database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap()
    };
    let (run_id, token) = {
        let mut database = Database::open(directory.path()).unwrap();
        database.begin_scan_session("session-a", 10).unwrap();
        let lease = database.lease_next_scan("session-a", 11, 100).unwrap();
        // No job was explicitly queued, so the first session has no work.
        assert!(lease.is_none());
        let job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 12)
            .unwrap();
        let lease = database
            .lease_next_scan("session-a", 13, 100)
            .unwrap()
            .unwrap();
        assert_eq!(lease.job.id, job.job_id);
        (lease.run.id, lease.run.lease_token)
    };

    let mut database = Database::open(directory.path()).unwrap();
    database.begin_scan_session("session-b", 20).unwrap();
    assert_eq!(
        database.scan_run(&run_id).unwrap().state,
        ScanRunState::Interrupted
    );
    assert!(matches!(
        database.finish_scan_run(&run_id, "session-a", &token, 21, ScanRunOutcome::Completed,),
        Err(StorageError::Conflict)
    ));
    let recovery = database
        .lease_next_scan("session-b", 22, 100)
        .unwrap()
        .unwrap();
    assert_eq!(recovery.job.kind, ScanKind::Recovery);
    assert_eq!(recovery.root.id, root.id);
}

#[test]
fn expired_or_replaced_tokens_and_sessions_cannot_renew_or_finish() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("session-1", 1).unwrap();
    database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("session-1", 3, 10)
        .unwrap()
        .unwrap();
    assert!(matches!(
        database.renew_scan_lease(
            &lease.run.id,
            "other-session",
            &lease.run.lease_token,
            4,
            10,
        ),
        Err(StorageError::Conflict)
    ));
    assert!(matches!(
        database.renew_scan_lease(&lease.run.id, "session-1", "replaced-token", 4, 10,),
        Err(StorageError::Conflict)
    ));
    assert_eq!(
        database
            .renew_scan_lease(&lease.run.id, "session-1", &lease.run.lease_token, 4, 10,)
            .unwrap(),
        14
    );
    assert!(matches!(
        database.finish_scan_run(
            &lease.run.id,
            "session-1",
            "replaced-token",
            5,
            ScanRunOutcome::Completed,
        ),
        Err(StorageError::Conflict)
    ));
}

#[test]
fn cancellation_ordering_is_durable_and_late_cancel_cannot_roll_back() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("session-1", 1).unwrap();
    database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let cancelled = database
        .lease_next_scan("session-1", 3, 100)
        .unwrap()
        .unwrap();
    assert_eq!(
        database
            .request_scan_cancellation(&cancelled.run.id, 4)
            .unwrap(),
        ScanRunState::Running
    );
    assert_eq!(
        database
            .finish_scan_run(
                &cancelled.run.id,
                "session-1",
                &cancelled.run.lease_token,
                5,
                ScanRunOutcome::Completed,
            )
            .unwrap(),
        ScanRunState::Cancelled
    );
    assert_eq!(
        database
            .request_scan_cancellation(&cancelled.run.id, 6)
            .unwrap(),
        ScanRunState::Cancelled
    );

    database
        .enqueue_scan(&root.id, ScanKind::Manual, 7)
        .unwrap();
    let completed = database
        .lease_next_scan("session-1", 8, 100)
        .unwrap()
        .unwrap();
    assert_eq!(
        database
            .finish_scan_run(
                &completed.run.id,
                "session-1",
                &completed.run.lease_token,
                9,
                ScanRunOutcome::Completed,
            )
            .unwrap(),
        ScanRunState::Completed
    );
    assert_eq!(
        database
            .request_scan_cancellation(&completed.run.id, 10)
            .unwrap(),
        ScanRunState::Completed
    );
}

#[test]
fn disabling_and_removing_roots_invalidate_work_and_readding_gets_new_identity() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    let initial = database.scan_root_execution(&root.id).unwrap();
    assert_eq!((initial.configuration_revision, initial.generation), (0, 0));
    database.begin_scan_session("session-1", 1).unwrap();
    database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("session-1", 3, 100)
        .unwrap()
        .unwrap();
    let disabled = database
        .set_scan_root_enabled_at(&root.id, false, 4)
        .unwrap();
    assert!(!disabled.enabled);
    assert_eq!(
        (
            database
                .scan_root_execution(&root.id)
                .unwrap()
                .configuration_revision,
            database.scan_root_execution(&root.id).unwrap().generation
        ),
        (1, 1)
    );
    assert_eq!(
        database.scan_run(&lease.run.id).unwrap().state,
        ScanRunState::Cancelled
    );
    assert!(matches!(
        database.finish_scan_run(
            &lease.run.id,
            "session-1",
            &lease.run.lease_token,
            5,
            ScanRunOutcome::Completed,
        ),
        Err(StorageError::Conflict)
    ));
    let enabled = database
        .set_scan_root_enabled_at(&root.id, true, 6)
        .unwrap();
    assert!(enabled.enabled);
    let current = database.scan_root_execution(&root.id).unwrap();
    assert_eq!((current.configuration_revision, current.generation), (2, 2));

    database.remove_scan_root_at(&root.id, 7).unwrap();
    assert!(matches!(
        database.scan_root_execution(&root.id),
        Err(StorageError::NotFound)
    ));
    let replacement = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    assert_ne!(replacement.id, root.id);
    assert_eq!(
        database
            .scan_root_execution(&replacement.id)
            .unwrap()
            .generation,
        0
    );
}

#[test]
fn migration_to_execution_schema_preserves_roots_preferences_and_defaults() {
    let directory = TestDirectory::new();
    {
        let mut fixture =
            Database::open_with_migrations(directory.path(), &MIGRATIONS[..2]).unwrap();
        fixture.set_startup_view(StartupView::Board).unwrap();
        fixture
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
    }
    let database = Database::open(directory.path()).unwrap();
    assert_eq!(database.schema_version().unwrap(), 3);
    assert_eq!(database.startup_view().unwrap(), StartupView::Board);
    let root = &database.list_scan_roots().unwrap()[0];
    let execution = database.scan_root_execution(&root.id).unwrap();
    assert_eq!(
        (execution.configuration_revision, execution.generation),
        (0, 0)
    );
}
