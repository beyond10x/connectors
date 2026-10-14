//! Credential custody of the local host: immutable, scoped versions held in one
//! protected platform store (`contracts/auth/custody/v1alpha1/semantics.md` §5.1),
//! or read from an explicitly selected read-only mounted directory (§5.2).
//!
//! A binding has two capabilities. [`ReadBackend`] is what every binding answers:
//! read an exact version and inspect the store's state. [`Backend`] adds the
//! writes: write a new scoped version, confirm one and delete one. Each platform
//! has exactly one writable binding, fixed when the binary is built
//! ([`PLATFORM`]): the freedesktop Secret Service on Linux, the login keychain on
//! macOS and the Credential Manager on Windows. [`mounted`] (Linux) implements
//! only the read capability, so no write reaches it: the compiler refuses one.
//! It is used only when the owner configuration names it. None falls back to
//! another binding, to a file or to an environment variable, and none
//! activates, unlocks or prompts.
//! [`memory`] is the reference every platform runs the same contract suite
//! against.
//!
//! The host's coordinator (its guards, its in-process writer lock and its
//! metadata fences) is not here: a backend runs the guard it is handed while
//! its own store-level exclusion is held, and nothing else.
use std::{fmt, time::Instant};
use uuid::Uuid;
use zeroize::Zeroizing;

#[cfg(any(test, feature = "test-support"))]
pub mod contract;
pub mod memory;
pub mod parts;
#[cfg(test)]
mod tests;

#[cfg(target_os = "windows")]
pub mod credential_manager;
#[cfg(target_os = "macos")]
pub mod keychain;
#[cfg(target_os = "linux")]
pub mod mounted;
#[cfg(target_os = "linux")]
pub mod secret_service;

/// The largest value the custody model admits (64 KiB), on every platform.
/// The Windows binding splits a value over Credential Manager entries
/// ([`parts`]) to hold it.
pub const MAX_BYTES: usize = 64 * 1024;
/// The exact size of an approval signing seed.
pub const SIGNING_SEED_BYTES: usize = 32;

/// The store a binding uses (`connectors.cli.CustodyBackend`). There is no
/// fallback variant: `MountedSecret` is read-only and chosen only by an
/// explicit selection in the owner configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyBackend {
    SecretService,
    MacosKeychain,
    WindowsCredentialManager,
    MountedSecret,
}

impl CustodyBackend {
    /// The model's variant name, as `setup check` reports it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SecretService => "secret_service",
            Self::MacosKeychain => "macos_keychain",
            Self::WindowsCredentialManager => "windows_credential_manager",
            Self::MountedSecret => "mounted_secret",
        }
    }

    /// The largest value this binding stores or reads: [`MAX_BYTES`] for every one.
    pub fn max_bytes(self) -> usize {
        match self {
            Self::SecretService
            | Self::MacosKeychain
            | Self::WindowsCredentialManager
            | Self::MountedSecret => MAX_BYTES,
        }
    }

    /// Whether the binding writes, confirms and deletes versions. A read-only
    /// binding provides none of them (§4).
    pub fn writable(self) -> bool {
        !matches!(self, Self::MountedSecret)
    }
}

/// The binding of the platform this binary was built for; `None` where the
/// local host has no qualified binding.
pub const PLATFORM: Option<CustodyBackend> = if cfg!(target_os = "linux") {
    Some(CustodyBackend::SecretService)
} else if cfg!(target_os = "macos") {
    Some(CustodyBackend::MacosKeychain)
} else if cfg!(target_os = "windows") {
    Some(CustodyBackend::WindowsCredentialManager)
} else {
    None
};

/// The platform binding's backend type.
#[cfg(target_os = "linux")]
pub type Platform = secret_service::SecretService;
#[cfg(target_os = "macos")]
pub type Platform = keychain::Keychain;
#[cfg(target_os = "windows")]
pub type Platform = credential_manager::CredentialManager;

/// Closed failures. They carry neither store diagnostics, locators nor bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Missing,
    Unavailable,
    Denied,
    InvalidMaterial,
    Conflict,
    OutcomeUnknown,
    /// The binding is read-only: it writes, confirms and deletes nothing.
    ReadOnly,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Missing => "custody version missing",
            Self::Unavailable => "custody unavailable",
            Self::Denied => "custody version denied",
            Self::InvalidMaterial => "custody material invalid",
            Self::Conflict => "custody version conflict",
            Self::OutcomeUnknown => "custody outcome unknown",
            Self::ReadOnly => "custody binding read-only",
        })
    }
}
impl std::error::Error for Failure {}

pub type Result<T> = std::result::Result<T, Failure>;

/// What passive inspection reports. It never activates, unlocks or prompts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Available,
    Locked,
    Unavailable,
}

/// What a scope's versions hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Purpose {
    Credential,
    ApprovalSigning,
}

impl Purpose {
    fn label(self) -> &'static str {
        match self {
            Self::Credential => "credential",
            Self::ApprovalSigning => "approval-signing",
        }
    }
}

/// Host-assigned namespace: an injective mapping of the CustodyVersion scope,
/// not a provider identity or a public connection selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scope {
    authority: Uuid,
    allocation: Uuid,
    purpose: Purpose,
}

impl Scope {
    pub fn new(authority: Uuid, allocation: Uuid) -> Result<Self> {
        Self::with_purpose(authority, allocation, Purpose::Credential)
    }

    pub fn approval_signing(authority: Uuid, allocation: Uuid) -> Result<Self> {
        Self::with_purpose(authority, allocation, Purpose::ApprovalSigning)
    }

    fn with_purpose(authority: Uuid, allocation: Uuid, purpose: Purpose) -> Result<Self> {
        if authority.is_nil() || allocation.is_nil() {
            return Err(Failure::Denied);
        }
        Ok(Self {
            authority,
            allocation,
            purpose,
        })
    }

    pub fn authority(&self) -> Uuid {
        self.authority
    }
    pub fn allocation(&self) -> Uuid {
        self.allocation
    }
    pub fn purpose(&self) -> Purpose {
        self.purpose
    }
}

/// One immutable version in one scope. The coordinator allocates and records
/// this exact tuple before attempting a write, so an unknown response does not
/// lose the candidate's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Version {
    scope: Scope,
    id: Uuid,
}

impl Version {
    pub fn new(scope: Scope, id: Uuid) -> Result<Self> {
        if id.is_nil() {
            return Err(Failure::Denied);
        }
        Ok(Self { scope, id })
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// The name every platform binding keys this version's item by: the
    /// purpose, the scope and the version ids, and nothing else.
    pub fn item_name(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.scope.purpose.label(),
            self.scope.authority,
            self.scope.allocation,
            self.id
        )
    }
}

/// Credential bytes in custody. Zeroized on drop; its `Debug` never shows them.
pub struct Material(Zeroizing<Vec<u8>>);

impl Material {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(Zeroizing::new(bytes))
    }
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self::new(bytes.to_vec())
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Moves the bytes out; the caller now owns their erasure.
    pub fn into_vec(mut self) -> Vec<u8> {
        std::mem::take(&mut *self.0)
    }
    /// Compares without an early exit on the first differing byte.
    pub fn ct_eq(&self, other: &[u8]) -> bool {
        use subtle::ConstantTimeEq;
        bool::from(self.0.as_slice().ct_eq(other))
    }
}

impl fmt::Debug for Material {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Material(redacted)")
    }
}

/// The material rule every binding applies before any store call: a value is
/// non-empty and within the binding's bound, and a signing seed is exactly
/// [`SIGNING_SEED_BYTES`].
pub fn admit_material(scope: Scope, material: &Material, max_bytes: usize) -> Result<()> {
    if material.is_empty()
        || material.len() > max_bytes
        || (scope.purpose == Purpose::ApprovalSigning && material.len() != SIGNING_SEED_BYTES)
    {
        return Err(Failure::InvalidMaterial);
    }
    Ok(())
}

/// The read capability of one binding opened for one admitted scope: what
/// every binding answers, the read-only `mounted_secret` included. A version of
/// another scope is `Denied` by every method.
pub trait ReadBackend {
    /// The binding this backend realizes.
    fn kind(&self) -> CustodyBackend;
    /// The one scope this handle admits.
    fn scope(&self) -> Scope;
    /// The largest value this binding stores.
    fn max_bytes(&self) -> usize {
        self.kind().max_bytes()
    }
    /// Passive inspection of the store this handle is bound to.
    fn state(&self) -> State;
    /// Admit a version: its scope is this handle's and the store is usable.
    fn admit(&self, version: Version) -> Result<()>;
    /// Read one exact version; an absent one is `Missing`.
    fn read(&self, version: Version) -> Result<Material>;
}

/// The write capability of a binding: every platform binding has it and a
/// read-only binding does not, so a write against one does not compile.
///
/// The guards are the coordinator's: each runs after the backend holds its own
/// store-level exclusion and before the store changes, and its refusal is
/// returned unchanged with nothing written or deleted.
pub trait Backend: ReadBackend {
    /// Write a new immutable version. An existing version is `Conflict` and is
    /// never overwritten. A failure after the store may have changed is
    /// `OutcomeUnknown`; it is never retried here. A success is a definite
    /// acknowledgement: the exact bytes were read back.
    fn write_new(
        &self,
        version: Version,
        material: &Material,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()>;
    /// Re-establish an existing version under the store's exclusion: run the
    /// guard, read the version, admit it as material and hand it to
    /// `validate`, then make it durable. Used to recover a staged signing key.
    fn confirm(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Material) -> Result<()>,
    ) -> Result<()>;
    /// Delete one exact version. An absent version is already deleted.
    fn delete(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()>;
}

/// Open the platform binding for one scope. `secret_service_socket` selects
/// the Secret Service bus on Linux; every other binding takes no binding
/// and refuses one as `Unavailable`.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub fn open(scope: Scope, secret_service_socket: Option<&std::path::Path>) -> Result<Platform> {
    #[cfg(target_os = "linux")]
    {
        secret_service::SecretService::open_at(scope, secret_service_socket)
    }
    #[cfg(not(target_os = "linux"))]
    {
        if secret_service_socket.is_some() {
            return Err(Failure::Unavailable);
        }
        Platform::open(scope)
    }
}

/// Passive state of the platform store. Never activates, unlocks or prompts.
pub fn inspect(secret_service_socket: Option<&std::path::Path>) -> State {
    #[cfg(target_os = "linux")]
    {
        secret_service::inspect_at(secret_service_socket)
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        if secret_service_socket.is_some() {
            return State::Unavailable;
        }
        Platform::inspect()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = secret_service_socket;
        State::Unavailable
    }
}

/// Passive qualification of the platform store for custody: available and,
/// where the binding has one, its durability qualification holds. Reads no
/// credential.
pub fn available(secret_service_socket: Option<&std::path::Path>) -> bool {
    #[cfg(target_os = "linux")]
    {
        secret_service::available_at(secret_service_socket)
    }
    #[cfg(not(target_os = "linux"))]
    {
        inspect(secret_service_socket) == State::Available
    }
}

/// Wait for a store-level lock until `until`, polling `try_lock`.
fn poll_until<T>(until: Instant, mut try_lock: impl FnMut() -> Option<Result<T>>) -> Result<T> {
    loop {
        if let Some(result) = try_lock() {
            return result;
        }
        if Instant::now() >= until {
            return Err(Failure::Unavailable);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
