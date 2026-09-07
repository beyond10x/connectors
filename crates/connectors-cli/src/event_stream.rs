//! Declared endpoint subscription lifecycle; provider routes remain daemon-owned.

use std::collections::BTreeMap;

use protocol::event::v2::{self, EventRequest, SubscribeRequest, UnsubscribeRequest};
use serde_json::Value;

use super::*;

#[derive(Debug, clap::Args)]
pub(super) struct Subscribe {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    state_root: Option<PathBuf>,
    #[arg(long)]
    endpoint_ref: String,
    /// Installed channel binding declared by the endpoint's provider.
    #[arg(long)]
    channel_binding: String,
    /// JSON object of declared channel parameters, at most 16 KiB.
    #[arg(long, default_value = "{}")]
    parameters_json: String,
}

#[derive(Debug, clap::Args)]
pub(super) struct Unsubscribe {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    state_root: Option<PathBuf>,
    #[arg(long)]
    subscription_ref: String,
}

pub(super) async fn subscribe(
    format: Format,
    target: Target,
    options: Subscribe,
) -> Result<(), MainError> {
    target.validate(&options.config, &options.state_root)?;
    if options.parameters_json.len() > v2::MAX_PARAMETER_BYTES {
        return Err(MainError::EventParameters);
    }
    let parameters: BTreeMap<String, Value> =
        serde_json::from_str(&options.parameters_json).map_err(|_| MainError::EventParameters)?;
    if parameters.len() > v2::MAX_PARAMETERS {
        return Err(MainError::EventParameters);
    }
    dispatch(
        format,
        target,
        options.config,
        options.state_root,
        EventRequest::Subscribe(SubscribeRequest {
            endpoint_ref: options.endpoint_ref,
            channel_binding: options.channel_binding,
            parameters,
        }),
    )
    .await
}

pub(super) async fn unsubscribe(
    format: Format,
    target: Target,
    options: Unsubscribe,
) -> Result<(), MainError> {
    target.validate(&options.config, &options.state_root)?;
    dispatch(
        format,
        target,
        options.config,
        options.state_root,
        EventRequest::Unsubscribe(UnsubscribeRequest {
            subscription_ref: options.subscription_ref,
        }),
    )
    .await
}

async fn dispatch(
    format: Format,
    target: Target,
    config: Option<PathBuf>,
    state_root: Option<PathBuf>,
    request: EventRequest,
) -> Result<(), MainError> {
    let response = if target == Target::Hosted {
        AuthenticatedHostedClient::active()?
            .event_v2(request)
            .await?
    } else {
        let config = read_config(config)?;
        let state_root = state_root.map_or_else(default_state_root, Ok)?;
        connectors_console::daemon::require(&state_root).await?;
        LocalClient::new(state_root.join("connectors.sock"))
            .event_v2(&config.owner_context(), request)
            .await?
    };
    emit_targeted(format, &reduce_envelope!(response)?, target.as_str())?;
    Ok(())
}
