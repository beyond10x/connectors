use std::collections::BTreeSet;

use connector_resolve::document::{
    ImplementationForm, InteractionShape, Operation, PlacementRequirement, ProtocolRequestTemplate,
    RequiredCapability,
};
use domain::{
    AdmittedOperation, Capability, DriverId, HttpPlan, Implementation, Interaction, OperationFacts,
    Placement, ProtocolPlan, SipPlan, ZeroIoPlan,
};

/// Deployment facts consulted during pure planning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningEnvironment {
    pub available_drivers: BTreeSet<DriverId>,
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

    let protocol = match &operation.request {
        ProtocolRequestTemplate::HttpV1(request) => ProtocolPlan::HttpV1(HttpPlan {
            method: request.method.clone(),
            url_template: request.url.clone(),
        }),
        ProtocolRequestTemplate::SipV1 => ProtocolPlan::SipV1(SipPlan {
            connection: admission.connection().to_owned(),
        }),
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

    fn admission() -> AdmittedOperation {
        AdmittedOperation::from_grant_decision(
            "acme",
            "acme-call",
            "principal-1",
            "grant-1",
            "connection-1",
        )
    }

    fn environment(driver: DriverId) -> PlanningEnvironment {
        PlanningEnvironment {
            available_drivers: BTreeSet::from([driver]),
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
}
