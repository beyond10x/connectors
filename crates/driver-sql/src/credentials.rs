//! Credential custody: references in, a self-redacting secret out.
//!
//! A connection config names **where** the password lives, never what it is. The shipped source
//! resolves file and environment references — the same shapes the hosted Kubernetes integration
//! uses for its own projected token file — and the Kubernetes-secret reference (the shape the
//! S-059 discovery descriptors emit as `secret_ref { name, namespace }`) is declared here
//! behind the same trait but refuses until the in-cluster resolver is composed. Declaring the
//! reference now and the resolver later keeps the descriptor contract stable while the custody
//! wiring lands in its own story.

use std::path::PathBuf;

use zeroize::Zeroizing;

/// Where a password lives. Every variant is a reference; none can carry a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialReference {
    /// A file the deployment mounted, in the mold of the kubelet-projected token file the
    /// hosted Kubernetes integration reads. Re-read per resolution so a rotated mount is
    /// picked up.
    File {
        /// The file's path. A path is deployment topology, not a secret.
        path: PathBuf,
    },
    /// An environment variable of the serving process. The dev-and-test shape.
    Env {
        /// The variable's name.
        variable: String,
    },
    /// A key of a Kubernetes `Secret`, as an S-059 discovery descriptor references it.
    /// Not yet resolvable: see [`FileEnvCredentialSource`].
    KubernetesSecret {
        /// The Secret's name.
        name: String,
        /// The Secret's namespace.
        namespace: String,
        /// The data key holding the password.
        key: String,
    },
}

impl CredentialReference {
    /// The reference's caller-safe description — what error messages carry.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::File { path } => format!("file {}", path.display()),
            Self::Env { variable } => format!("environment variable {variable}"),
            Self::KubernetesSecret {
                name,
                namespace,
                key,
            } => format!("kubernetes secret {namespace}/{name} key {key}"),
        }
    }
}

/// One resolved password. Redacts itself in `Debug`, zeroizes on drop, and is never `Clone`:
/// the wire modules borrow it for the lifetime of one call and nothing else holds it.
pub struct ResolvedSecret(Zeroizing<String>);

impl ResolvedSecret {
    /// Wrap a resolved value.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// The value, exposed deliberately at the one place a client library needs it.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ResolvedSecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// Named refusal from credential resolution. Carries the reference's description, never a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CredentialError {
    /// The referenced location does not exist or cannot be read.
    #[error("credential reference {reference} cannot be read: {detail}")]
    Unreadable {
        /// The reference's description.
        reference: String,
        /// The I/O or lookup failure, which names locations, never values.
        detail: String,
    },
    /// The referenced location exists but holds nothing usable.
    #[error("credential reference {reference} resolved to an empty value")]
    Empty {
        /// The reference's description.
        reference: String,
    },
    /// The reference's resolver is not composed in this deployment.
    #[error("credential reference {reference} is not resolvable here: {detail}")]
    SourceUnavailable {
        /// The reference's description.
        reference: String,
        /// What is missing.
        detail: String,
    },
}

/// The custody seam. One resolver per deployment shape; the driver takes it as a trait object
/// so composition, not this crate, decides which custody a placement carries.
pub trait CredentialSource: Send + Sync {
    /// Resolve a reference into a secret.
    ///
    /// # Errors
    ///
    /// [`CredentialError`] naming the reference — never a value.
    fn resolve(&self, reference: &CredentialReference) -> Result<ResolvedSecret, CredentialError>;
}

/// The shipped v1 source: files and environment variables resolve; the Kubernetes-secret
/// reference refuses until the in-cluster resolver is composed against the S-059 descriptors.
pub struct FileEnvCredentialSource;

impl CredentialSource for FileEnvCredentialSource {
    fn resolve(&self, reference: &CredentialReference) -> Result<ResolvedSecret, CredentialError> {
        match reference {
            CredentialReference::File { path } => {
                let raw = Zeroizing::new(std::fs::read_to_string(path).map_err(|error| {
                    CredentialError::Unreadable {
                        reference: reference.describe(),
                        detail: error.to_string(),
                    }
                })?);
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err(CredentialError::Empty {
                        reference: reference.describe(),
                    });
                }
                Ok(ResolvedSecret::new(trimmed.to_owned()))
            }
            CredentialReference::Env { variable } => {
                let value = std::env::var(variable).map_err(|error| {
                    CredentialError::Unreadable {
                        reference: reference.describe(),
                        detail: error.to_string(),
                    }
                })?;
                if value.trim().is_empty() {
                    return Err(CredentialError::Empty {
                        reference: reference.describe(),
                    });
                }
                Ok(ResolvedSecret::new(value))
            }
            CredentialReference::KubernetesSecret { .. } => {
                Err(CredentialError::SourceUnavailable {
                    reference: reference.describe(),
                    detail: "the in-cluster secret resolver is not composed yet; it lands with \
                             the S-059 discovery wiring"
                        .to_owned(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resolved_secret_redacts_itself_in_debug() {
        let secret = ResolvedSecret::new("S3CRET-SENTINEL".to_owned());
        assert_eq!(format!("{secret:?}"), "<redacted>");
    }

    #[test]
    fn a_file_reference_resolves_and_trims() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(file.path(), "the-password\n").expect("write");
        let secret = FileEnvCredentialSource
            .resolve(&CredentialReference::File {
                path: file.path().to_path_buf(),
            })
            .expect("resolves");
        assert_eq!(secret.expose(), "the-password");
    }

    #[test]
    fn a_missing_file_reports_the_path_and_no_value() {
        let error = FileEnvCredentialSource
            .resolve(&CredentialReference::File {
                path: "/nonexistent/driver-sql-credential".into(),
            })
            .expect_err("must refuse");
        let rendered = error.to_string();
        assert!(rendered.contains("/nonexistent/driver-sql-credential"), "{rendered}");
    }

    #[test]
    fn an_empty_file_is_refused_rather_than_resolved() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        std::fs::write(file.path(), "  \n").expect("write");
        let error = FileEnvCredentialSource
            .resolve(&CredentialReference::File {
                path: file.path().to_path_buf(),
            })
            .expect_err("must refuse");
        assert!(matches!(error, CredentialError::Empty { .. }));
    }

    /// The Kubernetes-secret reference is declared but deliberately unresolvable until the
    /// S-059 wiring composes the in-cluster resolver — a named refusal, not a silent fallback.
    #[test]
    fn the_kubernetes_secret_reference_refuses_by_name() {
        let error = FileEnvCredentialSource
            .resolve(&CredentialReference::KubernetesSecret {
                name: "db-conn".to_owned(),
                namespace: "latest".to_owned(),
                key: "password".to_owned(),
            })
            .expect_err("must refuse");
        assert!(matches!(error, CredentialError::SourceUnavailable { .. }));
        assert!(error.to_string().contains("S-059"), "{error}");
    }
}
