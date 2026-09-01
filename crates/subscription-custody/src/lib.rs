#![forbid(unsafe_code)]

//! Connector-owned custody for a user subscription credential and the narrow lease that lets an
//! explicitly bound Harness attempt spend it. No API in this crate lists or exports stored values.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use connector_secrets::{CredentialRef, Secret, SecretStore, StoreError};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

const AUTHORITY: &str = "com.anthropic.claude-code";
const CREDENTIAL: &str = "subscription_token";
const DEFAULT_SERVICE: &str = "default";
const MAX_CREDENTIAL_BYTES: usize = 16 * 1024;
const MAX_ATTEMPT_BYTES: usize = 256;
const MAX_LEASES: usize = 10_000;
const MAX_LEASE_USES: u16 = 1_024;

#[derive(Debug, thiserror::Error)]
pub enum CustodyError {
    #[error("the subscription credential is malformed")]
    InvalidCredential,
    #[error("the attempt binding is malformed")]
    InvalidAttempt,
    #[error("subscription credential custody is unavailable")]
    Unavailable,
    #[error("no subscription credential is connected")]
    NotConnected,
    #[error("the subscription credential lease was refused")]
    LeaseRefused,
}

/// The only secret returned to an HTTP adapter: a short-lived lease capability, never the stored
/// provider credential.
pub struct LeaseCapability {
    pub lease_id: String,
    token: Zeroizing<String>,
    pub expires_at: u64,
}

impl LeaseCapability {
    #[must_use]
    pub fn expose_at_transport_boundary(&self) -> &str {
        self.token.as_str()
    }
}

impl std::fmt::Debug for LeaseCapability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseCapability")
            .field("lease_id", &self.lease_id)
            .field("token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

struct Lease {
    token_sha256: [u8; 32],
    credential_ref: CredentialRef,
    attempt_id: String,
    expires_at: u64,
    remaining_uses: u16,
}

/// One in-process lease authority over a durable Connector-owned secret store. Restarting loses
/// leases and therefore revokes them; it never loses the underlying connection.
#[derive(Clone)]
pub struct SubscriptionCustody {
    store: Arc<dyn SecretStore>,
    leases: Arc<Mutex<BTreeMap<String, Lease>>>,
}

impl std::fmt::Debug for SubscriptionCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionCustody")
            .field("store", &"SecretStore")
            .field("leases", &"[REDACTED]")
            .finish()
    }
}

impl SubscriptionCustody {
    #[must_use]
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self {
            store,
            leases: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Replaces the credential for one verified tenant subject.
    pub async fn connect(
        &self,
        tenant_id: &str,
        subject: &str,
        credential: Zeroizing<String>,
    ) -> Result<(), CustodyError> {
        if credential.len() < 16
            || credential.len() > MAX_CREDENTIAL_BYTES
            || credential.chars().any(char::is_whitespace)
        {
            return Err(CustodyError::InvalidCredential);
        }
        let reference = credential_ref(tenant_id, subject)?;
        let mut leases = self.leases.lock().await;
        self.store
            .put(&reference, &Secret::new(credential.as_str()))
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.credential_ref != reference);
        Ok(())
    }

    /// Reports presence without returning or hashing the provider credential.
    pub async fn connected(&self, tenant_id: &str, subject: &str) -> Result<bool, CustodyError> {
        let reference = credential_ref(tenant_id, subject)?;
        self.store
            .exists(&reference)
            .await
            .map_err(|_| CustodyError::Unavailable)
    }

    /// Revokes the connection and every currently live lease for that credential address.
    pub async fn disconnect(&self, tenant_id: &str, subject: &str) -> Result<(), CustodyError> {
        let reference = credential_ref(tenant_id, subject)?;
        let mut leases = self.leases.lock().await;
        self.store
            .delete(&reference)
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.credential_ref != reference);
        Ok(())
    }

    /// Creates a capability bound to one attempt, a duration, and a finite number of wire calls.
    pub async fn lease(
        &self,
        tenant_id: &str,
        subject: &str,
        attempt_id: &str,
        ttl: Duration,
        maximum_uses: u16,
    ) -> Result<LeaseCapability, CustodyError> {
        validate_attempt(attempt_id)?;
        if ttl.is_zero()
            || ttl > Duration::from_secs(60 * 60)
            || maximum_uses == 0
            || maximum_uses > MAX_LEASE_USES
        {
            return Err(CustodyError::LeaseRefused);
        }
        let reference = credential_ref(tenant_id, subject)?;
        let mut leases = self.leases.lock().await;
        match self.store.get(&reference).await {
            Ok(_) => {}
            Err(StoreError::NotFound { .. }) => return Err(CustodyError::NotConnected),
            Err(_) => return Err(CustodyError::Unavailable),
        }
        let lease_id = connector_oauth::random_token(18).map_err(|_| CustodyError::Unavailable)?;
        let token = connector_oauth::random_token(32).map_err(|_| CustodyError::Unavailable)?;
        let current_time = now()?;
        let expires_at = current_time
            .checked_add(ttl.as_secs())
            .ok_or(CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.expires_at > current_time);
        if leases.len() >= MAX_LEASES {
            return Err(CustodyError::Unavailable);
        }
        leases.insert(
            lease_id.clone(),
            Lease {
                token_sha256: Sha256::digest(token.as_bytes()).into(),
                credential_ref: reference,
                attempt_id: attempt_id.to_owned(),
                expires_at,
                remaining_uses: maximum_uses,
            },
        );
        Ok(LeaseCapability {
            lease_id,
            token: Zeroizing::new(token),
            expires_at,
        })
    }

    /// Redeems one use and returns the provider credential directly to the Harness bearer source.
    pub async fn redeem(
        &self,
        lease_id: &str,
        lease_token: &str,
        attempt_id: &str,
    ) -> Result<Secret, CustodyError> {
        validate_attempt(attempt_id)?;
        let mut leases = self.leases.lock().await;
        let reference = {
            let lease = leases.get_mut(lease_id).ok_or(CustodyError::LeaseRefused)?;
            let candidate: [u8; 32] = Sha256::digest(lease_token.as_bytes()).into();
            if !constant_time_equal(&lease.token_sha256, &candidate)
                || lease.attempt_id != attempt_id
                || lease.expires_at <= now()?
                || lease.remaining_uses == 0
            {
                return Err(CustodyError::LeaseRefused);
            }
            lease.remaining_uses -= 1;
            lease.credential_ref.clone()
        };
        self.store
            .get(&reference)
            .await
            .map_err(|error| match error {
                StoreError::NotFound { .. } => CustodyError::NotConnected,
                _ => CustodyError::Unavailable,
            })
    }
}

fn credential_ref(tenant_id: &str, subject: &str) -> Result<CredentialRef, CustodyError> {
    let digest = hex::encode(Sha256::digest(subject.as_bytes()));
    let instance = format!(
        "{}-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[12..16],
        &digest[16..20],
        &digest[20..32]
    );
    CredentialRef::for_instance(tenant_id, AUTHORITY, &instance, DEFAULT_SERVICE, CREDENTIAL)
        .map_err(|_| CustodyError::Unavailable)
}

fn validate_attempt(attempt_id: &str) -> Result<(), CustodyError> {
    if attempt_id.is_empty()
        || attempt_id.len() > MAX_ATTEMPT_BYTES
        || !attempt_id.bytes().all(|byte| byte.is_ascii_graphic())
    {
        Err(CustodyError::InvalidAttempt)
    } else {
        Ok(())
    }
}

fn now() -> Result<u64, CustodyError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| CustodyError::Unavailable)
}

fn constant_time_equal(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use connector_secrets::MemoryStore;

    use super::*;

    #[tokio::test]
    async fn custody_never_exports_but_an_exact_attempt_lease_can_redeem() {
        let custody = SubscriptionCustody::new(Arc::new(MemoryStore::new()));
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token".to_owned()),
            )
            .await
            .unwrap();
        assert!(custody
            .connected("tenant-one", "human-alice")
            .await
            .unwrap());
        let capability = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                1,
            )
            .await
            .unwrap();
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "wrong-attempt"
            )
            .await
            .is_err());
        let secret = custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one",
            )
            .await
            .unwrap();
        assert_eq!(secret.expose_secret(), "synthetic-subscription-token");
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one"
            )
            .await
            .is_err());
        assert!(!format!("{capability:?}").contains("synthetic"));
    }

    #[tokio::test]
    async fn replacing_a_credential_revokes_every_lease_over_the_old_generation() {
        let custody = SubscriptionCustody::new(Arc::new(MemoryStore::new()));
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token-one".to_owned()),
            )
            .await
            .unwrap();
        let capability = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                2,
            )
            .await
            .unwrap();
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token-two".to_owned()),
            )
            .await
            .unwrap();
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one"
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn disconnect_revokes_live_leases_and_removes_presence() {
        let custody = SubscriptionCustody::new(Arc::new(MemoryStore::new()));
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token".to_owned()),
            )
            .await
            .unwrap();
        let capability = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                2,
            )
            .await
            .unwrap();
        custody
            .disconnect("tenant-one", "human-alice")
            .await
            .unwrap();
        assert!(!custody
            .connected("tenant-one", "human-alice")
            .await
            .unwrap());
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one"
            )
            .await
            .is_err());
    }
}
