//! Executable composition: native configuration and protected entry terminate
//! here. The business adapter receives only an immutable password capability.
//!
//! This is the third execution family in the repository. GitLab and Kubernetes
//! reach their provider over HTTP and share `ScopedHttp`; PostgreSQL speaks its
//! own wire protocol, so there is no HTTP port, no probe capability and no
//! bearer header — the credential establishes a session instead of signing each
//! request, which is why the profile selects `session_authority`.
use connectors_host::local::{
    filesystem, registry,
    runtime::{Baseline, Bootstrap, Effect, EntryField, Failure, Profile, Requirement, Result},
};
use connectors_sdk::{Adapter as _, Credential, Secret};
use connectors_sql::{Config, Sql, auth};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    format: String,
    instance: String,
    host: String,
    port: u16,
    database: String,
    user: String,
    #[serde(default)]
    allow_plaintext: bool,
    ca_file: Option<PathBuf>,
}

pub struct Local {
    bootstrap: Bootstrap,
    config: Config,
    instance: String,
    effective: Value,
}
impl Local {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = filesystem::read_bounded(
            filesystem::private_file(path).map_err(|_| Failure::InvalidConfiguration)?,
            1024 * 1024,
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        let config: Configuration =
            connectors_core::read_json(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        if config.format != "connectors-sql-local/1"
            || !connectors_core::valid_id(&config.instance)
            || config.host.is_empty()
            || config.host.len() > 512
            // A leading slash selects a Unix socket directory, which carries no
            // TLS and no host authority this binding can record.
            || config.host.starts_with('/')
            || config.port == 0
            || config.database.is_empty()
            || config.database.len() > 512
            || config.user.is_empty()
            || config.user.len() > 512
        {
            return Err(Failure::InvalidConfiguration);
        }
        let ca = config
            .ca_file
            .as_ref()
            .map(|path| filesystem::read_bounded(filesystem::private_file(path)?, 1024 * 1024))
            .transpose()
            .map_err(|_| Failure::InvalidConfiguration)?;
        let effective = json!({"format":config.format,"instance":config.instance,
            "host":config.host,"port":config.port,"database":config.database,"user":config.user,
            "allow_plaintext":config.allow_plaintext,
            "ca_digest":ca.as_ref().map(|b|connectors_core::digest(&json!(b)))});
        let configuration_revision = connectors_core::digest(&effective);
        // The provider authority is the exact database endpoint, so a changed
        // host, port or database is a changed binding rather than a new session.
        let provider_authority = format!(
            "postgresql://{}:{}/{}",
            config.host, config.port, config.database
        );
        let native = Config {
            host: config.host,
            port: config.port,
            database: config.database,
            user: config.user,
            allow_plaintext: config.allow_plaintext,
            ca_file: config.ca_file,
        };
        // This template has no credential and opens no session.
        let adapter = Sql::new(
            &config.instance,
            native.clone(),
            effective.clone(),
            Arc::new(Absent),
        )
        .map_err(Failure::from_service)?;
        // A password establishes a session; it does not sign a request and
        // carries no scope grant, so the profile declares no required scopes.
        let mut profile = Profile {
            id: auth::PROFILE_ID.into(),
            revision: String::new(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "session_authority".into(),
            capability: "session-authority".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: auth::EVIDENCE_LIFETIME_MS,
            fields: vec![EntryField {
                name: "password".into(),
                label: "PostgreSQL password".into(),
                max_bytes: 8192,
            }],
        };
        profile.revision = connectors_core::digest(
            &serde_json::to_value(&profile).map_err(|_| Failure::Protocol)?,
        );
        let descriptor = adapter.descriptor();
        let bootstrap = Bootstrap {
            instance: config.instance.clone(),
            adapter: "sql".into(),
            protocol: connectors_core::WIRE_VERSION.into(),
            configuration_revision,
            provider_authority,
            descriptor: serde_json::to_string(&descriptor).map_err(|_| Failure::Protocol)?,
            profiles: vec![profile],
            requirements: descriptor
                .operations
                .iter()
                .map(|o| Requirement {
                    operation: o.id.clone(),
                    profile: auth::PROFILE_ID.into(),
                    scopes: BTreeSet::new(),
                    // Both operations read inside a read-only transaction the
                    // adapter opens itself; neither can express a write.
                    effect: match o.id.as_str() {
                        "schema.list" | "query.read" => Effect::Read,
                        _ => Effect::Unknown,
                    },
                })
                .collect(),
        };
        bootstrap.validate_for(connectors_host::local::runtime::PrivateProtocol::V1)?;
        Ok(Self {
            bootstrap,
            config: native,
            instance: config.instance,
            effective,
        })
    }
    pub fn description(&self) -> &Bootstrap {
        &self.bootstrap
    }
    /// One adapter bound to one admitted credential. No session is opened here.
    fn bound(&self, mut document: Secret) -> Result<Sql> {
        let password = auth::ProtectedEntry::parse(std::mem::take(&mut document.0))
            .map_err(native_failure)?
            .into_secret();
        Sql::new(
            &self.instance,
            self.config.clone(),
            self.effective.clone(),
            Arc::new(Fixed(password)),
        )
        .map_err(Failure::from_service)
    }
}

/// The bootstrap template holds no credential and must never resolve one.
struct Absent;
#[async_trait::async_trait]
impl Credential for Absent {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Err(connectors_core::Error::new(
            connectors_core::ErrorCode::Unauthorized,
            "the bootstrap template holds no credential",
        ))
    }
}
struct Fixed(Secret);
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.0.clone()))
    }
}

#[async_trait::async_trait]
impl connectors_host::local::runtime::Adapter for Local {
    fn bootstrap(&self) -> Bootstrap {
        self.bootstrap.clone()
    }
    async fn validate(&self, profile: &str, document: Secret) -> Result<Baseline> {
        if profile != auth::PROFILE_ID {
            return Err(Failure::Unsupported);
        }
        let collected_at_ms = connectors_sdk::now_ms();
        self.bound(document)?
            .validate_session()
            .await
            .map_err(Failure::from_provider)?;
        Ok(Baseline {
            // The role name is the principal PostgreSQL authorises against, and
            // the database scopes it, so the pair is the identity. A changed
            // host or port is a changed configuration revision, not a changed
            // identity, and is refused before this point.
            identity: registry::ExternalIdentity {
                kind: "postgresql.role".into(),
                subject: format!("{}@{}", self.config.user, self.config.database),
            },
            // A password carries no scope grant and no readable expiry.
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms,
            valid_until_ms: collected_at_ms.saturating_add(auth::EVIDENCE_LIFETIME_MS),
        })
    }
    async fn invoke(
        &self,
        operation: &str,
        _partition: &str,
        document: Secret,
        input: Value,
    ) -> Result<Value> {
        // No cursor state exists for either operation, so the connection
        // partition selects nothing and is deliberately unused.
        self.bound(document)?
            .invoke(operation, input)
            .await
            .map_err(Failure::from_provider)
    }
}
fn native_failure(value: auth::Failure) -> Failure {
    match value {
        auth::Failure::InvalidEntry => Failure::InvalidInput,
    }
}
