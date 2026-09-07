//! Crossplane database resources join the same endpoint plane without reading Secret contents.

use std::collections::{BTreeMap, BTreeSet};

use domain::endpoint::{
    Endpoint, EndpointBinding, EndpointCredentialReference, EndpointState, EndpointTransport,
};
use kube::api::{ApiResource, DynamicObject, ListParams};
use kube::Api;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    dns_name, routes::source_error, EndpointScan, EndpointSourceError, KubernetesEndpointSource,
    MAX_PAGES, PAGE_SIZE,
};

impl KubernetesEndpointSource {
    pub(super) async fn discover_crossplane(&self, scan: &mut EndpointScan) {
        for engine in ["mysql", "postgresql"] {
            let configs = match self
                .crossplane_list(engine, "ProviderConfig", "providerconfigs")
                .await
            {
                Ok(configs) => configs,
                Err(error) => {
                    scan.complete = false;
                    scan.warnings
                        .push(format!("{engine} ProviderConfig discovery {error}"));
                    continue;
                }
            };
            let databases = match self.crossplane_list(engine, "Database", "databases").await {
                Ok(databases) => databases,
                Err(error) => {
                    scan.complete = false;
                    scan.warnings
                        .push(format!("{engine} database discovery {error}"));
                    continue;
                }
            };
            for database in &databases {
                if let Some(endpoint) =
                    project_database(&self.source_ref, engine, database, &configs, |namespace| {
                        self.admits_namespace(namespace)
                    })
                {
                    scan.endpoints.push(endpoint);
                }
            }
        }
    }

    async fn crossplane_list(
        &self,
        engine: &str,
        kind: &str,
        plural: &str,
    ) -> Result<Vec<DynamicObject>, EndpointSourceError> {
        let resource = resource(engine, kind, plural);
        let api: Api<DynamicObject> = Api::all_with(self.client.clone(), &resource);
        let mut result = Vec::new();
        let mut cursor = None;
        let mut seen = BTreeSet::new();
        for _ in 0..MAX_PAGES {
            let mut params = ListParams::default().limit(PAGE_SIZE);
            if let Some(cursor) = cursor.as_deref() {
                params = params.continue_token(cursor);
            }
            let list = match api.list(&params).await {
                Ok(list) => list,
                Err(kube::Error::Api(response)) if response.code == 404 && cursor.is_none() => {
                    // Distinguish an absent CRD from an incorrectly served collection. A known
                    // collection that returned 404 is incomplete, not an empty successful scan.
                    let request =
                        http::Request::get(format!("/apis/{engine}.sql.crossplane.io/v1alpha1"))
                            .body(Vec::new())
                            .map_err(|_| EndpointSourceError::Unavailable)?;
                    match self.client.request::<Value>(request).await {
                        Err(kube::Error::Api(response)) if response.code == 404 => {
                            return Ok(Vec::new())
                        }
                        Ok(discovery)
                            if !discovery
                                .get("resources")
                                .and_then(Value::as_array)
                                .is_some_and(|resources| {
                                    resources.iter().any(|resource| {
                                        resource.get("name").and_then(Value::as_str) == Some(plural)
                                    })
                                }) =>
                        {
                            return Ok(Vec::new())
                        }
                        _ => return Err(EndpointSourceError::Unavailable),
                    }
                }
                Err(error) => return Err(source_error(error)),
            };
            if list.items.len() > PAGE_SIZE as usize {
                return Err(EndpointSourceError::Capacity);
            }
            result.extend(list.items);
            cursor = list.metadata.continue_.filter(|value| !value.is_empty());
            match &cursor {
                None => return Ok(result),
                Some(value) if !seen.insert(value.clone()) => {
                    return Err(EndpointSourceError::Unavailable)
                }
                _ => {}
            }
        }
        Err(EndpointSourceError::Capacity)
    }

    pub(super) async fn validate_crossplane(
        &self,
        endpoint: Endpoint,
    ) -> Result<Endpoint, EndpointSourceError> {
        let engine = match endpoint.resource_kind.as_str() {
            "PostgresqlDatabase" => "postgresql",
            "MysqlDatabase" => "mysql",
            _ => return Err(EndpointSourceError::Stale),
        };
        let api: Api<DynamicObject> = Api::all_with(
            self.client.clone(),
            &resource(engine, "Database", "databases"),
        );
        let database = api
            .get(&endpoint.resource_name)
            .await
            .map_err(source_error)?;
        if database.metadata.uid.as_deref() != Some(&endpoint.resource_uid) {
            return Err(EndpointSourceError::Stale);
        }
        let config_name = database
            .data
            .pointer("/spec/providerConfigRef/name")
            .and_then(Value::as_str)
            .ok_or(EndpointSourceError::Stale)?;
        if !dns_name(config_name) {
            return Err(EndpointSourceError::Stale);
        }
        let api: Api<DynamicObject> = Api::all_with(
            self.client.clone(),
            &resource(engine, "ProviderConfig", "providerconfigs"),
        );
        let config = api.get(config_name).await.map_err(source_error)?;
        let current = project_database(
            &self.source_ref,
            engine,
            &database,
            &[config],
            |namespace| self.admits_namespace(namespace),
        )
        .ok_or(EndpointSourceError::Denied)?;
        if current.endpoint_ref != endpoint.endpoint_ref {
            return Err(EndpointSourceError::Stale);
        }
        // An implicit ProviderConfig binding must still name the same Secret at invocation.
        let explicit = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?
            .bindings
            .contains_key(&endpoint.endpoint_ref);
        if !explicit && current.binding != endpoint.binding {
            return Err(EndpointSourceError::Stale);
        }
        Ok(endpoint)
    }
}

fn resource(engine: &str, kind: &str, plural: &str) -> ApiResource {
    ApiResource {
        group: format!("{engine}.sql.crossplane.io"),
        version: "v1alpha1".to_owned(),
        api_version: format!("{engine}.sql.crossplane.io/v1alpha1"),
        kind: kind.to_owned(),
        plural: plural.to_owned(),
    }
}

fn project_database(
    source: &str,
    engine: &str,
    database: &DynamicObject,
    configs: &[DynamicObject],
    admits: impl Fn(&str) -> bool,
) -> Option<Endpoint> {
    let name = database.metadata.name.as_deref()?;
    let uid = database.metadata.uid.as_deref()?;
    if !dns_name(name) || uid.is_empty() {
        return None;
    }
    let config_name = database
        .data
        .pointer("/spec/providerConfigRef/name")?
        .as_str()?;
    let config = configs
        .iter()
        .find(|config| config.metadata.name.as_deref() == Some(config_name))?;
    let secret = config
        .data
        .pointer("/spec/credentials/connectionSecretRef")?;
    let secret_name = secret.get("name")?.as_str()?;
    let namespace = secret.get("namespace")?.as_str()?;
    if !dns_name(secret_name) || !admits(namespace) {
        return None;
    }
    let actual_name = database
        .metadata
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get("crossplane.io/external-name"))
        .filter(|name| !name.is_empty() && name.len() <= 512)
        .cloned();
    let identity = format!("{source}\0{engine}\0{name}\0{uid}");
    Some(Endpoint {
        endpoint_ref: format!(
            "endpoint:kubernetes:{}",
            hex::encode(Sha256::digest(identity.as_bytes()))
        ),
        source_ref: source.to_owned(),
        namespace: None,
        resource_kind: if engine == "mysql" {
            "MysqlDatabase"
        } else {
            "PostgresqlDatabase"
        }
        .to_owned(),
        resource_name: name.to_owned(),
        resource_uid: uid.to_owned(),
        port_name: None,
        port: None,
        transport: EndpointTransport::Tcp,
        interface: engine.to_owned(),
        provider: Some(engine.to_owned()),
        state: EndpointState::UnavailableRoute,
        binding: Some(EndpointBinding {
            provider: engine.to_owned(),
            base_path: None,
            direct_address: None,
            database: actual_name,
            tls: None,
            scheme: None,
            credential: Some(EndpointCredentialReference::KubernetesSecret {
                namespace: namespace.to_owned(),
                name: secret_name.to_owned(),
                keys: BTreeMap::from([
                    ("username".to_owned(), "username".to_owned()),
                    ("password".to_owned(), "password".to_owned()),
                ]),
            }),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_database_uses_external_identity_and_named_secret_namespace() {
        let database: DynamicObject = serde_json::from_value(serde_json::json!({
            "metadata":{"name":"managed-db","uid":"database-uid","annotations":{"crossplane.io/external-name":"actual_db"}},
            "spec":{"providerConfigRef":{"name":"provider"}}
        })).unwrap();
        let config: DynamicObject = serde_json::from_value(serde_json::json!({
            "metadata":{"name":"provider"},"spec":{"credentials":{"connectionSecretRef":{"name":"db-auth","namespace":"data"}}}
        })).unwrap();
        let endpoint = project_database(
            "source",
            "postgresql",
            &database,
            &[config.clone()],
            |namespace| namespace == "data",
        )
        .unwrap();
        assert_eq!(endpoint.namespace, None);
        assert_eq!(endpoint.port, None);
        assert_eq!(
            endpoint.binding.unwrap().database.as_deref(),
            Some("actual_db")
        );
        assert!(
            project_database("source", "postgresql", &database, &[config], |_| false).is_none()
        );
    }
}
