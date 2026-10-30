//! One catalog instance on the served HTTP host (docs/running-services.md,
//! "Catalog instances"): `connectors-catalog-provider --config <file>`.
//!
//! Without `service.standing_grants` the instance's description is the
//! private bootstrap's first one, which lists reads only. With it the
//! description is `bootstrap_v2()`, which lists the instance's writes too
//! (story:served-keyed-writes-under-a-standing-approval). Its credential
//! document is a file the platform mounts, read at every request under the
//! `mounted_secret` checks (docs/local-mounted-secret-custody.md). A request
//! meets these refusals, in this order, and none of them reaches the
//! provider:
//!
//! 1. Admission, by the host (`connectors_host::server`): `unauthorized` or
//!    `unavailable`, before the body is read.
//! 2. An operation the description does not list (every catalog write when no
//!    grant file is configured): the host answers `not_found` before any
//!    anchor or adapter call.
//! 3. A write: the host's standing-grant gate
//!    (crates/connectors-host/src/server/v1alpha2.rs, module docs). Without
//!    `idempotency_key` it is `invalid_input`; a key live for another request
//!    is `idempotency_conflict`; a write no grant in the file covers is
//!    `approval_required`; a grant's hour used up is `rate_limited`; a grant
//!    file that cannot be read safely is `unavailable`. Each before the anchor.
//!    On `POST /v1/invoke`, which reads no grant, a write is `unsupported`
//!    before any adapter call.
//! 4. [`Bridge::invoke`]: an operation outside the described reads and writes
//!    is `unsupported` before the credential is read. This repeats 2.
//! 5. The mounted credential: an item absent, or refused by any check, is
//!    `unavailable`, and nothing is sent.
//!
//! A read is dispatched through `invoke_explained`, the read path. A write is
//! dispatched through `runtime::Adapter::prepare_write` and executed once;
//! only an applied write with a deliverable result is a success. Any other
//! answer after dispatch is an error, which the host records as `unknown`
//! and never dispatches again.
//!
//! Start refuses a description that lists an operation of unknown effect, a
//! read under the `mutation` profile, a write under any other profile, a
//! write whose answer carries a credential it created (the served host has no
//! custody target), more than one profile, a profile whose material is
//! acquired by OAuth, alternative bindings, and a credential that cannot be
//! read now.
use crate::local::{Instance, Local};
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_host::{
    local::runtime::{self, Failure, Refusal},
    server::ServiceConfigV3,
};
use connectors_sdk::{Secret, WriteOutcome};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
#[cfg(unix)]
mod git;
/// The `git` member: the relay on Unix; elsewhere start refuses it.
#[cfg(unix)]
type GitSection = git::GitConfig;
#[cfg(not(unix))]
type GitSection = Value;

/// An `external_write` operation's profile (contracts/operations/v1alpha1/
/// semantics.md § 2); a read never carries it.
const MUTATION_PROFILE: &str = "mutation";
/// When a read's reply is due to the adapter, inside the host's 20-second
/// execution deadline: an adapter that waits before sending again never
/// waits past it.
const READ_DEADLINE: Duration = Duration::from_secs(15);

/// The served configuration: the host's, the instance's and its credential's.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub service: ServiceConfigV3,
    pub catalog: Catalog,
    pub credential: CredentialConfig,
    /// The git smart-HTTP relay (`git`), served beside the operations when
    /// present.
    #[serde(default)]
    pub git: Option<GitSection>,
}

/// Which catalog instance is served: exactly one of a shipped `connector`, a
/// `local_config` file, or a `definition` file under `definitions_root`.
/// `connector` and `definition` name the instance (`<connector>.<instance>`)
/// and may move its `endpoint` and choose its `profile`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    #[serde(default)]
    connector: Option<String>,
    #[serde(default)]
    local_config: Option<PathBuf>,
    #[serde(default)]
    definition: Option<PathBuf>,
    #[serde(default)]
    definitions_root: Option<PathBuf>,
    #[serde(default)]
    instance: Option<String>,
    #[serde(default)]
    endpoint: Option<String>,
    #[serde(default)]
    profile: Option<String>,
}

/// Where the credential document comes from. One binding exists.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialConfig {
    binding: Binding,
    /// The mounted directory; the item is `<directory>/<instance id>/<profile id>`.
    directory: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Binding {
    MountedSecret,
}

/// Read the configuration at `path`, build the instance and its bridge, and
/// serve it until the host stops.
pub async fn run(path: &Path) -> Result<()> {
    let config: Configuration = connectors_host::read_config(path)?;
    let local = config.catalog.local()?;
    let bridge = if config.service.standing_grants.is_some() {
        Bridge::serving_writes(local, &config.credential).await?
    } else {
        Bridge::new(local, &config.credential).await?
    };
    if config.service.instance != bridge.instance {
        return Err(Error::invalid(
            "service.instance must be the catalog instance's id",
        ));
    }
    serve(config, bridge).await
}

/// Elsewhere than on Unix there is no git relay, and a `git` member refuses
/// start.
#[cfg(not(unix))]
async fn serve(config: Configuration, bridge: Bridge<Local>) -> Result<()> {
    if config.git.is_some() {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "git: the relay runs on Unix only",
        ));
    }
    connectors_host::server::serve(config.service, Arc::new(bridge)).await
}

/// The operations, and the git relay beside them when `git` is set.
#[cfg(unix)]
async fn serve(config: Configuration, bridge: Bridge<Local>) -> Result<()> {
    let relay = match config.git {
        None => None,
        Some(git) => {
            // The relay's own instance: its token cache is its own, and the
            // bridge keeps the one it dispatches with.
            let upstream = Upstream {
                local: config.catalog.local()?,
                credential: Mounted {
                    directory: config.credential.directory.clone(),
                    alias: bridge.credential.alias.clone(),
                    profile: bridge.credential.profile.clone(),
                },
            };
            Some(Arc::new(git::Relay::new(git, Arc::new(upstream))?))
        }
    };
    connectors_host::server::serve_with_routes(config.service, Arc::new(bridge), |admission| {
        match relay {
            Some(relay) => git::routes(relay, admission),
            None => axum::Router::new(),
        }
    })
    .await
}

#[cfg(unix)]
/// The forge token of the instance's profile, derived from the mounted
/// document at each use; the git relay sends it and nothing else does.
pub struct Upstream {
    local: Local,
    credential: Mounted,
}

#[cfg(unix)]
impl Upstream {
    async fn token(&self) -> Result<Secret> {
        let document = self.credential.read().await?;
        self.local
            .upstream_token(document)
            .await
            .map_err(|failure| {
                error(Refusal {
                    failure,
                    reason: None,
                    retry_after_seconds: None,
                })
            })
    }
}

fn refused(what: &str, failure: Failure) -> Error {
    Error::invalid(format!("{what}: {failure:?}"))
}

impl Catalog {
    /// The configured instance, built as the private host builds it.
    pub fn local(&self) -> Result<Local> {
        match self {
            Self {
                connector: Some(connector),
                local_config: None,
                definition: None,
                definitions_root: None,
                instance: Some(name),
                endpoint,
                profile,
            } => Local::derive(&Instance {
                connector,
                name,
                endpoint: endpoint.as_deref(),
                profile: profile.as_deref(),
            })
            .map_err(|failure| refused("the connector instance cannot be configured", failure)),
            Self {
                connector: None,
                local_config: Some(path),
                definition: None,
                definitions_root: None,
                instance: None,
                endpoint: None,
                profile: None,
            } => Local::load(path)
                .map_err(|failure| refused("the local configuration cannot be loaded", failure)),
            Self {
                connector: None,
                local_config: None,
                definition: Some(definition),
                definitions_root: Some(root),
                instance: Some(name),
                endpoint,
                profile,
            } => lowered(
                definition,
                root,
                name,
                endpoint.as_deref(),
                profile.as_deref(),
            ),
            _ => Err(Error::invalid(
                "catalog names exactly one of connector (with instance), local_config, or \
                 definition and definitions_root (with instance)",
            )),
        }
    }
}

/// Lower `definition` into a private scratch directory, configure the
/// instance from what it wrote, and remove the directory: the instance holds
/// everything it read.
fn lowered(
    definition: &Path,
    root: &Path,
    name: &str,
    endpoint: Option<&str>,
    profile: Option<&str>,
) -> Result<Local> {
    let scratch = tempfile::Builder::new()
        .prefix("connectors-served-")
        .tempdir()
        .map_err(|_| Error::invalid("the definition has no private scratch directory"))?;
    let work = scratch.path().join("work");
    std::fs::create_dir(&work)
        .map_err(|_| Error::invalid("the definition has no private scratch directory"))?;
    let out = scratch.path().join("out");
    let lowered = connectors_catalog::definition::lower(&connectors_catalog::definition::Request {
        definition,
        root,
        out: &out,
        scratch: &work,
    })
    .map_err(|refusal| Error::invalid(format!("the definition does not lower: {refusal}")))?;
    Local::derive_lowered(&lowered, &out, name, endpoint, profile)
        .map_err(|failure| refused("the definition instance cannot be configured", failure))
}

/// The mounted credential item of the served instance.
pub struct Mounted {
    directory: PathBuf,
    alias: String,
    profile: String,
}

impl Mounted {
    /// The item's bytes now, under checks 1 to 7; any refusal, an absent item
    /// included, is `unavailable` and names neither path nor bytes.
    async fn read(&self) -> Result<Secret> {
        let unavailable = || {
            Error::new(
                ErrorCode::Unavailable,
                "the mounted credential is not available",
            )
        };
        #[cfg(target_os = "linux")]
        {
            let item = connectors_custody::mounted::Item::new(&self.alias, &self.profile)
                .map_err(|_| unavailable())?;
            let directory = self.directory.clone();
            let material = tokio::task::spawn_blocking(move || {
                connectors_custody::mounted::read_current(&directory, &item)
            })
            .await
            .map_err(|_| unavailable())?
            .map_err(|_| unavailable())?;
            Ok(Secret(material.into_vec()))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (&self.directory, &self.alias, &self.profile);
            Err(unavailable())
        }
    }
}

/// How the served host dispatches one described operation.
enum Dispatch {
    Read,
    Write,
}

/// How the served host dispatches an operation of `effect` under `profile`,
/// or `None` when start must refuse it. A read is never under the `mutation`
/// profile; a write is only under it, so the host's standing-grant gate
/// applies to it, and only when `writes` is set (a grant file is configured)
/// and its answer carries no created credential.
fn dispatched(
    effect: runtime::Effect,
    profile: &str,
    custody: bool,
    writes: bool,
) -> Option<Dispatch> {
    match effect {
        runtime::Effect::Read if profile != MUTATION_PROFILE && !custody => Some(Dispatch::Read),
        runtime::Effect::Write if writes && profile == MUTATION_PROFILE && !custody => {
            Some(Dispatch::Write)
        }
        _ => None,
    }
}

/// A private-runtime adapter on the HTTP host: the description its bootstrap
/// gives, its reads and writes, and the mounted credential each call uses.
pub struct Bridge<A> {
    adapter: A,
    descriptor: Descriptor,
    reads: BTreeSet<String>,
    writes: BTreeSet<String>,
    instance: String,
    credential: Mounted,
}

impl<A: runtime::Adapter> Bridge<A> {
    /// The instance's reads: the first bootstrap's description, which lists
    /// no write.
    pub async fn new(adapter: A, credential: &CredentialConfig) -> Result<Self> {
        let bootstrap = adapter.bootstrap();
        Self::described(adapter, bootstrap, credential, false).await
    }

    /// The instance's reads and writes: the second bootstrap's description.
    /// Only for a host whose configuration names a standing grant file, so
    /// each write passes the host's grant gate before it reaches
    /// [`Bridge::invoke`].
    pub async fn serving_writes(adapter: A, credential: &CredentialConfig) -> Result<Self> {
        let bootstrap = adapter.bootstrap_v2().map_err(|_| {
            Error::invalid("the instance does not describe its writes to the served host")
        })?;
        Self::described(adapter, bootstrap, credential, true).await
    }

    async fn described(
        adapter: A,
        bootstrap: runtime::Bootstrap,
        credential: &CredentialConfig,
        writes_served: bool,
    ) -> Result<Self> {
        if bootstrap.bindings.is_some() {
            return Err(Error::invalid(
                "a served catalog instance has one binding; alternative bindings are not served",
            ));
        }
        let [profile] = &bootstrap.profiles[..] else {
            return Err(Error::invalid(
                "a served catalog instance uses exactly one profile",
            ));
        };
        // As the mounted binding refuses it on the private host: an OAuth
        // grant's material is written by the CLI, never mounted.
        if profile.acquisition.is_some() {
            return Err(Error::invalid(
                "a profile whose material is acquired by OAuth has no mounted document",
            ));
        }
        let descriptor: Descriptor = serde_json::from_str(&bootstrap.descriptor)
            .map_err(|_| Error::invalid("the instance's description cannot be read"))?;
        if descriptor.instance != bootstrap.instance {
            return Err(Error::invalid(
                "the instance's description names another instance",
            ));
        }
        let mut reads = BTreeSet::new();
        let mut writes = BTreeSet::new();
        for operation in &descriptor.operations {
            let requirement = bootstrap
                .requirements
                .iter()
                .find(|requirement| requirement.operation == operation.id)
                .filter(|requirement| requirement.profile == profile.id);
            let dispatch = requirement.and_then(|requirement| {
                dispatched(
                    requirement.effect,
                    &operation.profile,
                    requirement.custody,
                    writes_served,
                )
            });
            match dispatch {
                Some(Dispatch::Read) => reads.insert(operation.id.clone()),
                Some(Dispatch::Write) => writes.insert(operation.id.clone()),
                None => {
                    return Err(Error::invalid(
                        "the instance's description lists an operation the served host does not dispatch",
                    ));
                }
            };
        }
        let Binding::MountedSecret = credential.binding;
        if !cfg!(target_os = "linux") {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "the mounted_secret binding is available on Linux only",
            ));
        }
        let bridge = Self {
            adapter,
            descriptor,
            reads,
            writes,
            instance: bootstrap.instance.clone(),
            credential: Mounted {
                directory: credential.directory.clone(),
                alias: bootstrap.instance,
                profile: profile.id.clone(),
            },
        };
        // A credential that cannot be read now refuses start.
        drop(bridge.credential.read().await?);
        Ok(bridge)
    }
}

#[async_trait::async_trait]
impl<A: runtime::Adapter> connectors_sdk::Adapter for Bridge<A> {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }

    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let write = self.writes.contains(operation);
        // Refusal: an operation this bridge did not describe.
        if !write && !self.reads.contains(operation) {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "the served host dispatches only the operations it describes",
            ));
        }
        // Refusal: no mounted credential now, so nothing is sent.
        let document = self.credential.read().await?;
        if write {
            return self.write(operation, document, input).await;
        }
        self.adapter
            .invoke_explained(
                operation,
                &self.instance,
                document,
                input,
                Instant::now() + READ_DEADLINE,
            )
            .await
            .map_err(error)
    }
}

impl<A: runtime::Adapter> Bridge<A> {
    /// One write the host admitted under a standing grant, its attempt open:
    /// prepared on the private write path and executed once. Only an applied
    /// write with a deliverable result is `Ok`. A refused preparation, a
    /// refused or unknown write, and an applied write whose result cannot be
    /// delivered are each an error; after dispatch the host records any error
    /// as `unknown` and never dispatches the attempt again.
    async fn write(&self, operation: &str, document: Secret, input: Value) -> Result<Value> {
        let prepared = self
            .adapter
            .prepare_write(operation, &self.instance, document, input)
            .await
            .map_err(|failure| error(Refusal::from(failure)))?;
        match prepared.execute().await {
            WriteOutcome::Applied(Ok(result)) => Ok(result),
            WriteOutcome::Applied(Err(error))
            | WriteOutcome::Refused(error)
            | WriteOutcome::Unknown(error) => Err(error),
        }
    }
}

/// The host's error for a refused read: the code the private failure stands
/// for, a fixed message, and the delay a rate-limit answer named. The
/// upstream's own reason is not forwarded.
fn error(refusal: Refusal) -> Error {
    use Failure::*;
    let (code, message, answered) = match refusal.failure {
        InvalidInput => (ErrorCode::InvalidInput, "invalid input", false),
        InvalidCredential => (
            ErrorCode::Unauthorized,
            "the provider refused the mounted credential",
            true,
        ),
        InsufficientScope | IdentityMismatch => (
            ErrorCode::Forbidden,
            "the mounted credential does not grant this read",
            false,
        ),
        Forbidden => (ErrorCode::Forbidden, "forbidden", false),
        Unavailable | Interrupted => (
            ErrorCode::Unavailable,
            "the provider is not available",
            false,
        ),
        Timeout => (
            ErrorCode::Timeout,
            "the provider did not answer in time",
            false,
        ),
        Protocol => (
            ErrorCode::UpstreamProtocol,
            "the provider's answer cannot be read",
            false,
        ),
        Capacity => (ErrorCode::Capacity, "capacity exceeded", false),
        Unsupported => (ErrorCode::Unsupported, "unsupported", false),
        NotFound => (ErrorCode::NotFound, "not found", false),
        StaleDescription => (
            ErrorCode::StaleDescription,
            "refresh the descriptor before resubmitting",
            false,
        ),
        StaleCursor => (ErrorCode::StaleCursor, "the cursor is stale", false),
        ReadinessMismatch | IncarnationMismatch | InvalidConfiguration => {
            (ErrorCode::Internal, "internal error", false)
        }
        ProviderNotFound => (ErrorCode::NotFound, "the provider found nothing", true),
        ProviderRateLimited => (ErrorCode::RateLimited, "the provider is rate limited", true),
        ProviderInternal => (ErrorCode::Internal, "the provider failed", true),
        ProviderForbidden => (ErrorCode::Forbidden, "the provider refused the read", true),
        ProviderTimeout => (ErrorCode::Timeout, "the provider timed out", true),
        ProviderCapacity => (ErrorCode::Capacity, "the provider is at capacity", true),
        ProviderConflict => (
            ErrorCode::Conflict,
            "the provider reported a conflict",
            true,
        ),
    };
    let mut error = Error::new(code, message);
    if answered {
        error = error.answered();
    }
    if refusal.failure == ProviderRateLimited {
        error.retry_after_seconds = refusal.retry_after_seconds;
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use connectors_host::local::runtime::{Baseline, Bootstrap, Effect, Requirement};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A private adapter whose bootstrap lists `operations` with `effect`,
    /// counting the reads it is asked for.
    struct Listed {
        bootstrap: Bootstrap,
        calls: AtomicUsize,
    }

    impl Listed {
        fn new(operations: &[(&str, Effect, &str)]) -> Self {
            let descriptor = serde_json::json!({
                "version": connectors_core::WIRE_VERSION,
                "instance": "fixture.one",
                "adapter": "catalog",
                "revision": "rev-1",
                "operations": operations.iter().map(|(id, _, profile)| serde_json::json!({
                    "id": id, "description": "d", "contract": "c", "profile": profile,
                    "input_schema": {"type": "object"}, "output_schema": {},
                })).collect::<Vec<_>>(),
                "configuration_schema": {"type": "object"},
            });
            let profile = serde_json::json!({
                "id": "fixture.pat", "revision": "r", "purpose": "delegated_user",
                "subject": "user", "scheme": "token", "capability": "c",
                "minimum_scopes": [], "evidence_lifetime_ms": 1000,
                "fields": [{"name": "token", "label": "Token", "max_bytes": 64}],
            });
            let bootstrap = Bootstrap {
                instance: "fixture.one".into(),
                adapter: "catalog".into(),
                protocol: connectors_core::WIRE_VERSION.into(),
                configuration_revision: "cfg".into(),
                credential_revision: None,
                executable: None,
                provider_authority: "https://provider.example".into(),
                descriptor: descriptor.to_string(),
                profiles: vec![serde_json::from_value(profile).unwrap()],
                requirements: operations
                    .iter()
                    .map(|(id, effect, _)| Requirement {
                        operation: (*id).into(),
                        custody: false,
                        profile: "fixture.pat".into(),
                        scopes: BTreeSet::new(),
                        effect: *effect,
                    })
                    .collect(),
                bindings: None,
            };
            Self {
                bootstrap,
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl runtime::Adapter for Listed {
        fn bootstrap(&self) -> Bootstrap {
            self.bootstrap.clone()
        }
        async fn validate(&self, _profile: &str, _document: Secret) -> runtime::Result<Baseline> {
            Err(Failure::Unsupported)
        }
        async fn invoke(
            &self,
            _operation: &str,
            _partition: &str,
            _document: Secret,
            _input: Value,
        ) -> runtime::Result<Value> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Value::Null)
        }
        async fn prepare_write(
            &self,
            _operation: &str,
            _partition: &str,
            _document: Secret,
            _input: Value,
        ) -> runtime::Result<Box<dyn runtime::PreparedWrite>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(Failure::Unsupported)
        }
    }

    fn mounted() -> CredentialConfig {
        CredentialConfig {
            binding: Binding::MountedSecret,
            directory: PathBuf::from("/nonexistent/connectors-served-test"),
        }
    }

    /// story:served-instance-admits-a-service-account-token: a description
    /// that lists a write, or a read under the mutation profile, refuses
    /// start before the credential is read.
    #[tokio::test]
    async fn a_description_listing_a_write_refuses_start() {
        for operations in [
            vec![
                ("read", Effect::Read, "generic-http"),
                ("write", Effect::Write, "mutation"),
            ],
            vec![("unknown", Effect::Unknown, "generic-http")],
            vec![("posing", Effect::Read, MUTATION_PROFILE)],
        ] {
            let Err(error) = Bridge::new(Listed::new(&operations), &mounted()).await else {
                panic!("a description listing a write must refuse start");
            };
            assert_eq!(error.code, ErrorCode::InvalidInput);
            assert!(
                error.message.contains("does not dispatch"),
                "{}",
                error.message
            );
        }
    }

    /// story:served-keyed-writes-under-a-standing-approval: which described
    /// operation the served host dispatches, and how. A write only with a
    /// grant file, only under the `mutation` profile (so the host's grant gate
    /// applies) and never one whose answer carries a created credential.
    #[test]
    fn a_write_is_dispatched_only_under_a_grant_file_and_the_mutation_profile() {
        use runtime::Effect::{Read, Unknown, Write};
        for (case, (effect, profile, custody, writes, expected)) in [
            (Read, "generic-http", false, false, Some("read")),
            (Read, "generic-http", false, true, Some("read")),
            (Read, MUTATION_PROFILE, false, true, None),
            (Read, "generic-http", true, true, None),
            (Write, MUTATION_PROFILE, false, false, None),
            (Write, MUTATION_PROFILE, false, true, Some("write")),
            (Write, "generic-http", false, true, None),
            (Write, MUTATION_PROFILE, true, true, None),
            (Unknown, "generic-http", false, true, None),
            (Unknown, MUTATION_PROFILE, false, true, None),
        ]
        .into_iter()
        .enumerate()
        {
            let dispatch = dispatched(effect, profile, custody, writes).map(|d| match d {
                Dispatch::Read => "read",
                Dispatch::Write => "write",
            });
            assert_eq!(dispatch, expected, "case {case}");
        }
    }

    /// An instance with no second bootstrap refuses to serve writes.
    #[tokio::test]
    async fn an_instance_without_a_write_description_refuses_to_serve_writes() {
        let listed = Listed::new(&[("read", Effect::Read, "generic-http")]);
        let Err(error) = Bridge::serving_writes(listed, &mounted()).await else {
            panic!("an instance without bootstrap_v2 must refuse to serve writes");
        };
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    /// A described write whose credential cannot be read is `unavailable`
    /// with no preparation and no provider request.
    #[tokio::test]
    async fn a_write_without_a_credential_is_unavailable_with_no_preparation() {
        let adapter = Listed::new(&[("write", Effect::Write, MUTATION_PROFILE)]);
        let bridge = Bridge {
            descriptor: serde_json::from_str(&adapter.bootstrap.descriptor).unwrap(),
            reads: BTreeSet::new(),
            writes: BTreeSet::from(["write".to_owned()]),
            instance: "fixture.one".into(),
            credential: Mounted {
                directory: PathBuf::from("/nonexistent/connectors-served-test"),
                alias: "fixture.one".into(),
                profile: "fixture.pat".into(),
            },
            adapter,
        };
        use connectors_sdk::Adapter as _;
        let error = bridge.invoke("write", Value::Null).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert_eq!(bridge.adapter.calls.load(Ordering::SeqCst), 0);
    }

    /// An operation outside the reads is `unsupported` before the credential
    /// is read and before any adapter call; a read whose credential cannot
    /// be read is `unavailable` with no adapter call.
    #[tokio::test]
    async fn only_a_listed_read_reaches_the_adapter_and_only_with_a_credential() {
        let adapter = Listed::new(&[("read", Effect::Read, "generic-http")]);
        let bridge = Bridge {
            descriptor: serde_json::from_str(&adapter.bootstrap.descriptor).unwrap(),
            reads: BTreeSet::from(["read".to_owned()]),
            writes: BTreeSet::new(),
            instance: "fixture.one".into(),
            credential: Mounted {
                directory: PathBuf::from("/nonexistent/connectors-served-test"),
                alias: "fixture.one".into(),
                profile: "fixture.pat".into(),
            },
            adapter,
        };
        use connectors_sdk::Adapter as _;
        let error = bridge.invoke("write", Value::Null).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unsupported);
        let error = bridge.invoke("read", Value::Null).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert_eq!(bridge.adapter.calls.load(Ordering::SeqCst), 0);
    }

    /// Every private failure maps to a host code; only the provider's own
    /// answers are marked answered, and only a rate limit keeps its delay.
    #[test]
    fn a_refused_read_keeps_its_code_and_drops_the_upstream_reason() {
        let refusal = Refusal {
            failure: Failure::ProviderRateLimited,
            reason: Some("private-upstream-reason".into()),
            retry_after_seconds: Some(7),
        };
        let mapped = error(refusal);
        assert_eq!(mapped.code, ErrorCode::RateLimited);
        assert_eq!(mapped.retry_after_seconds, Some(7));
        assert!(mapped.upstream_answer);
        assert!(mapped.upstream_reason.is_none());
        assert!(!mapped.message.contains("private-upstream-reason"));
        let mapped = error(Refusal {
            failure: Failure::Timeout,
            reason: None,
            retry_after_seconds: Some(7),
        });
        assert_eq!(mapped.code, ErrorCode::Timeout);
        assert_eq!(mapped.retry_after_seconds, None);
        assert!(!mapped.upstream_answer);
    }
}
