#![forbid(unsafe_code)]

//! Runtime composition for B10x-owned Connector capabilities.
//!
//! This Integration is the only runtime adapter that joins the reviewed B10x catalog to the
//! closed local audio/CDP drivers and to deployment-owned private Work/Ontology origins. Agent sees
//! only the credential-free operation protocol. It cannot select a driver, executable, profile,
//! voice, filesystem path, HTTP origin, bearer, or placement.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::Read as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use async_trait::async_trait;
use catalog::Placement as CredentialPlacement;
use connector_resolve::auth::Assembled;
use connector_resolve::document::{Document, ProtocolDriver};
use connectors_config::B10xIntegrationConfig;
use domain::{AdmittedOperation, Capability, ConnectionAuthority, DriverId};
use driver_audio::{LocalSpeechDriver, SpeechCancellation, SpeechEngine as _};
use driver_cdp::LocalBrowserDriver;
use protocol::audio::{
    SpeechSpeakInput, SPEECH_SPEAK_OPERATION, SPEECH_SPEAK_TOOL_REF, SPEECH_STATUS_OPERATION,
    SPEECH_STATUS_TOOL_REF,
};
use protocol::browser::{
    BrowserGotoInput, BrowserOpenInput, BROWSER_CLOSE_OPERATION, BROWSER_CLOSE_TOOL_REF,
    BROWSER_GOTO_OPERATION, BROWSER_GOTO_TOOL_REF, BROWSER_OPEN_OPERATION, BROWSER_OPEN_TOOL_REF,
    BROWSER_SCREENSHOT_OPERATION, BROWSER_SCREENSHOT_TOOL_REF, BROWSER_SNAPSHOT_OPERATION,
    BROWSER_SNAPSHOT_TOOL_REF,
};
use protocol::connection::{
    ConnectionDescription, ConnectionError, ConnectionErrorCode, ConnectionInitiator,
    ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary as LifecycleConnectionSummary,
};
use protocol::operation::{
    ConnectionSummary, DescribeRequest, InvocationResult, InvokeRequest, OperationDescription,
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OperationSummary,
};
use serde_json::Value;
use service::{
    admit_audio_plan, admit_browser_address, admit_browser_plan, admit_speech_speak,
    plan_operation, BackendCapabilities, ConnectorBackend, PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

mod audit;
mod policy;

use audit::{AuditEvent, AuditJournal};
use policy::{
    all_operation_rows, approval, check_approval, effect, operation_row, post_dispatch_error,
    response_schema,
};

const PROVIDER: &str = "b10x";
const DOCUMENT: &str = include_str!("../../../catalog/b10x.catalog.json");
const MAX_BEARER_BYTES: u64 = 512;
const ONTOLOGY_BEARER_CREDENTIAL: &str = "b10x.internal.ontology_bearer";
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HTTP_TOTAL_TIMEOUT: Duration = Duration::from_secs(30);

const OPERATIONS: [(&str, &str, &str); 15] = [
    (
        SPEECH_SPEAK_OPERATION,
        SPEECH_SPEAK_TOOL_REF,
        "Speak on local audio",
    ),
    (
        SPEECH_STATUS_OPERATION,
        SPEECH_STATUS_TOOL_REF,
        "Inspect local speech readiness",
    ),
    (
        BROWSER_OPEN_OPERATION,
        BROWSER_OPEN_TOOL_REF,
        "Open a dedicated browser",
    ),
    (
        BROWSER_GOTO_OPERATION,
        BROWSER_GOTO_TOOL_REF,
        "Navigate the dedicated browser",
    ),
    (
        BROWSER_SNAPSHOT_OPERATION,
        BROWSER_SNAPSHOT_TOOL_REF,
        "Read the browser page structure",
    ),
    (
        BROWSER_SCREENSHOT_OPERATION,
        BROWSER_SCREENSHOT_TOOL_REF,
        "Capture a browser screenshot",
    ),
    (
        BROWSER_CLOSE_OPERATION,
        BROWSER_CLOSE_TOOL_REF,
        "Close the dedicated browser",
    ),
    (
        "knowledge-query",
        "knowledge.query",
        "Query visible Ontology claims",
    ),
    (
        "knowledge-explain",
        "knowledge.explain",
        "Explain one Ontology claim",
    ),
    (
        "knowledge-snapshot",
        "knowledge.snapshot",
        "Create an Ontology context snapshot",
    ),
    (
        "work-request-create",
        "work.requests.create",
        "Create a Work request",
    ),
    (
        "work-request-get",
        "work.requests.get",
        "Get a Work request",
    ),
    (
        "work-request-list",
        "work.requests.list",
        "List Work requests",
    ),
    (
        "work-task-create",
        "work.tasks.create",
        "Create a Work task",
    ),
    ("work-task-get", "work.tasks.get", "Get a Work task"),
];

const REMAINING_WORK_OPERATIONS: [(&str, &str, &str); 3] = [
    ("work-task-list", "work.tasks.list", "List Work tasks"),
    (
        "work-task-status-update",
        "work.tasks.status.update",
        "Update Work task status",
    ),
    ("work-event-list", "work.events.list", "List Work events"),
];

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
    Tenant(String),
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
}

impl B10xBackend {
    /// Compose a personal-local backend pinned to one exact Agent authority snapshot.
    pub fn personal(
        config: B10xIntegrationConfig,
        principal: PrincipalContext,
        state_root: &Path,
    ) -> Result<Self, B10xIntegrationError> {
        Self::new(config, PrincipalAdmission::Exact(principal), state_root)
    }

    /// Compose a hosted backend for Identity-verified principals in one tenant.
    pub fn hosted(
        config: B10xIntegrationConfig,
        tenant_id: String,
        state_root: &Path,
    ) -> Result<Self, B10xIntegrationError> {
        Self::new(config, PrincipalAdmission::Tenant(tenant_id), state_root)
    }

    fn new(
        config: B10xIntegrationConfig,
        admission: PrincipalAdmission,
        state_root: &Path,
    ) -> Result<Self, B10xIntegrationError> {
        let document = Document::parse(DOCUMENT)
            .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
        let catalog: Value = serde_json::from_str(DOCUMENT)
            .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
        if document.connector != PROVIDER {
            return Err(B10xIntegrationError::InvalidConfiguration);
        }
        let client = http_client(HTTP_CONNECT_TIMEOUT, HTTP_TOTAL_TIMEOUT)?;
        let deployment_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&config)
                    .map_err(|_| B10xIntegrationError::InvalidConfiguration)?,
            )
        );
        Ok(Self {
            audio: config.audio_route().map(|_| Arc::new(Mutex::new(None))),
            browser: config.browser_route().map(|_| Arc::new(Mutex::new(None))),
            config,
            admission,
            document,
            catalog,
            client,
            http_total_timeout: HTTP_TOTAL_TIMEOUT,
            catalog_sha256: format!("{:x}", Sha256::digest(DOCUMENT.as_bytes())),
            deployment_sha256,
            audit: AuditJournal::new(state_root.join("b10x-operation-audit.jsonl")),
        })
    }

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
            PrincipalAdmission::Tenant(tenant) => tenant == context.tenant_id(),
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
                self.config.ontology_origin.is_some()
            }
            value if value.starts_with("work-") => self.config.work_origin.is_some(),
            _ => false,
        }
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
            ProtocolDriver::HttpV1 => self.invoke_http(canonical, operation, request.input).await,
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
                if canonical.starts_with("knowledge-") {
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
        canonical: &str,
        operation: &connector_resolve::document::Operation,
        input: Value,
    ) -> Result<Value, OperationError> {
        tokio::time::timeout(
            self.http_total_timeout,
            self.invoke_http_within_deadline(canonical, operation, input),
        )
        .await
        .map_err(|_| unavailable())?
    }

    async fn invoke_http_within_deadline(
        &self,
        canonical: &str,
        operation: &connector_resolve::document::Operation,
        input: Value,
    ) -> Result<Value, OperationError> {
        let origin = self.origin(canonical).ok_or_else(unavailable)?;
        let credentials = if canonical.starts_with("knowledge-") {
            vec![Assembled::new(
                ONTOLOGY_BEARER_CREDENTIAL,
                read_bearer(
                    self.config
                        .ontology_bearer_file
                        .as_deref()
                        .ok_or_else(not_granted)?,
                )?,
                CredentialPlacement::Header {
                    name: "Authorization",
                    prefix: "Bearer ",
                },
            )]
        } else {
            Vec::new()
        };
        let plan =
            connector_resolve::resolve(operation, &origin, &input, &BTreeMap::new(), &credentials)
                .map_err(|_| invalid())?;
        let method = reqwest::Method::from_bytes(plan.request.method.as_bytes())
            .map_err(|_| unavailable())?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| unavailable())?;
        same_origin(&origin, &target)
            .then_some(())
            .ok_or_else(not_granted)?;
        let mut outbound = self.client.request(method, target);
        for (name, value) in plan.request.headers {
            outbound = outbound.header(name, value);
        }
        if let Some(body) = plan.request.body {
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
        if canonical.starts_with("knowledge-") {
            self.config.ontology_origin()
        } else if canonical.starts_with("work-") {
            self.config.work_origin()
        } else {
            None
        }
    }
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
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            operations: true,
            connections: true,
            events: false,
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
                Ok(ConnectionResult::Describe(ConnectionDescription {
                    summary: self.lifecycle_connection(),
                    channels: Vec::new(),
                }))
            }
            _ => Err(ConnectionError::new(
                ConnectionErrorCode::NotFound,
                "B10x Integration Connection was not found",
                false,
            )),
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

fn read_bearer(path: &Path) -> Result<String, OperationError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| not_granted())?;
    let metadata = file.metadata().map_err(|_| not_granted())?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > MAX_BEARER_BYTES
    {
        return Err(not_granted());
    }
    let mut value = String::new();
    (&mut file)
        .take(MAX_BEARER_BYTES + 1)
        .read_to_string(&mut value)
        .map_err(|_| not_granted())?;
    let value = value.trim_end_matches(['\r', '\n']);
    if !(32..=MAX_BEARER_BYTES as usize).contains(&value.len())
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
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
mod tests {
    use super::*;
    use connectors_config::{B10xConnectionConfig, InitiationConfig};
    use protocol::operation::{DescribeRequest, InvokeRequest, OwnerContext, SearchRequest};
    use std::fs;
    use std::io::Write as _;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    fn config() -> B10xIntegrationConfig {
        B10xIntegrationConfig {
            connection: B10xConnectionConfig {
                connection_ref: "connection-b10x".to_owned(),
                label: "B10x local".to_owned(),
                grant_ref: "grant-b10x".to_owned(),
                initiation: InitiationConfig::B10x,
            },
            work_origin: Some("http://127.0.0.1:4180".to_owned()),
            ontology_origin: None,
            ontology_bearer_file: None,
            audio: None,
            browser: None,
        }
    }

    fn principal() -> PrincipalContext {
        PrincipalContext::local(&OwnerContext {
            tenant_id: "tenant-test".to_owned(),
            agent_id: "agent-test".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot-test".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        })
        .unwrap()
    }

    fn fake_http(response_body: &'static str) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let task = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let expected = loop {
                let mut buffer = [0_u8; 2048];
                let read = stream.read(&mut buffer).unwrap();
                assert_ne!(read, 0, "HTTP request ended before its declared body");
                request.extend_from_slice(&buffer[..read]);
                assert!(request.len() <= 64 * 1024);
                let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = std::str::from_utf8(&request[..header_end]).unwrap();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                break header_end + 4 + content_length;
            };
            while request.len() < expected {
                let mut buffer = [0_u8; 2048];
                let read = stream.read(&mut buffer).unwrap();
                assert_ne!(read, 0, "HTTP request ended before its declared body");
                request.extend_from_slice(&buffer[..read]);
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                response_body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8(request).unwrap()
        });
        (origin, task)
    }

    fn stalling_http(delay: Duration) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let task = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(delay);
        });
        (origin, task)
    }

    async fn invoke_operation(
        backend: &B10xBackend,
        operation_ref: &str,
        input: Value,
        approval_evidence_ref: Option<&str>,
    ) -> Result<OperationResult, OperationError> {
        let OperationResult::Describe(description) = backend
            .handle(
                &principal(),
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: operation_ref.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("description expected")
        };
        backend
            .handle(
                &principal(),
                OperationRequest::Invoke(InvokeRequest {
                    operation_ref: operation_ref.to_owned(),
                    connection_ref: "connection-b10x".to_owned(),
                    description_ref: description.description_ref,
                    input,
                    approval_evidence_ref: approval_evidence_ref.map(ToOwned::to_owned),
                }),
            )
            .await
    }

    async fn invoke_read(backend: &B10xBackend, operation_ref: &str, input: Value) -> Value {
        let OperationResult::Invoke(result) = invoke_operation(backend, operation_ref, input, None)
            .await
            .unwrap()
        else {
            panic!("invocation expected")
        };
        result.output
    }

    fn audit_outcomes(root: &Path) -> Vec<String> {
        fs::read_to_string(root.join("b10x-operation-audit.jsonl"))
            .unwrap()
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line).unwrap()["event"]["outcome"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect()
    }

    #[tokio::test]
    async fn search_projects_only_configured_capabilities() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = B10xBackend::personal(config(), principal(), temporary.path()).unwrap();
        let OperationResult::Search { operations } = backend
            .handle(
                &principal(),
                OperationRequest::Search(SearchRequest {
                    query: String::new(),
                    limit: 25,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("search result expected")
        };
        assert_eq!(operations.len(), 8);
        assert!(operations
            .iter()
            .all(|operation| operation.operation_ref.starts_with("work.")));
        assert!(operations
            .iter()
            .all(|operation| operation.connections.len() == 1));

        let ConnectionResult::Search { connections } = backend
            .handle_connection(
                &principal(),
                ConnectionRequest::Search(protocol::connection::SearchRequest {
                    query: "b10x".to_owned(),
                    limit: 64,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("Connection search result expected")
        };
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].integration_ref, PROVIDER);
        assert_eq!(connections[0].state, ConnectionState::Callable);
        assert_eq!(connections[0].route, ConnectionRoute::Direct);
    }

    #[test]
    fn browser_catalog_symbol_is_translated_into_the_closed_driver_input() {
        let document = Document::parse(DOCUMENT).unwrap();
        let operation = document.operation(BROWSER_OPEN_OPERATION).unwrap();
        assert_eq!(
            operation.input_schema()["required"],
            serde_json::json!(["url_2"])
        );
        assert_eq!(
            browser_open_input(serde_json::json!({"url_2":"https://example.com"}))
                .unwrap()
                .url
                .as_deref(),
            Some("https://example.com")
        );
        assert!(browser_open_input(serde_json::json!({"url":"https://example.com"})).is_err());
        assert!(browser_goto_input(serde_json::json!({"url_2":7})).is_err());
    }

    #[test]
    fn a_mutating_post_dispatch_failure_is_not_declared_retriable() {
        let document = Document::parse(DOCUMENT).unwrap();
        let operation = document.operation("work-request-create").unwrap();
        let error = post_dispatch_error(operation, unavailable());
        assert_eq!(error.code, OperationErrorCode::OutcomeUnknown);
        assert!(!error.retriable);
    }

    #[tokio::test]
    async fn work_invocation_crosses_the_private_http_boundary_without_a_credential() {
        let (origin, server) = fake_http(r#"{"items":[],"next_cursor":null}"#);
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut configured = config();
        configured.work_origin = Some(origin);
        let backend =
            B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
        let output = invoke_read(
            &backend,
            "work.requests.list",
            serde_json::json!({"cursor":"", "limit":1}),
        )
        .await;
        assert_eq!(output, serde_json::json!({"items":[], "next_cursor":null}));
        let request = server.join().unwrap();
        assert!(request.starts_with("GET /api/work/v1/requests?"));
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        assert_eq!(audit_outcomes(temporary.path()), ["attempted", "completed"]);
    }

    #[tokio::test]
    async fn copied_static_approval_text_cannot_authorize_an_effect() {
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = B10xBackend::personal(config(), principal(), temporary.path()).unwrap();

        let missing = invoke_operation(
            &backend,
            "work.requests.create",
            serde_json::json!({}),
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(missing.code, OperationErrorCode::ApprovalRequired);

        let copied = invoke_operation(
            &backend,
            "work.requests.create",
            serde_json::json!({}),
            Some("approval-policy:deployment:copied-from-config"),
        )
        .await
        .unwrap_err();
        assert_eq!(copied.code, OperationErrorCode::ApprovalDenied);
        assert!(!temporary
            .path()
            .join("b10x-operation-audit.jsonl")
            .exists());
    }

    #[tokio::test]
    async fn invalid_post_dispatch_output_is_audited_as_indeterminate() {
        let (origin, server) = fake_http(r#"{}"#);
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut configured = config();
        configured.work_origin = Some(origin);
        let backend =
            B10xBackend::personal(configured, principal(), temporary.path()).unwrap();

        let error = invoke_operation(
            &backend,
            "work.requests.list",
            serde_json::json!({"cursor":"", "limit":1}),
            None,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::Unavailable);
        server.join().unwrap();
        assert_eq!(
            audit_outcomes(temporary.path()),
            ["attempted", "indeterminate"]
        );
    }

    #[tokio::test]
    async fn total_http_deadline_bounds_a_stalled_private_service() {
        let (origin, server) = stalling_http(Duration::from_millis(250));
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let mut configured = config();
        configured.work_origin = Some(origin);
        let mut backend =
            B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
        backend.client = http_client(Duration::from_millis(25), Duration::from_millis(50)).unwrap();
        backend.http_total_timeout = Duration::from_millis(50);

        let operation = backend.document.operation("work-request-list").unwrap();
        let started = Instant::now();
        let error = backend
            .invoke_http(
                "work-request-list",
                operation,
                serde_json::json!({"cursor":"", "limit":1}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::Unavailable);
        let elapsed = started.elapsed();
        assert!(elapsed < Duration::from_millis(200), "elapsed {elapsed:?}");
        server.join().unwrap();
    }

    #[tokio::test]
    async fn ontology_invocation_reads_the_owner_only_bearer_at_call_time() {
        let (origin, server) = fake_http(r#"{"claims":[],"truncated":false}"#);
        let temporary = tempfile::tempdir().unwrap();
        fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let bearer = temporary.path().join("ontology.bearer");
        let mut bearer_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&bearer)
            .unwrap();
        bearer_file
            .write_all(b"SENTINEL-NOT-A-REAL-ONTOLOGY-BEARER\n")
            .unwrap();
        drop(bearer_file);
        let mut configured = config();
        configured.work_origin = None;
        configured.ontology_origin = Some(origin);
        configured.ontology_bearer_file = Some(bearer);
        let backend =
            B10xBackend::personal(configured, principal(), temporary.path()).unwrap();
        let output = invoke_read(
            &backend,
            "knowledge.query",
            serde_json::json!({
                "branches": ["main"],
                "limit": 10,
                "predicate": null,
                "subject": null
            }),
        )
        .await;
        assert_eq!(output, serde_json::json!({"claims":[], "truncated":false}));
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/query HTTP/1.1\r\n"));
        assert!(request.contains("SENTINEL-NOT-A-REAL-ONTOLOGY-BEARER"));
        let audit =
            fs::read_to_string(temporary.path().join("b10x-operation-audit.jsonl")).unwrap();
        assert!(!audit.contains("SENTINEL-NOT-A-REAL-ONTOLOGY-BEARER"));
    }

    #[test]
    fn every_projected_operation_has_a_response_schema() {
        let catalog: Value = serde_json::from_str(DOCUMENT).unwrap();
        for (canonical, _, _) in all_operation_rows() {
            assert!(response_schema(&catalog, canonical).is_ok(), "{canonical}");
        }
    }

    #[test]
    fn ontology_nullable_fields_are_still_strict_after_catalog_lowering() {
        assert!(validate_semantic_input(
            "knowledge-query",
            &serde_json::json!({
                "branches": ["main"],
                "limit": 10,
                "predicate": null,
                "subject": "entity:one"
            }),
        )
        .is_ok());
        for invalid in [
            serde_json::json!({
                "branches": ["main"], "limit": 10, "predicate": {}, "subject": null
            }),
            serde_json::json!({
                "branches": ["main", "main"], "limit": 10, "predicate": null, "subject": null
            }),
            serde_json::json!({
                "branches": [], "limit": 1.5, "predicate": null, "subject": null
            }),
            serde_json::json!({
                "branches": [], "limit": 10, "predicate": null, "subject": null, "origin": "caller-selected"
            }),
        ] {
            assert!(validate_semantic_input("knowledge-query", &invalid).is_err());
        }
    }
}
