//! Closed composition of independently configured hosted Integration backends.

use std::sync::Arc;

use async_trait::async_trait;
use connector_secrets::SecretStore;
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, ConnectionRequest, ConnectionResult,
};
use protocol::event::{EventError, EventErrorCode, EventRequest, EventResult};
use protocol::operation::{
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OwnerContext,
};
use server::local::OperationBackend;

pub struct CompositeBackend {
    backends: Vec<Arc<dyn OperationBackend>>,
}

/// Keeps the hosted credential port in the runtime composition while Integration backends receive
/// it through explicit constructors as they gain durable credential support.
pub struct CredentialStoreBackend {
    backend: Arc<dyn OperationBackend>,
    _credential_store: Arc<dyn SecretStore>,
}

impl CredentialStoreBackend {
    #[must_use]
    pub fn new(backend: Arc<dyn OperationBackend>, credential_store: Arc<dyn SecretStore>) -> Self {
        Self {
            backend,
            _credential_store: credential_store,
        }
    }
}

impl CompositeBackend {
    #[must_use]
    pub fn new(backends: Vec<Arc<dyn OperationBackend>>) -> Self {
        Self { backends }
    }
}

#[async_trait]
impl OperationBackend for CompositeBackend {
    async fn handle(
        &self,
        context: &OwnerContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        if let OperationRequest::Search(search) = &request {
            let mut operations = Vec::new();
            for backend in &self.backends {
                match backend.handle(context, request.clone()).await {
                    Ok(OperationResult::Search { operations: found }) => operations.extend(found),
                    Ok(_) => return Err(protocol()),
                    Err(error) if error.code == OperationErrorCode::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            operations.truncate(usize::from(search.limit));
            return Ok(OperationResult::Search { operations });
        }
        for backend in &self.backends {
            match backend.handle(context, request.clone()).await {
                Err(error) if error.code == OperationErrorCode::NotFound => {}
                result => return result,
            }
        }
        Err(OperationError::new(
            OperationErrorCode::NotFound,
            "no configured Integration owns this operation",
            false,
        ))
    }

    async fn handle_connection(
        &self,
        context: &OwnerContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        if let ConnectionRequest::Search(search) = &request {
            let mut connections = Vec::new();
            for backend in self
                .backends
                .iter()
                .filter(|backend| backend.supports_connections())
            {
                match backend.handle_connection(context, request.clone()).await {
                    Ok(ConnectionResult::Search { connections: found }) => {
                        connections.extend(found);
                    }
                    Ok(_) => return Err(connection_protocol()),
                    Err(error) if error.code == ConnectionErrorCode::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            connections.sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
            connections.dedup_by(|left, right| left.connection_ref == right.connection_ref);
            connections.truncate(usize::from(search.limit));
            return Ok(ConnectionResult::Search { connections });
        }
        for backend in self
            .backends
            .iter()
            .filter(|backend| backend.supports_connections())
        {
            match backend.handle_connection(context, request.clone()).await {
                Err(error) if error.code == ConnectionErrorCode::NotFound => {}
                result => return result,
            }
        }
        Err(ConnectionError::new(
            ConnectionErrorCode::NotFound,
            "no configured Integration owns this Connection request",
            false,
        ))
    }

    async fn handle_event(
        &self,
        context: &OwnerContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        if let EventRequest::Search(search) = &request {
            let mut channels = Vec::new();
            for backend in self
                .backends
                .iter()
                .filter(|backend| backend.supports_events())
            {
                match backend.handle_event(context, request.clone()).await {
                    Ok(EventResult::Search { channels: found }) => channels.extend(found),
                    Ok(_) => return Err(event_protocol()),
                    Err(error) if error.code == EventErrorCode::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            channels.sort_by(|left, right| left.channel_ref.cmp(&right.channel_ref));
            channels.dedup_by(|left, right| left.channel_ref == right.channel_ref);
            channels.truncate(usize::from(search.limit));
            return Ok(EventResult::Search { channels });
        }
        for backend in self
            .backends
            .iter()
            .filter(|backend| backend.supports_events())
        {
            match backend.handle_event(context, request.clone()).await {
                Err(error) if error.code == EventErrorCode::NotFound => {}
                result => return result,
            }
        }
        Err(EventError::new(
            EventErrorCode::NotFound,
            "no configured Integration owns this event request",
            false,
        ))
    }

    fn supports_connections(&self) -> bool {
        self.backends
            .iter()
            .any(|backend| backend.supports_connections())
    }

    fn supports_events(&self) -> bool {
        self.backends
            .iter()
            .any(|backend| backend.supports_events())
    }

    async fn shutdown(&self) {
        for backend in &self.backends {
            backend.shutdown().await;
        }
    }
}

#[async_trait]
impl OperationBackend for CredentialStoreBackend {
    async fn handle(
        &self,
        context: &OwnerContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.backend.handle(context, request).await
    }

    async fn handle_connection(
        &self,
        context: &OwnerContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.backend.handle_connection(context, request).await
    }

    async fn handle_event(
        &self,
        context: &OwnerContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        self.backend.handle_event(context, request).await
    }

    fn supports_connections(&self) -> bool {
        self.backend.supports_connections()
    }

    fn supports_events(&self) -> bool {
        self.backend.supports_events()
    }

    async fn shutdown(&self) {
        self.backend.shutdown().await;
    }
}

fn protocol() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "configured Integration returned the wrong result variant",
        false,
    )
}

fn connection_protocol() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "configured Integration returned the wrong Connection result variant",
        false,
    )
}

fn event_protocol() -> EventError {
    EventError::new(
        EventErrorCode::Protocol,
        "configured Integration returned the wrong event result variant",
        false,
    )
}

#[cfg(test)]
mod tests {
    use protocol::connection::{
        ConnectionInitiator, ConnectionRoute, ConnectionState, ConnectionSummary, SearchRequest,
    };
    use protocol::event::ChannelSummary;

    use super::*;

    struct OperationOnly;

    #[async_trait]
    impl OperationBackend for OperationOnly {
        async fn handle(
            &self,
            _context: &OwnerContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            Ok(OperationResult::Search {
                operations: Vec::new(),
            })
        }
    }

    struct ConnectionAndEvent;

    #[async_trait]
    impl OperationBackend for ConnectionAndEvent {
        async fn handle(
            &self,
            _context: &OwnerContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            Err(OperationError::new(
                OperationErrorCode::NotFound,
                "operation not found",
                false,
            ))
        }

        async fn handle_connection(
            &self,
            _context: &OwnerContext,
            _request: ConnectionRequest,
        ) -> Result<ConnectionResult, ConnectionError> {
            Ok(ConnectionResult::Search {
                connections: vec![ConnectionSummary {
                    connection_ref: "connection:test".to_owned(),
                    integration_ref: "test".to_owned(),
                    label: "Test".to_owned(),
                    state: ConnectionState::Callable,
                    initiation: vec![ConnectionInitiator::B10x],
                    route: ConnectionRoute::Direct,
                }],
            })
        }

        async fn handle_event(
            &self,
            _context: &OwnerContext,
            _request: EventRequest,
        ) -> Result<EventResult, EventError> {
            Ok(EventResult::Search {
                channels: vec![ChannelSummary {
                    channel_ref: "channel:test".to_owned(),
                    connection_ref: "connection:test".to_owned(),
                    integration_ref: "test".to_owned(),
                    binding_ref: "binding:test".to_owned(),
                    events: vec!["test.event".to_owned()],
                }],
            })
        }

        fn supports_connections(&self) -> bool {
            true
        }

        fn supports_events(&self) -> bool {
            true
        }
    }

    fn owner() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-test".to_owned(),
            agent_id: "agent-test".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot-test".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn capability_aware_composition_skips_operation_only_backends() {
        let backend =
            CompositeBackend::new(vec![Arc::new(OperationOnly), Arc::new(ConnectionAndEvent)]);
        let ConnectionResult::Search { connections } = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::Search(SearchRequest {
                    query: String::new(),
                    limit: 16,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("Connection search expected");
        };
        assert_eq!(connections[0].connection_ref, "connection:test");

        let EventResult::Search { channels } = backend
            .handle_event(
                &owner(),
                EventRequest::Search(protocol::event::SearchRequest {
                    query: String::new(),
                    limit: 16,
                }),
            )
            .await
            .unwrap()
        else {
            panic!("event search expected");
        };
        assert_eq!(channels[0].channel_ref, "channel:test");
    }
}
