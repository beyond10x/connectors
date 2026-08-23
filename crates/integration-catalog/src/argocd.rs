//! Minting an Argo CD API token from a password, so nobody needs the `argocd` binary.
//!
//! # Why this exists at all
//!
//! Argo CD's API accepts exactly one thing: a bearer JWT. The documented way to get a durable one
//! is `argocd account generate-token`, and expecting everyone who runs `connectors` to have Argo
//! CD's CLI installed is the requirement this module removes. That CLI is a thin wrapper over the
//! same REST API this repository already speaks, so the whole flow is four ordinary HTTPS calls.
//!
//! # Why a project role rather than a local account
//!
//! `POST /api/v1/account/{name}/token` needs the target account to carry Argo CD's `apiKey`
//! capability, and capabilities live in the `argocd-cm` **ConfigMap**, which no Argo CD API can
//! write. Worse, the built-in `admin` account ships with `login` only — `util/settings/accounts.go`
//! at v3.5.1 constructs it as `Capabilities: []AccountCapability{AccountCapabilityLogin}` — so
//! admin cannot even mint one for itself. That route always ends at "go run kubectl", which is the
//! thing we are trying not to ask for.
//!
//! Project roles live in the `AppProject` custom resource, and that **is** API-writable. So the
//! whole bootstrap stays inside HTTPS to one origin:
//!
//! ```text
//! POST /api/v1/session                                  -> a session JWT, good for users.session.duration (24h default)
//! GET  /api/v1/projects/{project}                       -> the AppProject as it stands
//! PUT  /api/v1/projects/{project}                       -> the same AppProject plus one scoped role
//! POST /api/v1/projects/{project}/roles/{role}/token    -> the durable token, which is what we keep
//! ```
//!
//! # What the operator's password does and does not become
//!
//! It buys one session JWT and is then dropped. Neither it nor the session JWT is stored, returned,
//! logged, or placed on any request after step one. The only value that outlives this function is
//! the minted project-role token, and the caller puts that straight into the credential store.
//!
//! # What the minted token can do
//!
//! Exactly what the role's policies say, and no more. Argo CD validates project-role policies
//! against a closed resource set — `pkg/apis/application/v1alpha1/types.go` refuses any resource
//! outside `applications`, `applicationsets`, `repositories`, `exec`, `logs`, `clusters` — and
//! pins every object to `<project>/…`. So this token cannot leave its project even if the policy
//! text tried to, which is a stronger fence than a local-account token gets.
//!
//! The consequence worth stating plainly: `argocd-projects-list` will answer 403 for a token minted
//! here, because `projects` is not a project-scoped resource and no role can carry it. That
//! operation is catalogued but not exposed, so nothing a model can reach is affected.

use std::collections::BTreeMap;

use connector_resolve::Request;
use serde_json::{json, Value};
use service::{EgressHttpRequest, EgressTransport, EgressTransportError};
use zeroize::Zeroizing;

/// Re-exported so a caller can name the minted token's type without taking its own `zeroize` edge.
/// The composition root has to write this return type down and has no other reason to link the
/// crate; the alternative is a newtype whose only job is to avoid one dependency line.
pub use zeroize::Zeroizing as SecretString;

/// Bound on every response read during acquisition.
///
/// An `AppProject` is a small custom resource and a JWT is a short string; nothing here should
/// approach this. It exists so a wrong origin answering with something enormous is refused rather
/// than buffered.
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// One year, in seconds — the reviewed default lifetime for a minted token.
///
/// Argo CD treats `expiresIn: 0` as "never expires", which is what its own CLI does by default and
/// what this module deliberately does not do: a credential with no expiry is one whose loss has no
/// end date. A year is long enough that re-running this is a calendar event rather than a chore,
/// and short enough that an abandoned installation stops being able to deploy.
pub const DEFAULT_EXPIRES_IN_SECONDS: u64 = 365 * 24 * 60 * 60;

/// A ready-to-run acquisition, with its transport already inside it.
///
/// The type lives here rather than beside either of its two users because it is what crosses
/// between them: the composition root builds one, the operator console calls it, and neither has to
/// name a transport or depend on the other to do so.
pub type Acquire = Box<
    dyn FnOnce(
        AcquireRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(Zeroizing<String>, AcquiredToken), String>>>,
    >,
>;

/// What the operator supplies once, at connect time.
///
/// `password` is `Zeroizing` because it is the one value here that must not outlive its single use.
pub struct AcquireRequest {
    /// The operator-approved HTTPS origin, without an API path — `https://argocd.infra.example`.
    pub origin: String,
    /// An Argo CD login with `projects, update` on `project`. The built-in `admin` qualifies.
    pub username: String,
    /// That login's password. Used for exactly one request and then dropped.
    pub password: Zeroizing<String>,
    /// The `AppProject` whose applications this Connection will read and sync.
    pub project: String,
    /// The role to create or reuse inside that project.
    pub role: String,
    /// Whether the minted token may sync, on top of reading.
    pub allow_sync: bool,
    /// Token lifetime in seconds. Zero is refused rather than passed through as "never expires".
    pub expires_in_seconds: u64,
}

/// What acquisition produced, for a caller that has to explain it to a person.
///
/// The token is separate from this on purpose: everything here is safe to print, and the token
/// never is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredToken {
    /// The project the token is confined to.
    pub project: String,
    /// The role whose policies bound it.
    pub role: String,
    /// The stable token id, so it can be revoked by name later.
    pub token_id: String,
    /// Whether this run created the role or found it already there.
    pub role_created: bool,
    /// The policies the role carries after this run.
    pub policies: Vec<String>,
    /// When the token stops working, as a Unix second, or `None` when Argo CD reported none.
    pub expires_in_seconds: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum AcquireError {
    #[error("an Argo CD origin, username, password, project and role are all required")]
    Incomplete,
    #[error("a token lifetime of zero would never expire; give a positive number of seconds")]
    NeverExpires,
    #[error(
        "Argo CD rejected the sign-in: check the username and password, and note that Argo CD \
         rate-limits after 5 failed attempts in 5 minutes"
    )]
    SignInRejected,
    #[error(
        "the sign-in succeeded but `{0}` refused: this login needs `projects, update` on that \
         project, which the built-in `admin` account has"
    )]
    NotPermitted(String),
    #[error("Argo CD has no project named `{0}`")]
    NoSuchProject(String),
    #[error(
        "Argo CD was not reached: either the origin is wrong or unreachable, or this is a \
         private address and the acquisition was not admitted to reach it"
    )]
    Unreachable,
    #[error("Argo CD answered {status} at {step}, which this flow cannot continue from")]
    Unexpected { step: &'static str, status: u16 },
    #[error("Argo CD's answer at {0} was not the shape its API documents")]
    Malformed(&'static str),
}

/// Mint a project-scoped Argo CD API token, using the operator's password exactly once.
///
/// # Errors
///
/// An incomplete request, a rejected sign-in, a login without `projects, update`, a project that
/// does not exist, an origin that could not be reached, or an answer this flow cannot read.
pub async fn acquire(
    egress: &dyn EgressTransport,
    request: AcquireRequest,
) -> Result<(Zeroizing<String>, AcquiredToken), AcquireError> {
    let origin = request.origin.trim_end_matches('/');
    if origin.is_empty()
        || request.username.trim().is_empty()
        || request.password.trim().is_empty()
        || !valid_name(&request.project)
        || !valid_name(&request.role)
    {
        return Err(AcquireError::Incomplete);
    }
    if request.expires_in_seconds == 0 {
        return Err(AcquireError::NeverExpires);
    }

    // Step one, and the only one the password appears in. The binding holds it no longer than the
    // request it is serialized into, and both are dropped before the token below is returned.
    let session = {
        let body = json!({"username": request.username, "password": request.password.as_str()});
        let response = send(
            egress,
            "sign-in",
            post(origin, "/api/v1/session", None, &body),
        )
        .await?;
        match response.status {
            200 => token_field(&response.body, "sign-in")?,
            401 | 403 => return Err(AcquireError::SignInRejected),
            status => {
                return Err(AcquireError::Unexpected {
                    step: "sign-in",
                    status,
                })
            }
        }
    };

    // Read-modify-write, because Argo CD publishes no endpoint that adds a role on its own: the
    // CLI's `proj role create` is this same sequence. Sending back the project we just read keeps
    // every field we do not understand intact rather than defaulting it away.
    let mut project = {
        let response = send(
            egress,
            "read the project",
            get(
                origin,
                &format!("/api/v1/projects/{}", encode(&request.project)),
                Some(&session),
            ),
        )
        .await?;
        match response.status {
            200 => parse(&response.body, "read the project")?,
            403 => return Err(AcquireError::NotPermitted(request.project.clone())),
            404 => return Err(AcquireError::NoSuchProject(request.project.clone())),
            status => {
                return Err(AcquireError::Unexpected {
                    step: "read the project",
                    status,
                })
            }
        }
    };

    let policies = policies_for(&request.project, &request.role, request.allow_sync);
    let role_created = upsert_role(&mut project, &request.role, &policies);
    if role_created {
        let response = send(
            egress,
            "add the role",
            put(
                origin,
                &format!("/api/v1/projects/{}", encode(&request.project)),
                &session,
                &json!({"project": project}),
            ),
        )
        .await?;
        match response.status {
            200 => {}
            403 => return Err(AcquireError::NotPermitted(request.project.clone())),
            status => {
                return Err(AcquireError::Unexpected {
                    step: "add the role",
                    status,
                })
            }
        }
    }

    // A stable id, so the token is revocable by name and a re-run is legible in `argocd proj role
    // list`. Argo CD generates a UUID when none is given, which is exactly what you cannot point at
    // later when you want it gone.
    let token_id = format!("b10x-{}", request.role);
    let minted = {
        let body = json!({
            "project": request.project,
            "role": request.role,
            "id": token_id,
            "expiresIn": request.expires_in_seconds,
            "description": "B10x Connector",
        });
        let response = send(
            egress,
            "mint the token",
            post(
                origin,
                &format!(
                    "/api/v1/projects/{}/roles/{}/token",
                    encode(&request.project),
                    encode(&request.role)
                ),
                Some(&session),
                &body,
            ),
        )
        .await?;
        match response.status {
            200 => token_field(&response.body, "mint the token")?,
            403 => return Err(AcquireError::NotPermitted(request.project.clone())),
            status => {
                return Err(AcquireError::Unexpected {
                    step: "mint the token",
                    status,
                })
            }
        }
    };

    Ok((
        minted,
        AcquiredToken {
            project: request.project,
            role: request.role,
            token_id,
            role_created,
            policies,
            expires_in_seconds: request.expires_in_seconds,
        },
    ))
}

/// The policies the minted token carries.
///
/// `applications, get` covers every read this connector ships. `applications, sync` is the grant
/// Argo CD enforces for sync — and, in the same breath, for rollback and terminate
/// (`server/application/application.go` checks `ActionSync` for all three at v3.5.1). Argo CD has
/// no finer distinction to offer, so the fence between "may sync" and "may roll back" stays where
/// this repository already put it: rollback is declared `destructive` and unexposed, and sits above
/// the default grant ceiling.
fn policies_for(project: &str, role: &str, allow_sync: bool) -> Vec<String> {
    let subject = format!("proj:{project}:{role}");
    let mut policies = vec![format!(
        "p, {subject}, applications, get, {project}/*, allow"
    )];
    if allow_sync {
        policies.push(format!(
            "p, {subject}, applications, sync, {project}/*, allow"
        ));
    }
    policies
}

/// Put the role into the project document, returning whether the document needs writing back.
///
/// An existing role is **left alone**. Re-running acquisition to replace an expired token must not
/// silently rewrite policies an operator has since widened or narrowed by hand; minting a second
/// token against the role they already reviewed is the smaller, more predictable act.
fn upsert_role(project: &mut Value, role: &str, policies: &[String]) -> bool {
    let roles = project
        .get_mut("spec")
        .and_then(Value::as_object_mut)
        .map(|spec| spec.entry("roles").or_insert_with(|| json!([])));
    let Some(Value::Array(roles)) = roles else {
        return false;
    };
    if roles
        .iter()
        .any(|existing| existing.get("name").and_then(Value::as_str) == Some(role))
    {
        return false;
    }
    roles.push(json!({
        "name": role,
        "description": "B10x Connector",
        "policies": policies,
    }));
    true
}

async fn send(
    egress: &dyn EgressTransport,
    step: &'static str,
    request: Request,
) -> Result<service::EgressHttpResponse, AcquireError> {
    egress
        .execute(
            "connect-session:argocd",
            EgressHttpRequest {
                request,
                maximum_response_bytes: MAX_RESPONSE_BYTES,
                response_headers: Vec::new(),
            },
        )
        .await
        .map_err(|error| match error {
            EgressTransportError::Refused => AcquireError::Unreachable,
            EgressTransportError::ResponseTooLarge => AcquireError::Unexpected { step, status: 0 },
        })
}

fn headers(session: Option<&Zeroizing<String>>, json_body: bool) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    headers.insert("accept".to_owned(), "application/json".to_owned());
    if json_body {
        headers.insert("content-type".to_owned(), "application/json".to_owned());
    }
    if let Some(session) = session {
        headers.insert(
            "authorization".to_owned(),
            format!("Bearer {}", session.as_str()),
        );
    }
    headers
}

fn get(origin: &str, path: &str, session: Option<&Zeroizing<String>>) -> Request {
    Request {
        method: "GET".to_owned(),
        url: format!("{origin}{path}"),
        headers: headers(session, false),
        body: None,
    }
}

fn post(origin: &str, path: &str, session: Option<&Zeroizing<String>>, body: &Value) -> Request {
    Request {
        method: "POST".to_owned(),
        url: format!("{origin}{path}"),
        headers: headers(session, true),
        body: Some(body.to_string()),
    }
}

fn put(origin: &str, path: &str, session: &Zeroizing<String>, body: &Value) -> Request {
    Request {
        method: "PUT".to_owned(),
        url: format!("{origin}{path}"),
        headers: headers(Some(session), true),
        body: Some(body.to_string()),
    }
}

fn parse(body: &[u8], step: &'static str) -> Result<Value, AcquireError> {
    serde_json::from_slice(body).map_err(|_| AcquireError::Malformed(step))
}

/// Both `sessionSessionResponse` and `projectProjectTokenResponse` are `{"token": "…"}`.
fn token_field(body: &[u8], step: &'static str) -> Result<Zeroizing<String>, AcquireError> {
    let value = parse(body, step)?;
    let token = value
        .get("token")
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty())
        .ok_or(AcquireError::Malformed(step))?;
    Ok(Zeroizing::new(token.to_owned()))
}

/// Refused rather than encoded: a project or role name is an operator's own identifier, and one
/// carrying a `/` or a `?` is a mistake whose silently-encoded form produces a 404 nobody can read.
/// Argo CD's own role-name rule is alphanumerics plus `-` and `_`; project names are DNS labels.
fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// Names are validated above rather than escaped, so this only asserts that invariant at the point
/// of use. It exists so a future caller that loosens `valid_name` fails loudly here.
fn encode(value: &str) -> &str {
    debug_assert!(valid_name(value));
    value
}

#[cfg(test)]
#[path = "argocd_tests.rs"]
mod tests;
