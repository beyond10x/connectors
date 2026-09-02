//! Correlation and closed-envelope validation for Connector client responses.

use protocol::{catalog, connection, datasource, event, operation};

use crate::ClientError;

pub(crate) fn validate_catalog_response(
    response: catalog::ResponseEnvelope,
    request_id: &str,
) -> Result<catalog::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_operation_response(
    response: operation::ResponseEnvelope,
    request_id: &str,
) -> Result<operation::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_connection_response(
    response: connection::ResponseEnvelope,
    request_id: &str,
) -> Result<connection::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_event_response(
    response: event::ResponseEnvelope,
    request_id: &str,
) -> Result<event::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_datasource_response(
    response: datasource::ResponseEnvelope,
    request_id: &str,
) -> Result<datasource::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}
