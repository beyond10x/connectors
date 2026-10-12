//! Values split over bounded entries: the layout of the Windows binding
//! (`semantics.md` §5.1), kept platform-neutral so its ordering and its crash
//! cases are tested on every platform against a fake entry store.
//!
//! A Credential Manager blob holds at most [`PART_BYTES`]. Every version is
//! stored in the same form, whatever its size: its value cut into numbered part
//! entries, written first in ascending order, then one header entry naming the
//! value's length, the part count and the value's SHA-256, written last. The
//! version exists only once its header exists. A read needs the header and
//! every part and checks the digest; a missing part or a mismatch is
//! `InvalidMaterial`. A delete removes the header first, then every part in
//! descending order. Parts without a header, left by a crash during a write or
//! a delete, are an unacknowledged version: a read answers `Missing`, a write
//! of that version is `Conflict`, and a delete reclaims them.
//!
//! Writes ascend and deletions descend, so the parts present are a prefix of
//! the part numbers except after external tampering; a delete still sweeps
//! every possible part number, so it reclaims any remainder.
use crate::{
    Backend, CustodyBackend, Failure, MAX_BYTES, Material, Result, Scope, State, Version,
    admit_material,
};
use sha2::{Digest, Sha256};
use std::time::Instant;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// The largest blob one entry holds (`CRED_MAX_CREDENTIAL_BLOB_SIZE`).
pub const PART_BYTES: usize = 2560;
/// The most parts a value within [`MAX_BYTES`] needs.
pub const MAX_PARTS: usize = MAX_BYTES.div_ceil(PART_BYTES);
/// Every entry name starts with this prefix.
pub const TARGET_PREFIX: &str = "connectors/";

const MAGIC: &[u8; 8] = b"CCPARTS1";
const HEADER_BYTES: usize = MAGIC.len() + 4 + 4 + 32;

/// The header entry's name: the prefix and the version's item name.
pub fn header_name(version: Version) -> String {
    format!("{TARGET_PREFIX}{}", version.item_name())
}

/// Part `index`'s entry name, below the header's.
pub fn part_name(version: Version, index: usize) -> String {
    format!("{TARGET_PREFIX}{}/part/{index}", version.item_name())
}

/// What a header names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub length: usize,
    pub parts: usize,
    pub digest: [u8; 32],
}

impl Header {
    fn of(value: &[u8]) -> Self {
        Self {
            length: value.len(),
            parts: value.len().div_ceil(PART_BYTES),
            digest: Sha256::digest(value).into(),
        }
    }

    /// `CCPARTS1`, the length and the part count as big-endian `u32`, then the
    /// SHA-256 of the value.
    pub fn encode(&self) -> Vec<u8> {
        let mut blob = Vec::with_capacity(HEADER_BYTES);
        blob.extend_from_slice(MAGIC);
        blob.extend_from_slice(&(self.length as u32).to_be_bytes());
        blob.extend_from_slice(&(self.parts as u32).to_be_bytes());
        blob.extend_from_slice(&self.digest);
        blob
    }

    /// A header that is not exactly this encoding of a value within the bound
    /// is `InvalidMaterial`.
    pub fn decode(blob: &[u8]) -> Result<Self> {
        if blob.len() != HEADER_BYTES || &blob[..MAGIC.len()] != MAGIC {
            return Err(Failure::InvalidMaterial);
        }
        let word = |at: usize| {
            u32::from_be_bytes(blob[at..at + 4].try_into().expect("four bytes")) as usize
        };
        let (length, parts) = (word(MAGIC.len()), word(MAGIC.len() + 4));
        let mut digest = [0; 32];
        digest.copy_from_slice(&blob[MAGIC.len() + 8..]);
        if length == 0 || length > MAX_BYTES || parts != length.div_ceil(PART_BYTES) {
            return Err(Failure::InvalidMaterial);
        }
        Ok(Self {
            length,
            parts,
            digest,
        })
    }

    /// The exact size part `index` holds.
    fn part_len(&self, index: usize) -> usize {
        if index + 1 == self.parts {
            self.length - index * PART_BYTES
        } else {
            PART_BYTES
        }
    }
}

/// A store of small named entries: the primitives the split layout is built
/// on. Implemented by the Windows Credential Manager and by a fake store in
/// tests.
pub trait Entries {
    /// The store-level exclusion of writes and deletions, held until dropped.
    type Lock<'a>
    where
        Self: 'a;
    /// Acquire the exclusion by `until`, or answer `Unavailable`.
    fn lock(&self, until: Instant) -> Result<Self::Lock<'_>>;
    /// Passive state of the store.
    fn state(&self) -> State;
    /// One entry's blob, or `None` when it is absent.
    fn read(&self, name: &str) -> Result<Option<Material>>;
    /// Create or replace one entry. `Unavailable` when the store refused
    /// before anything changed; any other failure is `OutcomeUnknown`.
    fn write(&self, name: &str, blob: &[u8]) -> Result<()>;
    /// Remove one entry; an absent entry is already removed.
    fn delete(&self, name: &str) -> Result<()>;
}

/// The split layout over an entry store, opened for one scope.
pub struct Parted<E> {
    entries: E,
    scope: Scope,
}

impl<E> std::fmt::Debug for Parted<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parted")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl<E: Entries> Parted<E> {
    pub fn new(entries: E, scope: Scope) -> Self {
        Self { entries, scope }
    }

    pub fn entries(&self) -> &E {
        &self.entries
    }

    fn present(&self, name: &str) -> Result<bool> {
        Ok(self.entries.read(name)?.is_some())
    }

    /// Read an admitted version: the header, every part, and the digest.
    fn read_parts(&self, version: Version) -> Result<Material> {
        let blob = self
            .entries
            .read(&header_name(version))?
            .ok_or(Failure::Missing)?;
        let header = Header::decode(blob.as_bytes())?;
        let mut value = Zeroizing::new(Vec::with_capacity(header.length));
        for index in 0..header.parts {
            let part = self
                .entries
                .read(&part_name(version, index))?
                .ok_or(Failure::InvalidMaterial)?;
            if part.len() != header.part_len(index) {
                return Err(Failure::InvalidMaterial);
            }
            value.extend_from_slice(part.as_bytes());
        }
        let digest: [u8; 32] = Sha256::digest(value.as_slice()).into();
        if !bool::from(digest.ct_eq(&header.digest)) {
            return Err(Failure::InvalidMaterial);
        }
        Ok(Material::new(std::mem::take(&mut *value)))
    }
}

impl<E: Entries> Backend for Parted<E> {
    fn kind(&self) -> CustodyBackend {
        CustodyBackend::WindowsCredentialManager
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn state(&self) -> State {
        self.entries.state()
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

    fn write_new(
        &self,
        version: Version,
        material: &Material,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        // The bound is checked before any call: nothing is written for a
        // larger value.
        admit_material(self.scope, material, MAX_BYTES)?;
        let _lock = self.entries.lock(until)?;
        self.admit(version)?;
        guard()?;
        // A header is a version; a first part without one is an
        // unacknowledged version of the same id. Neither is overwritten.
        if self.present(&header_name(version))? || self.present(&part_name(version, 0))? {
            return Err(Failure::Conflict);
        }
        for (index, part) in material.as_bytes().chunks(PART_BYTES).enumerate() {
            match self.entries.write(&part_name(version, index), part) {
                Ok(()) => {}
                Err(Failure::Unavailable) if index == 0 => return Err(Failure::Unavailable),
                Err(_) => return Err(Failure::OutcomeUnknown),
            }
        }
        let header = Header::of(material.as_bytes());
        self.entries
            .write(&header_name(version), &header.encode())
            .map_err(|_| Failure::OutcomeUnknown)?;
        match self.read_parts(version) {
            Ok(retained) if retained.ct_eq(material.as_bytes()) => Ok(()),
            _ => Err(Failure::OutcomeUnknown),
        }
    }

    fn read(&self, version: Version) -> Result<Material> {
        self.admit(version)?;
        self.read_parts(version)
    }

    fn confirm(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Material) -> Result<()>,
    ) -> Result<()> {
        let _lock = self.entries.lock(until)?;
        self.admit(version)?;
        guard()?;
        let material = self.read_parts(version)?;
        admit_material(self.scope, &material, MAX_BYTES)?;
        validate(&material)
    }

    fn delete(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let _lock = self.entries.lock(until)?;
        self.admit(version)?;
        guard()?;
        let header = header_name(version);
        self.entries
            .delete(&header)
            .map_err(|_| Failure::OutcomeUnknown)?;
        for index in (0..MAX_PARTS).rev() {
            self.entries
                .delete(&part_name(version, index))
                .map_err(|_| Failure::OutcomeUnknown)?;
        }
        let mut names = (0..MAX_PARTS).map(|index| part_name(version, index));
        let remains = self.present(&header).map_err(|_| Failure::OutcomeUnknown)?
            || names
                .try_fold(false, |found, name| {
                    Ok::<_, Failure>(found || self.present(&name)?)
                })
                .map_err(|_| Failure::OutcomeUnknown)?;
        if remains {
            return Err(Failure::OutcomeUnknown);
        }
        Ok(())
    }
}

/// An in-memory entry store standing in for the Credential Manager in tests:
/// a crash is a mutation that fails, and the store can be tampered with.
#[cfg(test)]
pub mod fake {
    use super::Entries;
    use crate::{Failure, Material, Result, State};
    use std::{
        collections::BTreeMap,
        sync::{Arc, Mutex, MutexGuard},
        time::Instant,
    };
    use zeroize::Zeroizing;

    #[derive(Default)]
    struct Inner {
        entries: BTreeMap<String, Zeroizing<Vec<u8>>>,
        /// Mutations admitted before every later one fails, as a crash would.
        mutations_left: Option<usize>,
        writes: usize,
        deletes: usize,
    }

    #[derive(Clone, Default)]
    pub struct FakeEntries {
        inner: Arc<Mutex<Inner>>,
        writer: Arc<Mutex<()>>,
    }

    impl std::fmt::Debug for FakeEntries {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("FakeEntries")
        }
    }

    impl FakeEntries {
        fn inner(&self) -> MutexGuard<'_, Inner> {
            self.inner.lock().expect("fake store")
        }
        /// Every entry name, in order.
        pub fn names(&self) -> Vec<String> {
            self.inner().entries.keys().cloned().collect()
        }
        /// Admit `count` more mutations, then fail every one after.
        pub fn crash_after(&self, count: usize) {
            self.inner().mutations_left = Some(count);
        }
        /// Admit every mutation again.
        pub fn recover(&self) {
            self.inner().mutations_left = None;
        }
        /// Remove an entry behind the layout's back.
        pub fn remove(&self, name: &str) -> bool {
            self.inner().entries.remove(name).is_some()
        }
        /// Flip one byte of an entry behind the layout's back.
        pub fn corrupt(&self, name: &str) -> bool {
            match self.inner().entries.get_mut(name) {
                Some(blob) => {
                    blob[0] ^= 0x01;
                    true
                }
                None => false,
            }
        }
        /// (writes, deletes) the store admitted or refused so far.
        pub fn calls(&self) -> (usize, usize) {
            let inner = self.inner();
            (inner.writes, inner.deletes)
        }
        fn mutate(inner: &mut Inner) -> Result<()> {
            match inner.mutations_left.as_mut() {
                Some(0) => Err(Failure::OutcomeUnknown),
                Some(left) => {
                    *left -= 1;
                    Ok(())
                }
                None => Ok(()),
            }
        }
    }

    impl Entries for FakeEntries {
        type Lock<'a> = MutexGuard<'a, ()>;

        fn lock(&self, until: Instant) -> Result<Self::Lock<'_>> {
            crate::poll_until(until, || match self.writer.try_lock() {
                Ok(guard) => Some(Ok(guard)),
                Err(std::sync::TryLockError::Poisoned(_)) => Some(Err(Failure::Unavailable)),
                Err(std::sync::TryLockError::WouldBlock) => None,
            })
        }

        fn state(&self) -> State {
            State::Available
        }

        fn read(&self, name: &str) -> Result<Option<Material>> {
            Ok(self
                .inner()
                .entries
                .get(name)
                .map(|blob| Material::from_slice(blob)))
        }

        fn write(&self, name: &str, blob: &[u8]) -> Result<()> {
            let mut inner = self.inner();
            inner.writes += 1;
            Self::mutate(&mut inner)?;
            assert!(blob.len() <= super::PART_BYTES, "an entry over one blob");
            inner
                .entries
                .insert(name.to_owned(), Zeroizing::new(blob.to_vec()));
            Ok(())
        }

        fn delete(&self, name: &str) -> Result<()> {
            let mut inner = self.inner();
            inner.deletes += 1;
            Self::mutate(&mut inner)?;
            inner.entries.remove(name);
            Ok(())
        }
    }
}
