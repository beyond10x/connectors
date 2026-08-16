//! Application-layer admission of the exact browser facts a `cdp_v1` driver may consume.
//!
//! The same shape as SIP and local-audio admission, for the same reason: the crate that is allowed
//! to spawn a browser and open sockets receives non-serializable evidence it cannot mint itself,
//! and every host fact in that evidence is deployment-owned. A caller supplies at most one ordinary
//! web address; it never supplies — and cannot name — an executable, a profile directory, an
//! artifact directory, a debugging port, or a window.

use std::path::{Path, PathBuf};
use std::time::Duration;

use domain::{DriverId, Interaction, ProtocolPlan, ZeroIoPlan};
use protocol::browser::{
    admit_address, AddressError, BROWSER_CLOSE_OPERATION, BROWSER_GOTO_OPERATION,
    BROWSER_OPEN_OPERATION, BROWSER_SCREENSHOT_OPERATION, BROWSER_SNAPSHOT_OPERATION,
    MAX_SNAPSHOT_NODES,
};

/// Maximum wall-clock time one admitted navigation may occupy the lease.
pub const MAX_NAVIGATION: Duration = Duration::from_secs(120);

/// How many pages one admitted Connection may navigate to across the life of its lease.
pub const MAX_NAVIGATIONS_PER_CONNECTION: u32 = 64;

/// How many screenshots one admitted Connection may write.
pub const MAX_SCREENSHOTS_PER_CONNECTION: u32 = 32;

/// Deployment-selected browser route. No request or model field can construct any part of this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDeploymentRoute {
    /// The Connection this route belongs to.
    pub connection: String,
    /// An explicit absolute browser executable. `None` leaves discovery to the driver, which walks
    /// a closed candidate list on `PATH` and never resolves a symlink it finds.
    pub executable: Option<PathBuf>,
    /// The absolute dedicated profile directory. **Never the operator's own**: a page the driver
    /// visits must hold none of the operator's logged-in sessions, so a page that tries to steer
    /// the agent cannot act as them against their accounts.
    pub user_data_dir: PathBuf,
    /// The absolute directory screenshots are written into for the operator to open.
    pub artifacts_dir: PathBuf,
    /// The admitted per-snapshot node bound.
    pub maximum_nodes: u32,
    /// The admitted wall-clock bound on one navigation.
    pub maximum_navigation: Duration,
}

/// Profile directories that belong to the operator's own browser, never to a driver.
///
/// A dedicated profile is the whole basis of the isolation claim. Admitting one of these would
/// silently hand every logged-in session the operator has to whoever writes the next page.
const OPERATOR_PROFILE_MARKERS: [&str; 5] = [
    "/.config/BraveSoftware",
    "/.config/google-chrome",
    "/.config/chromium",
    "/Library/Application Support/Google/Chrome",
    "/AppData/Local/Google/Chrome/User Data",
];

/// Failure before the browser-capable crate receives a plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BrowserAdmissionError {
    /// The plan is not an admitted browser leased-session operation.
    #[error("operation is not an admitted browser operation")]
    WrongOperation,
    /// Organization, principal, or grant is missing from the admitted plan.
    #[error("admitted browser identity is incomplete")]
    InvalidIdentity,
    /// The deployment route belongs to a different Connection.
    #[error("deployment browser route belongs to another Connection")]
    ConnectionMismatch,
    /// A host path is relative, so what it resolves to would depend on ambient state.
    #[error("browser route path is not absolute")]
    RelativePath,
    /// The profile directory is the operator's own browser profile.
    #[error("browser route profile directory is the operator's own browser profile")]
    OperatorProfileRefused,
    /// The profile and artifact directories are the same directory.
    #[error("browser route writes artifacts into its own profile directory")]
    ArtifactsInsideProfile,
    /// The node bound is zero or above the published ceiling.
    #[error("browser route node bound is outside the published ceiling")]
    InvalidNodeBound,
    /// The navigation deadline is zero or above the published ceiling.
    #[error("browser route has an invalid finite deadline")]
    InvalidDeadline,
    /// The caller's address failed the published grammar.
    #[error("browser address was refused: {0}")]
    AddressRefused(AddressError),
}

/// Check every deployment-owned field before it can reach the browser-capable crate.
///
/// # Errors
///
/// Returns the exact refusal naming the field at fault.
pub fn validate_browser_deployment_route(
    route: &BrowserDeploymentRoute,
) -> Result<(), BrowserAdmissionError> {
    for required in [route.user_data_dir.as_path(), route.artifacts_dir.as_path()] {
        if !required.is_absolute() {
            return Err(BrowserAdmissionError::RelativePath);
        }
    }
    if route
        .executable
        .as_deref()
        .is_some_and(|path: &Path| !path.is_absolute())
    {
        return Err(BrowserAdmissionError::RelativePath);
    }
    let profile = route.user_data_dir.to_string_lossy().replace('\\', "/");
    if OPERATOR_PROFILE_MARKERS
        .iter()
        .any(|marker| profile.contains(&marker.replace('\\', "/")))
    {
        return Err(BrowserAdmissionError::OperatorProfileRefused);
    }
    if route.artifacts_dir == route.user_data_dir
        || route.artifacts_dir.starts_with(&route.user_data_dir)
    {
        return Err(BrowserAdmissionError::ArtifactsInsideProfile);
    }
    if route.maximum_nodes == 0
        || usize::try_from(route.maximum_nodes).unwrap_or(usize::MAX) > MAX_SNAPSHOT_NODES
    {
        return Err(BrowserAdmissionError::InvalidNodeBound);
    }
    if route.maximum_navigation.is_zero() || route.maximum_navigation > MAX_NAVIGATION {
        return Err(BrowserAdmissionError::InvalidDeadline);
    }
    Ok(())
}

/// Join grant admission and deployment-only browser selection into one process-spawning proof.
///
/// # Errors
///
/// Refuses a plan for another driver, another lifecycle, another Connection, an incomplete
/// identity, or a route that fails [`validate_browser_deployment_route`].
pub fn admit_browser_plan(
    plan: &ZeroIoPlan,
    route: BrowserDeploymentRoute,
) -> Result<AdmittedBrowserPlan, BrowserAdmissionError> {
    let ProtocolPlan::CdpV1(browser) = plan.protocol() else {
        return Err(BrowserAdmissionError::WrongOperation);
    };
    // The lifecycle is checked as well as the driver: a browser is a resource held across calls,
    // and a plan claiming any other shape did not come from this catalog surface.
    if plan.protocol().driver() != DriverId::CdpV1
        || plan.facts().interaction != Interaction::LeasedSession
    {
        return Err(BrowserAdmissionError::WrongOperation);
    }
    if !matches!(
        plan.facts().operation.as_str(),
        BROWSER_OPEN_OPERATION
            | BROWSER_GOTO_OPERATION
            | BROWSER_SNAPSHOT_OPERATION
            | BROWSER_SCREENSHOT_OPERATION
            | BROWSER_CLOSE_OPERATION
    ) {
        return Err(BrowserAdmissionError::WrongOperation);
    }
    if plan.admission().organization().is_empty()
        || plan.admission().principal().is_empty()
        || plan.admission().grant().is_empty()
    {
        return Err(BrowserAdmissionError::InvalidIdentity);
    }
    if browser.connection != route.connection || plan.admission().connection() != route.connection {
        return Err(BrowserAdmissionError::ConnectionMismatch);
    }
    validate_browser_deployment_route(&route)?;
    Ok(AdmittedBrowserPlan {
        provider: plan.facts().provider.clone(),
        operation: plan.facts().operation.clone(),
        organization: plan.admission().organization().to_owned(),
        principal: plan.admission().principal().to_owned(),
        grant: plan.admission().grant().to_owned(),
        route,
        _proof: AdmissionProof,
    })
}

/// Admit one address for the two operations that accept one.
///
/// The address is checked against the grammar the catalog publishes before the plan is admitted, so
/// a refused scheme never reaches a browser at all.
///
/// # Errors
///
/// Refuses any operation other than `browser-open`/`browser-goto`, an absent address on
/// `browser-goto`, and any address the published grammar rejects.
pub fn admit_browser_address(
    plan: &ZeroIoPlan,
    url: Option<&str>,
    route: BrowserDeploymentRoute,
) -> Result<AdmittedBrowserPlan, BrowserAdmissionError> {
    match (plan.facts().operation.as_str(), url) {
        (BROWSER_OPEN_OPERATION, None) => {}
        (BROWSER_OPEN_OPERATION | BROWSER_GOTO_OPERATION, Some(url)) => {
            admit_address(url).map_err(BrowserAdmissionError::AddressRefused)?;
        }
        _ => return Err(BrowserAdmissionError::WrongOperation),
    }
    admit_browser_plan(plan, route)
}

/// Non-serializable evidence handed only to the browser-capable `driver-cdp` crate.
pub struct AdmittedBrowserPlan {
    provider: String,
    operation: String,
    organization: String,
    principal: String,
    grant: String,
    route: BrowserDeploymentRoute,
    _proof: AdmissionProof,
}

struct AdmissionProof;

impl AdmittedBrowserPlan {
    /// The Provider this browser operation belongs to.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }

    /// The exact admitted operation id.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// The admitted organization.
    #[must_use]
    pub fn organization(&self) -> &str {
        &self.organization
    }

    /// The admitted principal.
    #[must_use]
    pub fn principal(&self) -> &str {
        &self.principal
    }

    /// The admitted Connector Grant.
    #[must_use]
    pub fn grant(&self) -> &str {
        &self.grant
    }

    /// The deployment-selected browser route.
    #[must_use]
    pub fn route(&self) -> &BrowserDeploymentRoute {
        &self.route
    }
}

impl std::fmt::Debug for AdmittedBrowserPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdmittedBrowserPlan")
            .field("provider", &self.provider)
            .field("operation", &self.operation)
            .field("organization", &self.organization)
            .field("principal", &self.principal)
            .field("grant", &self.grant)
            .field("route", &self.route)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use domain::{
        AdmittedOperation, BrowserPlan, Capability, ConnectionAuthority, Implementation,
        InitiationPolicy, OperationFacts, Placement,
    };

    use super::*;

    fn route() -> BrowserDeploymentRoute {
        BrowserDeploymentRoute {
            connection: "connection-1".to_owned(),
            executable: None,
            user_data_dir: PathBuf::from("/var/lib/b10x/browser/profile"),
            artifacts_dir: PathBuf::from("/var/lib/b10x/browser/artifacts"),
            maximum_nodes: 400,
            maximum_navigation: Duration::from_secs(45),
        }
    }

    fn plan(operation: &str) -> ZeroIoPlan {
        ZeroIoPlan::new(
            OperationFacts {
                provider: "b10x".to_owned(),
                operation: operation.to_owned(),
                service: "default".to_owned(),
                interaction: Interaction::LeasedSession,
                placement: Placement::ConnectorsDeployment,
                implementation: Implementation::BuiltIn,
                required_capabilities: BTreeSet::from([
                    Capability::PublicNetwork,
                    Capability::Process,
                ]),
                permission_subjects: vec!["browser:dedicated-profile".to_owned()],
            },
            AdmittedOperation::from_grant_decision(
                "b10x",
                operation,
                "org",
                "principal-1",
                "grant-1",
                ConnectionAuthority::new("connection-1", InitiationPolicy::b10x_only())
                    .unwrap(),
            ),
            ProtocolPlan::CdpV1(BrowserPlan {
                connection: "connection-1".to_owned(),
            }),
        )
    }

    #[test]
    fn every_admitted_browser_operation_and_its_route_admit_together() {
        for operation in protocol::browser::BROWSER_OPERATIONS {
            let admitted = admit_browser_plan(&plan(operation), route()).expect("admitted");
            assert_eq!(admitted.provider(), "b10x");
            assert_eq!(admitted.operation(), operation);
            assert_eq!(admitted.route().maximum_nodes, 400);
        }
    }

    #[test]
    fn a_plan_for_another_driver_or_operation_is_refused() {
        let foreign = ZeroIoPlan::new(
            plan(BROWSER_OPEN_OPERATION).facts().clone(),
            AdmittedOperation::from_grant_decision(
                "b10x",
                BROWSER_OPEN_OPERATION,
                "org",
                "principal-1",
                "grant-1",
                ConnectionAuthority::new("connection-1", InitiationPolicy::b10x_only())
                    .unwrap(),
            ),
            ProtocolPlan::SipV1(domain::SipPlan {
                connection: "connection-1".to_owned(),
            }),
        );
        assert_eq!(
            admit_browser_plan(&foreign, route()).expect_err("a SIP plan is not a browser plan"),
            BrowserAdmissionError::WrongOperation
        );
        assert_eq!(
            admit_browser_plan(&plan("browser-click"), route())
                .expect_err("interaction is not on this surface"),
            BrowserAdmissionError::WrongOperation
        );
    }

    #[test]
    fn a_unary_lifecycle_is_refused_because_a_browser_spans_calls() {
        let mut facts = plan(BROWSER_SNAPSHOT_OPERATION).facts().clone();
        facts.interaction = Interaction::Unary;
        let unary = ZeroIoPlan::new(
            facts,
            AdmittedOperation::from_grant_decision(
                "b10x",
                BROWSER_SNAPSHOT_OPERATION,
                "org",
                "principal-1",
                "grant-1",
                ConnectionAuthority::new("connection-1", InitiationPolicy::b10x_only())
                    .unwrap(),
            ),
            ProtocolPlan::CdpV1(BrowserPlan {
                connection: "connection-1".to_owned(),
            }),
        );
        assert_eq!(
            admit_browser_plan(&unary, route()).expect_err("a browser lease is not unary"),
            BrowserAdmissionError::WrongOperation
        );
    }

    #[test]
    fn a_route_for_another_connection_is_refused() {
        let mut elsewhere = route();
        elsewhere.connection = "connection-2".to_owned();
        assert_eq!(
            admit_browser_plan(&plan(BROWSER_OPEN_OPERATION), elsewhere)
                .expect_err("another Connection's route is refused"),
            BrowserAdmissionError::ConnectionMismatch
        );
    }

    #[test]
    fn the_operators_own_browser_profile_is_never_an_admitted_route() {
        for operator in [
            "/home/person/.config/BraveSoftware/Brave-Browser",
            "/home/person/.config/google-chrome",
            "/home/person/.config/chromium/Default",
        ] {
            let mut theirs = route();
            theirs.user_data_dir = PathBuf::from(operator);
            assert_eq!(
                validate_browser_deployment_route(&theirs),
                Err(BrowserAdmissionError::OperatorProfileRefused),
                "admitted {operator}"
            );
        }
    }

    #[test]
    fn relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser() {
        let mut relative = route();
        relative.user_data_dir = PathBuf::from("browser/profile");
        assert_eq!(
            validate_browser_deployment_route(&relative),
            Err(BrowserAdmissionError::RelativePath)
        );

        let mut relative_executable = route();
        relative_executable.executable = Some(PathBuf::from("bin/brave"));
        assert_eq!(
            validate_browser_deployment_route(&relative_executable),
            Err(BrowserAdmissionError::RelativePath)
        );

        let mut nested = route();
        nested.artifacts_dir = nested.user_data_dir.join("artifacts");
        assert_eq!(
            validate_browser_deployment_route(&nested),
            Err(BrowserAdmissionError::ArtifactsInsideProfile)
        );

        for nodes in [0, u32::try_from(MAX_SNAPSHOT_NODES).expect("bound") + 1] {
            let mut bound = route();
            bound.maximum_nodes = nodes;
            assert_eq!(
                validate_browser_deployment_route(&bound),
                Err(BrowserAdmissionError::InvalidNodeBound)
            );
        }

        for deadline in [Duration::ZERO, MAX_NAVIGATION + Duration::from_secs(1)] {
            let mut bound = route();
            bound.maximum_navigation = deadline;
            assert_eq!(
                validate_browser_deployment_route(&bound),
                Err(BrowserAdmissionError::InvalidDeadline)
            );
        }
    }

    #[test]
    fn a_non_web_address_is_refused_before_any_browser_is_touched() {
        for refused in ["file:///etc/passwd", "chrome://settings", "javascript:x"] {
            assert_eq!(
                admit_browser_address(&plan(BROWSER_GOTO_OPERATION), Some(refused), route())
                    .map(|_| ())
                    .expect_err("refusal"),
                BrowserAdmissionError::AddressRefused(AddressError::SchemeRefused),
                "admitted {refused}"
            );
        }
        assert!(admit_browser_address(
            &plan(BROWSER_GOTO_OPERATION),
            Some("https://example.test/"),
            route()
        )
        .is_ok());
    }

    #[test]
    fn only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one() {
        assert!(admit_browser_address(&plan(BROWSER_OPEN_OPERATION), None, route()).is_ok());
        assert_eq!(
            admit_browser_address(&plan(BROWSER_GOTO_OPERATION), None, route())
                .map(|_| ())
                .expect_err("goto needs an address"),
            BrowserAdmissionError::WrongOperation
        );
        assert_eq!(
            admit_browser_address(
                &plan(BROWSER_SNAPSHOT_OPERATION),
                Some("https://example.test/"),
                route()
            )
            .map(|_| ())
            .expect_err("snapshot carries no address"),
            BrowserAdmissionError::WrongOperation
        );
    }

    #[test]
    fn admitted_evidence_never_prints_as_a_serializable_route_secret() {
        let admitted =
            admit_browser_plan(&plan(BROWSER_CLOSE_OPERATION), route()).expect("admitted");
        let rendered = format!("{admitted:?}");
        assert!(rendered.contains("AdmittedBrowserPlan"));
        assert!(rendered.contains(".."), "evidence must stay non-exhaustive");
    }
}
