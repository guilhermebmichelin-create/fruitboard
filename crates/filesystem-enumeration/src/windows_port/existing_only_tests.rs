use super::*;
use std::os::windows::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "fruitboard-enumeration-existing-only-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create owned test directory: {error}"),
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only these two owned names can have been created by the regression.
        // Never traverse a link or recursively delete temporary storage.
        for name in ["vanished-file", "vanished-directory"] {
            let path = self.0.join(name);
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) => {
                    assert_eq!(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT, 0);
                    if metadata.is_dir() {
                        std::fs::remove_dir(path).unwrap();
                    } else {
                        std::fs::remove_file(path).unwrap();
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("inspect owned test name: {error}"),
            }
        }
        let metadata = std::fs::symlink_metadata(&self.0).unwrap();
        assert_eq!(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT, 0);
        std::fs::remove_dir(&self.0).unwrap();
    }
}

#[test]
fn disappeared_file_is_not_recreated_by_relative_open() {
    let fixture = Fixture::new();
    let path = fixture.0.join("vanished-file");
    std::fs::write(&path, b"private synthetic regression").unwrap();
    let parent = open_path(&fixture.0).unwrap();
    std::fs::remove_file(&path).unwrap();

    let result = open_relative(
        &parent,
        OsStr::new("vanished-file"),
        FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT_OPTION,
    );
    assert!(
        !path.exists(),
        "relative open recreated the disappeared file"
    );
    assert!(matches!(result, Err(PortError::NotFound)));
}

#[test]
fn disappeared_directory_is_not_recreated_by_relative_open() {
    let fixture = Fixture::new();
    let path = fixture.0.join("vanished-directory");
    std::fs::create_dir(&path).unwrap();
    let parent = open_path(&fixture.0).unwrap();
    std::fs::remove_dir(&path).unwrap();

    let result = open_relative(
        &parent,
        OsStr::new("vanished-directory"),
        FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
        FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_REPARSE_POINT_OPTION,
    );
    assert!(
        !path.exists(),
        "relative open recreated the disappeared directory"
    );
    assert!(matches!(result, Err(PortError::NotFound)));
}
