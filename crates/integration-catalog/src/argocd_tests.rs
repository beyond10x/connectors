//! What acquisition must do, and the two things it must never do: put the password anywhere after
//! the sign-in, or hand back a token whose reach nobody declared.

use std::sync::Mutex;

use super::*;
use async_trait::async_trait;

/// One request the fake was asked to make: method, URL, body, headers.
type Recorded = (String, String, Option<String>, BTreeMap<String, String>);

/// A scripted Argo CD that records every request it was asked to make.
///
/// It answers by path rather than by call order so a test reads as a statement about the API rather
/// than about this file's sequencing.
struct FakeArgoCd {
    seen: Mutex<Vec<Recorded>>,
    project: Value,
    sign_in_status: u16,
    project_status: u16,
    update_status: u16,
    mint_status: u16,
}

impl FakeArgoCd {
    fn new(project: Value) -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
            project,
            sign_in_status: 200,
            project_status: 200,
            update_status: 200,
            mint_status: 200,
        }
    }

    fn calls(&self) -> Vec<Recorded> {
        self.seen.lock().unwrap().clone()
    }

    fn paths(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .map(|(method, url, _, _)| {
                let path = url.split_once("://").map_or(url.clone(), |(_, rest)| {
                    rest.split_once('/')
                        .map_or(String::new(), |(_, path)| format!("/{path}"))
                });
                format!("{method} {path}")
            })
            .collect()
    }
}

#[async_trait]
impl EgressTransport for FakeArgoCd {
    async fn execute(
        &self,
        _authority: &str,
        request: EgressHttpRequest,
    ) -> Result<service::EgressHttpResponse, EgressTransportError> {
        let Request {
            method,
            url,
            headers,
            body,
        } = request.request;
        self.seen
            .lock()
            .unwrap()
            .push((method.clone(), url.clone(), body.clone(), headers));

        let answer = |status: u16, value: Value| {
            Ok(service::EgressHttpResponse {
                status,
                headers: BTreeMap::new(),
                body: serde_json::to_vec(&value).unwrap(),
            })
        };
        if url.ends_with("/api/v1/session") {
            return answer(
                self.sign_in_status,
                serde_json::json!({"token": "session-jwt"}),
            );
        }
        if url.ends_with("/token") {
            return answer(self.mint_status, serde_json::json!({"token": "minted-jwt"}));
        }
        if method == "PUT" {
            return answer(self.update_status, self.project.clone());
        }
        answer(self.project_status, self.project.clone())
    }

    async fn connect_websocket(
        &self,
        _authority: &str,
        _url: String,
        _maximum: usize,
    ) -> Result<Box<dyn service::EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

fn project(roles: Value) -> Value {
    serde_json::json!({
        "metadata": {"name": "babelforce"},
        "spec": {
            "description": "payments",
            "sourceRepos": ["*"],
            "roles": roles,
        },
    })
}

fn request() -> AcquireRequest {
    AcquireRequest {
        origin: "https://argocd.infra.example".to_owned(),
        username: "admin".to_owned(),
        password: Zeroizing::new("hunter2-not-a-real-password".to_owned()),
        project: "babelforce".to_owned(),
        role: "b10x".to_owned(),
        allow_sync: true,
        expires_in_seconds: DEFAULT_EXPIRES_IN_SECONDS,
    }
}

#[tokio::test]
async fn the_four_calls_happen_in_order_and_the_token_comes_back() {
    let argocd = FakeArgoCd::new(project(serde_json::json!([])));
    let (token, acquired) = acquire(&argocd, request()).await.expect("acquisition");

    assert_eq!(token.as_str(), "minted-jwt");
    assert_eq!(
        argocd.paths(),
        vec![
            "POST /api/v1/session",
            "GET /api/v1/projects/babelforce",
            "PUT /api/v1/projects/babelforce",
            "POST /api/v1/projects/babelforce/roles/b10x/token",
        ]
    );
    assert!(acquired.role_created);
    assert_eq!(acquired.token_id, "b10x-b10x");
    assert_eq!(acquired.expires_in_seconds, 365 * 24 * 60 * 60);
}

/// **The one that matters.** The password buys a session and then vanishes: it must appear in
/// exactly one request body, and in no header, URL or body of any later call.
#[tokio::test]
async fn the_password_appears_once_and_the_session_token_never_persists() {
    let argocd = FakeArgoCd::new(project(serde_json::json!([])));
    let secret = "hunter2-not-a-real-password";
    let (token, _) = acquire(&argocd, request()).await.expect("acquisition");

    let calls = argocd.calls();
    let carrying = calls
        .iter()
        .filter(|(_, url, body, headers)| {
            url.contains(secret)
                || body.as_deref().is_some_and(|body| body.contains(secret))
                || headers.values().any(|value| value.contains(secret))
        })
        .count();
    assert_eq!(carrying, 1, "the password may travel exactly once");
    assert!(calls[0].1.ends_with("/api/v1/session"));

    // Every call after the sign-in authenticates with the session JWT, and the value this function
    // returns is the minted token rather than that session.
    for (_, _, _, headers) in &calls[1..] {
        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer session-jwt")
        );
    }
    assert_ne!(token.as_str(), "session-jwt");
}

/// A role an operator already reviewed is not rewritten. Re-running to replace an expired token
/// mints against the policies they have, rather than resetting policies they may have narrowed.
#[tokio::test]
async fn an_existing_role_is_reused_and_its_policies_are_left_alone() {
    let existing = serde_json::json!([{
        "name": "b10x",
        "policies": ["p, proj:babelforce:b10x, applications, get, babelforce/one, allow"],
    }]);
    let argocd = FakeArgoCd::new(project(existing));
    let (_, acquired) = acquire(&argocd, request()).await.expect("acquisition");

    assert!(!acquired.role_created);
    assert_eq!(
        argocd.paths(),
        vec![
            "POST /api/v1/session",
            "GET /api/v1/projects/babelforce",
            "POST /api/v1/projects/babelforce/roles/b10x/token",
        ],
        "no PUT when the role is already there"
    );
}

/// Read-modify-write, not replace: fields this flow does not understand survive the update.
#[tokio::test]
async fn the_project_is_written_back_whole() {
    let argocd = FakeArgoCd::new(project(serde_json::json!([])));
    acquire(&argocd, request()).await.expect("acquisition");

    let put = argocd
        .calls()
        .into_iter()
        .find(|(method, _, _, _)| method == "PUT")
        .expect("an update");
    let body: Value = serde_json::from_str(put.2.as_deref().expect("a body")).unwrap();
    let spec = &body["project"]["spec"];
    assert_eq!(spec["description"], "payments");
    assert_eq!(spec["sourceRepos"][0], "*");
    assert_eq!(spec["roles"][0]["name"], "b10x");
}

#[test]
fn read_only_acquisition_carries_no_sync_policy() {
    let read_only = policies_for("babelforce", "b10x", false);
    assert_eq!(
        read_only,
        vec!["p, proj:babelforce:b10x, applications, get, babelforce/*, allow"]
    );

    let syncing = policies_for("babelforce", "b10x", true);
    assert_eq!(syncing.len(), 2);
    assert!(syncing[1].contains("applications, sync, babelforce/*"));
}

/// Argo CD refuses any project-role policy outside its project-scoped resource set and any object
/// outside `<project>/…`. Everything this module writes has to satisfy that, or the update is
/// rejected at the far end where the message is much harder to read.
#[test]
fn every_generated_policy_is_one_argo_cd_will_accept() {
    for policy in policies_for("babelforce", "b10x", true) {
        let parts = policy.split(", ").collect::<Vec<_>>();
        assert_eq!(parts.len(), 6, "{policy}");
        assert_eq!(parts[0], "p");
        assert_eq!(parts[1], "proj:babelforce:b10x");
        assert!(
            [
                "applications",
                "applicationsets",
                "repositories",
                "exec",
                "logs",
                "clusters"
            ]
            .contains(&parts[2]),
            "{policy} names a resource no project role may carry"
        );
        assert!(parts[4].starts_with("babelforce/"), "{policy}");
        assert_eq!(parts[5], "allow");
    }
}

#[tokio::test]
async fn a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project() {
    let mut argocd = FakeArgoCd::new(project(serde_json::json!([])));
    argocd.sign_in_status = 401;
    let error = acquire(&argocd, request()).await.expect_err("refusal");
    assert!(matches!(error, AcquireError::SignInRejected));
    assert_eq!(
        argocd.paths().len(),
        1,
        "nothing is attempted after a refusal"
    );
}

#[tokio::test]
async fn a_login_without_projects_update_is_told_which_grant_it_lacks() {
    let mut argocd = FakeArgoCd::new(project(serde_json::json!([])));
    argocd.project_status = 403;
    let error = acquire(&argocd, request()).await.expect_err("refusal");
    let AcquireError::NotPermitted(project) = error else {
        panic!("expected a permission refusal, got {error:?}");
    };
    assert_eq!(project, "babelforce");
}

#[tokio::test]
async fn a_missing_project_is_not_reported_as_a_permission_problem() {
    let mut argocd = FakeArgoCd::new(project(serde_json::json!([])));
    argocd.project_status = 404;
    let error = acquire(&argocd, request()).await.expect_err("refusal");
    assert!(matches!(error, AcquireError::NoSuchProject(name) if name == "babelforce"));
}

/// Argo CD reads `expiresIn: 0` as "never expires". Passing it through would quietly mint the one
/// credential shape this flow exists to avoid.
#[tokio::test]
async fn a_zero_lifetime_is_refused_rather_than_meaning_forever() {
    let argocd = FakeArgoCd::new(project(serde_json::json!([])));
    let mut never = request();
    never.expires_in_seconds = 0;
    assert!(matches!(
        acquire(&argocd, never).await.expect_err("refusal"),
        AcquireError::NeverExpires
    ));
    assert!(
        argocd.paths().is_empty(),
        "nothing is sent before validation"
    );
}

/// A name with a `/` in it would silently become a different URL path. Refused before any request,
/// because the 404 it produces is unreadable.
#[tokio::test]
async fn a_project_or_role_name_that_could_change_the_path_is_refused() {
    let argocd = FakeArgoCd::new(project(serde_json::json!([])));
    for (project, role) in [
        ("babelforce/../admin", "b10x"),
        ("babelforce", "b10x?x=1"),
        ("", "b10x"),
    ] {
        let mut bad = request();
        bad.project = project.to_owned();
        bad.role = role.to_owned();
        assert!(
            matches!(
                acquire(&argocd, bad).await.expect_err("refusal"),
                AcquireError::Incomplete
            ),
            "{project}/{role} must be refused"
        );
    }
    assert!(argocd.paths().is_empty());
}

#[tokio::test]
async fn an_unreachable_origin_names_the_aperture_as_a_possibility() {
    struct Refusing;
    #[async_trait]
    impl EgressTransport for Refusing {
        async fn execute(
            &self,
            _authority: &str,
            _request: EgressHttpRequest,
        ) -> Result<service::EgressHttpResponse, EgressTransportError> {
            Err(EgressTransportError::Refused)
        }
        async fn connect_websocket(
            &self,
            _authority: &str,
            _url: String,
            _maximum: usize,
        ) -> Result<Box<dyn service::EgressWebSocket>, EgressTransportError> {
            Err(EgressTransportError::Refused)
        }
    }
    assert!(matches!(
        acquire(&Refusing, request()).await.expect_err("refusal"),
        AcquireError::Unreachable
    ));
}
