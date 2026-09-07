//! Resolve deployment-owned endpoint bindings before acquisition or egress admission.

use std::collections::{BTreeMap, BTreeSet};

use connector_address::HttpsOrigin;
use connectors_config::{HostedCatalogBinding, HostedCatalogConfig, NetworkScopeConfig};

use crate::{DeclaredConfig, HostedCatalogError};

pub(crate) fn bindings(
    policy: &HostedCatalogConfig,
) -> Result<BTreeMap<String, HostedCatalogBinding>, HostedCatalogError> {
    for provider in &policy.providers {
        if catalog::provider(catalog::ProviderKey::id(provider)).is_none() {
            return Err(HostedCatalogError::InvalidPolicy);
        }
    }
    policy
        .bindings
        .iter()
        .map(|(id, supplied)| {
            if !policy.enabled || (!policy.providers.is_empty() && !policy.providers.contains(id)) {
                return Err(HostedCatalogError::InvalidPolicy);
            }
            let provider = catalog::provider(catalog::ProviderKey::id(id))
                .ok_or(HostedCatalogError::InvalidPolicy)?;
            if provider.authority.is_none()
                || provider.verify.is_none()
                || provider.auth.iter().all(|credential| {
                    !matches!(credential.acquire, catalog::Acquisition::ConnectSession)
                        || matches!(credential.subject, catalog::Subject::Unstated)
                })
            {
                return Err(HostedCatalogError::InvalidPolicy);
            }
            let mut binding = supplied.clone();
            let config = DeclaredConfig::new(supplied.endpoints.clone(), true);
            let mut used = BTreeSet::new();
            for service in provider.services {
                for tail in service.base_url.split('{').skip(1) {
                    let (variable, _) = tail
                        .split_once('}')
                        .ok_or(HostedCatalogError::InvalidPolicy)?;
                    let declaration = provider.config.iter().find(|field| {
                        field.service == service.name
                            && field.binds == format!("endpoint.{variable}")
                            && !field.secret
                    });
                    let declaration = declaration.ok_or(HostedCatalogError::InvalidPolicy)?;
                    // A templated hosted destination is always an explicit deployment choice,
                    // even where the personal catalogue resolver offers a vendor default.
                    if !supplied.endpoints.contains_key(variable) {
                        return Err(HostedCatalogError::InvalidPolicy);
                    }
                    let value = connector_resolve::resolve_endpoint(
                        provider.verify.expect("checked verification operation"),
                        provider,
                        service.name,
                        "deployment",
                        variable,
                        &config,
                    )
                    .map_err(|_| HostedCatalogError::InvalidPolicy)?;
                    // Origin fields have the shared HTTPS parser. Other endpoint slots are
                    // hostname components, not a way to replace the catalogue-owned URL path.
                    if declaration.format != "origin"
                        && !value.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')
                        })
                    {
                        return Err(HostedCatalogError::InvalidPolicy);
                    }
                    if used.contains(variable) && binding.endpoints.get(variable) != Some(&value) {
                        return Err(HostedCatalogError::InvalidPolicy);
                    }
                    used.insert(variable.to_owned());
                    binding.endpoints.insert(variable.to_owned(), value);
                }
            }
            if supplied.endpoints.keys().any(|key| !used.contains(key)) {
                return Err(HostedCatalogError::InvalidPolicy);
            }
            origins(provider, &binding)?;
            Ok((id.clone(), binding))
        })
        .collect()
}

pub(crate) fn origins(
    provider: &catalog::Provider,
    binding: &HostedCatalogBinding,
) -> Result<Vec<String>, HostedCatalogError> {
    let mut origins = BTreeSet::new();
    for service in provider.services {
        let mut base = service.base_url.to_owned();
        for (name, value) in &binding.endpoints {
            base = base.replace(&format!("{{{name}}}"), value);
        }
        if base.contains(['{', '}']) {
            return Err(HostedCatalogError::InvalidPolicy);
        }
        let url = url::Url::parse(&base).map_err(|_| HostedCatalogError::InvalidPolicy)?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(HostedCatalogError::InvalidPolicy);
        }
        let origin = HttpsOrigin::parse(&url.origin().ascii_serialization())
            .map_err(|_| HostedCatalogError::InvalidPolicy)?;
        origins.insert(origin.into_string());
    }
    Ok(origins.into_iter().collect())
}

/// Exact destinations and post-DNS network scope admitted by hosted deployment policy.
/// Conflicting scope declarations for the same origin refuse instead of silently widening it.
pub fn hosted_admitted_destinations(
    policy: &HostedCatalogConfig,
) -> Result<Vec<(String, NetworkScopeConfig)>, HostedCatalogError> {
    let bindings = bindings(policy)?;
    let mut destinations = BTreeMap::new();
    for provider in catalog::providers() {
        if (!policy.providers.is_empty() && !policy.providers.iter().any(|id| id == provider.id))
            || provider.auth.iter().all(|credential| {
                !matches!(credential.acquire, catalog::Acquisition::ConnectSession)
                    || matches!(credential.subject, catalog::Subject::Unstated)
            })
        {
            continue;
        }
        let binding = bindings.get(provider.id).cloned().unwrap_or_default();
        if provider
            .services
            .iter()
            .any(|service| service.base_url.contains('{'))
            && !bindings.contains_key(provider.id)
        {
            continue;
        }
        for origin in origins(provider, &binding)? {
            if destinations
                .insert(origin, binding.network)
                .is_some_and(|previous| previous != binding.network)
            {
                return Err(HostedCatalogError::InvalidPolicy);
            }
        }
    }
    Ok(destinations.into_iter().collect())
}
