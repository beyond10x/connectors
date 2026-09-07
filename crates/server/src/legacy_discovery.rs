//! Migration boundary for predecessor discovery wire methods.

use protocol::endpoint::{EndpointError, EndpointErrorCode, EndpointRequest};

pub(crate) fn admit(request: &EndpointRequest) -> Result<(), EndpointError> {
    if matches!(
        request,
        EndpointRequest::CandidateSearch(_)
            | EndpointRequest::CandidateActivate(_)
            | EndpointRequest::ObservationSearch(_)
            | EndpointRequest::Materialize(_)
    ) {
        return Err(EndpointError::new(EndpointErrorCode::Unavailable,
            "legacy discovery is retired; configure the source with setup connect, then use the endpoint list, show, refresh and bind methods and operation v0alpha5 endpoint_ref",
            false));
    }
    Ok(())
}
