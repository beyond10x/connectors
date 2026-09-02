use super::*;
use axum::body::{to_bytes, Body};
use axum::http::{header, Request, StatusCode};
use connector_secrets::{CredentialRef, MemoryStore, SecretStore as _};
use connector_state::MemoryState;
use serde_json::Value;
use service::{
    AdminConfigurationField, AdminCredentialRequirement, AdminIntegration, AdminRegistry,
};

fn admin_router(credentials: Arc<MemoryStore>) -> Router {
    admin_router_with_verifier(credentials, Arc::new(Verifier))
}

fn admin_router_with_verifier(
    credentials: Arc<MemoryStore>,
    verifier: Arc<dyn IdentityVerifier>,
) -> Router {
    let requirement = CredentialRef::new(
        "tenant-dev",
        "com.gitlab.api",
        "login",
        "oauth_client_secret",
    )
    .unwrap();
    let admin = AdminRegistry::new(
        "tenant-dev".to_owned(),
        credentials,
        Arc::new(MemoryState::new()),
        vec![AdminIntegration::new(
            "gitlab",
            vec![AdminConfigurationField::valid("oauth_client_id")],
            vec![AdminCredentialRequirement::token(
                "oauth_client_secret",
                true,
                requirement,
            )],
        )],
    )
    .unwrap();
    router_with_admin(
        verifier,
        Arc::new(Backend),
        HostedAdmissionPolicy::new(["operator".to_owned()]),
        HostedAuthority::unbound(),
        None,
        Some(Arc::new(admin)),
    )
}

struct MissingScopeVerifier;

#[async_trait]
impl IdentityVerifier for MissingScopeVerifier {
    async fn ready(&self) -> Result<(), IdentityVerificationError> {
        Ok(())
    }

    async fn verify(
        &self,
        credential: &str,
        audience: &str,
    ) -> Result<HostedPrincipal, IdentityVerificationError> {
        let mut principal = Verifier.verify(credential, audience).await?;
        principal.scopes.remove("connectors.integrations.manage");
        Ok(principal)
    }
}

#[tokio::test]
async fn auth_metadata_is_public_and_selects_exact_authority() {
    let response = admin_router(Arc::new(MemoryStore::new()))
        .oneshot(
            Request::get("/admin/auth-metadata")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(body["identity_origin"], "https://identity.example.test");
    assert_eq!(body["audience"], CONNECTORS_AUDIENCE);
    assert_eq!(body["scope"], "connectors.integrations.manage");
}

#[tokio::test]
async fn operator_can_write_and_status_never_returns_the_secret() {
    let credentials = Arc::new(MemoryStore::new());
    let app = admin_router(credentials.clone());
    let marker = "route-secret-marker";
    let response = app
        .clone()
        .oneshot(
            Request::put("/admin/integrations/gitlab/credentials/oauth_client_secret")
                .header(header::AUTHORIZATION, "Bearer access")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"request_id":"request-one","value":"{marker}","replace":false}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(!response
        .windows(marker.len())
        .any(|part| part == marker.as_bytes()));

    let response = app
        .oneshot(
            Request::get("/admin/integrations")
                .header(header::AUTHORIZATION, "Bearer access")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    assert!(!body
        .windows(marker.len())
        .any(|part| part == marker.as_bytes()));
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body["integrations"][0]["credentials"][0]["state"],
        "present"
    );

    let reference = CredentialRef::new(
        "tenant-dev",
        "com.gitlab.api",
        "login",
        "oauth_client_secret",
    )
    .unwrap();
    assert_eq!(
        credentials.get(&reference).await.unwrap().expose_secret(),
        marker
    );
}

#[tokio::test]
async fn operator_group_without_the_exact_scope_cannot_write() {
    let credentials = Arc::new(MemoryStore::new());
    let response = admin_router_with_verifier(credentials.clone(), Arc::new(MissingScopeVerifier))
        .oneshot(
            Request::put("/admin/integrations/gitlab/credentials/oauth_client_secret")
                .header(header::AUTHORIZATION, "Bearer access")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"request_id":"request-one","value":"must-not-be-written"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let reference = CredentialRef::new(
        "tenant-dev",
        "com.gitlab.api",
        "login",
        "oauth_client_secret",
    )
    .unwrap();
    assert!(!credentials.exists(&reference).await.unwrap());
}
