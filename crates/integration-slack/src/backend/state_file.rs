use super::*;

/// The file a personal placement keeps its Connection list in, under its state root.
///
/// Named here rather than at each call site: this module owns the format, and a caller that has to
/// spell the file name is a second place for it to be spelled differently — which reads to the
/// person running it as a placement that lost every Connection it had.
const CONNECTIONS_FILE: &str = "connections.json";

pub(super) fn read_state(
    state_root: &Path,
    hosted_state: Option<&dyn StateStore>,
) -> Result<StateFile, SlackError> {
    let path = &state_root.join(CONNECTIONS_FILE);
    if let Some(hosted_state) = hosted_state {
        let Some(bytes) = hosted_state
            .read(CONNECTION_STATE_KEY, MAX_STATE_BYTES as usize)
            .map_err(|_| SlackError::new("connection-state"))?
        else {
            let state = StateFile::default();
            write_state(state_root, Some(hosted_state), &state)?;
            return Ok(state);
        };
        return decode_state(&bytes);
    }
    let Some(mut file) = open_owner_read(path, MAX_STATE_BYTES)? else {
        let state = StateFile::default();
        write_state(state_root, None, &state)?;
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
    state_root: &Path,
    hosted_state: Option<&dyn StateStore>,
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
    let path = state_root.join(CONNECTIONS_FILE);
    ensure_owner_directory(state_root)?;
    let temporary = state_root.join(".connections.json.tmp");
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
    fs::rename(&temporary, &path).map_err(|_| SlackError::new("connection-state"))?;
    File::open(state_root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| SlackError::new("connection-state"))
}
