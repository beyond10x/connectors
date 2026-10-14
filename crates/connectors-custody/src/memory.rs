//! The in-memory reference binding. It keeps the contract's rules (immutable
//! scoped versions, `Conflict`, `Missing`, `Denied`, the value bound) and no
//! durability: it is what the contract suite runs against on every platform,
//! never a fallback for a missing platform store.
use crate::{
    Backend, CustodyBackend, Failure, Material, ReadBackend, Result, Scope, State, Version,
    admit_material, poll_until,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, TryLockError},
    time::Instant,
};
use zeroize::Zeroizing;

/// One shared store; [`MemoryStore::open`] hands out scoped handles to it.
#[derive(Clone)]
pub struct MemoryStore {
    inner: Arc<Inner>,
}

struct Inner {
    items: Mutex<HashMap<Version, Zeroizing<Vec<u8>>>>,
    writer: Mutex<()>,
    state: Mutex<State>,
    kind: CustodyBackend,
}

impl std::fmt::Debug for MemoryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryStore")
            .field("kind", &self.inner.kind)
            .finish_non_exhaustive()
    }
}

impl MemoryStore {
    /// A reference store with the value bound of `kind`, so the suite can hold
    /// it to the bound of the binding it stands in for.
    pub fn new(kind: CustodyBackend) -> Self {
        Self {
            inner: Arc::new(Inner {
                items: Mutex::new(HashMap::new()),
                writer: Mutex::new(()),
                state: Mutex::new(State::Available),
                kind,
            }),
        }
    }

    /// Simulate the store's state, as a locked or absent platform store.
    pub fn set_state(&self, state: State) {
        if let Ok(mut current) = self.inner.state.lock() {
            *current = state;
        }
    }

    pub fn open(&self, scope: Scope) -> Result<Memory> {
        Ok(Memory {
            store: self.clone(),
            scope,
        })
    }

    fn items(&self) -> Result<MutexGuard<'_, HashMap<Version, Zeroizing<Vec<u8>>>>> {
        self.inner.items.lock().map_err(|_| Failure::Unavailable)
    }

    fn lock(&self, until: Instant) -> Result<MutexGuard<'_, ()>> {
        poll_until(until, || match self.inner.writer.try_lock() {
            Ok(guard) => Some(Ok(guard)),
            Err(TryLockError::Poisoned(_)) => Some(Err(Failure::Unavailable)),
            Err(TryLockError::WouldBlock) => None,
        })
    }
}

/// A handle to a [`MemoryStore`] for one scope.
#[derive(Debug)]
pub struct Memory {
    store: MemoryStore,
    scope: Scope,
}

impl ReadBackend for Memory {
    fn kind(&self) -> CustodyBackend {
        self.store.inner.kind
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn state(&self) -> State {
        self.store
            .inner
            .state
            .lock()
            .map(|state| *state)
            .unwrap_or(State::Unavailable)
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
        self.admit(version)?;
        let items = self.store.items()?;
        let bytes = items.get(&version).ok_or(Failure::Missing)?;
        Ok(Material::from_slice(bytes))
    }
}

impl Backend for Memory {
    fn write_new(
        &self,
        version: Version,
        material: &Material,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.admit(version)?;
        admit_material(self.scope, material, self.max_bytes())?;
        let _writer = self.store.lock(until)?;
        self.admit(version)?;
        guard()?;
        let mut items = self.store.items()?;
        if items.contains_key(&version) {
            return Err(Failure::Conflict);
        }
        items.insert(version, Zeroizing::new(material.as_bytes().to_vec()));
        Ok(())
    }

    fn confirm(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Material) -> Result<()>,
    ) -> Result<()> {
        let _writer = self.store.lock(until)?;
        self.admit(version)?;
        guard()?;
        let material = self.read(version)?;
        admit_material(self.scope, &material, self.max_bytes())?;
        validate(&material)
    }

    fn delete(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        self.admit(version)?;
        let _writer = self.store.lock(until)?;
        self.admit(version)?;
        guard()?;
        self.store.items()?.remove(&version);
        Ok(())
    }
}
