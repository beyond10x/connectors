//! Bounded endpoint event receipts; reconnecting is always an explicit subscription action.

use super::*;
use connector_state::StateStore;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tokio::sync::{oneshot, Notify};

const MAX_STORE_BYTES: usize = 16 * 1024 * 1024;
const MAX_EVENTS: usize = 4096;
const MAX_SUBSCRIPTIONS: usize = 64;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::kubernetes_endpoints) struct Subscription {
    pub owner: String,
    pub endpoint: Endpoint,
    pub channel: event::ChannelSummary,
    pub authority: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEvent {
    sequence: u64,
    delivery: Option<String>,
    event: event::DataEvent,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    sequence: u64,
    subscriptions: BTreeMap<String, Subscription>,
    events: Vec<StoredEvent>,
}

pub(in crate::kubernetes_endpoints) struct Producer {
    pub stop: oneshot::Sender<()>,
    pub task: tokio::task::JoinHandle<()>,
}

pub(crate) struct EndpointEvents {
    store: Arc<dyn StateStore>,
    key: String,
    image: Mutex<Result<Image, event::EventError>>,
    producers: Mutex<BTreeMap<String, Producer>>,
    pub subscription_lock: tokio::sync::Mutex<()>,
    pub notify: Notify,
}

impl EndpointEvents {
    pub fn new(source: &KubernetesEndpointSource) -> Self {
        let store = source.metadata_store();
        let key = format!(
            "endpoint.events.{}",
            hex::encode(Sha256::digest(source.source_ref()))
        );
        let image = store
            .read(&key, MAX_STORE_BYTES)
            .map_err(|_| unavailable())
            .and_then(|bytes| match bytes {
                Some(bytes) => serde_json::from_slice::<Image>(&bytes).map_err(|_| unavailable()),
                None => Ok(Image::default()),
            })
            .and_then(|image| {
                if image.events.len() > MAX_EVENTS
                    || image.subscriptions.len() > MAX_SUBSCRIPTIONS
                    || image.subscriptions.values().any(|subscription| {
                        !subscription.endpoint.validate()
                            || subscription.endpoint.source_ref != source.source_ref()
                            || subscription.owner.len() != 64
                            || subscription.authority.len() != 64
                    })
                {
                    Err(unavailable())
                } else {
                    Ok(image)
                }
            });
        Self {
            store,
            key,
            image: Mutex::new(image),
            producers: Mutex::new(BTreeMap::new()),
            subscription_lock: tokio::sync::Mutex::new(()),
            notify: Notify::new(),
        }
    }

    pub fn subscriptions(&self) -> Result<BTreeMap<String, Subscription>, event::EventError> {
        self.image
            .lock()
            .map_err(|_| unavailable())?
            .as_ref()
            .map(|image| image.subscriptions.clone())
            .map_err(Clone::clone)
    }

    pub fn subscription(&self, reference: &str) -> Result<Subscription, event::EventError> {
        self.subscriptions()?
            .remove(reference)
            .ok_or_else(not_found)
    }

    pub fn by_channel(&self, reference: &str) -> Result<Subscription, event::EventError> {
        self.subscriptions()?
            .into_values()
            .find(|item| item.channel.channel_ref == reference)
            .ok_or_else(not_found)
    }

    pub fn insert(
        &self,
        reference: String,
        subscription: Subscription,
    ) -> Result<(), event::EventError> {
        let mut guard = self.image.lock().map_err(|_| unavailable())?;
        let image = guard.as_mut().map_err(|error| error.clone())?;
        if !image.subscriptions.contains_key(&reference)
            && image.subscriptions.len() >= MAX_SUBSCRIPTIONS
        {
            return Err(unavailable());
        }
        let mut next = image.clone();
        next.subscriptions.insert(reference, subscription);
        self.persist(&next)?;
        *image = next;
        Ok(())
    }

    pub fn is_active(&self, reference: &str) -> bool {
        self.producers.lock().is_ok_and(|items| {
            items
                .get(reference)
                .is_some_and(|item| !item.task.is_finished())
        })
    }

    pub fn start(&self, reference: String, producer: Producer) -> Result<(), event::EventError> {
        let mut producers = self.producers.lock().map_err(|_| unavailable())?;
        if let Some(previous) = producers.insert(reference, producer) {
            previous.task.abort();
        }
        Ok(())
    }

    pub async fn stop(&self, reference: &str) -> Result<(), event::EventError> {
        let producer = self
            .producers
            .lock()
            .map_err(|_| unavailable())?
            .remove(reference);
        if let Some(mut producer) = producer {
            let _ = producer.stop.send(());
            if tokio::time::timeout(Duration::from_secs(2), &mut producer.task)
                .await
                .is_err()
            {
                producer.task.abort();
            }
        }
        Ok(())
    }

    pub fn stop_all(&self) {
        if let Ok(mut producers) = self.producers.lock() {
            for (_, producer) in std::mem::take(&mut *producers) {
                producer.task.abort();
            }
        }
    }

    pub fn append(
        &self,
        subscription: &Subscription,
        delivery: Option<String>,
        event_type: String,
        payload: Value,
    ) -> Result<(), event::EventError> {
        let mut guard = self.image.lock().map_err(|_| unavailable())?;
        let image = guard.as_mut().map_err(|error| error.clone())?;
        if delivery.as_ref().is_some_and(|delivery| {
            image.events.iter().any(|stored| {
                stored.delivery.as_ref() == Some(delivery)
                    && stored.event.channel_ref == subscription.channel.channel_ref
            })
        }) {
            return Ok(());
        }
        if image.events.len() >= MAX_EVENTS {
            return Err(unavailable());
        }
        let sequence = image.sequence.checked_add(1).ok_or_else(unavailable)?;
        let received_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| unavailable())?
            .as_millis()
            .try_into()
            .map_err(|_| unavailable())?;
        let event = event::DataEvent {
            event_ref: format!(
                "event:endpoint:{}",
                hex::encode(Sha256::digest(format!(
                    "{}\0{sequence}",
                    subscription.channel.channel_ref
                )))
            ),
            channel_ref: subscription.channel.channel_ref.clone(),
            connection_ref: subscription.channel.connection_ref.clone(),
            integration_ref: subscription.channel.integration_ref.clone(),
            event_type,
            provenance: event::EventProvenance::Native,
            received_at_unix_ms,
            payload,
        };
        if serde_json::to_vec(&event).map_err(|_| unavailable())?.len() > event::MAX_EVENT_BYTES {
            return Err(unavailable());
        }
        let mut next = image.clone();
        next.sequence = sequence;
        next.events.push(StoredEvent {
            sequence,
            delivery,
            event,
        });
        self.persist(&next)?;
        *image = next;
        self.notify.notify_waiters();
        Ok(())
    }

    fn persist(&self, image: &Image) -> Result<(), event::EventError> {
        let bytes = serde_json::to_vec(image).map_err(|_| unavailable())?;
        self.store
            .replace(&self.key, &bytes, MAX_STORE_BYTES)
            .map_err(|_| unavailable())
    }

    pub fn receive(
        &self,
        channel: &str,
        after: u64,
        limit: usize,
    ) -> Result<(Vec<event::DataEvent>, u64), event::EventError> {
        let guard = self.image.lock().map_err(|_| unavailable())?;
        let image = guard.as_ref().map_err(Clone::clone)?;
        let mut size = 256;
        let selected = image
            .events
            .iter()
            .filter(|stored| stored.sequence > after && stored.event.channel_ref == channel)
            .take(limit)
            .take_while(|stored| {
                size += serde_json::to_vec(&stored.event)
                    .map_or(event::MAX_RESPONSE_BYTES, |bytes| bytes.len());
                size < event::MAX_RESPONSE_BYTES - 1024
            })
            .collect::<Vec<_>>();
        let next = selected.last().map_or(after, |stored| stored.sequence);
        Ok((
            selected
                .into_iter()
                .map(|stored| stored.event.clone())
                .collect(),
            next,
        ))
    }

    pub fn replay(&self, reference: &str) -> Result<event::DataEvent, event::EventError> {
        let guard = self.image.lock().map_err(|_| unavailable())?;
        guard
            .as_ref()
            .map_err(Clone::clone)?
            .events
            .iter()
            .find(|stored| stored.event.event_ref == reference)
            .map(|stored| stored.event.clone())
            .ok_or_else(not_found)
    }
}

impl Drop for EndpointEvents {
    fn drop(&mut self) {
        self.stop_all();
    }
}
