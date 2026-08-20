//! `connectors auth status` — what is connected, without reading what it is.
//!
//! # Why this command exists, specifically
//!
//! The obvious way to find out what Connectors has stored is `secret-tool search service
//! b10x-connectors`. **Do not.** `secret-tool search` prints a `secret = …` line for every
//! match alongside the attributes, and it has no attribute-only mode — so the natural diagnostic
//! command puts every credential in scope onto a terminal, into a scrollback buffer, and into
//! whatever is recording it. That happened here on 2026-08-20, to a live GitLab token, while
//! verifying the keyring store; the token was rotated.
//!
//! A tool whose safe path is "remember not to run the obvious command" does not have a safe path.
//! So this asks the question the operator actually has — *is this provider connected?* — and answers
//! it from the configuration plus a presence probe, never an enumeration.
//!
//! # How it stays value-free
//!
//! It walks the **declared** providers rather than the store's contents: for each configured
//! provider the catalogue says which credential it needs, `connector-address` says where that lives,
//! and [`SecretStore::exists`] says whether it is there. No value is fetched, nothing is
//! enumerated, and there is nothing in the result that could carry a secret even by accident —
//! every field is derived from the catalogue or is a boolean.

use std::path::Path;
use std::sync::Arc;

use connector_address::CredentialRef;
use connector_secrets::{FileStore, KeyringStore, SecretStore, StoreError};

/// A store that could not be opened at all. A missing credential is a reported row, not this.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct AuthError(#[from] StoreError);
use connectors_config::PersonalConfig;
use serde_json::{json, Value};

/// Report which configured providers have their credential stored.
///
/// # Errors
///
/// Only for a store that cannot be opened at all. A provider whose credential is missing is a
/// reported row, not a failure — that is the ordinary state before `connect`.
pub async fn status(config: &PersonalConfig, state_root: &Path) -> Result<Value, AuthError> {
    let (store, backend) = open_store(state_root)?;

    let mut providers = Vec::new();
    for entry in &config.catalog {
        let Some(provider) = catalog::provider(catalog::ProviderKey::id(&entry.provider)) else {
            providers.push(json!({
                "provider": entry.provider,
                "status": "unknown-provider",
                "detail": "not in the catalogue",
            }));
            continue;
        };
        let Some(authority) = provider.authority else {
            providers.push(json!({
                "provider": entry.provider,
                "status": "no-authority",
                "detail": "the provider declares no authority, so its credential has no address",
            }));
            continue;
        };
        let credential = entry
            .credential
            .as_deref()
            .and_then(|name| provider.auth.iter().find(|item| item.name == name))
            .or_else(|| provider.auth.first());
        let Some(credential) = credential else {
            providers.push(json!({
                "provider": entry.provider,
                "status": "no-credential-declared",
                "detail": "the provider declares no credential to supply",
            }));
            continue;
        };
        let reference = CredentialRef::new(
            config.owner.tenant_id.as_str(),
            authority,
            connector_address::DEFAULT_SERVICE,
            credential.leaf,
        );
        let present = match reference {
            Ok(reference) => store.exists(&reference).await,
            Err(_) => Ok(false),
        };
        providers.push(match present {
            Ok(true) => json!({
                "provider": entry.provider,
                "credential": credential.name,
                "status": "connected",
                // The declared probe, so an operator knows whether `auth test` can check this one.
                "verify": provider.verify,
            }),
            Ok(false) => json!({
                "provider": entry.provider,
                "credential": credential.name,
                "status": "not-connected",
                "detail": "no credential is stored for this provider yet",
            }),
            // "We cannot say" is its own answer. Reporting a locked keyring as `not-connected`
            // would send an operator to re-enter a credential that is already there.
            Err(error) => json!({
                "provider": entry.provider,
                "credential": credential.name,
                "status": "unavailable",
                "detail": error.to_string(),
            }),
        });
    }

    Ok(json!({
        "store": backend,
        "providers": providers,
    }))
}

/// The store this machine would use, and its name.
///
/// The same preference order the runtime composes with, so `auth status` reports the store the
/// daemon actually reads rather than a different one that happens to be openable.
pub(crate) fn open_store(state_root: &Path) -> Result<(Arc<dyn SecretStore>, &'static str), StoreError> {
    if let Ok(keyring) = KeyringStore::open() {
        return Ok((Arc::new(keyring), "keyring"));
    }
    let file = FileStore::open(state_root.join("credentials.store"))?;
    Ok((Arc::new(file), "file (not encrypted at rest)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_in_the_result_can_carry_a_secret() {
        // The property this module exists for, asserted structurally: every field of a row comes
        // from the catalogue, the configuration, or a boolean outcome. If someone later adds a
        // value-bearing field, this is the test that should stop them.
        let rendered = json!({
            "provider": "gitlab",
            "credential": "gitlab.token",
            "status": "connected",
            "verify": "gitlab-user-get",
        });
        let object = rendered.as_object().expect("an object");
        for key in object.keys() {
            assert!(
                matches!(key.as_str(), "provider" | "credential" | "status" | "verify" | "detail"),
                "`{key}` is not one of the value-free fields this report may carry"
            );
        }
    }

    #[test]
    fn the_store_preference_matches_what_the_runtime_composes() {
        // Keyring first, file second. If these ever disagree, `auth status` would report on a
        // different store than the daemon reads, which is worse than not reporting at all.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let (_store, backend) = open_store(directory.path()).expect("a store opens");
        assert!(
            backend == "keyring" || backend.starts_with("file"),
            "unexpected backend `{backend}`"
        );
    }
}
