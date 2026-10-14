//! Windows path admission (contracts/cli/v1alpha1/semantics.md, "Windows
//! platform guarantees"). Ownership and permission are read from the opened
//! handle: the owner SID and the discretionary access control list stand in for
//! the owner uid and the mode bits. Every component is opened with
//! `FILE_FLAG_OPEN_REPARSE_POINT` and refused when it is a reparse point, so no
//! symbolic link or junction is followed.
//!
//! Windows has no `openat`. A held directory is opened without
//! `FILE_SHARE_DELETE`, so it cannot be renamed or removed while held, and a
//! child is opened by the held directory's own final path joined with one
//! literal name (`\\?\` paths are not normalized).
use super::super::{Failure, Result};
use std::{
    ffi::{OsStr, OsString, c_void},
    fs::{File, OpenOptions},
    io::Write,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    sync::OnceLock,
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_DELETE_PENDING,
        ERROR_FILE_EXISTS, GENERIC_ALL, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
    },
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_REVISION, AddAccessAllowedAceEx,
        Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
        CONTAINER_INHERIT_ACE, CreateWellKnownSid, DACL_SECURITY_INFORMATION, EqualSid,
        GetLengthSid, GetTokenInformation, INHERIT_ONLY_ACE, InitializeAcl,
        InitializeSecurityDescriptor, OBJECT_INHERIT_ACE, OWNER_SECURITY_INFORMATION, PSID,
        SE_DACL_PROTECTED, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, SECURITY_MAX_SID_SIZE,
        SetSecurityDescriptorControl, SetSecurityDescriptorDacl, SetSecurityDescriptorOwner,
        TOKEN_ELEVATION, TOKEN_OWNER, TOKEN_QUERY, TOKEN_USER, TokenElevation, TokenOwner,
        TokenUser, WinBuiltinAdministratorsSid, WinLocalSystemSid,
    },
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CREATE_NEW, CreateDirectoryW, CreateFileW, DELETE, DeleteFileW,
        FILE_ALL_ACCESS, FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_DELETE_CHILD, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_STANDARD_INFO, FILE_WRITE_ATTRIBUTES,
        FILE_WRITE_DATA, FILE_WRITE_EA, FileStandardInfo, GetFileInformationByHandle,
        GetFileInformationByHandleEx, GetFinalPathNameByHandleW, GetVolumeInformationByHandleW,
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW, READ_CONTROL, SYNCHRONIZE,
        WRITE_DAC, WRITE_OWNER,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

/// `ACCESS_ALLOWED_ACE_TYPE` and its callback form carry a SID at `SidStart`.
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const ACCESS_ALLOWED_CALLBACK_ACE_TYPE: u8 = 9;
/// Allow entries whose SID this module does not parse: compound, object and
/// callback object. Any one present refuses, never passes unread.
const UNPARSED_ALLOW_ACE_TYPES: [u8; 3] = [4, 5, 11];
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
const MAXIMUM_ALLOWED: u32 = 0x0200_0000;
/// The rights that let a trustee replace, remove, relink or re-permission a
/// directory's existing entries or the directory itself, the counterpart of
/// group or other write on Linux. `FILE_ADD_SUBDIRECTORY` alone only adds new
/// names, which cannot shadow an existing one; a volume root grants it to
/// every authenticated user.
const ANCESTOR_WRITE: u32 = GENERIC_WRITE
    | GENERIC_ALL
    | MAXIMUM_ALLOWED
    | DELETE
    | WRITE_DAC
    | WRITE_OWNER
    | FILE_DELETE_CHILD
    | FILE_WRITE_DATA
    | FILE_WRITE_EA
    | FILE_WRITE_ATTRIBUTES;
/// NT SERVICE\TrustedInstaller, the owner of a volume root and of system
/// directories: S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464.
const TRUSTED_INSTALLER: [u8; 32] = {
    let mut sid = [0u8; 32];
    sid[0] = 1;
    sid[1] = 6;
    sid[7] = 5;
    let parts: [u32; 6] = [
        80,
        956_008_885,
        3_418_522_649,
        1_831_038_044,
        1_853_292_631,
        2_271_478_464,
    ];
    let mut index = 0;
    while index < 6 {
        let bytes = parts[index].to_le_bytes();
        sid[8 + index * 4] = bytes[0];
        sid[9 + index * 4] = bytes[1];
        sid[10 + index * 4] = bytes[2];
        sid[11 + index * 4] = bytes[3];
        index += 1;
    }
    sid
};

/// A SID held in owned, `u32`-aligned storage.
#[derive(Clone)]
pub(crate) struct Sid(Vec<u32>);
impl Sid {
    fn from_raw(sid: PSID) -> Option<Self> {
        // SAFETY: the caller passes a valid SID; its length bounds the copy.
        let length = unsafe { GetLengthSid(sid) } as usize;
        if length == 0 || length > SECURITY_MAX_SID_SIZE as usize {
            return None;
        }
        let mut storage = vec![0u32; length.div_ceil(4)];
        // SAFETY: both regions are at least `length` bytes and do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(sid as *const u8, storage.as_mut_ptr().cast(), length)
        };
        Some(Self(storage))
    }
    fn well_known(kind: i32) -> Option<Self> {
        let mut storage = vec![0u32; (SECURITY_MAX_SID_SIZE as usize).div_ceil(4)];
        let mut size = SECURITY_MAX_SID_SIZE;
        // SAFETY: the buffer holds SECURITY_MAX_SID_SIZE bytes, passed as its size.
        let ok = unsafe {
            CreateWellKnownSid(
                kind,
                std::ptr::null_mut(),
                storage.as_mut_ptr().cast(),
                &mut size,
            )
        };
        (ok != 0).then_some(Self(storage))
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut storage = vec![0u32; bytes.len().div_ceil(4)];
        // SAFETY: storage holds at least bytes.len() bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), storage.as_mut_ptr().cast(), bytes.len())
        };
        Self(storage)
    }
    pub(crate) fn as_psid(&self) -> PSID {
        self.0.as_ptr() as PSID
    }
    fn equals(&self, other: PSID) -> bool {
        // SAFETY: both are valid SIDs for the duration of the call.
        unsafe { EqualSid(self.as_psid(), other) != 0 }
    }
    /// The relative identifier: the last subauthority.
    fn rid(&self) -> Option<u32> {
        let bytes: &[u8] =
            // SAFETY: the storage is plain bytes of at least the SID's length.
            unsafe { std::slice::from_raw_parts(self.0.as_ptr().cast(), self.0.len() * 4) };
        let count = usize::from(*bytes.get(1)?);
        let start = 8 + count.checked_sub(1)? * 4;
        Some(u32::from_le_bytes(
            bytes.get(start..start + 4)?.try_into().ok()?,
        ))
    }
}

/// The current user's SID, from the process token.
pub(crate) fn current_user() -> Result<&'static Sid> {
    static USER: OnceLock<Option<Sid>> = OnceLock::new();
    USER.get_or_init(|| {
        let mut token: HANDLE = std::ptr::null_mut();
        // SAFETY: the pseudo handle of this process and a writable out pointer.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return None;
        }
        let mut buffer = vec![0u64; 64];
        let mut length = 0u32;
        // SAFETY: the buffer is 512 bytes, passed as its size, and aligned for TOKEN_USER.
        let ok = unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                (buffer.len() * 8) as u32,
                &mut length,
            )
        };
        // SAFETY: the token handle was opened above and is closed once.
        unsafe { CloseHandle(token) };
        if ok == 0 {
            return None;
        }
        // SAFETY: a successful TokenUser query wrote a TOKEN_USER at the start.
        let user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
        Sid::from_raw(user.User.Sid)
    })
    .as_ref()
    .ok_or(Failure::InvalidConfiguration)
}

fn local_system() -> Result<&'static Sid> {
    static SID: OnceLock<Option<Sid>> = OnceLock::new();
    SID.get_or_init(|| Sid::well_known(WinLocalSystemSid))
        .as_ref()
        .ok_or(Failure::InvalidConfiguration)
}

fn administrators() -> Result<&'static Sid> {
    static SID: OnceLock<Option<Sid>> = OnceLock::new();
    SID.get_or_init(|| Sid::well_known(WinBuiltinAdministratorsSid))
        .as_ref()
        .ok_or(Failure::InvalidConfiguration)
}

fn trusted_installer() -> &'static Sid {
    static SID: OnceLock<Sid> = OnceLock::new();
    SID.get_or_init(|| Sid::from_bytes(&TRUSTED_INSTALLER))
}

/// The configured owner number on Windows: the relative identifier of the
/// current user's SID. It is a recorded label only; every admission compares
/// the full owner SID on the opened handle. `u32::MAX` when the token cannot
/// be read, which no admission then accepts because the SID check fails.
pub fn uid() -> u32 {
    current_user().ok().and_then(Sid::rid).unwrap_or(u32::MAX)
}

/// The trustees an owner-only object may name: the current user, LocalSystem
/// and BUILTIN\Administrators, which read whatever the list says, as root does
/// on Linux whatever the mode bits say.
fn owner_only_trustee(sid: PSID) -> Result<bool> {
    Ok(current_user()?.equals(sid) || local_system()?.equals(sid) || administrators()?.equals(sid))
}

/// Whether `owner` counts as the current user's for the owner-only rule:
/// the user's own SID, or BUILTIN\Administrators when this process's token
/// is elevated and makes that group the default owner of what it creates, as
/// Windows does in an elevated administrator session for files a library
/// creates without an explicit owner (SQLite's `-wal` and `-shm`, for one).
/// Otherwise an Administrators owner refuses, as before.
fn admitted_owner(owner: PSID) -> Result<bool> {
    Ok(current_user()?.equals(owner)
        || (administrators()?.equals(owner) && elevated_administrators_owner()))
}

/// Whether this process's token is elevated and names BUILTIN\Administrators
/// as the default owner of objects it creates. Read once; a token that cannot
/// be read is neither.
fn elevated_administrators_owner() -> bool {
    static ELEVATED: OnceLock<bool> = OnceLock::new();
    *ELEVATED.get_or_init(|| {
        let mut token: HANDLE = std::ptr::null_mut();
        // SAFETY: the pseudo handle of this process and a writable out pointer.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut length = 0u32;
        // SAFETY: a live token and a TOKEN_ELEVATION of the size passed.
        let elevated = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut length,
            )
        } != 0
            && elevation.TokenIsElevated != 0;
        let mut buffer = vec![0u64; 16];
        // SAFETY: the buffer is 128 bytes, passed as its size, and aligned
        // for TOKEN_OWNER followed by the SID it points at.
        let owned = elevated
            && unsafe {
                GetTokenInformation(
                    token,
                    TokenOwner,
                    buffer.as_mut_ptr().cast(),
                    (buffer.len() * 8) as u32,
                    &mut length,
                )
            } != 0;
        // SAFETY: the token handle was opened above and is closed once.
        unsafe { CloseHandle(token) };
        owned
            && administrators().is_ok_and(|sid| {
                // SAFETY: a successful TokenOwner query wrote a TOKEN_OWNER at
                // the start, whose SID lies in the same buffer.
                let owner = unsafe { (*(buffer.as_ptr() as *const TOKEN_OWNER)).Owner };
                sid.equals(owner)
            })
    })
}

/// The owner SID and DACL of an opened handle.
struct Security {
    descriptor: *mut c_void,
    owner: PSID,
    dacl: *mut ACL,
}
impl Drop for Security {
    fn drop(&mut self) {
        // SAFETY: GetSecurityInfo allocated the descriptor with LocalAlloc.
        unsafe { LocalFree(self.descriptor) };
    }
}
impl Security {
    fn of(file: &File) -> Result<Self> {
        let mut owner: PSID = std::ptr::null_mut();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: *mut c_void = std::ptr::null_mut();
        // SAFETY: a live handle opened with READ_CONTROL and writable out pointers.
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if status != 0 {
            return Err(Failure::InvalidConfiguration);
        }
        let security = Self {
            descriptor,
            owner,
            dacl,
        };
        // A missing owner or a null DACL (which grants everyone everything) refuses.
        if security.owner.is_null() || security.dacl.is_null() {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(security)
    }

    /// Every allow entry that applies to this object, as (SID, access mask).
    /// Inherit-only entries apply to children only and are skipped; an allow
    /// entry this module does not parse refuses.
    fn allowed(&self) -> Result<Vec<(PSID, u32)>> {
        // SAFETY: the DACL is non-null and lives as long as the descriptor.
        let count = unsafe { (*self.dacl).AceCount };
        let mut entries = Vec::with_capacity(usize::from(count));
        for index in 0..u32::from(count) {
            let mut ace: *mut c_void = std::ptr::null_mut();
            // SAFETY: index is below the ACL's own entry count.
            if unsafe { windows_sys::Win32::Security::GetAce(self.dacl, index, &mut ace) } == 0 {
                return Err(Failure::InvalidConfiguration);
            }
            // SAFETY: every ACE starts with an ACE_HEADER.
            let header = unsafe { *(ace as *const ACE_HEADER) };
            if u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0 {
                continue;
            }
            if UNPARSED_ALLOW_ACE_TYPES.contains(&header.AceType) {
                return Err(Failure::InvalidConfiguration);
            }
            if header.AceType == ACCESS_ALLOWED_ACE_TYPE
                || header.AceType == ACCESS_ALLOWED_CALLBACK_ACE_TYPE
            {
                let entry = ace as *const ACCESS_ALLOWED_ACE;
                // SAFETY: both types lay out Header, Mask, then the SID at SidStart.
                let (mask, sid) =
                    unsafe { ((*entry).Mask, std::ptr::addr_of!((*entry).SidStart) as PSID) };
                entries.push((sid, mask));
            }
        }
        Ok(entries)
    }

    /// Owner-only: the current user owns it (see [`admitted_owner`]) and only
    /// the three admitted trustees are allowed any access.
    fn owner_only(&self) -> Result<()> {
        if !admitted_owner(self.owner)? {
            return Err(Failure::InvalidConfiguration);
        }
        for (sid, _) in self.allowed()? {
            if !owner_only_trustee(sid)? {
                return Err(Failure::InvalidConfiguration);
            }
        }
        Ok(())
    }

    /// A traversed directory: owned by the current user or the system, and no
    /// other trustee may replace, remove or re-permission its entries.
    fn trusted_ancestor(&self, forbidden: u32) -> Result<()> {
        let trusted = |sid: PSID| -> Result<bool> {
            Ok(owner_only_trustee(sid)? || trusted_installer().equals(sid))
        };
        if !trusted(self.owner)? {
            return Err(Failure::InvalidConfiguration);
        }
        for (sid, mask) in self.allowed()? {
            if mask & forbidden != 0 && !trusted(sid)? {
                return Err(Failure::InvalidConfiguration);
            }
        }
        Ok(())
    }
}

/// Volume serial, file index, attributes and link count of an opened handle.
pub(crate) struct Information {
    pub volume: u32,
    pub index: u64,
    pub attributes: u32,
    pub links: u32,
}
pub(crate) fn information(file: &File) -> Result<Information> {
    // SAFETY: zero is a valid BY_HANDLE_FILE_INFORMATION.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: a live handle and a writable structure.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, &mut info) } == 0 {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(Information {
        volume: info.dwVolumeSerialNumber,
        index: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
        attributes: info.dwFileAttributes,
        links: info.nNumberOfLinks,
    })
}

/// The identity a Unix caller reads as `(dev, ino)`.
pub(crate) fn identity(file: &File) -> Result<(u64, u64)> {
    let info = information(file)?;
    Ok((u64::from(info.volume), info.index))
}

/// The identity of the object at `path`, without following a final link.
pub(crate) fn path_identity(path: &Path) -> Option<(u64, u64)> {
    let file = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .ok()?;
    identity(&file).ok()
}

/// The final path of an opened handle, as `\\?\C:\…`.
pub(crate) fn final_path(file: &File) -> Result<PathBuf> {
    let mut buffer = vec![0u16; 512];
    loop {
        // SAFETY: a live handle and a writable buffer of the length passed.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle() as HANDLE,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                0,
            )
        } as usize;
        if length == 0 {
            return Err(Failure::InvalidConfiguration);
        }
        if length < buffer.len() {
            return Ok(OsString::from_wide(&buffer[..length]).into());
        }
        if length > 32_768 {
            return Err(Failure::InvalidConfiguration);
        }
        buffer.resize(length + 1, 0);
    }
}

/// One literal path component: no separators, no stream or drive colon, no
/// wildcard or control character, and no trailing dot or space.
pub(crate) fn name(value: &OsStr) -> Result<&OsStr> {
    let wide: Vec<u16> = value.encode_wide().collect();
    if wide.is_empty()
        || value == "."
        || value == ".."
        || wide.len() > 255
        || wide
            .iter()
            .any(|&unit| unit < 0x20 || (unit < 0x80 && b"/\\:*?\"<>|".contains(&(unit as u8))))
        || matches!(wide.last(), Some(&0x2e) | Some(&0x20))
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(value)
}

/// Absolute local paths only: a drive letter, the root, then plain names.
pub fn validate_path(path: &Path) -> Result<char> {
    let mut components = path.components();
    let drive = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) if letter.is_ascii_alphabetic() => {
                char::from(letter).to_ascii_uppercase()
            }
            _ => return Err(Failure::InvalidConfiguration),
        },
        _ => return Err(Failure::InvalidConfiguration),
    };
    if components.next() != Some(Component::RootDir)
        || path.as_os_str().len() > 4096
        || components.any(|part| match part {
            Component::Normal(part) => name(part).is_err(),
            _ => true,
        })
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(drive)
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

const DIRECTORY_ACCESS: u32 =
    FILE_LIST_DIRECTORY | READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE;

/// Open a directory without following a reparse point, held so that it cannot
/// be renamed or removed.
fn open_directory(path: &Path) -> std::io::Result<File> {
    let file = OpenOptions::new()
        .access_mode(DIRECTORY_ACCESS)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let info = information(&file).map_err(|_| std::io::Error::other("information"))?;
    if info.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || info.attributes & FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    Ok(file)
}

/// Open a child of a held directory for reading without following a reparse
/// point. The child may be shared for reading, writing and deletion, as an
/// SQLite database or a lock file is.
pub(crate) fn open_child(parent: &File, child: &OsStr, access: u32) -> std::io::Result<File> {
    let child = name(child).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let path = final_path(parent)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
        .join(child);
    let file = OpenOptions::new()
        .access_mode(access)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let info = information(&file).map_err(|_| std::io::Error::other("information"))?;
    if info.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    Ok(file)
}

fn admitted_directory(file: &File, private: bool) -> Result<()> {
    let security = Security::of(file)?;
    if private {
        security.owner_only()
    } else {
        security.trusted_ancestor(ANCESTOR_WRITE)
    }
}

/// An owner-only security descriptor naming only the current user, protected
/// from inheritance, for one creation call.
fn with_owner_only<T>(
    directory: bool,
    create: impl FnOnce(*const SECURITY_ATTRIBUTES, *mut ACL) -> T,
) -> Result<T> {
    let user = current_user()?;
    // SAFETY: the SID is valid.
    let sid_length = unsafe { GetLengthSid(user.as_psid()) } as usize;
    let acl_length = (std::mem::size_of::<ACL>() + std::mem::size_of::<ACCESS_ALLOWED_ACE>()
        - std::mem::size_of::<u32>()
        + sid_length)
        .next_multiple_of(4);
    let mut acl = vec![0u32; acl_length / 4];
    let acl_pointer = acl.as_mut_ptr() as *mut ACL;
    let inherit = if directory {
        OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE
    } else {
        0
    };
    // SAFETY: zero is a valid SECURITY_DESCRIPTOR to initialize.
    let mut descriptor: SECURITY_DESCRIPTOR = unsafe { std::mem::zeroed() };
    let descriptor_pointer = (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast::<c_void>();
    // SAFETY: the ACL buffer is acl_length bytes; the descriptor and SID are
    // live for every call, and the descriptor only points at them.
    let ok = unsafe {
        InitializeAcl(acl_pointer, acl_length as u32, ACL_REVISION) != 0
            && AddAccessAllowedAceEx(
                acl_pointer,
                ACL_REVISION,
                inherit,
                FILE_ALL_ACCESS,
                user.as_psid(),
            ) != 0
            && InitializeSecurityDescriptor(descriptor_pointer, SECURITY_DESCRIPTOR_REVISION) != 0
            && SetSecurityDescriptorOwner(descriptor_pointer, user.as_psid(), 0) != 0
            && SetSecurityDescriptorDacl(descriptor_pointer, 1, acl_pointer, 0) != 0
            && SetSecurityDescriptorControl(
                descriptor_pointer,
                SE_DACL_PROTECTED,
                SE_DACL_PROTECTED,
            ) != 0
    };
    if !ok {
        return Err(Failure::InvalidConfiguration);
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor_pointer,
        bInheritHandle: 0,
    };
    let result = create(&attributes, acl_pointer);
    drop(acl);
    Ok(result)
}

/// Make an existing file or directory owner-only: the current user as owner
/// and a protected list naming only that user. The test counterpart of
/// `chmod 0700` or `chmod 0600` on a fixture the test created itself.
#[cfg(test)]
pub(crate) fn owner_only(path: &Path) {
    use windows_sys::Win32::Security::{
        Authorization::SetNamedSecurityInfoW, PROTECTED_DACL_SECURITY_INFORMATION,
    };
    let directory = path.is_dir();
    let user = current_user().unwrap().as_psid();
    let name = wide(path);
    // SAFETY: a NUL-terminated path, a valid SID and the live owner-only ACL.
    let status = with_owner_only(directory, |_, acl| unsafe {
        SetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION
                | DACL_SECURITY_INFORMATION
                | PROTECTED_DACL_SECURITY_INFORMATION,
            user,
            std::ptr::null_mut(),
            acl,
            std::ptr::null(),
        )
    })
    .unwrap();
    assert_eq!(status, 0, "owner-only security on {}", path.display());
}

/// Allow Everyone to read a fixture: the test counterpart of `chmod 0644`.
#[cfg(test)]
pub(crate) fn grant_everyone_read(path: &Path) {
    let status = std::process::Command::new("icacls")
        .arg(path)
        .args(["/grant", "*S-1-1-0:(R)"])
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "icacls grant on {}", path.display());
}

/// Create one owner-only directory; an existing name is left to the caller's
/// admission.
fn create_directory(path: &Path) -> Result<()> {
    let path = wide(path);
    // SAFETY: a NUL-terminated path and live security attributes.
    let created = with_owner_only(true, |attributes, _| unsafe {
        CreateDirectoryW(path.as_ptr(), attributes)
    })?;
    if created == 0
        && std::io::Error::last_os_error().raw_os_error() != Some(ERROR_ALREADY_EXISTS as i32)
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Create a new owner-only file for writing, refusing any existing name.
pub(crate) fn create_new(path: &Path) -> std::io::Result<File> {
    let path = wide(path);
    // SAFETY: a NUL-terminated path and live security attributes; a returned
    // handle is new and owned.
    let handle = with_owner_only(false, |attributes, _| unsafe {
        CreateFileW(
            path.as_ptr(),
            GENERIC_WRITE | READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            0,
            attributes,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    })
    .map_err(|_| std::io::Error::from(std::io::ErrorKind::PermissionDenied))?;
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: CreateFileW returned a new owned handle.
    Ok(unsafe { File::from_raw_handle(handle as _) })
}

/// Open an admitted directory, optionally creating missing components
/// owner-only. Existing directories are never re-permissioned.
pub fn directory(path: &Path, create: bool, private: bool) -> Result<File> {
    let drive = validate_path(path)?;
    let mut current = open_directory(Path::new(&format!(r"\\?\{drive}:\")))
        .map_err(|_| Failure::InvalidConfiguration)?;
    admitted_directory(&current, false)?;
    for component in path.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        let here = final_path(&current)?.join(name(part)?);
        let next = match open_directory(&here) {
            Ok(next) => next,
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                create_directory(&here)?;
                open_directory(&here).map_err(|_| Failure::InvalidConfiguration)?
            }
            Err(_) => return Err(Failure::InvalidConfiguration),
        };
        admitted_directory(&next, false)?;
        current = next;
    }
    admitted_directory(&current, private)?;
    Ok(current)
}

/// A configured executable: a regular file, not a reparse point, owned by the
/// current user or the system, and writable by no other trustee (appending
/// included), the counterpart of a uid of the user or root and no group or
/// other write bit.
pub(crate) fn check_trusted_file(file: &File) -> Result<()> {
    let info = information(file)?;
    if info.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0 {
        return Err(Failure::InvalidConfiguration);
    }
    Security::of(file)?.trusted_ancestor(ANCESTOR_WRITE | FILE_APPEND_DATA)
}

/// The owner-only rule alone, on any opened handle: owned by the current user,
/// and no trustee but the three admitted is allowed access.
pub(crate) fn check_owner_only(file: &File) -> Result<()> {
    Security::of(file)?.owner_only()
}

/// Open a path for reading without following a final reparse point, which is
/// refused, as `O_NOFOLLOW` refuses a final symlink.
pub(crate) fn open_no_follow(path: &Path) -> std::io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let info = information(&file).map_err(|_| std::io::Error::other("information"))?;
    if info.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    Ok(file)
}

/// An owner-only regular file with one link, checked on its handle.
pub fn check_private_file(file: &File) -> Result<()> {
    let info = information(file)?;
    if info.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        || info.links != 1
    {
        return Err(Failure::InvalidConfiguration);
    }
    Security::of(file)?.owner_only()
}

pub fn private_file_at(parent: &File, child: &OsStr) -> Result<File> {
    let file = open_child(parent, child, windows_sys::Win32::Foundation::GENERIC_READ)
        .map_err(|_| Failure::InvalidConfiguration)?;
    check_private_file(&file)?;
    Ok(file)
}

/// The same check through a handle that reads no data, so it never conflicts
/// with an SQLite connection's share mode, reporting absence rather than
/// refusing it.
pub fn private_file_metadata_if_present_at(parent: &File, child: &OsStr) -> Result<bool> {
    match open_child(
        parent,
        child,
        READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
    ) {
        Ok(file) => check_private_file(&file).map(|()| true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(Failure::InvalidConfiguration),
    }
}

/// What one probe of a metadata-database sidecar found at its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sidecar {
    Absent,
    /// A file passing `check_private_file`.
    Private,
    /// A file being deleted. Under delete-on-close semantics its name stays
    /// until every handle to it closes, and opening it answers access denied
    /// meanwhile; a handle opened just before a deletion sees the file marked
    /// for deletion, or with no link left once the name is gone.
    Retiring,
}

/// `private_file_metadata_if_present_at` for a SQLite sidecar, which SQLite's
/// last close may be deleting while it is probed. Access denied on the open
/// is reported as `Retiring`, since that is how a file pending deletion
/// answers; a caller that keeps seeing it refuses. A present file that is not
/// retiring is checked in full.
pub(crate) fn sidecar_at(parent: &File, child: &OsStr) -> Result<Sidecar> {
    match open_child(
        parent,
        child,
        READ_CONTROL | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
    ) {
        Ok(file) if retiring(&file)? => Ok(Sidecar::Retiring),
        // A deletion between the probe above and this check leaves the handle
        // with no link, which the private check refuses: ask the handle again.
        Ok(file) => match check_private_file(&file) {
            Ok(()) => Ok(Sidecar::Private),
            Err(_) if retiring(&file)? => Ok(Sidecar::Retiring),
            Err(failure) => Err(failure),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Sidecar::Absent),
        Err(error)
            if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
                || error.raw_os_error() == Some(ERROR_DELETE_PENDING as i32) =>
        {
            Ok(Sidecar::Retiring)
        }
        Err(_) => Err(Failure::InvalidConfiguration),
    }
}

/// Whether an opened file is marked for deletion or has lost its last link.
fn retiring(file: &File) -> Result<bool> {
    // SAFETY: zero is a valid FILE_STANDARD_INFO.
    let mut info: FILE_STANDARD_INFO = unsafe { std::mem::zeroed() };
    // SAFETY: a live handle and a writable structure of the size passed.
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle() as HANDLE,
            FileStandardInfo,
            (&raw mut info).cast(),
            std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
        )
    };
    if ok == 0 {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(info.DeletePending || info.NumberOfLinks == 0)
}

/// Inspect a held directory without following or creating the destination. A
/// dangling link is an existing destination.
pub(crate) fn require_absent(parent: &File, child: &OsStr) -> Result<()> {
    match open_child(parent, child, FILE_READ_ATTRIBUTES | SYNCHRONIZE) {
        Ok(_) => Err(Failure::ConfigurationExists),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
            Err(Failure::ConfigurationExists)
        }
        Err(_) => Err(Failure::InvalidConfiguration),
    }
}

/// `atomic_no_replace_rename` holds on NTFS only: a held directory on any other
/// file system is refused.
fn qualify_ntfs(directory: &File) -> Result<()> {
    let mut name = [0u16; 261];
    // SAFETY: a live handle and a writable buffer of the length passed.
    let ok = unsafe {
        GetVolumeInformationByHandleW(
            directory.as_raw_handle() as HANDLE,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            name.as_mut_ptr(),
            name.len() as u32,
        )
    };
    let length = name
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(name.len());
    if ok == 0 || OsString::from_wide(&name[..length]) != "NTFS" {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Publish one durable owner-only file. Without `replace`, `MoveFileExW`
/// without `MOVEFILE_REPLACE_EXISTING` refuses an existing destination in the
/// same call; with it, a reader sees the old bytes or the new ones.
pub(crate) fn publish(parent: &File, child: &OsStr, contents: &[u8], replace: bool) -> Result<()> {
    if !replace {
        qualify_ntfs(parent)?;
    }
    let directory = final_path(parent)?;
    let temporary = directory.join(format!(".connectors-{}", uuid::Uuid::new_v4()));
    let target = directory.join(name(child)?);
    let mut file = create_new(&temporary).map_err(|_| Failure::InvalidConfiguration)?;
    let result = (|| {
        file.write_all(contents)
            .map_err(|_| Failure::InvalidConfiguration)?;
        file.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
        drop(file);
        let flags = MOVEFILE_WRITE_THROUGH
            | if replace {
                MOVEFILE_REPLACE_EXISTING
            } else {
                0
            };
        let (from, to) = (wide(&temporary), wide(&target));
        // SAFETY: two NUL-terminated paths.
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), flags) } == 0 {
            let code = std::io::Error::last_os_error().raw_os_error();
            return Err(
                if code == Some(ERROR_ALREADY_EXISTS as i32)
                    || code == Some(ERROR_FILE_EXISTS as i32)
                {
                    Failure::ConfigurationExists
                } else {
                    Failure::InvalidConfiguration
                },
            );
        }
        Ok(())
    })();
    // Only this call's unguessable temporary name is removed. After a
    // successful move it no longer exists; never remove the published file.
    let temporary = wide(&temporary);
    // SAFETY: a NUL-terminated path.
    unsafe { DeleteFileW(temporary.as_ptr()) };
    result
}

#[cfg(test)]
mod tests {
    //! Run on the Windows runner only: the owner-only rule on the handle, the
    //! reparse-point refusal and the no-replace rename.
    use super::*;
    use std::os::windows::process::CommandExt;

    fn root() -> (tempfile::TempDir, File, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        let directory = directory(&state, true, true).unwrap();
        (root, directory, state)
    }

    fn icacls(arguments: &[&str]) {
        let status = std::process::Command::new("icacls")
            .args(arguments)
            .creation_flags(0x0800_0000)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "icacls {arguments:?}");
    }

    /// A directory and a file the host creates are owner-only on their handles.
    #[test]
    fn created_owner_only_objects_are_admitted() {
        let (_root, directory, state) = root();
        admitted_directory(&directory, true).unwrap();
        publish(&directory, OsStr::new("config.toml"), b"one", false).unwrap();
        private_file_at(&directory, OsStr::new("config.toml")).unwrap();
        assert!(
            private_file_metadata_if_present_at(&directory, OsStr::new("config.toml")).unwrap()
        );
        assert!(!private_file_metadata_if_present_at(&directory, OsStr::new("absent")).unwrap());
        assert_eq!(std::fs::read(state.join("config.toml")).unwrap(), b"one");
    }

    /// An allow entry for any other trustee refuses the file, as group or
    /// other mode bits do on Linux.
    #[test]
    fn an_extra_trustee_is_refused() {
        let (_root, directory, state) = root();
        publish(&directory, OsStr::new("secret"), b"one", false).unwrap();
        let path = state.join("secret");
        icacls(&[path.to_str().unwrap(), "/grant", "*S-1-1-0:(R)"]);
        assert_eq!(
            private_file_at(&directory, OsStr::new("secret")).unwrap_err(),
            Failure::InvalidConfiguration
        );
        icacls(&[path.to_str().unwrap(), "/remove:g", "*S-1-1-0"]);
        private_file_at(&directory, OsStr::new("secret")).unwrap();
        icacls(&[state.to_str().unwrap(), "/grant", "*S-1-1-0:(R)"]);
        assert!(directory_reopen(&state, true).is_err());
    }

    fn directory_reopen(path: &Path, private: bool) -> Result<File> {
        super::directory(path, false, private)
    }

    /// The no-replace rename refuses an existing target and leaves its bytes;
    /// the replacing one replaces them and leaves no temporary file.
    #[test]
    fn the_no_replace_rename_refuses_an_existing_target() {
        let (_root, directory, state) = root();
        publish(&directory, OsStr::new("first"), b"one", false).unwrap();
        assert_eq!(
            publish(&directory, OsStr::new("first"), b"two", false),
            Err(Failure::ConfigurationExists)
        );
        assert_eq!(std::fs::read(state.join("first")).unwrap(), b"one");
        assert_eq!(
            require_absent(&directory, OsStr::new("first")),
            Err(Failure::ConfigurationExists)
        );
        require_absent(&directory, OsStr::new("second")).unwrap();
        publish(&directory, OsStr::new("first"), b"four", true).unwrap();
        assert_eq!(std::fs::read(state.join("first")).unwrap(), b"four");
        assert_eq!(std::fs::read_dir(&state).unwrap().count(), 1);
    }

    /// A junction on the path refuses the walk; a hard link refuses the file.
    #[test]
    fn reparse_points_and_hard_links_are_refused() {
        let (root, directory, state) = root();
        let junction = root.path().join("junction");
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&junction)
            .arg(&state)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        assert!(super::directory(&junction, false, true).is_err());
        publish(&directory, OsStr::new("one"), b"one", false).unwrap();
        std::fs::hard_link(state.join("one"), state.join("two")).unwrap();
        assert!(private_file_at(&directory, OsStr::new("one")).is_err());
    }

    /// Names that Windows would reinterpret are refused before any open.
    #[test]
    fn names_are_literal() {
        for bad in ["a:b", "a\\b", "a/b", "a.", "a ", "..", "."] {
            assert!(name(OsStr::new(bad)).is_err(), "{bad}");
        }
        name(OsStr::new("config.toml")).unwrap();
        assert!(validate_path(Path::new(r"\\server\share\x")).is_err());
        assert!(validate_path(Path::new("relative")).is_err());
        assert_eq!(validate_path(Path::new(r"c:\Users")).unwrap(), 'C');
    }

    /// An elevated administrator session makes BUILTIN\Administrators the
    /// owner of files created without an explicit owner. Such a file is
    /// admitted as the user's own exactly when the token is elevated and names
    /// that group as its default owner; otherwise the owner check refuses it.
    #[test]
    fn an_administrators_owner_is_admitted_only_in_an_elevated_session() {
        let (_root, directory, state) = root();
        publish(&directory, OsStr::new("owned"), b"one", false).unwrap();
        private_file_at(&directory, OsStr::new("owned")).unwrap();
        let path = state.join("owned");
        // Only a token in which the group is enabled can make it the owner.
        let set = std::process::Command::new("icacls")
            .arg(&path)
            .args(["/setowner", "*S-1-5-32-544"])
            .creation_flags(0x0800_0000)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success();
        let admitted = private_file_at(&directory, OsStr::new("owned")).is_ok();
        if set {
            assert_eq!(admitted, elevated_administrators_owner());
        } else {
            assert!(!elevated_administrators_owner());
            assert!(admitted, "the owner is still the user's own");
        }
        assert!(admitted_owner(current_user().unwrap().as_psid()).unwrap());
        assert!(!admitted_owner(local_system().unwrap().as_psid()).unwrap());
        assert_eq!(
            admitted_owner(administrators().unwrap().as_psid()).unwrap(),
            elevated_administrators_owner()
        );
    }

    /// The configured owner number is the user SID's relative identifier.
    #[test]
    fn the_owner_number_is_the_relative_identifier() {
        assert_ne!(uid(), u32::MAX);
        assert_eq!(Some(uid()), current_user().unwrap().rid());
        assert!(trusted_installer().rid() == Some(2_271_478_464));
    }
}
