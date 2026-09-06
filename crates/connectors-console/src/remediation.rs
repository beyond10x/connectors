//! Trusted personal-local remediation: private human handoff, then a closed readiness result.

use std::io::Read as _;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _};
use std::path::{Path, PathBuf};

use connectors_client::LocalClient;
use connectors_config::PersonalConfig;
use protocol::connection_v2::RemediationStartRequest;
use serde_json::{json, Value};

/// Closed public failure classes. Daemon strings, inputs and private endpoints are never copied.
#[derive(Debug, thiserror::Error)]
pub enum RemediationError {
    #[error("authentication remediation requires a persistent personal daemon; run `connectors serve local` with the same configuration and state root")]
    DaemonRequired,
    #[error(
        "authentication remediation requires exactly one configured and admitted OAuth binding"
    )]
    Configuration,
    #[error("authentication remediation requires a controlling terminal or a new owner-only instruction file in a private directory")]
    PrivateInstructions,
    #[error("authentication remediation was refused, expired, or returned invalid evidence")]
    Refused,
    #[error("remediation input requires one bounded JSON source: --input-json, --input-file, or --input -")]
    Input,
}
impl RemediationError {
    /// Stable failure class without caller/daemon data.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DaemonRequired => "daemon-required",
            Self::Configuration => "configuration",
            Self::PrivateInstructions => "private-instructions-required",
            Self::Refused => "authentication-remediation-refused",
            Self::Input => "invalid-argument",
        }
    }
}

/// Read one existing input-source spelling with the bound required by Connection v2.
/// This intentionally leaves the older generic input reader's contract unchanged.
pub fn read_input(
    inline: Option<String>,
    file: Option<PathBuf>,
    stdin: Option<String>,
) -> Result<Value, RemediationError> {
    let maximum = protocol::connection_v2::MAX_INPUT_BYTES;
    let mut bytes = Vec::new();
    match (inline, file, stdin) {
        (Some(text), None, None) => {
            if text.len() > maximum {
                return Err(RemediationError::Input);
            }
            bytes = text.into_bytes();
        }
        (None, Some(path), None) => {
            std::fs::File::open(path)
                .map_err(|_| RemediationError::Input)?
                .take((maximum + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| RemediationError::Input)?;
        }
        (None, None, Some(marker)) if marker == "-" => {
            std::io::stdin()
                .lock()
                .take((maximum + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| RemediationError::Input)?;
        }
        _ => return Err(RemediationError::Input),
    }
    if bytes.len() > maximum {
        return Err(RemediationError::Input);
    }
    serde_json::from_slice(&bytes).map_err(|_| RemediationError::Input)
}

/// Personal-local bound acquisition only. Hosted acquisition is Unsupported; this function neither
/// issues hosted credentials nor invents a combined Identity scope.
/// The returned public value is reference-free and never contains trusted transport metadata.
pub async fn run(
    config: &PersonalConfig,
    state_root: &Path,
    request: RemediationStartRequest,
    instruction_file: Option<&Path>,
) -> Result<Value, RemediationError> {
    require_daemon(state_root)?;
    let principal = config
        .principal_context()
        .map_err(|_| RemediationError::Configuration)?;
    let matches: Vec<_> = config
        .catalog
        .iter()
        .filter(|entry| entry.oauth.is_some())
        .filter(|entry| {
            integration_catalog::personal_oauth_admitted_connection_ref(&principal, entry)
                .is_ok_and(|reference| reference == request.connection_ref)
        })
        .collect();
    let [entry] = matches.as_slice() else {
        return Err(RemediationError::Configuration);
    };
    let registration = entry
        .oauth
        .as_ref()
        .ok_or(RemediationError::Configuration)?;
    let origins = integration_catalog::personal_oauth_admitted_origins(&principal, entry)
        .map_err(|_| RemediationError::Configuration)?;
    let [origin] = origins.as_slice() else {
        return Err(RemediationError::Configuration);
    };
    let mut destination = crate::connect::PrivateInstructionDestination::open(instruction_file)
        .map_err(|_| RemediationError::PrivateInstructions)?;
    let client = LocalClient::new(state_root.join("connectors.sock"));
    let owner = config.owner_context();
    let pending = client
        .begin_remediation(&owner, request, &entry.provider, &registration.auth_profile)
        .await
        .map_err(|_| RemediationError::Refused)?;
    let deadline = pending.instruction_deadline();
    let result = tokio::time::timeout_at(deadline, async {
        if let Some(instructions) = client
            .remediation_instructions(&pending, origin)
            .await
            .map_err(|_| RemediationError::Refused)?
        {
            instructions
                .write_human(&mut destination.file)
                .map_err(|_| RemediationError::PrivateInstructions)?;
        }
        client
            .finish_remediation(&owner, pending)
            .await
            .map_err(|_| RemediationError::Refused)
    })
    .await
    .map_err(|_| RemediationError::Refused)?;
    destination
        .clear()
        .map_err(|_| RemediationError::PrivateInstructions)?;
    result?;
    Ok(json!({"ready":true,"attempt":"not_attempted","next_action":"explicit_invoke"}))
}

/// Refuse unsafe or absent daemon sockets before acquiring caller input or private instructions.
/// The presenter repeats this check; it does not reserve the path against later replacement.
pub fn require_daemon(root: &Path) -> Result<(), RemediationError> {
    let owner = rustix::process::geteuid().as_raw();
    let metadata = std::fs::symlink_metadata(root).map_err(|_| RemediationError::DaemonRequired)?;
    if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o077 != 0 {
        return Err(RemediationError::Configuration);
    }
    let metadata = std::fs::symlink_metadata(root.join("connectors.sock"))
        .map_err(|_| RemediationError::DaemonRequired)?;
    if !metadata.file_type().is_socket() || metadata.uid() != owner || metadata.mode() & 0o077 != 0
    {
        return Err(RemediationError::Configuration);
    }
    Ok(())
}
