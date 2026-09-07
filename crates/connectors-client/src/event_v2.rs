//! Explicit endpoint subscription requests with no version fallback or stream-opening retry.
use super::*;
use protocol::event::v2;

impl LocalClient {
    /// Subscribe, stop a subscription or read events through the owner-only daemon socket.
    pub async fn event_v2(
        &self,
        context: &operation::OwnerContext,
        request: v2::EventRequest,
    ) -> Result<v2::ResponseEnvelope, ClientError> {
        let request = envelope(context, request)?;
        let bytes = self
            .versioned_exchange(serde_json::to_vec(&request)?, v2::MAX_RESPONSE_BYTES)
            .await?;
        response(&bytes, &request)
    }
}

impl HostedClient {
    /// Use the exact v2 event contract under existing verified event authority.
    pub async fn event_v2(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: v2::EventRequest,
    ) -> Result<v2::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request = envelope(context, request)?;
        let (_, bytes) = self
            .versioned_exchange(
                &endpoint(&self.base, "events"),
                bearer,
                serde_json::to_vec(&request)?,
                v2::MAX_RESPONSE_BYTES,
                false,
            )
            .await?;
        response(&bytes, &request)
    }
}

fn envelope(
    context: &operation::OwnerContext,
    request: v2::EventRequest,
) -> Result<v2::RequestEnvelope, ClientError> {
    let envelope = v2::RequestEnvelope {
        protocol: v2::CONTRACT.into(),
        request_id: request_id(),
        context: context.clone(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
    Ok(envelope)
}
fn response(
    bytes: &[u8],
    request: &v2::RequestEnvelope,
) -> Result<v2::ResponseEnvelope, ClientError> {
    let response: v2::ResponseEnvelope =
        serde_json::from_slice(bytes).map_err(|_| ClientError::InvalidResponse)?;
    if response.validate().is_err()
        || response.request_id != request.request_id
        || response
            .response
            .as_ref()
            .is_some_and(|result| !result.matches(&request.request))
    {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_refuses_cross_subscription_method_and_version_responses() {
        let request = v2::RequestEnvelope {
            protocol: v2::CONTRACT.into(),
            request_id: "request:stop".into(),
            context: operation::OwnerContext {
                tenant_id: "tenant".into(),
                agent_id: "agent".into(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot".into(),
                authority_snapshot_sha256: "a".repeat(64),
            },
            request: v2::EventRequest::Unsubscribe(v2::UnsubscribeRequest {
                subscription_ref: "subscription:one".into(),
            }),
        };
        for result in [
            v2::EventResult::Unsubscribe {
                subscription_ref: "subscription:other".into(),
            },
            v2::EventResult::Search {
                channels: Vec::new(),
            },
        ] {
            let wrong = v2::ResponseEnvelope::success(&request.request_id, result);
            assert!(wrong.validate().is_ok());
            assert!(matches!(
                response(&serde_json::to_vec(&wrong).unwrap(), &request),
                Err(ClientError::InvalidResponse)
            ));
        }
        let mut valid = v2::ResponseEnvelope::success(
            &request.request_id,
            v2::EventResult::Unsubscribe {
                subscription_ref: "subscription:one".into(),
            },
        );
        assert!(response(&serde_json::to_vec(&valid).unwrap(), &request).is_ok());
        valid.protocol = event::CONTRACT.into();
        assert!(matches!(
            response(&serde_json::to_vec(&valid).unwrap(), &request),
            Err(ClientError::InvalidResponse)
        ));
    }
}
