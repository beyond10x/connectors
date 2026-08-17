#![forbid(unsafe_code)]

//! Runtime composition for B10x-owned Connector capabilities.
//!
//! This Integration is the only runtime adapter that joins the reviewed B10x catalog to the
//! closed local audio/CDP drivers and to deployment-owned private Work/Ontology/Planner origins. Agent sees
//! only the credential-free operation protocol. It cannot select a driver, executable, profile,
//! voice, filesystem path, HTTP origin, bearer, or placement.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Read as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use connector_resolve::document::{Document, ProtocolDriver};
use connectors_config::B10xIntegrationConfig;
use domain::{AdmittedOperation, Capability, ConnectionAuthority, DriverId};
use driver_audio::{LocalSpeechDriver, SpeechCancellation, SpeechEngine as _};
use driver_cdp::LocalBrowserDriver;
use ed25519_dalek::pkcs8::DecodePrivateKey as _;
use ed25519_dalek::{Signer as _, SigningKey};
use protocol::audio::{SpeechSpeakInput, SPEECH_SPEAK_OPERATION, SPEECH_STATUS_OPERATION};
use protocol::browser::{
    BrowserGotoInput, BrowserOpenInput, BROWSER_CLOSE_OPERATION, BROWSER_GOTO_OPERATION,
    BROWSER_OPEN_OPERATION, BROWSER_SCREENSHOT_OPERATION, BROWSER_SNAPSHOT_OPERATION,
};
use protocol::connection::{
    ChannelState, ConnectionDescription, ConnectionError, ConnectionErrorCode, ConnectionInitiator,
    ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary as LifecycleConnectionSummary,
};
use protocol::event::{
    ChannelSummary, DataEvent, EventError, EventErrorCode, EventProvenance, EventRequest,
    EventResult,
};
use protocol::operation::{
    ConnectionSummary, DescribeRequest, InvocationResult, InvokeRequest, OperationDescription,
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OperationSummary,
};
use serde::Serialize;
use serde_json::Value;
use service::{
    admit_audio_plan, admit_browser_address, admit_browser_plan, admit_speech_speak,
    plan_operation, BackendCapabilities, ConnectorBackend, PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

mod audit;
mod composition;
mod policy;
mod surface;
mod work_events;

use audit::{AuditEvent, AuditJournal};
use policy::{
    all_operation_rows, approval, check_approval, effect, module_operation, operation_row,
    post_dispatch_error, response_schema,
};
use work_events::ModuleEventStore;

const PROVIDER: &str = "b10x";
const DOCUMENT: &str = include_str!("../../../catalog/b10x.catalog.json");
use surface::{
    WorkOwnerEventPage, HTTP_CONNECT_TIMEOUT, HTTP_TOTAL_TIMEOUT, MODULE_AUTHORIZATION_SCHEME,
    MODULE_REQUEST_TTL_SECONDS, MODULE_REQUEST_TYPE, OPERATIONS, PLANNER_EVENT_BINDING,
    PLANNER_EVENT_CHANNEL, WORK_EVENT_BINDING, WORK_EVENT_CHANNEL,
};

/// Runtime construction failure. Messages deliberately carry no bearer or remote response body.
#[derive(Debug, thiserror::Error)]
pub enum B10xIntegrationError {
    #[error("B10x Integration configuration is invalid")]
    InvalidConfiguration,
    #[error("B10x Integration HTTP client is unavailable")]
    HttpClient,
}

#[derive(Clone)]
enum PrincipalAdmission {
    Exact(PrincipalContext),
    Tenants(BTreeSet<String>),
}

/// One composed B10x Provider backend.
pub struct B10xBackend {
    config: B10xIntegrationConfig,
    admission: PrincipalAdmission,
    document: Document,
    catalog: Value,
    client: reqwest::Client,
    http_total_timeout: Duration,
    catalog_sha256: String,
    deployment_sha256: String,
    audio: Option<Arc<Mutex<Option<LocalSpeechDriver>>>>,
    browser: Option<Arc<Mutex<Option<LocalBrowserDriver>>>>,
    audit: AuditJournal,
    work_events: ModuleEventStore,
    planner_events: ModuleEventStore,
    module_signer: Option<ModuleSigner>,
}

struct ModuleSigner {
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
    grants: [&'a str; 1],
    iat: u64,
    nbf: u64,
    exp: u64,
    jti: String,
}

impl ModuleSigner {
    fn load(
        config: &B10xIntegrationConfig,
    ) -> Result<Option<Self>, B10xIntegrationError> {
        if config.work_origin.is_none()
            && config.ontology_origin.is_none()
            && config.planner_origin.is_none()
        {
            return Ok(None);
        }
        let encoded = read_owner_key(
            config
                .module_signing_key_file
                .as_deref()
                .ok_or(B10xIntegrationError::InvalidConfiguration)?,
        )
        .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
        let key = if encoded.starts_with("-----BEGIN PRIVATE KEY-----") {
            SigningKey::from_pkcs8_pem(&encoded)
                .map_err(|_| B10xIntegrationError::InvalidConfiguration)?
        } else {
            let decoded = URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
            let bytes: [u8; 32] = decoded
                .try_into()
                .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
            SigningKey::from_bytes(&bytes)
        };
        Ok(Some(Self {
            issuer: config
                .module_signing_issuer
                .clone()
                .ok_or(B10xIntegrationError::InvalidConfiguration)?,
            kid: config
                .module_signing_key_id
                .clone()
                .ok_or(B10xIntegrationError::InvalidConfiguration)?,
            key,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn authorization(
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

impl B10xBackend {
    fn check_context(&self, context: &PrincipalContext) -> Result<(), OperationError> {
        if self.context_admitted(context) {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn check_connection_context(&self, context: &PrincipalContext) -> Result<(), ConnectionError> {
        if self.context_admitted(context) {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn context_admitted(&self, context: &PrincipalContext) -> bool {
        match &self.admission {
            PrincipalAdmission::Exact(expected) => expected == context,
            PrincipalAdmission::Tenants(tenants) => tenants.contains(context.tenant_id()),
        }
    }

    fn configured(&self, canonical: &str) -> bool {
        match canonical {
            SPEECH_SPEAK_OPERATION | SPEECH_STATUS_OPERATION => self.audio.is_some(),
            BROWSER_OPEN_OPERATION
            | BROWSER_GOTO_OPERATION
            | BROWSER_SNAPSHOT_OPERATION
            | BROWSER_SCREENSHOT_OPERATION
            | BROWSER_CLOSE_OPERATION => self.browser.is_some(),
            "knowledge-query" | "knowledge-explain" | "knowledge-snapshot" => {
                self.module_admitted("ontology") && self.config.ontology_origin.is_some()
            }
            value if value.starts_with("ontology-") => {
                self.module_admitted("ontology") && self.config.ontology_origin.is_some()
            }
            value if value.starts_with("work-") => {
                self.module_admitted("work") && self.config.work_origin.is_some()
            }
            value if value.starts_with("planner-") => {
                self.module_admitted("planner") && self.config.planner_origin.is_some()
            }
            _ => false,
        }
    }

    fn module_admitted(&self, module: &str) -> bool {
        matches!(&self.admission, PrincipalAdmission::Exact(_))
            || self.config.tenant_member_module_enabled(module)
    }

    fn operation(
        &self,
        operation_ref: &str,
    ) -> Option<(
        &connector_resolve::document::Operation,
        &'static str,
        &'static str,
    )> {
        let (canonical, _, title) = operation_row(operation_ref)?;
        self.configured(canonical)
            .then(|| self.document.operation(canonical))
            .flatten()
            .map(|operation| (operation, canonical, title))
    }

    fn connection(&self) -> ConnectionSummary {
        ConnectionSummary {
            connection_ref: self.config.connection.connection_ref.clone(),
            label: self.config.connection.label.clone(),
            provider: PROVIDER.to_owned(),
            audiences: catalog::provider(catalog::ProviderKey::id(PROVIDER))
                .map(|provider| {
                    provider
                        .audiences
                        .iter()
                        .map(|audience| audience.as_str().to_owned())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn lifecycle_connection(&self) -> LifecycleConnectionSummary {
        let initiation = match self.config.connection.initiation {
            connectors_config::InitiationConfig::B10x => {
                vec![ConnectionInitiator::B10x]
            }
            connectors_config::InitiationConfig::Provider => vec![ConnectionInitiator::Provider],
            connectors_config::InitiationConfig::Both => vec![
                ConnectionInitiator::B10x,
                ConnectionInitiator::Provider,
            ],
        };
        LifecycleConnectionSummary {
            connection_ref: self.config.connection.connection_ref.clone(),
            integration_ref: PROVIDER.to_owned(),
            label: self.config.connection.label.clone(),
            state: ConnectionState::Callable,
            initiation,
            route: ConnectionRoute::Direct,
        }
    }

    fn search(&self, query: &str) -> Vec<OperationSummary> {
        let needle = query.to_ascii_lowercase();
        all_operation_rows()
            .filter_map(|(canonical, operation_ref, title)| {
                let operation = self
                    .configured(canonical)
                    .then(|| self.document.operation(canonical))
                    .flatten()?;
                let haystack = format!(
                    "{operation_ref} {title} {}",
                    operation.contract_description()
                )
                .to_ascii_lowercase();
                (needle.is_empty() || haystack.contains(&needle)).then(|| OperationSummary {
                    operation_ref: operation_ref.to_owned(),
                    title: title.to_owned(),
                    effect: effect(operation.effects()),
                    approval: approval(canonical),
                    connections: vec![self.connection()],
                })
            })
            .collect()
    }

    fn description_ref(&self, context: &PrincipalContext, canonical: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(self.catalog_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(self.deployment_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(serde_json::to_vec(context).expect("principal context serializes"));
        digest.update(b"\0");
        digest.update(canonical.as_bytes());
        digest.update(b"\0");
        digest.update(self.config.connection.grant_ref.as_bytes());
        format!("description-sha256-{:x}", digest.finalize())
    }

    fn describe(
        &self,
        context: &PrincipalContext,
        request: DescribeRequest,
    ) -> Result<OperationResult, OperationError> {
        let (operation, canonical, title) = self
            .operation(&request.operation_ref)
            .ok_or_else(not_found)?;
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: request.operation_ref,
            title: title.to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: response_schema(&self.catalog, canonical)?,
            effect: effect(operation.effects()),
            approval: approval(canonical),
            connections: vec![self.connection()],
            description_ref: self.description_ref(context, canonical),
        }))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        let (operation, canonical, _) = self
            .operation(&request.operation_ref)
            .ok_or_else(not_found)?;
        if request.connection_ref != self.config.connection.connection_ref {
            return Err(not_granted());
        }
        if request.description_ref != self.description_ref(context, canonical) {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "operation description lease is stale",
                false,
            ));
        }
        check_approval(canonical, request.approval_evidence_ref.as_deref())?;
        validate_json(operation.input_schema(), &request.input).map_err(|_| invalid())?;
        validate_semantic_input(canonical, &request.input)?;
        let plan = self.plan(context, operation, canonical)?;
        let audit_ref = opaque_ref("audit")?;
        let audit = AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            tenant_id: context.tenant_id(),
            actor_subject: context.actor_subject(),
            outcome: "attempted",
        };
        // The attempted record is durable before dispatch. If audit custody is unavailable, no
        // device, browser, or HTTP effect is allowed to begin.
        self.audit.begin(audit).map_err(|_| unavailable())?;
        let dispatched = match operation.protocol_driver() {
            ProtocolDriver::AudioV1 => self.invoke_audio(canonical, plan, request.input).await,
            ProtocolDriver::CdpV1 => self.invoke_browser(canonical, plan, request.input).await,
            ProtocolDriver::HttpV1 => {
                self.invoke_http(context, canonical, operation, request.input)
                    .await
            }
            ProtocolDriver::SipV1 => Err(unavailable()),
        };
        let dispatched = match dispatched {
            Ok(output) => output,
            Err(error) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "indeterminate",
                        ..audit
                    })
                    .map_err(|_| post_dispatch_error(operation, unavailable()))?;
                return Err(post_dispatch_error(operation, error));
            }
        };
        let output_schema = match response_schema(&self.catalog, canonical) {
            Ok(schema) => schema,
            Err(error) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "indeterminate",
                        ..audit
                    })
                    .map_err(|_| post_dispatch_error(operation, unavailable()))?;
                return Err(post_dispatch_error(operation, error));
            }
        };
        let output = match validate_json(&output_schema, &dispatched) {
            Ok(()) => dispatched,
            Err(()) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "indeterminate",
                        ..audit
                    })
                    .map_err(|_| post_dispatch_error(operation, unavailable()))?;
                return Err(post_dispatch_error(operation, unavailable()));
            }
        };
        self.audit
            .finish(AuditEvent {
                audit_ref: &audit_ref,
                operation_ref: &request.operation_ref,
                connection_ref: &request.connection_ref,
                tenant_id: context.tenant_id(),
                actor_subject: context.actor_subject(),
                outcome: "completed",
            })
            .map_err(|_| post_dispatch_error(operation, unavailable()))?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: request.operation_ref,
            output,
            connector_audit_ref: audit_ref,
            execution_ref: None,
        }))
    }

    fn plan(
        &self,
        context: &PrincipalContext,
        operation: &connector_resolve::document::Operation,
        canonical: &str,
    ) -> Result<domain::ZeroIoPlan, OperationError> {
        let connection = ConnectionAuthority::new(
            &self.config.connection.connection_ref,
            self.config.initiation_policy(),
        )
        .map_err(|_| not_granted())?;
        let admission = AdmittedOperation::from_grant_decision(
            PROVIDER,
            canonical,
            context.tenant_id(),
            context.actor_subject(),
            &self.config.connection.grant_ref,
            connection,
        );
        let (drivers, capabilities, permission_subjects) = match operation.protocol_driver() {
            ProtocolDriver::AudioV1 => (
                BTreeSet::from([DriverId::AudioV1]),
                BTreeSet::from([Capability::Device]),
                vec!["device:local-audio-output".to_owned()],
            ),
            ProtocolDriver::CdpV1 => (
                BTreeSet::from([DriverId::CdpV1]),
                BTreeSet::from([Capability::PublicNetwork, Capability::Process]),
                vec!["browser:dedicated-profile".to_owned()],
            ),
            ProtocolDriver::HttpV1 => {
                let origin = self.origin(canonical).ok_or_else(unavailable)?;
                let mut capabilities = BTreeSet::from([Capability::PrivateNetwork]);
                if is_ontology_operation(canonical) {
                    capabilities.insert(Capability::FileSecret);
                }
                (
                    BTreeSet::from([DriverId::HttpV1]),
                    capabilities,
                    vec![origin],
                )
            }
            ProtocolDriver::SipV1 => return Err(unavailable()),
        };
        plan_operation(
            PROVIDER,
            operation,
            admission,
            &PlanningEnvironment {
                available_drivers: drivers,
                available_route_adapters: BTreeSet::new(),
                capabilities,
                permission_subjects,
            },
        )
        .map_err(|_| not_granted())
    }

    async fn invoke_audio(
        &self,
        canonical: &str,
        plan: domain::ZeroIoPlan,
        input: Value,
    ) -> Result<Value, OperationError> {
        let route = self.config.audio_route().ok_or_else(unavailable)?;
        let state = self.audio.as_ref().ok_or_else(unavailable)?.clone();
        let canonical = canonical.to_owned();
        tokio::task::spawn_blocking(move || {
            let (admitted, speak_input) = if canonical == SPEECH_SPEAK_OPERATION {
                let input: SpeechSpeakInput =
                    serde_json::from_value(input).map_err(|_| invalid())?;
                let (admitted, _) =
                    admit_speech_speak(&plan, &input, route).map_err(|_| invalid())?;
                (admitted, Some(input))
            } else {
                (
                    admit_audio_plan(&plan, route).map_err(|_| not_granted())?,
                    None,
                )
            };
            let mut driver = lock(&state);
            if driver.is_none() {
                let mut engine = driver_audio::engine_for(&admitted);
                let attestation = engine.probe().map_err(|_| unavailable())?;
                *driver = Some(
                    LocalSpeechDriver::new(
                        &admitted,
                        Box::new(engine),
                        attestation,
                        SpeechCancellation::new(),
                    )
                    .map_err(|_| unavailable())?,
                );
            }
            let output = match speak_input {
                Some(input) => serde_json::to_value(
                    driver
                        .as_mut()
                        .expect("driver initialized")
                        .speak(&admitted, &input)
                        .map_err(|_| unavailable())?,
                ),
                None => serde_json::to_value(
                    driver
                        .as_ref()
                        .expect("driver initialized")
                        .status(&admitted)
                        .map_err(|_| unavailable())?,
                ),
            };
            output.map_err(|_| unavailable())
        })
        .await
        .map_err(|_| unavailable())?
    }

    async fn invoke_browser(
        &self,
        canonical: &str,
        plan: domain::ZeroIoPlan,
        input: Value,
    ) -> Result<Value, OperationError> {
        let route = self.config.browser_route().ok_or_else(unavailable)?;
        let state = self.browser.as_ref().ok_or_else(unavailable)?.clone();
        let canonical = canonical.to_owned();
        tokio::task::spawn_blocking(move || {
            let (admitted, open, go) = match canonical.as_str() {
                BROWSER_OPEN_OPERATION => {
                    let input = browser_open_input(input)?;
                    input.validate().map_err(|_| invalid())?;
                    (
                        admit_browser_address(&plan, input.url.as_deref(), route)
                            .map_err(|_| invalid())?,
                        Some(input),
                        None,
                    )
                }
                BROWSER_GOTO_OPERATION => {
                    let input = browser_goto_input(input)?;
                    input.validate().map_err(|_| invalid())?;
                    (
                        admit_browser_address(&plan, Some(&input.url), route)
                            .map_err(|_| invalid())?,
                        None,
                        Some(input),
                    )
                }
                _ => (
                    admit_browser_plan(&plan, route).map_err(|_| not_granted())?,
                    None,
                    None,
                ),
            };
            let mut driver = lock(&state);
            if driver.is_none() {
                *driver = Some(
                    LocalBrowserDriver::new(&admitted, Box::new(driver_cdp::engine_for(&admitted)))
                        .map_err(|_| unavailable())?,
                );
            }
            let driver = driver.as_mut().expect("driver initialized");
            let output = match canonical.as_str() {
                BROWSER_OPEN_OPERATION => serde_json::to_value(
                    driver
                        .open(&admitted, &open.expect("open input"))
                        .map_err(|_| unavailable())?,
                ),
                BROWSER_GOTO_OPERATION => serde_json::to_value(
                    driver
                        .goto(&admitted, &go.expect("goto input"))
                        .map_err(|_| unavailable())?,
                ),
                BROWSER_SNAPSHOT_OPERATION => {
                    serde_json::to_value(driver.snapshot(&admitted).map_err(|_| unavailable())?)
                }
                BROWSER_SCREENSHOT_OPERATION => {
                    serde_json::to_value(driver.screenshot(&admitted).map_err(|_| unavailable())?)
                }
                BROWSER_CLOSE_OPERATION => {
                    serde_json::to_value(driver.close(&admitted).map_err(|_| unavailable())?)
                }
                _ => return Err(not_found()),
            };
            output.map_err(|_| unavailable())
        })
        .await
        .map_err(|_| unavailable())?
    }

    async fn invoke_http(
        &self,
        context: &PrincipalContext,
        canonical: &str,
        operation: &connector_resolve::document::Operation,
        input: Value,
    ) -> Result<Value, OperationError> {
        tokio::time::timeout(
            self.http_total_timeout,
            self.invoke_http_within_deadline(context, canonical, operation, input),
        )
        .await
        .map_err(|_| unavailable())?
    }

    async fn invoke_http_within_deadline(
        &self,
        context: &PrincipalContext,
        canonical: &str,
        operation: &connector_resolve::document::Operation,
        input: Value,
    ) -> Result<Value, OperationError> {
        let origin = self.origin(canonical).ok_or_else(unavailable)?;
        let credentials = Vec::new();
        let plan =
            connector_resolve::resolve(operation, &origin, &input, &BTreeMap::new(), &credentials)
                .map_err(|_| invalid())?;
        let method = reqwest::Method::from_bytes(plan.request.method.as_bytes())
            .map_err(|_| unavailable())?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| unavailable())?;
        same_origin(&origin, &target)
            .then_some(())
            .ok_or_else(not_granted)?;
        let request_method = plan.request.method.clone();
        let target_binding = match target.query() {
            Some(query) => format!("{}?{query}", target.path()),
            None => target.path().to_owned(),
        };
        let body = plan.request.body.unwrap_or_default();
        let idempotency_key = plan
            .request
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("idempotency-key"))
            .map(|(_, value)| value.as_str());
        let operation = module_operation(canonical).ok_or_else(not_granted)?;
        let audience = if canonical.starts_with("work-") {
            "urn:b10x:module:work"
        } else if canonical.starts_with("planner-") {
            "urn:b10x:module:planner"
        } else {
            "urn:b10x:module:ontology"
        };
        let authorization = self
            .module_signer
            .as_ref()
            .ok_or_else(not_granted)?
            .authorization(
                context,
                audience,
                operation,
                &request_method,
                &target_binding,
                body.as_bytes(),
                idempotency_key,
            )?;
        let mut outbound = self
            .client
            .request(method, target)
            .header(reqwest::header::AUTHORIZATION, authorization);
        for (name, value) in plan.request.headers {
            outbound = outbound.header(name, value);
        }
        if !body.is_empty() {
            outbound = outbound.body(body);
        }
        let mut response = outbound.send().await.map_err(|_| unavailable())?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(not_found());
        }
        if response.status().is_client_error() {
            return Err(invalid());
        }
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|size| size > protocol::operation::MAX_RESULT_BYTES as u64)
        {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > protocol::operation::MAX_RESULT_BYTES)
            {
                return Err(OperationError::new(
                    OperationErrorCode::ResultTooLarge,
                    "operation result exceeds the transport bound",
                    false,
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| unavailable())
    }

    fn origin(&self, canonical: &str) -> Option<String> {
        if is_ontology_operation(canonical) {
            self.config.ontology_origin()
        } else if canonical.starts_with("work-") {
            self.config.work_origin()
        } else if canonical.starts_with("planner-") {
            self.config.planner_origin()
        } else {
            None
        }
    }

    async fn refresh_work_events(&self, context: &PrincipalContext) -> Result<(), EventError> {
        let origin = self.config.work_origin().ok_or_else(event_not_granted)?;
        let mut url = reqwest::Url::parse(&format!("{origin}/api/work/v2/events"))
            .map_err(|_| event_protocol())?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(cursor) = self.work_events.owner_cursor(context.tenant_id())? {
                query.append_pair("cursor", &cursor);
            }
            query.append_pair("limit", "100");
        }
        let target = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        let authorization = self
            .module_signer
            .as_ref()
            .ok_or_else(event_not_granted)?
            .authorization(
                context,
                "urn:b10x:module:work",
                "module.events.read",
                "GET",
                &target,
                &[],
                None,
            )
            .map_err(|_| event_not_granted())?;
        let response = self
            .client
            .get(url)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .send()
            .await
            .map_err(|_| event_unavailable())?;
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(EventError::new(
                EventErrorCode::Protocol,
                "Work owner cursor requires a full resynchronization",
                false,
            ));
        }
        if !response.status().is_success() {
            return Err(event_unavailable());
        }
        let bytes = response.bytes().await.map_err(|_| event_unavailable())?;
        if bytes.len() > protocol::event::MAX_RESPONSE_BYTES {
            return Err(event_protocol());
        }
        let page: WorkOwnerEventPage =
            serde_json::from_slice(&bytes).map_err(|_| event_protocol())?;
        if page.has_more && page.next_cursor.is_none() {
            return Err(event_protocol());
        }
        let received_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let mut events = Vec::with_capacity(page.events.len());
        for owner in page.events {
            if owner.protocol != "b10x.module-event.v1"
                || owner.module != "work"
                || owner.schema_version != 1
                || owner.id.is_empty()
                || owner.cursor.is_empty()
                || !matches!(
                    owner.key.as_str(),
                    "request.created" | "task.created" | "task.status-changed"
                )
            {
                return Err(event_protocol());
            }
            let owner_id = owner.id.clone();
            let event_type = owner.key.clone();
            let payload = serde_json::to_value(owner).map_err(|_| event_protocol())?;
            events.push((
                owner_id,
                DataEvent {
                    event_ref: "pending:b10x:work".to_owned(),
                    channel_ref: WORK_EVENT_CHANNEL.to_owned(),
                    connection_ref: self.config.connection.connection_ref.clone(),
                    integration_ref: PROVIDER.to_owned(),
                    event_type,
                    provenance: EventProvenance::Polled,
                    received_at_unix_ms,
                    payload,
                },
            ));
        }
        self.work_events
            .append(context.tenant_id(), page.next_cursor, events)
    }

    async fn refresh_planner_events(&self, context: &PrincipalContext) -> Result<(), EventError> {
        let origin = self.config.planner_origin().ok_or_else(event_not_granted)?;
        let mut url = reqwest::Url::parse(&format!("{origin}/api/planner/v1/events"))
            .map_err(|_| event_protocol())?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(cursor) = self.planner_events.owner_cursor(context.tenant_id())? {
                query.append_pair("cursor", &cursor);
            }
            query.append_pair("limit", "100");
        }
        let target = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        let authorization = self
            .module_signer
            .as_ref()
            .ok_or_else(event_not_granted)?
            .authorization(
                context,
                "urn:b10x:module:planner",
                "module.events.read",
                "GET",
                &target,
                &[],
                None,
            )
            .map_err(|_| event_not_granted())?;
        let response = self
            .client
            .get(url)
            .header(reqwest::header::AUTHORIZATION, authorization)
            .send()
            .await
            .map_err(|_| event_unavailable())?;
        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(EventError::new(
                EventErrorCode::Protocol,
                "Planner owner cursor requires a full resynchronization",
                false,
            ));
        }
        if !response.status().is_success() {
            return Err(event_unavailable());
        }
        let bytes = response.bytes().await.map_err(|_| event_unavailable())?;
        if bytes.len() > protocol::event::MAX_RESPONSE_BYTES {
            return Err(event_protocol());
        }
        let page: WorkOwnerEventPage =
            serde_json::from_slice(&bytes).map_err(|_| event_protocol())?;
        if page.has_more && page.next_cursor.is_none() {
            return Err(event_protocol());
        }
        let received_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let mut events = Vec::with_capacity(page.events.len());
        for owner in page.events {
            if owner.protocol != "b10x.module-event.v1"
                || owner.module != "planner"
                || owner.schema_version != 1
                || owner.id.is_empty()
                || owner.cursor.is_empty()
                || !matches!(
                    owner.key.as_str(),
                    "project.registered"
                        | "entity.created"
                        | "entity.updated"
                        | "entity.deleted"
                        | "entity.restored"
                        | "sync-conflict.recorded"
                        | "sync-conflict.resolved"
                )
            {
                return Err(event_protocol());
            }
            let owner_id = owner.id.clone();
            let event_type = owner.key.clone();
            let payload = serde_json::to_value(owner).map_err(|_| event_protocol())?;
            events.push((
                owner_id,
                DataEvent {
                    event_ref: "pending:b10x:planner".to_owned(),
                    channel_ref: PLANNER_EVENT_CHANNEL.to_owned(),
                    connection_ref: self.config.connection.connection_ref.clone(),
                    integration_ref: PROVIDER.to_owned(),
                    event_type,
                    provenance: EventProvenance::Polled,
                    received_at_unix_ms,
                    payload,
                },
            ));
        }
        self.planner_events
            .append(context.tenant_id(), page.next_cursor, events)
    }
}

fn is_ontology_operation(canonical: &str) -> bool {
    canonical.starts_with("knowledge-") || canonical.starts_with("ontology-")
}

// `url` is reserved by the generic request assembler, so the frozen catalog contract publishes
// the caller symbol `url_2` beside the provider-authored wire name `url`. Built-in CDP dispatch
// does not use the generic HTTP assembler; it must perform that same explicit symbol-to-driver
// translation before entering the closed browser protocol.
fn browser_open_input(input: Value) -> Result<BrowserOpenInput, OperationError> {
    Ok(BrowserOpenInput {
        url: Some(browser_url_symbol(input)?),
    })
}

fn browser_goto_input(input: Value) -> Result<BrowserGotoInput, OperationError> {
    Ok(BrowserGotoInput {
        url: browser_url_symbol(input)?,
    })
}

fn browser_url_symbol(input: Value) -> Result<String, OperationError> {
    let mut fields = input.as_object().cloned().ok_or_else(invalid)?;
    if fields.len() != 1 {
        return Err(invalid());
    }
    fields
        .remove("url_2")
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .ok_or_else(invalid)
}

#[async_trait]
impl ConnectorBackend for B10xBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        // Construction validates local drivers and configured origins. Remote Work/Ontology
        // availability is operation-scoped and must not become global process readiness.
        Ok(())
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: true,
            connections: true,
            events: self.config.work_origin.is_some() || self.config.planner_origin.is_some(),
        }
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => self.operation(&request.operation_ref).is_some(),
            OperationRequest::Invoke(request) => {
                request.connection_ref == self.config.connection.connection_ref
                    && self.operation(&request.operation_ref).is_some()
            }
            OperationRequest::Search(_)
            | OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_) => false,
        }
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        matches!(request, ConnectionRequest::Describe(request)
            if request.connection_ref == self.config.connection.connection_ref)
    }

    fn owns_event(&self, request: &EventRequest) -> bool {
        match request {
            EventRequest::Receive(request) => {
                matches!(
                    request.channel_ref.as_str(),
                    WORK_EVENT_CHANNEL | PLANNER_EVENT_CHANNEL
                )
            }
            EventRequest::Replay(request) => {
                request.event_ref.starts_with("event:b10x:work:")
                    || request.event_ref.starts_with("event:b10x:planner:")
            }
            EventRequest::Search(_) => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.check_context(context)?;
        match request {
            OperationRequest::Search(request) => Ok(OperationResult::Search {
                operations: self.search(&request.query),
            }),
            OperationRequest::Describe(request) => self.describe(context, request),
            OperationRequest::Invoke(request) => self.invoke(context, request).await,
            OperationRequest::SessionStatus(_)
            | OperationRequest::SessionTerminate(_)
            | OperationRequest::SessionReconcile(_) => Err(not_found()),
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.check_connection_context(context)?;
        match request {
            ConnectionRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let summary = self.lifecycle_connection();
                let connections = (query.is_empty()
                    || PROVIDER.contains(&query)
                    || summary.label.to_ascii_lowercase().contains(&query))
                .then_some(summary)
                .into_iter()
                .take(usize::from(request.limit))
                .collect();
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request)
                if request.connection_ref == self.config.connection.connection_ref =>
            {
                let mut channels = Vec::new();
                if self.configured("work-request-list") {
                    channels.push(protocol::connection::ChannelSummary {
                        channel_ref: WORK_EVENT_CHANNEL.to_owned(),
                        binding_ref: WORK_EVENT_BINDING.to_owned(),
                        state: ChannelState::Connected,
                        events: vec![
                            "work/request.created".to_owned(),
                            "work/task.created".to_owned(),
                            "work/task.status-changed".to_owned(),
                        ],
                    });
                }
                if self.configured("planner-project-list") {
                    channels.push(protocol::connection::ChannelSummary {
                        channel_ref: PLANNER_EVENT_CHANNEL.to_owned(),
                        binding_ref: PLANNER_EVENT_BINDING.to_owned(),
                        state: ChannelState::Connected,
                        events: vec![
                            "planner/project.registered".to_owned(),
                            "planner/entity.created".to_owned(),
                            "planner/entity.updated".to_owned(),
                            "planner/entity.deleted".to_owned(),
                            "planner/entity.restored".to_owned(),
                            "planner/sync-conflict.recorded".to_owned(),
                            "planner/sync-conflict.resolved".to_owned(),
                        ],
                    });
                }
                Ok(ConnectionResult::Describe(ConnectionDescription {
                    summary: self.lifecycle_connection(),
                    channels,
                }))
            }
            _ => Err(ConnectionError::new(
                ConnectionErrorCode::NotFound,
                "B10x Integration Connection was not found",
                false,
            )),
        }
    }

    async fn handle_event(
        &self,
        context: &PrincipalContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        if !self.context_admitted(context) {
            return Err(EventError::new(
                EventErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ));
        }
        match request {
            EventRequest::Search(request) => {
                let query = request.query.to_ascii_lowercase();
                let mut channels = Vec::new();
                if self.configured("work-request-list")
                    && (query.is_empty()
                        || "work".contains(&query)
                        || WORK_EVENT_BINDING.contains(&query))
                {
                    channels.push(work_event_channel(&self.config));
                }
                if self.configured("planner-project-list")
                    && (query.is_empty()
                        || "planner".contains(&query)
                        || PLANNER_EVENT_BINDING.contains(&query))
                {
                    channels.push(planner_event_channel(&self.config));
                }
                channels.truncate(usize::from(request.limit));
                Ok(EventResult::Search { channels })
            }
            EventRequest::Receive(request) => {
                let after = request
                    .after
                    .as_deref()
                    .unwrap_or("0")
                    .parse::<u64>()
                    .map_err(|_| event_invalid())?;
                let (events, next) = match request.channel_ref.as_str() {
                    WORK_EVENT_CHANNEL if self.configured("work-request-list") => {
                        self.refresh_work_events(context).await?;
                        self.work_events.receive(
                            context.tenant_id(),
                            after,
                            usize::from(request.limit),
                        )?
                    }
                    PLANNER_EVENT_CHANNEL if self.configured("planner-project-list") => {
                        self.refresh_planner_events(context).await?;
                        self.planner_events.receive(
                            context.tenant_id(),
                            after,
                            usize::from(request.limit),
                        )?
                    }
                    WORK_EVENT_CHANNEL | PLANNER_EVENT_CHANNEL => {
                        return Err(event_not_granted());
                    }
                    _ => return Err(event_not_found()),
                };
                Ok(EventResult::Receive {
                    events,
                    next: next.to_string(),
                })
            }
            EventRequest::Replay(request) => {
                let event = if request.event_ref.starts_with("event:b10x:work:")
                    && self.configured("work-request-list")
                {
                    self.work_events
                        .replay(context.tenant_id(), &request.event_ref)?
                } else if request.event_ref.starts_with("event:b10x:planner:")
                    && self.configured("planner-project-list")
                {
                    self.planner_events
                        .replay(context.tenant_id(), &request.event_ref)?
                } else {
                    return Err(event_not_found());
                }
                .ok_or_else(event_not_found)?;
                Ok(EventResult::Replay(event))
            }
        }
    }

    async fn shutdown(&self) {
        if let Some(browser) = &self.browser {
            let mut browser = lock(browser);
            // ChromiumBrowserEngine's Drop terminates the process. Removing the owned driver here
            // ensures the process is gone before the Connector transport endpoint disappears.
            *browser = None;
        }
    }
}

fn http_client(
    connect_timeout: Duration,
    total_timeout: Duration,
) -> Result<reqwest::Client, B10xIntegrationError> {
    if connect_timeout.is_zero() || total_timeout.is_zero() || connect_timeout > total_timeout {
        return Err(B10xIntegrationError::InvalidConfiguration);
    }
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(connect_timeout)
        .timeout(total_timeout)
        .build()
        .map_err(|_| B10xIntegrationError::HttpClient)
}

fn work_event_channel(config: &B10xIntegrationConfig) -> ChannelSummary {
    ChannelSummary {
        channel_ref: WORK_EVENT_CHANNEL.to_owned(),
        connection_ref: config.connection.connection_ref.clone(),
        integration_ref: PROVIDER.to_owned(),
        binding_ref: WORK_EVENT_BINDING.to_owned(),
        events: vec![
            "work/request.created".to_owned(),
            "work/task.created".to_owned(),
            "work/task.status-changed".to_owned(),
        ],
    }
}

fn planner_event_channel(config: &B10xIntegrationConfig) -> ChannelSummary {
    ChannelSummary {
        channel_ref: PLANNER_EVENT_CHANNEL.to_owned(),
        connection_ref: config.connection.connection_ref.clone(),
        integration_ref: PROVIDER.to_owned(),
        binding_ref: PLANNER_EVENT_BINDING.to_owned(),
        events: vec![
            "planner/project.registered".to_owned(),
            "planner/entity.created".to_owned(),
            "planner/entity.updated".to_owned(),
            "planner/entity.deleted".to_owned(),
            "planner/entity.restored".to_owned(),
            "planner/sync-conflict.recorded".to_owned(),
            "planner/sync-conflict.resolved".to_owned(),
        ],
    }
}

fn event_unavailable() -> EventError {
    EventError::new(
        EventErrorCode::Unavailable,
        "Module event owner is unavailable",
        true,
    )
}

fn event_not_found() -> EventError {
    EventError::new(
        EventErrorCode::NotFound,
        "Module event or channel was not found",
        false,
    )
}

fn event_not_granted() -> EventError {
    EventError::new(
        EventErrorCode::NotGranted,
        "Module events are not granted to this tenant member",
        false,
    )
}

fn event_invalid() -> EventError {
    EventError::new(
        EventErrorCode::InvalidInput,
        "Work event request is invalid",
        false,
    )
}

fn event_protocol() -> EventError {
    EventError::new(
        EventErrorCode::Protocol,
        "Work owner event response was refused",
        false,
    )
}

fn validate_json(schema: &Value, value: &Value) -> Result<(), ()> {
    let validator = jsonschema::validator_for(schema).map_err(|_| ())?;
    validator.is_valid(value).then_some(()).ok_or(())
}

fn validate_semantic_input(canonical: &str, input: &Value) -> Result<(), OperationError> {
    match canonical {
        "knowledge-query" | "knowledge-snapshot" => {
            let fields = input.as_object().ok_or_else(invalid)?;
            if fields.len() != 4
                || !["branches", "limit", "predicate", "subject"]
                    .iter()
                    .all(|name| fields.contains_key(*name))
            {
                return Err(invalid());
            }
            let branches = fields["branches"].as_array().ok_or_else(invalid)?;
            let mut unique = BTreeSet::new();
            if branches.len() > 16
                || branches.iter().any(|branch| {
                    branch
                        .as_str()
                        .is_none_or(|branch| !valid_ontology_ref(branch) || !unique.insert(branch))
                })
                || !fields["limit"]
                    .as_u64()
                    .is_some_and(|limit| (1..=100).contains(&limit))
                || !nullable_ontology_ref(&fields["predicate"])
                || !nullable_ontology_ref(&fields["subject"])
            {
                return Err(invalid());
            }
        }
        "knowledge-explain" => {
            let fields = input.as_object().ok_or_else(invalid)?;
            if fields.len() != 2
                || !["branch", "claim"].iter().all(|name| {
                    fields
                        .get(*name)
                        .and_then(Value::as_str)
                        .is_some_and(valid_ontology_ref)
                })
            {
                return Err(invalid());
            }
        }
        _ => {}
    }
    Ok(())
}

fn nullable_ontology_ref(value: &Value) -> bool {
    value.is_null() || value.as_str().is_some_and(valid_ontology_ref)
}

fn valid_ontology_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control())
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

fn same_origin(origin: &str, target: &url::Url) -> bool {
    let Ok(origin) = url::Url::parse(origin) else {
        return false;
    };
    origin.scheme() == target.scheme()
        && origin.host_str() == target.host_str()
        && origin.port_or_known_default() == target.port_or_known_default()
        && target.username().is_empty()
        && target.password().is_none()
        && target.fragment().is_none()
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

fn not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "operation was not found",
        false,
    )
}

fn not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Connection",
        false,
    )
}

fn invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "operation input is invalid",
        false,
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
