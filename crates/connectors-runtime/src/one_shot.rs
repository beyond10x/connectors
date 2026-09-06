//! Bounded personal commands share the persistent runtime's composition and dispatch owners.

use std::io;
use std::path::{Path, PathBuf};

use protocol::{connection, operation};
use server::local::{
    preflight_one_shot_operation_v3, validate_one_shot_connection, validate_one_shot_operation,
    LocalOneShot, OneShotOperationV3Outcome,
};

use crate::{PersonalRuntime, RuntimeError};

/// Only a missing directory entry permits daemon-free composition. An unsafe or stale existing
/// object, a permission error, or any error after transport dispatch is never a retry signal.
pub fn local_socket_absent(state_root: &Path) -> Result<bool, io::Error> {
    match std::fs::symlink_metadata(state_root.join("connectors.sock")) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error),
    }
}

impl PersonalRuntime {
    /// Execute an explicitly selected v3 operation through the same bounded local runtime.
    /// The existing v2 entry point remains available to predecessor callers.
    pub async fn one_shot_operation_v3(
        config_path: &Path,
        state_root: impl Into<PathBuf>,
        context: operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, RuntimeError> {
        Self::one_shot_operation_v3_outcome(config_path, state_root, context, request)
            .await
            .map(OneShotOperationV3Outcome::into_response)
    }

    /// Preserve receiver-owned daemon requirements without trusting backend error text.
    pub async fn one_shot_operation_v3_outcome(
        config_path: &Path,
        state_root: impl Into<PathBuf>,
        context: operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<OneShotOperationV3Outcome, RuntimeError> {
        let envelope = operation::v3::RequestEnvelope {
            protocol: operation::v3::CONTRACT.to_owned(),
            request_id: "local-one-shot".to_owned(),
            context,
            request,
        };
        if let Some(outcome) = preflight_one_shot_operation_v3(&envelope) {
            return Ok(outcome);
        }
        let composed = Self::compose(Some(config_path), state_root.into(), None, false).await?;
        Ok(LocalOneShot::new(composed.ownership, composed.registry)?
            .operation_v3_outcome(envelope)
            .await?)
    }

    /// Execute one operation without binding a socket or creating a supervised child process.
    pub async fn one_shot_operation(
        config_path: &Path,
        state_root: impl Into<PathBuf>,
        context: operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::ResponseEnvelope, RuntimeError> {
        let envelope = operation::RequestEnvelope {
            protocol: operation::CONTRACT.to_owned(),
            request_id: "local-one-shot".to_owned(),
            context,
            request,
        };
        if let Err(error) = validate_one_shot_operation(&envelope) {
            return Ok(operation::ResponseEnvelope::failure(
                &envelope.request_id,
                error,
            ));
        }
        let composed = Self::compose(Some(config_path), state_root.into(), None, false).await?;
        Ok(LocalOneShot::new(composed.ownership, composed.registry)?
            .operation(envelope)
            .await?)
    }

    /// Read one bounded Connection metadata result. Persistent acquisition and activation flows
    /// are refused before composition, so no dead completion handle can escape the process.
    pub async fn one_shot_connection(
        config_path: &Path,
        state_root: impl Into<PathBuf>,
        context: operation::OwnerContext,
        request: connection::ConnectionRequest,
    ) -> Result<connection::ResponseEnvelope, RuntimeError> {
        let envelope = connection::RequestEnvelope {
            protocol: connection::CONTRACT.to_owned(),
            request_id: "local-one-shot".to_owned(),
            context,
            request,
        };
        if let Err(error) = validate_one_shot_connection(&envelope) {
            return Ok(connection::ResponseEnvelope::failure(
                &envelope.request_id,
                error,
            ));
        }
        let composed = Self::compose(Some(config_path), state_root.into(), None, false).await?;
        Ok(LocalOneShot::new(composed.ownership, composed.registry)?
            .connection(envelope)
            .await?)
    }
}
