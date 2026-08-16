//! Crash-recoverable prepared transactions whose secret candidates remain inside Vault.

use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use connector_secrets::{
    CredentialRef, CredentialScope, Layout, PreparedSecretError, PreparedSecretStore, Secret,
    SecretBatch, SecretProposalDigest, SecretStore, SecretTransactionGeneration,
    SecretTransactionId, SecretTransactionState, StoreError, TenantLayout,
    MAX_TERMINAL_TRANSACTIONS,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const JOURNAL_VERSION: u8 = 1;
const MAX_JOURNAL_BYTES: u64 = 4 * 1024 * 1024;

/// A Vault-backed prepared store. Candidate values are staged under transaction-only Vault
/// addresses; the local journal contains addresses and states only, never credential values.
pub struct PreparedVaultStore {
    inner: Arc<dyn SecretStore>,
    journal_path: PathBuf,
    journal: Mutex<Journal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    version: u8,
    retired_through: u64,
    transactions: Vec<Transaction>,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            version: JOURNAL_VERSION,
            retired_through: 0,
            transactions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transaction {
    id: String,
    generation: u64,
    digest: String,
    phase: Phase,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Staging,
    Prepared,
    Committing,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    published: String,
    staged: String,
}

impl PreparedVaultStore {
    /// Open the value-free transaction journal. Recovery is completed by [`initialize`](Self::initialize).
    pub fn open(
        inner: Arc<dyn SecretStore>,
        journal_path: impl Into<PathBuf>,
    ) -> Result<Self, StoreError> {
        let journal_path = journal_path.into();
        if !journal_path.is_absolute() {
            return Err(journal_error(
                &journal_path,
                "an absolute journal path is required",
            ));
        }
        validate_parent(&journal_path)?;
        remove_incomplete_temporary(&journal_path)?;
        let journal = read_journal(&journal_path)?;
        Ok(Self {
            inner,
            journal_path,
            journal: Mutex::new(journal),
        })
    }

    /// Resolve incomplete commits and remove incomplete staging before the store is exposed.
    pub async fn initialize(&self) -> Result<(), StoreError> {
        let mut journal = self.journal.lock().await;
        let mut journal_changed = false;
        for index in 0..journal.transactions.len() {
            match journal.transactions[index].phase {
                Phase::Staging => {
                    self.remove_staged(&journal.transactions[index]).await?;
                    journal.transactions[index].phase = Phase::Aborted;
                    journal_changed = true;
                }
                Phase::Committing => {
                    self.publish(&journal.transactions[index]).await?;
                    journal.transactions[index].phase = Phase::Committed;
                    write_journal(&self.journal_path, &journal)?;
                    journal_changed = false;
                    self.remove_staged(&journal.transactions[index]).await?;
                }
                Phase::Committed => {
                    self.remove_staged(&journal.transactions[index]).await?;
                }
                Phase::Prepared | Phase::Aborted => {}
            }
        }
        if journal_changed {
            write_journal(&self.journal_path, &journal)?;
        }
        Ok(())
    }

    async fn publish(&self, transaction: &Transaction) -> Result<(), StoreError> {
        for entry in &transaction.entries {
            let staged = parse_ref(&entry.staged)?;
            let published = parse_ref(&entry.published)?;
            let value = self.inner.get(&staged).await?;
            self.inner.put(&published, &value).await?;
        }
        Ok(())
    }

    async fn remove_staged(&self, transaction: &Transaction) -> Result<(), StoreError> {
        for entry in &transaction.entries {
            self.inner.delete(&parse_ref(&entry.staged)?).await?;
        }
        Ok(())
    }

    async fn has_live_transaction(&self) -> bool {
        self.journal
            .lock()
            .await
            .transactions
            .iter()
            .any(|transaction| {
                matches!(
                    transaction.phase,
                    Phase::Staging | Phase::Prepared | Phase::Committing
                )
            })
    }
}

#[async_trait]
impl SecretStore for PreparedVaultStore {
    async fn ready(&self) -> Result<(), StoreError> {
        self.inner.ready().await
    }

    async fn get(&self, reference: &CredentialRef) -> Result<Secret, StoreError> {
        self.inner.get(reference).await
    }

    async fn put(&self, reference: &CredentialRef, secret: &Secret) -> Result<(), StoreError> {
        if self.has_live_transaction().await {
            return Err(StoreError::Conflict {
                path: "<vault-prepared-store>".to_owned(),
                reason: "a prepared secret transaction owns the mutation slot".to_owned(),
            });
        }
        self.inner.put(reference, secret).await
    }

    async fn delete(&self, reference: &CredentialRef) -> Result<(), StoreError> {
        if self.has_live_transaction().await {
            return Err(StoreError::Conflict {
                path: "<vault-prepared-store>".to_owned(),
                reason: "a prepared secret transaction owns the mutation slot".to_owned(),
            });
        }
        self.inner.delete(reference).await
    }

    async fn references(&self, scope: &CredentialScope) -> Result<Vec<CredentialRef>, StoreError> {
        self.inner.references(scope).await
    }

    async fn apply(&self, _batch: &SecretBatch) -> Result<(), StoreError> {
        Err(StoreError::Unsupported {
            operation: "atomic batch".to_owned(),
            reason: "use the prepared transaction protocol for hosted Vault generations".to_owned(),
        })
    }
}

#[async_trait]
impl PreparedSecretStore for PreparedVaultStore {
    async fn prepare(
        &self,
        id: SecretTransactionId,
        digest: SecretProposalDigest,
        batch: &SecretBatch,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        let key = hex::encode(id.protocol_bytes());
        let generation = generation_value(id);
        let digest = hex::encode(digest.protocol_bytes());
        {
            let journal = self.journal.lock().await;
            if generation <= journal.retired_through {
                return Err(PreparedSecretError::Retired);
            }
            if let Some(existing) = journal.transactions.iter().find(|item| item.id == key) {
                if existing.phase == Phase::Aborted {
                    return Err(PreparedSecretError::TransactionIdReused);
                }
                if existing.digest != digest {
                    return Err(PreparedSecretError::DigestMismatch);
                }
                return match existing.phase {
                    Phase::Prepared => Ok(SecretTransactionState::Prepared),
                    Phase::Committed => Ok(SecretTransactionState::Committed),
                    Phase::Staging | Phase::Committing => Err(PreparedSecretError::Busy),
                    Phase::Aborted => unreachable!("aborted transactions return above"),
                };
            }
            if journal.transactions.iter().any(|transaction| {
                matches!(
                    transaction.phase,
                    Phase::Staging | Phase::Prepared | Phase::Committing
                )
            }) {
                return Err(PreparedSecretError::Busy);
            }
            if journal.transactions.len() >= MAX_TERMINAL_TRANSACTIONS {
                return Err(PreparedSecretError::Capacity);
            }
        }
        let puts = batch
            .put_entries()
            .filter(|entries| !entries.is_empty())
            .ok_or(PreparedSecretError::InvalidBatch)?;
        let mut entries = Vec::with_capacity(puts.len());
        let mut staged_values = Vec::with_capacity(puts.len());
        for (index, (published, value)) in puts.into_iter().enumerate() {
            let staged =
                staged_ref(published, &id, index).map_err(|_| PreparedSecretError::InvalidBatch)?;
            entries.push(Entry {
                published: TenantLayout.render(published),
                staged: TenantLayout.render(&staged),
            });
            staged_values.push((staged, value.clone()));
        }
        {
            let mut journal = self.journal.lock().await;
            journal.transactions.push(Transaction {
                id: key.clone(),
                generation,
                digest,
                phase: Phase::Staging,
                entries,
            });
            write_journal(&self.journal_path, &journal)
                .map_err(|_| PreparedSecretError::Backend)?;
        }
        for (staged, value) in staged_values {
            if self.inner.put(&staged, &value).await.is_err() {
                let transaction = self
                    .journal
                    .lock()
                    .await
                    .transactions
                    .iter()
                    .find(|item| item.id == key)
                    .cloned()
                    .ok_or(PreparedSecretError::Backend)?;
                if self.remove_staged(&transaction).await.is_ok() {
                    let mut journal = self.journal.lock().await;
                    if let Some(transaction) =
                        journal.transactions.iter_mut().find(|item| item.id == key)
                    {
                        transaction.phase = Phase::Aborted;
                    }
                    let _ = write_journal(&self.journal_path, &journal);
                }
                return Err(PreparedSecretError::Backend);
            }
        }
        let mut journal = self.journal.lock().await;
        let transaction = journal
            .transactions
            .iter_mut()
            .find(|item| item.id == key)
            .ok_or(PreparedSecretError::Backend)?;
        transaction.phase = Phase::Prepared;
        write_journal(&self.journal_path, &journal).map_err(|_| PreparedSecretError::Backend)?;
        Ok(SecretTransactionState::Prepared)
    }

    async fn state(
        &self,
        id: SecretTransactionId,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        let journal = self.journal.lock().await;
        if generation_value(id) <= journal.retired_through {
            return Err(PreparedSecretError::Retired);
        }
        Ok(
            match journal
                .transactions
                .iter()
                .find(|item| item.id == hex::encode(id.protocol_bytes()))
                .map(|item| item.phase)
            {
                Some(Phase::Prepared | Phase::Staging | Phase::Committing) => {
                    SecretTransactionState::Prepared
                }
                Some(Phase::Committed) => SecretTransactionState::Committed,
                Some(Phase::Aborted) | None => SecretTransactionState::Absent,
            },
        )
    }

    async fn commit(
        &self,
        id: SecretTransactionId,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        let key = hex::encode(id.protocol_bytes());
        let transaction = {
            let mut journal = self.journal.lock().await;
            if generation_value(id) <= journal.retired_through {
                return Err(PreparedSecretError::Retired);
            }
            let transaction = journal
                .transactions
                .iter_mut()
                .find(|item| item.id == key)
                .ok_or(PreparedSecretError::NotPrepared)?;
            match transaction.phase {
                Phase::Committed => return Ok(SecretTransactionState::Committed),
                Phase::Aborted => return Err(PreparedSecretError::TransactionIdReused),
                Phase::Staging => return Err(PreparedSecretError::Busy),
                Phase::Prepared | Phase::Committing => transaction.phase = Phase::Committing,
            }
            let transaction = transaction.clone();
            write_journal(&self.journal_path, &journal)
                .map_err(|_| PreparedSecretError::Backend)?;
            transaction
        };
        self.publish(&transaction)
            .await
            .map_err(|_| PreparedSecretError::Backend)?;
        {
            let mut journal = self.journal.lock().await;
            let stored = journal
                .transactions
                .iter_mut()
                .find(|item| item.id == key)
                .ok_or(PreparedSecretError::Backend)?;
            stored.phase = Phase::Committed;
            write_journal(&self.journal_path, &journal)
                .map_err(|_| PreparedSecretError::Backend)?;
        }
        let _ = self.remove_staged(&transaction).await;
        Ok(SecretTransactionState::Committed)
    }

    async fn abort(
        &self,
        id: SecretTransactionId,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        let key = hex::encode(id.protocol_bytes());
        let transaction = {
            let mut journal = self.journal.lock().await;
            if generation_value(id) <= journal.retired_through {
                return Err(PreparedSecretError::Retired);
            }
            if let Some(existing) = journal.transactions.iter_mut().find(|item| item.id == key) {
                match existing.phase {
                    Phase::Committed => return Err(PreparedSecretError::AlreadyCommitted),
                    Phase::Committing => return Err(PreparedSecretError::Busy),
                    Phase::Aborted => return Ok(SecretTransactionState::Absent),
                    Phase::Staging | Phase::Prepared => {}
                }
                existing.clone()
            } else {
                if journal.transactions.iter().any(|transaction| {
                    matches!(
                        transaction.phase,
                        Phase::Staging | Phase::Prepared | Phase::Committing
                    )
                }) {
                    return Err(PreparedSecretError::Busy);
                }
                journal.transactions.push(Transaction {
                    id: key,
                    generation: generation_value(id),
                    digest: String::new(),
                    phase: Phase::Aborted,
                    entries: Vec::new(),
                });
                write_journal(&self.journal_path, &journal)
                    .map_err(|_| PreparedSecretError::Backend)?;
                return Ok(SecretTransactionState::Absent);
            }
        };
        self.remove_staged(&transaction)
            .await
            .map_err(|_| PreparedSecretError::Backend)?;
        let mut journal = self.journal.lock().await;
        let stored = journal
            .transactions
            .iter_mut()
            .find(|item| item.id == key)
            .ok_or(PreparedSecretError::Backend)?;
        stored.phase = Phase::Aborted;
        write_journal(&self.journal_path, &journal).map_err(|_| PreparedSecretError::Backend)?;
        Ok(SecretTransactionState::Absent)
    }

    async fn reclaim(
        &self,
        through: SecretTransactionGeneration,
    ) -> Result<(), PreparedSecretError> {
        let through = u64::from_be_bytes(through.protocol_bytes());
        let mut journal = self.journal.lock().await;
        if journal.transactions.iter().any(|transaction| {
            transaction.generation <= through
                && matches!(
                    transaction.phase,
                    Phase::Staging | Phase::Prepared | Phase::Committing
                )
        }) {
            return Err(PreparedSecretError::Busy);
        }
        journal.retired_through = journal.retired_through.max(through);
        let retired_through = journal.retired_through;
        journal
            .transactions
            .retain(|transaction| transaction.generation > retired_through);
        write_journal(&self.journal_path, &journal).map_err(|_| PreparedSecretError::Backend)
    }
}

fn generation_value(id: SecretTransactionId) -> u64 {
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&id.protocol_bytes()[..8]);
    u64::from_be_bytes(bytes)
}

fn staged_ref(
    published: &CredentialRef,
    id: &SecretTransactionId,
    index: usize,
) -> Result<CredentialRef, String> {
    let suffix = hex::encode(&id.protocol_bytes()[16..]);
    let credential = format!("prepared_{suffix}_{index}");
    match published.instance() {
        Some(instance) => CredentialRef::for_instance(
            published.tenant(),
            published.authority(),
            instance.as_str(),
            published.service(),
            &credential,
        ),
        None => CredentialRef::new(
            published.tenant(),
            published.authority(),
            published.service(),
            &credential,
        ),
    }
}

fn parse_ref(path: &str) -> Result<CredentialRef, StoreError> {
    TenantLayout
        .parse(path)
        .map_err(|reason| StoreError::Layout { reason })
}

fn read_journal(path: &Path) -> Result<Journal, StoreError> {
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Journal::default()),
        Err(error) => return Err(journal_error(path, &error.to_string())),
    };
    let metadata = file
        .metadata()
        .map_err(|error| journal_error(path, &error.to_string()))?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > MAX_JOURNAL_BYTES
    {
        return Err(journal_error(path, "journal ownership or shape is unsafe"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut file)
        .take(MAX_JOURNAL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| journal_error(path, &error.to_string()))?;
    let journal: Journal = serde_json::from_slice(&bytes)
        .map_err(|_| journal_error(path, "journal payload is invalid"))?;
    if journal.version != JOURNAL_VERSION {
        return Err(journal_error(path, "journal version is unsupported"));
    }
    Ok(journal)
}

fn write_journal(path: &Path, journal: &Journal) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| journal_error(path, "journal parent is missing"))?;
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(&temporary)
        .map_err(|error| journal_error(path, &error.to_string()))?;
    let bytes = serde_json::to_vec(journal)
        .map_err(|_| journal_error(path, "journal could not be encoded"))?;
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err(journal_error(path, "journal exceeded its size bound"));
    }
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| journal_error(path, &error.to_string()))?;
    fs::rename(&temporary, path).map_err(|error| journal_error(path, &error.to_string()))?;
    OpenOptions::new()
        .read(true)
        .open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| journal_error(path, &error.to_string()))
}

fn validate_parent(path: &Path) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| journal_error(path, "journal parent is missing"))?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|error| journal_error(path, &error.to_string()))?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(journal_error(
            path,
            "journal parent ownership or mode is unsafe",
        ));
    }
    Ok(())
}

fn remove_incomplete_temporary(path: &Path) -> Result<(), StoreError> {
    let temporary = path.with_extension("json.tmp");
    let metadata = match fs::symlink_metadata(&temporary) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(journal_error(path, &error.to_string())),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(journal_error(path, "incomplete journal file is unsafe"));
    }
    fs::remove_file(&temporary).map_err(|error| journal_error(path, &error.to_string()))
}

fn journal_error(path: &Path, reason: &str) -> StoreError {
    StoreError::Unreachable {
        path: path.display().to_string(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use connector_secrets::MemoryStore;

    const OLD: &str = "SENTINEL-NOT-A-REAL-SECRET-old";
    const NEW: &str = "SENTINEL-NOT-A-REAL-SECRET-new";

    fn transaction() -> SecretTransactionId {
        let generation = SecretTransactionGeneration::from_protocol_bytes(1_u64.to_be_bytes())
            .expect("non-zero generation");
        SecretTransactionId::new(generation, [7; 24])
    }

    fn reference() -> CredentialRef {
        CredentialRef::for_instance(
            "tenant-dev",
            "com.slack.api",
            "00000000-0000-4000-8000-000000000001",
            "default",
            "bot_token",
        )
        .expect("valid reference")
    }

    fn batch(value: &str) -> SecretBatch {
        let reference = reference();
        let mut batch = SecretBatch::new(
            CredentialScope::new(reference.tenant(), reference.authority()).expect("valid scope"),
        );
        batch.put(reference, Secret::new(value)).expect("valid put");
        batch
    }

    #[tokio::test]
    async fn clean_initialize_does_not_create_an_empty_journal() {
        let root = tempfile::tempdir().expect("temporary root");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).expect("owner mode");
        let journal = root.path().join("vault-prepared-transactions.json");
        let store =
            PreparedVaultStore::open(Arc::new(MemoryStore::new()), journal.clone()).expect("open");

        store.initialize().await.expect("initialize");

        assert!(!journal.exists());
    }

    #[tokio::test]
    async fn candidate_values_stay_in_the_secret_store_and_are_invisible_until_commit() {
        let root = tempfile::tempdir().expect("temporary root");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).expect("owner mode");
        let inner = Arc::new(MemoryStore::new());
        inner
            .put(&reference(), &Secret::new(OLD))
            .await
            .expect("seed");
        let store = PreparedVaultStore::open(
            inner.clone(),
            root.path().join("vault-prepared-transactions.json"),
        )
        .expect("open");
        store.initialize().await.expect("initialize");
        let id = transaction();
        store
            .prepare(
                id,
                SecretProposalDigest::from_protocol_bytes([3; 32]),
                &batch(NEW),
            )
            .await
            .expect("prepare");
        assert_eq!(store.get(&reference()).await.unwrap().expose_secret(), OLD);
        assert!(matches!(
            store.put(&reference(), &Secret::new("blocked")).await,
            Err(StoreError::Conflict { .. })
        ));
        store.commit(id).await.expect("commit");
        assert_eq!(store.get(&reference()).await.unwrap().expose_secret(), NEW);

        let journal = fs::read_to_string(root.path().join("vault-prepared-transactions.json"))
            .expect("read journal");
        assert!(!journal.contains(OLD));
        assert!(!journal.contains(NEW));
        drop(store);
        let reopened =
            PreparedVaultStore::open(inner, root.path().join("vault-prepared-transactions.json"))
                .expect("reopen");
        reopened.initialize().await.expect("recover");
        assert_eq!(
            reopened.state(id).await.expect("state"),
            SecretTransactionState::Committed
        );
    }
}
