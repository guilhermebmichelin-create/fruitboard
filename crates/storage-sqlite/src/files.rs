use crate::{Result, StorageError};
use std::fs::{self, File, OpenOptions};
use std::path::{Component, Path, PathBuf};

pub(crate) const DATABASE_NAME: &str = "fruitboard.db";
const DATABASE_FILES: [&str; 4] = [
    DATABASE_NAME,
    "fruitboard.db-journal",
    "fruitboard.db-wal",
    "fruitboard.db-shm",
];

// These checks reject pre-existing links/reparse points. The per-user directory
// is the access boundary; this does not defend against another process running
// as the same user racing filesystem operations.
pub(crate) fn check_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(StorageError::UnsafeLocation);
    }
    #[cfg(windows)]
    {
        use std::path::Prefix;
        if !matches!(path.components().next(), Some(Component::Prefix(prefix))
            if matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)))
        {
            return Err(StorageError::UnsafeLocation);
        }
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(StorageError::UnsafeLocation);
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        return Err(StorageError::UnsafeLocation);
                    }
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.is_file() && metadata.nlink() != 1 {
                        return Err(StorageError::UnsafeLocation);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(crate) fn private_directory(path: &Path) -> Result<()> {
    check_path(path)?;
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub(crate) fn private_file(path: &Path, new: bool) -> Result<File> {
    check_path(path)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if new {
        options.create_new(true);
    } else {
        options.create(true).truncate(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(file)
}

pub(crate) struct Location {
    pub(crate) directory: PathBuf,
    // OS lock is released even if a process terminates without unwinding.
    pub(crate) _lock: File,
}

impl Location {
    pub(crate) fn acquire(app_data: &Path) -> Result<Self> {
        let directory = app_data.join("storage");
        private_directory(&directory)?;
        let lock = private_file(&directory.join("owner.lock"), false)?;
        lock.try_lock().map_err(|_| StorageError::Busy)?;
        for name in DATABASE_FILES {
            check_path(&directory.join(name))?;
        }
        Ok(Self {
            directory,
            _lock: lock,
        })
    }
    pub(crate) fn database(&self) -> PathBuf {
        self.directory.join(DATABASE_NAME)
    }

    pub(crate) fn ensure_recovery_destination_empty(&self) -> Result<()> {
        // A database and its companions are one recovery unit. An orphan hot
        // journal can overwrite restored pages on the first SQLite read.
        // Refuse every existing entry, including empty files, without opening
        // it in SQLite or deleting evidence needed for recovery.
        for name in DATABASE_FILES {
            match fs::symlink_metadata(self.directory.join(name)) {
                Ok(_) => return Err(StorageError::UnsafeLocation),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
