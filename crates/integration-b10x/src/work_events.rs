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
    owner_cursor: Option<String>,
    #[serde(default)]
    events: Vec<StoredEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEvent {
    sequence: u64,
    owner_id: String,
    event: DataEvent,
}

pub(super) struct WorkEventStore {
    path: PathBuf,
    state: Mutex<State>,
}

impl WorkEventStore {
    pub(super) fn open(path: PathBuf) -> Result<Self, ()> {
        let state = if path.exists() {
            let bytes = fs::read(&path).map_err(|_| ())?;
            serde_json::from_slice(&bytes).map_err(|_| ())?
        } else {
            State::default()
        };
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    pub(super) fn owner_cursor(&self) -> Result<Option<String>, EventError> {
        Ok(self.state()?.owner_cursor.clone())
    }

    pub(super) fn append(
        &self,
        owner_cursor: Option<String>,
        incoming: Vec<(String, DataEvent)>,
    ) -> Result<(), EventError> {
        let mut guard = self.state()?;
        for (owner_id, mut event) in incoming {
            if guard.events.iter().any(|stored| stored.owner_id == owner_id) {
                continue;
            }
            let sequence = guard
                .events
                .last()
                .map_or(1, |stored| stored.sequence.saturating_add(1));
            event.event_ref = format!("event:b10x:work:{sequence}");
            guard.events.push(StoredEvent {
                sequence,
                owner_id,
                event,
            });
        }
        if owner_cursor.is_some() {
            guard.owner_cursor = owner_cursor;
        }
        persist(&self.path, &guard)
    }

    pub(super) fn receive(
        &self,
        after: u64,
        limit: usize,
    ) -> Result<(Vec<DataEvent>, u64), EventError> {
        let guard = self.state()?;
        let events: Vec<_> = guard
            .events
            .iter()
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

    pub(super) fn replay(&self, event_ref: &str) -> Result<Option<DataEvent>, EventError> {
        Ok(self
            .state()?
            .events
            .iter()
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
