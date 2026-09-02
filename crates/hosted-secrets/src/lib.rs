//! Remote `connector-secrets` port over the shared Secrets HTTP API.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use connector_secrets::{
    CredentialRef, CredentialScope, Secret, SecretBatch, SecretStore, StoreError,
};
use connectors_config::HostedSecretsConfig;
use reqwest::{Certificate, Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use url::Url;
use uuid::Uuid;

const MAX_TOKEN_BYTES: u64 = 64 * 1024;

#[derive(Clone)]
pub struct HostedSecretsStore {
    client: Client,
    origin: Url,
    token_file: PathBuf,
}

impl HostedSecretsStore {
    pub fn new(config: &HostedSecretsConfig) -> Result<Self, StoreError> {
        let origin_text = config
            .origin
            .as_deref()
            .ok_or_else(|| backend("<secrets>", "missing origin"))?;
        let origin = Url::parse(origin_text).map_err(|_| backend("<secrets>", "invalid origin"))?;
        if !matches!(origin.scheme(), "http" | "https")
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
        {
            return Err(backend("<secrets>", "invalid origin"));
        }
        let token_file = config
            .token_file
            .clone()
            .ok_or_else(|| backend("<secrets>", "missing projected token file"))?;
        let mut builder = Client::builder();
        if let Some(path) = &config.ca_file {
            let bytes = bounded_read(path, 1024 * 1024)
                .map_err(|_| backend("<secrets>", "CA file could not be read"))?;
            builder = builder.add_root_certificate(
                Certificate::from_pem(&bytes)
                    .map_err(|_| backend("<secrets>", "CA file is invalid"))?,
            );
        }
        let client = builder
            .build()
            .map_err(|_| backend("<secrets>", "HTTP client could not be built"))?;
        Ok(Self {
            client,
            origin,
            token_file,
        })
    }

    async fn put_as(
        &self,
        reference: &CredentialRef,
        owner: &str,
        secret: &Secret,
    ) -> Result<(), StoreError> {
        let request = PutRequest {
            reference: wire_ref(reference),
            owner_subject: owner,
            value: STANDARD.encode(secret.expose_secret().as_bytes()),
            disclosure: "workload_only",
            labels: serde_json::json!({"managed_by":"connectors"}),
        };
        let _: Metadata = self
            .json(
                reqwest::Method::PUT,
                "v1/workload/secrets",
                Some(&request),
                &path(reference),
            )
            .await?;
        Ok(())
    }

    async fn json<B: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<&B>,
        path: &str,
    ) -> Result<R, StoreError> {
        let token = self.token(path)?;
        let mut request = self
            .client
            .request(method, self.url(endpoint, path)?)
            .bearer_auth(token);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| unreachable(path, "request failed"))?;
        classify(response.status(), path)?;
        response
            .json()
            .await
            .map_err(|_| backend(path, "response did not match the Secrets contract"))
    }

    async fn empty<B: Serialize + ?Sized>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<&B>,
        path: &str,
        missing_ok: bool,
    ) -> Result<(), StoreError> {
        let token = self.token(path)?;
        let mut request = self
            .client
            .request(method, self.url(endpoint, path)?)
            .bearer_auth(token);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| unreachable(path, "request failed"))?;
        if missing_ok && response.status() == StatusCode::NOT_FOUND {
            return Ok(());
        }
        classify(response.status(), path)
    }

    fn token(&self, path: &str) -> Result<String, StoreError> {
        let bytes = bounded_read(&self.token_file, MAX_TOKEN_BYTES)
            .map_err(|_| unreachable(path, "projected token could not be read"))?;
        let token =
            String::from_utf8(bytes).map_err(|_| backend(path, "projected token is not UTF-8"))?;
        let token = token.trim();
        if token.is_empty() {
            Err(backend(path, "projected token is empty"))
        } else {
            Ok(token.to_owned())
        }
    }

    fn url(&self, endpoint: &str, path: &str) -> Result<Url, StoreError> {
        self.origin
            .join(endpoint)
            .map_err(|_| backend(path, "endpoint could not be resolved"))
    }
}

#[async_trait]
impl SecretStore for HostedSecretsStore {
    async fn ready(&self) -> Result<(), StoreError> {
        let response = self
            .client
            .get(self.url("health/ready", "<secrets-ready>")?)
            .send()
            .await
            .map_err(|_| unreachable("<secrets-ready>", "request failed"))?;
        classify(response.status(), "<secrets-ready>")
    }

    async fn get(&self, reference: &CredentialRef) -> Result<Secret, StoreError> {
        let response: GetResponse = self
            .json(
                reqwest::Method::POST,
                "v1/workload/secrets:get",
                Some(&wire_ref(reference)),
                &path(reference),
            )
            .await?;
        let bytes = STANDARD
            .decode(response.value)
            .map_err(|_| backend(&path(reference), "value encoding is invalid"))?;
        let value = String::from_utf8(bytes)
            .map_err(|_| backend(&path(reference), "value is not UTF-8"))?;
        Ok(Secret::new(value))
    }

    async fn put(&self, reference: &CredentialRef, secret: &Secret) -> Result<(), StoreError> {
        self.put_as(reference, "", secret).await
    }

    async fn put_owned(
        &self,
        reference: &CredentialRef,
        owner_subject: &str,
        secret: &Secret,
    ) -> Result<(), StoreError> {
        self.put_as(reference, owner_subject, secret).await
    }

    async fn delete(&self, reference: &CredentialRef) -> Result<(), StoreError> {
        let request = DeleteRequest {
            reference: wire_ref(reference),
            actor: "connectors",
        };
        self.empty(
            reqwest::Method::POST,
            "v1/workload/secrets:delete",
            Some(&request),
            &path(reference),
            true,
        )
        .await
    }

    async fn exists(&self, reference: &CredentialRef) -> Result<bool, StoreError> {
        let response: ExistsResponse = self
            .json(
                reqwest::Method::POST,
                "v1/workload/secrets:exists",
                Some(&wire_ref(reference)),
                &path(reference),
            )
            .await?;
        Ok(response.exists)
    }

    async fn references(&self, scope: &CredentialScope) -> Result<Vec<CredentialRef>, StoreError> {
        let request = ScopeRequest {
            tenant: scope.tenant(),
            namespace: scope.authority(),
        };
        let response: ListResponse = self
            .json(
                reqwest::Method::POST,
                "v1/workload/secrets:list",
                Some(&request),
                "<secrets-list>",
            )
            .await?;
        response
            .secrets
            .into_iter()
            .map(|metadata| parse_ref(&metadata.reference))
            .collect()
    }

    async fn apply(&self, batch: &SecretBatch) -> Result<(), StoreError> {
        let entries = batch.put_entries().ok_or_else(|| StoreError::Unsupported {
            operation: "atomic batch".into(),
            reason: "the remote adapter admits put-only prepared generations".into(),
        })?;
        let mutations = entries
            .into_iter()
            .map(|(reference, secret)| Mutation::Put {
                secret: PutRequest {
                    reference: wire_ref(reference),
                    owner_subject: "",
                    value: STANDARD.encode(secret.expose_secret().as_bytes()),
                    disclosure: "workload_only",
                    labels: serde_json::json!({"managed_by":"connectors"}),
                },
            })
            .collect::<Vec<_>>();
        let transaction = Uuid::now_v7();
        let endpoint = format!(
            "v1/workload/tenants/{}/transactions/{transaction}",
            batch.scope().tenant()
        );
        self.empty(
            reqwest::Method::PUT,
            &endpoint,
            Some(&PrepareRequest {
                actor: "connectors",
                mutations: &mutations,
            }),
            "<secrets-transaction>",
            false,
        )
        .await?;
        let commit = format!("{endpoint}:commit");
        self.empty::<()>(
            reqwest::Method::POST,
            &commit,
            None,
            "<secrets-transaction>",
            false,
        )
        .await
    }
}

#[derive(Serialize)]
struct WireRef<'a> {
    tenant: &'a str,
    namespace: &'a str,
    key: String,
}
#[derive(Deserialize)]
struct OwnedWireRef {
    tenant: String,
    namespace: String,
    key: String,
}
#[derive(Serialize)]
struct PutRequest<'a> {
    reference: WireRef<'a>,
    owner_subject: &'a str,
    value: String,
    disclosure: &'static str,
    labels: serde_json::Value,
}
#[derive(Serialize)]
struct DeleteRequest<'a> {
    reference: WireRef<'a>,
    actor: &'static str,
}
#[derive(Serialize)]
struct ScopeRequest<'a> {
    tenant: &'a str,
    namespace: &'a str,
}
#[derive(Serialize)]
struct PrepareRequest<'a> {
    actor: &'static str,
    mutations: &'a [Mutation<'a>],
}
#[derive(Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Mutation<'a> {
    Put { secret: PutRequest<'a> },
}
#[derive(Deserialize)]
struct Metadata {
    reference: OwnedWireRef,
}
#[derive(Deserialize)]
struct GetResponse {
    value: String,
}
#[derive(Deserialize)]
struct ExistsResponse {
    exists: bool,
}
#[derive(Deserialize)]
struct ListResponse {
    secrets: Vec<Metadata>,
}

fn wire_ref(reference: &CredentialRef) -> WireRef<'_> {
    let key = match reference.instance() {
        Some(instance) => format!(
            "instance/{}/{}/{}",
            instance.as_str(),
            reference.service(),
            reference.credential()
        ),
        None => format!("single/{}/{}", reference.service(), reference.credential()),
    };
    WireRef {
        tenant: reference.tenant(),
        namespace: reference.authority(),
        key,
    }
}
fn parse_ref(reference: &OwnedWireRef) -> Result<CredentialRef, StoreError> {
    let parts = reference.key.split('/').collect::<Vec<_>>();
    match parts.as_slice() {
        ["single", service, credential] => {
            CredentialRef::new(&reference.tenant, &reference.namespace, service, credential)
                .map_err(|_| backend("<secrets-list>", "reference is invalid"))
        }
        ["instance", instance, service, credential] => CredentialRef::for_instance(
            &reference.tenant,
            &reference.namespace,
            instance,
            service,
            credential,
        )
        .map_err(|_| backend("<secrets-list>", "reference is invalid")),
        _ => Err(backend("<secrets-list>", "reference key is invalid")),
    }
}
fn path(reference: &CredentialRef) -> String {
    format!(
        "{}/{}/{}",
        reference.tenant(),
        reference.authority(),
        wire_ref(reference).key
    )
}
fn bounded_read(path: &Path, maximum: u64) -> std::io::Result<Vec<u8>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > maximum {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file is too large",
        ));
    }
    fs::read(path)
}
fn classify(status: StatusCode, path: &str) -> Result<(), StoreError> {
    if status.is_success() {
        Ok(())
    } else if status == StatusCode::NOT_FOUND {
        Err(StoreError::NotFound { path: path.into() })
    } else if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
        Err(StoreError::Denied {
            path: path.into(),
            reason: "request was refused".into(),
        })
    } else if status.is_server_error() {
        Err(unreachable(path, "service failed"))
    } else {
        Err(backend(path, "request was rejected"))
    }
}
fn backend(path: &str, reason: &str) -> StoreError {
    StoreError::Backend {
        path: path.into(),
        reason: reason.into(),
    }
}
fn unreachable(path: &str, reason: &str) -> StoreError {
    StoreError::Unreachable {
        path: path.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_reference_round_trips() {
        let input = CredentialRef::for_instance(
            "tenant",
            "com.example.api",
            "018f2f43-80ab-7e79-8f34-123456789abc",
            "default",
            "token",
        )
        .unwrap();
        let wire = wire_ref(&input);
        let owned = OwnedWireRef {
            tenant: wire.tenant.into(),
            namespace: wire.namespace.into(),
            key: wire.key,
        };
        assert_eq!(parse_ref(&owned).unwrap(), input);
    }
}
