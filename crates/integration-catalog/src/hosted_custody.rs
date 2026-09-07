//! Ordinary prepared custody commit, retaining only the failure stage.

use super::*;
use service::HostedCustodyFailure;

impl Inner {
    pub(super) async fn commit_connection(
        &self,
        session_ref: &str,
        session: &Session,
        secret: Secret,
        verification: Option<(String, u64)>,
    ) -> Result<String, HostedCustodyFailure> {
        let provider = catalog::provider(catalog::ProviderKey::id(&session.provider))
            .ok_or(HostedCustodyFailure::Setup)?;
        let authority = provider.authority.ok_or(HostedCustodyFailure::Setup)?;
        let credential = self
            .profile(&session.provider, &session.credential)
            .ok_or(HostedCustodyFailure::Setup)?;
        let instance = random_uuid().map_err(|_| HostedCustodyFailure::Setup)?;
        let entry = CatalogIntegrationConfig {
            provider: session.provider.clone(),
            instance: Some(instance.clone()),
            label: Some(session.label.clone()),
            grant_ref: self.grant_ref.clone(),
            initiation: InitiationConfig::Platform,
            allow_writes: false,
            endpoints: session.binding.endpoints.clone(),
            // Hosted self-service stores no user half today, so a `basic` connector is not
            // connectable through it. Stated as an empty map rather than left implicit: the
            // personal placement fills this from `[catalog.usernames]`, and the hosted gap is a
            // missing acquisition surface, not a different resolution rule.
            usernames: BTreeMap::new(),
            operator_approved: !session.binding.endpoints.is_empty(),
            credential: Some(session.credential.clone()),
            network: session.binding.network,
            credential_file: None,
            oauth: None,
        };
        let reference = credential_address(&self.tenant_id, authority, &entry, credential.leaf)
            .map_err(|_| HostedCustodyFailure::Setup)?;
        let connection = StoredConnection {
            connection_ref: connection_ref(&session.provider, &instance),
            provider: session.provider.clone(),
            instance,
            credential: session.credential.clone(),
            label: session.label.clone(),
            owner_subject: session.owner_subject.clone(),
            binding: session.binding.clone(),
            actor: match credential.subject {
                Subject::User => StoredActor::User,
                Subject::App => StoredActor::App,
                Subject::Unstated => return Err(HostedCustodyFailure::Setup),
            },
            credential_sha256: verification.as_ref().map(|value| value.0.clone()),
            last_verified_at_unix_ms: verification.map(|value| value.1),
        };
        let mut batch = SecretBatch::new(
            CredentialScope::new(&self.tenant_id, authority)
                .map_err(|_| HostedCustodyFailure::Setup)?,
        );
        batch
            .put(reference, secret)
            .map_err(|_| HostedCustodyFailure::Setup)?;
        let mut current_intent = None;
        let prepared = service::prepare_credential_batch(
            self.prepared.as_ref(),
            proposal_digest(&batch),
            &batch,
            |minimum| {
                let (transaction, generation) = self
                    .reserve_transaction(minimum)
                    .map_err(|_| HostedCustodyFailure::ReserveState)?;
                let transaction_id = hex::encode(transaction.protocol_bytes());
                let mut metadata = lock(&self.metadata);
                // A repeated callback means the preceding prepare returned Retired, proving
                // it had no effects. Replace only that exact write-ahead intent.
                if let Some(previous) = current_intent {
                    let previous = hex::encode(SecretTransactionId::protocol_bytes(previous));
                    metadata
                        .pending
                        .retain(|pending| pending.transaction_id != previous);
                }
                current_intent = Some(transaction);
                metadata.pending.push(PendingCommit {
                    transaction_id,
                    published: false,
                    discarded: false,
                    intent: true,
                    connection: connection.clone(),
                });
                self.persist(&metadata)
                    .map_err(|_| HostedCustodyFailure::PersistPending)?;
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
                    service::CredentialPreparationError::Store(error) => {
                        HostedCustodyFailure::Prepare(error)
                    }
                });
            }
        };
        let transaction_id = hex::encode(transaction.protocol_bytes());
        self.prepared
            .commit(transaction)
            .await
            .map_err(HostedCustodyFailure::Commit)?;
        {
            let mut metadata = lock(&self.metadata);
            let prior = metadata.clone();
            if let Some(pending) = metadata
                .pending
                .iter_mut()
                .find(|pending| pending.transaction_id == transaction_id)
            {
                pending.published = true;
            }
            metadata
                .connections
                .retain(|candidate| candidate.connection_ref != connection.connection_ref);
            metadata.connections.push(connection.clone());
            metadata
                .connections
                .sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
            if self.persist(&metadata).is_err() {
                *metadata = prior;
                return Err(HostedCustodyFailure::PersistConnection);
            }
        }
        self.acknowledge_publications().await;
        let _ = session_ref;
        Ok(connection.connection_ref)
    }
    async fn discard_pending(&self, transaction: SecretTransactionId, transaction_id: &str) {
        // This path owns a newly persisted write-ahead intent, never a legacy receipt.
        let absent = matches!(
            self.prepared.abort(transaction).await,
            Ok(SecretTransactionState::Absent)
                | Err(connector_secrets::PreparedSecretError::Retired)
        );
        let durable = {
            let mut metadata = lock(&self.metadata);
            let prior = metadata.clone();
            if absent {
                if let Some(receipt) = metadata
                    .pending
                    .iter_mut()
                    .find(|receipt| receipt.transaction_id == transaction_id)
                {
                    receipt.discarded = true;
                }
            }
            if self.persist(&metadata).is_ok() {
                true
            } else {
                *metadata = prior;
                false
            }
        };
        if absent && durable {
            self.acknowledge_publications().await;
        }
    }

    pub(super) async fn acknowledge_publications(&self) {
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
            if service::acknowledge_credential_publication(self.prepared.as_ref(), transaction)
                .await
            {
                let mut metadata = lock(&self.metadata);
                let prior = metadata.clone();
                metadata
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_id);
                if self.persist(&metadata).is_err() {
                    *metadata = prior;
                }
            }
        }
    }
}
