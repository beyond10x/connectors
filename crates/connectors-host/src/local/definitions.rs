//! The instances of invoked definition files (story:invoke-definition-file,
//! architecture-decision-record:provider-definitions-and-runtimes D12 point 3).
//!
//! `connectors operations invoke --definition <file>` lowers the file into an
//! owner-only cache under the state directory, `<state>/definitions/<digest>/`,
//! and holds the instance's entry in an owner-only file beside it,
//! `<state>/definitions/instances.toml`. A configuration read with
//! [`Config::read_at`] carries those entries beside the operator's own, as the
//! instance `definition` of the definition's connector, so the owner and every
//! command see the instance as they see a configured one; the operator's file
//! is never written. An instance the operator's file already has is never
//! replaced: its entry is not held. An entry grants nothing by itself: no
//! connection, no admission and no OAuth exchange (D12 point 4).
use super::{
    Failure, Result,
    config::{Adapter, Config, Installation, Paths, Refusal, Selector},
    filesystem as fs, runtime,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};

/// The instance name of every invoked definition's connector.
pub const INSTANCE: &str = "definition";
/// The owner-only directory under the state directory.
pub const DIRECTORY: &str = "definitions";
/// The owner-only file of entries in [`DIRECTORY`].
pub const ENTRIES: &str = "instances.toml";
const FORMAT: &str = "connectors-definitions/1";
/// The most bytes the entries file may hold.
const LIMIT: usize = 1024 * 1024;
/// The most invoked definitions held at once, as a configuration holds at most
/// 64 entries.
pub const MOST: usize = 64;

/// The alias, and the instance id, of the definition instance of `connector`:
/// `<connector>.definition`, as a derived instance's is `<connector>.<instance>`.
pub fn alias(connector: &str) -> String {
    format!("{connector}.{INSTANCE}")
}

/// One invoked definition's instance: the cache entry its native configuration
/// is in, and the entry the installed catalog provider was pinned to for it.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The digest of the definition and its source: the cache directory
    /// `<state>/definitions/<digest>/`.
    pub digest: String,
    pub adapter: Adapter,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entries {
    format: String,
    #[serde(default)]
    instances: BTreeMap<String, Entry>,
}

/// The owner-only definitions directory under an existing owner-only state
/// directory, created when `create`.
pub fn directory(paths: &Paths, create: bool) -> Result<(PathBuf, std::fs::File)> {
    fs::directory(&paths.state, false, true)?;
    let path = paths.state.join(DIRECTORY);
    let directory = fs::directory(&path, create, true)?;
    Ok((path, directory))
}

/// Whether `entry`, held for `connector`, has the shape every recorded entry
/// has: the catalog adapter under the connector's definition alias, on the
/// current protocols, with an admissible executable selection.
fn admitted(connector: &str, entry: &Entry) -> bool {
    let alias = alias(connector);
    let adapter = &entry.adapter;
    connectors_core::valid_id(connector)
        && connectors_core::valid_id(&alias)
        && entry.digest.len() == 64
        && entry
            .digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && adapter.instance_id == alias
        && adapter.adapter_id == "catalog"
        && adapter.protocol == connectors_core::WIRE_VERSION
        && adapter.private_protocol == Some(runtime::PrivateProtocol::V2)
        && connectors_core::valid_id(&adapter.configuration_revision)
        && adapter
            .credential_revision
            .as_deref()
            .is_none_or(connectors_core::valid_id)
        && adapter.permissions.custody.is_empty()
        && adapter.permissions.profiles.len() <= 64
        && adapter.permissions.operations.len() <= 256
        && adapter
            .permissions
            .profiles
            .iter()
            .chain(&adapter.permissions.operations)
            .all(|id| connectors_core::valid_id(id))
        && adapter.executable.validate().is_ok()
}

/// The recorded entries, by connector: none when nothing was ever recorded.
/// A file that is not owner-only, too large, malformed, or holds an entry of
/// another shape is refused as a whole.
pub fn entries(paths: &Paths) -> Result<BTreeMap<String, Entry>> {
    match std::fs::symlink_metadata(paths.state.join(DIRECTORY)) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeMap::new());
        }
        _ => {}
    }
    let (_, directory) = directory(paths, false)?;
    if !fs::private_file_metadata_if_present_at(&directory, OsStr::new(ENTRIES))? {
        return Ok(BTreeMap::new());
    }
    let file = fs::private_file_at(&directory, OsStr::new(ENTRIES))?;
    let text = String::from_utf8(fs::read_bounded(file, LIMIT)?)
        .map_err(|_| Failure::InvalidConfiguration)?;
    let entries: Entries = toml::from_str(&text).map_err(|_| Failure::InvalidConfiguration)?;
    if entries.format != FORMAT
        || entries.instances.len() > MOST
        || !entries
            .instances
            .iter()
            .all(|(connector, entry)| admitted(connector, entry))
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(entries.instances)
}

/// Record `entry` as the instance of `connector`, replacing its earlier one and
/// keeping every other. The file is replaced atomically and read back: a
/// concurrent record of another connector that replaced it in between is
/// followed by another attempt, so neither is lost.
pub fn record(paths: &Paths, connector: &str, entry: &Entry) -> Result<()> {
    if !admitted(connector, entry) {
        return Err(Failure::InvalidConfiguration);
    }
    let (_, directory) = directory(paths, true)?;
    for _ in 0..8 {
        let mut instances = entries(paths)?;
        instances.insert(connector.to_owned(), entry.clone());
        if instances.len() > MOST {
            return Err(Failure::InvalidConfiguration);
        }
        let text = toml::to_string(&Entries {
            format: FORMAT.to_owned(),
            instances,
        })
        .map_err(|_| Failure::InvalidConfiguration)?;
        fs::publish_replace(&directory, OsStr::new(ENTRIES), text.as_bytes())?;
        let held = entries(paths)?;
        if held.get(connector).is_some_and(|held| {
            held.digest == entry.digest && held.adapter.selection() == entry.adapter.selection()
        }) {
            return Ok(());
        }
    }
    Err(Failure::ConcurrentRevision)
}

/// Whether `config` already has the definition instance of `connector`: an
/// instance `definition` of it, listed or pinned, an entry under its alias, or
/// an entry whose instance id is that alias.
pub fn collides(config: &Config, connector: &str) -> bool {
    let alias = alias(connector);
    config
        .select(Selector::Instance {
            connector,
            instance: INSTANCE,
        })
        .is_some()
        || config.awaiting_profile(connector, INSTANCE).is_some()
        || config.adapters.contains_key(&alias)
        || config.adapters.values().any(|a| a.instance_id == alias)
}

/// The entry of the installed catalog provider started on the native
/// configuration at `native`, pinned under the definition alias of `connector`
/// as a derived entry is, and the bootstrap its `connectors-private/2` child
/// reported. The provider is started without any credential: once to print its
/// bootstrap, once to report the private one; nothing is sent to the provider's
/// API.
pub fn derive(connector: &str, native: &std::path::Path) -> Result<(Adapter, runtime::Bootstrap)> {
    let installation = Installation::installed().ok_or(Failure::InvalidConfiguration)?;
    let native = native.to_str().ok_or(Failure::InvalidConfiguration)?;
    installation.pinned(
        &alias(connector),
        vec!["--local-config".to_owned(), native.to_owned()],
    )
}

impl Config {
    /// The configuration at `paths.config`, read as [`Config::read`] reads it,
    /// with the instance of every invoked definition recorded under
    /// `paths.state` beside its own entries ([`Config::hold`]). Entries that
    /// cannot be read are not held: the configuration is the operator's alone.
    pub fn read_at(paths: &Paths) -> std::result::Result<Self, Refusal> {
        let mut config = Self::read(&paths.config)?;
        if let Ok(entries) = entries(paths) {
            config.hold(entries);
        }
        Ok(config)
    }

    /// [`Config::read_at`], refused as [`Config::load`] refuses.
    pub fn load_at(paths: &Paths) -> Result<Self> {
        Self::read_at(paths).map_err(|refusal| refusal.failure())
    }

    /// Hold each entry as the instance `definition` of its connector, unless
    /// this configuration already has that instance ([`collides`]).
    pub fn hold(&mut self, entries: BTreeMap<String, Entry>) {
        for (connector, entry) in entries {
            if collides(self, &connector) {
                continue;
            }
            let alias = alias(&connector);
            self.adapters.insert(alias.clone(), entry.adapter);
            self.definitions.insert(alias, connector);
        }
    }
}

#[cfg(test)]
mod tests;
