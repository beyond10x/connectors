//! **The OS keyring**, so a credential on a workstation is sealed by the desktop session rather
//! than by file permissions alone.
//!
//! # What this replaces
//!
//! [`FileStore`](crate::FileStore) protects a value with Unix ownership and mode `0600`. That is a
//! real guarantee against another user on the machine and **no guarantee at all** against a copied
//! backup, a synced home directory, or anything running as you. The freedesktop Secret Service
//! keeps the value in a collection the session unlocks, encrypted at rest by the keyring daemon.
//!
//! # Why a subprocess, and not the `keyring` crate
//!
//! Measured before choosing, because the plan required it. `keyring` v4 with the zbus Secret
//! Service backend resolves to 97 packages, of which **48 are not already in this component's
//! lockfile** — and they include `async-io`, `async-executor`, `async-task`, `blocking`, `polling`
//! and `futures-lite`: the whole smol executor stack, linked alongside the Tokio this component
//! already runs. A second async runtime inside the binary, and a 17% enlargement of a root
//! workspace member's dependency graph, to make a DBus call.
//!
//! `secret-tool` is libsecret's own command-line interface, it is what every other desktop consumer
//! of this keyring uses, and it adds **no Rust dependency at all**. The cost is a process spawn per
//! operation, which is paid when a Connection is opened rather than per request.
//!
//! **The value never touches `argv`.** `secret-tool store` reads the secret from stdin, precisely
//! so a credential does not appear in `ps` output or a shell history. `lookup` returns it on stdout,
//! which is a pipe between two processes the same user owns.
//!
//! # What this store deliberately does not do
//!
//! [`SecretStore::references`] stays unimplemented — the trait's `Unsupported` refusal rather than
//! a wrong answer. The only enumeration `secret-tool` offers is `search --all`, and its output
//! includes a `secret = …` line for every match: listing the addresses in a scope would mean
//! reading every value in that scope into this process's memory, which is the exact opposite of
//! what the method promises. A value-free index would have to live somewhere else, and inventing a
//! second source of truth for what is in the keyring is worse than answering "not supported".

use std::io::Write as _;
use std::process::{Command, Stdio};

use connector_address::CredentialRef;
use zeroize::Zeroizing;

use crate::{Secret, SecretStore, StoreError};

/// The collection attribute every entry this component owns carries.
///
/// One well-known value so an operator can find, audit and revoke everything B10x stored
/// with a single `secret-tool search service b10x-connectors` — and so nothing this component
/// writes can collide with another application's entry.
const SERVICE_ATTRIBUTE: &str = "b10x-connectors";

/// The attribute name used when a credential address carries no instance.
///
/// A literal, not an omitted attribute: `secret-tool lookup` matches on the exact attribute set it
/// is given, so an entry stored without the attribute and one stored with it are different keys.
/// Writing `-` keeps every entry's attribute set identical in shape.
const NO_INSTANCE: &str = "-";

/// Credentials in the freedesktop Secret Service, through `secret-tool`.
#[derive(Debug, Clone)]
pub struct KeyringStore {
    tool: std::path::PathBuf,
}

impl KeyringStore {
    /// Locate `secret-tool` and confirm the keyring answers.
    ///
    /// # Errors
    ///
    /// [`StoreError::Unreachable`] when `secret-tool` is absent or the Secret Service does not
    /// respond. Deliberately *not* `NotFound`: "there is no keyring here" and "this credential was
    /// never stored" are different facts, and a deployment that confuses them reports an unlocked
    /// keyring as an unconfigured integration.
    pub fn open() -> Result<Self, StoreError> {
        let tool = which_secret_tool().ok_or(unreachable("the Secret Service did not answer"))?;
        Ok(Self { tool })
    }

    /// The attribute pairs addressing one credential.
    ///
    /// The address is spread across attributes rather than flattened into one string so an operator
    /// reading `secret-tool search` sees which tenant, which provider authority and which credential
    /// an entry belongs to, instead of an opaque path.
    fn attributes(reference: &CredentialRef) -> Vec<String> {
        vec![
            "service".to_owned(),
            SERVICE_ATTRIBUTE.to_owned(),
            "tenant".to_owned(),
            reference.tenant().to_owned(),
            "authority".to_owned(),
            reference.authority().to_owned(),
            "instance".to_owned(),
            reference
                .instance()
                .map_or_else(|| NO_INSTANCE.to_owned(), |id| id.as_str().to_owned()),
            "surface".to_owned(),
            reference.service().to_owned(),
            "credential".to_owned(),
            reference.credential().to_owned(),
        ]
    }

    fn run(&self, verb: &str, reference: &CredentialRef, label: Option<&str>) -> Command {
        let mut command = Command::new(&self.tool);
        command.arg(verb);
        if let Some(label) = label {
            command.arg(format!("--label={label}"));
        }
        command.args(Self::attributes(reference));
        command
    }
}

#[async_trait::async_trait]
impl SecretStore for KeyringStore {
    async fn ready(&self) -> Result<(), StoreError> {
        // A lookup of an address that is not there: it exercises the DBus round trip and the
        // collection, and a missing entry is the expected answer rather than a failure.
        let probe = CredentialRef::new(
            "readiness",
            "test.b10x.readiness",
            connector_address::DEFAULT_SERVICE,
            "probe",
        )
        .map_err(|_| unreachable("the Secret Service did not answer"))?;
        let output = self
            .run("lookup", &probe, None)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|_| unreachable("the Secret Service did not answer"))?;
        // `secret-tool lookup` exits non-zero for a miss, which is fine. What is not fine is being
        // unable to reach the service at all — that prints to stderr and is what this distinguishes.
        if output.status.success() || output.stderr.is_empty() {
            Ok(())
        } else {
            Err(unreachable("the Secret Service did not answer"))
        }
    }

    async fn get(&self, reference: &CredentialRef) -> Result<Secret, StoreError> {
        let output = self
            .run("lookup", reference, None)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|_| unreachable("the Secret Service did not answer"))?;
        if !output.status.success() || output.stdout.is_empty() {
            // A miss and an unreachable service are told apart by stderr: `secret-tool` is silent
            // on a miss and complains when it cannot reach the collection.
            if output.stderr.is_empty() {
                return Err(StoreError::NotFound {
                    path: reference.tenant().to_owned(),
                });
            }
            return Err(unreachable("the Secret Service did not answer"));
        }
        // No trimming. `secret-tool` writes the stored bytes and nothing else, and a credential
        // that legitimately ends in whitespace must survive a round trip unchanged.
        let value = Zeroizing::new(
            String::from_utf8(output.stdout)
                .map_err(|_| unreachable("the Secret Service did not answer"))?,
        );
        Ok(Secret::new(value.as_str()))
    }

    async fn put(&self, reference: &CredentialRef, secret: &Secret) -> Result<(), StoreError> {
        let label = format!(
            "B10x {} {}",
            reference.authority(),
            reference.credential()
        );
        let mut child = self
            .run("store", reference, Some(&label))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| unreachable("the Secret Service did not answer"))?;
        {
            // Scoped so the pipe closes before the wait: `secret-tool` reads until EOF, and holding
            // the handle open would deadlock both processes.
            let mut stdin = child
                .stdin
                .take()
                .ok_or(unreachable("the Secret Service did not answer"))?;
            stdin
                .write_all(secret.expose_secret().as_bytes())
                .map_err(|_| unreachable("the Secret Service did not answer"))?;
        }
        let status = child
            .wait()
            .map_err(|_| unreachable("the Secret Service did not answer"))?;
        if status.success() {
            Ok(())
        } else {
            Err(unreachable("the Secret Service did not answer"))
        }
    }

    async fn delete(&self, reference: &CredentialRef) -> Result<(), StoreError> {
        let output = self
            .run("clear", reference, None)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .map_err(|_| unreachable("the Secret Service did not answer"))?;
        // **`secret-tool clear` exits non-zero when nothing matched**, which is not a failure here:
        // the trait's contract is that deleting an absent credential succeeds, so a caller cleaning
        // up does not have to know whether it got there first. Measured, not assumed — a miss exits
        // `1` with an empty stderr, and a real fault writes to stderr. That is the same rule `get`
        // uses to tell a miss from an unreachable service, and it is the only signal `secret-tool`
        // gives.
        if output.status.success() || output.stderr.is_empty() {
            Ok(())
        } else {
            Err(unreachable("the Secret Service did not answer"))
        }
    }
}

/// Every failure that is not a miss. The path is the store's identity rather than a credential
/// address: naming the address in an error would put a tenant and a provider into a log line that
/// a miss never needed.
fn unreachable(reason: &str) -> StoreError {
    StoreError::Unreachable {
        path: "keyring".to_owned(),
        reason: reason.to_owned(),
    }
}

fn which_secret_tool() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join("secret-tool"))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(credential: &str) -> CredentialRef {
        CredentialRef::new(
            "keyring-test",
            "test.b10x.keyring",
            connector_address::DEFAULT_SERVICE,
            credential,
        )
        .expect("a valid address")
    }

    #[test]
    fn every_entry_carries_the_component_wide_service_attribute() {
        // The property that makes `secret-tool search service b10x-connectors` a complete
        // audit of what this component stored, and that stops a collision with another
        // application's entry.
        let attributes = KeyringStore::attributes(&reference("api_token"));
        assert_eq!(attributes[0], "service");
        assert_eq!(attributes[1], SERVICE_ATTRIBUTE);
        assert!(attributes.contains(&"authority".to_owned()));
        assert!(attributes.contains(&"test.b10x.keyring".to_owned()));
    }

    #[test]
    fn an_address_without_an_instance_still_carries_the_attribute() {
        // `secret-tool` matches on the exact attribute set given, so omitting the attribute for
        // single-instance addresses would make them a different key from multi-instance ones and
        // every lookup would miss.
        let attributes = KeyringStore::attributes(&reference("api_token"));
        let position = attributes
            .iter()
            .position(|item| item == "instance")
            .expect("the instance attribute is always present");
        assert_eq!(attributes[position + 1], NO_INSTANCE);
    }

    #[test]
    fn addresses_differing_only_in_credential_are_different_keys() {
        assert_ne!(
            KeyringStore::attributes(&reference("api_token")),
            KeyringStore::attributes(&reference("refresh_token"))
        );
    }

    /// A real round trip through this machine's keyring.
    ///
    /// `#[ignore]`d for the same reason `vault_live.rs` is: CI has no desktop session, and a test
    /// that silently passed because no Secret Service was present would assert nothing. Run it with
    /// `cargo test -p connector-secrets -- --ignored` on a workstation.
    #[tokio::test]
    #[ignore = "requires a running freedesktop Secret Service and secret-tool"]
    async fn a_credential_round_trips_through_the_real_keyring() {
        let store = KeyringStore::open().expect("secret-tool is installed");
        store.ready().await.expect("the keyring answers");
        let reference = reference("round_trip");
        let _ = store.delete(&reference).await;

        assert!(
            matches!(
                store.get(&reference).await,
                Err(StoreError::NotFound { .. })
            ),
            "an unstored credential is NotFound, never Unreachable: the difference is what stops \
             a locked keyring reading as an unconfigured integration"
        );

        // A value with trailing whitespace on purpose: `secret-tool` must round-trip the bytes, and
        // a store that trimmed would silently corrupt a credential that ends in one.
        let value = "SENTINEL-NOT-A-REAL-SECRET \n";
        store
            .put(&reference, &Secret::new(value))
            .await
            .expect("store");
        assert_eq!(
            store
                .get(&reference)
                .await
                .expect("read back")
                .expose_secret(),
            value
        );

        store.delete(&reference).await.expect("delete");
        assert!(matches!(
            store.get(&reference).await,
            Err(StoreError::NotFound { .. })
        ));
        store
            .delete(&reference)
            .await
            .expect("delete is idempotent");
    }
}
