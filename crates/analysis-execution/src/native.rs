//! Windows local NTFS source qualification on the held authorized object.
use super::{OpenSource, SourceAuthority, SourceObservation};
use fruitboard_flp_parser::supervisor::CancellationToken;
use fruitboard_storage::{AnalysisFailure, AnalysisSource};

pub struct WindowsAuthority;
pub struct WindowsSource {
    #[cfg(windows)]
    source: fruitboard_flp_parser::AuthorizedSource,
    #[cfg(windows)]
    identity: (u64, u128),
}
impl SourceAuthority for WindowsAuthority {
    type Guard = WindowsSource;
    fn open(
        &self,
        source: &AnalysisSource,
        cancellation: &CancellationToken,
    ) -> Result<WindowsSource, AnalysisFailure> {
        if cancellation.is_cancelled() {
            return Err(AnalysisFailure::Interrupted);
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::{
                GetDriveTypeW, GetFileInformationByHandleEx, GetVolumeInformationByHandleW,
                GetVolumePathNameW,
            };
            let guard = fruitboard_flp_parser::authorize_source(source.path(), source.root())
                .map_err(|_| AnalysisFailure::SourceUnavailable)?;
            let raw = guard.file().as_raw_handle();
            let mut filesystem = [0u16; 32];
            let mut volume_path = [0u16; 32768];
            let path: Vec<u16> = source
                .path()
                .as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect();
            // Handles pin every ancestor. Volume information is read from the
            // actual file handle; path lookup only establishes local drive type.
            unsafe {
                if GetVolumeInformationByHandleW(
                    raw,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    filesystem.as_mut_ptr(),
                    filesystem.len() as u32,
                ) == 0
                    || GetVolumePathNameW(
                        path.as_ptr(),
                        volume_path.as_mut_ptr(),
                        volume_path.len() as u32,
                    ) == 0
                    || GetDriveTypeW(volume_path.as_ptr()) != 3
                {
                    return Err(AnalysisFailure::SourceUnavailable);
                }
            }
            let end = filesystem
                .iter()
                .position(|c| *c == 0)
                .ok_or(AnalysisFailure::SourceUnavailable)?;
            if String::from_utf16(&filesystem[..end]).ok().as_deref() != Some("NTFS") {
                return Err(AnalysisFailure::SourceUnavailable);
            }
            #[repr(C)]
            struct FileId {
                volume: u64,
                id: [u8; 16],
            }
            let mut id = FileId {
                volume: 0,
                id: [0; 16],
            };
            unsafe {
                if GetFileInformationByHandleEx(
                    raw,
                    18,
                    (&mut id as *mut FileId).cast(),
                    std::mem::size_of::<FileId>() as u32,
                ) == 0
                {
                    return Err(AnalysisFailure::SourceUnavailable);
                }
            }
            let identity = (id.volume, u128::from_le_bytes(id.id));
            // Legacy/unqualified identity is deliberately ineligible for this
            // first content-reading slice; a new authoritative scan supplies it.
            if source.identity() != Some(identity) {
                return Err(AnalysisFailure::SourceChanged);
            }
            Ok(WindowsSource {
                source: guard,
                identity,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = source;
            Err(AnalysisFailure::SourceUnavailable)
        }
    }
}
impl OpenSource for WindowsSource {
    fn observe(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<SourceObservation, AnalysisFailure> {
        #[cfg(windows)]
        {
            use fruitboard_flp_parser::{MAX_FILE_BYTES, sha256_hex};
            use std::io::{Read, Seek, SeekFrom};
            use std::os::windows::fs::MetadataExt;
            use std::time::UNIX_EPOCH;
            let file = self.source.file();
            let before = file
                .metadata()
                .map_err(|_| AnalysisFailure::SourceUnavailable)?;
            if !before.is_file()
                || before.file_attributes() & (0x400 | 0x1000 | 0x40000 | 0x400000) != 0
                || before.len() > MAX_FILE_BYTES
            {
                return Err(AnalysisFailure::SourceUnavailable);
            }
            let ns = i64::try_from(
                before
                    .modified()
                    .map_err(|_| AnalysisFailure::SourceUnavailable)?
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| AnalysisFailure::SourceUnavailable)?
                    .as_nanos(),
            )
            .map_err(|_| AnalysisFailure::SourceUnavailable)?;
            let mut reader = file;
            reader
                .seek(SeekFrom::Start(0))
                .map_err(|_| AnalysisFailure::SourceUnavailable)?;
            let mut bytes = Vec::with_capacity(before.len() as usize);
            let mut buffer = [0u8; 65536];
            loop {
                if cancellation.is_cancelled() {
                    return Err(AnalysisFailure::Interrupted);
                }
                let read = reader
                    .read(&mut buffer)
                    .map_err(|_| AnalysisFailure::SourceUnavailable)?;
                if read == 0 {
                    break;
                }
                if bytes.len().saturating_add(read) > MAX_FILE_BYTES as usize {
                    return Err(AnalysisFailure::SourceChanged);
                }
                bytes.extend_from_slice(&buffer[..read]);
            }
            let after = file
                .metadata()
                .map_err(|_| AnalysisFailure::SourceUnavailable)?;
            if before.len() != bytes.len() as u64
                || after.len() != before.len()
                || after.modified().ok() != before.modified().ok()
            {
                return Err(AnalysisFailure::SourceChanged);
            }
            Ok(SourceObservation {
                byte_size: before.len(),
                modified_at_ns: ns,
                identity: Some(self.identity),
                sha256: sha256_hex(&bytes),
            })
        }
        #[cfg(not(windows))]
        {
            let _ = cancellation;
            Err(AnalysisFailure::SourceUnavailable)
        }
    }
}
