//! The macOS binding: generic password items in the user's login keychain,
//! service `connectors`, account the purpose, scope and version ids
//! ([`Version::item_name`]). User interaction is disallowed for every call, so
//! a locked keychain answers `Unavailable` and never shows an unlock prompt.
//! Every item is created with an access whose decrypt ACLs admit any
//! application of the user, so an upgraded or rebuilt binary reads the items
//! an earlier one wrote without a prompt (contract §5.1). The data-protection
//! keychain needs a signed binary with entitlements and is not used.
//!
//! The keychain is injectable ([`Keychain::with_keychain`]) so a test can
//! create, lock and delete its own temporary keychain.
use crate::{
    Backend, CustodyBackend, Failure, MAX_BYTES, Material, ReadBackend, Result, Scope, State,
    Version, admit_material,
};
use core_foundation::{
    array::{CFArray, CFArrayRef},
    base::{CFType, CFTypeRef, TCFType, ToVoid},
    data::CFData,
    dictionary::CFMutableDictionary,
    string::{CFString, CFStringRef},
};
use security_framework::os::macos::{access::SecAccess, keychain::SecKeychain};
use security_framework_sys::{
    base::{SecAccessRef, SecKeychainRef, errSecDuplicateItem, errSecItemNotFound},
    item::{
        kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecUseKeychain,
        kSecValueData,
    },
    keychain::{SecKeychainGetUserInteractionAllowed, SecKeychainSetUserInteractionAllowed},
    keychain_item::{SecItemAdd, SecKeychainItemDelete},
};
use std::{
    ffi::CStr,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::{Mutex, MutexGuard, TryLockError},
    time::Instant,
};

/// The service every custody item carries.
pub const SERVICE: &str = "connectors";
// `security_framework_sys::base` imports `OSStatus` privately; it is `i32` in
// core-foundation-sys, as in the Security framework headers.
type OSStatus = i32;
const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;
const UNLOCKED: u32 = 1; // kSecUnlockStateStatus

type Found = (
    security_framework::os::macos::passwords::SecKeychainItemPassword,
    security_framework::os::macos::keychain_item::SecKeychainItem,
);

/// `SecACLRef`, an element of the array `SecAccessCopyMatchingACLList` returns.
type SecAclRef = CFTypeRef;
/// `SecKeychainPromptSelector`, a `uint16` in `SecBase.h`.
type PromptSelector = u16;

// SAFETY: declarations of Security framework symbols that
// security-framework-sys 2.17.0 does not bind, with the C signatures of
// `SecKeychain.h`, `SecAccess.h`, `SecACL.h` and `SecItem.h`.
#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecKeychainGetStatus(keychain: SecKeychainRef, status: *mut u32) -> OSStatus;
    #[cfg(any(test, feature = "test-support"))]
    fn SecKeychainLock(keychain: SecKeychainRef) -> OSStatus;
    #[cfg(any(test, feature = "test-support"))]
    fn SecKeychainDelete(keychain: SecKeychainRef) -> OSStatus;
    static kSecAttrAccess: CFStringRef;
    static kSecACLAuthorizationDecrypt: CFStringRef;
    fn SecAccessCreate(
        descriptor: CFStringRef,
        trusted_list: CFArrayRef,
        access: *mut SecAccessRef,
    ) -> OSStatus;
    fn SecAccessCopyMatchingACLList(access: SecAccessRef, authorization: CFTypeRef) -> CFArrayRef;
    fn SecACLCopyContents(
        acl: SecAclRef,
        application_list: *mut CFArrayRef,
        description: *mut CFStringRef,
        prompt_selector: *mut PromptSelector,
    ) -> OSStatus;
    fn SecACLSetContents(
        acl: SecAclRef,
        application_list: CFArrayRef,
        description: CFStringRef,
        prompt_selector: PromptSelector,
    ) -> OSStatus;
}

/// An item access whose decrypt ACLs admit any application: each ACL that
/// authorizes decrypt gets a NULL application list, which means any
/// application. An empty list would mean none. The other ACLs, the owner's
/// change-ACL authorization among them, keep what `SecAccessCreate` gave them.
fn any_application_access() -> Result<SecAccess> {
    let descriptor = CFString::new(SERVICE);
    let mut access: SecAccessRef = std::ptr::null_mut();
    // SAFETY: descriptor is live for the call, a NULL trusted list is
    // documented (the calling application), and access is a valid output.
    let status = unsafe {
        SecAccessCreate(
            descriptor.as_concrete_TypeRef(),
            std::ptr::null(),
            &mut access,
        )
    };
    if status != 0 || access.is_null() {
        return Err(Failure::Unavailable);
    }
    // SAFETY: SecAccessCreate returned the access retained (create rule).
    let access = unsafe { SecAccess::wrap_under_create_rule(access) };
    // SAFETY: the access is live and the tag is a Security framework constant.
    let acls = unsafe {
        SecAccessCopyMatchingACLList(
            access.as_concrete_TypeRef(),
            kSecACLAuthorizationDecrypt.cast(),
        )
    };
    if acls.is_null() {
        return Err(Failure::Unavailable);
    }
    // SAFETY: the copy function returned the array retained (create rule).
    let acls = unsafe { CFArray::<CFType>::wrap_under_create_rule(acls) };
    if acls.is_empty() {
        return Err(Failure::Unavailable);
    }
    for acl in acls.iter() {
        let acl = acl.as_CFTypeRef();
        let mut applications: CFArrayRef = std::ptr::null();
        let mut description: CFStringRef = std::ptr::null();
        let mut prompt: PromptSelector = 0;
        // SAFETY: acl is an element of the live array; the outputs are valid.
        let status =
            unsafe { SecACLCopyContents(acl, &mut applications, &mut description, &mut prompt) };
        // SAFETY: both outputs are returned retained (copy rule) or NULL.
        let _applications = (!applications.is_null())
            .then(|| unsafe { CFArray::<CFType>::wrap_under_create_rule(applications) });
        // SAFETY: as above.
        let description = (!description.is_null())
            .then(|| unsafe { CFString::wrap_under_create_rule(description) });
        if status != 0 {
            return Err(Failure::Unavailable);
        }
        let description = description.unwrap_or_else(|| descriptor.clone());
        // SAFETY: acl and description are live; a NULL application list
        // means any application.
        let status = unsafe {
            SecACLSetContents(
                acl,
                std::ptr::null(),
                description.as_concrete_TypeRef(),
                prompt,
            )
        };
        if status != 0 {
            return Err(Failure::Unavailable);
        }
    }
    Ok(access)
}

/// The writes and deletions of this process, one at a time. A second write of
/// a version from another process is refused by the keychain itself
/// (`errSecDuplicateItem`).
static WRITER: Mutex<()> = Mutex::new(());

/// User interaction is process-wide state. Every custody call holds it
/// disallowed; the last one out restores what it found.
static INTERACTION: Mutex<(usize, bool)> = Mutex::new((0, true));

struct NoInteraction;

impl NoInteraction {
    fn enter() -> Result<Self> {
        let mut state = INTERACTION.lock().map_err(|_| Failure::Unavailable)?;
        if state.0 == 0 {
            let mut allowed = 0;
            // SAFETY: allowed is a valid output location.
            if unsafe { SecKeychainGetUserInteractionAllowed(&mut allowed) } != 0 {
                return Err(Failure::Unavailable);
            }
            // SAFETY: a scalar process setting.
            if unsafe { SecKeychainSetUserInteractionAllowed(0) } != 0 {
                return Err(Failure::Unavailable);
            }
            state.1 = allowed != 0;
        }
        state.0 += 1;
        Ok(Self)
    }
}

impl Drop for NoInteraction {
    fn drop(&mut self) {
        if let Ok(mut state) = INTERACTION.lock() {
            state.0 -= 1;
            if state.0 == 0 && state.1 {
                // SAFETY: a scalar process setting.
                unsafe { SecKeychainSetUserInteractionAllowed(1) };
            }
        }
    }
}

fn writer(until: Instant) -> Result<MutexGuard<'static, ()>> {
    crate::poll_until(until, || match WRITER.try_lock() {
        Ok(guard) => Some(Ok(guard)),
        Err(TryLockError::Poisoned(_)) => Some(Err(Failure::Unavailable)),
        Err(TryLockError::WouldBlock) => None,
    })
}

/// The user's login keychain, from the account database rather than `HOME`.
fn login_keychain_path() -> Result<PathBuf> {
    let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut buffer = vec![0 as libc::c_char; 16 * 1024];
    let mut result = std::ptr::null_mut();
    // SAFETY: every pointer is valid for the call; buffer outlives the use of
    // the strings it backs.
    let status = unsafe {
        libc::getpwuid_r(
            libc::geteuid(),
            entry.as_mut_ptr(),
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() {
        return Err(Failure::Unavailable);
    }
    // SAFETY: getpwuid_r succeeded and initialized the entry.
    let entry = unsafe { entry.assume_init() };
    if entry.pw_dir.is_null() {
        return Err(Failure::Unavailable);
    }
    // SAFETY: pw_dir is a NUL-terminated string inside buffer.
    let home = unsafe { CStr::from_ptr(entry.pw_dir) };
    let home = PathBuf::from(std::ffi::OsStr::from_bytes(home.to_bytes()));
    if !home.is_absolute() {
        return Err(Failure::Unavailable);
    }
    Ok(home.join("Library/Keychains/login.keychain-db"))
}

/// The login keychain opened for one scope.
pub struct Keychain {
    keychain: SecKeychain,
    scope: Scope,
}

impl std::fmt::Debug for Keychain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Keychain")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl Keychain {
    /// The user's login keychain. Opening reads nothing and never unlocks.
    pub fn open(scope: Scope) -> Result<Self> {
        let path = login_keychain_path()?;
        let keychain = SecKeychain::open(path).map_err(|_| Failure::Unavailable)?;
        Ok(Self::with_keychain(scope, keychain))
    }

    /// Any keychain the caller holds, such as a test's temporary keychain.
    pub fn with_keychain(scope: Scope, keychain: SecKeychain) -> Self {
        Self { keychain, scope }
    }

    /// Passive state of the login keychain.
    pub fn inspect() -> State {
        login_keychain_path()
            .and_then(|path| SecKeychain::open(path).map_err(|_| Failure::Unavailable))
            .map(|keychain| status(&keychain))
            .unwrap_or(State::Unavailable)
    }

    /// Adds the generic password item of `version` to this keychain with
    /// [`any_application_access`], answering the keychain's status code.
    fn add(&self, version: Version, material: &Material) -> Result<OSStatus> {
        let access = any_application_access()?;
        let service = CFString::new(SERVICE);
        let account = CFString::new(&version.item_name());
        let data = CFData::from_buffer(material.as_bytes());
        let mut attributes = CFMutableDictionary::from_CFType_pairs(&[]);
        // SAFETY: the keys are Security framework constants; every value is
        // live until the dictionary is released after the call.
        unsafe {
            attributes.add(&kSecClass.to_void(), &kSecClassGenericPassword.to_void());
            attributes.add(&kSecAttrService.to_void(), &service.to_void());
            attributes.add(&kSecAttrAccount.to_void(), &account.to_void());
            attributes.add(&kSecValueData.to_void(), &data.to_void());
            attributes.add(&kSecUseKeychain.to_void(), &self.keychain.to_void());
            attributes.add(&kSecAttrAccess.to_void(), &access.to_void());
        }
        let attributes = attributes.to_immutable();
        // SAFETY: the dictionary is live for the call; no result is requested.
        Ok(unsafe { SecItemAdd(attributes.as_concrete_TypeRef(), std::ptr::null_mut()) })
    }

    fn find(&self, version: Version) -> Result<Option<Found>> {
        match self
            .keychain
            .find_generic_password(SERVICE, &version.item_name())
        {
            Ok(found) => Ok(Some(found)),
            Err(error) if error.code() == errSecItemNotFound => Ok(None),
            Err(_) => Err(Failure::Unavailable),
        }
    }

    /// One read of `version` answered as [`Backend::read`] answers it, with
    /// the keychain's status code when the item lookup failed. It observes the
    /// keychain's access decision for this executable and changes nothing.
    pub fn read_observed(&self, version: Version) -> (Result<Material>, Option<OSStatus>) {
        let _quiet = match NoInteraction::enter() {
            Ok(quiet) => quiet,
            Err(failure) => return (Err(failure), None),
        };
        if let Err(failure) = self.admit(version) {
            return (Err(failure), None);
        }
        match self
            .keychain
            .find_generic_password(SERVICE, &version.item_name())
        {
            Ok((password, _item)) => {
                let material = Material::from_slice(&password);
                drop(password);
                if material.is_empty() || material.len() > MAX_BYTES {
                    return (Err(Failure::Unavailable), None);
                }
                (Ok(material), None)
            }
            Err(error) if error.code() == errSecItemNotFound => {
                (Err(Failure::Missing), Some(error.code()))
            }
            Err(error) => (Err(Failure::Unavailable), Some(error.code())),
        }
    }
}

fn status(keychain: &SecKeychain) -> State {
    let Ok(_quiet) = NoInteraction::enter() else {
        return State::Unavailable;
    };
    let mut flags = 0;
    // SAFETY: the keychain reference is live and flags is a valid output.
    if unsafe { SecKeychainGetStatus(keychain.as_concrete_TypeRef(), &mut flags) } != 0 {
        return State::Unavailable;
    }
    if flags & UNLOCKED != 0 {
        State::Available
    } else {
        State::Locked
    }
}

impl ReadBackend for Keychain {
    fn kind(&self) -> CustodyBackend {
        CustodyBackend::MacosKeychain
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn state(&self) -> State {
        status(&self.keychain)
    }

    fn admit(&self, version: Version) -> Result<()> {
        if version.scope() != self.scope {
            return Err(Failure::Denied);
        }
        if self.state() != State::Available {
            return Err(Failure::Unavailable);
        }
        Ok(())
    }

    fn read(&self, version: Version) -> Result<Material> {
        let _quiet = NoInteraction::enter()?;
        self.admit(version)?;
        let (password, _item) = self.find(version)?.ok_or(Failure::Missing)?;
        let material = Material::from_slice(&password);
        drop(password);
        if material.is_empty() || material.len() > MAX_BYTES {
            return Err(Failure::Unavailable);
        }
        Ok(material)
    }
}

impl Backend for Keychain {
    fn write_new(
        &self,
        version: Version,
        material: &Material,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        admit_material(self.scope, material, MAX_BYTES)?;
        let _quiet = NoInteraction::enter()?;
        let _writer = writer(until)?;
        self.admit(version)?;
        guard()?;
        if self.find(version)?.is_some() {
            return Err(Failure::Conflict);
        }
        match self.add(version, material)? {
            0 => {}
            status if status == errSecDuplicateItem => return Err(Failure::Conflict),
            ERR_SEC_INTERACTION_NOT_ALLOWED => return Err(Failure::Unavailable),
            _ => return Err(Failure::OutcomeUnknown),
        }
        let retained = self.read(version).map_err(|_| Failure::OutcomeUnknown)?;
        if !retained.ct_eq(material.as_bytes()) {
            return Err(Failure::OutcomeUnknown);
        }
        Ok(())
    }

    fn confirm(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Material) -> Result<()>,
    ) -> Result<()> {
        let _quiet = NoInteraction::enter()?;
        let _writer = writer(until)?;
        self.admit(version)?;
        guard()?;
        let material = self.read(version)?;
        admit_material(self.scope, &material, MAX_BYTES)?;
        validate(&material)
    }

    fn delete(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let _quiet = NoInteraction::enter()?;
        let _writer = writer(until)?;
        self.admit(version)?;
        guard()?;
        if let Some((password, item)) = self.find(version)? {
            drop(password);
            // SAFETY: the item reference is live for the call.
            if unsafe { SecKeychainItemDelete(item.as_concrete_TypeRef()) } != 0 {
                return Err(Failure::OutcomeUnknown);
            }
        }
        if self
            .find(version)
            .map_err(|_| Failure::OutcomeUnknown)?
            .is_some()
        {
            return Err(Failure::OutcomeUnknown);
        }
        Ok(())
    }
}

/// A keychain file a test creates, can lock, and deletes when dropped.
#[cfg(any(test, feature = "test-support"))]
pub struct TemporaryKeychain {
    keychain: SecKeychain,
}

#[cfg(any(test, feature = "test-support"))]
impl TemporaryKeychain {
    pub fn create(path: &std::path::Path) -> Result<Self> {
        let password = uuid::Uuid::new_v4().simple().to_string();
        let keychain = security_framework::os::macos::keychain::CreateOptions::new()
            .password(&password)
            .prompt_user(false)
            .create(path)
            .map_err(|_| Failure::Unavailable)?;
        Ok(Self { keychain })
    }

    pub fn keychain(&self) -> SecKeychain {
        self.keychain.clone()
    }

    pub fn lock(&self) -> Result<()> {
        // SAFETY: the keychain reference is live for the call.
        if unsafe { SecKeychainLock(self.keychain.as_concrete_TypeRef()) } != 0 {
            return Err(Failure::Unavailable);
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Drop for TemporaryKeychain {
    fn drop(&mut self) {
        // SAFETY: the keychain reference is live for the call.
        unsafe { SecKeychainDelete(self.keychain.as_concrete_TypeRef()) };
    }
}
