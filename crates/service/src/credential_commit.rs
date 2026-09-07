//! Share a retiring custody store while keeping reservation and publication with each owner.

use connector_secrets::{
    PreparedSecretError, PreparedSecretStore, SecretBatch, SecretProposalDigest,
    SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
};

/// Preparation never retries ambiguous outcomes. Reservation failures belong to the coordinator.
#[derive(Debug)]
pub enum CredentialPreparationError<E> {
    Reservation(E),
    Store(PreparedSecretError),
}

/// Reserve above the store's current retirement watermark and prepare one exact batch.
/// Only Retired proves that no candidate was staged, permitting a bounded new reservation.
/// The callback must durably reserve a generation at least `minimum` and persist its owner
/// intent before returning its id. A repeated callback replaces only the prior Retired intent.
pub async fn prepare_credential_batch<E>(
    store: &dyn PreparedSecretStore,
    digest: SecretProposalDigest,
    batch: &SecretBatch,
    mut reserve: impl FnMut(u64) -> Result<(SecretTransactionId, SecretTransactionGeneration), E>,
) -> Result<(SecretTransactionId, SecretTransactionGeneration), CredentialPreparationError<E>> {
    for attempt in 0..4 {
        let minimum = match store.retirement_watermark().await {
            Ok(Some(watermark)) => watermark
                .checked_next()
                .map(|next| u64::from_be_bytes(next.protocol_bytes()))
                .ok_or(CredentialPreparationError::Store(
                    PreparedSecretError::Retired,
                ))?,
            Ok(None) | Err(PreparedSecretError::Unsupported) => 1,
            Err(error) => return Err(CredentialPreparationError::Store(error)),
        };
        let (id, generation) = reserve(minimum).map_err(CredentialPreparationError::Reservation)?;
        if u64::from_be_bytes(generation.protocol_bytes()) < minimum
            || id.protocol_bytes()[..8] != generation.protocol_bytes()
        {
            return Err(CredentialPreparationError::Store(
                PreparedSecretError::InvalidBatch,
            ));
        }
        match store.prepare(id, digest, batch).await {
            Ok(SecretTransactionState::Prepared | SecretTransactionState::Committed) => {
                return Ok((id, generation))
            }
            Ok(SecretTransactionState::Absent) => {
                return Err(CredentialPreparationError::Store(
                    PreparedSecretError::NotPrepared,
                ))
            }
            Err(PreparedSecretError::Retired) if attempt < 3 => continue,
            Err(error) => return Err(CredentialPreparationError::Store(error)),
        }
    }
    unreachable!("the final preparation attempt always returns")
}

/// Recovery disposition for a durable owner intent; none implies provider verification.
pub enum CredentialRecovery {
    Committed,
    Discarded,
    Deferred,
}

/// Recover an exact receipt, fencing an absent write-ahead intent before its owner discards it.
/// A legacy receipt was written after prepare, so its retirement is never commit evidence.
pub async fn recover_credential_intent(
    store: &dyn PreparedSecretStore,
    transaction: SecretTransactionId,
    write_ahead: bool,
) -> Result<CredentialRecovery, PreparedSecretError> {
    match store.state(transaction).await {
        Ok(SecretTransactionState::Prepared) => match store.commit(transaction).await? {
            SecretTransactionState::Committed => Ok(CredentialRecovery::Committed),
            _ => Err(PreparedSecretError::NotPrepared),
        },
        Ok(SecretTransactionState::Committed) => Ok(CredentialRecovery::Committed),
        Ok(SecretTransactionState::Absent) => match store.abort(transaction).await {
            Ok(SecretTransactionState::Absent) => Ok(CredentialRecovery::Discarded),
            Err(PreparedSecretError::Retired) if write_ahead => Ok(CredentialRecovery::Discarded),
            Err(PreparedSecretError::Busy) => Ok(CredentialRecovery::Deferred),
            Err(error) => Err(error),
            _ => Err(PreparedSecretError::NotPrepared),
        },
        Err(PreparedSecretError::Retired) if write_ahead => Ok(CredentialRecovery::Discarded),
        Err(error) => Err(error),
    }
}

/// A coordinator uses this only after its own exact publication receipt is durable.
pub async fn acknowledge_credential_publication(
    store: &dyn PreparedSecretStore,
    transaction: SecretTransactionId,
) -> bool {
    matches!(
        store.acknowledge(transaction).await,
        Ok(()) | Err(PreparedSecretError::Retired)
    )
}

#[cfg(test)]
#[path = "credential_commit_tests.rs"]
mod tests;
