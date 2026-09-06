//! The typed catalog-to-operation projection of advisory rates.

use protocol::operation::{
    ConditionalRateAdvice, ConditionalRateLimit, FixedRateLimit, OperationRateAdvice,
    PublishedRate, RateLimitBasis,
};

/// Carry every published alternative, deriving advisory spacing without selecting a category.
#[must_use]
pub fn operation_rate_advice(operation: &catalog::Operation) -> Option<OperationRateAdvice> {
    let fixed = operation.rate_limit.as_ref().map(|rate| FixedRateLimit {
        requests: rate.requests,
        per_seconds: rate.per_seconds,
        bucket: rate.bucket.clone(),
    });
    let alternatives = operation
        .conditional_rate_limits
        .iter()
        .map(|declaration| {
            ConditionalRateAdvice::new(ConditionalRateLimit {
                applies_when: declaration.applies_when.clone(),
                source_url: declaration.source_url.clone(),
                rate: declaration.rate.as_ref().map(|rate| PublishedRate {
                    requests: rate.requests,
                    per_seconds: rate.per_seconds,
                    basis: match rate.basis {
                        catalog::RateLimitBasis::MinimumAllowance => {
                            RateLimitBasis::MinimumAllowance
                        }
                        catalog::RateLimitBasis::Ceiling => RateLimitBasis::Ceiling,
                    },
                }),
            })
        })
        .collect::<Vec<_>>();
    (fixed.is_some() || !alternatives.is_empty()).then_some(OperationRateAdvice {
        fixed,
        alternatives,
    })
}
