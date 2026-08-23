#![forbid(unsafe_code)]

//! Runtime composition for B10x-owned Connector capabilities.
//!
//! This Integration is the only runtime adapter that joins the reviewed B10x catalog to the
//! closed local audio/CDP drivers and to deployment-owned private Work/Ontology/Planner origins. Agent sees
//! only the credential-free operation protocol. It cannot select a driver, executable, profile,
//! voice, filesystem path, HTTP origin, bearer, or placement.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connector_resolve::document::{Document, ProtocolDriver};
use connectors_config::B10xIntegrationConfig;
use domain::{AdmittedOperation, Capability, ConnectionAuthority, DriverId};
use driver_cdp::LocalBrowserDriver;
use driver_speech::{LocalSpeechDriver, SpeechCancellation, SpeechEngine as _};
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
use protocol::datasource::{
    AccessMode, BindingSearchRequest, Completeness, DatasourceBinding, DatasourceDescription,
    DatasourceError, DatasourceErrorCode, DatasourcePage, DatasourceProvenance, DatasourceRead,
    DatasourceRecord, DatasourceRequest, DatasourceResult, DatasourceSummary,
    DescribeRequest as DatasourceDescribeRequest, ReadRequest, ReadVerb, RecordView,
    SearchRequest as DatasourceSearchRequest,
};
use protocol::event::{
    ChannelSummary, DataEvent, EventError, EventErrorCode, EventProvenance, EventRequest,
    EventResult,
};
use protocol::operation::{
    ConnectionSummary, InvocationResult, InvokeRequest, OperationError, OperationErrorCode,
    OperationRequest, OperationResult,
};
use serde_json::Value;
use service::{
    admit_audio_plan, admit_browser_address, admit_browser_plan, admit_speech_speak,
    plan_operation, BackendCapabilities, ConnectorBackend, PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};

mod audit;
mod composition;
mod datasource;
mod module_signing;
mod policy;
mod surface;
mod transport;
mod work_events;

use audit::{AuditEvent, AuditJournal};
use module_signing::ModuleSigner;
use policy::{
    all_operation_rows, approval, effect, module_operation, operation_row, post_dispatch_error,
    response_schema,
};
use transport::{module_client, module_id, module_origin};
use work_events::ModuleEventStore;

const PROVIDER: &str = "b10x";
const WORKSPACES_DATASOURCE: &str = "b10x.workspaces";
const DOCUMENT: &str = include_str!("../../../catalog/b10x.catalog.json");
use surface::{
    ResolvedOperation, WorkOwnerEventPage, HTTP_CONNECT_TIMEOUT, HTTP_TOTAL_TIMEOUT, OPERATIONS,
    PLANNER_EVENT_BINDING, PLANNER_EVENT_CHANNEL, WORK_EVENT_BINDING, WORK_EVENT_CHANNEL,
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
    Exact(Box<PrincipalContext>),
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
            PrincipalAdmission::Exact(expected) => expected.as_ref() == context,
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
                self.module_admitted("ontology") && self.config.module_configured("ontology")
            }
            value if value.starts_with("ontology-") => {
                self.module_admitted("ontology") && self.config.module_configured("ontology")
            }
            value if value.starts_with("work-") => {
                self.module_admitted("work") && self.config.module_configured("work")
            }
            value if value.starts_with("planner-") => {
                self.module_admitted("planner") && self.config.module_configured("planner")
            }
            value if value.starts_with("workspaces-") || value.starts_with("workspace-") => {
                self.module_admitted("workspaces") && self.config.module_configured("workspaces")
            }
            value if value.starts_with("colab-") => {
                self.module_admitted("colab") && self.config.module_configured("colab")
            }
            _ => false,
        }
    }

    fn module_admitted(&self, module: &str) -> bool {
        matches!(&self.admission, PrincipalAdmission::Exact(_))
            || self.config.tenant_member_module_enabled(module)
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
            purpose: None,
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
            scope: None,
            actor: None,
            auth_profile: None,
        }
    }

    fn description_ref(&self, context: &PrincipalContext, canonical: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(self.catalog_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(self.deployment_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(canonical.as_bytes());
        digest.update(b"\0");
        digest.update(self.config.connection.grant_ref.as_bytes());
        format!("description-sha256-{:x}", digest.finalize())
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        let ResolvedOperation {
            contract: operation,
            canonical,
            ..
        } = self
            .operation(&request.operation_ref)
            .ok_or_else(not_found)?;
        if canonical == BROWSER_GOTO_OPERATION
            || (canonical == BROWSER_OPEN_OPERATION
                && request
                    .input
                    .get("url_2")
                    .is_some_and(|value| !value.is_null()))
        {
            return Err(OperationError::new(
                OperationErrorCode::Unavailable,
                "browser network navigation is disabled until Connection-bound post-DNS egress confinement is available",
                false,
            ));
        }
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
        // Writes and the named device operations describe `ApprovalPosture::Required`; the
        // demanded approval is verified and spent upstream by the sealed proof chain (S-045,
        // S-046) on the hosted route, and a personal placement is the owner's own local
        // admission. No local reading of the evidence reference decides admission (S-047).
        validate_json(operation.input_schema(), &request.input).map_err(|_| invalid())?;
        validate_semantic_input(canonical, &request.input)?;
        let plan = self.plan(context, operation, canonical)?;
        let audit_ref = opaque_ref("audit")?;
        let audit = AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            tenant_id: context.tenant_id(),
            subject: context.subject(),
            actor_subject: context.actor_subject(),
            issuer: context.issuer(),
            token_id: context.token_id(),
            deployment_id: context.deployment_id(),
            request_id: context.request_id(),
            trace_id: context.trace_id(),
            authority_snapshot_id: context.authority_snapshot_id(),
            authority_snapshot_sha256: context.authority_snapshot_sha256(),
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
                subject: context.subject(),
                actor_subject: context.actor_subject(),
                issuer: context.issuer(),
                token_id: context.token_id(),
                deployment_id: context.deployment_id(),
                request_id: context.request_id(),
                trace_id: context.trace_id(),
                authority_snapshot_id: context.authority_snapshot_id(),
                authority_snapshot_sha256: context.authority_snapshot_sha256(),
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
        // The hosted registry serves this backend too, so read-only operations that pass the
        // hosted fence reach this owner assertion for a hosted principal. S-046 replaces it
        // with a GrantDecision on that path.
        let admission = AdmittedOperation::for_local_owner(
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
                let module = module_id(canonical).ok_or_else(not_granted)?;
                let local_socket = self.config.module_socket(module);
                let mut capabilities = if local_socket.is_some() {
                    // The reviewed catalog classifies module HTTP as private-network I/O. The
                    // local adapter additionally proves that dispatch is constrained to one Unix
                    // socket; it retains the catalog capability so the common zero-I/O planner
                    // can validate the same operation contract in both placements.
                    BTreeSet::from([Capability::PrivateNetwork, Capability::UnixSocket])
                } else {
                    BTreeSet::from([Capability::PrivateNetwork])
                };
                // Every private module request is signed with the deployment-owned key loaded
                // from its projected file. The plan must account for that file-secret authority,
                // not just Ontology's separate bearer-file case.
                if local_socket.is_none() && module_operation(canonical).is_some() {
                    capabilities.insert(Capability::FileSecret);
                }
                (
                    BTreeSet::from([DriverId::HttpV1]),
                    capabilities,
                    vec![local_socket.map_or(origin, |path| path.display().to_string())],
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
                let mut engine = driver_speech::engine_for(&admitted);
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
        let origin = self.origin(canonical).ok_or_else(|| {
            module_unavailable(
                canonical,
                "no module origin is configured for this operation",
            )
        })?;
        let credentials = Vec::new();
        let plan =
            connector_resolve::resolve(operation, &origin, &input, &BTreeMap::new(), &credentials)
                .map_err(|_| invalid())?;
        let method = reqwest::Method::from_bytes(plan.request.method.as_bytes()).map_err(|_| {
            module_unavailable(canonical, "the operation declares an unusable HTTP method")
        })?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| {
            module_unavailable(canonical, "the operation resolved an unusable request URL")
        })?;
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
        let audience =
            if canonical.starts_with("workspaces-") || canonical.starts_with("workspace-") {
                "urn:b10x:module:workspaces"
            } else if canonical.starts_with("colab-") {
                "urn:b10x:module:colab"
            } else if canonical.starts_with("work-") {
                "urn:b10x:module:work"
            } else if canonical.starts_with("planner-") {
                "urn:b10x:module:planner"
            } else {
                "urn:b10x:module:ontology"
            };
        let module = module_id(canonical).ok_or_else(not_granted)?;
        let client = self.module_client(module).map_err(|_| {
            module_unavailable(canonical, "the module transport could not be opened")
        })?;
        let mut outbound = client.request(method, target);
        if self.config.module_socket(module).is_none() {
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
            outbound = outbound.header(reqwest::header::AUTHORIZATION, authorization);
        }
        for (name, value) in plan.request.headers {
            outbound = outbound.header(name, value);
        }
        if !body.is_empty() {
            outbound = outbound.body(body);
        }
        let mut response = outbound.send().await.map_err(|_| {
            module_unavailable(
                canonical,
                "the module did not answer; it may be unreachable or still starting",
            )
        })?;
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
            return Err(module_unavailable(
                canonical,
                &format!(
                    "the module answered with status {}",
                    response.status().as_u16()
                ),
            ));
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
        module_origin(&self.config, canonical)
    }

    fn module_client(&self, module: &str) -> Result<reqwest::Client, B10xIntegrationError> {
        module_client(
            &self.config,
            &self.client,
            module,
            HTTP_CONNECT_TIMEOUT,
            HTTP_TOTAL_TIMEOUT,
        )
    }

    async fn refresh_work_events(&self, context: &PrincipalContext) -> Result<(), EventError> {
        let origin = self
            .config
            .work_origin()
            .or_else(|| {
                self.config
                    .module_socket("work")
                    .map(|_| "http://localhost".to_owned())
            })
            .ok_or_else(event_not_granted)?;
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
        let client = self
            .module_client("work")
            .map_err(|_| event_unavailable())?;
        let mut request = client.get(url);
        if self.config.module_socket("work").is_none() {
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
            request = request.header(reqwest::header::AUTHORIZATION, authorization);
        }
        let response = request.send().await.map_err(|_| event_unavailable())?;
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
        let origin = self
            .config
            .planner_origin()
            .or_else(|| {
                self.config
                    .module_socket("planner")
                    .map(|_| "http://localhost".to_owned())
            })
            .ok_or_else(event_not_granted)?;
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
        let client = self
            .module_client("planner")
            .map_err(|_| event_unavailable())?;
        let mut request = client.get(url);
        if self.config.module_socket("planner").is_none() {
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
            request = request.header(reqwest::header::AUTHORIZATION, authorization);
        }
        let response = request.send().await.map_err(|_| event_unavailable())?;
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
            events: self.config.module_configured("work")
                || self.config.module_configured("planner"),
            datasources: self.workspace_datasource_admitted(),
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
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => false,
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

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        match request {
            DatasourceRequest::Describe(request) => request.datasource_ref == WORKSPACES_DATASOURCE,
            DatasourceRequest::Bindings(request) => request.datasource_ref == WORKSPACES_DATASOURCE,
            DatasourceRequest::Read(request) => request.datasource_ref == WORKSPACES_DATASOURCE,
            DatasourceRequest::Search(_) => false,
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
            | OperationRequest::SessionReconcile(_)
            | OperationRequest::SessionSignal(_) => Err(not_found()),
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

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.handle_workspace_datasource(context, request).await
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

/// A refusal that names the operation and what actually failed. A single opaque
/// "connector runtime is unavailable" for every branch of module dispatch left both people and
/// models guessing between "not configured", "unreachable", and "the module said no".
fn module_unavailable(canonical: &str, reason: &str) -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        format!("{canonical}: {reason}"),
        true,
    )
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
