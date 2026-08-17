use super::*;

pub(super) fn read_state(
    path: &Path,
    hosted_state: Option<&PostgresState>,
) -> Result<StateFile, SlackError> {
    if let Some(hosted_state) = hosted_state {
        let Some(bytes) = hosted_state
            .read(CONNECTION_STATE_KEY, MAX_STATE_BYTES as usize)
            .map_err(|_| SlackError::new("connection-state"))?
        else {
            let state = StateFile::default();
            write_state(path, Some(hosted_state), &state)?;
            return Ok(state);
        };
        return decode_state(&bytes);
    }
    let Some(mut file) = open_owner_read(path, MAX_STATE_BYTES)? else {
        let state = StateFile::default();
        write_state(path, None, &state)?;
        return Ok(state);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|_| SlackError::new("connection-state"))?;
    decode_state(&bytes)
}

fn decode_state(bytes: &[u8]) -> Result<StateFile, SlackError> {
    let mut state: StateFile =
        serde_json::from_slice(bytes).map_err(|_| SlackError::new("connection-state"))?;
    if !matches!(state.version, 1 | STATE_VERSION) || state.next_transaction_generation == 0 {
        return Err(SlackError::new("connection-state-version"));
    }
    state.version = STATE_VERSION;
    Ok(state)
}

pub(super) fn write_state(
    path: &Path,
    hosted_state: Option<&PostgresState>,
    state: &StateFile,
) -> Result<(), SlackError> {
    let bytes = serde_json::to_vec(state).map_err(|_| SlackError::new("connection-state"))?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(SlackError::new("connection-state-bound"));
    }
    if let Some(hosted_state) = hosted_state {
        return hosted_state
            .replace(CONNECTION_STATE_KEY, &bytes, MAX_STATE_BYTES as usize)
            .map_err(|_| SlackError::new("connection-state"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| SlackError::new("connection-state"))?;
    ensure_owner_directory(parent)?;
    let temporary = parent.join(".connections.json.tmp");
    refuse_existing_non_owner_file(&temporary)?;
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(&temporary)
        .map_err(|_| SlackError::new("connection-state"))?;
    inspect_owner_file(&file, "connection-state")?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| SlackError::new("connection-state"))?;
    fs::rename(&temporary, path).map_err(|_| SlackError::new("connection-state"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| SlackError::new("connection-state"))
}
