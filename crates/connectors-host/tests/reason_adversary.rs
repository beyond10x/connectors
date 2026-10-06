//! Adversary case for story:service-failure-carries-upstream-reason.
//!
//! `contracts/cli/v1alpha1/semantics.md` admits `Failure.service_reason` only
//! "beside `service_code`, or on the provider's own `forbidden`" and says it is
//! "absent otherwise". The owner keeps it for every provider origin, so a child
//! that answers `provider_timeout` or `provider_capacity` with a `reason` puts
//! it on a `timeout` / `capacity` failure that has neither.
use connectors_host::local::{
    owner,
    runtime::{Failure, Refusal},
};

#[test]
fn a_reason_rides_only_beside_a_service_code_or_a_provider_forbidden() {
    let carried: Vec<_> = [Failure::ProviderTimeout, Failure::ProviderCapacity]
        .into_iter()
        .filter_map(|failure| {
            let error: owner::Error = Refusal {
                failure,
                reason: Some("Unauthorized; scope does not match".into()),
                retry_after_seconds: None,
            }
            .into();
            error
                .service_code
                .is_none()
                .then_some(())
                .and(error.service_reason)
                .map(|reason| format!("{failure:?} -> {:?}: {reason}", error.code))
        })
        .collect();
    assert!(
        carried.is_empty(),
        "a reason without a service_code or provider forbidden: {carried:#?}"
    );
}
