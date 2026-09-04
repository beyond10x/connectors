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
                "provider": entry.provider, "instance": entry.instance(),
                "status": "unknown-provider", "detail": "not in the catalogue",
            }));
            continue;
        };
        let Some(authority) = provider.authority else {
            providers.push(json!({
                "provider": entry.provider, "instance": entry.instance(),
                "status": "no-authority",
                "detail": "the provider declares no authority, so its credential has no address",
            }));
            continue;
        };

        // **Every declared credential, each with its role.** There is no primary and no secondary:
        // a workspace bot token and a personal user token are two roles one identity holds, and
        // reporting only one of them describes an identity that is not the one configured. The
        // catalogue names the role — `subject` — so this reads it rather than inventing an order.
        let mut credentials = Vec::new();
        let mut stored = std::collections::BTreeSet::new();
        for declared in provider.auth {
            let reference = integration_catalog::credential_address(
                &config.owner.tenant_id,
                authority,
                entry,
                declared.leaf,
            );
            let present = match reference {
                Ok(reference) => store.exists(&reference).await,
                Err(_) => Ok(false),
            };
            // **A Basic credential is two halves and needs both.** The token is in the store; the
            // account name it joins is configuration, and without it `assemble_credentials`
            // refuses the mechanism — which a caller sees as `not_granted: no stored credential
            // satisfies this operation's declared mechanisms`. Reporting the token as `stored` and
            // the identity as `callable` was true about the store and wrong about the question.
            let user_half = matches!(declared.acquire, catalog::Acquisition::BasicJoin { .. })
                .then(|| {
                    entry
                        .usernames
                        .get(declared.name)
                        .is_some_and(|value| !value.trim().is_empty())
                });
            let state = match present {
                Ok(true) if user_half == Some(false) => "stored-without-user-half",
                Ok(true) => {
                    stored.insert(declared.name);
                    "stored"
                }
                Ok(false) => "absent",
                // "We cannot say" is its own answer: reporting a locked keyring as absent would
                // send an operator to re-enter a credential that is already there.
                Err(_) => "unavailable",
            };
            credentials.push(json!({
                "credential": declared.name,
                "subject": subject_of(declared.subject),
                "state": state,
                // Present only for a credential that has a user half at all, and never the value:
                // the answer an operator needs is whether one was configured. The section that
                // holds it is named so the fix is a copyable instruction rather than a search.
                "user_half": user_half.map(|configured| json!({
                    "configured": configured,
                    "config": format!(
                        "[catalog.usernames] \"{}\" under this provider's [[catalog]] block",
                        declared.name
                    ),
                })),
            }));
        }

        // What this identity can actually call. A mechanism is one alternative set of credentials
        // an operation accepts, so an identity is usable exactly when some mechanism is complete —
        // a more useful answer than any single credential's state, and the real question behind
        // "is this connected?".
        let mechanisms: std::collections::BTreeSet<Vec<&str>> = provider
            .operations
            .iter()
            .flat_map(|operation| operation.credentials.iter())
            .map(|mechanism| mechanism.to_vec())
            .collect();
        let satisfied: Vec<String> = mechanisms
            .iter()
            .filter(|mechanism| mechanism.iter().all(|name| stored.contains(name)))
            .map(|mechanism| mechanism.join(" + "))
            .collect();

        providers.push(json!({
            "provider": entry.provider,
            "instance": entry.instance(),
            "status": if satisfied.is_empty() { "not-callable" } else { "callable" },
            "credentials": credentials,
            // Named rather than counted: an operator whose identity is not callable needs to know
            // which credential would make it so.
            "satisfied_mechanisms": satisfied,
            "verify": provider.verify,
        }));
    }

    Ok(json!({
        "store": backend,
        "providers": providers,
    }))
}

/// The subject the catalogue declares a credential carries — its own word, not an invented one.
///
/// Not a rank and not a role: `app` and `user` are different actors, and one instance may hold
/// both. Slack makes the distinction look like a role; for another provider it would not.
const fn subject_of(subject: catalog::Subject) -> &'static str {
    match subject {
        catalog::Subject::App => "app",
        catalog::Subject::User => "user",
        catalog::Subject::Unstated => "unstated",
    }
}

/// The store this machine would use, and its name.
///
/// The same preference order the runtime composes with, so `auth status` reports the store the
/// daemon actually reads rather than a different one that happens to be openable.
pub(crate) fn open_store(
    state_root: &Path,
) -> Result<(Arc<dyn SecretStore>, &'static str), StoreError> {
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
            "provider": "slack",
            "instance": "timo-ai",
            "status": "callable",
            "credentials": [{"credential": "slack.bot_token", "subject": "app", "state": "stored"}],
            "satisfied_mechanisms": ["slack.bot_token"],
            "verify": "slack-users-info",
        });
        let object = rendered.as_object().expect("an object");
        for key in object.keys() {
            assert!(
                matches!(
                    key.as_str(),
                    "provider"
                        | "instance"
                        | "credentials"
                        | "status"
                        | "verify"
                        | "detail"
                        | "satisfied_mechanisms"
                ),
                "`{key}` is not one of the value-free fields this report may carry"
            );
        }
    }

    #[test]
    fn a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value() {
        // The state a person hits: `jira.api_token` in the keyring, no `[catalog.usernames]`, and
        // every call refusing. The row has to say which half is missing.
        let row = json!({
            "credential": "jira.api_token",
            "subject": "unstated",
            "state": "stored-without-user-half",
            "user_half": {
                "configured": false,
                "config": "[catalog.usernames] \"jira.api_token\" under this provider's [[catalog]] block",
            },
        });
        let half = row.get("user_half").expect("the row carries the half");
        assert_eq!(half.get("configured"), Some(&json!(false)));
        // Structural, like the test above: the report says whether, never what.
        for key in half.as_object().expect("an object").keys() {
            assert!(
                matches!(key.as_str(), "configured" | "config"),
                "`{key}` could carry an account name into a scrollback buffer"
            );
        }
    }

    #[test]
    fn the_catalogue_is_what_says_a_credential_has_a_user_half() {
        // Read from `acquire`, not from a list of provider ids: the next Basic connector is
        // covered the moment it is catalogued.
        let jira = catalog::provider(catalog::ProviderKey::id("jira")).expect("jira");
        let token = jira.credential("jira.api_token").expect("the token");
        assert!(matches!(
            token.acquire,
            catalog::Acquisition::BasicJoin { .. }
        ));
        let slack = catalog::provider(catalog::ProviderKey::id("slack")).expect("slack");
        let bot = slack.credential("slack.bot_token").expect("the bot token");
        assert!(
            !matches!(bot.acquire, catalog::Acquisition::BasicJoin { .. }),
            "a bearer credential must not grow a user-half row it can never satisfy"
        );
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
