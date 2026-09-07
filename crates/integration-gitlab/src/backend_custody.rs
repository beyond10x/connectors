//! Prepared credential intents and exact owner acknowledgement.

use super::*;

impl GitlabInner {
    pub(super) async fn commit_credentials(
        &self,
        mut connection: StoredConnection,
        credentials: CredentialValues,
    ) -> Result<(), GitlabError> {
        let mut batch = SecretBatch::new(
            CredentialScope::new(&self.tenant_id, AUTHORITY)
                .map_err(|_| GitlabError::new("credential-address"))?,
        );
        batch
            .put(
                self.connection_credential_ref(&connection, ACCESS_TOKEN_CREDENTIAL)?,
                credentials.access_token,
            )
            .map_err(|_| GitlabError::new("credential-batch"))?;
        if let Some(refresh) = credentials.refresh_token {
            batch
                .put(
                    self.connection_credential_ref(&connection, REFRESH_TOKEN_CREDENTIAL)?,
                    refresh,
                )
                .map_err(|_| GitlabError::new("credential-batch"))?;
        }
        let mut current_intent = None;
        let prepared = service::prepare_credential_batch(
            self.credential_store.as_ref(),
            proposal_digest(&batch),
            &batch,
            |minimum| {
                let (transaction, generation) = self.reserve_transaction(minimum)?;
                connection.credential_generation = u64::from_be_bytes(generation.protocol_bytes());
                let transaction_id = hex::encode(transaction.protocol_bytes());
                let mut state = lock(&self.metadata);
                // A repeated callback means the preceding prepare returned Retired, proving
                // it had no effects. Replace only that exact write-ahead intent.
                if let Some(previous) = current_intent {
                    let previous = hex::encode(SecretTransactionId::protocol_bytes(previous));
                    state
                        .pending
                        .retain(|pending| pending.transaction_id != previous);
                }
                current_intent = Some(transaction);
                state.pending.push(PendingCommit {
                    transaction_id,
                    published: false,
                    discarded: false,
                    intent: true,
                    connection: connection.clone(),
                });
                self.persist(&state)?;
                Ok((transaction, generation))
            },
        )
        .await;
        let (transaction, _) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                if let Some(transaction) = current_intent {
                    self.discard_pending(transaction, &hex::encode(transaction.protocol_bytes()))
                        .await;
                }
                return Err(match error {
                    service::CredentialPreparationError::Reservation(error) => error,
                    service::CredentialPreparationError::Store(_) => {
                        GitlabError::new("credential-prepare")
                    }
                });
            }
        };
        let transaction_id = hex::encode(transaction.protocol_bytes());
        self.credential_store
            .commit(transaction)
            .await
            .map_err(|_| GitlabError::new("credential-commit"))?;
        {
            let mut state = lock(&self.metadata);
            let prior = state.clone();
            if let Some(pending) = state
                .pending
                .iter_mut()
                .find(|pending| pending.transaction_id == transaction_id)
            {
                pending.published = true;
            }
            upsert_connection(&mut state.connections, connection);
            if let Err(error) = self.persist(&state) {
                *state = prior;
                return Err(error);
            }
        }
        self.acknowledge_publications().await;
        Ok(())
    }

    fn reserve_transaction(
        &self,
        minimum: u64,
    ) -> Result<(SecretTransactionId, SecretTransactionGeneration), GitlabError> {
        let mut state = lock(&self.metadata);
        state.next_transaction_generation = state.next_transaction_generation.max(minimum);
        let generation = SecretTransactionGeneration::from_protocol_bytes(
            state.next_transaction_generation.to_be_bytes(),
        )
        .ok_or_else(|| GitlabError::new("transaction-generation"))?;
        state.next_transaction_generation = state
            .next_transaction_generation
            .checked_add(1)
            .ok_or_else(|| GitlabError::new("transaction-generation"))?;
        self.persist(&state)?;
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| GitlabError::new("randomness"))?;
        Ok((SecretTransactionId::new(generation, nonce), generation))
    }

    pub(super) async fn recover_pending(&self) -> Result<(), GitlabError> {
        for pass in 0..2 {
            let pending_commits = { lock(&self.metadata).pending.clone() };
            for pending in pending_commits {
                if pending.connection.grant_ref != self.policy.user_grant_ref {
                    continue;
                }
                let transaction = decode_transaction(&pending.transaction_id)?;
                if !pending.published && !pending.discarded {
                    match service::recover_credential_intent(
                        self.credential_store.as_ref(),
                        transaction,
                        pending.intent,
                    )
                    .await
                    .map_err(|_| GitlabError::new("credential-recovery"))?
                    {
                        service::CredentialRecovery::Committed => {}
                        service::CredentialRecovery::Deferred => {
                            if pass == 1 {
                                return Err(GitlabError::new("credential-recovery"));
                            }
                            continue;
                        }
                        service::CredentialRecovery::Discarded => {
                            let mut state = lock(&self.metadata);
                            let prior = state.clone();
                            if let Some(receipt) = state
                                .pending
                                .iter_mut()
                                .find(|receipt| receipt.transaction_id == pending.transaction_id)
                            {
                                receipt.discarded = true;
                            }
                            if let Err(error) = self.persist(&state) {
                                *state = prior;
                                return Err(error);
                            }
                            continue;
                        }
                    }
                    let mut state = lock(&self.metadata);
                    let prior = state.clone();
                    if let Some(receipt) = state
                        .pending
                        .iter_mut()
                        .find(|candidate| candidate.transaction_id == pending.transaction_id)
                    {
                        receipt.published = true;
                    }
                    upsert_connection(&mut state.connections, pending.connection);
                    if let Err(error) = self.persist(&state) {
                        *state = prior;
                        return Err(error);
                    }
                }
                self.acknowledge_publications().await;
            }
            self.acknowledge_publications().await;
        }
        Ok(())
    }

    async fn discard_pending(&self, transaction: SecretTransactionId, transaction_id: &str) {
        // This path owns a newly persisted write-ahead intent, never a legacy receipt.
        let absent = matches!(
            self.credential_store.abort(transaction).await,
            Ok(SecretTransactionState::Absent)
                | Err(connector_secrets::PreparedSecretError::Retired)
        );
        let durable = {
            let mut state = lock(&self.metadata);
            let prior = state.clone();
            if absent {
                if let Some(receipt) = state
                    .pending
                    .iter_mut()
                    .find(|receipt| receipt.transaction_id == transaction_id)
                {
                    receipt.discarded = true;
                }
            }
            if self.persist(&state).is_ok() {
                true
            } else {
                *state = prior;
                false
            }
        };
        if absent && durable {
            self.acknowledge_publications().await;
        }
    }

    async fn acknowledge_publications(&self) {
        let receipts = {
            lock(&self.metadata)
                .pending
                .iter()
                .filter(|pending| pending.published || pending.discarded)
                .map(|pending| pending.transaction_id.clone())
                .collect::<Vec<_>>()
        };
        for transaction_id in receipts {
            let Ok(transaction) = decode_transaction(&transaction_id) else {
                continue;
            };
            if service::acknowledge_credential_publication(
                self.credential_store.as_ref(),
                transaction,
            )
            .await
            {
                let mut state = lock(&self.metadata);
                let prior = state.clone();
                state
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_id);
                if self.persist(&state).is_err() {
                    *state = prior;
                }
            }
        }
    }
}
