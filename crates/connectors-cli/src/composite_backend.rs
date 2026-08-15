//! Closed composition of independently configured hosted Integration backends.

use std::sync::Arc;

use async_trait::async_trait;
use protocol::operation::{
    OperationError, OperationErrorCode, OperationRequest, OperationResult, OwnerContext,
};
use server::local::OperationBackend;

pub struct CompositeBackend {
    backends: Vec<Arc<dyn OperationBackend>>,
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

    async fn shutdown(&self) {
        for backend in &self.backends {
            backend.shutdown().await;
        }
    }
}

fn protocol() -> OperationError {
    OperationError::new(
        OperationErrorCode::Protocol,
        "configured Integration returned the wrong result variant",
        false,
    )
}
