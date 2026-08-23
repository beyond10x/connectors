use std::fs::OpenOptions;
use std::io::Read as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use connectors_config::PlatformIntegrationConfig;
use ed25519_dalek::pkcs8::DecodePrivateKey as _;
use ed25519_dalek::{Signer as _, SigningKey};
use protocol::operation::{OperationError, OperationErrorCode};
use serde::Serialize;
use service::PrincipalContext;
use sha2::{Digest as _, Sha256};

use crate::surface::{
    MODULE_AUTHORIZATION_SCHEME, MODULE_REQUEST_TTL_SECONDS, MODULE_REQUEST_TYPE,
};
use crate::PlatformIntegrationError;

pub(super) struct ModuleSigner {
    issuer: String,
    kid: String,
    key: SigningKey,
}

#[derive(Serialize)]
struct ModuleProtectedHeader<'a> {
    alg: &'static str,
    kid: &'a str,
    typ: &'static str,
}

#[derive(Serialize)]
struct ModuleRequestClaims<'a> {
    iss: &'a str,
    aud: &'a str,
    tenant_id: &'a str,
    sub: &'a str,
    act: &'a str,
    operation: &'a str,
    method: &'a str,
    target: &'a str,
    body_sha256: String,
    idempotency_key_sha256: Option<String>,
    authority_snapshot_id: &'a str,
    authority_snapshot_sha256: &'a str,
    request_id: String,
    trace_id: String,
    grants: [&'a str; 1],
    iat: u64,
    nbf: u64,
    exp: u64,
    jti: String,
}

impl ModuleSigner {
    pub(super) fn load(
        config: &PlatformIntegrationConfig,
    ) -> Result<Option<Self>, PlatformIntegrationError> {
        if config.work_origin.is_none()
            && config.ontology_origin.is_none()
            && config.planner_origin.is_none()
            && config.workspaces_origin.is_none()
            && config.colab_origin.is_none()
        {
            return Ok(None);
        }
        let encoded = read_owner_key(
            config
                .module_signing_key_file
                .as_deref()
                .ok_or(PlatformIntegrationError::InvalidConfiguration)?,
        )
        .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?;
        let key = if encoded.starts_with("-----BEGIN PRIVATE KEY-----") {
            SigningKey::from_pkcs8_pem(&encoded)
                .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?
        } else {
            let decoded = URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?;
            let bytes: [u8; 32] = decoded
                .try_into()
                .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?;
            SigningKey::from_bytes(&bytes)
        };
        Ok(Some(Self {
            issuer: config
                .module_signing_issuer
                .clone()
                .ok_or(PlatformIntegrationError::InvalidConfiguration)?,
            kid: config
                .module_signing_key_id
                .clone()
                .ok_or(PlatformIntegrationError::InvalidConfiguration)?,
            key,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn authorization(
        &self,
        context: &PrincipalContext,
        audience: &str,
        operation: &str,
        method: &str,
        target: &str,
        body: &[u8],
        idempotency_key: Option<&str>,
    ) -> Result<String, OperationError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| unavailable())?
            .as_secs();
        let protected = ModuleProtectedHeader {
            alg: "EdDSA",
            kid: &self.kid,
            typ: MODULE_REQUEST_TYPE,
        };
        let request_id = context
            .request_id()
            .map(str::to_owned)
            .map_or_else(|| opaque_ref("request"), Ok)?;
        let trace_id = context
            .trace_id()
            .map(str::to_owned)
            .map_or_else(|| opaque_ref("trace"), Ok)?;
        let claims = ModuleRequestClaims {
            iss: &self.issuer,
            aud: audience,
            tenant_id: context.tenant_id(),
            sub: context.subject(),
            act: context.actor_subject(),
            operation,
            method,
            target,
            body_sha256: format!("{:x}", Sha256::digest(body)),
            idempotency_key_sha256: idempotency_key
                .map(|value| format!("{:x}", Sha256::digest(value.as_bytes()))),
            authority_snapshot_id: context.authority_snapshot_id(),
            authority_snapshot_sha256: context.authority_snapshot_sha256(),
            request_id,
            trace_id,
            grants: [operation],
            iat: now,
            nbf: now,
            exp: now.saturating_add(MODULE_REQUEST_TTL_SECONDS),
            jti: opaque_ref("module-request")?,
        };
        let protected =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&protected).map_err(|_| unavailable())?);
        let claims =
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(|_| unavailable())?);
        let signing_input = format!("{protected}.{claims}");
        let signature = self.key.sign(signing_input.as_bytes());
        Ok(format!(
            "{MODULE_AUTHORIZATION_SCHEME}{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }
}

fn read_owner_key(path: &Path) -> Result<String, OperationError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| not_granted())?;
    let metadata = file.metadata().map_err(|_| not_granted())?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > 4096
    {
        return Err(not_granted());
    }
    let mut value = String::new();
    (&mut file)
        .take(4097)
        .read_to_string(&mut value)
        .map_err(|_| not_granted())?;
    let value = value.trim_end_matches(['\r', '\n']);
    if value.len() < 32
        || value.len() > 4096
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\r' | '\n'))
    {
        return Err(not_granted());
    }
    Ok(value.to_owned())
}

fn opaque_ref(prefix: &str) -> Result<String, OperationError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
    Ok(format!("{prefix}-{}", hex::encode(bytes)))
}

fn unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "connector runtime is unavailable",
        true,
    )
}

fn not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Connection",
        false,
    )
}
