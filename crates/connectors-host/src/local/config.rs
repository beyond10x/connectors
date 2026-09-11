use super::{Failure, Result, filesystem as fs};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "ConfigInput")]
pub struct Config {
    pub format: String,
    pub owner_uid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_clock: Option<super::clock::Configuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_service_socket: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_adapter: Option<String>,
    #[serde(default)]
    pub adapters: BTreeMap<String, Adapter>,
}

// Deserialize the closed envelope before selecting its version. Validation is
// part of decoding: even callers using serde directly cannot give v1 a v2 field
// or omit v2's explicit private selection.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigInput {
    format: String,
    owner_uid: u32,
    approval_clock: Option<super::clock::Configuration>,
    secret_service_socket: Option<PathBuf>,
    default_adapter: Option<String>,
    #[serde(default)]
    adapters: BTreeMap<String, Adapter>,
}
impl TryFrom<ConfigInput> for Config {
    type Error = &'static str;
    fn try_from(input: ConfigInput) -> std::result::Result<Self, Self::Error> {
        let config = Self {
            format: input.format,
            owner_uid: input.owner_uid,
            approval_clock: input.approval_clock,
            secret_service_socket: input.secret_service_socket,
            default_adapter: input.default_adapter,
            adapters: input.adapters,
        };
        config
            .validate()
            .map_err(|_| "invalid local configuration")?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    pub instance_id: String,
    pub adapter_id: String,
    pub configuration_revision: String,
    pub protocol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "private_selection")]
    pub private_protocol: Option<super::runtime::PrivateProtocol>,
    #[serde(default)]
    pub startup: Startup,
    #[serde(default)]
    pub restart: Restart,
    pub executable: Executable,
    #[serde(default)]
    pub permissions: Permissions,
}

fn private_selection<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> std::result::Result<Option<super::runtime::PrivateProtocol>, D::Error> {
    // Presence must mean an explicit supported string, never null-as-absence.
    super::runtime::PrivateProtocol::deserialize(input).map(Some)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    #[serde(default)]
    pub profiles: BTreeSet<String>,
    #[serde(default)]
    pub operations: BTreeSet<String>,
}
impl Adapter {
    pub fn private_protocol(&self) -> super::runtime::PrivateProtocol {
        self.private_protocol
            .unwrap_or(super::runtime::PrivateProtocol::V1)
    }
    pub fn selection(&self) -> String {
        let mut value = serde_json::json!({"instance":self.instance_id,"adapter":self.adapter_id,
            "revision":self.configuration_revision,"protocol":self.protocol,"executable":self.executable});
        // Preserve every v1 digest byte. An explicit v2-format selection,
        // including private/1, is a new selection requiring fresh admission.
        if let Some(protocol) = self.private_protocol {
            value["private_protocol"] = serde_json::json!(protocol);
        }
        connectors_core::digest(&value)
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum Startup {
    #[default]
    #[serde(rename = "on-demand")]
    OnDemand,
    #[serde(rename = "automatic")]
    Automatic,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub enum Restart {
    #[default]
    #[serde(rename = "never")]
    Never,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Executable {
    pub path: PathBuf,
    pub sha256: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Resolved process context. Environment variables select directories only.
pub struct Paths {
    pub config: PathBuf,
    pub state: PathBuf,
}

impl Paths {
    pub fn resolve(config: Option<&Path>, state: Option<&Path>) -> Result<Self> {
        let default = |variable: &str, home_suffix: &str, leaf: &str| -> Result<PathBuf> {
            let base = if let Some(value) = std::env::var_os(variable).filter(|v| !v.is_empty()) {
                PathBuf::from(value)
            } else {
                PathBuf::from(std::env::var_os("HOME").ok_or(Failure::InvalidConfiguration)?)
                    .join(home_suffix)
            };
            fs::validate_path(&base)?;
            Ok(base.join(leaf))
        };
        let config = match config {
            Some(path) => path.to_owned(),
            None => default("XDG_CONFIG_HOME", ".config", "connectors/config.toml")?,
        };
        let state = match state {
            Some(path) => path.to_owned(),
            None => default("XDG_STATE_HOME", ".local/state", "connectors")?,
        };
        fs::validate_path(&config)?;
        fs::validate_path(&state)?;
        if config.file_name().is_none() || state.file_name().is_none() {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(Self { config, state })
    }
}

fn selector(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && connectors_core::valid_id(value)
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read_bounded(fs::private_file(path)?, 1024 * 1024)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        let config: Self = toml::from_str(text).map_err(|_| Failure::InvalidConfiguration)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(clock) = &self.approval_clock {
            clock
                .validate()
                .map_err(|_| Failure::InvalidConfiguration)?;
        }
        if let Some(path) = &self.secret_service_socket {
            fs::validate_path(path)?;
        }
        if !matches!(
            self.format.as_str(),
            "connectors-local/1" | "connectors-local/2"
        ) || self.owner_uid != fs::uid()
            || self.adapters.len() > 64
            || self
                .default_adapter
                .as_ref()
                .is_some_and(|alias| !self.adapters.contains_key(alias))
        {
            return Err(Failure::InvalidConfiguration);
        }
        let mut instances = BTreeSet::new();
        for (alias, entry) in &self.adapters {
            if !selector(alias)
                || matches!(
                    alias.as_str(),
                    "setup"
                        | "adapters"
                        | "connections"
                        | "operations"
                        | "approvals"
                        | "auth"
                        | "describe"
                        | "invoke"
                        | "serve"
                        | "server"
                )
                || !selector(&entry.instance_id)
                || !selector(&entry.adapter_id)
                || !selector(&entry.configuration_revision)
                || !instances.insert(&entry.instance_id)
                || entry.protocol != "v1alpha1"
                || (self.format == "connectors-local/2") != entry.private_protocol.is_some()
                || entry.permissions.profiles.len() > 64
                || entry.permissions.operations.len() > 256
                || entry
                    .permissions
                    .profiles
                    .iter()
                    .chain(&entry.permissions.operations)
                    .any(|id| !selector(id))
                || entry.executable.sha256.len() != 64
                || !entry
                    .executable
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || entry.executable.args.len() > 256
                || entry
                    .executable
                    .args
                    .iter()
                    .any(|arg| arg.len() > 4096 || arg.contains('\0'))
            {
                return Err(Failure::InvalidConfiguration);
            }
            fs::validate_path(&entry.executable.path)?;
        }
        Ok(())
    }

    pub fn initialize(paths: &Paths) -> Result<()> {
        let parent = fs::directory(
            paths.config.parent().ok_or(Failure::InvalidConfiguration)?,
            true,
            true,
        )?;
        // Refuse an existing destination before touching state, including an
        // unsafe existing destination. The final publication is also exclusive.
        match std::fs::symlink_metadata(&paths.config) {
            Ok(_) => return Err(Failure::ConfigurationExists),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Failure::InvalidConfiguration),
        }
        let state = fs::directory(&paths.state, true, true)?;
        super::metadata::Metadata::initialize(&paths.state)?;
        state.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
        let config = Self {
            format: "connectors-local/1".into(),
            owner_uid: fs::uid(),
            approval_clock: None,
            secret_service_socket: None,
            default_adapter: None,
            adapters: BTreeMap::new(),
        };
        let text = format!(
            "# Linux Secret Service custody is required; credentials never belong here.\n{}",
            toml::to_string(&config).map_err(|_| Failure::InvalidConfiguration)?
        );
        fs::publish_new(
            &parent,
            paths
                .config
                .file_name()
                .ok_or(Failure::InvalidConfiguration)?,
            text.as_bytes(),
        )
    }
}

impl Executable {
    /// Inspect the admitted artifact without executing it. Launch must repeat
    /// admission and retain the exact descriptor for exec; this is no readiness.
    pub fn check(&self) -> Result<()> {
        self.open().map(|_| ())
    }

    /// Return the admitted source file at EOF. Launch additionally captures and
    /// verifies a sealed snapshot: an inode can still be written in place.
    pub fn open(&self) -> Result<std::fs::File> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        };
        let parent = fs::directory(
            self.path.parent().ok_or(Failure::InvalidConfiguration)?,
            false,
            false,
        )?;
        let name = std::ffi::CString::new(
            self.path
                .file_name()
                .ok_or(Failure::InvalidConfiguration)?
                .as_bytes(),
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        // SAFETY: descriptor and C string are live, returned descriptor is owned.
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(Failure::InvalidConfiguration);
        }
        // SAFETY: openat returned a new owned descriptor.
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        let stat = file.metadata().map_err(|_| Failure::InvalidConfiguration)?;
        if !stat.is_file()
            || (stat.uid() != fs::uid() && stat.uid() != 0)
            || stat.mode() & 0o022 != 0
            || stat.mode() & 0o111 == 0
            || stat.len() > 512 * 1024 * 1024
        {
            return Err(Failure::InvalidConfiguration);
        }
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut remaining: usize = 512 * 1024 * 1024 + 1;
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| Failure::InvalidConfiguration)?;
            if count == 0 {
                break;
            }
            remaining = remaining
                .checked_sub(count)
                .ok_or(Failure::InvalidConfiguration)?;
            digest.update(&buffer[..count]);
        }
        if hex::encode(digest.finalize()) != self.sha256 {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(file)
    }
}
