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

// These checks reject pre-existing links/reparse points. private_file also
// validates the opened leaf before permission changes or returning it for
// writes. The per-user directory remains the access boundary: pathname checks
// do not pin parent directories, and another process running as the same user
// can still race later filesystem operations or SQLite's separate open.
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
                    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
                    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                        return Err(StorageError::UnsafeLocation);
                    }
                    if metadata.is_file() {
                        // A metadata-only handle checks the existing object,
                        // including database/lock/backup hardlinks, before any
                        // caller opens it in SQLite or requests write access.
                        check_opened_file(&metadata_file(ancestor)?)?;
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

#[cfg(windows)]
fn metadata_file(path: &Path) -> Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
    };
    Ok(OpenOptions::new()
        .read(true)
        .access_mode(FILE_READ_ATTRIBUTES)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?)
}

fn check_opened_file(file: &File) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
            GetFileInformationByHandle,
        };
        // SAFETY: this POD output is initialized by the OS; the borrowed
        // File owns a live handle throughout the query. No mutation occurs.
        let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if information.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
            != 0
            || information.nNumberOfLinks != 1
        {
            return Err(StorageError::UnsafeLocation);
        }
    }
    #[cfg(not(windows))]
    {
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(StorageError::UnsafeLocation);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 1 {
                return Err(StorageError::UnsafeLocation);
            }
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
    private_file_inner(path, new, || {})
}

#[cfg(test)]
pub(crate) fn private_file_before_open(
    path: &Path,
    new: bool,
    before_open: impl FnOnce(),
) -> Result<File> {
    private_file_inner(path, new, before_open)
}

fn private_file_inner(path: &Path, new: bool, before_open: impl FnOnce()) -> Result<File> {
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
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    before_open();
    let file = options.open(path)?;
    check_opened_file(&file)?;
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

/// Only lock contention is retryable. Any other OS failure keeps its error
/// kind so a real filesystem problem is not mislabeled as a busy database.
pub(crate) fn lock_failure(error: std::fs::TryLockError) -> StorageError {
    match error {
        std::fs::TryLockError::WouldBlock => StorageError::Busy,
        std::fs::TryLockError::Error(error) => StorageError::from(error),
    }
}

impl Location {
    pub(crate) fn acquire(app_data: &Path) -> Result<Self> {
        let directory = app_data.join("storage");
        private_directory(&directory)?;
        let lock = private_file(&directory.join("owner.lock"), false)?;
        lock.try_lock().map_err(lock_failure)?;
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
