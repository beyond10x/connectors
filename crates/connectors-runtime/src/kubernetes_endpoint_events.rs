//! Explicit, principal-bound subscriptions to catalog-declared endpoint socket channels.

use super::*;
use connector_resolve::{ConfigField, ConfigPort, ConfigValue, PreparedChannelPlan};
use protocol::event::{self, v2};
use serde_json::Value;
use service::{EgressWebSocket, EgressWebSocketFrame};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

#[path = "kubernetes_endpoint_event_store.rs"]
mod store;
pub(super) use store::EndpointEvents;
use store::{Producer, Subscription};

#[cfg(test)]
#[path = "kubernetes_endpoint_events_tests.rs"]
mod tests;

impl KubernetesEndpointBackend {
    pub(super) fn owns_endpoint_event(&self, request: &event::EventRequest) -> bool {
        match request {
            event::EventRequest::Search(_) => false,
            event::EventRequest::Receive(request) => {
                self.events.by_channel(&request.channel_ref).is_ok()
            }
            event::EventRequest::Replay(request) => self.events.replay(&request.event_ref).is_ok(),
        }
    }

    pub(super) fn owns_endpoint_event_v2(&self, request: &v2::EventRequest) -> bool {
        match request {
            v2::EventRequest::Subscribe(request) => self.source.show(&request.endpoint_ref).is_ok(),
            v2::EventRequest::Unsubscribe(request) => {
                self.events.subscription(&request.subscription_ref).is_ok()
            }
            _ => request
                .legacy_read()
                .is_some_and(|request| self.owns_endpoint_event(&request)),
        }
    }

    pub(super) async fn endpoint_event_v2(
        &self,
        context: &PrincipalContext,
        request: v2::EventRequest,
    ) -> Result<v2::EventResult, event::EventError> {
        match request {
            v2::EventRequest::Subscribe(request) => self.subscribe_endpoint(context, request).await,
            v2::EventRequest::Unsubscribe(request) => {
                let _serial = self.events.subscription_lock.lock().await;
                let subscription = self.events.subscription(&request.subscription_ref)?;
                // An owner can always release its own resources, including after policy revocation.
                if subscription.owner != owner(context) || !self.policy.owns(context) {
                    return Err(denied());
                }
                self.events.stop(&request.subscription_ref).await?;
                Ok(v2::EventResult::Unsubscribe {
                    subscription_ref: request.subscription_ref,
                })
            }
            request => self
                .endpoint_event(context, request.legacy_read().ok_or_else(invalid)?)
                .await
                .map(Into::into),
        }
    }

    pub(super) async fn endpoint_event(
        &self,
        context: &PrincipalContext,
        request: event::EventRequest,
    ) -> Result<event::EventResult, event::EventError> {
        match request {
            event::EventRequest::Search(request) => {
                if request.limit == 0
                    || request.limit > event::MAX_SEARCH_RESULTS
                    || request.query.len() > 512
                {
                    return Err(invalid());
                }
                let query = request.query.to_lowercase();
                let mut channels = Vec::new();
                for subscription in self.events.subscriptions()?.into_values() {
                    if subscription.owner != owner(context)
                        || !self.policy.reads(context, &subscription.endpoint)
                    {
                        continue;
                    }
                    if !query.is_empty()
                        && !format!(
                            "{} {}",
                            subscription.channel.integration_ref, subscription.channel.binding_ref
                        )
                        .to_lowercase()
                        .contains(&query)
                    {
                        continue;
                    }
                    if self
                        .verify_subscription(context, &subscription)
                        .await
                        .is_ok()
                    {
                        channels.push(subscription.channel);
                        if channels.len() == usize::from(request.limit) {
                            break;
                        }
                    }
                }
                Ok(event::EventResult::Search { channels })
            }
            event::EventRequest::Receive(request) => {
                if request.limit == 0
                    || request.limit > event::MAX_RECEIVE_RESULTS
                    || request.wait_ms > event::MAX_WAIT_MS
                {
                    return Err(invalid());
                }
                let subscription = self.events.by_channel(&request.channel_ref)?;
                self.verify_subscription(context, &subscription).await?;
                let after = request
                    .after
                    .as_deref()
                    .unwrap_or("0")
                    .parse::<u64>()
                    .map_err(|_| invalid())?;
                let notified = self.events.notify.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                let (mut events, mut next) =
                    self.events
                        .receive(&request.channel_ref, after, usize::from(request.limit))?;
                if events.is_empty() && request.wait_ms > 0 {
                    let _ = tokio::time::timeout(
                        Duration::from_millis(u64::from(request.wait_ms)),
                        notified,
                    )
                    .await;
                    self.verify_subscription(context, &subscription).await?;
                    (events, next) = self.events.receive(
                        &request.channel_ref,
                        after,
                        usize::from(request.limit),
                    )?;
                }
                Ok(event::EventResult::Receive {
                    events,
                    next: next.to_string(),
                })
            }
            event::EventRequest::Replay(request) => {
                let event = self.events.replay(&request.event_ref)?;
                let subscription = self.events.by_channel(&event.channel_ref)?;
                self.verify_subscription(context, &subscription).await?;
                Ok(event::EventResult::Replay(event))
            }
        }
    }

    async fn verify_subscription(
        &self,
        context: &PrincipalContext,
        subscription: &Subscription,
    ) -> Result<(), event::EventError> {
        if subscription.owner != owner(context)
            || !self.policy.reads(context, &subscription.endpoint)
        {
            return Err(denied());
        }
        let endpoint = self
            .source
            .validate(&subscription.endpoint.endpoint_ref)
            .await
            .map_err(source_error)?;
        current_subscription(&self.source, subscription)?;
        if endpoint.binding != subscription.endpoint.binding
            || !self.policy.reads(context, &endpoint)
        {
            return Err(denied());
        }
        Ok(())
    }

    async fn subscribe_endpoint(
        &self,
        context: &PrincipalContext,
        request: v2::SubscribeRequest,
    ) -> Result<v2::EventResult, event::EventError> {
        let _serial = self.events.subscription_lock.lock().await;
        let endpoint = self
            .source
            .show(&request.endpoint_ref)
            .map_err(source_error)?;
        if !self.policy.manages(context) || !self.policy.reads(context, &endpoint) {
            return Err(denied());
        }
        let provider = endpoint
            .provider
            .as_deref()
            .and_then(|id| catalog::provider(catalog::ProviderKey::id(id)))
            .ok_or_else(invalid)?;
        if self.source.grant_for(provider.id).is_none() {
            return Err(denied());
        }
        let channel = provider
            .channel(&request.channel_binding)
            .ok_or_else(invalid)?;
        if channel.transport != catalog::ChannelTransport::Socket
            || channel.connect.is_none()
            || channel.events.is_empty()
            || channel
                .connect
                .is_some_and(|connect| !connect.subprotocols.is_empty())
            || channel
                .discriminator
                .is_some_and(|selector| selector.source != "body")
            || channel
                .delivery_id
                .is_some_and(|selector| selector.source != "body")
        {
            return Err(invalid());
        }
        let parameters = parameters(provider, channel, request.parameters)?;
        // All caller-shaped channel input is now admitted. Identity reads still precede Secret GET.
        let endpoint = self
            .source
            .validate(&endpoint.endpoint_ref)
            .await
            .map_err(source_error)?;
        if !self.policy.reads(context, &endpoint) {
            return Err(denied());
        }
        let authority = self
            .source
            .binding_digest(provider.id)
            .map_err(source_error)?;
        let identity = hex::encode(Sha256::digest(
            serde_json::to_vec(&(
                owner(context),
                &endpoint.endpoint_ref,
                channel.name,
                &parameters,
                &authority,
            ))
            .map_err(|_| invalid())?,
        ));
        let subscription_ref = format!("subscription:endpoint:{identity}");
        let summary = event::ChannelSummary {
            channel_ref: format!("channel:endpoint:{identity}"),
            connection_ref: connection_ref(&endpoint),
            integration_ref: provider.id.to_owned(),
            binding_ref: channel.name.to_owned(),
            events: channel
                .events
                .iter()
                .map(|event| (*event).to_owned())
                .collect(),
        };
        if self.events.is_active(&subscription_ref) {
            return Ok(v2::EventResult::Subscribe {
                subscription_ref,
                channel: summary,
            });
        }
        let credentials = self
            .source
            .resolve_credentials(&endpoint)
            .await
            .map_err(source_error)?;
        let (config, secrets) = http_credentials(context, provider.id, &credentials)
            .await
            .map_err(|_| unavailable())?;
        let config = ChannelConfig {
            base: config,
            binding: channel.name,
            parameters,
        };
        let route = self
            .source
            .open_route(&endpoint, &credentials)
            .await
            .map_err(source_error)?;
        let plan = connector_resolve::channel_plan_for_endpoint(
            provider,
            channel,
            context.tenant_id(),
            None,
            &secrets,
            &config,
            route.logical_url.as_str(),
        )
        .await
        .map_err(|_| invalid())?;
        let headers = plan
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), value.expose_secret().to_owned()))
            .collect();
        let redactions = zeroize::Zeroizing::new(
            provider
                .auth
                .iter()
                .filter_map(|credential| {
                    credentials
                        .expose(credential.name)
                        .or_else(|| credentials.expose(credential.leaf))
                })
                .map(str::to_owned)
                .chain(
                    plan.headers
                        .values()
                        .map(|value| value.expose_secret().to_owned()),
                )
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>(),
        );
        let transport = self
            .egress
            .transport(&summary.connection_ref, &route)
            .map_err(source_error)?;
        let socket = tokio::time::timeout(
            Duration::from_secs(15),
            transport.connect_websocket_with_headers(
                &summary.connection_ref,
                plan.url.expose_secret().to_owned(),
                headers,
                event::MAX_EVENT_BYTES,
            ),
        )
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?;
        let subscription = Subscription {
            owner: owner(context),
            endpoint,
            channel: summary.clone(),
            authority,
        };
        current_subscription(&self.source, &subscription)?;
        self.events
            .insert(subscription_ref.clone(), subscription.clone())?;
        let (stop, stopped) = oneshot::channel();
        let source = self.source.clone();
        let events = Arc::downgrade(&self.events);
        let task = tokio::spawn(async move {
            produce(
                events,
                source,
                subscription,
                provider,
                plan,
                socket,
                route,
                redactions,
                stopped,
            )
            .await;
        });
        self.events
            .start(subscription_ref.clone(), Producer { stop, task })?;
        Ok(v2::EventResult::Subscribe {
            subscription_ref,
            channel: summary,
        })
    }
}

fn current_subscription(
    source: &KubernetesEndpointSource,
    subscription: &Subscription,
) -> Result<(), event::EventError> {
    let endpoint = source
        .show(&subscription.endpoint.endpoint_ref)
        .map_err(source_error)?;
    let provider = endpoint.provider.as_deref().ok_or_else(denied)?;
    if endpoint.binding != subscription.endpoint.binding
        || source.grant_for(provider).is_none()
        || source.binding_digest(provider).map_err(source_error)? != subscription.authority
        || matches!(endpoint.state, EndpointState::Denied | EndpointState::Stale)
    {
        return Err(stale());
    }
    let declaration = catalog::provider(catalog::ProviderKey::id(provider))
        .and_then(|provider| provider.channel(&subscription.channel.binding_ref))
        .ok_or_else(stale)?;
    if !declaration.events.iter().copied().eq(subscription
        .channel
        .events
        .iter()
        .map(String::as_str))
    {
        return Err(stale());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn produce(
    events: std::sync::Weak<EndpointEvents>,
    source: Arc<KubernetesEndpointSource>,
    subscription: Subscription,
    provider: &'static catalog::Provider,
    plan: PreparedChannelPlan,
    mut socket: Box<dyn EgressWebSocket>,
    _route: EndpointRouteLease,
    redactions: zeroize::Zeroizing<Vec<String>>,
    mut stop: oneshot::Receiver<()>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(15));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;
    loop {
        tokio::select! {
            biased;
            _ = &mut stop => break,
            _ = interval.tick() => {
                if current_subscription(&source, &subscription).is_err()
                    || source.validate(&subscription.endpoint.endpoint_ref).await.is_err() { break; }
            }
            frame = socket.receive() => {
                if current_subscription(&source, &subscription).is_err() { break; }
                match frame {
                    Ok(EgressWebSocketFrame::Text(text)) => {
                        let Some(events) = events.upgrade() else { break; };
                        match normalize(provider, &plan, &text, &redactions) {
                            Ok(Some((delivery, event_type, payload))) => {
                                if events.append(&subscription, delivery, event_type, payload).is_err() { break; }
                            }
                            Ok(None) => {}
                            Err(_) => break,
                        }
                    }
                    Ok(EgressWebSocketFrame::Ping(bytes)) => {
                        if !tokio::time::timeout(Duration::from_secs(2), socket.send_pong(bytes)).await
                            .is_ok_and(|result| result.is_ok()) { break; }
                    }
                    Ok(EgressWebSocketFrame::Other) => {}
                    _ => break,
                }
            }
        }
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.close()).await;
}

fn normalize(
    provider: &'static catalog::Provider,
    plan: &PreparedChannelPlan,
    text: &str,
    redactions: &[String],
) -> Result<Option<(Option<String>, String, Value)>, event::EventError> {
    if text.len() > event::MAX_EVENT_BYTES {
        return Err(invalid());
    }
    let envelope: Value = serde_json::from_str(text).map_err(|_| invalid())?;
    if contains_secret(&envelope, redactions) {
        return Err(invalid());
    }
    let wire = plan
        .discriminator
        .and_then(|selector| select(&envelope, selector.name))
        .and_then(Value::as_str)
        .or_else(|| (plan.events.len() == 1).then_some(plan.events[0]))
        .ok_or_else(invalid)?;
    let Some(event_type) = plan.wire_events.get(wire) else {
        return Ok(None);
    };
    let payload = if plan.payload_root {
        envelope.clone()
    } else {
        let mut payload = serde_json::Map::new();
        for pair in plan.payload {
            payload.insert(
                pair.name.to_owned(),
                select(&envelope, pair.value).cloned().ok_or_else(invalid)?,
            );
        }
        Value::Object(payload)
    };
    let declaration = provider
        .events
        .iter()
        .find(|event| event.name == *event_type)
        .ok_or_else(invalid)?;
    if let Some(schema) = declaration.schema {
        let schema: Value = serde_json::from_str(schema).map_err(|_| invalid())?;
        if !jsonschema::is_valid(&schema, &payload) {
            return Err(invalid());
        }
    }
    let delivery = plan
        .delivery_id
        .and_then(|selector| select(&envelope, selector.name))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok(Some((delivery, (*event_type).to_owned(), payload)))
}

fn select<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    if name.starts_with('/') {
        return value.pointer(name);
    }
    name.strip_prefix("$.")
        .unwrap_or(name)
        .split('.')
        .try_fold(value, |value, field| value.get(field))
}

fn contains_secret(value: &Value, secrets: &[String]) -> bool {
    match value {
        Value::String(value) => secrets.iter().any(|secret| value.contains(secret)),
        Value::Object(values) => values.iter().any(|(key, value)| {
            secrets.iter().any(|secret| key.contains(secret)) || contains_secret(value, secrets)
        }),
        Value::Array(values) => values.iter().any(|value| contains_secret(value, secrets)),
        _ => false,
    }
}

fn parameters(
    provider: &'static catalog::Provider,
    channel: &'static catalog::Channel,
    input: BTreeMap<String, Value>,
) -> Result<BTreeMap<String, String>, event::EventError> {
    if input.len() > 32 || serde_json::to_vec(&input).map_err(|_| invalid())?.len() > 16 * 1024 {
        return Err(invalid());
    }
    let prefix = format!("channel.{}.query.", channel.name);
    let declarations = provider
        .config
        .iter()
        .filter(|field| field.service == channel.service && field.binds.starts_with(&prefix))
        .map(|field| (field.name, field))
        .collect::<BTreeMap<_, _>>();
    if input
        .keys()
        .any(|name| !declarations.contains_key(name.as_str()))
    {
        return Err(invalid());
    }
    let mut parameters = BTreeMap::new();
    for (name, field) in declarations {
        if field.secret || field.approval != catalog::Approval::None {
            return Err(invalid());
        }
        let value = match input.get(name) {
            Some(Value::String(value)) => Some(value.clone()),
            Some(Value::Bool(value)) => Some(value.to_string()),
            Some(_) => return Err(invalid()),
            None => field.default.map(str::to_owned),
        };
        let Some(value) = value else {
            if field.required {
                return Err(invalid());
            } else {
                continue;
            }
        };
        connector_resolve::Slot::Query
            .validate(&value)
            .map_err(|_| invalid())?;
        parameters.insert(
            field
                .binds
                .strip_prefix(&prefix)
                .ok_or_else(invalid)?
                .to_owned(),
            value,
        );
    }
    Ok(parameters)
}

struct ChannelConfig {
    base: integration_catalog::DeclaredConfig,
    binding: &'static str,
    parameters: BTreeMap<String, String>,
}
impl ConfigPort for ChannelConfig {
    fn resolve(&self, field: ConfigField<'_>) -> Option<ConfigValue> {
        match field {
            ConfigField::ChannelQuery { channel, parameter } if channel == self.binding => self
                .parameters
                .get(parameter)
                .cloned()
                .map(ConfigValue::operator_approved),
            _ => self.base.resolve(field),
        }
    }
}

fn owner(context: &PrincipalContext) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(&(
            context.tenant_id(),
            context.realm(),
            context.subject(),
            context.actor_subject(),
            context.agent_revision(),
        ))
        .expect("principal identity is serializable"),
    ))
}
fn source_error(error: EndpointSourceError) -> event::EventError {
    match error {
        EndpointSourceError::Denied => denied(),
        EndpointSourceError::Stale => stale(),
        EndpointSourceError::InvalidBinding => invalid(),
        _ => unavailable(),
    }
}
fn invalid() -> event::EventError {
    event::EventError::new(
        event::EventErrorCode::InvalidInput,
        "endpoint channel input is not admitted by its declaration",
        false,
    )
}
fn denied() -> event::EventError {
    event::EventError::new(
        event::EventErrorCode::NotGranted,
        "endpoint channel is not granted to this principal",
        false,
    )
}
fn stale() -> event::EventError {
    event::EventError::new(
        event::EventErrorCode::StaleAuthority,
        "endpoint channel binding or source policy changed",
        false,
    )
}
fn not_found() -> event::EventError {
    event::EventError::new(
        event::EventErrorCode::NotFound,
        "endpoint channel or event was not found",
        false,
    )
}
fn unavailable() -> event::EventError {
    event::EventError::new(
        event::EventErrorCode::Unavailable,
        "endpoint channel is unavailable",
        true,
    )
}
