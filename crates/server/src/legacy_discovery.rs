//! Migration boundary for predecessor discovery wire methods.

use protocol::connection::{ConnectionError, ConnectionErrorCode, ConnectionRequest};

pub(crate) fn admit(request: &ConnectionRequest) -> Result<(), ConnectionError> {
    if matches!(
        request,
        ConnectionRequest::CandidateSearch(_)
            | ConnectionRequest::CandidateActivate(_)
            | ConnectionRequest::ObservationSearch(_)
            | ConnectionRequest::Materialize(_)
    ) {
        return Err(ConnectionError::new(ConnectionErrorCode::Unavailable,
            "legacy discovery is retired; configure the source with setup connect, then use the endpoint list, show, refresh and bind methods and operation v0alpha4 endpoint_ref",
            false));
    }
    Ok(())
}
