use super::*;
use crate::execution::wall_clock_ms;
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

fn publish_empty_scan(database: &mut Database, lease: &LeasedScan, now_ms: i64) {
    database
        .begin_scan_staging(
            &lease.run.id,
            &lease.run.session_id,
            &lease.run.lease_token,
            now_ms,
        )
        .unwrap();
    database
        .publish_scan_run(
            &lease.run.id,
            &lease.run.session_id,
            &lease.run.lease_token,
            now_ms,
        )
        .unwrap();
}

fn publish_observations(
    database: &mut Database,
    lease: &LeasedScan,
    now_ms: i64,
    observations: &[ScanObservation],
) {
    database
        .stage_scan_observations(
            &lease.run.id,
            &lease.run.session_id,
            &lease.run.lease_token,
            now_ms,
            observations,
        )
        .unwrap();
    database
        .publish_scan_run(
            &lease.run.id,
            &lease.run.session_id,
            &lease.run.lease_token,
            now_ms + 1,
        )
        .unwrap();
}

/// Map a fixture display path to a valid insensitive `LocatorKeyV1`.
/// This emulates the enumerator folding an insensitive component (§1.1);
/// case-sensitive and mixed-mode keys use `exact_observation` explicitly.
fn v1_key(path: &str) -> String {
    let segments = path
        .replace('\\', "/")
        .split('/')
        .map(|segment| format!("i:{segment}"))
        .collect::<Vec<_>>();
    format!("v1:{}", segments.join("/"))
}

fn staged_observation(
    path: &str,
    byte_size: u64,
    modified_at_ns: i128,
    identity: Option<&str>,
) -> ScanObservation {
    ScanObservation {
        locator_key: v1_key(path),
        relative_path: path.to_owned(),
        byte_size,
        modified_at_ns,
        identity: identity.map(|label| EncodedIdentity {
            volume_serial: "1".to_owned(),
            file_id: label
                .bytes()
                .fold(1_u128, |value, byte| value * 257 + u128::from(byte))
                .to_string(),
        }),
    }
}

fn exact_observation(
    locator_key: &str,
    relative_path: &str,
    byte_size: u64,
    modified_at_ns: i128,
    identity: Option<EncodedIdentity>,
) -> ScanObservation {
    ScanObservation {
        locator_key: locator_key.to_owned(),
        relative_path: relative_path.to_owned(),
        byte_size,
        modified_at_ns,
        identity,
    }
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
        assert_eq!(database.schema_version().unwrap(), 5);
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
fn migration_rejects_legacy_timestamp_that_cannot_be_converted() {
    let directory = TestDirectory::new();
    let fixture = Database::open_with_migrations(directory.path(), &MIGRATIONS[..4]).unwrap();
    fixture
        .connection
        .execute(
            "INSERT INTO project_file
             (id, display_filename, extension, byte_size, modified_at_ms,
              created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            rusqlite::params![
                "legacy-project",
                "legacy.flp",
                ".flp",
                1_i64,
                i64::MAX,
                1_i64
            ],
        )
        .unwrap();
    drop(fixture);

    assert!(matches!(
        Database::open_with_migrations(directory.path(), MIGRATIONS),
        Err(StorageError::MigrationFailed)
    ));
    let fixture = Database::open_with_migrations(directory.path(), &MIGRATIONS[..4]).unwrap();
    assert_eq!(fixture.schema_version().unwrap(), 4);
    assert_eq!(
        fixture
            .connection
            .query_row(
                "SELECT modified_at_ms FROM project_file WHERE id = ?1",
                ["legacy-project"],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        i64::MAX
    );
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
        6
    );
    let backups: Vec<_> = fs::read_dir(directory.path().join("storage/backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let recovered_directory = TestDirectory::new();
    let recovered = Database::recover_to(&backups[0], recovered_directory.path()).unwrap();
    assert_eq!(recovered.schema_version().unwrap(), 5);
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
    assert_eq!(database.schema_version().unwrap(), 5);
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
    let (backup, run_id, job_id, chain, generation) = {
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
        (
            backup,
            lease.run.id,
            lease.job.id,
            lease.job.retry_chain_id,
            lease.run.generation,
        )
    };
    let destination = TestDirectory::new();
    let mut recovered = Database::recover_to(&backup, destination.path()).unwrap();
    assert_eq!(
        recovered.scan_run(&run_id).unwrap().state,
        ScanRunState::Interrupted
    );
    let interrupted = recovered.scan_job(&job_id).unwrap();
    assert_eq!(interrupted.state, ScanJobState::Queued);
    assert_eq!(interrupted.retry_chain_id, chain);
    assert_eq!(interrupted.attempt, 1);
    assert!(
        recovered
            .list_scan_jobs()
            .unwrap()
            .iter()
            .all(|job| job.state != ScanJobState::Running)
    );
    let resume_at = interrupted.not_before_ms;
    recovered
        .begin_scan_session("destination-session", resume_at)
        .unwrap();
    let resumed = recovered
        .lease_next_scan("destination-session", resume_at, 100)
        .unwrap()
        .unwrap();
    assert_eq!(resumed.job.id, job_id);
    assert_eq!(resumed.job.retry_chain_id, chain);
    assert_eq!(resumed.job.attempt, 2);
    assert!(resumed.run.generation > generation);
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
    assert_eq!(recovered.schema_version().unwrap(), 5);
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
    assert_eq!(database.schema_version().unwrap(), 5);
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
    let third = database
        .enqueue_scan(&root.id, ScanKind::Manual, 22)
        .unwrap();
    assert!(third.coalesced);
    assert!(third.follow_up_requested);
    assert_eq!(third.job_id, first.job_id);

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
        ScanRunState::Interrupted
    );

    let finished_run = database.scan_run(&lease.run.id).unwrap();
    assert_eq!(finished_run.state, ScanRunState::Interrupted);
    assert_eq!(finished_run.outcome, Some(ScanRunOutcome::Interrupted));
    assert_eq!(
        finished_run.error_code.as_deref(),
        Some("follow_up_requested")
    );
    let jobs = database.list_scan_jobs().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].state, ScanJobState::Interrupted);
    assert!(jobs[0].follow_up_requested);
    assert_eq!(jobs[1].state, ScanJobState::Queued);
    assert_eq!(jobs[1].kind, ScanKind::Manual);
    let follow_up = database
        .lease_next_scan("session-1", jobs[1].not_before_ms, 100)
        .unwrap()
        .unwrap();
    assert_eq!(follow_up.job.id, jobs[1].id);
    assert!(follow_up.run.generation > lease.run.generation);
    publish_empty_scan(&mut database, &follow_up, jobs[1].not_before_ms + 1);
    assert_eq!(
        database.scan_run(&follow_up.run.id).unwrap().state,
        ScanRunState::Completed
    );
}

#[test]
fn execution_finish_transition_matrix_covers_outcomes_followups_and_cancellation() {
    #[derive(Clone, Copy)]
    struct FinishCase {
        name: &'static str,
        outcome: ScanRunOutcome,
        follow_up: bool,
        durable_cancellation: bool,
    }

    // Cross every finish outcome with the two invalidation sources and the
    // presence of a coalesced follow-up. The budget loop below runs this
    // complete matrix once with remaining budget and once at the boundary.
    let cases = [
        FinishCase {
            name: "completed_plain",
            outcome: ScanRunOutcome::Completed,
            follow_up: false,
            durable_cancellation: false,
        },
        FinishCase {
            name: "completed_follow_up",
            outcome: ScanRunOutcome::Completed,
            follow_up: true,
            durable_cancellation: false,
        },
        FinishCase {
            name: "failed_plain",
            outcome: ScanRunOutcome::Failed,
            follow_up: false,
            durable_cancellation: false,
        },
        FinishCase {
            name: "failed_follow_up",
            outcome: ScanRunOutcome::Failed,
            follow_up: true,
            durable_cancellation: false,
        },
        FinishCase {
            name: "cancelled_plain",
            outcome: ScanRunOutcome::Cancelled,
            follow_up: false,
            durable_cancellation: false,
        },
        FinishCase {
            name: "cancelled_follow_up",
            outcome: ScanRunOutcome::Cancelled,
            follow_up: true,
            durable_cancellation: false,
        },
        FinishCase {
            name: "interrupted_plain",
            outcome: ScanRunOutcome::Interrupted,
            follow_up: false,
            durable_cancellation: false,
        },
        FinishCase {
            name: "interrupted_follow_up",
            outcome: ScanRunOutcome::Interrupted,
            follow_up: true,
            durable_cancellation: false,
        },
        FinishCase {
            name: "durable_cancel_completed_plain",
            outcome: ScanRunOutcome::Completed,
            follow_up: false,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_completed_follow_up",
            outcome: ScanRunOutcome::Completed,
            follow_up: true,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_failed_plain",
            outcome: ScanRunOutcome::Failed,
            follow_up: false,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_failed_follow_up",
            outcome: ScanRunOutcome::Failed,
            follow_up: true,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_cancelled_plain",
            outcome: ScanRunOutcome::Cancelled,
            follow_up: false,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_cancelled_follow_up",
            outcome: ScanRunOutcome::Cancelled,
            follow_up: true,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_interrupted_plain",
            outcome: ScanRunOutcome::Interrupted,
            follow_up: false,
            durable_cancellation: true,
        },
        FinishCase {
            name: "durable_cancel_interrupted_follow_up",
            outcome: ScanRunOutcome::Interrupted,
            follow_up: true,
            durable_cancellation: true,
        },
    ];

    for case in cases {
        for (budget_index, max_attempts) in [DEFAULT_SCAN_MAX_ATTEMPTS, 1].into_iter().enumerate() {
            let directory = TestDirectory::new();
            let mut database = Database::open(directory.path()).unwrap();
            let root = database
                .add_scan_root("Projects", "C:\\Music\\Projects")
                .unwrap();
            database
                .begin_scan_session("finish-matrix-session", 1)
                .unwrap();
            let queued_at = 2;
            let job_id = database
                .enqueue_scan(&root.id, ScanKind::Manual, queued_at)
                .unwrap()
                .job_id;
            if max_attempts != DEFAULT_SCAN_MAX_ATTEMPTS {
                database
                    .connection
                    .execute(
                        "UPDATE scan_job SET max_attempts = ?1 WHERE id = ?2",
                        rusqlite::params![max_attempts, &job_id],
                    )
                    .unwrap();
            }
            let lease = database
                .lease_next_scan("finish-matrix-session", 3, 100)
                .unwrap()
                .unwrap();
            if case.follow_up {
                assert!(
                    database
                        .enqueue_scan(&root.id, ScanKind::Periodic, 4)
                        .unwrap()
                        .coalesced,
                    "{} budget {budget_index} must coalesce",
                    case.name
                );
            }
            if case.durable_cancellation {
                database
                    .request_scan_cancellation(&lease.run.id, 5)
                    .unwrap();
            }

            let cancellation_wins =
                case.durable_cancellation || case.outcome == ScanRunOutcome::Cancelled;
            let follow_up_invalidates = case.follow_up && !cancellation_wins;
            let expected_outcome = if cancellation_wins {
                ScanRunOutcome::Cancelled
            } else if follow_up_invalidates {
                ScanRunOutcome::Interrupted
            } else {
                case.outcome
            };
            let expected_state = match expected_outcome {
                ScanRunOutcome::Completed => ScanRunState::Completed,
                ScanRunOutcome::Failed => ScanRunState::Failed,
                ScanRunOutcome::Cancelled => ScanRunState::Cancelled,
                ScanRunOutcome::Interrupted => ScanRunState::Interrupted,
            };
            let expected_job_state = match expected_state {
                ScanRunState::Completed => ScanJobState::Completed,
                ScanRunState::Failed => ScanJobState::Failed,
                ScanRunState::Cancelled => ScanJobState::Cancelled,
                ScanRunState::Interrupted => ScanJobState::Interrupted,
                ScanRunState::Running => ScanJobState::Running,
            };
            assert_eq!(
                if expected_outcome == ScanRunOutcome::Completed {
                    publish_empty_scan(&mut database, &lease, 6);
                    ScanRunState::Completed
                } else {
                    database
                        .finish_scan_run(
                            &lease.run.id,
                            "finish-matrix-session",
                            &lease.run.lease_token,
                            6,
                            case.outcome,
                        )
                        .unwrap()
                },
                expected_state,
                "{} budget {budget_index}",
                case.name
            );

            let run = database.scan_run(&lease.run.id).unwrap();
            assert_eq!(run.state, expected_state);
            assert_eq!(run.outcome, Some(expected_outcome));
            assert_eq!(run.attempt, 1);
            assert_eq!(run.retry_chain_id, lease.job.retry_chain_id);
            assert_eq!(run.cancellation_requested, cancellation_wins);
            let jobs = database.list_scan_jobs().unwrap();
            let queued_jobs = jobs
                .iter()
                .filter(|job| job.state == ScanJobState::Queued)
                .collect::<Vec<_>>();
            assert_eq!(
                jobs.len(),
                if follow_up_invalidates { 2 } else { 1 },
                "{} budget {budget_index} total jobs",
                case.name
            );
            assert_eq!(
                queued_jobs.len(),
                usize::from(follow_up_invalidates),
                "{} budget {budget_index} queued jobs",
                case.name
            );
            let original = database.scan_job(&job_id).unwrap();
            assert_eq!(original.state, expected_job_state);
            assert_eq!(original.retry_chain_id, lease.job.retry_chain_id);
            assert_eq!(original.attempt, 1);
            assert_eq!(original.not_before_ms, queued_at);
            assert_eq!(original.cancellation_requested, cancellation_wins);

            if follow_up_invalidates {
                let follow_up = queued_jobs[0];
                assert_ne!(follow_up.id, job_id);
                assert_ne!(follow_up.retry_chain_id, lease.job.retry_chain_id);
                assert_eq!(follow_up.attempt, 0);
                assert_eq!(follow_up.not_before_ms, 6);
                let resumed = database
                    .lease_next_scan("finish-matrix-session", follow_up.not_before_ms, 100)
                    .unwrap()
                    .unwrap();
                assert_eq!(resumed.job.id, follow_up.id);
                assert_eq!(resumed.job.attempt, 1);
                assert!(resumed.run.generation > lease.run.generation);
                publish_empty_scan(&mut database, &resumed, 7);
                assert_eq!(
                    database.scan_run(&resumed.run.id).unwrap().state,
                    ScanRunState::Completed
                );
            }
        }
    }
}

#[test]
fn worker_cancelled_outcome_suppresses_followup_on_restart_and_backup_recovery() {
    let source_directory = TestDirectory::new();
    let (backup, root_id, run_id, job_id, chain) = {
        let mut database = Database::open(source_directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database.begin_scan_session("session-1", 1).unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap();
        let lease = database
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Periodic, 4)
            .unwrap();
        assert_eq!(
            database
                .finish_scan_run(
                    &lease.run.id,
                    "session-1",
                    &lease.run.lease_token,
                    5,
                    ScanRunOutcome::Cancelled,
                )
                .unwrap(),
            ScanRunState::Cancelled
        );
        let run = database.scan_run(&lease.run.id).unwrap();
        assert!(run.cancellation_requested);
        let job = database.scan_job(&lease.job.id).unwrap();
        assert_eq!(job.state, ScanJobState::Cancelled);
        assert!(job.cancellation_requested);
        assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
        (
            database.create_backup().unwrap(),
            root.id,
            run.id,
            job.id,
            job.retry_chain_id,
        )
    };

    let mut restarted = Database::open(source_directory.path()).unwrap();
    restarted.begin_scan_session("session-2", 6).unwrap();
    assert_eq!(restarted.list_scan_jobs().unwrap().len(), 1);
    assert_eq!(
        restarted.scan_job(&job_id).unwrap().state,
        ScanJobState::Cancelled
    );
    assert!(
        restarted
            .lease_next_scan("session-2", 6, 100)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        restarted.scan_run(&run_id).unwrap().state,
        ScanRunState::Cancelled
    );
    let explicit = restarted
        .enqueue_scan(&root_id, ScanKind::Manual, 7)
        .unwrap();
    assert!(!explicit.coalesced);
    assert_ne!(explicit.job_id, job_id);
    let explicit_lease = restarted
        .lease_next_scan("session-2", 7, 100)
        .unwrap()
        .unwrap();
    assert_eq!(explicit_lease.job.id, explicit.job_id);
    assert_eq!(explicit_lease.job.attempt, 1);
    assert_ne!(explicit_lease.job.retry_chain_id, chain);

    let destination = TestDirectory::new();
    let mut recovered = Database::recover_to(&backup, destination.path()).unwrap();
    recovered
        .begin_scan_session("recovered-session", wall_clock_ms())
        .unwrap();
    assert_eq!(recovered.list_scan_jobs().unwrap().len(), 1);
    assert!(
        recovered
            .lease_next_scan("recovered-session", wall_clock_ms(), 100)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        recovered.scan_run(&run_id).unwrap().state,
        ScanRunState::Cancelled
    );
}

#[test]
fn exhausted_worker_failure_chain_is_not_recovered_by_diagnostic() {
    let directory = TestDirectory::new();
    let (root_id, job_id, run_id, chain) = {
        let mut database = Database::open(directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database.begin_scan_session("session-1", 1).unwrap();
        let job_id = database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap()
            .job_id;
        database
            .connection
            .execute(
                "UPDATE scan_job SET max_attempts = 1 WHERE id = ?1",
                [&job_id],
            )
            .unwrap();
        let lease = database
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        assert_eq!(lease.job.attempt, 1);
        database
            .finish_scan_run(
                &lease.run.id,
                "session-1",
                &lease.run.lease_token,
                4,
                ScanRunOutcome::Failed,
            )
            .unwrap();
        let job = database.scan_job(&job_id).unwrap();
        assert_eq!(job.state, ScanJobState::Failed);
        assert_eq!(job.attempt, 1);
        assert_eq!(job.max_attempts, 1);
        assert_eq!(job.last_error_code.as_deref(), Some("worker_failed"));
        (root.id, job.id, lease.run.id, job.retry_chain_id)
    };

    let mut database = Database::open(directory.path()).unwrap();
    database.begin_scan_session("session-2", 5).unwrap();
    let jobs = database.list_scan_jobs().unwrap();
    assert_eq!(jobs.len(), 1);
    let exhausted = database.scan_job(&job_id).unwrap();
    assert_eq!(exhausted.state, ScanJobState::Failed);
    assert_eq!(exhausted.retry_chain_id, chain);
    assert_eq!(exhausted.attempt, 1);
    assert_eq!(exhausted.max_attempts, 1);
    assert_eq!(exhausted.last_error_code.as_deref(), Some("worker_failed"));
    assert!(
        database
            .lease_next_scan("session-2", 5, 100)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        database.scan_run(&run_id).unwrap().state,
        ScanRunState::Failed
    );
    let explicit = database
        .enqueue_scan(&root_id, ScanKind::Manual, 6)
        .unwrap();
    assert!(!explicit.coalesced);
    assert_ne!(explicit.job_id, job_id);
    let explicit_lease = database
        .lease_next_scan("session-2", 6, 100)
        .unwrap()
        .unwrap();
    assert_eq!(explicit_lease.job.id, explicit.job_id);
    assert_eq!(explicit_lease.job.attempt, 1);
    assert_ne!(explicit_lease.job.retry_chain_id, chain);
}

#[test]
fn execution_recovery_transition_matrix_covers_restart_and_backup() {
    #[derive(Clone, Copy)]
    enum RecoveryPath {
        Restart,
        Backup,
    }

    #[derive(Clone, Copy)]
    struct RecoveryCase {
        name: &'static str,
        path: RecoveryPath,
        max_attempts: i64,
    }

    // An active attempt is recovered through both durable boundaries, with a
    // remaining budget and at the exhausted boundary. Recovery must preserve
    // one chain and its current attempt rather than creating a fresh request.
    let cases = [
        RecoveryCase {
            name: "restart_remaining_budget",
            path: RecoveryPath::Restart,
            max_attempts: DEFAULT_SCAN_MAX_ATTEMPTS,
        },
        RecoveryCase {
            name: "restart_exhausted_budget",
            path: RecoveryPath::Restart,
            max_attempts: 1,
        },
        RecoveryCase {
            name: "backup_remaining_budget",
            path: RecoveryPath::Backup,
            max_attempts: DEFAULT_SCAN_MAX_ATTEMPTS,
        },
        RecoveryCase {
            name: "backup_exhausted_budget",
            path: RecoveryPath::Backup,
            max_attempts: 1,
        },
    ];

    for case in cases {
        let source_directory = TestDirectory::new();
        let (backup, run_id, job_id, chain, first_generation) = {
            let mut database = Database::open(source_directory.path()).unwrap();
            let root = database
                .add_scan_root("Projects", "C:\\Music\\Projects")
                .unwrap();
            database.begin_scan_session("source-session", 1).unwrap();
            let job_id = database
                .enqueue_scan(&root.id, ScanKind::Manual, 2)
                .unwrap()
                .job_id;
            if case.max_attempts != DEFAULT_SCAN_MAX_ATTEMPTS {
                database
                    .connection
                    .execute(
                        "UPDATE scan_job SET max_attempts = ?1 WHERE id = ?2",
                        rusqlite::params![case.max_attempts, &job_id],
                    )
                    .unwrap();
            }
            let lease = database
                .lease_next_scan("source-session", 3, 100)
                .unwrap()
                .unwrap();
            let backup = match case.path {
                RecoveryPath::Restart => None,
                RecoveryPath::Backup => Some(database.create_backup().unwrap()),
            };
            (
                backup,
                lease.run.id,
                job_id,
                lease.job.retry_chain_id,
                lease.run.generation,
            )
        };

        let destination_directory = match case.path {
            RecoveryPath::Restart => None,
            RecoveryPath::Backup => Some(TestDirectory::new()),
        };
        let (mut database, backup_before, backup_after) = match case.path {
            RecoveryPath::Restart => (Database::open(source_directory.path()).unwrap(), None, None),
            RecoveryPath::Backup => {
                let before = wall_clock_ms();
                let database = Database::recover_to(
                    backup.as_ref().unwrap(),
                    destination_directory.as_ref().unwrap().path(),
                )
                .unwrap();
                let after = wall_clock_ms();
                (database, Some(before), Some(after))
            }
        };
        let session_now = match case.path {
            RecoveryPath::Restart => 20,
            RecoveryPath::Backup => wall_clock_ms(),
        };
        database
            .begin_scan_session("recovery-matrix-session", session_now)
            .unwrap();

        let run = database.scan_run(&run_id).unwrap();
        assert_eq!(
            run.state,
            ScanRunState::Interrupted,
            "{} run state",
            case.name
        );
        assert_eq!(run.outcome, Some(ScanRunOutcome::Interrupted));
        assert_eq!(run.retry_chain_id, chain);
        assert_eq!(run.attempt, 1);
        let jobs = database.list_scan_jobs().unwrap();
        let queued_jobs = jobs
            .iter()
            .filter(|job| job.state == ScanJobState::Queued)
            .collect::<Vec<_>>();
        let expected_requeued = case.max_attempts > 1;
        assert_eq!(jobs.len(), 1, "{} total jobs", case.name);
        assert_eq!(
            queued_jobs.len(),
            usize::from(expected_requeued),
            "{} queued jobs",
            case.name
        );
        let recovered_job = database.scan_job(&job_id).unwrap();
        assert_eq!(recovered_job.retry_chain_id, chain);
        assert_eq!(recovered_job.attempt, 1);
        assert_eq!(recovered_job.max_attempts, case.max_attempts);

        if expected_requeued {
            assert_eq!(recovered_job.state, ScanJobState::Queued);
            if let RecoveryPath::Restart = case.path {
                assert_eq!(recovered_job.not_before_ms, session_now + 1_000);
            }
            if let (RecoveryPath::Backup, Some(before), Some(after)) =
                (case.path, backup_before, backup_after)
            {
                assert!(recovered_job.not_before_ms >= before + 1_000);
                assert!(recovered_job.not_before_ms <= after + 1_000);
            }
            assert!(
                database
                    .lease_next_scan(
                        "recovery-matrix-session",
                        recovered_job.not_before_ms - 1,
                        100,
                    )
                    .unwrap()
                    .is_none(),
                "{} must wait for its persisted eligibility time",
                case.name
            );
            let resumed = database
                .lease_next_scan("recovery-matrix-session", recovered_job.not_before_ms, 100)
                .unwrap()
                .unwrap();
            assert_eq!(resumed.job.id, job_id);
            assert_eq!(resumed.job.retry_chain_id, chain);
            assert_eq!(resumed.job.attempt, 2);
            assert!(resumed.run.generation > first_generation);
        } else {
            assert_eq!(recovered_job.state, ScanJobState::Failed);
            assert_eq!(
                recovered_job.last_error_code.as_deref(),
                Some("retry_exhausted")
            );
            assert!(
                database
                    .lease_next_scan("recovery-matrix-session", session_now, 100)
                    .unwrap()
                    .is_none(),
                "{} must not create implicit work",
                case.name
            );
        }
    }
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
    publish_empty_scan(&mut database, &lease, 5);
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
    assert!(second.run.generation > first.run.generation);
    publish_empty_scan(&mut database, &second, 7);
    assert_eq!(
        database.scan_run(&second.run.id).unwrap().state,
        ScanRunState::Completed
    );
    database
        .enqueue_scan(&root.id, ScanKind::Manual, 8)
        .unwrap();
    let third = database
        .lease_next_scan("session-1", 9, 100)
        .unwrap()
        .unwrap();
    assert_eq!(third.job.attempt, 1);
    assert!(third.run.generation > second.run.generation);
    publish_empty_scan(&mut database, &third, 10);
    assert_eq!(
        database.scan_run(&third.run.id).unwrap().state,
        ScanRunState::Completed
    );
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
fn restart_requeues_the_interrupted_attempt_without_resetting_its_chain() {
    let directory = TestDirectory::new();
    let root = {
        let mut database = Database::open(directory.path()).unwrap();
        database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap()
    };
    let (run_id, token, job_id, chain, first_generation) = {
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
        (
            lease.run.id,
            lease.run.lease_token,
            job.job_id,
            lease.job.retry_chain_id,
            lease.run.generation,
        )
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
    let requeued = database.scan_job(&job_id).unwrap();
    assert_eq!(requeued.state, ScanJobState::Queued);
    assert_eq!(requeued.retry_chain_id, chain);
    assert_eq!(requeued.attempt, 1);
    assert_eq!(requeued.not_before_ms, 1_020);
    let recovery = database
        .lease_next_scan("session-b", 1_020, 100)
        .unwrap()
        .unwrap();
    assert_eq!(recovery.job.id, job_id);
    assert_eq!(recovery.job.kind, ScanKind::Manual);
    assert_eq!(recovery.job.retry_chain_id, chain);
    assert_eq!(recovery.job.attempt, 2);
    assert!(recovery.run.generation > first_generation);
    assert_eq!(recovery.root.id, root.id);
}

#[test]
fn restart_adds_recovery_only_for_roots_without_eligible_work() {
    let directory = TestDirectory::new();
    let (active_root, active_job) = {
        let mut database = Database::open(directory.path()).unwrap();
        let active_root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        let _idle_root = database.add_scan_root("Loops", "C:\\Music\\Loops").unwrap();
        database.begin_scan_session("session-1", 1).unwrap();
        let active_job = database
            .enqueue_scan(&active_root.id, ScanKind::Manual, 2)
            .unwrap();
        let lease = database
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        assert_eq!(lease.root.id, active_root.id);
        (active_root, active_job)
    };

    let mut database = Database::open(directory.path()).unwrap();
    database.begin_scan_session("session-2", 20).unwrap();
    let jobs = database.list_scan_jobs().unwrap();
    assert_eq!(jobs.len(), 2);
    let active = database.scan_job(&active_job.job_id).unwrap();
    assert_eq!(active.state, ScanJobState::Queued);
    assert_eq!(active.kind, ScanKind::Manual);
    let recovery = jobs.iter().find(|job| job.id != active_job.job_id).unwrap();
    assert_eq!(
        recovery.scan_root_id,
        database.list_scan_roots().unwrap()[1].id
    );
    assert_eq!(recovery.kind, ScanKind::Recovery);
    assert_eq!(recovery.state, ScanJobState::Queued);
    assert_eq!(active.scan_root_id, active_root.id);
}

#[test]
fn repeated_active_restarts_exhaust_one_persisted_retry_chain() {
    let directory = TestDirectory::new();
    let (root_id, job_id, mut previous_run_id, mut previous_token, chain, mut previous_generation) = {
        let mut database = Database::open(directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database.begin_scan_session("session-1", 1).unwrap();
        let job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap();
        let lease = database
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        (
            root.id,
            job.job_id,
            lease.run.id,
            lease.run.lease_token,
            lease.job.retry_chain_id,
            lease.run.generation,
        )
    };

    let mut restart_at = 10;
    for session_number in 2..=4 {
        let session_id = format!("session-{session_number}");
        let mut database = Database::open(directory.path()).unwrap();
        database
            .begin_scan_session(&session_id, restart_at)
            .unwrap();
        assert_eq!(
            database.scan_run(&previous_run_id).unwrap().state,
            ScanRunState::Interrupted
        );
        let queued = database.scan_job(&job_id).unwrap();
        assert_eq!(queued.state, ScanJobState::Queued);
        assert_eq!(queued.retry_chain_id, chain);
        assert_eq!(queued.attempt, session_number as i64 - 1);
        assert_eq!(queued.max_attempts, DEFAULT_SCAN_MAX_ATTEMPTS);
        let lease_at = queued.not_before_ms;
        let lease = database
            .lease_next_scan(&session_id, lease_at, 100)
            .unwrap()
            .unwrap();
        assert_eq!(lease.root.id, root_id);
        assert_eq!(lease.job.id, job_id);
        assert_eq!(lease.job.retry_chain_id, chain);
        assert_eq!(lease.job.attempt, session_number as i64);
        assert!(lease.run.generation > previous_generation);
        previous_run_id = lease.run.id;
        previous_token = lease.run.lease_token;
        previous_generation = lease.run.generation;
        restart_at = lease_at + 1;
    }

    let mut database = Database::open(directory.path()).unwrap();
    database
        .begin_scan_session("session-6", restart_at)
        .unwrap();
    assert_eq!(
        database.scan_run(&previous_run_id).unwrap().state,
        ScanRunState::Interrupted
    );
    let exhausted = database.scan_job(&job_id).unwrap();
    assert_eq!(exhausted.state, ScanJobState::Failed);
    assert_eq!(exhausted.attempt, DEFAULT_SCAN_MAX_ATTEMPTS);
    assert_eq!(exhausted.retry_chain_id, chain);
    assert_eq!(
        exhausted.last_error_code.as_deref(),
        Some("retry_exhausted")
    );
    assert!(
        database
            .lease_next_scan("session-6", restart_at, 100)
            .unwrap()
            .is_none()
    );
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
    let terminal_before = database.scan_run(&previous_run_id).unwrap();
    assert_eq!(
        database
            .request_scan_cancellation(&previous_run_id, restart_at + 1)
            .unwrap(),
        ScanRunState::Interrupted
    );
    assert_eq!(
        database.scan_run(&previous_run_id).unwrap(),
        terminal_before
    );
    assert!(matches!(
        database.finish_scan_run(
            &previous_run_id,
            "session-4",
            &previous_token,
            restart_at + 2,
            ScanRunOutcome::Completed,
        ),
        Err(StorageError::Conflict)
    ));
}

#[test]
fn pending_follow_up_survives_restart_and_invalidates_the_resumed_attempt() {
    let directory = TestDirectory::new();
    let (root_id, job_id, chain) = {
        let mut database = Database::open(directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database.begin_scan_session("session-1", 1).unwrap();
        let job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap();
        let lease = database
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Periodic, 4)
            .unwrap();
        (root.id, job.job_id, lease.job.retry_chain_id)
    };

    let mut database = Database::open(directory.path()).unwrap();
    database.begin_scan_session("session-2", 20).unwrap();
    let queued = database.scan_job(&job_id).unwrap();
    assert_eq!(queued.state, ScanJobState::Queued);
    assert!(queued.follow_up_requested);
    assert_eq!(queued.retry_chain_id, chain);
    let resumed = database
        .lease_next_scan("session-2", queued.not_before_ms, 100)
        .unwrap()
        .unwrap();
    assert_eq!(resumed.root.id, root_id);
    assert_eq!(resumed.job.id, job_id);
    assert_eq!(resumed.job.attempt, 2);
    assert!(resumed.job.follow_up_requested);
    assert_eq!(
        database
            .finish_scan_run(
                &resumed.run.id,
                "session-2",
                &resumed.run.lease_token,
                queued.not_before_ms + 1,
                ScanRunOutcome::Completed,
            )
            .unwrap(),
        ScanRunState::Interrupted
    );
    let jobs = database.list_scan_jobs().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].state, ScanJobState::Interrupted);
    assert_eq!(jobs[1].state, ScanJobState::Queued);
    assert_eq!(jobs[1].kind, ScanKind::Manual);
    assert_ne!(jobs[1].retry_chain_id, chain);
}

#[test]
fn pending_follow_up_is_suppressed_by_cancellation_disable_and_removal() {
    {
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
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Periodic, 4)
            .unwrap();
        database
            .request_scan_cancellation(&lease.run.id, 5)
            .unwrap();
        assert_eq!(
            database
                .finish_scan_run(
                    &lease.run.id,
                    "session-1",
                    &lease.run.lease_token,
                    6,
                    ScanRunOutcome::Completed,
                )
                .unwrap(),
            ScanRunState::Cancelled
        );
        assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
    }
    {
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
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Periodic, 4)
            .unwrap();
        database
            .set_scan_root_enabled_at(&root.id, false, 5)
            .unwrap();
        assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
        assert_eq!(
            database.scan_run(&lease.run.id).unwrap().state,
            ScanRunState::Cancelled
        );
    }
    {
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
            .lease_next_scan("session-1", 3, 100)
            .unwrap()
            .unwrap();
        database
            .enqueue_scan(&root.id, ScanKind::Periodic, 4)
            .unwrap();
        database.remove_scan_root_at(&root.id, 5).unwrap();
        assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
        assert_eq!(
            database.scan_run(&lease.run.id).unwrap().state,
            ScanRunState::Cancelled
        );
    }
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
    publish_empty_scan(&mut database, &completed, 9);
    assert_eq!(
        database.scan_run(&completed.run.id).unwrap().state,
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
        (1, 2)
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
    assert_eq!((current.configuration_revision, current.generation), (2, 3));

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
    assert_eq!(database.schema_version().unwrap(), 5);
    assert_eq!(database.startup_view().unwrap(), StartupView::Board);
    let root = &database.list_scan_roots().unwrap()[0];
    let execution = database.scan_root_execution(&root.id).unwrap();
    assert_eq!(
        (execution.configuration_revision, execution.generation),
        (0, 0)
    );
}

#[test]
fn staging_is_invisible_until_atomic_publication_and_tracks_aliases() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("publication-session", 1)
        .unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("publication-session", 3, 100)
        .unwrap()
        .unwrap();
    // Completing with no stage row is "no open stage" (`NotFound`), distinct
    // from the stale-lease `Conflict`; either way no completed row precedes
    // publication.
    assert!(matches!(
        database.finish_scan_run(
            &lease.run.id,
            "publication-session",
            &lease.run.lease_token,
            4,
            ScanRunOutcome::Completed,
        ),
        Err(StorageError::NotFound)
    ));
    let observations = [
        staged_observation("a.flp", 10, 20, Some("file-1")),
        staged_observation("b.flp", 10, 20, Some("file-1")),
    ];
    let staging = database
        .stage_scan_observations(
            &lease.run.id,
            "publication-session",
            &lease.run.lease_token,
            4,
            &observations,
        )
        .unwrap();
    assert_eq!(staging.state, ScanStageState::Open);
    assert_eq!(staging.record_count, 2);
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        database.scan_run(&lease.run.id).unwrap().state,
        ScanRunState::Running
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Running
    );

    let publication = database
        .publish_scan_run(
            &lease.run.id,
            "publication-session",
            &lease.run.lease_token,
            5,
        )
        .unwrap();
    assert_eq!(publication.location_count, 2);
    assert_eq!(
        database.scan_run(&lease.run.id).unwrap().state,
        ScanRunState::Completed
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    let locations = database.list_published_locations(&root.id).unwrap();
    assert_eq!(locations.len(), 2);
    let marker = database.scan_root_publication(&root.id).unwrap();
    assert!(
        locations
            .iter()
            .all(|location| location.presence == FilePresence::Present)
    );
    assert_eq!(locations[0].project_file_id, locations[1].project_file_id);
    assert_eq!(
        marker.last_successful_run_id.as_deref(),
        Some(lease.run.id.as_str())
    );
    assert_eq!(
        marker.last_successful_generation,
        Some(lease.run.generation)
    );
    assert_eq!(marker.last_successful_at_ms, Some(5));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Published
    );
}

#[test]
fn publication_marks_missing_and_restores_each_alias_without_collapsing_it() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("presence-session", 1).unwrap();

    let _first_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("presence-session", 3, 100)
        .unwrap()
        .unwrap();
    let initial = [
        staged_observation("a.flp", 10, 20, Some("same")),
        staged_observation("b.flp", 10, 20, Some("same")),
    ];
    database
        .stage_scan_observations(
            &first.run.id,
            "presence-session",
            &first.run.lease_token,
            4,
            &initial,
        )
        .unwrap();
    database
        .publish_scan_run(&first.run.id, "presence-session", &first.run.lease_token, 5)
        .unwrap();
    let initial_locations = database.list_published_locations(&root.id).unwrap();
    let shared_file = initial_locations[0].project_file_id.clone();

    let second_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let second = database
        .lease_next_scan("presence-session", 7, 100)
        .unwrap()
        .unwrap();
    assert_eq!(second.job.id, second_job.job_id);
    database
        .stage_scan_observations(
            &second.run.id,
            "presence-session",
            &second.run.lease_token,
            8,
            &[staged_observation("b.flp", 10, 20, Some("same"))],
        )
        .unwrap();
    database
        .publish_scan_run(
            &second.run.id,
            "presence-session",
            &second.run.lease_token,
            9,
        )
        .unwrap();
    let missing = database.list_published_locations(&root.id).unwrap();
    assert_eq!(missing.len(), 2);
    let a = missing
        .iter()
        .find(|location| location.locator_key == v1_key("a.flp"))
        .unwrap();
    let b = missing
        .iter()
        .find(|location| location.locator_key == v1_key("b.flp"))
        .unwrap();
    assert_eq!(a.presence, FilePresence::Missing);
    assert_eq!(b.presence, FilePresence::Present);
    assert_eq!(b.project_file_id, shared_file);

    let third_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 10)
        .unwrap();
    let third = database
        .lease_next_scan("presence-session", 11, 100)
        .unwrap()
        .unwrap();
    assert_eq!(third.job.id, third_job.job_id);
    let restored = [
        staged_observation("a.flp", 10, 20, Some("same")),
        staged_observation("b.flp", 10, 20, Some("same")),
    ];
    database
        .stage_scan_observations(
            &third.run.id,
            "presence-session",
            &third.run.lease_token,
            12,
            &restored,
        )
        .unwrap();
    database
        .publish_scan_run(
            &third.run.id,
            "presence-session",
            &third.run.lease_token,
            13,
        )
        .unwrap();
    let restored_locations = database.list_published_locations(&root.id).unwrap();
    assert!(
        restored_locations
            .iter()
            .all(|location| location.presence == FilePresence::Present)
    );
    assert!(
        restored_locations
            .iter()
            .all(|location| location.project_file_id == shared_file)
    );
}

#[test]
fn historical_missing_identity_is_not_reused_by_a_new_path() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("historical-session", 1)
        .unwrap();

    let first_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("historical-session", 3, 100)
        .unwrap()
        .unwrap();
    assert_eq!(first.job.id, first_job.job_id);
    publish_observations(
        &mut database,
        &first,
        4,
        &[staged_observation("old.flp", 1, 1, Some("identity-x"))],
    );
    let old = database
        .list_published_locations(&root.id)
        .unwrap()
        .into_iter()
        .find(|location| location.locator_key == v1_key("old.flp"))
        .unwrap();

    let empty_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let empty = database
        .lease_next_scan("historical-session", 7, 100)
        .unwrap()
        .unwrap();
    assert_eq!(empty.job.id, empty_job.job_id);
    publish_empty_scan(&mut database, &empty, 8);
    assert_eq!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .iter()
            .find(|location| location.locator_key == v1_key("old.flp"))
            .unwrap()
            .presence,
        FilePresence::Missing
    );

    let replacement_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 10)
        .unwrap();
    let replacement = database
        .lease_next_scan("historical-session", 11, 100)
        .unwrap()
        .unwrap();
    assert_eq!(replacement.job.id, replacement_job.job_id);
    publish_observations(
        &mut database,
        &replacement,
        12,
        &[staged_observation("new.flp", 1, 1, Some("identity-x"))],
    );

    let locations = database.list_published_locations(&root.id).unwrap();
    let new = locations
        .iter()
        .find(|location| location.locator_key == v1_key("new.flp"))
        .unwrap();
    assert_eq!(new.presence, FilePresence::Present);
    assert_ne!(new.project_file_id, old.project_file_id);
    assert_eq!(
        locations
            .iter()
            .find(|location| location.locator_key == v1_key("old.flp"))
            .unwrap()
            .project_file_id,
        old.project_file_id
    );
}

#[test]
fn same_scan_rename_replacement_and_surviving_aliases_are_order_independent() {
    fn run_scan(observations: &[ScanObservation]) -> Vec<PublishedLocation> {
        let directory = TestDirectory::new();
        let mut database = Database::open(directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database
            .begin_scan_session("association-session", 1)
            .unwrap();

        let initial_job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap();
        let initial = database
            .lease_next_scan("association-session", 3, 100)
            .unwrap()
            .unwrap();
        assert_eq!(initial.job.id, initial_job.job_id);
        publish_observations(
            &mut database,
            &initial,
            4,
            &[
                staged_observation("old.flp", 1, 1, Some("identity-x")),
                staged_observation("survivor.flp", 1, 1, Some("identity-x")),
                staged_observation("replace.flp", 1, 1, Some("identity-x")),
            ],
        );

        let next_job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 6)
            .unwrap();
        let next = database
            .lease_next_scan("association-session", 7, 100)
            .unwrap()
            .unwrap();
        assert_eq!(next.job.id, next_job.job_id);
        publish_observations(&mut database, &next, 8, observations);
        database.list_published_locations(&root.id).unwrap()
    }

    let ordered = [
        staged_observation("moved.flp", 1, 1, Some("identity-x")),
        staged_observation("replace.flp", 2, 2, Some("identity-y")),
        staged_observation("survivor.flp", 1, 1, Some("identity-x")),
    ];
    let reversed = [ordered[2].clone(), ordered[1].clone(), ordered[0].clone()];
    let first = run_scan(&ordered);
    let second = run_scan(&reversed);

    let project_for = |locations: &[PublishedLocation], path: &str| {
        locations
            .iter()
            .find(|location| location.locator_key == v1_key(path))
            .unwrap()
            .project_file_id
            .clone()
    };
    let old_project = project_for(&first, "old.flp");
    assert_eq!(
        first
            .iter()
            .find(|location| location.locator_key == v1_key("old.flp"))
            .unwrap()
            .presence,
        FilePresence::Missing
    );
    assert_eq!(project_for(&first, "moved.flp"), old_project);
    assert_eq!(project_for(&first, "survivor.flp"), old_project);
    assert_ne!(project_for(&first, "replace.flp"), old_project);

    let canonical = |locations: &[PublishedLocation]| {
        let mut project_groups = Vec::<String>::new();
        locations
            .iter()
            .map(|location| {
                let group = project_groups
                    .iter()
                    .position(|id| id == &location.project_file_id)
                    .unwrap_or_else(|| {
                        project_groups.push(location.project_file_id.clone());
                        project_groups.len() - 1
                    });
                (location.locator_key.clone(), location.presence, group)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(canonical(&first), canonical(&second));
}

#[test]
fn library_query_is_bounded_cursored_and_snapshot_stable() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("library-session", 1).unwrap();
    let first_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("library-session", 3, 100)
        .unwrap()
        .unwrap();
    assert_eq!(first.job.id, first_job.job_id);
    publish_observations(
        &mut database,
        &first,
        4,
        &[
            staged_observation("a.flp", 1, 1, None),
            staged_observation("b.flp", 1, 1, None),
            staged_observation("c.flp", 1, 1, None),
        ],
    );

    let page_one = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 2,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(
        page_one
            .locations
            .iter()
            .map(|location| location.locator_key.clone())
            .collect::<Vec<_>>(),
        vec![v1_key("a.flp"), v1_key("b.flp")]
    );
    assert!(page_one.has_more);
    let cursor = page_one.next_cursor.clone().unwrap();

    let page_two = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 2,
            cursor: Some(cursor),
            snapshot: Some(page_one.snapshot.clone()),
        })
        .unwrap();
    assert_eq!(page_two.locations.len(), 1);
    assert_eq!(page_two.locations[0].locator_key, v1_key("c.flp"));
    assert!(!page_two.has_more);
    assert!(page_two.next_cursor.is_some());

    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: MAX_LIBRARY_PAGE_SIZE + 1,
            cursor: None,
            snapshot: None,
        }),
        Err(StorageError::InvalidSchema)
    ));
    let mut stale_snapshot = page_one.snapshot.clone();
    stale_snapshot.last_successful_at_ms = Some(999);
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 2,
            cursor: None,
            snapshot: Some(stale_snapshot),
        }),
        Err(StorageError::StaleCursor)
    ));

    let next_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let next = database
        .lease_next_scan("library-session", 7, 100)
        .unwrap()
        .unwrap();
    assert_eq!(next.job.id, next_job.job_id);
    publish_observations(
        &mut database,
        &next,
        8,
        &[staged_observation("d.flp", 1, 1, None)],
    );
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root.id,
            page_size: 2,
            cursor: Some(page_one.next_cursor.unwrap()),
            snapshot: Some(page_one.snapshot),
        }),
        Err(StorageError::StaleCursor)
    ));
}

#[test]
fn library_cursor_survives_failed_and_cancelled_runs() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("cursor-outcome-session", 1)
        .unwrap();

    let initial_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let initial = database
        .lease_next_scan("cursor-outcome-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &initial,
        4,
        &[
            staged_observation("a.flp", 1, 1, None),
            staged_observation("b.flp", 1, 1, None),
        ],
    );
    assert_eq!(
        database.scan_job(&initial_job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    let first_page = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 1,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    let cursor = first_page.next_cursor.clone().unwrap();
    let snapshot = first_page.snapshot.clone();

    let failed_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let failed = database
        .lease_next_scan("cursor-outcome-session", 7, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &failed.run.id,
            "cursor-outcome-session",
            &failed.run.lease_token,
            8,
            &[staged_observation("failed.flp", 1, 1, None)],
        )
        .unwrap();
    assert_eq!(
        database
            .finish_scan_run(
                &failed.run.id,
                "cursor-outcome-session",
                &failed.run.lease_token,
                9,
                ScanRunOutcome::Failed,
            )
            .unwrap(),
        ScanRunState::Failed
    );
    assert_eq!(
        database.scan_job(&failed_job.job_id).unwrap().state,
        ScanJobState::Failed
    );
    let after_failed = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 1,
            cursor: Some(cursor.clone()),
            snapshot: Some(snapshot.clone()),
        })
        .unwrap();
    assert_eq!(after_failed.snapshot, snapshot);
    assert_eq!(after_failed.locations.len(), 1);
    assert_eq!(after_failed.locations[0].locator_key, v1_key("b.flp"));
    assert!(!after_failed.has_more);

    let cancelled_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 10)
        .unwrap();
    let cancelled = database
        .lease_next_scan("cursor-outcome-session", 11, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &cancelled.run.id,
            "cursor-outcome-session",
            &cancelled.run.lease_token,
            12,
            &[staged_observation("cancelled.flp", 1, 1, None)],
        )
        .unwrap();
    assert_eq!(
        database
            .request_scan_cancellation(&cancelled.run.id, 13)
            .unwrap(),
        ScanRunState::Running
    );
    assert_eq!(
        database
            .finish_scan_run(
                &cancelled.run.id,
                "cursor-outcome-session",
                &cancelled.run.lease_token,
                14,
                ScanRunOutcome::Cancelled,
            )
            .unwrap(),
        ScanRunState::Cancelled
    );
    assert_eq!(
        database.scan_job(&cancelled_job.job_id).unwrap().state,
        ScanJobState::Cancelled
    );
    let after_cancelled = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id,
            page_size: 1,
            cursor: Some(cursor),
            snapshot: Some(after_failed.snapshot.clone()),
        })
        .unwrap();
    assert_eq!(after_cancelled.snapshot, after_failed.snapshot);
    assert_eq!(after_cancelled.locations.len(), 1);
    assert_eq!(after_cancelled.locations[0].locator_key, v1_key("b.flp"));
    assert!(!after_cancelled.has_more);
}

#[test]
fn publication_rejects_stale_or_cancelled_runs_without_touching_committed_rows() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("fence-session", 1).unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("fence-session", 3, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &lease.run.id,
            "fence-session",
            &lease.run.lease_token,
            4,
            &[staged_observation("stale.flp", 1, 1, None)],
        )
        .unwrap();
    assert!(matches!(
        database.publish_scan_run(&lease.run.id, "fence-session", "stale-token", 5),
        Err(StorageError::Conflict)
    ));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Open
    );
    database
        .request_scan_cancellation(&lease.run.id, 5)
        .unwrap();
    assert!(matches!(
        database.publish_scan_run(&lease.run.id, "fence-session", &lease.run.lease_token, 6),
        Err(StorageError::Conflict)
    ));
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Discarded
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Running
    );

    let next = database
        .enqueue_scan(&root.id, ScanKind::Manual, 7)
        .unwrap();
    assert!(next.coalesced);
    database.cancel_scan_job(&job.job_id, 8).unwrap();
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Running
    );
}

#[test]
fn publication_rolls_back_visible_rows_and_ledger_on_apply_failure() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("rollback-session", 1).unwrap();
    let first_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("rollback-session", 3, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &first.run.id,
            "rollback-session",
            &first.run.lease_token,
            4,
            &[staged_observation("old.flp", 1, 1, None)],
        )
        .unwrap();
    database
        .publish_scan_run(&first.run.id, "rollback-session", &first.run.lease_token, 5)
        .unwrap();
    let committed_before = database.list_published_locations(&root.id).unwrap();
    let marker_before = database.scan_root_publication(&root.id).unwrap();

    let second_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let second = database
        .lease_next_scan("rollback-session", 7, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &second.run.id,
            "rollback-session",
            &second.run.lease_token,
            8,
            &[staged_observation("new.flp", 2, 2, None)],
        )
        .unwrap();
    database
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_publication_project_file
             BEFORE INSERT ON project_file
             BEGIN SELECT RAISE(ABORT, 'injected publication failure'); END;",
        )
        .unwrap();
    assert!(matches!(
        database.publish_scan_run(
            &second.run.id,
            "rollback-session",
            &second.run.lease_token,
            9,
        ),
        Err(StorageError::Database)
    ));
    database
        .connection
        .execute_batch("DROP TRIGGER fail_publication_project_file;")
        .unwrap();
    assert_eq!(
        database.list_published_locations(&root.id).unwrap(),
        committed_before
    );
    assert_eq!(
        database.scan_root_publication(&root.id).unwrap(),
        marker_before
    );
    assert_eq!(
        database.scan_run(&second.run.id).unwrap().state,
        ScanRunState::Running
    );
    assert_eq!(
        database.scan_job(&second_job.job_id).unwrap().state,
        ScanJobState::Running
    );
    assert_eq!(
        database.scan_staging(&second.run.id).unwrap().state,
        ScanStageState::Open
    );
    database
        .publish_scan_run(
            &second.run.id,
            "rollback-session",
            &second.run.lease_token,
            10,
        )
        .unwrap();
    let committed_after = database.list_published_locations(&root.id).unwrap();
    assert_eq!(committed_after.len(), 2);
    assert!(
        committed_after
            .iter()
            .any(|location| location.locator_key == v1_key("old.flp"))
    );
    assert!(
        committed_after
            .iter()
            .any(|location| location.locator_key == v1_key("new.flp"))
    );
    assert_eq!(
        database.scan_job(&first_job.job_id).unwrap().state,
        ScanJobState::Completed
    );
}

#[test]
fn recovery_discards_open_staging_and_root_removal_detaches_history() {
    let source_directory = TestDirectory::new();
    let (backup, root_id, run_id) = {
        let mut database = Database::open(source_directory.path()).unwrap();
        let root = database
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        database.begin_scan_session("recovery-session", 1).unwrap();
        let _job = database
            .enqueue_scan(&root.id, ScanKind::Manual, 2)
            .unwrap();
        let lease = database
            .lease_next_scan("recovery-session", 3, 100)
            .unwrap()
            .unwrap();
        database
            .stage_scan_observations(
                &lease.run.id,
                "recovery-session",
                &lease.run.lease_token,
                4,
                &[staged_observation("history.flp", 1, 1, Some("history"))],
            )
            .unwrap();
        let backup = database.create_backup().unwrap();
        (backup, root.id, lease.run.id)
    };
    let recovered_directory = TestDirectory::new();
    let mut recovered = Database::recover_to(&backup, recovered_directory.path()).unwrap();
    assert_eq!(
        recovered.scan_staging(&run_id).unwrap().state,
        ScanStageState::Discarded
    );
    assert_eq!(
        recovered.scan_run(&run_id).unwrap().state,
        ScanRunState::Interrupted
    );
    assert!(
        recovered
            .list_published_locations(&root_id)
            .unwrap()
            .is_empty()
    );

    let root = recovered.list_scan_roots().unwrap().remove(0);
    recovered.begin_scan_session("detach-session", 20).unwrap();
    let job = recovered
        .enqueue_scan(&root.id, ScanKind::Manual, 21)
        .unwrap();
    let queued = recovered.scan_job(&job.job_id).unwrap();
    let lease = recovered
        .lease_next_scan("detach-session", queued.not_before_ms, 100)
        .unwrap()
        .unwrap();
    let staged_at = queued.not_before_ms + 1;
    recovered
        .stage_scan_observations(
            &lease.run.id,
            "detach-session",
            &lease.run.lease_token,
            staged_at,
            &[staged_observation("history.flp", 1, 1, Some("history"))],
        )
        .unwrap();
    recovered
        .publish_scan_run(
            &lease.run.id,
            "detach-session",
            &lease.run.lease_token,
            staged_at + 1,
        )
        .unwrap();
    recovered
        .remove_scan_root_at(&root.id, staged_at + 2)
        .unwrap();
    let detached = recovered.list_detached_locations().unwrap();
    assert_eq!(detached.len(), 1);
    assert_eq!(detached[0].scan_root_id, None);
    assert_eq!(
        detached[0].detached_scan_root_id.as_deref(),
        Some(root.id.as_str())
    );
    assert_eq!(detached[0].presence, FilePresence::Present);
    assert_eq!(
        recovered.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Completed
    );

    let replacement = recovered
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    assert_ne!(replacement.id, root.id);
    assert!(
        recovered
            .list_published_locations(&replacement.id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(recovered.list_detached_locations().unwrap().len(), 1);
}

#[test]
fn conflicting_duplicates_across_batches_invalidate_prior_staging() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("conflict-session", 1).unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("conflict-session", 3, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &lease.run.id,
            "conflict-session",
            &lease.run.lease_token,
            4,
            &[staged_observation("same.flp", 1, 1, Some("first"))],
        )
        .unwrap();

    assert!(matches!(
        database.stage_scan_observations(
            &lease.run.id,
            "conflict-session",
            &lease.run.lease_token,
            5,
            &[staged_observation("same.flp", 2, 1, Some("second"))],
        ),
        Err(StorageError::StagingRejected)
    ));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Discarded
    );
    assert_eq!(
        database
            .scan_run(&lease.run.id)
            .unwrap()
            .error_code
            .as_deref(),
        Some("staging_rejected")
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Failed
    );
    assert!(matches!(
        database.publish_scan_run(&lease.run.id, "conflict-session", &lease.run.lease_token, 6,),
        Err(StorageError::Conflict)
    ));
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn retryable_staging_sql_failure_keeps_the_prior_stage_open() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("retryable-stage-session", 1)
        .unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("retryable-stage-session", 3, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &lease.run.id,
            "retryable-stage-session",
            &lease.run.lease_token,
            4,
            &[staged_observation("first.flp", 1, 1, None)],
        )
        .unwrap();
    database
        .connection
        .execute_batch(
            "CREATE TRIGGER fail_stage_insert
             BEFORE INSERT ON scan_stage_observation
             BEGIN SELECT RAISE(ABORT, 'injected staging failure'); END;",
        )
        .unwrap();
    assert!(matches!(
        database.stage_scan_observations(
            &lease.run.id,
            "retryable-stage-session",
            &lease.run.lease_token,
            5,
            &[staged_observation("second.flp", 1, 1, None)],
        ),
        Err(StorageError::Database)
    ));
    database
        .connection
        .execute_batch("DROP TRIGGER fail_stage_insert;")
        .unwrap();
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Open
    );
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().record_count,
        1
    );
    database
        .stage_scan_observations(
            &lease.run.id,
            "retryable-stage-session",
            &lease.run.lease_token,
            6,
            &[staged_observation("second.flp", 1, 1, None)],
        )
        .unwrap();
    database
        .publish_scan_run(
            &lease.run.id,
            "retryable-stage-session",
            &lease.run.lease_token,
            7,
        )
        .unwrap();
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    assert_eq!(
        database.list_published_locations(&root.id).unwrap().len(),
        2
    );
}

#[test]
fn staging_batches_enforce_record_and_path_budgets_before_writing() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("quota-session", 1).unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("quota-session", 3, 100)
        .unwrap()
        .unwrap();

    let mut next_index = 0usize;
    for batch in 0..(MAX_STAGED_RECORDS as usize / MAX_STAGED_BATCH_RECORDS) {
        let observations: Vec<_> = (0..MAX_STAGED_BATCH_RECORDS)
            .map(|offset| {
                let index = next_index + offset;
                staged_observation(&format!("bounded-{batch}-{index}.flp"), 1, 1, None)
            })
            .collect();
        database
            .stage_scan_observations(
                &lease.run.id,
                "quota-session",
                &lease.run.lease_token,
                6 + batch as i64,
                &observations,
            )
            .unwrap();
        next_index += MAX_STAGED_BATCH_RECORDS;
    }
    let remainder = MAX_STAGED_RECORDS as usize % MAX_STAGED_BATCH_RECORDS;
    let observations: Vec<_> = (0..remainder)
        .map(|offset| {
            staged_observation(
                &format!("bounded-remainder-{}.flp", next_index + offset),
                1,
                1,
                None,
            )
        })
        .collect();
    database
        .stage_scan_observations(
            &lease.run.id,
            "quota-session",
            &lease.run.lease_token,
            100,
            &observations,
        )
        .unwrap();
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().record_count,
        MAX_STAGED_RECORDS
    );

    assert!(matches!(
        database.stage_scan_observations(
            &lease.run.id,
            "quota-session",
            &lease.run.lease_token,
            101,
            &[staged_observation("over-budget.flp", 1, 1, None)],
        ),
        Err(StorageError::StagingRejected)
    ));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Discarded
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Failed
    );
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn staging_path_budget_after_prior_batch_is_terminal() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("path-quota-session", 1)
        .unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("path-quota-session", 3, 100)
        .unwrap()
        .unwrap();

    let prefix = "p".repeat(32_000);
    let prior_batch: Vec<_> = (0..65)
        .map(|index| staged_observation(&format!("{prefix}-{index}"), 1, 1, None))
        .collect();
    database
        .stage_scan_observations(
            &lease.run.id,
            "path-quota-session",
            &lease.run.lease_token,
            4,
            &prior_batch,
        )
        .unwrap();
    assert!(database.scan_staging(&lease.run.id).unwrap().path_bytes < MAX_STAGED_PATH_BYTES);

    assert!(matches!(
        database.stage_scan_observations(
            &lease.run.id,
            "path-quota-session",
            &lease.run.lease_token,
            5,
            &[staged_observation(
                &format!("{prefix}-overflow"),
                1,
                1,
                None
            )],
        ),
        Err(StorageError::StagingRejected)
    ));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Discarded
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Failed
    );
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn integration_locator_keys_preserve_case_renames_and_case_sensitive_aliases() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("locator-session", 1).unwrap();

    let first_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("locator-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &first,
        4,
        &[exact_observation(
            "v1:i:projects/i:foo.flp",
            "Projects\\Foo.flp",
            10,
            1_000_000_001,
            Some(EncodedIdentity {
                volume_serial: "7".to_owned(),
                file_id: "9".to_owned(),
            }),
        )],
    );
    let before = database.list_published_locations(&root.id).unwrap();
    let before_id = before[0].id.clone();
    let before_project = before[0].project_file_id.clone();

    let second_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    assert_ne!(first_job.job_id, second_job.job_id);
    let second = database
        .lease_next_scan("locator-session", 7, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &second,
        8,
        &[exact_observation(
            "v1:i:projects/i:foo.flp",
            "Projects\\foo.flp",
            10,
            1_000_000_999,
            Some(EncodedIdentity {
                volume_serial: "7".to_owned(),
                file_id: "9".to_owned(),
            }),
        )],
    );
    let renamed = database.list_published_locations(&root.id).unwrap();
    assert_eq!(renamed.len(), 1);
    assert_eq!(renamed[0].id, before_id);
    assert_eq!(renamed[0].project_file_id, before_project);
    assert_eq!(renamed[0].relative_path, "Projects\\foo.flp");
    assert_eq!(renamed[0].presence, FilePresence::Present);

    // The storage boundary receives distinct keys for a Windows
    // case-sensitive directory; it does not infer that mode from spelling.
    let third_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 10)
        .unwrap();
    let third = database
        .lease_next_scan("locator-session", 11, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &third,
        12,
        &[
            exact_observation("v1:s:Projects/s:Foo.flp", "Projects\\Foo.flp", 1, 1, None),
            exact_observation("v1:s:Projects/s:foo.flp", "Projects\\foo.flp", 1, 1, None),
        ],
    );
    let locations = database.list_published_locations(&root.id).unwrap();
    assert_eq!(locations.len(), 3);
    assert_eq!(
        locations
            .iter()
            .map(|location| location.locator_key.as_str())
            .collect::<Vec<_>>(),
        [
            "v1:i:projects/i:foo.flp",
            "v1:s:Projects/s:Foo.flp",
            "v1:s:Projects/s:foo.flp"
        ]
    );
    assert_eq!(
        database.scan_job(&third_job.job_id).unwrap().state,
        ScanJobState::Completed
    );
}

#[test]
fn integration_identity_timestamp_and_numeric_bounds_are_checked() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("numeric-session", 1).unwrap();
    let maximum_identity = EncodedIdentity {
        volume_serial: u64::MAX.to_string(),
        file_id: u128::MAX.to_string(),
    };
    let encoded = serde_json::to_value(&maximum_identity).unwrap();
    assert_eq!(encoded["volumeSerial"], u64::MAX.to_string());
    assert_eq!(encoded["fileId"], u128::MAX.to_string());
    let observation_json = serde_json::to_value(exact_observation(
        "v1:i:encoded/i:json",
        "json.flp",
        u64::MAX,
        i128::MAX,
        None,
    ))
    .unwrap();
    assert_eq!(observation_json["modifiedAtNs"], i128::MAX.to_string());
    assert_eq!(observation_json["byteSize"], u64::MAX.to_string());
    assert!(observation_json["byteSize"].is_string());
    assert!(observation_json["modifiedAtNs"].is_string());

    database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("numeric-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &first,
        4,
        &[exact_observation(
            "v1:i:encoded/i:one",
            "one.flp",
            i64::MAX as u64,
            i64::MIN as i128,
            Some(maximum_identity.clone()),
        )],
    );
    let committed = database.list_published_locations(&root.id).unwrap();
    assert_eq!(committed[0].byte_size, i64::MAX as u64);
    assert_eq!(committed[0].modified_at_ns, i64::MIN);
    assert_eq!(committed[0].identity, Some(maximum_identity.clone()));

    let second_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let second = database
        .lease_next_scan("numeric-session", 7, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &second,
        8,
        &[exact_observation(
            "v1:i:encoded/i:one",
            "one.flp",
            i64::MAX as u64,
            i64::MIN as i128 + 1,
            Some(maximum_identity.clone()),
        )],
    );
    assert_eq!(
        database.list_published_locations(&root.id).unwrap()[0].modified_at_ns,
        i64::MIN + 1
    );
    assert_eq!(
        database.scan_job(&second_job.job_id).unwrap().state,
        ScanJobState::Completed
    );

    let out_of_range_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 10)
        .unwrap();
    let out_of_range = database
        .lease_next_scan("numeric-session", 11, 100)
        .unwrap()
        .unwrap();
    assert!(matches!(
        database.stage_scan_observations(
            &out_of_range.run.id,
            "numeric-session",
            &out_of_range.run.lease_token,
            12,
            &[exact_observation(
                "v1:i:encoded/i:one",
                "one.flp",
                i64::MAX as u64 + 1,
                i64::MAX as i128 + 1,
                Some(maximum_identity.clone()),
            )],
        ),
        Err(StorageError::StagingRejected)
    ));
    assert_eq!(
        database.scan_job(&out_of_range_job.job_id).unwrap().state,
        ScanJobState::Failed
    );
    assert_eq!(
        database.list_published_locations(&root.id).unwrap()[0].modified_at_ns,
        i64::MIN + 1
    );

    let malformed_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 14)
        .unwrap();
    let malformed = database
        .lease_next_scan("numeric-session", 15, 100)
        .unwrap()
        .unwrap();
    assert!(matches!(
        database.stage_scan_observations(
            &malformed.run.id,
            "numeric-session",
            &malformed.run.lease_token,
            16,
            &[exact_observation(
                "v1:i:encoded/i:bad",
                "bad.flp",
                1,
                1,
                Some(EncodedIdentity {
                    volume_serial: "01".to_owned(),
                    file_id: "340282366920938463463374607431768211456".to_owned(),
                }),
            )],
        ),
        Err(StorageError::StagingRejected)
    ));
    assert_eq!(
        database.scan_job(&malformed_job.job_id).unwrap().state,
        ScanJobState::Failed
    );
}

#[test]
fn publication_round_trips_maximum_modified_at_ns() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("maximum-timestamp-session", 1)
        .unwrap();
    database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("maximum-timestamp-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &lease,
        4,
        &[exact_observation(
            "v1:i:maximum.flp",
            "maximum.flp",
            1,
            i64::MAX as i128,
            None,
        )],
    );

    let locations = database.list_published_locations(&root.id).unwrap();
    assert_eq!(locations.len(), 1);
    assert_eq!(locations[0].modified_at_ns, i64::MAX);
    let page = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id,
            page_size: 1,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(page.locations[0].modified_at_ns, i64::MAX);
}

#[test]
fn integration_library_pages_are_root_scoped_and_snapshot_bound() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root_a = database.add_scan_root("A", "C:\\Music\\A").unwrap();
    let root_b = database.add_scan_root("B", "C:\\Music\\B").unwrap();
    database
        .begin_scan_session("library-contract-session", 1)
        .unwrap();

    let a_job = database
        .enqueue_scan(&root_a.id, ScanKind::Manual, 2)
        .unwrap();
    let a = database
        .lease_next_scan("library-contract-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &a,
        4,
        &[
            staged_observation("a.flp", 1, 1, None),
            staged_observation("b.flp", 1, 1, None),
        ],
    );
    assert_eq!(
        database.scan_job(&a_job.job_id).unwrap().state,
        ScanJobState::Completed
    );

    let b_job = database
        .enqueue_scan(&root_b.id, ScanKind::Manual, 6)
        .unwrap();
    let b = database
        .lease_next_scan("library-contract-session", 7, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &b,
        8,
        &[staged_observation("only-b.flp", 1, 1, None)],
    );
    assert_eq!(
        database.scan_job(&b_job.job_id).unwrap().state,
        ScanJobState::Completed
    );

    let page_a = database
        .query_library(&LibraryQuery {
            scan_root_id: root_a.id.clone(),
            page_size: 1,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    let cursor_a = page_a.next_cursor.clone().unwrap();
    let page_b = database
        .query_library(&LibraryQuery {
            scan_root_id: root_b.id.clone(),
            page_size: 1,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(page_b.scan_root_id, root_b.id);
    assert_eq!(page_b.locations.len(), 1);
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root_b.id.clone(),
            page_size: 1,
            cursor: Some(cursor_a.clone()),
            snapshot: Some(page_a.snapshot.clone()),
        }),
        Err(StorageError::InvalidCursor)
    ));
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root_a.id.clone(),
            page_size: 1,
            cursor: Some(cursor_a.clone()),
            snapshot: None,
        }),
        Err(StorageError::InvalidCursor)
    ));
    let mut malformed_cursor = cursor_a.clone();
    malformed_cursor.locator_key.push('\0');
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root_a.id.clone(),
            page_size: 1,
            cursor: Some(malformed_cursor),
            snapshot: Some(page_a.snapshot.clone()),
        }),
        Err(StorageError::InvalidCursor)
    ));

    // Publishing another root does not change root A's committed snapshot.
    database
        .enqueue_scan(&root_b.id, ScanKind::Manual, 10)
        .unwrap();
    let b_follow_up = database
        .lease_next_scan("library-contract-session", 11, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &b_follow_up,
        12,
        &[staged_observation("only-b.flp", 2, 2, None)],
    );
    assert_eq!(
        database
            .query_library(&LibraryQuery {
                scan_root_id: root_a.id.clone(),
                page_size: 1,
                cursor: Some(cursor_a.clone()),
                snapshot: Some(page_a.snapshot.clone()),
            })
            .unwrap()
            .locations
            .len(),
        1
    );

    // A publication in the queried root expires the old cursor atomically.
    database
        .enqueue_scan(&root_a.id, ScanKind::Manual, 14)
        .unwrap();
    let a_follow_up = database
        .lease_next_scan("library-contract-session", 15, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &a_follow_up,
        16,
        &[
            staged_observation("a.flp", 1, 1, None),
            staged_observation("b.flp", 1, 1, None),
        ],
    );
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root_a.id.clone(),
            page_size: 1,
            cursor: Some(cursor_a),
            snapshot: Some(page_a.snapshot),
        }),
        Err(StorageError::StaleCursor)
    ));
    let restarted = database
        .query_library(&LibraryQuery {
            scan_root_id: root_a.id,
            page_size: 200,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(restarted.locations.len(), 2);
}

#[test]
fn integration_cancellation_uses_job_before_lease_and_run_after_lease() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("cancel-contract-session", 1)
        .unwrap();

    let queued = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    assert!(matches!(
        database.scan_run(&queued.job_id),
        Err(StorageError::NotFound)
    ));
    assert_eq!(
        database.cancel_scan_job(&queued.job_id, 3).unwrap(),
        ScanJobState::Cancelled
    );
    assert!(
        database
            .lease_next_scan("cancel-contract-session", 4, 100)
            .unwrap()
            .is_none()
    );

    let leased_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 5)
        .unwrap();
    let leased = database
        .lease_next_scan("cancel-contract-session", 6, 100)
        .unwrap()
        .unwrap();
    assert_eq!(leased.run.scan_job_id, leased_job.job_id);
    assert_ne!(leased.run.id, leased_job.job_id);
    database
        .begin_scan_staging(
            &leased.run.id,
            "cancel-contract-session",
            &leased.run.lease_token,
            7,
        )
        .unwrap();
    database
        .stage_scan_observations(
            &leased.run.id,
            "cancel-contract-session",
            &leased.run.lease_token,
            8,
            &[staged_observation("cancelled.flp", 1, 1, None)],
        )
        .unwrap();
    assert_eq!(
        database.cancel_scan_job(&leased_job.job_id, 9).unwrap(),
        ScanJobState::Running
    );
    assert!(matches!(
        database.publish_scan_run(
            &leased.run.id,
            "cancel-contract-session",
            &leased.run.lease_token,
            10,
        ),
        Err(StorageError::Conflict)
    ));
    assert_eq!(
        database
            .finish_scan_run(
                &leased.run.id,
                "cancel-contract-session",
                &leased.run.lease_token,
                11,
                ScanRunOutcome::Cancelled,
            )
            .unwrap(),
        ScanRunState::Cancelled
    );
    assert_eq!(
        database.scan_job(&leased_job.job_id).unwrap().state,
        ScanJobState::Cancelled
    );
    assert_eq!(
        database.scan_run(&leased.run.id).unwrap().state,
        ScanRunState::Cancelled
    );
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );

    // Commit order decides the race: publication committed before cancellation
    // stays completed and the late cancellation reports that outcome.
    let final_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 12)
        .unwrap();
    let decided = database
        .lease_next_scan("cancel-contract-session", 13, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &decided,
        14,
        &[staged_observation("final.flp", 1, 1, None)],
    );
    assert_eq!(
        database.scan_job(&final_job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    assert_eq!(
        database.cancel_scan_job(&final_job.job_id, 15).unwrap(),
        ScanJobState::Completed
    );
    assert_eq!(
        database.list_published_locations(&root.id).unwrap().len(),
        1
    );
}

#[test]
fn integration_canonical_decimal_and_locator_key_shape_are_enforced() {
    // JSON DTO: canonical decimal strings round-trip; numbers, leading zeros,
    // explicit signs, and partial identities never parse.
    let valid = serde_json::json!({
        "locatorKey": "v1:i:projects/i:a.flp",
        "relativePath": "Projects\\a.flp",
        "byteSize": "18446744073709551615",
        "modifiedAtNs": "-1700000000000000000",
        "identity": {"volumeSerial": "7", "fileId": "9"},
    });
    let parsed: ScanObservation = serde_json::from_value(valid).unwrap();
    assert_eq!(parsed.byte_size, u64::MAX);
    assert_eq!(parsed.modified_at_ns, -1_700_000_000_000_000_000);

    let observation = |byte_size: serde_json::Value,
                       modified_at_ns: serde_json::Value,
                       identity: serde_json::Value| {
        serde_json::json!({
            "locatorKey": "v1:i:projects/i:a.flp",
            "relativePath": "a.flp",
            "byteSize": byte_size,
            "modifiedAtNs": modified_at_ns,
            "identity": identity,
        })
    };
    let full_identity = serde_json::json!({"volumeSerial": "7", "fileId": "9"});
    for invalid in [
        observation(
            serde_json::json!(1),
            serde_json::json!("1"),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("01"),
            serde_json::json!("1"),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("+1"),
            serde_json::json!("1"),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("-1"),
            serde_json::json!("1"),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("1"),
            serde_json::json!(1),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("1"),
            serde_json::json!("+1"),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("1"),
            serde_json::json!("-0"),
            full_identity.clone(),
        ),
        observation(
            serde_json::json!("1"),
            serde_json::json!("1"),
            serde_json::json!({"volumeSerial": "7"}),
        ),
        observation(
            serde_json::json!("1"),
            serde_json::json!("1"),
            serde_json::json!({"volumeSerial": "07", "fileId": "9"}),
        ),
        observation(
            serde_json::json!("1"),
            serde_json::json!("1"),
            serde_json::json!({"volumeSerial": "7", "fileId": "340282366920938463463374607431768211456"}),
        ),
    ] {
        assert!(
            serde_json::from_value::<ScanObservation>(invalid).is_err(),
            "non-canonical JSON observation must not parse"
        );
    }

    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("shape-session", 1).unwrap();

    fn attempt(
        database: &mut Database,
        root_id: &str,
        at: i64,
        observations: Vec<ScanObservation>,
    ) -> (String, std::result::Result<ScanStaging, StorageError>) {
        let job_id = database
            .enqueue_scan(root_id, ScanKind::Manual, at)
            .unwrap()
            .job_id;
        let lease = database
            .lease_next_scan("shape-session", at + 1, 100)
            .unwrap()
            .unwrap();
        let result = database.stage_scan_observations(
            &lease.run.id,
            "shape-session",
            &lease.run.lease_token,
            at + 2,
            &observations,
        );
        (job_id, result)
    }

    // Baseline: mixed-mode and percent-encoded (NFC Unicode) keys are valid.
    let baseline_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let baseline = database
        .lease_next_scan("shape-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &baseline,
        4,
        &[
            exact_observation(
                "v1:i:shared/s:BuildOutput/i:artifact.flp",
                "artifact.flp",
                3,
                3,
                None,
            ),
            exact_observation("v1:i:caf%C3%A9.flp", "caf\u{e9}.flp", 4, 4, None),
            exact_observation("v1:i:lower%c3%a9.flp", "lower-caf\u{e9}.flp", 5, 5, None),
        ],
    );
    assert_eq!(
        database.scan_job(&baseline_job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    let committed = database.list_published_locations(&root.id).unwrap();
    assert_eq!(committed.len(), 3);

    // Every malformed key rejects its run without touching committed rows.
    let invalid_keys = [
        "Projects/foo.flp",
        "ci:projects:foo.flp",
        "v1:",
        "v1:i:",
        "v1:x:foo.flp",
        "v1:i:foo/bar",
        "v1:i:a//i:b",
        "v1:i:a\\b",
        "v1:i:caf\u{e9}.flp",
        "v1:i:bad%2.flp",
        "v1:i:bad%zz.flp",
        "v1:i:ok.flp%",
    ];
    let mut at = 10_i64;
    for key in invalid_keys {
        let (job_id, result) = attempt(
            &mut database,
            &root.id,
            at,
            vec![exact_observation(key, "a.flp", 1, 1, None)],
        );
        assert!(
            matches!(result, Err(StorageError::StagingRejected)),
            "key must reject: {key}"
        );
        assert_eq!(
            database.scan_job(&job_id).unwrap().state,
            ScanJobState::Failed
        );
        at += 4;
    }

    // Over-length keys reject; the exact 32 KiB maximum stages cleanly.
    let over_key = format!("v1:i:{}", "a".repeat(32_768 - 4));
    assert_eq!(over_key.len(), 32_769);
    let (job_id, result) = attempt(
        &mut database,
        &root.id,
        at,
        vec![exact_observation(&over_key, "a.flp", 1, 1, None)],
    );
    assert!(matches!(result, Err(StorageError::StagingRejected)));
    assert_eq!(
        database.scan_job(&job_id).unwrap().state,
        ScanJobState::Failed
    );
    at += 4;

    let max_key = format!("v1:i:{}", "a".repeat(32_768 - 5));
    assert_eq!(max_key.len(), 32_768);
    database
        .enqueue_scan(&root.id, ScanKind::Manual, at)
        .unwrap();
    let max_lease = database
        .lease_next_scan("shape-session", at + 1, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &max_lease.run.id,
            "shape-session",
            &max_lease.run.lease_token,
            at + 2,
            &[exact_observation(&max_key, "max.flp", 1, 1, None)],
        )
        .unwrap();
    database
        .finish_scan_run(
            &max_lease.run.id,
            "shape-session",
            &max_lease.run.lease_token,
            at + 3,
            ScanRunOutcome::Cancelled,
        )
        .unwrap();
    at += 4;

    // u64::MAX is valid JSON but exceeds the SQLite i64 byte-size boundary.
    let (job_id, result) = attempt(
        &mut database,
        &root.id,
        at,
        vec![exact_observation(
            "v1:i:projects/i:big.flp",
            "big.flp",
            u64::MAX,
            1,
            None,
        )],
    );
    assert!(matches!(result, Err(StorageError::StagingRejected)));
    assert_eq!(
        database.scan_job(&job_id).unwrap().state,
        ScanJobState::Failed
    );
    at += 4;

    // Duplicate keys in one batch reject the whole run with no partial rows.
    let duplicate = exact_observation("v1:i:projects/i:dup.flp", "dup.flp", 1, 1, None);
    let (job_id, result) = attempt(
        &mut database,
        &root.id,
        at,
        vec![duplicate.clone(), duplicate],
    );
    assert!(matches!(result, Err(StorageError::StagingRejected)));
    assert_eq!(
        database.scan_job(&job_id).unwrap().state,
        ScanJobState::Failed
    );

    assert_eq!(
        database.list_published_locations(&root.id).unwrap(),
        committed
    );
}

#[test]
fn integration_staging_is_never_visible_to_library_reads() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("invisible-session", 1).unwrap();

    let first_job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let first = database
        .lease_next_scan("invisible-session", 3, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &first,
        4,
        &[staged_observation("visible.flp", 1, 1, None)],
    );
    assert_eq!(
        database.scan_job(&first_job.job_id).unwrap().state,
        ScanJobState::Completed
    );

    let page = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 200,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(page.locations.len(), 1);
    let cursor = page.next_cursor.clone().unwrap();
    let snapshot = page.snapshot.clone();

    // Queued (unleased, unpublished) work does not expire the cursor.
    database
        .enqueue_scan(&root.id, ScanKind::Manual, 6)
        .unwrap();
    let queued = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 200,
            cursor: Some(cursor.clone()),
            snapshot: Some(snapshot.clone()),
        })
        .unwrap();
    assert_eq!(queued.snapshot, snapshot);
    assert!(queued.locations.is_empty());
    assert!(!queued.has_more);

    // Staged-but-unpublished observations are invisible to every read shape.
    let staged = database
        .lease_next_scan("invisible-session", 7, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &staged.run.id,
            "invisible-session",
            &staged.run.lease_token,
            8,
            &[staged_observation("staged.flp", 2, 2, None)],
        )
        .unwrap();
    let reread = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 200,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(reread.snapshot, snapshot);
    assert_eq!(reread.locations.len(), 1);
    assert_eq!(reread.locations[0].locator_key, v1_key("visible.flp"));
    database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 200,
            cursor: Some(cursor.clone()),
            snapshot: Some(snapshot.clone()),
        })
        .unwrap();

    // Publication expires the old cursor atomically; restart sees both rows.
    database
        .publish_scan_run(
            &staged.run.id,
            "invisible-session",
            &staged.run.lease_token,
            9,
        )
        .unwrap();
    assert!(matches!(
        database.query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 200,
            cursor: Some(cursor),
            snapshot: Some(snapshot),
        }),
        Err(StorageError::StaleCursor)
    ));
    let restarted = database
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 200,
            cursor: None,
            snapshot: None,
        })
        .unwrap();
    assert_eq!(restarted.locations.len(), 2);
}

#[test]
fn migration_quarantines_legacy_keys_and_first_v1_scan_retains_projects() {
    let directory = TestDirectory::new();
    let root_id;
    {
        let mut fixture =
            Database::open_with_migrations(directory.path(), &MIGRATIONS[..4]).unwrap();
        let root = fixture
            .add_scan_root("Projects", "C:\\Music\\Projects")
            .unwrap();
        root_id = root.id.clone();
        // Populated v4 database: legacy display-ish paths, millisecond
        // timestamps, and unbounded legacy identities, including one Unicode
        // path and one non-canonical identity.
        fixture
            .connection
            .execute(
                "INSERT INTO project_file
                 (id, display_filename, extension, byte_size, modified_at_ms,
                  created_at_ms, updated_at_ms)
                 VALUES ('project-foo', 'Foo.flp', '.flp', 10, 1700000000000, 1, 1),
                        ('project-cafe', 'cafe.flp', '.flp', 20, 1700000001000, 1, 1),
                        ('project-bad', 'bad.flp', '.flp', 30, 1700000002000, 1, 1)",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO file_location
                 (id, project_file_id, scan_root_id, detached_scan_root_id,
                  normalized_path, relative_path, byte_size, modified_at_ms,
                  volume_id, filesystem_file_id, presence,
                  last_seen_scan_run_id, last_seen_at_ms, created_at_ms, updated_at_ms)
                 VALUES ('location-foo', 'project-foo', ?1, NULL,
                    'Projects\\Foo.flp', 'Projects\\Foo.flp', 10, 1700000000000,
                    7, '9', 'present', NULL, NULL, 1, 1),
                        ('location-cafe', 'project-cafe', ?1, NULL,
                    'Projets\\caf\u{e9}.flp', 'Projets\\caf\u{e9}.flp',
                    20, 1700000001000,
                    7, '123456789012345678901234567890', 'present', NULL, NULL, 1, 1),
                        ('location-bad', 'project-bad', ?1, NULL,
                    'Projects\\bad.flp', 'Projects\\bad.flp', 30, 1700000002000,
                    7, '007', 'present', NULL, NULL, 1, 1)",
                [&root_id],
            )
            .unwrap();
        drop(fixture);
    }

    let mut database = Database::open(directory.path()).unwrap();
    assert_eq!(database.schema_version().unwrap(), 5);
    let migrated = database.list_published_locations(&root_id).unwrap();
    assert_eq!(migrated.len(), 3);

    // Legacy keys are quarantined, never promoted to v1:.
    let foo = migrated
        .iter()
        .find(|location| location.id == "location-foo")
        .unwrap();
    assert_eq!(foo.locator_key, "v0:legacy:Projects\\Foo.flp");
    assert_eq!(foo.relative_path, "Projects\\Foo.flp");
    assert_eq!(foo.modified_at_ns, 1_700_000_000_000_000_000);
    assert_eq!(
        foo.identity,
        Some(EncodedIdentity {
            volume_serial: "7".to_owned(),
            file_id: "9".to_owned(),
        })
    );
    let kept: String = database
        .connection
        .query_row(
            "SELECT normalized_path FROM file_location WHERE id = 'location-foo'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(kept, "Projects\\Foo.flp");

    // Non-ASCII legacy paths migrate without failing.
    let cafe = migrated
        .iter()
        .find(|location| location.id == "location-cafe")
        .unwrap();
    assert_eq!(cafe.locator_key, "v0:legacy:Projets\\caf\u{e9}.flp");
    assert_eq!(
        cafe.identity,
        Some(EncodedIdentity {
            volume_serial: "7".to_owned(),
            file_id: "123456789012345678901234567890".to_owned(),
        })
    );

    // Non-canonical historical identity becomes unavailable evidence.
    let bad = migrated
        .iter()
        .find(|location| location.id == "location-bad")
        .unwrap();
    assert_eq!(bad.locator_key, "v0:legacy:Projects\\bad.flp");
    assert_eq!(bad.identity, None);

    // First V1 scan: disjoint key space mints new locationIds, while the
    // qualified-identity evidence retains the project associations,
    // including across a case-only rename and a Unicode respelling.
    database
        .begin_scan_session("quarantine-session", 10)
        .unwrap();
    database
        .enqueue_scan(&root_id, ScanKind::Manual, 11)
        .unwrap();
    let lease = database
        .lease_next_scan("quarantine-session", 12, 100)
        .unwrap()
        .unwrap();
    publish_observations(
        &mut database,
        &lease,
        13,
        &[
            exact_observation(
                "v1:i:projects/i:foo.flp",
                "Projects\\foo.flp",
                11,
                1_700_000_001_000_000_000,
                Some(EncodedIdentity {
                    volume_serial: "7".to_owned(),
                    file_id: "9".to_owned(),
                }),
            ),
            exact_observation(
                "v1:i:projets/i:caf%C3%A9.flp",
                "Projets\\caf\u{e9}.flp",
                21,
                1_700_000_002_000_000_000,
                Some(EncodedIdentity {
                    volume_serial: "7".to_owned(),
                    file_id: "123456789012345678901234567890".to_owned(),
                }),
            ),
        ],
    );

    let after = database.list_published_locations(&root_id).unwrap();
    assert_eq!(after.len(), 5);
    let new_foo = after
        .iter()
        .find(|location| location.locator_key == "v1:i:projects/i:foo.flp")
        .unwrap();
    assert_ne!(new_foo.id, "location-foo");
    assert_eq!(new_foo.project_file_id, "project-foo");
    assert_eq!(new_foo.relative_path, "Projects\\foo.flp");
    assert_eq!(new_foo.presence, FilePresence::Present);
    let new_cafe = after
        .iter()
        .find(|location| location.locator_key == "v1:i:projets/i:caf%C3%A9.flp")
        .unwrap();
    assert_ne!(new_cafe.id, "location-cafe");
    assert_eq!(new_cafe.project_file_id, "project-cafe");
    assert_eq!(new_cafe.relative_path, "Projets\\caf\u{e9}.flp");
    for legacy_id in ["location-foo", "location-cafe", "location-bad"] {
        let legacy = after
            .iter()
            .find(|location| location.id == legacy_id)
            .unwrap();
        assert_eq!(legacy.presence, FilePresence::Missing);
        assert!(legacy.locator_key.starts_with("v0:legacy:"));
    }
}

#[test]
fn completed_delegation_publishes_a_non_empty_stage_and_splits_error_codes() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database.begin_scan_session("delegate-session", 1).unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("delegate-session", 3, 100)
        .unwrap()
        .unwrap();

    // Completing a run that has no stage row at all is "no open stage"
    // (`NotFound`), not a stale-lease `Conflict`.
    assert!(matches!(
        database.finish_scan_run(
            &lease.run.id,
            "delegate-session",
            &lease.run.lease_token,
            4,
            ScanRunOutcome::Completed,
        ),
        Err(StorageError::NotFound)
    ));

    // A stale token still reports the fence as `Conflict`.
    database
        .begin_scan_staging(&lease.run.id, "delegate-session", &lease.run.lease_token, 4)
        .unwrap();
    assert!(matches!(
        database.finish_scan_run(
            &lease.run.id,
            "delegate-session",
            "wrong-token",
            5,
            ScanRunOutcome::Completed,
        ),
        Err(StorageError::Conflict)
    ));

    // Delegation must publish a NON-EMPTY open stage, not just an empty one.
    database
        .stage_scan_observations(
            &lease.run.id,
            "delegate-session",
            &lease.run.lease_token,
            6,
            &[staged_observation("delegated.flp", 7, 7, Some("delegated"))],
        )
        .unwrap();
    assert_eq!(
        database
            .finish_scan_run(
                &lease.run.id,
                "delegate-session",
                &lease.run.lease_token,
                7,
                ScanRunOutcome::Completed,
            )
            .unwrap(),
        ScanRunState::Completed
    );
    let published = database.list_published_locations(&root.id).unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].relative_path, "delegated.flp");
    assert_eq!(published[0].presence, FilePresence::Present);
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Published
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    assert_eq!(
        database
            .scan_root_publication(&root.id)
            .unwrap()
            .last_successful_run_id
            .as_deref(),
        Some(lease.run.id.as_str())
    );
}

#[test]
fn publication_fences_an_expired_lease_before_applying() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("expired-lease-session", 1)
        .unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("expired-lease-session", 3, 10)
        .unwrap()
        .unwrap();
    assert_eq!(lease.run.lease_expires_at_ms, 13);
    database
        .stage_scan_observations(
            &lease.run.id,
            "expired-lease-session",
            &lease.run.lease_token,
            4,
            &[staged_observation("expired.flp", 1, 1, None)],
        )
        .unwrap();

    // At the boundary the lease is already expired: `lease_expires_at_ms <=
    // now`. Both staging and publication must refuse, and the open stage
    // must survive untouched until a renewed lease takes over.
    assert!(matches!(
        database.stage_scan_observations(
            &lease.run.id,
            "expired-lease-session",
            &lease.run.lease_token,
            13,
            &[staged_observation("after-expiry.flp", 1, 1, None)],
        ),
        Err(StorageError::Conflict)
    ));
    assert!(matches!(
        database.publish_scan_run(
            &lease.run.id,
            "expired-lease-session",
            &lease.run.lease_token,
            13,
        ),
        Err(StorageError::Conflict)
    ));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Open
    );
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().record_count,
        1
    );
    assert_eq!(
        database.scan_run(&lease.run.id).unwrap().state,
        ScanRunState::Running
    );
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );

    // A renewed lease publishes the surviving stage normally.
    database
        .renew_scan_lease(
            &lease.run.id,
            "expired-lease-session",
            &lease.run.lease_token,
            12,
            10,
        )
        .unwrap();
    database
        .publish_scan_run(
            &lease.run.id,
            "expired-lease-session",
            &lease.run.lease_token,
            15,
        )
        .unwrap();
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Completed
    );
    assert_eq!(
        database.list_published_locations(&root.id).unwrap().len(),
        1
    );
}

#[test]
fn single_call_staging_over_the_batch_record_limit_is_rejected_without_writing() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("batch-limit-session", 1)
        .unwrap();
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, 2)
        .unwrap();
    let lease = database
        .lease_next_scan("batch-limit-session", 3, 100)
        .unwrap()
        .unwrap();

    let observations: Vec<_> = (0..=MAX_STAGED_BATCH_RECORDS)
        .map(|index| staged_observation(&format!("over-limit-{index}.flp"), 1, 1, None))
        .collect();
    assert_eq!(observations.len(), MAX_STAGED_BATCH_RECORDS + 1);
    assert!(matches!(
        database.stage_scan_observations(
            &lease.run.id,
            "batch-limit-session",
            &lease.run.lease_token,
            4,
            &observations,
        ),
        Err(StorageError::StagingRejected)
    ));
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().state,
        ScanStageState::Discarded
    );
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().record_count,
        0
    );
    assert_eq!(
        database
            .scan_run(&lease.run.id)
            .unwrap()
            .error_code
            .as_deref(),
        Some("staging_rejected")
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Failed
    );
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn staging_rejects_invalid_relative_path_fixtures_terminally() {
    let directory = TestDirectory::new();
    let mut database = Database::open(directory.path()).unwrap();
    let root = database
        .add_scan_root("Projects", "C:\\Music\\Projects")
        .unwrap();
    database
        .begin_scan_session("relative-path-session", 1)
        .unwrap();

    let fixtures = [
        ("dot_dot", ".."),
        ("leading_slash", "/escape.flp"),
        ("empty", ""),
        ("nul", "bad\u{0}name.flp"),
        ("colon", "with:colon.flp"),
    ];
    let mut at = 2;
    for (name, relative_path) in fixtures {
        let job = database
            .enqueue_scan(&root.id, ScanKind::Manual, at)
            .unwrap();
        let lease = database
            .lease_next_scan("relative-path-session", at, 100)
            .unwrap()
            .unwrap();
        assert!(
            matches!(
                database.stage_scan_observations(
                    &lease.run.id,
                    "relative-path-session",
                    &lease.run.lease_token,
                    at,
                    &[exact_observation("v1:i:bad.flp", relative_path, 1, 1, None)],
                ),
                Err(StorageError::StagingRejected)
            ),
            "fixture {name} must be rejected"
        );
        assert_eq!(
            database.scan_staging(&lease.run.id).unwrap().state,
            ScanStageState::Discarded,
            "fixture {name} must terminate its stage"
        );
        assert_eq!(
            database.scan_job(&job.job_id).unwrap().state,
            ScanJobState::Failed
        );
        at += 2;
    }

    // A well-formed relative_path still stages normally.
    let job = database
        .enqueue_scan(&root.id, ScanKind::Manual, at)
        .unwrap();
    let lease = database
        .lease_next_scan("relative-path-session", at, 100)
        .unwrap()
        .unwrap();
    database
        .stage_scan_observations(
            &lease.run.id,
            "relative-path-session",
            &lease.run.lease_token,
            at,
            &[staged_observation("good.flp", 1, 1, None)],
        )
        .unwrap();
    assert_eq!(
        database.scan_staging(&lease.run.id).unwrap().record_count,
        1
    );
    assert_eq!(
        database.scan_job(&job.job_id).unwrap().state,
        ScanJobState::Running
    );
    assert!(
        database
            .list_published_locations(&root.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn migration_005_timestamp_guards_cover_locations_and_staging() {
    // A legacy file_location row whose millisecond timestamp cannot be
    // converted to nanoseconds must abort migration 005.
    let location_directory = TestDirectory::new();
    {
        let fixture =
            Database::open_with_migrations(location_directory.path(), &MIGRATIONS[..4]).unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO project_file
                 (id, display_filename, extension, byte_size, modified_at_ms,
                  created_at_ms, updated_at_ms)
                 VALUES ('project-guard', 'Guard.flp', '.flp', 1, 1, 1, 1)",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_root (id, display_name, canonical_path)
                 VALUES ('root-guard', 'Guard', 'C:\\Guard')",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO file_location
                 (id, project_file_id, scan_root_id, detached_scan_root_id,
                  normalized_path, relative_path, byte_size, modified_at_ms,
                  presence, created_at_ms, updated_at_ms)
                 VALUES ('location-guard', 'project-guard', 'root-guard', NULL,
                    'Guard.flp', 'Guard.flp', 1, ?1, 'present', 1, 1)",
                rusqlite::params![i64::MAX],
            )
            .unwrap();
        drop(fixture);
    }
    assert!(matches!(
        Database::open_with_migrations(location_directory.path(), MIGRATIONS),
        Err(StorageError::MigrationFailed)
    ));

    // The same bound applies to legacy scan_stage_observation rows.
    let stage_directory = TestDirectory::new();
    {
        let fixture =
            Database::open_with_migrations(stage_directory.path(), &MIGRATIONS[..4]).unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_root (id, display_name, canonical_path)
                 VALUES ('root-guard', 'Guard', 'C:\\Guard')",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_job
                 (id, scan_root_id, kind, retry_chain_id, not_before_ms,
                  created_at_ms, updated_at_ms)
                 VALUES ('job-guard', 'root-guard', 'manual', 'chain-guard', 1, 1, 1)",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_run
                 (id, scan_job_id, scan_root_id, generation, configuration_revision,
                  retry_chain_id, attempt, session_id, lease_token, started_at_ms,
                  lease_expires_at_ms)
                 VALUES ('run-guard', 'job-guard', 'root-guard', 0, 0, 'chain-guard',
                    1, 'session-guard', 'token-guard', 1, 100)",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_stage
                 (run_id, scan_root_id, generation, configuration_revision,
                  session_id, lease_token, created_at_ms, updated_at_ms)
                 VALUES ('run-guard', 'root-guard', 0, 0, 'session-guard',
                    'token-guard', 1, 1)",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_stage_observation
                 (run_id, normalized_path, relative_path, byte_size, modified_at_ms)
                 VALUES ('run-guard', 'guard.flp', 'guard.flp', 1, ?1)",
                rusqlite::params![i64::MIN],
            )
            .unwrap();
        drop(fixture);
    }
    assert!(matches!(
        Database::open_with_migrations(stage_directory.path(), MIGRATIONS),
        Err(StorageError::MigrationFailed)
    ));
}

#[test]
fn migration_005_keeps_only_canonical_u128_file_ids_at_the_39_digit_boundary() {
    let directory = TestDirectory::new();
    let root_id;
    {
        let fixture = Database::open_with_migrations(directory.path(), &MIGRATIONS[..4]).unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO project_file
                 (id, display_filename, extension, byte_size, modified_at_ms,
                  created_at_ms, updated_at_ms)
                 VALUES ('project-max', 'Max.flp', '.flp', 1, 1, 1, 1),
                        ('project-overflow', 'Overflow.flp', '.flp', 1, 1, 1, 1),
                        ('project-wide', 'Wide.flp', '.flp', 1, 1, 1, 1)",
                [],
            )
            .unwrap();
        fixture
            .connection
            .execute(
                "INSERT INTO scan_root (id, display_name, canonical_path)
                 VALUES ('root-boundary', 'Boundary', 'C:\\Boundary')",
                [],
            )
            .unwrap();
        root_id = "root-boundary".to_owned();
        // u128::MAX is exactly 39 digits and canonical: it must survive.
        // u128::MAX + 1 is also 39 digits but out of range: it must be
        // dropped. A 40-digit value is out of range by length.
        fixture
            .connection
            .execute(
                "INSERT INTO file_location
                 (id, project_file_id, scan_root_id, detached_scan_root_id,
                  normalized_path, relative_path, byte_size, modified_at_ms,
                  volume_id, filesystem_file_id, presence,
                  created_at_ms, updated_at_ms)
                 VALUES ('location-max', 'project-max', ?1, NULL,
                    'Max.flp', 'Max.flp', 1, 1, 7,
                    '340282366920938463463374607431768211455', 'present', 1, 1),
                        ('location-overflow', 'project-overflow', ?1, NULL,
                    'Overflow.flp', 'Overflow.flp', 1, 1, 7,
                    '340282366920938463463374607431768211456', 'present', 1, 1),
                        ('location-wide', 'project-wide', ?1, NULL,
                    'Wide.flp', 'Wide.flp', 1, 1, 7,
                    '9999999999999999999999999999999999999999', 'present', 1, 1)",
                [&root_id],
            )
            .unwrap();
        drop(fixture);
    }

    let database = Database::open(directory.path()).unwrap();
    let locations = database.list_published_locations(&root_id).unwrap();
    assert_eq!(locations.len(), 3);
    let max = locations
        .iter()
        .find(|location| location.id == "location-max")
        .unwrap();
    assert_eq!(
        max.identity,
        Some(EncodedIdentity {
            volume_serial: "7".to_owned(),
            file_id: "340282366920938463463374607431768211455".to_owned(),
        })
    );
    for legacy_id in ["location-overflow", "location-wide"] {
        let dropped = locations
            .iter()
            .find(|location| location.id == legacy_id)
            .unwrap();
        assert_eq!(dropped.identity, None, "{legacy_id} must be quarantined");
    }
}
