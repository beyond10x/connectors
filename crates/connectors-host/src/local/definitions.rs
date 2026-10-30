//! The instances of invoked definition files (story:invoke-definition-file,
//! architecture-decision-record:provider-definitions-and-runtimes D12 point 3).
//!
//! `connectors invoke --definition <file>` lowers the file into an
//! owner-only cache under the state directory, `<state>/definitions/<digest>/`,
//! and holds the instance's entry in an owner-only file beside it,
//! `<state>/definitions/instances.toml`. A configuration read with
//! [`Config::read_at`] carries those entries beside the operator's own, as the
//! instance `definition` of the definition's connector, so the owner and every
//! command see the instance as they see a configured one; the operator's file
//! is never written. An instance the operator's file already has is never
//! replaced: its entry is not held. An entry grants nothing by itself: no
//! connection, no admission and no OAuth exchange (D12 point 4).
//!
//! A definition whose provider authority or profile authentication differs
//! from what a connection of its instance `definition` was admitted under,
//! which the registry refuses to connect under that instance id, is held as
//! the instance [`moved`] names instead (task:definition-moved-authority-new-instance).
use super::{
    Failure, Result,
    config::{Adapter, Config, Installation, Paths, Refusal, Selector},
    filesystem as fs, registry, runtime,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};

/// The instance name of every invoked definition's connector.
pub const INSTANCE: &str = "definition";
/// The prefix of a moved definition's instance name, followed by 16 lower-case
/// hexadecimal digits ([`moved`]).
pub const MOVED: &str = "definition-";
/// The hexadecimal digits of a moved instance name.
const MOVED_DIGITS: usize = 16;
/// The owner-only directory under the state directory.
pub const DIRECTORY: &str = "definitions";
/// The owner-only file of entries in [`DIRECTORY`].
pub const ENTRIES: &str = "instances.toml";
/// The catalog provider's native configuration in each cache entry.
pub const NATIVE: &str = "native.json";
const FORMAT: &str = "connectors-definitions/1";
/// The most bytes the entries file may hold.
const LIMIT: usize = 1024 * 1024;
/// The most invoked definitions held at once, as a configuration holds at most
/// 64 entries.
pub const MOST: usize = 64;

/// The alias, and the instance id, of the definition instance of `connector`:
/// `<connector>.definition`, as a derived instance's is `<connector>.<instance>`.
pub fn alias(connector: &str) -> String {
    alias_of(connector, INSTANCE)
}

/// The alias, and the instance id, of the definition instance `instance` of
/// `connector`: `<connector>.<instance>`.
pub fn alias_of(connector: &str, instance: &str) -> String {
    format!("{connector}.{instance}")
}

/// Whether `instance` names a definition's instance: [`INSTANCE`], or
/// [`MOVED`] followed by 16 lower-case hexadecimal digits.
pub fn is_instance(instance: &str) -> bool {
    instance == INSTANCE
        || instance.strip_prefix(MOVED).is_some_and(|digits| {
            digits.len() == MOVED_DIGITS
                && digits
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

/// The instance name of `alias` as a definition instance of `connector`.
pub fn instance_of<'a>(connector: &str, alias: &'a str) -> Option<&'a str> {
    alias
        .strip_prefix(connector)?
        .strip_prefix('.')
        .filter(|instance| is_instance(instance))
}

/// The instance a definition of `connector` bound as `binding` takes when its
/// instance `definition` cannot hold it (`Registry::needs_new_instance`):
/// [`MOVED`] and the first 16 hexadecimal digits of the digest of the
/// connector, the provider authority and the profile's authentication. Those
/// are what make a new instance id necessary, so the name is the same on every
/// run of the same definition, and it does not move with anything the registry
/// follows under one instance id: the configuration and credential revisions,
/// the executable, or the evidence lifetime alone.
pub fn moved(connector: &str, binding: &registry::Binding) -> String {
    let profile = &binding.profile;
    // The declaration without what the same authentication may differ in.
    let authentication = match &profile.declaration {
        Some(declaration) => {
            let mut declaration = declaration.clone();
            if let Some(fields) = declaration.as_object_mut() {
                fields.insert("revision".into(), serde_json::json!(""));
                fields.insert("evidence_lifetime_ms".into(), serde_json::json!(0));
            }
            connectors_core::digest(&declaration)
        }
        None => profile.revision.clone(),
    };
    let scopes: Vec<&str> = profile.minimum_scopes.iter().map(String::as_str).collect();
    let mut hash = Sha256::new();
    for part in [
        "connectors-definition-instance/1",
        connector,
        &binding.adapter_id,
        &binding.provider_authority,
        &profile.id,
        &serde_json::to_string(&profile.purpose).unwrap_or_default(),
        &serde_json::to_string(&profile.subject).unwrap_or_default(),
        &serde_json::to_string(&scopes).unwrap_or_default(),
        &authentication,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let digest = hex::encode(hash.finalize());
    format!("{MOVED}{}", &digest[..MOVED_DIGITS])
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
/// has: the catalog adapter under one of the connector's definition aliases
/// ([`is_instance`]), on the current protocols, the catalog provider started
/// as `--local-config` of the native configuration in its own cache entry
/// under `paths.state`, with an admissible executable selection.
fn admitted(paths: &Paths, connector: &str, entry: &Entry) -> bool {
    let adapter = &entry.adapter;
    let Some(instance) = instance_of(connector, &adapter.instance_id) else {
        return false;
    };
    let alias = alias_of(connector, instance);
    let native = paths.state.join(DIRECTORY).join(&entry.digest).join(NATIVE);
    let provider = format!(
        "connectors-catalog-provider{}",
        std::env::consts::EXE_SUFFIX
    );
    connectors_core::valid_id(connector)
        && adapter.executable.path.file_name() == Some(OsStr::new(&provider))
        // Compared as paths, component by component: every admitted spelling
        // of the state directory names the same entry.
        && matches!(
            adapter.executable.args.as_slice(),
            [flag, path] if flag == "--local-config" && std::path::Path::new(path) == native
        )
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
            .all(|(connector, entry)| admitted(paths, connector, entry))
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
    if !admitted(paths, connector, entry) {
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

/// The keys of a native configuration `invoke --definition` writes
/// (`apps/connectors/src/local/definition.rs` `native_configuration`).
const NATIVE_KEYS: [&str; 8] = [
    "format",
    "instance",
    "provider",
    "bundle_directory",
    "api_base",
    "auth",
    "operations_file",
    "request_prefix",
];

/// The scheme, host and port of an absolute HTTPS URL, in lower case.
fn origin(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    (!host.is_empty() && !host.contains('@'))
        .then(|| format!("https://{}", host.to_ascii_lowercase()))
}

/// Every string in `value` that names an absolute URL, at any depth.
fn urls<'a>(value: &'a serde_json::Value, found: &mut Vec<&'a str>) {
    match value {
        serde_json::Value::String(text) if text.contains("://") => found.push(text),
        serde_json::Value::Array(items) => items.iter().for_each(|item| urls(item, found)),
        serde_json::Value::Object(fields) => fields.values().for_each(|item| urls(item, found)),
        _ => {}
    }
}

/// Whether the recorded definition instance `alias` may receive a reused
/// credential (story:invoke-definition-reuse-credential): it is an invoked
/// definition's instance whose native configuration sends the credential to
/// its HTTPS API base alone: its profile names no URL (no token endpoint) and
/// it names no trust root or admitted host. The binding compares the provider
/// authority and the profile declaration, which a token endpoint, trust roots
/// and admitted hosts are not part of, so a configuration naming any of them
/// receives none (security review rounds 1 and 2).
pub fn receives_reuse(paths: &Paths, config: &Config, alias: &str) -> Result<bool> {
    let Some(connector) = config.definitions.get(alias) else {
        return Ok(false);
    };
    let Some(entry) = entries(paths)?.remove(connector) else {
        return Ok(false);
    };
    if config.adapters.get(alias).map(Adapter::selection) != Some(entry.adapter.selection()) {
        return Ok(false);
    }
    let native = paths.state.join(DIRECTORY).join(&entry.digest).join(NATIVE);
    let bytes = fs::private_file(&native).and_then(|file| fs::read_bounded(file, LIMIT))?;
    let native: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
    let Some(fields) = native.as_object() else {
        return Ok(false);
    };
    if fields
        .keys()
        .any(|key| !NATIVE_KEYS.contains(&key.as_str()))
    {
        return Ok(false);
    }
    if native["api_base"].as_str().and_then(origin).is_none() {
        return Ok(false);
    }
    // A profile naming any URL (a token endpoint) sends the credential to a
    // place the source's connection may never send it, on any origin.
    let mut found = Vec::new();
    urls(&native["auth"], &mut found);
    Ok(found.is_empty())
}

/// Whether `config` already has the definition instance `instance` of
/// `connector`: an instance of that name, listed or pinned, an entry under its
/// alias, or an entry whose instance id is that alias.
pub fn collides(config: &Config, connector: &str, instance: &str) -> bool {
    let alias = alias_of(connector, instance);
    config
        .select(Selector::Instance {
            connector,
            instance,
        })
        .is_some()
        || config.awaiting_profile(connector, instance).is_some()
        || config.adapters.contains_key(&alias)
        || config.adapters.values().any(|a| a.instance_id == alias)
}

/// The entry of the installed catalog provider started on the native
/// configuration at `native`, pinned under the definition alias `alias` as a
/// derived entry is, and the bootstrap its `connectors-private/2` child
/// reported. The provider is started without any credential: once to print its
/// bootstrap, once to report the private one; nothing is sent to the provider's
/// API.
pub fn derive(alias: &str, native: &std::path::Path) -> Result<(Adapter, runtime::Bootstrap)> {
    let installation = Installation::installed().ok_or(Failure::InvalidConfiguration)?;
    let native = native.to_str().ok_or(Failure::InvalidConfiguration)?;
    installation.pinned(alias, vec!["--local-config".to_owned(), native.to_owned()])
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

    /// Hold each entry as the definition instance of its connector its
    /// instance id names (`definition`, or a moved one), unless this
    /// configuration already has that instance ([`collides`]).
    pub fn hold(&mut self, entries: BTreeMap<String, Entry>) {
        for (connector, entry) in entries {
            let Some(instance) = instance_of(&connector, &entry.adapter.instance_id) else {
                continue;
            };
            if collides(self, &connector, instance) {
                continue;
            }
            let alias = alias_of(&connector, instance);
            self.adapters.insert(alias.clone(), entry.adapter);
            self.definitions.insert(alias, connector);
        }
    }
}

#[cfg(test)]
mod tests;
