//! One deterministic catalog projection shared by local and hosted transports.

use std::sync::OnceLock;

use protocol::catalog::{
    CatalogOperationSummary, CatalogRequest, CatalogResult, ProviderDescription, ProviderSummary,
    SetupProfileSummary,
};
use service::ConnectorBackend;

static CATALOG_READINESS: OnceLock<Result<(), protocol::catalog::CatalogError>> = OnceLock::new();

/// What a deployment can actually complete for a person who is not its operator.
///
/// The catalog itself is static, so it can say a provider is configurable but never whether this
/// deployment holds the configuration a self-service flow needs. Only the running backend knows
/// that, so it is asked per provider and its answer travels with the summary.
pub(crate) trait SetupProfileSource {
    fn setup_profiles(&self, provider_ref: &str) -> Vec<SetupProfileSummary>;
}

impl<T: ConnectorBackend + ?Sized> SetupProfileSource for T {
    fn setup_profiles(&self, provider_ref: &str) -> Vec<SetupProfileSummary> {
        ConnectorBackend::setup_profiles(self, provider_ref)
    }
}

/// The answer for a placement that publishes no self-service flow at all.
pub(crate) struct NoSelfServiceSetup;

impl SetupProfileSource for NoSelfServiceSetup {
    fn setup_profiles(&self, _provider_ref: &str) -> Vec<SetupProfileSummary> {
        Vec::new()
    }
}

pub(crate) fn handle<S: SetupProfileSource + ?Sized>(
    request: CatalogRequest,
    setup: &S,
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
                .map(|provider| summary(provider, setup))
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
                provider: summary(provider, setup),
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
    // Readiness checks the static projection; a deployment's self-service flows are a runtime
    // answer that cannot make a shipped catalog invalid.
    let setup = &NoSelfServiceSetup;
    let search = handle(
        CatalogRequest::Search(protocol::catalog::SearchRequest {
            query: String::new(),
            offset: 0,
            limit: protocol::catalog::MAX_PROVIDER_RESULTS,
        }),
        setup,
    )?;
    protocol::catalog::ResponseEnvelope::success("catalog-readiness-search", search).validate()?;

    for provider in catalog::providers() {
        let request_id = format!("catalog-readiness-{}", provider.id);
        let result = handle(
            CatalogRequest::Describe(protocol::catalog::DescribeRequest {
                provider_ref: provider.id.to_owned(),
            }),
            setup,
        )?;
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

fn summary<S: SetupProfileSource + ?Sized>(
    provider: &catalog::Provider,
    setup: &S,
) -> ProviderSummary {
    // This is runtime setup availability, not merely the existence of declarative config fields.
    // Only built-in integrations that own Connect Session dispatch may advertise it.
    let configurable = matches!(provider.id, "grafana" | "slack");
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
        configurable,
        setup_profiles: if configurable {
            setup.setup_profiles(provider.id)
        } else {
            Vec::new()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixtureSetup(Vec<SetupProfileSummary>);

    impl SetupProfileSource for FixtureSetup {
        fn setup_profiles(&self, provider_ref: &str) -> Vec<SetupProfileSummary> {
            if provider_ref == "slack" {
                self.0.clone()
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn search_is_whole_catalog_and_describe_is_descriptive_only() {
        let CatalogResult::Search { providers, .. } = handle(
            CatalogRequest::Search(protocol::catalog::SearchRequest {
                query: "slack".to_owned(),
                offset: 0,
                limit: 10,
            }),
            &NoSelfServiceSetup,
        )
        .unwrap() else {
            panic!("wrong catalog result");
        };
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider_ref, "slack");
        assert!(providers[0].configurable);

        let gitlab = catalog::provider(catalog::ProviderKey::id("gitlab")).unwrap();
        assert!(!gitlab.config.is_empty());
        assert!(!summary(gitlab, &NoSelfServiceSetup).configurable);

        let CatalogResult::Describe(description) = handle(
            CatalogRequest::Describe(protocol::catalog::DescribeRequest {
                provider_ref: "slack".to_owned(),
            }),
            &NoSelfServiceSetup,
        )
        .unwrap() else {
            panic!("wrong catalog result");
        };
        let json = serde_json::to_string(&description).unwrap();
        assert!(!json.contains("callable"));
        assert!(!json.contains("connection_ref"));
    }

    #[test]
    fn a_deployment_publishes_only_the_setup_flows_it_can_complete() {
        // A configurable provider whose deployment holds no self-service configuration publishes
        // no profile, and that is what lets a product show no control rather than a dead one.
        let bare = summary(
            catalog::provider(catalog::ProviderKey::id("slack")).unwrap(),
            &NoSelfServiceSetup,
        );
        assert!(bare.configurable);
        assert!(bare.setup_profiles.is_empty());

        let configured = FixtureSetup(vec![SetupProfileSummary {
            auth_profile: "slack.org_user".to_owned(),
            actor: protocol::catalog::SetupProfileActor::Person,
        }]);
        let published = summary(
            catalog::provider(catalog::ProviderKey::id("slack")).unwrap(),
            &configured,
        );
        assert_eq!(published.setup_profiles.len(), 1);
        assert_eq!(published.setup_profiles[0].auth_profile, "slack.org_user");
        protocol::catalog::ResponseEnvelope::success(
            "catalog-setup-profiles-test",
            CatalogResult::Search {
                providers: vec![published],
                next_offset: None,
            },
        )
        .validate()
        .unwrap();

        // A provider that publishes no setup form at all can never carry a profile, whatever a
        // backend claims: the wire contract refuses the pair.
        let grafana_profile = summary(
            catalog::provider(catalog::ProviderKey::id("gitlab")).unwrap(),
            &configured,
        );
        assert!(!grafana_profile.configurable);
        assert!(grafana_profile.setup_profiles.is_empty());
    }

    #[test]
    fn platform_provider_satisfies_the_catalog_wire_contract() {
        let result = handle(
            CatalogRequest::Describe(protocol::catalog::DescribeRequest {
                provider_ref: "b10x".to_owned(),
            }),
            &NoSelfServiceSetup,
        )
        .unwrap();
        protocol::catalog::ResponseEnvelope::success("catalog-platform-test", result)
            .validate()
            .unwrap();
    }

    #[test]
    fn every_shipped_provider_description_satisfies_the_catalog_wire_contract() {
        ready().unwrap();
    }
}
