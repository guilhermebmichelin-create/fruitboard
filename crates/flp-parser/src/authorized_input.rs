//! Authorize the opened object before reading any FLP bytes.
use std::fs::{self, File, Metadata, OpenOptions};
use std::path::{Component, Path};

pub(crate) struct AuthorizedInput {
    pub(crate) file: File,
    // Windows handles deny write/delete sharing until parsing has finished.
    _directories: Vec<File>,
}

fn normal_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| match component {
            Component::ParentDir => false,
            #[cfg(windows)]
            Component::Prefix(prefix) => matches!(
                prefix.kind(),
                std::path::Prefix::Disk(_)
                    | std::path::Prefix::VerbatimDisk(_)
                    | std::path::Prefix::UNC(_, _)
                    | std::path::Prefix::VerbatimUNC(_, _)
            ),
            #[cfg(windows)]
            Component::Normal(name) => name.to_str().is_some_and(|name| {
                // Deny ADS and Win32 spellings that normalize to another name.
                !name.contains(':') && !name.ends_with(['.', ' '])
            }),
            _ => true,
        })
}

fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[cfg(windows)]
fn resident_attributes(attributes: u32) -> bool {
    // Reject offline/recall objects before requesting a read handle. Analysis
    // must not hydrate a placeholder or treat its virtual metadata as local.
    attributes & (0x1000 | 0x40000 | 0x400000) == 0
}

fn open_read(path: &Path, directory: bool) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        options.share_mode(FILE_SHARE_READ).custom_flags(
            FILE_FLAG_OPEN_REPARSE_POINT
                | if directory {
                    FILE_FLAG_BACKUP_SEMANTICS
                } else {
                    0
                },
        );
    }
    #[cfg(not(windows))]
    let _ = directory;
    options.open(path)
}

#[cfg(unix)]
fn same_object(first: &Metadata, second: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    first.dev() == second.dev() && first.ino() == second.ino()
}

#[cfg(windows)]
fn opened_path(_file: &File, path: &Path) -> std::io::Result<std::path::PathBuf> {
    // Every ancestor and the leaf is held without delete/write sharing. The
    // checked path cannot be replaced between resolution and the read.
    fs::canonicalize(path)
}

#[cfg(target_os = "linux")]
fn opened_path(file: &File, _path: &Path) -> std::io::Result<std::path::PathBuf> {
    use std::os::fd::AsRawFd;
    // Resolve the descriptor, not an independently reopened pathname.
    fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))?.canonicalize()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn opened_path(_file: &File, _path: &Path) -> std::io::Result<std::path::PathBuf> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unsupported authorization platform",
    ))
}

pub(crate) fn open(path: &Path, allowed_roots: &[String]) -> Result<AuthorizedInput, &'static str> {
    open_before_leaf(path, allowed_roots, || {})
}

fn open_before_leaf(
    path: &Path,
    allowed_roots: &[String],
    before_leaf: impl FnOnce(),
) -> Result<AuthorizedInput, &'static str> {
    if !normal_absolute(path) {
        return Err("INVALID_PATH");
    }
    let root = allowed_roots
        .iter()
        .map(Path::new)
        .find(|root| normal_absolute(root) && path != *root && path.starts_with(root))
        .ok_or("INVALID_PATH")?;
    let parents = path.ancestors().skip(1).collect::<Vec<_>>();
    let mut directories = Vec::with_capacity(parents.len());
    for parent in parents.iter().rev() {
        let before = fs::symlink_metadata(parent).map_err(|_| "INVALID_PATH")?;
        if is_link(&before) || !before.is_dir() {
            return Err("INVALID_PATH");
        }
        let directory = open_read(parent, true).map_err(|_| "INVALID_PATH")?;
        let metadata = directory.metadata().map_err(|_| "INVALID_PATH")?;
        if is_link(&metadata) || !metadata.is_dir() {
            return Err("INVALID_PATH");
        }
        #[cfg(unix)]
        if !same_object(&before, &metadata) {
            return Err("INVALID_PATH");
        }
        directories.push(directory);
    }
    before_leaf();
    let leaf = fs::symlink_metadata(path).map_err(|_| "INPUT_OPEN_FAILED")?;
    if is_link(&leaf) || !leaf.is_file() {
        return Err("INVALID_PATH");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if !resident_attributes(leaf.file_attributes()) {
            return Err("INVALID_PATH");
        }
    }
    let file = open_read(path, false).map_err(|_| "INPUT_OPEN_FAILED")?;
    let metadata = file.metadata().map_err(|_| "INVALID_PATH")?;
    if is_link(&metadata) || !metadata.is_file() {
        return Err("INVALID_PATH");
    }
    #[cfg(unix)]
    {
        if !same_object(&leaf, &metadata) {
            return Err("INVALID_PATH");
        }
        for (parent, directory) in parents.iter().rev().zip(&directories) {
            let current = fs::symlink_metadata(parent).map_err(|_| "INVALID_PATH")?;
            if is_link(&current)
                || !same_object(&current, &directory.metadata().map_err(|_| "INVALID_PATH")?)
            {
                return Err("INVALID_PATH");
            }
        }
    }
    let canonical_root = fs::canonicalize(root).map_err(|_| "INVALID_PATH")?;
    let actual = opened_path(&file, path).map_err(|_| "INVALID_PATH")?;
    if actual == canonical_root || !actual.starts_with(&canonical_root) {
        return Err("INVALID_PATH");
    }
    Ok(AuthorizedInput {
        file,
        _directories: directories,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn offline_and_recall_flags_are_denied_before_open() {
        assert!(resident_attributes(0x20));
        for flag in [0x1000, 0x40000, 0x400000] {
            assert!(!resident_attributes(flag | 0x20));
        }
    }

    #[cfg(windows)]
    #[test]
    fn authorization_handles_prevent_path_replacement_and_writes_until_drop() {
        let fixture = crate::test_directory("authorization-locks");
        let root = fixture.join("allowed");
        fs::create_dir(&root).unwrap();
        let path = root.join("project.flp");
        fs::write(&path, b"synthetic").unwrap();
        let input = open(&path, &[root.to_string_lossy().into_owned()]).unwrap();
        assert!(fs::rename(&root, fixture.join("moved")).is_err());
        assert!(fs::rename(&path, root.join("moved.flp")).is_err());
        assert!(OpenOptions::new().write(true).open(&path).is_err());
        drop(input);
        fs::rename(&root, fixture.join("moved")).unwrap();
        fs::remove_dir_all(&fixture).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn replacing_an_ancestor_with_an_outside_symlink_during_open_is_rejected() {
        let fixture = crate::test_directory("authorization-race");
        let root = fixture.join("allowed");
        let nested = root.join("nested");
        let outside = fixture.join("outside");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(nested.join("project.flp"), b"inside").unwrap();
        fs::write(outside.join("project.flp"), b"outside").unwrap();
        let result = open_before_leaf(
            &nested.join("project.flp"),
            &[root.to_string_lossy().into_owned()],
            || {
                fs::rename(&nested, root.join("original")).unwrap();
                std::os::unix::fs::symlink(&outside, &nested).unwrap();
            },
        );
        assert!(matches!(result, Err("INVALID_PATH")));
        fs::remove_dir_all(&fixture).unwrap();
    }
}
