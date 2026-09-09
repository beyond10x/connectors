//! Deterministic illustrated request journeys, not a service or an OAuth verifier.
use serde_json::{json, Value};

pub const FIXTURES: &str = include_str!("../../walkthrough-fixtures.json");

#[derive(Clone)]
struct Step {
    id: &'static str,
    title: &'static str,
    explanation: &'static str,
    from: &'static str,
    to: &'static str,
    message: &'static str,
    credential: &'static str,
    phase: &'static str,
    rule: &'static str,
    payload: Value,
    error: Option<&'static str>,
    provider_call: bool,
    pause: bool,
}
impl Step {
    fn new(
        id: &'static str,
        title: &'static str,
        explanation: &'static str,
        from: &'static str,
        to: &'static str,
        message: &'static str,
    ) -> Self {
        Self {
            id,
            title,
            explanation,
            from,
            to,
            message,
            credential: "none",
            phase: "read",
            rule: "/contracts/service",
            payload: Value::Null,
            error: None,
            provider_call: false,
            pause: false,
        }
    }
    fn credential(mut self, value: &'static str) -> Self {
        self.credential = value;
        self
    }
    fn phase(mut self, value: &'static str) -> Self {
        self.phase = value;
        self
    }
    fn rule(mut self, value: &'static str) -> Self {
        self.rule = value;
        self
    }
    fn payload(mut self, value: Value) -> Self {
        self.payload = value;
        self
    }
    fn json(&self) -> Value {
        json!({"id":self.id,"title":self.title,"explanation":self.explanation,"from":self.from,"to":self.to,"message":self.message,"credential":self.credential,"phase":self.phase,"rule":self.rule,"payload":self.payload,"error":self.error})
    }
}

fn refuse(
    mut steps: Vec<Step>,
    at: &'static str,
    code: &'static str,
    title: &'static str,
    explanation: &'static str,
) -> Vec<Step> {
    let phase = steps.last().map_or("read", |s| s.phase);
    let rule = if phase == "connect" {
        "/contracts/auth/acquisition"
    } else if phase == "identity" {
        "/contracts/governed-service"
    } else {
        steps.last().map_or("/contracts/service", |s| s.rule)
    };
    let mut rejection = Step::new("refused", title, explanation, at, at, "Access stops here")
        .phase(phase)
        .rule(rule);
    rejection.error = Some(code);
    steps.push(rejection);
    if at == "adapter" {
        let mut response = Step::new("error_to_gateway", "The adapter returns a safe failure", "The gateway receives the adapter's failure. It does not switch to another adapter or retry the request.", "adapter", "gateway", "Failure, not an issue page").phase(phase).rule(rule);
        response.error = Some(code);
        steps.push(response);
    }
    let mut returned = Step::new(
        "failure_returned",
        "Your application receives the failure",
        explanation,
        if at == "provider" {
            "provider"
        } else {
            "gateway"
        },
        "client",
        "The request did not complete",
    )
    .phase(phase)
    .rule(rule);
    returned.error = Some(code);
    steps.push(returned);
    steps
}

fn journey(mode: &str, situation: &str) -> Vec<Step> {
    let fixture: Value = serde_json::from_str(FIXTURES).expect("checked-in walkthrough fixtures");
    let governed = mode == "governed";
    let mut steps = Vec::new();
    if governed {
        steps.push(Step::new("sign_in", "You sign in to the application", "The application's identity service authenticates you. The resulting Connectors access token identifies the caller for the intended audience. This is separate from signing in to GitLab.", "client", "identity", "Application sign-in").credential("identity").phase("identity").rule("/contracts/governed-service"));
        steps.push(Step::new("identity_returned", "The application has access for Connectors", "The application can present its audience-bound access token to Connectors. Your GitLab account and its provider token have not been involved yet.", "identity", "client", "Access for the Connectors audience").credential("identity").phase("identity").rule("/contracts/governed-service"));
        steps.push(Step::new("begin_connection", "You choose Connect GitLab", "The gateway authenticates the caller and checks permission to begin connection setup. The ordinary result can describe an action; it cannot expose reusable callback or completion authority.", "client", "gateway", "Begin connection setup").credential("identity").phase("connect").rule("/contracts/auth/management"));
        steps.push(Step::new("setup_delegated", "Setup reaches the owning adapter service", "Through the specified management binding, the receiving host verifies the delegation and independently admits the setup request. Its coordinator owns this acquisition; the gateway does not own the GitLab credential.", "gateway", "adapter", "Admitted setup request").credential("delegation").phase("connect").rule("/contracts/auth/management"));
        steps.push(Step::new("consent", "Your browser asks GitLab for consent", "A separately admitted trusted UI opens the provider authorization flow. You review access at GitLab. This illustration leaves provider-specific registration, scope names and OAuth parameters to an authored profile.", "client", "provider", "Review access at GitLab").credential("consent").phase("connect").rule("/contracts/auth/acquisition"));
        if situation == "consent_declined" {
            steps.push(Step::new("consent_callback", "GitLab reports declined consent", "The trusted callback tells the owning coordinator that authorization was declined. It does not contain usable provider credentials.", "provider", "adapter", "Consent declined").phase("connect").rule("/contracts/auth/acquisition"));
            return refuse(steps, "adapter", "refused_by_provider", "No connection is published", "GitLab consent was declined. Setup ends without a ready connection or any issue request.");
        }
        steps.push(Step::new("callback", "GitLab returns to the trusted callback", "The owning coordinator validates callback correlation, expiry and one-time use. Completion arrives through its protected callback binding; it is not an ordinary operation forwarded by the gateway.", "provider", "adapter", "Protected authorization callback").credential("callback").phase("connect").rule("/contracts/auth/acquisition"));
        steps.push(Step::new("exchange", "The adapter exchanges authorization evidence", "The provider auth implementation performs the server-side exchange and validates identity and granted access. The diagram is conceptual: it does not invent a GitLab OAuth profile or perform an exchange.", "adapter", "provider", "Server-side authorization exchange").credential("exchange").phase("connect").rule("/contracts/auth/acquisition"));
        steps.push(Step::new("provider_credential", "The provider credential returns to its owner", "Sensitive provider material returns to the executing adapter's auth boundary. Neither the application nor the federation gateway receives it.", "provider", "adapter", "Provider credential + safe identity metadata").credential("provider").phase("connect").rule("/contracts/auth/custody"));
        steps.push(Step::new("custody", "The coordinator stores and validates the connection", "Custody stores immutable sensitive material. Required identity and baseline checks must pass, and metadata publication must be acknowledged, before setup is reported as complete.", "adapter", "custody", "Store material; publish safe metadata").credential("provider").phase("connect").rule("/contracts/auth/acquisition"));
        steps.push(Step::new("connection_to_gateway", "Only a safe connection reference travels back", "The gateway receives the completed status and a connection reference. It receives no provider token, secret-store address or callback capability.", "adapter", "gateway", "Connection conn-example is available").credential("connection").phase("connect").rule("/contracts/auth/management"));
        let mut connected = Step::new("connected", "GitLab is connected. The read has not run.", "Your application now has a safe connection reference. Connecting an account does not execute a business request. Choose Read issues when you want to start the separate call.", "gateway", "client", "Connected — waiting for your request").credential("connection").phase("connect").rule("/contracts/auth/acquisition");
        connected.pause = true;
        steps.push(connected);
    }
    steps.push(Step::new("describe", "Your client asks what it can call", if governed { "The gateway verifies the application's access token and current disclosure policy before returning an admitted operation description." } else { "Your client presents the configured gateway service credential. The gateway authenticates it before returning its operation description. A shared token does not identify individual users." }, "client", "gateway", "Describe available operations").credential(if governed { "identity" } else { "client_service" }).rule(if governed { "/contracts/governed-service" } else { "/contracts/service" }));
    if situation == "gateway_denied" {
        return refuse(steps, "gateway", "unauthorized", "The gateway rejects the client credential", "This credential does not authenticate the client to the gateway. No downstream invocation or GitLab request is sent.");
    }
    steps.push(Step::new("described", "The client learns the GitLab operation", "The configured route exposes issues.list as gitlab__issues.list. Its input schema and current description revision let the client construct the request. Discovery does not create a route or prove that GitLab is ready.", "gateway", "client", "gitlab__issues.list + input schema").payload(json!({"operation":"gitlab__issues.list","input":fixture["input"]})));
    steps.push(Step::new("invoke", "You ask for the first 20 project issues", if governed { "The client submits its input, current description revision and selected connection with authenticated caller context. Caller-written fields cannot grant authority." } else { "The client submits the operation, input and current gateway description revision, authenticating again. Provider credentials and arbitrary destinations are not invocation input." }, "client", "gateway", "Read issues from acme/website").credential(if governed { "identity" } else { "client_service" }).payload(json!({"operation":"gitlab__issues.list","input":fixture["input"]})).rule(if governed { "/contracts/governed-service" } else { "/contracts/service" }));
    steps.push(Step::new("route", "The gateway resolves its configured route", "The gateway checks the submitted revision and maps the source-qualified operation to the configured GitLab adapter. The leaf has its own revision; the gateway forwards with that revision. It never takes a destination from the business input.", "gateway", "gateway", "gitlab__issues.list → issues.list"));
    steps.push(Step::new("forward", if governed { "The gateway sends a signed, scoped delegation" } else { "The gateway authenticates to the adapter" }, if governed { "The assertion binds verified caller authority, receiver, route, request bytes, purpose and deadline. The gateway sends a delegation proof rather than forwarding the user's login token or substituting a broad service identity." } else { "The gateway uses its independently configured downstream service credential. It does not forward the client's credential, and it has no need for the GitLab provider token." }, "gateway", "adapter", "Invoke issues.list at the configured adapter").credential(if governed { "delegation" } else { "downstream_service" }).payload(json!({"operation":"issues.list","input":fixture["input"]})).rule(if governed { "/contracts/delegation" } else { "/contracts/service" }));
    if situation == "adapter_unreachable" {
        return refuse(steps, "gateway", "unavailable", "The adapter cannot be reached", "The configured adapter is unavailable. The gateway returns a failure; it does not fall back to another source or claim the project has no issues.");
    }
    steps.push(Step::new("leaf_admission", if governed { "The receiving host checks authority independently" } else { "The adapter checks the request locally" }, if governed { "The receiver verifies signature, audience, request binding, expiry and one-use delivery admission against its configured trust. It then checks current policy for this caller, operation and connection. A signed assertion alone does not grant access." } else { "The service validates its own credential, description revision and typed input. The GitLab adapter independently checks that acme/website is in its configured project allowlist." }, "adapter", "adapter", "Authenticate; validate; check access").credential(if governed { "delegation" } else { "downstream_service" }).rule(if governed { "/contracts/delegation" } else { "/adapters/gitlab/contracts/reads" }));
    if situation == "connection_denied" {
        return refuse(steps, "adapter", "not_granted", "This caller cannot use the connection", "The receiving host's current policy denies this operation on the selected connection. Valid login and prior GitLab consent do not override that policy. No provider call is sent.");
    }
    if governed {
        steps.push(Step::new("connection_check", "The host checks the selected connection", "Current connection viability and operation-specific scope and permission evidence must satisfy the request. The auth capability resolves only the selected connection's material at the trusted execution boundary.", "adapter", "custody", "Check binding, evidence and credential availability").credential("provider").rule("/contracts/auth/connection"));
        if situation == "custody_unavailable" {
            return refuse(steps, "adapter", "connection_not_ready", "Required credential custody is unavailable", "The host cannot establish readiness of the required custody dependency. It does not pretend the credential is invalid or try anonymous access. No GitLab issue request is sent.");
        }
    }
    let mut dispatch = Step::new("provider_request", "The adapter calls the GitLab API", "The adapter constructs the project issue request. Its scoped HTTP capability attaches the GitLab credential only at the configured provider destination. This is the first GitLab business request in the journey.", "adapter", "provider", "GET project issues · page 1 · up to 20").credential("provider").payload(json!({"method":"GET","path":"/api/v4/projects/acme%2Fwebsite/issues","query":{"per_page":20,"page":1,"order_by":"created_at","sort":"asc"}})).rule("/adapters/gitlab/contracts/reads");
    dispatch.provider_call = true;
    steps.push(dispatch);
    if situation == "provider_rejected" {
        steps.push(Step::new("provider_denied", "GitLab rejects its credential", "GitLab returns HTTP 401. Authentication to Connectors succeeded earlier; this is a different credential boundary. The adapter maps the provider response to a safe error, without copying the raw provider body.", "provider", "adapter", "HTTP 401 from GitLab").credential("provider").rule("/adapters/gitlab/contracts/reads"));
        return refuse(steps, "adapter", "unauthorized", "No issue page was obtained", "The provider request failed authentication. This configured read profile does not automatically refresh, retry or switch credentials.");
    }
    steps.push(Step::new("provider_result", "GitLab returns an issue page", "GitLab checks the provider credential and project access before returning its data. In this fictional project there are three issues; the fixture includes both open and closed issues because the operation has no state filter.", "provider", "adapter", "3 issues + pagination information").payload(fixture["result"]["items"].clone()).rule("/adapters/gitlab/contracts/reads"));
    steps.push(Step::new("adapt_result", "The adapter returns a bounded, attributed result", "The adapter validates the page, preserves the issue data and adds pagination and source provenance. Complete means this fixture has no further page, not that an unavailable source was silently omitted.", "adapter", "gateway", "Issue page + source + continuation").payload(fixture["result"].clone()).rule("/adapters/gitlab/contracts/reads"));
    steps.push(Step::new("result", "The issues arrive back at your application", "The gateway preserves the adapter's result and source information. Your application can display the issues without handling a GitLab credential or implementing GitLab transport.", "gateway", "client", "Display the project issues").payload(fixture["result"].clone()));
    steps
}

pub struct Walkthrough {
    mode: String,
    situation: String,
    cursor: usize,
    steps: Vec<Step>,
}
impl Default for Walkthrough {
    fn default() -> Self {
        Self::new("configured", "success")
    }
}
impl Walkthrough {
    fn new(mode: &str, situation: &str) -> Self {
        Self {
            mode: mode.into(),
            situation: situation.into(),
            cursor: 0,
            steps: journey(mode, situation),
        }
    }
    fn waiting(&self) -> bool {
        self.cursor > 0 && self.steps[self.cursor - 1].pause
    }
    pub fn snapshot(&self) -> Value {
        let current = self.cursor.checked_sub(1).map(|i| &self.steps[i]);
        let result = current
            .filter(|s| s.id == "result")
            .map(|s| s.payload.clone());
        json!({"mode":self.mode,"situation":self.situation,"index":self.cursor,"total":self.steps.len(),"done":self.cursor==self.steps.len(),"waiting_for_read":self.waiting(),"step":current.map(Step::json),"result":result,"provider_calls":self.steps[..self.cursor].iter().filter(|s|s.provider_call).count(),"visited":self.steps[..self.cursor].iter().map(|s|s.id).collect::<Vec<_>>()})
    }
    pub fn request(&mut self, bytes: &[u8]) -> Value {
        if bytes.len() > 4096 {
            return json!({"error":"request_too_large"});
        }
        let Ok(Value::Object(fields)) = serde_json::from_slice::<Value>(bytes) else {
            return json!({"error":"invalid_request"});
        };
        if fields
            .iter()
            .any(|(k, v)| !["mode", "situation", "action"].contains(&k.as_str()) || !v.is_string())
        {
            return json!({"error":"invalid_request"});
        }
        let mode = fields
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or(&self.mode);
        let situation = fields.get("situation").and_then(Value::as_str).unwrap_or(
            if fields.contains_key("mode") {
                "success"
            } else {
                &self.situation
            },
        );
        let cases: &[&str] = match mode {
            "configured" => &[
                "success",
                "gateway_denied",
                "adapter_unreachable",
                "provider_rejected",
            ],
            "governed" => &[
                "success",
                "consent_declined",
                "connection_denied",
                "custody_unavailable",
            ],
            _ => return json!({"error":"unknown_mode"}),
        };
        if !cases.contains(&situation) {
            return json!({"error":"unknown_situation"});
        }
        let action = fields
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("snapshot");
        if !["snapshot", "next", "back", "restart", "read"].contains(&action) {
            return json!({"error":"unknown_action"});
        }
        // Validate the whole request before changing an existing session.
        let selecting = fields.contains_key("mode") || fields.contains_key("situation");
        if action == "read" && (selecting || !self.waiting()) {
            return json!({"error":"read_not_available"});
        }
        if selecting {
            *self = Self::new(mode, situation);
        }
        match action {
            "restart" => self.cursor = 0,
            "back" => self.cursor = self.cursor.saturating_sub(1),
            "read" => self.cursor += 1,
            "next" if !self.waiting() && self.cursor < self.steps.len() => self.cursor += 1,
            _ => {}
        }
        self.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn action(w: &mut Walkthrough, action: &str) -> Value {
        w.request(json!({"action":action}).to_string().as_bytes())
    }
    fn finish(w: &mut Walkthrough) -> Value {
        for _ in 0..60 {
            let state = w.snapshot();
            if state["done"] == true {
                return state;
            }
            action(
                w,
                if state["waiting_for_read"] == true {
                    "read"
                } else {
                    "next"
                },
            );
        }
        panic!("walkthrough did not terminate");
    }
    #[test]
    fn successful_journeys_return_the_same_bounded_issue_page() {
        for mode in ["configured", "governed"] {
            let end = finish(&mut Walkthrough::new(mode, "success"));
            assert_eq!(end["result"]["items"].as_array().unwrap().len(), 3);
            assert_eq!(end["provider_calls"], 1);
            assert_eq!(end["result"]["complete"], true);
            assert_eq!(end["result"]["provenance"]["instance"], "gitlab-remote");
        }
    }
    #[test]
    fn connecting_never_automatically_runs_a_read() {
        let mut w = Walkthrough::new("governed", "success");
        for _ in 0..60 {
            action(&mut w, "next");
        }
        assert!(w.waiting());
        assert_eq!(w.snapshot()["provider_calls"], 0);
        assert!(w.snapshot()["result"].is_null());
        action(&mut w, "read");
        assert!(!w.waiting());
        assert_eq!(finish(&mut w)["provider_calls"], 1);
    }
    #[test]
    fn failures_stop_at_the_responsible_boundary_without_empty_success() {
        for (mode, case, code, calls, forbidden) in [
            ("configured", "gateway_denied", "unauthorized", 0, "forward"),
            (
                "configured",
                "adapter_unreachable",
                "unavailable",
                0,
                "leaf_admission",
            ),
            (
                "configured",
                "provider_rejected",
                "unauthorized",
                1,
                "provider_result",
            ),
            (
                "governed",
                "consent_declined",
                "refused_by_provider",
                0,
                "exchange",
            ),
            (
                "governed",
                "connection_denied",
                "not_granted",
                0,
                "connection_check",
            ),
            (
                "governed",
                "custody_unavailable",
                "connection_not_ready",
                0,
                "provider_request",
            ),
        ] {
            let mut w = Walkthrough::new(mode, case);
            let end = finish(&mut w);
            assert_eq!(end["step"]["error"], code);
            assert_eq!(end["provider_calls"], calls);
            assert!(end["result"].is_null());
            assert!(!end["visited"]
                .as_array()
                .unwrap()
                .contains(&json!(forbidden)));
            assert_eq!(action(&mut w, "next"), end);
        }
    }
    #[test]
    fn credential_routes_never_carry_provider_material_to_the_gateway_or_client() {
        for mode in ["configured", "governed"] {
            for step in journey(mode, "success") {
                if step.credential == "provider" {
                    assert!(["adapter", "provider", "custody"].contains(&step.from));
                    assert!(["adapter", "provider", "custody"].contains(&step.to));
                }
                if step.id == "forward" {
                    assert_eq!(
                        step.credential,
                        if mode == "configured" {
                            "downstream_service"
                        } else {
                            "delegation"
                        }
                    );
                }
            }
        }
    }
    #[test]
    fn navigation_and_invalid_requests_preserve_deterministic_state() {
        let mut w = Walkthrough::default();
        action(&mut w, "next");
        let before = w.snapshot();
        action(&mut w, "next");
        assert_eq!(action(&mut w, "back"), before);
        for bytes in [
            b"[]".as_slice(),
            b"{\"mode\":\"governed\",\"action\":\"typo\"}",
            b"{\"situation\":\"consent_declined\"}",
            b"{\"token\":\"not-accepted\"}",
            b"{\"action\":\"read\"}",
        ] {
            assert!(w.request(bytes).get("error").is_some());
            assert_eq!(w.snapshot(), before);
        }
        assert!(w.request(&vec![b' '; 4097]).get("error").is_some());
        assert_eq!(action(&mut w, "restart"), Walkthrough::default().snapshot());
    }
}
