//! Adversary case for story:catalog-honours-retry-after.
//!
//! `contracts/cli/v1alpha1/semantics.md`: `Failure.retry_after_seconds` "is a
//! whole number of seconds from 0 to 4294967295; a value outside that range
//! ... is not read". `contracts/cli/v1alpha1/private-adapter.md` gives the
//! child's field the same range, and `ess/domains/cli.yaml` says "the range and
//! the presence rule are enforced by the host and pinned by tests". The host
//! decodes the child's value as `u64` and the owner keeps it unchecked.
use connectors_host::local::{
    owner,
    runtime::{Failure, Refusal},
};

#[test]
fn a_child_delay_beyond_32_bits_is_not_kept_by_the_host() {
    for out_of_range in [1_u64 << 32, u64::MAX] {
        let error: owner::Error = Refusal {
            failure: Failure::ProviderRateLimited,
            reason: None,
            retry_after_seconds: Some(out_of_range),
        }
        .into();
        assert_eq!(
            error.service_code,
            Some(connectors_core::ErrorCode::RateLimited)
        );
        assert_eq!(
            error.retry_after_seconds,
            None,
            "{out_of_range} reaches the CLI failure as {:?}",
            serde_json::to_string(&error).unwrap()
        );
    }
}
