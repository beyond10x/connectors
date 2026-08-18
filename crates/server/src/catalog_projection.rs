//! One deterministic catalog projection shared by local and hosted transports.

use std::sync::OnceLock;

use protocol::catalog::{
    CatalogOperationSummary, CatalogRequest, CatalogResult, ProviderDescription, ProviderSummary,
};

static CATALOG_READINESS: OnceLock<Result<(), protocol::catalog::CatalogError>> = OnceLock::new();

pub(crate) fn handle(
    request: CatalogRequest,
) -> Result<CatalogResult, protocol::catalog::CatalogError> {
    match request {
        CatalogRequest::Search(request) => {
            let query = request.query.to_ascii_lowercase();
            let matching = catalog::providers()
                .iter()
                .copied()
                .filter(|provider| {
                    query.is_empty()
                        || provider.id.to_ascii_lowercase().contains(&query)
                        || provider.vendor.to_ascii_lowercase().contains(&query)
                        || provider.description.to_ascii_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            let offset = usize::from(request.offset);
            let providers = matching
                .iter()
                .skip(offset)
                .take(usize::from(request.limit))
                .map(|provider| summary(provider))
                .collect::<Vec<_>>();
            let consumed = offset.saturating_add(providers.len());
            let next_offset = (consumed < matching.len())
                .then(|| u16::try_from(consumed).ok())
                .flatten();
            Ok(CatalogResult::Search {
                providers,
                next_offset,
            })
        }
        CatalogRequest::Describe(request) => {
            let provider = catalog::provider(catalog::ProviderKey::id(&request.provider_ref))
                .ok_or_else(protocol::catalog::not_found)?;
            Ok(CatalogResult::Describe(ProviderDescription {
                provider: summary(provider),
                operations: provider
                    .operations
                    .iter()
                    .map(|operation| CatalogOperationSummary {
                        operation_ref: operation.id.to_owned(),
                        service: operation.service.to_owned(),
                        description: operation.description.to_owned(),
                        risk: operation.risk.as_str().to_owned(),
                        exposed: operation.expose,
                    })
                    .collect(),
            }))
        }
    }
}

/// Validate every deployment-shipped provider through the exact response contract consumers use.
///
/// This is intentionally part of readiness as well as the test suite: a generated catalog can
/// compile while still violating a semantic wire bound such as a required non-empty description.
/// A hosted deployment must not advertise readiness when every authenticated Zwirn session would
/// deterministically reject that catalog.
pub(crate) fn ready() -> Result<(), protocol::catalog::CatalogError> {
    CATALOG_READINESS.get_or_init(validate_catalog).clone()
}

fn validate_catalog() -> Result<(), protocol::catalog::CatalogError> {
    let search = handle(CatalogRequest::Search(protocol::catalog::SearchRequest {
        query: String::new(),
        offset: 0,
        limit: protocol::catalog::MAX_PROVIDER_RESULTS,
    }))?;
    protocol::catalog::ResponseEnvelope::success("catalog-readiness-search", search).validate()?;

    for provider in catalog::providers() {
        let request_id = format!("catalog-readiness-{}", provider.id);
        let result = handle(CatalogRequest::Describe(
            protocol::catalog::DescribeRequest {
                provider_ref: provider.id.to_owned(),
            },
        ))?;
        protocol::catalog::ResponseEnvelope::success(request_id, result)
            .validate()
            .map_err(|error| protocol::catalog::CatalogError {
                code: error.code,
                message: format!(
                    "catalog provider {:?} violates the response contract: {}",
                    provider.id, error.message
                ),
            })?;
    }
    Ok(())
}

fn summary(provider: &catalog::Provider) -> ProviderSummary {
    ProviderSummary {
        provider_ref: provider.id.to_owned(),
        authority: provider.authority.map(ToOwned::to_owned),
        vendor: provider.vendor.to_owned(),
        description: provider.description.to_owned(),
        audiences: provider
            .audiences
            .iter()
            .map(|audience| audience.as_str().to_owned())
            .collect(),
        services: provider
            .services
            .iter()
            .map(|service| service.name.to_owned())
            .collect(),
        operation_count: u32::try_from(provider.operations.len()).unwrap_or(u32::MAX),
        // This is runtime setup availability, not merely the existence of declarative config
        // fields. Only built-in integrations that own Connect Session dispatch may advertise it.
        configurable: matches!(provider.id, "grafana" | "slack"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_is_whole_catalog_and_describe_is_descriptive_only() {
        let CatalogResult::Search { providers, .. } =
            handle(CatalogRequest::Search(protocol::catalog::SearchRequest {
                query: "slack".to_owned(),
                offset: 0,
                limit: 10,
            }))
            .unwrap()
        else {
            panic!("wrong catalog result");
        };
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_ref, "slack");
        assert!(providers[0].configurable);

        let gitlab = catalog::provider(catalog::ProviderKey::id("gitlab")).unwrap();
        assert!(!gitlab.config.is_empty());
        assert!(!summary(gitlab).configurable);

        let CatalogResult::Describe(description) = handle(CatalogRequest::Describe(
            protocol::catalog::DescribeRequest {
                provider_ref: "slack".to_owned(),
            },
        ))
        .unwrap() else {
            panic!("wrong catalog result");
        };
        let json = serde_json::to_string(&description).unwrap();
        assert!(!json.contains("callable"));
        assert!(!json.contains("connection_ref"));
    }

    #[test]
    fn b10x_provider_satisfies_the_catalog_wire_contract() {
        let result = handle(CatalogRequest::Describe(
            protocol::catalog::DescribeRequest {
                provider_ref: "b10x".to_owned(),
            },
        ))
        .unwrap();
        protocol::catalog::ResponseEnvelope::success("catalog-b10x-test", result)
            .validate()
            .unwrap();
    }

    #[test]
    fn every_shipped_provider_description_satisfies_the_catalog_wire_contract() {
        ready().unwrap();
    }
}
