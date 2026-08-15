pub(crate) fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "connection management is temporarily unavailable",
        true,
    )
}

pub(crate) fn connect_session_error(error: ConnectSessionLifecycleError) -> ConnectionError {
    match error {
        ConnectSessionLifecycleError::Capacity => ConnectionError::new(
            ConnectionErrorCode::Conflict,
            "too many Connect Sessions are pending",
            true,
        ),
        _ => connection_unavailable(),
    }
}

pub(crate) fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "connection, observation, or Connect Session was not found",
        false,
    )
}

pub(crate) fn connection_protocol() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "connection backend returned an incompatible response",
        false,
    )
}

pub(crate) fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "monitoring connector runtime is unavailable",
        true,
    )
}

pub(crate) fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "monitoring operation was not found",
        false,
    )
}

pub(crate) fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Connection",
        false,
    )
}

pub(crate) fn operation_invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "operation input is outside the catalog contract",
        false,
    )
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        mutex.clear_poison();
        poisoned.into_inner()
    })
}
use std::sync::{Mutex, MutexGuard};

use protocol::connection::{ConnectionError, ConnectionErrorCode};
use protocol::operation::{OperationError, OperationErrorCode};
use service::ConnectSessionLifecycleError;
