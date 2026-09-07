//! Endpoint discovery arguments and shared local/hosted client dispatch.

use protocol::endpoint::{self, EndpointRequest};

use super::*;

#[derive(Debug, clap::Args)]
pub(super) struct LocalOptions {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    state_root: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(super) enum EndpointCommand {
    /// List discovered service interfaces, including those without a supported driver.
    List {
        #[command(flatten)]
        local: LocalOptions,
        #[arg(long)]
        source: Option<String>,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = endpoint::MAX_RESULTS,
            value_parser = clap::value_parser!(u16).range(1..=i64::from(endpoint::MAX_RESULTS)))]
        limit: u16,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Inspect one discovered interface and its readiness reasons.
    Show {
        #[command(flatten)]
        local: LocalOptions,
        #[arg(long)]
        endpoint_ref: String,
    },
    /// Refresh a discovery source, or every configured source.
    Refresh {
        #[command(flatten)]
        local: LocalOptions,
        #[arg(long)]
        source: Option<String>,
    },
    /// Bind a discovered interface to a provider and named credential references.
    Bind {
        #[command(flatten)]
        local: LocalOptions,
        #[arg(long)]
        endpoint_ref: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        base_path: Option<String>,
        /// HTTP scheme for a bound interface, including Kubernetes port-forward routes.
        #[arg(long, value_parser = ["http", "https"])]
        scheme: Option<String>,
        /// Explicitly approved direct destination, for an externally routed interface.
        #[arg(long)]
        direct_address: Option<String>,
        /// Database name for a SQL interface when discovery does not supply it.
        #[arg(long)]
        database: Option<String>,
        /// SQL TLS policy. Required unless explicitly disabled here.
        #[arg(long, value_parser = ["required", "disabled"])]
        tls: Option<String>,
        /// Named Kubernetes Secret as namespace/name; never a credential value.
        #[arg(long, value_parser = secret_reference, requires = "credential_keys")]
        credential_secret: Option<(String, String)>,
        /// Declared credential field=Secret key. Repeat for multi-field credentials.
        #[arg(long = "credential-key", value_parser = enrol::parse_setting, requires = "credential_secret")]
        credential_keys: Vec<(String, String)>,
    },
}

fn secret_reference(value: &str) -> Result<(String, String), String> {
    let (namespace, name) = value.split_once('/').ok_or("expected namespace/name")?;
    if namespace.is_empty() || name.is_empty() || name.contains('/') {
        return Err("expected namespace/name".into());
    }
    Ok((namespace.to_owned(), name.to_owned()))
}

pub(super) async fn run(
    format: Format,
    target: Target,
    command: EndpointCommand,
) -> Result<(), MainError> {
    let (local, request) = match command {
        EndpointCommand::List {
            local,
            source,
            query,
            limit,
            cursor,
        } => (
            local,
            EndpointRequest::List(endpoint::ListRequest {
                source_ref: source,
                query,
                limit,
                cursor,
            }),
        ),
        EndpointCommand::Show {
            local,
            endpoint_ref,
        } => (
            local,
            EndpointRequest::Show(endpoint::ShowRequest { endpoint_ref }),
        ),
        EndpointCommand::Refresh { local, source } => (
            local,
            EndpointRequest::Refresh(endpoint::RefreshRequest { source_ref: source }),
        ),
        EndpointCommand::Bind {
            local,
            endpoint_ref,
            provider,
            base_path,
            scheme,
            direct_address,
            database,
            tls,
            credential_secret,
            credential_keys,
        } => {
            let mut keys = std::collections::BTreeMap::new();
            for (field, key) in credential_keys {
                if keys.insert(field, key).is_some() {
                    return Err(MainError::Client(
                        connectors_client::ClientError::InvalidRequest(
                            "credential fields must be unique".into(),
                        ),
                    ));
                }
            }
            let credential = credential_secret.map(|(namespace, name)| {
                endpoint::EndpointCredentialReference::KubernetesSecret {
                    namespace,
                    name,
                    keys,
                }
            });
            (
                local,
                EndpointRequest::Bind(endpoint::BindRequest {
                    endpoint_ref,
                    binding: endpoint::EndpointBinding {
                        provider,
                        base_path,
                        credential,
                        direct_address,
                        scheme: scheme.map(|value| {
                            if value == "https" {
                                endpoint::EndpointScheme::Https
                            } else {
                                endpoint::EndpointScheme::Http
                            }
                        }),
                        database,
                        tls: tls.map(|value| {
                            if value == "disabled" {
                                endpoint::EndpointTls::Disabled
                            } else {
                                endpoint::EndpointTls::Required
                            }
                        }),
                    },
                }),
            )
        }
    };
    target.validate(&local.config, &local.state_root)?;
    let response = if target == Target::Hosted {
        AuthenticatedHostedClient::active()?
            .endpoint(request)
            .await?
    } else {
        let config = read_config(local.config)?;
        let root = local.state_root.map_or_else(default_state_root, Ok)?;
        connectors_console::daemon::require(&root).await?;
        LocalClient::new(root.join("connectors.sock"))
            .endpoint(&config.owner_context(), request)
            .await?
    };
    emit_targeted(format, &reduce_envelope!(response)?, target.as_str())
}
