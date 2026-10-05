//! Only unsafe boundary. Each public-to-parent wrapper performs one syscall.
use crate::{HandleLease, ObjectIdentity, PortError, UncheckedReason};
use std::ffi::c_void;
use std::mem::{MaybeUninit, size_of};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::GetLastError;
use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileInformationByHandleEx, GetVolumeInformationByHandleW, QueryDosDeviceW,
};

pub(super) const DIRECTORY: u32 = 0x10;
pub(super) const EXCLUDED: u32 = 0x400 | 0x1000 | 0x40000 | 0x400000;
const ACCESS: u32 = 0x80 | 0x100000; // READ_ATTRIBUTES | SYNCHRONIZE, no data/list rights
const OPTIONS: u32 = 0x20 | 0x200000 | 0x400000; // synchronous | no-follow | no-recall
const OBJ_DONT_REPARSE: u32 = 0x1000;
const OBJ_CASE_INSENSITIVE: u32 = 0x40;

pub(super) struct Handle {
    // Declaration order: OS handle closes before releasing the budget token.
    owned: OwnedHandle,
    _lease: HandleLease,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Basic {
    creation: i64,
    access: i64,
    pub modified: i64,
    change: i64,
    pub attributes: u32,
    reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Standard {
    allocation: i64,
    pub size: i64,
    links: u32,
    pub delete_pending: u8,
    pub directory: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Id {
    volume: u64,
    file: [u8; 16],
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Case {
    flags: u32,
}
#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: *mut c_void,
    name: *mut UnicodeString,
    attributes: u32,
    security: *mut c_void,
    qos: *mut c_void,
}
#[repr(C)]
struct IoStatus {
    status_union: usize,
    information: usize,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtCreateFile(
        handle: *mut *mut c_void,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatus,
        allocation: *mut i64,
        file_attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
        ea: *mut c_void,
        ea_length: u32,
    ) -> i32;
    fn NtQueryAttributesFile(attributes: *mut ObjectAttributes, basic: *mut Basic) -> i32;
}

pub(super) fn unchecked(reason: UncheckedReason) -> PortError {
    PortError::Unchecked(reason)
}
fn win_error() -> PortError {
    match unsafe { GetLastError() } {
        5 | 32 => unchecked(UncheckedReason::AccessDenied),
        _ => unchecked(UncheckedReason::UnqualifiedFilesystem),
    }
}
fn nt_error(status: i32) -> PortError {
    match status as u32 {
        0xC000_0022 | 0xC000_0043 => unchecked(UncheckedReason::AccessDenied),
        0xC000_050B | 0xC000_0279 | 0xC000_0280 | 0xC000_0267 => {
            unchecked(UncheckedReason::ReparseOrOffline)
        }
        0xC000_0033 | 0xC000_0106 => unchecked(UncheckedReason::UnsupportedPathSyntax),
        _ => unchecked(UncheckedReason::UnqualifiedFilesystem),
    }
}

fn name_call<T>(
    parent: Option<&Handle>,
    name: &str,
    insensitive: bool,
    call: impl FnOnce(&mut ObjectAttributes) -> Result<T, PortError>,
) -> Result<T, PortError> {
    let mut wide: Vec<u16> = name.encode_utf16().collect();
    let length = u16::try_from(
        wide.len()
            .checked_mul(2)
            .ok_or_else(|| unchecked(UncheckedReason::LimitReached))?,
    )
    .map_err(|_| unchecked(UncheckedReason::LimitReached))?;
    if length == 0 {
        return Err(unchecked(UncheckedReason::UnsupportedPathSyntax));
    }
    let mut unicode = UnicodeString {
        length,
        maximum_length: length,
        buffer: wide.as_mut_ptr(),
    };
    let mut object = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root: parent.map_or(null_mut(), |h| h.owned.as_raw_handle()),
        name: &mut unicode,
        attributes: OBJ_DONT_REPARSE | if insensitive { OBJ_CASE_INSENSITIVE } else { 0 },
        security: null_mut(),
        qos: null_mut(),
    };
    call(&mut object)
}

pub(super) fn open(
    parent: Option<&Handle>,
    name: &str,
    insensitive: bool,
    directory: bool,
    lease: HandleLease,
) -> Result<Handle, PortError> {
    name_call(parent, name, insensitive, |attributes| {
        let mut raw = null_mut();
        let mut io = IoStatus {
            status_union: 0,
            information: 0,
        };
        // FILE_OPEN (1), not Win32 OPEN_EXISTING (3): no creation/overwrite.
        let status = unsafe {
            NtCreateFile(
                &mut raw,
                ACCESS,
                attributes,
                &mut io,
                null_mut(),
                0,
                if directory { 3 } else { 7 },
                1,
                // FILE_DIRECTORY_FILE is incompatible with NO_RECALL. A
                // directory is attested from the opened handle before use.
                OPTIONS | if directory { 0 } else { 0x40 },
                null_mut(),
                0,
            )
        };
        // Wrap every returned valid handle before checking status. Post-operation
        // cancellation and error paths then close it through RAII.
        let handle = if raw.is_null() || raw == -1isize as *mut c_void {
            None
        } else {
            Some(Handle {
                owned: unsafe { OwnedHandle::from_raw_handle(raw) },
                _lease: lease,
            })
        };
        if status != 0 {
            return Err(nt_error(status));
        }
        handle.ok_or_else(|| unchecked(UncheckedReason::UnqualifiedFilesystem))
    })
}

/// Exact child metadata, no handle/open/content. Only NAME_NOT_FOUND can attest
/// absence; PATH_NOT_FOUND and every other error remain unqualified.
pub(super) fn attributes(
    parent: &Handle,
    name: &str,
    insensitive: bool,
) -> Result<Option<Basic>, PortError> {
    name_call(Some(parent), name, insensitive, |object| {
        let mut basic = Basic::default();
        let status = unsafe { NtQueryAttributesFile(object, &mut basic) };
        match status as u32 {
            0 => Ok(Some(basic)),
            0xC000_0034 => Ok(None),
            _ => Err(nt_error(status)),
        }
    })
}

fn query<T: Copy>(handle: &Handle, class: i32) -> Result<T, PortError> {
    let mut result = MaybeUninit::<T>::zeroed();
    // Called only for the audited fixed POD layouts below; no invalid Rust
    // discriminant/reference values are constructed from kernel output.
    if unsafe {
        GetFileInformationByHandleEx(
            handle.owned.as_raw_handle(),
            class,
            result.as_mut_ptr().cast(),
            size_of::<T>() as u32,
        )
    } == 0
    {
        return Err(win_error());
    }
    Ok(unsafe { result.assume_init() })
}
pub(super) fn basic(handle: &Handle) -> Result<Basic, PortError> {
    query(handle, 0)
}
pub(super) fn standard(handle: &Handle) -> Result<Standard, PortError> {
    query(handle, 1)
}
pub(super) fn identity(handle: &Handle) -> Result<ObjectIdentity, PortError> {
    let id: Id = query(handle, 18)?;
    Ok(ObjectIdentity {
        volume: id.volume,
        file: u128::from_le_bytes(id.file),
    })
}
pub(super) fn case_sensitive(handle: &Handle) -> Result<bool, PortError> {
    let case: Case =
        query(handle, 23).map_err(|_| unchecked(UncheckedReason::UnsupportedCaseMode))?;
    if case.flags > 1 {
        return Err(unchecked(UncheckedReason::UnsupportedCaseMode));
    }
    Ok(case.flags == 1)
}
pub(super) fn filesystem(handle: &Handle) -> Result<(), PortError> {
    let mut name = [0u16; 32];
    if unsafe {
        GetVolumeInformationByHandleW(
            handle.owned.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            name.as_mut_ptr(),
            name.len() as u32,
        )
    } == 0
    {
        return Err(win_error());
    }
    let end = name
        .iter()
        .position(|c| *c == 0)
        .ok_or_else(|| unchecked(UncheckedReason::UnqualifiedFilesystem))?;
    if String::from_utf16(&name[..end]).ok().as_deref() != Some("NTFS") {
        return Err(unchecked(UncheckedReason::UnqualifiedFilesystem));
    }
    Ok(())
}
pub(super) fn drive_mapping(drive: u8) -> Result<String, PortError> {
    let name = [drive as u16, b':' as u16, 0];
    let mut target = [0u16; 1024];
    if unsafe { QueryDosDeviceW(name.as_ptr(), target.as_mut_ptr(), target.len() as u32) } == 0 {
        return Err(win_error());
    }
    let end = target
        .iter()
        .position(|c| *c == 0)
        .ok_or_else(|| unchecked(UncheckedReason::UnqualifiedFilesystem))?;
    let mapping = String::from_utf16(&target[..end])
        .map_err(|_| unchecked(UncheckedReason::UnqualifiedFilesystem))?;
    // No SUBST, redirected, remote or arbitrary device paths. Open the resolved
    // local volume root directly, then recheck the DOS binding before return.
    if !mapping
        .strip_prefix(r"\Device\HarddiskVolume")
        .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(unchecked(UncheckedReason::UnqualifiedFilesystem));
    }
    Ok(mapping)
}
pub(super) fn equal_ignoring_case(left: &str, right: &str) -> Result<bool, PortError> {
    let left: Vec<u16> = left.encode_utf16().collect();
    let right: Vec<u16> = right.encode_utf16().collect();
    let result = unsafe {
        CompareStringOrdinal(
            left.as_ptr(),
            left.len() as i32,
            right.as_ptr(),
            right.len() as i32,
            1,
        )
    };
    if result == 0 {
        return Err(unchecked(UncheckedReason::UnsupportedCaseMode));
    }
    Ok(result == CSTR_EQUAL)
}

#[cfg(test)]
pub(super) fn content_read_is_denied(handle: &Handle) -> bool {
    use windows_sys::Win32::Storage::FileSystem::ReadFile;
    let mut byte = [0u8];
    let mut read = 0u32;
    unsafe {
        ReadFile(
            handle.owned.as_raw_handle(),
            byte.as_mut_ptr().cast(),
            1,
            &mut read,
            null_mut(),
        ) == 0
            && GetLastError() == 5
    }
}

#[cfg(test)]
pub(super) fn independent_test_handle(file: std::fs::File, lease: HandleLease) -> Handle {
    Handle {
        owned: file.into(),
        _lease: lease,
    }
}

#[cfg(test)]
pub(super) fn set_case_sensitive(file: &std::fs::File) -> bool {
    use windows_sys::Win32::Storage::FileSystem::SetFileInformationByHandle;
    let case = Case { flags: 1 };
    unsafe {
        SetFileInformationByHandle(
            file.as_raw_handle(),
            23,
            (&case as *const Case).cast(),
            size_of::<Case>() as u32,
        ) != 0
    }
}

#[cfg(test)]
pub(super) fn junction(link: &std::path::Path, target: &std::path::Path) {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    std::fs::create_dir(link).unwrap();
    let file = std::fs::OpenOptions::new()
        .access_mode(0x40000000)
        .share_mode(7)
        .custom_flags(0x02000000 | 0x00200000)
        .open(link)
        .unwrap();
    let target = target.to_str().unwrap();
    let substitute: Vec<u16> = format!("\\??\\{target}").encode_utf16().collect();
    let print: Vec<u16> = target.encode_utf16().collect();
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&0xA0000003u32.to_le_bytes());
    buffer.extend_from_slice(
        &(8u16 + ((substitute.len() + print.len() + 2) * 2) as u16).to_le_bytes(),
    );
    buffer.extend_from_slice(&0u16.to_le_bytes());
    buffer.extend_from_slice(&0u16.to_le_bytes());
    buffer.extend_from_slice(&((substitute.len() * 2) as u16).to_le_bytes());
    buffer.extend_from_slice(&((substitute.len() * 2 + 2) as u16).to_le_bytes());
    buffer.extend_from_slice(&((print.len() * 2) as u16).to_le_bytes());
    for value in substitute
        .into_iter()
        .chain(Some(0))
        .chain(print)
        .chain(Some(0))
    {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let mut written = 0u32;
    assert_ne!(
        unsafe {
            DeviceIoControl(
                file.as_raw_handle(),
                0x000900A4,
                buffer.as_ptr().cast(),
                buffer.len() as u32,
                null_mut(),
                0,
                &mut written,
                null_mut(),
            )
        },
        0,
        "owned fixture junction creation failed"
    );
}

#[cfg(test)]
pub(super) struct RestoreAcl {
    path: Vec<u16>,
    original: Vec<u64>,
    protected: bool,
}
#[cfg(test)]
impl Drop for RestoreAcl {
    fn drop(&mut self) {
        use windows_sys::Win32::Security::SetFileSecurityW;
        assert_ne!(
            unsafe {
                SetFileSecurityW(
                    self.path.as_ptr(),
                    4 | if self.protected {
                        0x80000000
                    } else {
                        0x20000000
                    },
                    self.original.as_mut_ptr().cast(),
                )
            },
            0,
            "owned fixture ACL restoration failed"
        );
    }
}
#[cfg(test)]
pub(super) fn deny_read_attributes(path: &std::path::Path) -> RestoreAcl {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::{
        GetFileSecurityW, GetSecurityDescriptorControl, SetFileSecurityW,
    };
    let path: Vec<u16> = path
        .to_str()
        .unwrap()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut needed = 0u32;
    unsafe {
        GetFileSecurityW(path.as_ptr(), 4, null_mut(), 0, &mut needed);
    }
    assert!(needed > 0 && needed <= 65536);
    // u64 allocation provides sufficient SECURITY_DESCRIPTOR alignment.
    let mut original = vec![0u64; (needed as usize).div_ceil(8)];
    assert_ne!(
        unsafe {
            GetFileSecurityW(
                path.as_ptr(),
                4,
                original.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        },
        0
    );
    let mut control = 0u16;
    let mut revision = 0u32;
    assert_ne!(
        unsafe {
            GetSecurityDescriptorControl(original.as_mut_ptr().cast(), &mut control, &mut revision)
        },
        0
    );
    let restore = RestoreAcl {
        path,
        original,
        protected: control & 0x1000 != 0,
    };
    let sddl: Vec<u16> = "D:P(D;;FA;;;WD)".encode_utf16().chain(Some(0)).collect();
    let mut descriptor = null_mut();
    assert_ne!(
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut descriptor,
                null_mut(),
            )
        },
        0
    );
    let result = unsafe { SetFileSecurityW(restore.path.as_ptr(), 4 | 0x80000000, descriptor) };
    unsafe {
        LocalFree(descriptor);
    }
    assert_ne!(result, 0, "owned fixture permission setup failed");
    restore
}
