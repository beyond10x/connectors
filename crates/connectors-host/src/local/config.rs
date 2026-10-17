use super::{Failure, Result, filesystem as fs};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "ConfigInput")]
pub struct Config {
    pub format: String,
    pub owner_uid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_clock: Option<super::clock::ApprovalClock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_service_socket: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_adapter: Option<String>,
    #[serde(default)]
    pub adapters: BTreeMap<String, Adapter>,
    /// The explicitly selected custody binding (`connectors-local/4` only);
    /// absent, custody is the platform binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody: Option<Custody>,
    /// Operator-pinned consumer executables (`connectors-local/3` and `/4`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumers: BTreeMap<String, Consumer>,
}

// Deserialize the closed envelope before selecting its version. Validation is
// part of decoding: even callers using serde directly cannot give v1 a v2 field
// or omit v2's explicit private selection.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigInput {
    format: String,
    owner_uid: u32,
    approval_clock: Option<super::clock::ApprovalClock>,
    secret_service_socket: Option<PathBuf>,
    default_adapter: Option<String>,
    custody: Option<Custody>,
    #[serde(default)]
    adapters: BTreeMap<String, Adapter>,
    #[serde(default)]
    consumers: BTreeMap<String, Consumer>,
}
impl Config {
    fn assemble(input: ConfigInput) -> Self {
        Self {
            format: input.format,
            owner_uid: input.owner_uid,
            approval_clock: input.approval_clock,
            secret_service_socket: input.secret_service_socket,
            custody: input.custody,
            default_adapter: input.default_adapter,
            adapters: input.adapters,
            consumers: input.consumers,
        }
    }
}
impl TryFrom<ConfigInput> for Config {
    type Error = &'static str;
    fn try_from(input: ConfigInput) -> std::result::Result<Self, Self::Error> {
        let config = Self::assemble(input);
        config
            .validate()
            .map_err(|_| "invalid local configuration")?;
        Ok(config)
    }
}

/// Why a configuration file was refused. Only operator-written, already
/// admitted coordinates are carried; never parser, OS or database text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    Failure(Failure),
    /// An entry's `private_protocol` presence contradicts the file's format:
    /// `connectors-local/1` forbids it, `connectors-local/2` and `/3` require
    /// it. `format` is one of those values and `instance_id` a valid selector.
    PrivateProtocolMismatch {
        format: String,
        instance_id: String,
    },
    /// A rule of the configuration stage refused, named without a path or OS
    /// text (`connectors.cli.ConfigurationRule`).
    Rule(Rule),
}
impl Refusal {
    pub fn failure(&self) -> Failure {
        match self {
            Self::Failure(failure) => *failure,
            Self::PrivateProtocolMismatch { .. } | Self::Rule(_) => Failure::InvalidConfiguration,
        }
    }
}

/// Which rule of the configuration stage refused (`connectors.cli.ConfigurationRule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// No absolute configuration path resolves: no `--config`, no
    /// `XDG_CONFIG_HOME` and no `HOME`, or one that is not admissible.
    ConfigPathUnresolved,
    /// No file is at the resolved path.
    ConfigMissing,
    /// The file, or a directory above it, is not owner-only.
    ConfigNotPrivate,
    /// `owner_uid` is not the effective user.
    OwnerUidMismatch,
    /// A clock use's refusal, not the whole configuration's: `[approval_clock]`
    /// is the retired single-source `roughtime-clock/1` table.
    ApprovalClockFormatRetired,
}

impl Rule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigPathUnresolved => "config_path_unresolved",
            Self::ConfigMissing => "config_missing",
            Self::ConfigNotPrivate => "config_not_private",
            Self::OwnerUidMismatch => "owner_uid_mismatch",
            Self::ApprovalClockFormatRetired => "approval_clock_format_retired",
        }
    }
}

/// The `[custody]` table of a `connectors-local/4` configuration
/// (`connectors.cli.LocalCustodyBinding`): the binding, and the mounted
/// directory exactly when the binding is `mounted_secret`. Either half-table
/// (`mounted_secret` without a directory, a platform binding with one) is
/// refused while decoding, so no value of this type stands for it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "CustodyTable", into = "CustodyTable")]
pub enum Custody {
    /// A platform binding, selected explicitly.
    Platform(PlatformBinding),
    /// The read-only mounted binding and its directory.
    Mounted { directory: PathBuf },
}

/// A platform custody binding: every `connectors.cli.CustodyBackend` except
/// the read-only `mounted_secret`, which has no platform fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformBinding {
    SecretService,
    MacosKeychain,
    WindowsCredentialManager,
}

impl From<PlatformBinding> for CustodyBinding {
    fn from(binding: PlatformBinding) -> Self {
        match binding {
            PlatformBinding::SecretService => Self::SecretService,
            PlatformBinding::MacosKeychain => Self::MacosKeychain,
            PlatformBinding::WindowsCredentialManager => Self::WindowsCredentialManager,
        }
    }
}

/// The `[custody]` table as written.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CustodyTable {
    binding: CustodyBinding,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    directory: Option<PathBuf>,
}

impl TryFrom<CustodyTable> for Custody {
    type Error = &'static str;
    fn try_from(table: CustodyTable) -> std::result::Result<Self, Self::Error> {
        match (table.binding, table.directory) {
            (CustodyBinding::MountedSecret, Some(directory)) => Ok(Self::Mounted { directory }),
            (CustodyBinding::MountedSecret, None) => {
                Err("the mounted_secret binding names its directory")
            }
            (CustodyBinding::SecretService, None) => {
                Ok(Self::Platform(PlatformBinding::SecretService))
            }
            (CustodyBinding::MacosKeychain, None) => {
                Ok(Self::Platform(PlatformBinding::MacosKeychain))
            }
            (CustodyBinding::WindowsCredentialManager, None) => {
                Ok(Self::Platform(PlatformBinding::WindowsCredentialManager))
            }
            (_, Some(_)) => Err("only the mounted_secret binding names a directory"),
        }
    }
}

impl From<Custody> for CustodyTable {
    fn from(custody: Custody) -> Self {
        match custody {
            Custody::Platform(binding) => Self {
                binding: binding.into(),
                directory: None,
            },
            Custody::Mounted { directory } => Self {
                binding: CustodyBinding::MountedSecret,
                directory: Some(directory),
            },
        }
    }
}

/// `connectors.cli.CustodyBackend`, as a configuration names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyBinding {
    SecretService,
    MacosKeychain,
    WindowsCredentialManager,
    MountedSecret,
}

impl CustodyBinding {
    fn backend(self) -> connectors_custody::CustodyBackend {
        use connectors_custody::CustodyBackend as B;
        match self {
            Self::SecretService => B::SecretService,
            Self::MacosKeychain => B::MacosKeychain,
            Self::WindowsCredentialManager => B::WindowsCredentialManager,
            Self::MountedSecret => B::MountedSecret,
        }
    }
}

/// The custody binding a configuration selects: the platform binding with
/// its optional Secret Service socket, or the read-only mounted directory.
/// Never chosen by a request, and never another binding when this one is
/// unusable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Platform {
        secret_service_socket: Option<PathBuf>,
    },
    Mounted {
        directory: PathBuf,
    },
}

impl Selection {
    /// The binding `setup check` reports: `None` where the build has no
    /// platform binding.
    pub fn backend(&self) -> Option<connectors_custody::CustodyBackend> {
        match self {
            Self::Platform { .. } => connectors_custody::PLATFORM,
            Self::Mounted { .. } => Some(connectors_custody::CustodyBackend::MountedSecret),
        }
    }

    /// Whether the binding is read-only: it writes, rotates and deletes nothing.
    pub fn read_only(&self) -> bool {
        matches!(self, Self::Mounted { .. })
    }
}
impl From<Failure> for Refusal {
    fn from(failure: Failure) -> Self {
        Self::Failure(failure)
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
    /// Per write operation of this adapter whose answer carries a credential
    /// it created, the adapter alias and profile that credential is held under
    /// (`connectors.mutations.CredentialCustody`; `docs/local-mutation-ledger.md`,
    /// "Created credentials"). Omitted when empty, so a configuration without
    /// it keeps its bytes.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub custody: BTreeMap<String, CustodyTarget>,
}

/// Where a created credential is held: another configured adapter alias, and
/// one of the profiles its permissions grant. The configuration is refused
/// when it loads unless the target is not the writer, exists, grants the
/// profile and declares no custody target of its own.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustodyTarget {
    pub adapter: String,
    pub profile: String,
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

/// An operator-pinned consumer: an executable that `connections launch` starts
/// with one connection's protected document on descriptor 3. `executable.args`
/// is the pinned argv prefix; the caller's `--args` follow it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub executable: Executable,
    #[serde(default)]
    pub permissions: ConsumerPermissions,
    /// Name prefixes of the caller's environment variables passed to the
    /// consumer. Omitted or empty passes nothing.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub pass_env: BTreeSet<String>,
}

/// The adapter aliases whose connections a consumer may receive. Omitted or
/// empty denies every connection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumerPermissions {
    #[serde(default)]
    pub connections: BTreeSet<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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
    connectors_core::valid_id(value)
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        Self::read(path).map_err(|refusal| refusal.failure())
    }

    /// Load like [`Config::load`], keeping which rule refused the file where
    /// that can be named by operator-written coordinates alone.
    pub fn read(path: &Path) -> std::result::Result<Self, Refusal> {
        let file = fs::private_file(path).map_err(|failure| {
            // Only the rule's name depends on this second look; admission was
            // already refused by the descriptor walk above.
            match std::fs::symlink_metadata(path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    Refusal::Rule(Rule::ConfigMissing)
                }
                _ if failure == Failure::InvalidConfiguration => {
                    Refusal::Rule(Rule::ConfigNotPrivate)
                }
                _ => Refusal::Failure(failure),
            }
        })?;
        let bytes = fs::read_bounded(file, 1024 * 1024)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Failure::InvalidConfiguration)?;
        // Decode the closed envelope without its embedded validation, so the
        // refusal below can name its rule instead of collapsing into serde's.
        let input: ConfigInput = toml::from_str(text).map_err(|_| Failure::InvalidConfiguration)?;
        let config = Self::assemble(input);
        config.check()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        self.check().map_err(|refusal| refusal.failure())
    }

    /// The custody binding this configuration selects. Absent `[custody]`,
    /// the platform binding, exactly as before `connectors-local/4`.
    pub fn selection(&self) -> Selection {
        match &self.custody {
            Some(Custody::Mounted { directory }) => Selection::Mounted {
                directory: directory.clone(),
            },
            None | Some(Custody::Platform(_)) => Selection::Platform {
                secret_service_socket: self.secret_service_socket.clone(),
            },
        }
    }

    /// `[custody]` is a `connectors-local/4` table. Its binding is one this
    /// build supports: the platform's, or `mounted_secret` on Linux. The
    /// directory is named exactly for `mounted_secret`, which never sits beside
    /// a Secret Service socket.
    fn check_custody(&self, custody: &Custody) -> std::result::Result<(), Refusal> {
        let (supported, directory) = match custody {
            Custody::Mounted { directory } => (
                cfg!(target_os = "linux") && self.secret_service_socket.is_none(),
                Some(directory),
            ),
            Custody::Platform(binding) => (
                connectors_custody::PLATFORM == Some(CustodyBinding::from(*binding).backend()),
                None,
            ),
        };
        if self.format != "connectors-local/4" || !supported {
            return Err(Failure::InvalidConfiguration.into());
        }
        if let Some(directory) = directory {
            fs::validate_path(directory)?;
        }
        Ok(())
    }

    fn check(&self) -> std::result::Result<(), Refusal> {
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
            "connectors-local/1"
                | "connectors-local/2"
                | "connectors-local/3"
                | "connectors-local/4"
        ) {
            return Err(Failure::InvalidConfiguration.into());
        }
        if self.owner_uid != fs::uid() {
            return Err(Refusal::Rule(Rule::OwnerUidMismatch));
        }
        if let Some(custody) = &self.custody {
            self.check_custody(custody)?;
        }
        if self.adapters.len() > 64
            || self.consumers.len() > 64
            || (!matches!(
                self.format.as_str(),
                "connectors-local/3" | "connectors-local/4"
            ) && !self.consumers.is_empty())
            || self
                .default_adapter
                .as_ref()
                .is_some_and(|alias| !self.adapters.contains_key(alias))
        {
            return Err(Failure::InvalidConfiguration.into());
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
                || entry.permissions.profiles.len() > 64
                || entry.permissions.operations.len() > 256
                || entry
                    .permissions
                    .profiles
                    .iter()
                    .chain(&entry.permissions.operations)
                    .any(|id| !selector(id))
            {
                return Err(Failure::InvalidConfiguration.into());
            }
            entry.executable.validate()?;
        }
        for (alias, entry) in &self.adapters {
            for (operation, target) in &entry.permissions.custody {
                let held = self.adapters.get(&target.adapter);
                if self.format == "connectors-local/1"
                    || !entry.permissions.operations.contains(operation)
                    || &target.adapter == alias
                    || !selector(&target.profile)
                    || held.is_none_or(|held| {
                        !held.permissions.profiles.contains(&target.profile)
                            || !held.permissions.custody.is_empty()
                    })
                {
                    return Err(Failure::InvalidConfiguration.into());
                }
            }
        }
        for (name, consumer) in &self.consumers {
            if !selector(name)
                || consumer.permissions.connections.len() > 64
                || consumer
                    .permissions
                    .connections
                    .iter()
                    .any(|alias| !self.adapters.contains_key(alias))
                || consumer.pass_env.len() > 64
                || consumer.pass_env.iter().any(|prefix| {
                    prefix.is_empty()
                        || prefix.len() > 256
                        || prefix.contains('=')
                        || prefix.contains('\0')
                })
            {
                return Err(Failure::InvalidConfiguration.into());
            }
            consumer.executable.validate()?;
        }
        // Named only once every entry passed every other check, so the refusal
        // does not depend on alias order and both coordinates are already a
        // supported format and a valid selector. The first mismatch in alias
        // order is named.
        let current = self.format != "connectors-local/1";
        if let Some(entry) = self
            .adapters
            .values()
            .find(|entry| current != entry.private_protocol.is_some())
        {
            return Err(Refusal::PrivateProtocolMismatch {
                format: self.format.clone(),
                instance_id: entry.instance_id.clone(),
            });
        }
        Ok(())
    }

    pub fn initialize(paths: &Paths) -> Result<Initialized> {
        let parent = fs::directory(
            paths.config.parent().ok_or(Failure::InvalidConfiguration)?,
            true,
            true,
        )?;
        // Refuse an existing destination before touching state, including an
        // unsafe existing destination. The final publication is also exclusive.
        match std::fs::symlink_metadata(&paths.config) {
            Ok(_) => return Self::initialize_state(paths),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Failure::InvalidConfiguration),
        }
        let state = fs::directory(&paths.state, true, true)?;
        super::metadata::Metadata::initialize(&paths.state)?;
        fs::sync_directory(&state)?;
        let config = Self {
            format: "connectors-local/2".into(),
            owner_uid: fs::uid(),
            approval_clock: None,
            secret_service_socket: None,
            custody: None,
            default_adapter: None,
            adapters: BTreeMap::new(),
            consumers: BTreeMap::new(),
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
        )?;
        Ok(Initialized::Created)
    }

    /// An existing configuration is never rewritten. Its state is initialised
    /// only when no metadata database exists at the selected state path and
    /// the configuration itself loads; any existing database, and any
    /// configuration that does not load, is refused as an existing
    /// configuration. A present database is never opened, migrated or
    /// recreated here.
    fn initialize_state(paths: &Paths) -> Result<Initialized> {
        match std::fs::symlink_metadata(paths.state.join(METADATA_DATABASE)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(Failure::ConfigurationExists),
        }
        Self::load(&paths.config).map_err(|_| Failure::ConfigurationExists)?;
        let state = fs::directory(&paths.state, true, true)?;
        super::metadata::Metadata::initialize(&paths.state)?;
        fs::sync_directory(&state)?;
        Ok(Initialized::StateInitialized)
    }
}

/// The metadata database's file name in the state directory (owned by
/// `metadata.rs`); only its absence is observed here.
const METADATA_DATABASE: &str = "metadata.sqlite3";

/// What `setup init` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Initialized {
    /// A new configuration and its state were created.
    Created,
    /// The existing configuration was kept and its missing state created.
    StateInitialized,
}

impl Initialized {
    /// The CLI's `InitDisposition` variant.
    pub fn disposition(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::StateInitialized => "state_initialized",
        }
    }
}

impl Executable {
    /// The pinned selection's own form: a lowercase SHA-256 digest, a bounded
    /// argv without NUL and an admissible absolute path.
    fn validate(&self) -> Result<()> {
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || self.args.len() > 256
            || self
                .args
                .iter()
                .any(|arg| arg.len() > 4096 || arg.contains('\0'))
        {
            return Err(Failure::InvalidConfiguration);
        }
        fs::validate_path(&self.path)
    }

    /// Inspect the admitted artifact without executing it. Launch must repeat
    /// admission and retain the exact descriptor for exec; this is no readiness.
    pub fn check(&self) -> Result<()> {
        self.open().map(|_| ())
    }

    /// Return the admitted source file at EOF. Launch additionally captures and
    /// verifies a sealed snapshot: an inode can still be written in place.
    pub fn open(&self) -> Result<std::fs::File> {
        let mut file = self.source()?;
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

    /// Admit the source path and return only a verified sealed snapshot. Hash
    /// while copying: hashing the mutable source first adds no exec guarantee
    /// and needlessly spends the original startup deadline a second time.
    pub(crate) fn capture(
        &self,
        until: std::time::Instant,
    ) -> super::runtime::Result<super::runtime::artifact::Image> {
        let source = self
            .source()
            .map_err(|_| super::runtime::Failure::InvalidConfiguration)?;
        super::runtime::artifact::capture(source, &self.sha256, until)
    }

    // Content is deliberately unverified here. Keep this private so callers
    // cannot confuse source path admission with an executable snapshot.
    // Windows: the same admission on the opened handle, with the owner SID and
    // the access control list in place of uid and mode bits. Windows has no
    // execute bit to require; the PE format is checked at capture.
    #[cfg(windows)]
    fn source(&self) -> Result<std::fs::File> {
        let parent = fs::directory(
            self.path.parent().ok_or(Failure::InvalidConfiguration)?,
            false,
            false,
        )?;
        let file = fs::windows::open_child(
            &parent,
            self.path.file_name().ok_or(Failure::InvalidConfiguration)?,
            windows_sys::Win32::Foundation::GENERIC_READ,
        )
        .map_err(|_| Failure::InvalidConfiguration)?;
        fs::windows::check_trusted_file(&file)?;
        if file
            .metadata()
            .map_err(|_| Failure::InvalidConfiguration)?
            .len()
            > 512 * 1024 * 1024
        {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(file)
    }

    #[cfg(unix)]
    fn source(&self) -> Result<std::fs::File> {
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
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        let stat = file.metadata().map_err(|_| Failure::InvalidConfiguration)?;
        if !stat.is_file()
            || (stat.uid() != fs::uid() && stat.uid() != 0)
            || stat.mode() & 0o022 != 0
            || stat.mode() & 0o111 == 0
            || stat.len() > 512 * 1024 * 1024
        {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(file)
    }
}

// The fixtures configure Unix absolute executable paths.
#[cfg(all(test, unix))]
mod consumer_tests {
    //! story:launch-consumer-with-connection-credential: `connectors-local/3`
    //! adds `[consumers]`; `/1` and `/2` stay readable and refuse it.
    use super::*;

    const ADAPTER: &str = "[adapters.fixture]\ninstance_id='fixture-instance'\nadapter_id='fixture-adapter'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nPRIVATE[adapters.fixture.executable]\npath='/not-installed/adapter'\nsha256='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nargs=[]\n";
    const PROBE: &str = "[consumers.probe.executable]\npath='/not-installed/consumer'\nsha256='bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'\nargs=['--password-file','/proc/self/fd/3']\n[consumers.probe.permissions]\nconnections=['fixture']\n";

    fn read(format: &str, private: bool, consumers: &str) -> std::result::Result<Config, Refusal> {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        let adapter = ADAPTER.replace(
            "PRIVATE",
            if private {
                "private_protocol='connectors-private/1'\n"
            } else {
                ""
            },
        );
        std::fs::write(
            &paths.config,
            format!(
                "format='{format}'\nowner_uid={}\n{adapter}{consumers}",
                fs::uid()
            ),
        )
        .unwrap();
        Config::read(&paths.config)
    }

    #[test]
    fn format_three_adds_consumers_whose_permissions_name_adapter_aliases() {
        let config = read("connectors-local/3", true, PROBE).unwrap();
        let probe = &config.consumers["probe"];
        assert_eq!(
            probe.permissions.connections,
            BTreeSet::from(["fixture".to_owned()])
        );
        assert_eq!(
            probe.executable.args,
            ["--password-file", "/proc/self/fd/3"]
        );
        // Omitted permissions deny every connection.
        let bare = PROBE.replace(
            "[consumers.probe.permissions]\nconnections=['fixture']\n",
            "",
        );
        let config = read("connectors-local/3", true, &bare).unwrap();
        assert!(config.consumers["probe"].permissions.connections.is_empty());
        // `/3` without consumers loads, and the earlier formats keep loading.
        assert!(
            read("connectors-local/3", true, "")
                .unwrap()
                .consumers
                .is_empty()
        );
        assert!(read("connectors-local/2", true, "").is_ok());
        assert!(read("connectors-local/1", false, "").is_ok());
    }

    #[test]
    fn a_consumer_passes_caller_variables_only_by_listed_prefix() {
        let entry = |pass_env: &str| {
            PROBE.replace(
                "[consumers.probe.executable]",
                &format!("[consumers.probe]\npass_env={pass_env}\n[consumers.probe.executable]"),
            )
        };
        let config = read("connectors-local/3", true, &entry("['EKR_','CORTEX_']")).unwrap();
        assert_eq!(
            config.consumers["probe"].pass_env,
            BTreeSet::from(["CORTEX_".to_owned(), "EKR_".to_owned()])
        );
        // Without `pass_env` nothing passes.
        let config = read("connectors-local/3", true, PROBE).unwrap();
        assert!(config.consumers["probe"].pass_env.is_empty());
        // An empty prefix would pass everything; `=` and NUL are no name.
        for refused in ["['']", "['A=B']", "[\"A\\u0000\"]", "'EKR_'"] {
            assert_eq!(
                read("connectors-local/3", true, &entry(refused)).err(),
                Some(Refusal::Failure(Failure::InvalidConfiguration)),
                "{refused}"
            );
        }
    }

    #[test]
    fn consumers_are_refused_outside_format_three_and_when_malformed() {
        let invalid = Some(Refusal::Failure(Failure::InvalidConfiguration));
        assert_eq!(
            read("connectors-local/2", true, PROBE).err(),
            invalid.clone()
        );
        assert_eq!(
            read("connectors-local/1", false, PROBE).err(),
            invalid.clone()
        );
        for malformed in [
            // A permission naming no configured adapter alias.
            PROBE.replace("connections=['fixture']", "connections=['absent']"),
            // A name that is no selector.
            PROBE.replace("consumers.probe", "consumers.'not a selector'"),
            PROBE.replace("sha256='bbbb", "sha256='BBBB"),
            PROBE.replace("path='/not-installed/consumer'", "path='consumer'"),
            PROBE.replace("args=[", "env=[]\nargs=["),
            PROBE.replace("connections=", "operations=[]\nconnections="),
        ] {
            assert_eq!(
                read("connectors-local/3", true, &malformed).err(),
                invalid.clone(),
                "{malformed}"
            );
        }
        // `/3` keeps `/2`'s rule: every adapter entry selects a private protocol.
        assert_eq!(
            read("connectors-local/3", false, PROBE).err(),
            Some(Refusal::PrivateProtocolMismatch {
                format: "connectors-local/3".into(),
                instance_id: "fixture-instance".into(),
            })
        );
    }
}

#[cfg(test)]
mod custody_tests {
    //! story:mounted-secret-custody: a `[custody]` table names the mounted
    //! directory exactly for `mounted_secret`. Neither half-table decodes, so
    //! no configuration can carry a mounted binding without its directory and
    //! fall back to the platform binding.
    use super::*;

    #[test]
    fn a_mounted_binding_without_its_directory_does_not_decode() {
        assert!(toml::from_str::<Custody>("binding = 'mounted_secret'").is_err());
        assert!(
            toml::from_str::<Custody>("binding = 'secret_service'\ndirectory = '/srv/custody'")
                .is_err()
        );
        let mounted: Custody =
            toml::from_str("binding = 'mounted_secret'\ndirectory = '/srv/custody'").unwrap();
        let platform: Custody = toml::from_str("binding = 'secret_service'").unwrap();
        for custody in [mounted, platform] {
            let text = toml::to_string(&custody).unwrap();
            assert_eq!(toml::from_str::<Custody>(&text).unwrap(), custody, "{text}");
        }
    }
}

// The fixtures configure Unix absolute executable paths.
#[cfg(all(test, unix))]
mod custody_target_tests {
    //! `[adapters.<alias>.permissions.custody]` names, per write operation of
    //! that alias, another configured alias and one of its granted profiles a
    //! created credential is held under (`docs/local-mutation-ledger.md`,
    //! "Created credentials"). Everything else refuses the file when it loads.
    use super::*;

    fn entry(alias: &str, instance: &str, permissions: &str) -> String {
        format!(
            "[adapters.{alias}]\ninstance_id='{instance}'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/adapter'\nsha256='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nargs=[]\n[adapters.{alias}.permissions]\n{permissions}"
        )
    }
    const WRITER: &str = "profiles=['fixture.management']\noperations=['key.get','key.create']\n[adapters.minter.permissions.custody]\n'key.create'={adapter='minted',profile='fixture.key'}\n";
    const TARGET: &str = "profiles=['fixture.key']\noperations=['key.get']\n";

    fn read(format: &str, writer: &str, target: &str) -> std::result::Result<Config, Refusal> {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        std::fs::write(
            &paths.config,
            format!(
                "format='{format}'\nowner_uid={}\n{}{}",
                fs::uid(),
                entry("minter", "fixture-minter", writer),
                entry("minted", "fixture-minted", target),
            ),
        )
        .unwrap();
        Config::read(&paths.config)
    }

    #[test]
    fn a_custody_target_is_another_alias_and_one_of_its_granted_profiles() {
        let config = read("connectors-local/2", WRITER, TARGET).unwrap();
        assert_eq!(
            config.adapters["minter"].permissions.custody["key.create"],
            CustodyTarget {
                adapter: "minted".into(),
                profile: "fixture.key".into(),
            }
        );
        assert!(config.adapters["minted"].permissions.custody.is_empty());
        // Absent, the table serialises as it did before it existed.
        let text = toml::to_string(&config.adapters["minted"].permissions).unwrap();
        assert!(!text.contains("custody"), "{text}");
        let invalid = Some(Refusal::Failure(Failure::InvalidConfiguration));
        for (writer, target) in [
            // The writer itself.
            (
                WRITER.replace("adapter='minted'", "adapter='minter'"),
                TARGET.to_owned(),
            ),
            // No such alias.
            (
                WRITER.replace("adapter='minted'", "adapter='absent'"),
                TARGET.to_owned(),
            ),
            // A profile the target is not granted, and one that is no id.
            (
                WRITER.replace("profile='fixture.key'", "profile='fixture.other'"),
                TARGET.to_owned(),
            ),
            (
                WRITER.replace("profile='fixture.key'", "profile='not an id'"),
                TARGET.to_owned(),
            ),
            // An operation the writer is not granted.
            (
                WRITER.replace("'key.create'={", "'key.delete'={"),
                TARGET.to_owned(),
            ),
            // An unknown member of the target table.
            (
                WRITER.replace("profile='fixture.key'}", "profile='fixture.key',extra=1}"),
                TARGET.to_owned(),
            ),
            // A target that holds custody of its own.
            (
                WRITER.to_owned(),
                format!(
                    "{TARGET}[adapters.minted.permissions.custody]\n'key.get'={{adapter='minter',profile='fixture.management'}}\n"
                ),
            ),
        ] {
            assert_eq!(
                read("connectors-local/2", &writer, &target).err(),
                invalid,
                "{writer}{target}"
            );
        }
    }
}
