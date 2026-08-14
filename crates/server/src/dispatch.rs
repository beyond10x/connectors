use domain::{DriverId, ZeroIoPlan};

/// One value that may contain credential material and never reveals itself through `Debug`.
#[derive(Clone, PartialEq, Eq)]
pub struct SensitiveValue(String);

impl SensitiveValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SensitiveValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("<redacted>")
    }
}

/// Operation-scoped credentials prepared only after planning admitted capabilities and driver.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct CredentialSet(Vec<SensitiveValue>);

impl CredentialSet {
    pub fn new(values: Vec<SensitiveValue>) -> Self {
        Self(values)
    }

    pub fn values(&self) -> &[SensitiveValue] {
        &self.0
    }
}

impl std::fmt::Debug for CredentialSet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CredentialSet")
            .field("values", &format_args!("<redacted:{}>", self.0.len()))
            .finish()
    }
}

/// Shared enforcement applied once before any built-in driver receives its plan.
pub trait DispatchPolicy: Send + Sync {
    fn admit_egress(&self, plan: &ZeroIoPlan) -> Result<(), String>;
    fn register_redactions(&self, credentials: &CredentialSet) -> Result<(), String>;
}

/// Connector audit sink owned by the composition layer, not individual drivers.
pub trait AuditSink: Send + Sync {
    fn admitted(&self, plan: &ZeroIoPlan) -> Result<(), String>;
    fn completed(&self, plan: &ZeroIoPlan, result: &DispatchResult) -> Result<(), String>;
}

/// Closed driver boundary. Implementations receive only a fully admitted plan and prepared
/// operation-scoped credentials.
pub trait BuiltInDriver: Send + Sync {
    fn driver(&self) -> DriverId;
    fn dispatch(
        &self,
        plan: &ZeroIoPlan,
        credentials: &CredentialSet,
    ) -> Result<DispatchResult, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchResult {
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DispatchError {
    #[error("driver `{0}` is not installed")]
    DriverUnavailable(&'static str),
    #[error("driver registry slot `{slot}` contains `{actual}`")]
    DriverSlotMismatch {
        slot: &'static str,
        actual: &'static str,
    },
    #[error("egress policy refused dispatch: {0}")]
    Egress(String),
    #[error("redaction registration failed: {0}")]
    Redaction(String),
    #[error("audit failed: {0}")]
    Audit(String),
    #[error("driver failed: {0}")]
    Driver(String),
}

/// One closed, non-plugin registry and the sole dispatch composition point.
pub struct Dispatcher<'a> {
    pub http_v1: Option<&'a dyn BuiltInDriver>,
    pub sip_v1: Option<&'a dyn BuiltInDriver>,
    pub policy: &'a dyn DispatchPolicy,
    pub audit: &'a dyn AuditSink,
}

impl Dispatcher<'_> {
    pub fn dispatch(
        &self,
        plan: &ZeroIoPlan,
        credentials: &CredentialSet,
    ) -> Result<DispatchResult, DispatchError> {
        let selected = plan.protocol().driver();
        let driver = match selected {
            DriverId::HttpV1 => self.http_v1,
            DriverId::SipV1 => self.sip_v1,
        }
        .ok_or(DispatchError::DriverUnavailable(selected.as_str()))?;
        if driver.driver() != selected {
            return Err(DispatchError::DriverSlotMismatch {
                slot: selected.as_str(),
                actual: driver.driver().as_str(),
            });
        }

        self.policy
            .admit_egress(plan)
            .map_err(DispatchError::Egress)?;
        self.policy
            .register_redactions(credentials)
            .map_err(DispatchError::Redaction)?;
        self.audit.admitted(plan).map_err(DispatchError::Audit)?;
        let result = driver
            .dispatch(plan, credentials)
            .map_err(DispatchError::Driver)?;
        self.audit
            .completed(plan, &result)
            .map_err(DispatchError::Audit)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Mutex;

    use domain::{
        AdmittedOperation, Capability, Implementation, Interaction, OperationFacts, Placement,
        ProtocolPlan, SipPlan,
    };

    use super::*;

    struct RecordingPolicy(Mutex<Vec<&'static str>>);

    impl DispatchPolicy for RecordingPolicy {
        fn admit_egress(&self, _: &ZeroIoPlan) -> Result<(), String> {
            self.0.lock().unwrap().push("egress");
            Ok(())
        }

        fn register_redactions(&self, _: &CredentialSet) -> Result<(), String> {
            self.0.lock().unwrap().push("redaction");
            Ok(())
        }
    }

    impl AuditSink for RecordingPolicy {
        fn admitted(&self, _: &ZeroIoPlan) -> Result<(), String> {
            self.0.lock().unwrap().push("audit.admitted");
            Ok(())
        }

        fn completed(&self, _: &ZeroIoPlan, _: &DispatchResult) -> Result<(), String> {
            self.0.lock().unwrap().push("audit.completed");
            Ok(())
        }
    }

    struct SipDriver<'a>(&'a Mutex<Vec<&'static str>>);

    impl BuiltInDriver for SipDriver<'_> {
        fn driver(&self) -> DriverId {
            DriverId::SipV1
        }

        fn dispatch(&self, _: &ZeroIoPlan, _: &CredentialSet) -> Result<DispatchResult, String> {
            self.0.lock().unwrap().push("driver");
            Ok(DispatchResult {
                code: "session_established".to_owned(),
            })
        }
    }

    fn plan() -> ZeroIoPlan {
        ZeroIoPlan::new(
            OperationFacts {
                provider: "acme".to_owned(),
                operation: "acme-call".to_owned(),
                service: "default".to_owned(),
                interaction: Interaction::SessionEstablishment,
                placement: Placement::ConnectorsDeployment,
                implementation: Implementation::BuiltIn,
                required_capabilities: BTreeSet::from([Capability::PublicNetwork]),
                permission_subjects: vec!["public:pbx.example".to_owned()],
            },
            AdmittedOperation::from_grant_decision(
                "acme",
                "acme-call",
                "org",
                "principal",
                "grant",
                "connection",
            ),
            ProtocolPlan::SipV1(SipPlan {
                connection: "connection".to_owned(),
            }),
        )
    }

    #[test]
    fn composition_order_is_policy_redaction_audit_driver_audit() {
        let calls = Mutex::new(Vec::new());
        let policy = RecordingPolicy(Mutex::new(Vec::new()));
        let driver = SipDriver(&calls);
        let dispatcher = Dispatcher {
            http_v1: None,
            sip_v1: Some(&driver),
            policy: &policy,
            audit: &policy,
        };
        let credentials = CredentialSet::new(vec![SensitiveValue::new("SENTINEL")]);
        dispatcher
            .dispatch(&plan(), &credentials)
            .expect("dispatch succeeds");
        assert_eq!(
            *policy.0.lock().unwrap(),
            ["egress", "redaction", "audit.admitted", "audit.completed"]
        );
        assert_eq!(*calls.lock().unwrap(), ["driver"]);
        assert!(!format!("{credentials:?}").contains("SENTINEL"));
    }
}
