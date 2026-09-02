//! Hosted Catalog client boundary.

use crate::response::validate_catalog_response;
use crate::{catalog, operation, request_id, require_bearer, ClientError, HostedClient};

impl HostedClient {
    /// Sends one authenticated, credential-free hosted catalog request.
    pub async fn catalog(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: catalog::CatalogRequest,
    ) -> Result<catalog::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = catalog::RequestEnvelope {
            protocol: catalog::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.message))?;
        let response = self
            .exchange(
                &self.catalog,
                bearer,
                &envelope,
                catalog::MAX_FRAME_BYTES,
                catalog::MAX_RESPONSE_BYTES,
            )
            .await?;
        validate_catalog_response(response, &request_id)
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::HeaderMap;
    use axum::routing::post;
    use axum::Router;
    use tokio::net::TcpListener;

    use super::*;

    fn context() -> operation::OwnerContext {
        operation::OwnerContext {
            tenant_id: "tenant-1".to_owned(),
            agent_id: "agent-1".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn posts_and_validates_a_catalog_frame() {
        async fn catalog_handler(
            State(expected): State<operation::OwnerContext>,
            headers: HeaderMap,
            body: Bytes,
        ) -> Bytes {
            assert_eq!(
                headers.get(reqwest::header::AUTHORIZATION).unwrap(),
                "Bearer session-1"
            );
            let request: catalog::RequestEnvelope = serde_json::from_slice(&body).unwrap();
            request.validate().unwrap();
            assert_eq!(request.context, expected);
            Bytes::from(
                serde_json::to_vec(&catalog::ResponseEnvelope::success(
                    request.request_id,
                    catalog::CatalogResult::Search {
                        providers: Vec::new(),
                        next_offset: None,
                    },
                ))
                .unwrap(),
            )
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/api/connectors/v1/catalog", post(catalog_handler))
            .with_state(context());
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = HostedClient::new(&format!("http://{address}/api/connectors/v1")).unwrap();
        let response = client
            .catalog(
                "session-1",
                &context(),
                catalog::CatalogRequest::Search(catalog::SearchRequest {
                    query: "gitlab".to_owned(),
                    offset: 0,
                    limit: 1,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status, catalog::ResponseStatus::Ok);
        serving.abort();
    }
}
