use std::collections::BTreeSet;

use connector_resolve::document::{
    ImplementationForm, InteractionShape, Operation, PlacementRequirement, ProtocolRequestTemplate,
    RequiredCapability,
};
use domain::{
    AdmittedOperation, AudioPlan, BrowserPlan, Capability, ConnectionInitiator, ConnectionRoute,
    DriverId, HttpPlan, Implementation, Interaction, MediatedHttpPlan, OperationFacts, Placement,
    ProtocolPlan, RouteAdapter, SipPlan, ZeroIoPlan,
};

/// Deployment facts consulted during pure planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningEnvironment {
    pub available_drivers: BTreeSet<DriverId>,
    /// Closed mediated-route adapters installed at this placement.
    pub available_route_adapters: BTreeSet<RouteAdapter>,
    pub capabilities: BTreeSet<Capability>,
    /// Reviewed, deployment-selected egress/listener subjects. Caller input never populates this.
    pub permission_subjects: Vec<String>,
}

/// Named refusal from zero-I/O planning.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("admission is for `{admitted}` but catalog operation is `{catalog}`")]
    OperationMismatch { admitted: String, catalog: String },
    #[error("admission is for provider `{admitted}` but catalog provider is `{catalog}`")]
    ProviderMismatch { admitted: String, catalog: String },
    #[error("driver `{0}` is not available in this deployment")]
    DriverUnavailable(&'static str),
    #[error("required capability `{0}` is not admitted in this deployment")]
    CapabilityUnavailable(&'static str),
    #[error("operation has no reviewed permission subject")]
    PermissionSubjectMissing,
    #[error("Connection does not permit the platform to initiate operations")]
    ConnectionInitiationRefused,
    #[error("mediated route adapter `{0}` is not available in this deployment")]
    RouteAdapterUnavailable(&'static str),
    #[error("a mediated Connection cannot execute this protocol driver")]
    MediatedRouteDriverMismatch,
    #[error("a mediated HTTP operation has no target-relative request path")]
    MediatedTargetPathInvalid,
}

/// Produce the complete inert plan after grant admission and before credentials or I/O.
pub fn plan_operation(
    provider: &str,
    operation: &Operation,
    admission: AdmittedOperation,
    environment: &PlanningEnvironment,
) -> Result<ZeroIoPlan, PlanError> {
    if admission.provider() != provider {
        return Err(PlanError::ProviderMismatch {
            admitted: admission.provider().to_owned(),
            catalog: provider.to_owned(),
        });
    }
    if admission.operation() != operation.id {
        return Err(PlanError::OperationMismatch {
            admitted: admission.operation().to_owned(),
            catalog: operation.id.clone(),
        });
    }
    if !admission
        .connection_authority()
        .initiation()
        .allows(ConnectionInitiator::Platform)
    {
        return Err(PlanError::ConnectionInitiationRefused);
    }

    let driver = driver_of(operation);
    if !environment.available_drivers.contains(&driver) {
        return Err(PlanError::DriverUnavailable(driver.as_str()));
    }
    let required_capabilities = operation
        .required_capabilities()
        .iter()
        .copied()
        .map(capability_of)
        .collect::<BTreeSet<_>>();
    for capability in &required_capabilities {
        if !environment.capabilities.contains(capability) {
            return Err(PlanError::CapabilityUnavailable(capability_word(
                *capability,
            )));
        }
    }
    if environment.permission_subjects.is_empty() {
        return Err(PlanError::PermissionSubjectMissing);
    }

    let protocol = match (admission.connection_authority().route(), &operation.request) {
        (ConnectionRoute::Direct, ProtocolRequestTemplate::HttpV1(request)) => {
            ProtocolPlan::HttpV1(HttpPlan {
                method: request.method.clone(),
                url_template: request.url.clone(),
            })
        }
        (ConnectionRoute::Direct, ProtocolRequestTemplate::SipV1) => ProtocolPlan::SipV1(SipPlan {
            connection: admission.connection().to_owned(),
        }),
        (ConnectionRoute::Direct, ProtocolRequestTemplate::AudioV1) => {
            ProtocolPlan::AudioV1(AudioPlan {
                connection: admission.connection().to_owned(),
            })
        }
        (ConnectionRoute::Direct, ProtocolRequestTemplate::CdpV1) => {
            ProtocolPlan::CdpV1(BrowserPlan {
                connection: admission.connection().to_owned(),
            })
        }
        (
            ConnectionRoute::ViaConnection {
                parent_connection,
                resource_binding,
                adapter,
            },
            ProtocolRequestTemplate::HttpV1(request),
        ) => {
            if !environment.available_route_adapters.contains(adapter) {
                return Err(PlanError::RouteAdapterUnavailable(adapter.as_str()));
            }
            let target_path_template = request
                .url
                .strip_prefix("{base}")
                .filter(|path| path.starts_with('/'))
                .ok_or(PlanError::MediatedTargetPathInvalid)?;
            ProtocolPlan::MediatedHttpV1(MediatedHttpPlan {
                method: request.method.clone(),
                target_path_template: target_path_template.to_owned(),
                parent_connection: parent_connection.clone(),
                resource_binding: resource_binding.clone(),
                adapter: *adapter,
            })
        }
        (
            ConnectionRoute::ViaConnection { .. },
            ProtocolRequestTemplate::SipV1
            | ProtocolRequestTemplate::AudioV1
            | ProtocolRequestTemplate::CdpV1,
        ) => {
            return Err(PlanError::MediatedRouteDriverMismatch);
        }
    };
    let facts = OperationFacts {
        provider: provider.to_owned(),
        operation: operation.id.clone(),
        service: operation.service.clone(),
        interaction: interaction_of(operation.interaction_shape()),
        placement: placement_of(operation.placement_requirement()),
        implementation: implementation_of(operation.implementation_form()),
        required_capabilities,
        permission_subjects: environment.permission_subjects.clone(),
    };
    Ok(ZeroIoPlan::new(facts, admission, protocol))
}

fn driver_of(operation: &Operation) -> DriverId {
    match operation.request {
        ProtocolRequestTemplate::HttpV1(_) => DriverId::HttpV1,
        ProtocolRequestTemplate::SipV1 => DriverId::SipV1,
        ProtocolRequestTemplate::AudioV1 => DriverId::AudioV1,
        ProtocolRequestTemplate::CdpV1 => DriverId::CdpV1,
    }
}

fn interaction_of(value: InteractionShape) -> Interaction {
    match value {
        InteractionShape::Unary => Interaction::Unary,
        InteractionShape::Stream => Interaction::Stream,
        InteractionShape::Subscription => Interaction::Subscription,
        InteractionShape::LeasedSession => Interaction::LeasedSession,
        InteractionShape::SessionEstablishment => Interaction::SessionEstablishment,
    }
}

fn placement_of(value: PlacementRequirement) -> Placement {
    match value {
        PlacementRequirement::ConnectorsDeployment => Placement::ConnectorsDeployment,
        PlacementRequirement::SubstrateWorkload => Placement::SubstrateWorkload,
        PlacementRequirement::FederatedSatellite => Placement::FederatedSatellite,
    }
}

fn implementation_of(value: ImplementationForm) -> Implementation {
    match value {
        ImplementationForm::BuiltIn => Implementation::BuiltIn,
    }
}

fn capability_of(value: RequiredCapability) -> Capability {
    match value {
        RequiredCapability::PublicNetwork => Capability::PublicNetwork,
        RequiredCapability::PrivateNetwork => Capability::PrivateNetwork,
        RequiredCapability::UnixSocket => Capability::UnixSocket,
        RequiredCapability::FileSecret => Capability::FileSecret,
        RequiredCapability::Process => Capability::Process,
        RequiredCapability::Container => Capability::Container,
        RequiredCapability::Device => Capability::Device,
    }
}

fn capability_word(value: Capability) -> &'static str {
    match value {
        Capability::PublicNetwork => "public_network",
        Capability::PrivateNetwork => "private_network",
        Capability::UnixSocket => "unix_socket",
        Capability::FileSecret => "file_secret",
        Capability::Process => "process",
        Capability::Container => "container",
        Capability::Device => "device",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use connector_resolve::document::Document;
    use domain::{ConnectionAuthority, InitiationPolicy};

    fn document(driver: &str, shape: &str, request: &str) -> Document {
        Document::parse(&format!(
            r#"{{
                "connector":"acme",
                "services":[{{"name":"default","base_url":"https://api.example"}}],
                "operations":[{{
                    "id":"acme-call","service":"default","expose":false,"params":[],
                    "{driver_key}":"{driver}","request":{request},"endpoint":{{}},
                    "risk":"high","idempotency":"non_idempotent","effects":["write","network"],
                    "interaction_shape":"{shape}","placement_requirement":"connectors_deployment",
                    "implementation_form":"built_in","required_capabilities":["public_network"]
                }}]
            }}"#,
            driver_key = "protocol_driver"
        ))
        .expect("fixture document parses")
    }

    fn admission_with(initiation: InitiationPolicy) -> AdmittedOperation {
        AdmittedOperation::for_local_owner(
            "acme",
            "acme-call",
            "org-1",
            "principal-1",
            "grant-1",
            ConnectionAuthority::new("connection-1", initiation).unwrap(),
        )
    }

    fn admission() -> AdmittedOperation {
        admission_with(InitiationPolicy::platform_only())
    }

    fn environment(driver: DriverId) -> PlanningEnvironment {
        PlanningEnvironment {
            available_drivers: BTreeSet::from([driver]),
            available_route_adapters: BTreeSet::new(),
            capabilities: BTreeSet::from([Capability::PublicNetwork]),
            permission_subjects: vec!["public:api.example".to_owned()],
        }
    }

    #[test]
    fn sip_plan_has_no_http_fields() {
        let document = document("sip_v1", "session_establishment", "{}");
        let operation = document.operation("acme-call").expect("operation");
        let plan = plan_operation(
            "acme",
            operation,
            admission(),
            &environment(DriverId::SipV1),
        )
        .expect("planned");
        assert!(matches!(plan.protocol(), ProtocolPlan::SipV1(_)));
    }

    #[test]
    fn missing_driver_refuses_before_dispatch() {
        let document = document("sip_v1", "session_establishment", "{}");
        let operation = document.operation("acme-call").expect("operation");
        let error = plan_operation(
            "acme",
            operation,
            admission(),
            &environment(DriverId::HttpV1),
        )
        .expect_err("missing SIP driver refuses");
        assert_eq!(error, PlanError::DriverUnavailable("sip_v1"));
    }

    #[test]
    fn provider_only_connection_refuses_a_caller_initiated_operation() {
        let document = document("sip_v1", "session_establishment", "{}");
        let operation = document.operation("acme-call").expect("operation");
        let error = plan_operation(
            "acme",
            operation,
            admission_with(InitiationPolicy::provider_only()),
            &environment(DriverId::SipV1),
        )
        .expect_err("provider-only connection refuses outbound start");
        assert_eq!(error, PlanError::ConnectionInitiationRefused);
    }

    #[test]
    fn bidirectional_connection_still_requires_the_operation_admission() {
        let document = document("sip_v1", "session_establishment", "{}");
        let operation = document.operation("acme-call").expect("operation");
        let plan = plan_operation(
            "acme",
            operation,
            admission_with(InitiationPolicy::bidirectional()),
            &environment(DriverId::SipV1),
        )
        .expect("the platform is one allowed initiator");
        assert_eq!(plan.admission().grant(), "grant-1");
    }

    #[test]
    fn mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter() {
        let document = document(
            "http_v1",
            "unary",
            r#"{"method":"GET","url":"{base}/api/v1/query","headers":{},"query":[]}"#,
        );
        let operation = document.operation("acme-call").expect("operation");
        let admission = AdmittedOperation::for_local_owner(
            "acme",
            "acme-call",
            "org-1",
            "principal-1",
            "child-grant",
            ConnectionAuthority::mediated(
                "prometheus-via-grafana",
                InitiationPolicy::platform_only(),
                "grafana-infra",
                "observation:datasource-1",
                RouteAdapter::GrafanaDatasourceProxyV1,
            )
            .unwrap(),
        );
        let mut environment = environment(DriverId::HttpV1);
        assert_eq!(
            plan_operation("acme", operation, admission.clone(), &environment),
            Err(PlanError::RouteAdapterUnavailable(
                "grafana_datasource_proxy_v1"
            ))
        );

        environment
            .available_route_adapters
            .insert(RouteAdapter::GrafanaDatasourceProxyV1);
        let plan = plan_operation("acme", operation, admission, &environment).unwrap();
        let ProtocolPlan::MediatedHttpV1(http) = plan.protocol() else {
            panic!("expected a mediated HTTP plan")
        };
        assert_eq!(http.target_path_template, "/api/v1/query");
        assert_eq!(http.parent_connection, "grafana-infra");
        assert!(!format!("{http:?}").contains("prometheus.example"));
    }
}
