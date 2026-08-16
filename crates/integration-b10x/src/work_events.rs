use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use protocol::event::{DataEvent, EventError, EventErrorCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    #[serde(default)]
    tenants: BTreeMap<String, TenantState>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TenantState {
    owner_cursor: Option<String>,
    #[serde(default)]
    events: Vec<StoredEvent>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyState {
    owner_cursor: Option<String>,
    #[serde(default)]
    events: Vec<StoredEvent>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PersistedState {
    Partitioned(State),
    Legacy(LegacyState),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEvent {
    sequence: u64,
    owner_id: String,
    event: DataEvent,
}

pub(super) struct ModuleEventStore {
    module: &'static str,
    path: PathBuf,
    state: Mutex<State>,
}

impl ModuleEventStore {
    pub(super) fn open(
        module: &'static str,
        path: PathBuf,
        legacy_tenant: Option<&str>,
    ) -> Result<Self, ()> {
        let state = if path.exists() {
            let bytes = fs::read(&path).map_err(|_| ())?;
            match serde_json::from_slice::<PersistedState>(&bytes).map_err(|_| ())? {
                PersistedState::Partitioned(state) => state,
                PersistedState::Legacy(legacy) => {
                    let tenant = legacy_tenant.filter(|value| !value.is_empty()).ok_or(())?;
                    State {
                        tenants: BTreeMap::from([(
                            tenant.to_owned(),
                            TenantState {
                                owner_cursor: legacy.owner_cursor,
                                events: legacy.events,
                            },
                        )]),
                    }
                }
            }
        } else {
            State::default()
        };
        Ok(Self {
            module,
            path,
            state: Mutex::new(state),
        })
    }

    pub(super) fn owner_cursor(&self, tenant: &str) -> Result<Option<String>, EventError> {
        Ok(self
            .state()?
            .tenants
            .get(tenant)
            .and_then(|state| state.owner_cursor.clone()))
    }

    pub(super) fn append(
        &self,
        tenant: &str,
        owner_cursor: Option<String>,
        incoming: Vec<(String, DataEvent)>,
    ) -> Result<(), EventError> {
        let mut guard = self.state()?;
        let tenant_state = guard.tenants.entry(tenant.to_owned()).or_default();
        for (owner_id, mut event) in incoming {
            if tenant_state
                .events
                .iter()
                .any(|stored| stored.owner_id == owner_id)
            {
                continue;
            }
            let sequence = tenant_state
                .events
                .last()
                .map_or(1, |stored| stored.sequence.saturating_add(1));
            event.event_ref = format!("event:b10x:{}:{sequence}", self.module);
            tenant_state.events.push(StoredEvent {
                sequence,
                owner_id,
                event,
            });
        }
        if owner_cursor.is_some() {
            tenant_state.owner_cursor = owner_cursor;
        }
        persist(&self.path, &guard)
    }

    pub(super) fn receive(
        &self,
        tenant: &str,
        after: u64,
        limit: usize,
    ) -> Result<(Vec<DataEvent>, u64), EventError> {
        let guard = self.state()?;
        let events: Vec<_> = guard
            .tenants
            .get(tenant)
            .into_iter()
            .flat_map(|state| &state.events)
            .filter(|stored| stored.sequence > after)
            .take(limit)
            .map(|stored| stored.event.clone())
            .collect();
        let next = events
            .last()
            .and_then(|event| event.event_ref.rsplit(':').next())
            .and_then(|value| value.parse().ok())
            .unwrap_or(after);
        Ok((events, next))
    }

    pub(super) fn replay(
        &self,
        tenant: &str,
        event_ref: &str,
    ) -> Result<Option<DataEvent>, EventError> {
        Ok(self
            .state()?
            .tenants
            .get(tenant)
            .into_iter()
            .flat_map(|state| &state.events)
            .find(|stored| stored.event.event_ref == event_ref)
            .map(|stored| stored.event.clone()))
    }

    fn state(&self) -> Result<MutexGuard<'_, State>, EventError> {
        self.state.lock().map_err(|_| unavailable())
    }
}

fn persist(path: &Path, state: &State) -> Result<(), EventError> {
    let bytes = serde_json::to_vec(state).map_err(|_| protocol())?;
    let temporary = path.with_extension("json.next");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| unavailable())?;
    file.write_all(&bytes).map_err(|_| unavailable())?;
    file.sync_all().map_err(|_| unavailable())?;
    fs::rename(temporary, path).map_err(|_| unavailable())
}

fn unavailable() -> EventError {
    EventError::new(
        EventErrorCode::Unavailable,
        "Work event checkpoint storage is unavailable",
        true,
    )
}

fn protocol() -> EventError {
    EventError::new(
        EventErrorCode::Protocol,
        "Work event checkpoint storage was refused",
        false,
    )
}

#[cfg(test)]
mod tests {
    use protocol::event::EventProvenance;
    use serde_json::json;

    use super::*;

    fn event(owner: &str) -> DataEvent {
        DataEvent {
            event_ref: "pending".to_owned(),
            channel_ref: "channel:work".to_owned(),
            connection_ref: "connection:work".to_owned(),
            integration_ref: "b10x".to_owned(),
            event_type: "request.created".to_owned(),
            provenance: EventProvenance::Polled,
            received_at_unix_ms: 1,
            payload: json!({"owner": owner}),
        }
    }

    #[test]
    fn cursors_events_and_replay_are_partitioned_by_tenant() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("work-events.json");
        let store = ModuleEventStore::open("work", path.clone(), None).expect("open store");

        store
            .append(
                "tenant-a",
                Some("owner-a".to_owned()),
                vec![("owner-event-a".to_owned(), event("a"))],
            )
            .expect("append tenant a");
        store
            .append(
                "tenant-b",
                Some("owner-b".to_owned()),
                vec![("owner-event-b".to_owned(), event("b"))],
            )
            .expect("append tenant b");

        assert_eq!(
            store.owner_cursor("tenant-a").unwrap().as_deref(),
            Some("owner-a")
        );
        assert_eq!(
            store.owner_cursor("tenant-b").unwrap().as_deref(),
            Some("owner-b")
        );
        assert_eq!(
            store.receive("tenant-a", 0, 10).unwrap().0[0].payload,
            json!({"owner": "a"})
        );
        assert_eq!(
            store.receive("tenant-b", 0, 10).unwrap().0[0].payload,
            json!({"owner": "b"})
        );
        assert!(store
            .replay("tenant-b", "event:b10x:work:1")
            .unwrap()
            .is_some());

        drop(store);
        let reopened =
            ModuleEventStore::open("work", path, None).expect("reopen partitioned store");
        assert_eq!(reopened.receive("tenant-a", 0, 10).unwrap().0.len(), 1);
        assert_eq!(reopened.receive("tenant-b", 0, 10).unwrap().0.len(), 1);
    }
}
