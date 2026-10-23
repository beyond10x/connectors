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
#[serde(try_from = "ConfigInput", into = "Written")]
pub struct Config {
    pub format: String,
    pub owner_uid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_clock: Option<super::clock::ApprovalClock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_service_socket: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_adapter: Option<String>,
    /// `"<connector>/<instance>"`, selected when a command names no instance
    /// (`connectors-local/5`); never beside `default_adapter`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_instance: Option<String>,
    #[serde(default)]
    pub adapters: BTreeMap<String, Adapter>,
    /// The explicitly selected custody binding (`connectors-local/4` and `/5`);
    /// absent, custody is the platform binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody: Option<Custody>,
    /// Operator-pinned consumer executables (`connectors-local/3` and later).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub consumers: BTreeMap<String, Consumer>,
    /// The `[connectors]` table (`connectors-local/5`), `None` when the file
    /// has none. An instance's entry is in `adapters` under its alias, and
    /// only there: that entry is what is checked, served and written back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connectors: Option<BTreeMap<String, Connector>>,
    /// The instances of invoked definition files held beside this configuration
    /// (`definitions`), by alias, each with its connector: in `adapters` like
    /// every entry, named `<connector>/definition` or, moved, `<connector>/definition-<16 hex>`
    /// (`definitions::moved`), and never written to the
    /// operator's file. Empty unless read with [`Config::read_at`].
    pub definitions: BTreeMap<String, String>,
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
    default_instance: Option<String>,
    custody: Option<Custody>,
    #[serde(default)]
    adapters: BTreeMap<String, Adapter>,
    #[serde(default)]
    consumers: BTreeMap<String, Consumer>,
    connectors: Option<BTreeMap<String, ConnectorTable>>,
}
/// How an unpinned instance of a `connectors-local/5` file is admitted.
#[derive(Clone, Copy)]
enum Derivation<'a> {
    /// Its entry is derived from the installation; with none, nothing derives
    /// it and it refuses the file.
    From(Option<&'a Installation>),
    /// It is checked as a derivation checks it before the provider starts, and
    /// listed without an entry: no process is started (`connectors list`).
    Deferred,
}

impl Config {
    fn assemble(
        input: ConfigInput,
        installation: Derivation<'_>,
    ) -> std::result::Result<Self, Refusal> {
        let mut config = Self {
            format: input.format,
            owner_uid: input.owner_uid,
            approval_clock: input.approval_clock,
            secret_service_socket: input.secret_service_socket,
            custody: input.custody,
            default_adapter: input.default_adapter,
            default_instance: input.default_instance,
            adapters: input.adapters,
            consumers: input.consumers,
            connectors: None,
            definitions: BTreeMap::new(),
        };
        config.connectors = input
            .connectors
            .map(|tables| config.admit_instances(tables, installation))
            .transpose()?;
        Ok(config)
    }
}
impl TryFrom<ConfigInput> for Config {
    type Error = &'static str;
    fn try_from(input: ConfigInput) -> std::result::Result<Self, Self::Error> {
        let config = Self::assemble(input, Derivation::From(Installation::installed().as_ref()))
            .map_err(|_| "invalid local configuration")?;
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
    /// A configuration text is larger than [`CONFIG_LIMIT`], the most a read
    /// admits: the text `setup migrate` would write, which it then does not.
    ConfigTooLarge,
    /// An `operations invoke --definition` whose connector already has an
    /// instance named `definition` here, or whose instance id an entry here
    /// already uses (story:invoke-definition-file): the configured one is never
    /// replaced.
    DefinitionInstanceConfigured,
}

/// The most bytes a configuration read admits.
pub const CONFIG_LIMIT: usize = 1024 * 1024;

impl Rule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigPathUnresolved => "config_path_unresolved",
            Self::ConfigMissing => "config_missing",
            Self::ConfigNotPrivate => "config_not_private",
            Self::OwnerUidMismatch => "owner_uid_mismatch",
            Self::ApprovalClockFormatRetired => "approval_clock_format_retired",
            Self::ConfigTooLarge => "config_too_large",
            Self::DefinitionInstanceConfigured => "definition_instance_configured",
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
    /// The credential-side digest the implementation prints beside the
    /// configuration revision (`runtime::Bootstrap::credential_revision`),
    /// stated so a connection is bound to it (architecture-decision-record:
    /// provider-definitions-and-runtimes D12). A derived entry takes it from
    /// the printed bootstrap. Absent, connections keep the configuration
    /// revision rule. It is not part of the executable selection digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_revision: Option<String>,
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
    /// What this entry binds a connection to, for an observation that starts
    /// no provider (D12).
    pub fn configured(&self) -> super::registry::Configured<'_> {
        super::registry::Configured {
            configuration_revision: &self.configuration_revision,
            credential_revision: self.credential_revision.as_deref(),
            executable: Some(&self.executable.sha256),
        }
    }
}

/// A `[connectors.<connector>]` table (`connectors-local/5`): whether the
/// connector is enabled, and its instances by name. `enabled` is stated, never
/// assumed; what a disabled connector's instance means is not decided yet, so
/// only a connector without instances may be disabled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connector {
    pub enabled: bool,
    pub instances: BTreeMap<String, Instance>,
}

/// A `[connectors.<connector>.instances.<instance>]` table, without its pins.
/// An instance that carries the pins of a legacy `[adapters.<alias>]` entry,
/// its alias included, loads to exactly that entry, which is held in
/// `Config::adapters` under `alias`: the same instance id, configuration
/// revision, executable selection digest and mounted-custody item names. The
/// connector name of such an instance is checked as a name only, against no
/// connector declaration. An instance that carries no pin at all is
/// `derived` (story:connector-instance-derived-fields): the installed catalog
/// provider derives its entry, held under the alias `<connector>.<instance>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    pub description: Option<String>,
    pub endpoint: Option<String>,
    pub alias: String,
    /// Whether the entry is derived rather than pinned; a derived instance is
    /// written back without pins.
    pub derived: bool,
    /// The profile a derived instance names (`profile`), when its connector
    /// declares more than one.
    pub profile: Option<String>,
    /// The profiles the connector of a derived instance declares when there
    /// are several and the instance names none: it has no entry in
    /// `Config::adapters` until it does ([`Config::awaiting_profile`]).
    pub awaiting_profile: Option<BTreeSet<String>>,
}

/// `[connectors.<connector>]` as written.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ConnectorTable {
    enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    instances: BTreeMap<String, InstanceTable>,
}

/// `[connectors.<connector>.instances.<instance>]` as written: the instance's
/// own keys and the legacy entry's pins.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstanceTable {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    adapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    configuration_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    credential_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(deserialize_with = "private_selection")]
    private_protocol: Option<super::runtime::PrivateProtocol>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    startup: Option<Startup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    restart: Option<Restart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    executable: Option<Executable>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    permissions: Option<Permissions>,
}

impl InstanceTable {
    /// The instance and the legacy entry it pins; `None` unless it carries
    /// every pin the legacy entry requires. `startup`, `restart` and
    /// `permissions` default as in the legacy entry.
    fn pinned(self) -> Option<(Instance, Adapter)> {
        // A profile is the derived instance's; the pins already select one.
        if self.profile.is_some() {
            return None;
        }
        let adapter = Adapter {
            credential_revision: self.credential_revision,
            instance_id: self.instance_id?,
            adapter_id: self.adapter_id?,
            configuration_revision: self.configuration_revision?,
            protocol: self.protocol?,
            private_protocol: self.private_protocol,
            startup: self.startup.unwrap_or_default(),
            restart: self.restart.unwrap_or_default(),
            executable: self.executable?,
            permissions: self.permissions.unwrap_or_default(),
        };
        let instance = Instance {
            description: self.description,
            endpoint: self.endpoint,
            alias: self.alias?,
            derived: false,
            profile: None,
            awaiting_profile: None,
        };
        Some((instance, adapter))
    }

    /// The table written for `instance`, its pins taken from `adapter`, the
    /// entry under its alias: every pin is written, as the legacy entry
    /// writes them.
    fn written(instance: Instance, adapter: Adapter) -> Self {
        Self {
            description: instance.description,
            endpoint: instance.endpoint,
            profile: None,
            alias: Some(instance.alias),
            instance_id: Some(adapter.instance_id),
            adapter_id: Some(adapter.adapter_id),
            configuration_revision: Some(adapter.configuration_revision),
            credential_revision: adapter.credential_revision,
            protocol: Some(adapter.protocol),
            private_protocol: adapter.private_protocol,
            startup: Some(adapter.startup),
            restart: Some(adapter.restart),
            executable: Some(adapter.executable),
            permissions: Some(adapter.permissions),
        }
    }

    /// Whether the table states no pin: only `description` and `endpoint`.
    fn unpinned(&self) -> bool {
        self.alias.is_none()
            && self.instance_id.is_none()
            && self.adapter_id.is_none()
            && self.configuration_revision.is_none()
            && self.credential_revision.is_none()
            && self.protocol.is_none()
            && self.private_protocol.is_none()
            && self.startup.is_none()
            && self.restart.is_none()
            && self.executable.is_none()
            && self.permissions.is_none()
    }

    /// The instance `name` of the catalog connector `connector` and the entry
    /// the installed catalog provider derives for it: the provider started as
    /// `--connector <connector> --instance <name> [--endpoint <endpoint>]`,
    /// its SHA-256 computed here, the instance id and configuration revision
    /// of the bootstrap it prints for exactly that selection, and the profiles
    /// and every operation, writes included, of the bootstrap a
    /// `connectors-private/2` child of that entry reports, which must name the
    /// same instance and revision. A stated `profile` is passed as
    /// `--profile`. With none, an instance of a connector declaring several
    /// profiles is listed without an entry, naming them (the coordinator's
    /// decision, round 3), and refuses nothing else. A connector the provider
    /// does not ship (Slack, the Atlassian gateway, every non-catalog
    /// implementation) is refused.
    fn derived(
        self,
        connector: &str,
        name: &str,
        installation: &Installation,
    ) -> Result<(Instance, Option<Adapter>)> {
        let (mut instance, args) = self.underived(connector, name)?;
        match installation.derive(&instance.alias, args, instance.profile.is_some())? {
            Derived::Entry(adapter) => Ok((instance, Some(*adapter))),
            Derived::Awaiting(profiles) => {
                instance.awaiting_profile = Some(profiles);
                Ok((instance, None))
            }
        }
    }

    /// The instance `name` of the connector `connector` as [`Self::derived`]
    /// admits it before the provider is started, and the arguments the
    /// provider would be started with: everything a derivation checks except
    /// what only the provider answers.
    fn underived(self, connector: &str, name: &str) -> Result<(Instance, Vec<String>)> {
        // `<connector>.<instance>` stays unambiguous: neither name has a `.`.
        let plain = |value: &str| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        };
        let endpoint_admitted = self.endpoint.as_deref().is_none_or(|endpoint| {
            !endpoint.is_empty()
                && endpoint.len() <= 2048
                && !endpoint
                    .chars()
                    .any(|c| c.is_control() || c.is_whitespace())
        });
        if !plain(connector)
            || !plain(name)
            || !endpoint_admitted
            || self.profile.as_deref().is_some_and(|p| !selector(p))
        {
            return Err(Failure::InvalidConfiguration);
        }
        let mut args = ["--connector", connector, "--instance", name]
            .map(str::to_owned)
            .to_vec();
        if let Some(endpoint) = &self.endpoint {
            args.extend(["--endpoint".to_owned(), endpoint.clone()]);
        }
        if let Some(profile) = &self.profile {
            args.extend(["--profile".to_owned(), profile.clone()]);
        }
        let instance = Instance {
            description: self.description,
            endpoint: self.endpoint,
            alias: format!("{connector}.{name}"),
            derived: true,
            profile: self.profile,
            awaiting_profile: None,
        };
        Ok((instance, args))
    }

    /// The table written for a derived instance: its own keys, no pin.
    fn bare(instance: Instance) -> Self {
        Self {
            description: instance.description,
            endpoint: instance.endpoint,
            profile: instance.profile,
            alias: None,
            instance_id: None,
            adapter_id: None,
            configuration_revision: None,
            credential_revision: None,
            protocol: None,
            private_protocol: None,
            startup: None,
            restart: None,
            executable: None,
            permissions: None,
        }
    }
}

/// A configuration as written. `adapters` is the only home of an entry: an
/// instance's pins are rebuilt from the entry under its alias, an instance
/// whose entry was removed is not written, and an entry is under
/// `[adapters]` exactly when no instance names its alias.
#[derive(Serialize)]
struct Written {
    format: String,
    owner_uid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval_clock: Option<super::clock::ApprovalClock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_service_socket: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_adapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_instance: Option<String>,
    adapters: BTreeMap<String, Adapter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custody: Option<Custody>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    consumers: BTreeMap<String, Consumer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connectors: Option<BTreeMap<String, ConnectorTable>>,
}

impl From<Config> for Written {
    fn from(config: Config) -> Self {
        let mut adapters = config.adapters;
        // An invoked definition's instance is held beside the operator's file,
        // never in it.
        for alias in config.definitions.keys() {
            adapters.remove(alias);
        }
        let connectors = config.connectors.map(|connectors| {
            connectors
                .into_iter()
                .map(|(name, connector)| {
                    let instances = connector
                        .instances
                        .into_iter()
                        .filter_map(|(name, instance)| {
                            let adapter = adapters.remove(&instance.alias);
                            let table = if instance.derived {
                                InstanceTable::bare(instance)
                            } else {
                                InstanceTable::written(instance, adapter?)
                            };
                            Some((name, table))
                        })
                        .collect();
                    let table = ConnectorTable {
                        enabled: connector.enabled,
                        instances,
                    };
                    (name, table)
                })
                .collect()
        });
        Self {
            format: config.format,
            owner_uid: config.owner_uid,
            approval_clock: config.approval_clock,
            secret_service_socket: config.secret_service_socket,
            default_adapter: config.default_adapter,
            default_instance: config.default_instance,
            adapters,
            custody: config.custody,
            consumers: config.consumers,
            connectors,
        }
    }
}

impl Config {
    /// Add every instance's entry to `adapters` under its alias. An instance
    /// with every pin is that entry; one with no pin is derived from
    /// `installation`, the installed catalog provider, in a
    /// `connectors-local/5` file only. An instance with some pins but not all, an unpinned one
    /// nothing derives, or one whose alias is already taken by a legacy entry
    /// or another instance, refuses the file.
    fn admit_instances(
        &mut self,
        tables: BTreeMap<String, ConnectorTable>,
        installation: Derivation<'_>,
    ) -> std::result::Result<BTreeMap<String, Connector>, Refusal> {
        let mut connectors = BTreeMap::new();
        for (name, table) in tables {
            let mut instances = BTreeMap::new();
            for (instance_name, table) in table.instances {
                let unpinned = table.unpinned() && self.format == "connectors-local/5";
                let (instance, adapter) = match installation {
                    Derivation::From(Some(installation)) if unpinned => {
                        table.derived(&name, &instance_name, installation)?
                    }
                    Derivation::Deferred if unpinned => {
                        let (instance, _) = table.underived(&name, &instance_name)?;
                        // The entry a derivation would insert under this alias.
                        if self.adapters.contains_key(&instance.alias) {
                            return Err(Failure::InvalidConfiguration.into());
                        }
                        (instance, None)
                    }
                    _ => table
                        .pinned()
                        .map(|(instance, adapter)| (instance, Some(adapter)))
                        .ok_or(Failure::InvalidConfiguration)?,
                };
                if let Some(adapter) = adapter
                    && self
                        .adapters
                        .insert(instance.alias.clone(), adapter)
                        .is_some()
                {
                    return Err(Failure::InvalidConfiguration.into());
                }
                instances.insert(instance_name, instance);
            }
            let connector = Connector {
                enabled: table.enabled,
                instances,
            };
            connectors.insert(name, connector);
        }
        Ok(connectors)
    }

    /// `[connectors]`, even empty, is a `connectors-local/5` table. Names are
    /// selectors, a connector with instances is enabled, and no two instances
    /// name one alias. An instance's pins are its entry in `adapters`, which
    /// the legacy entry's rules check (instance-id uniqueness among them)
    /// together with every legacy entry; an instance whose entry is gone from
    /// `adapters` is removed, and is not written back.
    fn check_connectors(&self) -> std::result::Result<(), Refusal> {
        let Some(connectors) = &self.connectors else {
            return Ok(());
        };
        if self.format != "connectors-local/5" || connectors.len() > 64 {
            return Err(Failure::InvalidConfiguration.into());
        }
        let text = |value: &Option<String>, empty: bool, limit: usize, space: bool| {
            value.as_ref().is_none_or(|value| {
                (empty || !value.is_empty())
                    && value.len() <= limit
                    && !value
                        .chars()
                        .any(|c| c.is_control() || (!space && c.is_whitespace()))
            })
        };
        let mut aliases = BTreeSet::new();
        for (name, connector) in connectors {
            if !selector(name) || (!connector.enabled && !connector.instances.is_empty()) {
                return Err(Failure::InvalidConfiguration.into());
            }
            for (name, instance) in &connector.instances {
                if !selector(name)
                    || !text(&instance.description, true, 1024, true)
                    || !text(&instance.endpoint, false, 2048, false)
                    || !aliases.insert(&instance.alias)
                    // A listed instance without an entry keeps its alias.
                    || (instance.awaiting_profile.is_some()
                        && self.adapters.contains_key(&instance.alias))
                {
                    return Err(Failure::InvalidConfiguration.into());
                }
            }
        }
        Ok(())
    }
}

/// What a command selects its instance by (`contracts/cli/v1alpha1/semantics.md`
/// §3, "Selecting an instance").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector<'a> {
    /// `--adapter <alias>`, deprecated: an entry by its alias.
    Alias(&'a str),
    /// `--connector <connector> --instance <instance>`: an instance name is
    /// unique within its connector only, so both are needed.
    Instance {
        connector: &'a str,
        instance: &'a str,
    },
    /// No selector: `default_instance`, else `default_adapter`.
    Default,
}

/// The entry a selector names, and the names it is selected by.
#[derive(Clone, Copy, Debug)]
pub struct Selected<'a> {
    /// Its key in `Config::adapters`: the alias `--adapter` and the JSON
    /// `adapter` field carry.
    pub alias: &'a str,
    pub connector: &'a str,
    pub instance: &'a str,
    pub adapter: &'a Adapter,
}

impl Config {
    /// The connector name, instance name and alias of each instance admitted
    /// under [`Derivation::Deferred`]: derived, awaiting no profile, and
    /// without the entry a derivation would have put under its alias. A
    /// configuration read any other way has none, so every rule that consults
    /// this behaves for it exactly as without it.
    fn deferred(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.connectors
            .iter()
            .flatten()
            .flat_map(|(connector, table)| {
                table
                    .instances
                    .iter()
                    .map(move |(name, instance)| (connector.as_str(), name.as_str(), instance))
            })
            .filter(|(_, _, instance)| {
                instance.derived
                    && instance.awaiting_profile.is_none()
                    && !self.adapters.contains_key(&instance.alias)
            })
            .map(|(connector, name, instance)| (connector, name, instance.alias.as_str()))
    }

    /// Whether `alias` names an entry, or the entry a deferred derivation
    /// would put under it. A deferred entry's own contents (its profiles and
    /// permissions) are the provider's answer and are not known here.
    fn names_entry(&self, alias: &str) -> bool {
        self.adapters.contains_key(alias)
            || self.deferred().any(|(_, _, deferred)| deferred == alias)
    }

    /// The connector and instance names of the entry under `alias`: those of
    /// the `[connectors]` instance that names the alias or, for a legacy
    /// `[adapters.<alias>]` entry no instance names, its `adapter_id` and its
    /// alias. `None` when no entry has the alias.
    pub fn names(&self, alias: &str) -> Option<(&str, &str)> {
        let (alias, adapter) = self.adapters.get_key_value(alias)?;
        if let Some(connector) = self.definitions.get(alias) {
            let instance = super::definitions::instance_of(connector, alias)
                .unwrap_or(super::definitions::INSTANCE);
            return Some((connector.as_str(), instance));
        }
        let declared = self
            .connectors
            .iter()
            .flatten()
            .find_map(|(connector, table)| {
                table
                    .instances
                    .iter()
                    .find(|(_, instance)| instance.alias == *alias)
                    .map(|(instance, _)| (connector.as_str(), instance.as_str()))
            });
        Some(declared.unwrap_or((adapter.adapter_id.as_str(), alias.as_str())))
    }

    /// The entry `selector` names, `None` when it names none; with no
    /// selector, also when neither default is configured.
    pub fn select(&self, selector: Selector<'_>) -> Option<Selected<'_>> {
        let alias = match selector {
            Selector::Alias(alias) => alias,
            Selector::Instance {
                connector,
                instance,
            } => self
                .adapters
                .keys()
                .find(|alias| self.names(alias) == Some((connector, instance)))?,
            Selector::Default => match (&self.default_instance, &self.default_adapter) {
                (Some(default), _) => {
                    let (connector, instance) = default.split_once('/')?;
                    return self.select(Selector::Instance {
                        connector,
                        instance,
                    });
                }
                (None, Some(alias)) => alias,
                (None, None) => return None,
            },
        };
        let (alias, adapter) = self.adapters.get_key_value(alias)?;
        let (connector, instance) = self.names(alias)?;
        Some(Selected {
            alias,
            connector,
            instance,
            adapter,
        })
    }

    /// Every entry has its own connector and instance names, so the two
    /// selectors name the same entries. `default_instance` is a
    /// `connectors-local/5` key naming one of them as `<connector>/<instance>`,
    /// and never beside `default_adapter`.
    fn check_selectors(&self) -> std::result::Result<(), Refusal> {
        let mut named = BTreeSet::new();
        if !self
            .adapters
            .keys()
            .map(|alias| self.names(alias))
            .chain(
                self.deferred()
                    .map(|(connector, name, _)| Some((connector, name))),
            )
            .all(|names| named.insert(names))
        {
            return Err(Failure::InvalidConfiguration.into());
        }
        if let Some(default) = &self.default_instance {
            let pair = default.split_once('/');
            let admitted =
                pair.is_some_and(|(connector, instance)| selector(connector) && selector(instance));
            if !admitted
                || self.default_adapter.is_some()
                || self.format != "connectors-local/5"
                || (self.select(Selector::Default).is_none()
                    && !self
                        .deferred()
                        .any(|(connector, name, _)| Some((connector, name)) == pair))
            {
                return Err(Failure::InvalidConfiguration.into());
            }
        }
        Ok(())
    }
}

/// `setup migrate` (story:setup-migrate-adapter-entries): rewriting legacy
/// entries as the pinned instances of story:connector-config-dual-read.
impl Config {
    /// The aliases of the `[adapters.<alias>]` entries: those no
    /// `[connectors]` instance names, in alias order.
    pub fn legacy_aliases(&self) -> Vec<&str> {
        let named: BTreeSet<&str> = self
            .connectors
            .iter()
            .flatten()
            .flat_map(|(_, connector)| connector.instances.values())
            .map(|instance| instance.alias.as_str())
            .collect();
        self.adapters
            .keys()
            .map(String::as_str)
            .filter(|alias| !named.contains(alias))
            .filter(|alias| !self.definitions.contains_key(*alias))
            .collect()
    }

    /// Whether `[connectors.<connector>]` can take a new instance named
    /// `instance`: the table is absent, or enabled and without an instance of
    /// that name. A disabled table is left as the operator wrote it.
    pub fn can_hold(&self, connector: &str, instance: &str) -> bool {
        selector(connector)
            && selector(instance)
            && self
                .connectors
                .as_ref()
                .and_then(|connectors| connectors.get(connector))
                .is_none_or(|table| table.enabled && !table.instances.contains_key(instance))
    }

    /// This configuration with the legacy entry under each alias of `moved`
    /// written as the pinned instance `[connectors.<connector>.instances.<alias>]`
    /// of the connector `moved` maps it to. The instance carries every pin of
    /// the entry, its alias included, so it reads back as exactly that entry;
    /// nothing else changes. The format becomes `connectors-local/5` when any
    /// entry moves. An alias that is no legacy entry, or a connector that
    /// cannot hold it ([`Config::can_hold`]), is refused.
    pub fn with_instances(&self, moved: &BTreeMap<String, String>) -> Result<Self> {
        let mut config = self.clone();
        if moved.is_empty() {
            return Ok(config);
        }
        let legacy = self.legacy_aliases();
        for (alias, connector) in moved {
            if !legacy.contains(&alias.as_str()) || !config.can_hold(connector, alias) {
                return Err(Failure::InvalidConfiguration);
            }
            config
                .connectors
                .get_or_insert_with(BTreeMap::new)
                .entry(connector.clone())
                .or_insert_with(|| Connector {
                    enabled: true,
                    instances: BTreeMap::new(),
                })
                .instances
                .insert(
                    alias.clone(),
                    Instance {
                        description: None,
                        endpoint: None,
                        alias: alias.clone(),
                        derived: false,
                        profile: None,
                        awaiting_profile: None,
                    },
                );
        }
        config.format = "connectors-local/5".into();
        Ok(config)
    }

    /// The configuration file's text: what [`Config::parse`] reads back.
    /// Comments of the file it was read from are not kept.
    pub fn to_text(&self) -> Result<String> {
        toml::to_string(self).map_err(|_| Failure::InvalidConfiguration)
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
        let config = Self::config(config)?;
        let state = match state {
            Some(path) => path.to_owned(),
            None => Self::default("XDG_STATE_HOME", ".local/state", "connectors")?,
        };
        fs::validate_path(&state)?;
        if state.file_name().is_none() {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(Self { config, state })
    }

    /// The configuration file's path as [`Paths::resolve`] resolves it,
    /// without a state directory (`connectors list` needs none).
    pub fn config(config: Option<&Path>) -> Result<PathBuf> {
        let config = match config {
            Some(path) => path.to_owned(),
            None => Self::default("XDG_CONFIG_HOME", ".config", "connectors/config.toml")?,
        };
        fs::validate_path(&config)?;
        if config.file_name().is_none() {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(config)
    }

    fn default(variable: &str, home_suffix: &str, leaf: &str) -> Result<PathBuf> {
        let base = if let Some(value) = std::env::var_os(variable).filter(|v| !v.is_empty()) {
            PathBuf::from(value)
        } else {
            PathBuf::from(std::env::var_os("HOME").ok_or(Failure::InvalidConfiguration)?)
                .join(home_suffix)
        };
        fs::validate_path(&base)?;
        Ok(base.join(leaf))
    }
}

fn selector(value: &str) -> bool {
    connectors_core::valid_id(value)
}

/// The environment variable a starting owner receives naming the directory of
/// the CLI that started it. The owner runs from a sealed copy of that CLI
/// (Linux: a memory file; macOS: a private launch copy), so its own
/// executable's directory holds no catalog provider.
pub const INSTALLATION: &str = "CONNECTORS_INSTALLATION";

/// What unpinned catalog instances are derived from: the installed catalog
/// provider, and how a `connectors-private/2` child of a derived entry reports
/// its bootstrap ([`handshake`] outside unit tests).
pub(crate) struct Installation {
    pub(crate) provider: PathBuf,
    pub(crate) handshake: fn(&Adapter) -> Result<super::runtime::Bootstrap>,
}

impl Installation {
    pub(crate) fn installed() -> Option<Self> {
        Some(Self {
            provider: installed_provider()?,
            handshake,
        })
    }

    /// What the provider derives for the instance `alias`, started with
    /// `args`: its entry or, when the instance names no profile (`stated`
    /// false), the provider refuses the selection and the connector declares
    /// more than one profile, those profiles. It is remembered for the life
    /// of this process per provider file and arguments, because an owner
    /// loads its configuration for every request: a provider file that changed
    /// in any way ([`file_identity`]) is derived again. A remembered digest
    /// that no longer matches the file is refused when a child starts, from
    /// its sealed snapshot.
    fn derive(&self, alias: &str, args: Vec<String>, stated: bool) -> Result<Derived> {
        type Key = (PathBuf, Vec<String>, Vec<u64>, usize);
        static DERIVED: std::sync::OnceLock<std::sync::Mutex<BTreeMap<Key, Derived>>> =
            std::sync::OnceLock::new();
        fs::validate_path(&self.provider)?;
        let selection = Executable {
            path: self.provider.clone(),
            sha256: String::new(),
            args,
        };
        let mut source = selection.source()?;
        let key = file_identity(&source).map(|identity| {
            let handshake = self.handshake as usize;
            (
                selection.path.clone(),
                selection.args.clone(),
                identity,
                handshake,
            )
        });
        let cache = DERIVED.get_or_init(Default::default);
        if let Some(key) = &key
            && let Some(derived) = cache.lock().ok().and_then(|cache| cache.get(key).cloned())
        {
            return Ok(derived);
        }
        let executable = Executable {
            sha256: digest(&mut source)?,
            ..selection
        };
        executable.validate()?;
        let derived = match executable.print("--print-local-bootstrap") {
            Ok(printed) => Derived::Entry(Box::new(self.entry(alias, executable, &printed)?.0)),
            // Only a refusal is asked about; a provider past its deadline is
            // not started again.
            Err(Unprinted::Refused) if !stated => {
                let printed = executable
                    .print("--print-profiles")
                    .map_err(|_| Failure::InvalidConfiguration)?;
                let profiles: BTreeSet<String> =
                    serde_json::from_slice(&printed).map_err(|_| Failure::InvalidConfiguration)?;
                if !(2..=64).contains(&profiles.len()) || !profiles.iter().all(|p| selector(p)) {
                    return Err(Failure::InvalidConfiguration);
                }
                Derived::Awaiting(profiles)
            }
            Err(_) => return Err(Failure::InvalidConfiguration),
        };
        if let (Some(key), Ok(mut cache)) = (key, cache.lock()) {
            if cache.len() >= 64 {
                cache.clear();
            }
            cache.insert(key, derived.clone());
        }
        Ok(derived)
    }

    /// The entry of `executable` from the bootstrap it `printed`: the
    /// instance id and revision it printed, the profiles and every operation,
    /// writes included, of the bootstrap a `connectors-private/2` child of
    /// that entry reports, which must name the same instance and revision.
    fn entry(
        &self,
        alias: &str,
        executable: Executable,
        printed: &[u8],
    ) -> Result<(Adapter, super::runtime::Bootstrap)> {
        use super::runtime::PrivateProtocol;
        let printed: super::runtime::Bootstrap =
            serde_json::from_slice(printed).map_err(|_| Failure::InvalidConfiguration)?;
        if printed.validate().is_err()
            || printed.instance != alias
            || printed.adapter != "catalog"
            || printed.protocol != connectors_core::WIRE_VERSION
        {
            return Err(Failure::InvalidConfiguration);
        }
        let mut adapter = Adapter {
            credential_revision: printed.credential_revision,
            instance_id: printed.instance,
            adapter_id: printed.adapter,
            configuration_revision: printed.configuration_revision,
            protocol: printed.protocol,
            private_protocol: Some(PrivateProtocol::V2),
            startup: Startup::default(),
            restart: Restart::default(),
            executable,
            permissions: Permissions::default(),
        };
        // The printed bootstrap is the `connectors-private/1` one, reads only.
        let reported = (self.handshake)(&adapter)?;
        if reported.validate_for(PrivateProtocol::V2).is_err()
            || reported.instance != adapter.instance_id
            || reported.adapter != adapter.adapter_id
            || reported.protocol != adapter.protocol
            || reported.configuration_revision != adapter.configuration_revision
            || reported.credential_revision != adapter.credential_revision
        {
            return Err(Failure::InvalidConfiguration);
        }
        adapter.permissions = Permissions {
            profiles: reported.profiles.iter().map(|p| p.id.clone()).collect(),
            operations: reported
                .requirements
                .iter()
                .map(|r| r.operation.clone())
                .collect(),
            custody: Default::default(),
        };
        Ok((adapter, reported))
    }

    /// The entry of the installed provider started with `args`, under
    /// `alias`, pinned as a derived entry is ([`Installation::entry`]), and the
    /// bootstrap its `connectors-private/2` child reported. Nothing is
    /// remembered: an invoked definition's entry is derived once per command
    /// (`definitions`).
    pub(crate) fn pinned(
        &self,
        alias: &str,
        args: Vec<String>,
    ) -> Result<(Adapter, super::runtime::Bootstrap)> {
        fs::validate_path(&self.provider)?;
        let selection = Executable {
            path: self.provider.clone(),
            sha256: String::new(),
            args,
        };
        let mut source = selection.source()?;
        let executable = Executable {
            sha256: digest(&mut source)?,
            ..selection
        };
        executable.validate()?;
        let printed = executable
            .print("--print-local-bootstrap")
            .map_err(|_| Failure::InvalidConfiguration)?;
        self.entry(alias, executable, &printed)
    }
}

/// What the installed provider derives for an unpinned instance.
#[derive(Clone)]
enum Derived {
    /// The instance's entry.
    Entry(Box<Adapter>),
    /// The profiles its connector declares, more than one, of which the
    /// instance names none: nothing is derived until it does.
    Awaiting(BTreeSet<String>),
}

/// Why a provider printed nothing usable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unprinted {
    /// It exited unsuccessfully: it refused the selection.
    Refused,
    /// It did not start, printed more than the bound, or outlived the
    /// deadline, it or a descendant holding its output.
    Failed,
}

/// Kills a printing provider's process group (on Unix; on Windows the
/// provider itself) and reaps the provider.
fn stop(child: &mut std::process::Child) {
    #[cfg(unix)]
    if let Ok(group) = i32::try_from(child.id()) {
        // SAFETY: a negative pid names the process group the provider was
        // started as the leader of; kill takes no pointers.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// What identifies an admitted file's content without reading it: any write
/// moves its ctime. `None` remembers nothing.
#[cfg(unix)]
fn file_identity(file: &std::fs::File) -> Option<Vec<u64>> {
    let stat = file.metadata().ok()?;
    let time = |seconds: i64, nanoseconds: i64| {
        Some((
            u64::try_from(seconds).ok()?,
            u64::try_from(nanoseconds).ok()?,
        ))
    };
    let (mtime, mtime_nsec) = time(stat.mtime(), stat.mtime_nsec())?;
    let (ctime, ctime_nsec) = time(stat.ctime(), stat.ctime_nsec())?;
    Some(vec![
        stat.dev(),
        stat.ino(),
        stat.size(),
        mtime,
        mtime_nsec,
        ctime,
        ctime_nsec,
    ])
}

/// Without an inode, a file's length and modification time ([`portable_identity`]).
#[cfg(windows)]
fn file_identity(file: &std::fs::File) -> Option<Vec<u64>> {
    let metadata = file.metadata().ok()?;
    portable_identity(metadata.len(), metadata.modified().ok()?)
}

/// A file's length and modification time to the nanosecond; `None` before
/// the Unix epoch.
#[cfg_attr(unix, allow(dead_code))]
fn portable_identity(length: u64, modified: std::time::SystemTime) -> Option<Vec<u64>> {
    let since = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(vec![
        length,
        since.as_secs(),
        u64::from(since.subsec_nanos()),
    ])
}

/// The bootstrap a child of `adapter` reports when it starts: the child is
/// started, admitted exactly as for an invocation, and stopped again.
fn handshake(adapter: &Adapter) -> Result<super::runtime::Bootstrap> {
    let child = super::runtime::Child::spawn(adapter).map_err(|_| Failure::InvalidConfiguration)?;
    Ok(child.bootstrap().clone())
}

/// The directory the installed catalog provider is in: `handed`, the value of
/// [`INSTALLATION`], when set, otherwise that of the `running` executable.
fn installation_from(
    handed: Option<std::ffi::OsString>,
    running: Option<PathBuf>,
) -> Option<PathBuf> {
    match handed {
        Some(directory) => Some(PathBuf::from(directory)),
        None => Some(running?.parent()?.to_owned()),
    }
}

/// The installation this process derives from. A CLI hands it to the owner
/// it starts, in [`INSTALLATION`], so both bind the same executable.
pub fn installation_directory() -> Option<PathBuf> {
    installation_from(std::env::var_os(INSTALLATION), std::env::current_exe().ok())
}

/// The catalog provider in [`installation_directory`]. What it is, is decided
/// when it is admitted and digested, not here.
pub fn installed_provider() -> Option<PathBuf> {
    Some(installation_directory()?.join(format!(
        "connectors-catalog-provider{}",
        std::env::consts::EXE_SUFFIX
    )))
}

impl Config {
    /// The profiles the connector of the listed instance `<connector>/<instance>`
    /// declares when the instance names none and there are several: it has
    /// no entry, and every command that needs it is refused naming them.
    pub fn awaiting_profile(&self, connector: &str, instance: &str) -> Option<&BTreeSet<String>> {
        self.connectors
            .as_ref()?
            .get(connector)?
            .instances
            .get(instance)?
            .awaiting_profile
            .as_ref()
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        Self::read(path).map_err(|refusal| refusal.failure())
    }

    /// Load like [`Config::load`], keeping which rule refused the file where
    /// that can be named by operator-written coordinates alone.
    pub fn read(path: &Path) -> std::result::Result<Self, Refusal> {
        Self::read_with(path, Installation::installed().as_ref())
    }

    /// Read like [`Config::read`], deriving every unpinned catalog instance
    /// from `installation`; `None` derives none.
    pub(crate) fn read_with(
        path: &Path,
        installation: Option<&Installation>,
    ) -> std::result::Result<Self, Refusal> {
        Self::parse_with(&Self::text_at(path)?, Derivation::From(installation))
    }

    /// Read like [`Config::read`], with the text the configuration was read
    /// from: `setup migrate` keeps exactly that text as its backup, and
    /// replaces the file only while it still holds it.
    pub fn read_source(path: &Path) -> std::result::Result<(Self, String), Refusal> {
        let text = Self::text_at(path)?;
        let config = Self::parse_with(&text, Derivation::From(Installation::installed().as_ref()))?;
        Ok((config, text))
    }

    /// The instance names of each connector the configuration at `path` lists
    /// under `[connectors]`, by connector (`connectors list`). The file is
    /// read and checked exactly as [`Config::read`] reads it, every refusal
    /// included, except that an unpinned instance is not derived: it is
    /// checked as a derivation checks it before the catalog provider starts
    /// and listed without an entry, so no process is started. What only the
    /// provider answers (whether it ships the connector, which profiles it
    /// declares, and so whether the instance's entry exists, counts toward the
    /// entry limit and holds what a consumer, `default_adapter` or custody
    /// target expects of it) is not checked, and is accepted: this never
    /// refuses a file [`Config::read`] accepts. Legacy `[adapters.<alias>]`
    /// entries are not listed.
    pub fn listed_instances(
        path: &Path,
    ) -> std::result::Result<BTreeMap<String, Vec<String>>, Refusal> {
        let config = Self::parse_with(&Self::text_at(path)?, Derivation::Deferred)?;
        Ok(config
            .connectors
            .unwrap_or_default()
            .into_iter()
            .map(|(connector, table)| (connector, table.instances.into_keys().collect()))
            .collect())
    }

    /// Decode, assemble and check configuration `text` exactly as
    /// [`Config::read`] does a file's, its [`CONFIG_LIMIT`] included.
    pub fn parse(text: &str) -> std::result::Result<Self, Refusal> {
        if text.len() > CONFIG_LIMIT {
            return Err(Refusal::Rule(Rule::ConfigTooLarge));
        }
        Self::parse_with(text, Derivation::From(Installation::installed().as_ref()))
    }

    fn parse_with(text: &str, installation: Derivation<'_>) -> std::result::Result<Self, Refusal> {
        // Decode the closed envelope without its embedded validation, so the
        // refusal below can name its rule instead of collapsing into serde's.
        let input: ConfigInput = toml::from_str(text).map_err(|_| Failure::InvalidConfiguration)?;
        let config = Self::assemble(input, installation)?;
        config.check()?;
        Ok(config)
    }

    /// The admitted configuration file's text.
    fn text_at(path: &Path) -> std::result::Result<String, Refusal> {
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
        let bytes = fs::read_bounded(file, CONFIG_LIMIT)?;
        Ok(String::from_utf8(bytes).map_err(|_| Failure::InvalidConfiguration)?)
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

    /// `[custody]` is a `connectors-local/4` and `/5` table. Its binding is one this
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
        if !matches!(
            self.format.as_str(),
            "connectors-local/4" | "connectors-local/5"
        ) || !supported
        {
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
                | "connectors-local/5"
        ) {
            return Err(Failure::InvalidConfiguration.into());
        }
        if self.owner_uid != fs::uid() {
            return Err(Refusal::Rule(Rule::OwnerUidMismatch));
        }
        if let Some(custody) = &self.custody {
            self.check_custody(custody)?;
        }
        self.check_connectors()?;
        if self.adapters.len() > 64
            || self.consumers.len() > 64
            || (!matches!(
                self.format.as_str(),
                "connectors-local/3" | "connectors-local/4" | "connectors-local/5"
            ) && !self.consumers.is_empty())
            || self
                .default_adapter
                .as_ref()
                .is_some_and(|alias| !self.names_entry(alias))
        {
            return Err(Failure::InvalidConfiguration.into());
        }
        self.check_selectors()?;
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
                // A deferred target's profiles and custody are the provider's.
                let deferred = held.is_none() && self.names_entry(&target.adapter);
                if self.format == "connectors-local/1"
                    || !entry.permissions.operations.contains(operation)
                    || &target.adapter == alias
                    || !selector(&target.profile)
                    || (!deferred
                        && held.is_none_or(|held| {
                            !held.permissions.profiles.contains(&target.profile)
                                || !held.permissions.custody.is_empty()
                        }))
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
                    .any(|alias| !self.names_entry(alias))
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
            default_instance: None,
            adapters: BTreeMap::new(),
            consumers: BTreeMap::new(),
            connectors: None,
            definitions: BTreeMap::new(),
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

/// The lowercase SHA-256 of an admitted executable read to its end, at most
/// 512 MiB.
fn digest(file: &mut std::fs::File) -> Result<String> {
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
    Ok(hex::encode(digest.finalize()))
}

impl Executable {
    /// The pinned selection's own form: a lowercase SHA-256 digest, a bounded
    /// argv without NUL and an admissible absolute path.
    pub(crate) fn validate(&self) -> Result<()> {
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
        if digest(&mut file)? != self.sha256 {
            return Err(Failure::InvalidConfiguration);
        }
        Ok(file)
    }

    /// What the selected catalog provider prints for its arguments and `flag`
    /// (`--print-local-bootstrap`, `--print-profiles`): no private channel and
    /// no credential, an empty environment, at most 4 MiB and ten seconds.
    /// The provider runs in its own process group, which is killed when it
    /// ends or at the deadline, and its output is not waited for past the
    /// deadline, even while a descendant still holds it open. A printed
    /// revision is checked again when the child starts, against the bootstrap
    /// the child itself reports.
    fn print(&self, flag: &str) -> std::result::Result<Vec<u8>, Unprinted> {
        use std::process::{Command, Stdio};
        use std::time::{Duration, Instant};
        const LIMIT: u64 = 4 * 1024 * 1024;
        let mut command = Command::new(&self.path);
        command
            .args(&self.args)
            .arg(flag)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);
        #[cfg(windows)]
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let mut child = command.spawn().map_err(|_| Unprinted::Failed)?;
        let Some(stdout) = child.stdout.take() else {
            stop(&mut child);
            return Err(Unprinted::Failed);
        };
        let (sender, received) = std::sync::mpsc::channel();
        // Detached: a descendant holding the output keeps only this thread,
        // until the group is killed and the pipe closes.
        std::thread::spawn(move || {
            let mut printed = Vec::new();
            let read = stdout.take(LIMIT + 1).read_to_end(&mut printed);
            let _ = sender.send(read.map(|_| printed));
        });
        let until = Instant::now() + Duration::from_secs(10);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(5)),
                _ => break None,
            }
        };
        // The group goes when the provider ends, so a descendant holding the
        // output closes it now rather than at the deadline; what the provider
        // wrote before it ended stays in the pipe for the reader.
        stop(&mut child);
        let printed = status.and_then(|_| {
            received
                .recv_timeout(until.saturating_duration_since(Instant::now()))
                .ok()
        });
        match (status, printed) {
            (Some(status), _) if !status.success() => Err(Unprinted::Refused),
            (Some(_), Some(Ok(printed))) if printed.len() as u64 <= LIMIT => Ok(printed),
            _ => Err(Unprinted::Failed),
        }
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
mod connector_instance_tests {
    //! story:connector-config-dual-read: `connectors-local/5` reads
    //! `[connectors.<connector>.instances.<instance>]` beside the legacy
    //! `[adapters.<alias>]`. An instance carrying a legacy entry's pins loads to
    //! that entry's `Adapter`, under its alias, so the persisted records keyed
    //! by its instance id, configuration revision, executable selection digest
    //! and alias stay valid.
    use super::*;

    /// One adapter kind's legacy entry, and the connector and instance names
    /// the same pins are written under.
    struct Kind {
        alias: &'static str,
        connector: &'static str,
        instance: &'static str,
        scalars: &'static str,
        executable: &'static str,
        permissions: &'static str,
    }

    const KINDS: [Kind; 5] = [
        Kind {
            alias: "gmail-read",
            connector: "gmail",
            instance: "private",
            scalars: "instance_id='gmail-read'\nadapter_id='catalog'\nconfiguration_revision='3f1c0a9e5b7d4e2a8c6b0f1e3d5a7c9b2e4f6a8c0d1b3e5f7a9c2d4e6f8a0b1c'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n",
            executable: "path='/opt/connectors/bin/connectors-catalog'\nsha256='1111111111111111111111111111111111111111111111111111111111111111'\nargs=['--local-config','/etc/connectors/gmail-read.json']\n",
            permissions: "profiles=['google.oauth']\noperations=['users.messages.list','users.messages.get']\n",
        },
        Kind {
            alias: "slack-bot",
            connector: "slack",
            instance: "bot",
            scalars: "instance_id='slack-bot'\nadapter_id='slack-credentials'\nconfiguration_revision='cfg-7'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\nstartup='automatic'\nrestart='never'\n",
            executable: "path='/opt/connectors/bin/slack-credentials'\nsha256='2222222222222222222222222222222222222222222222222222222222222222'\nargs=[]\n",
            permissions: "profiles=['slack.bot']\noperations=['auth.test']\n",
        },
        Kind {
            alias: "docs-mcp",
            connector: "docs",
            instance: "local",
            scalars: "instance_id='docs-mcp'\nadapter_id='mcp'\nconfiguration_revision='mcp-cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n",
            executable: "path='/opt/connectors/bin/connectors-mcp'\nsha256='3333333333333333333333333333333333333333333333333333333333333333'\nargs=['--server','/opt/mcp/docs-server']\n",
            permissions: "profiles=['mcp.none']\noperations=['tools.call']\n",
        },
        Kind {
            alias: "warehouse",
            connector: "postgres",
            instance: "warehouse",
            scalars: "instance_id='postgres-local'\nadapter_id='sql'\nconfiguration_revision='sql-cfg-2'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\nstartup='on-demand'\n",
            executable: "path='/opt/connectors/bin/connectors-sql'\nsha256='4444444444444444444444444444444444444444444444444444444444444444'\nargs=['--local-config','/etc/connectors/postgres.json']\n",
            permissions: "profiles=['postgres.password']\noperations=['query.read']\n",
        },
        Kind {
            alias: "cluster",
            connector: "kubernetes",
            instance: "work",
            scalars: "instance_id='kubernetes-local'\nadapter_id='kubernetes'\nconfiguration_revision='kube-cfg-3'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\nstartup='on-demand'\nrestart='never'\n",
            executable: "path='/opt/connectors/bin/connectors-kubernetes'\nsha256='5555555555555555555555555555555555555555555555555555555555555555'\nargs=['--local-config','/etc/connectors/kubernetes.json']\n",
            permissions: "profiles=['kubernetes.token']\noperations=['resources.list','endpoints.discover']\n",
        },
    ];

    impl Kind {
        fn legacy(&self) -> String {
            Self::table(&format!("adapters.{}", self.alias), self.scalars, self)
        }

        /// The same pins as an instance entry, with the legacy alias.
        fn instance(&self, extra: &str) -> String {
            format!(
                "[connectors.{}]\nenabled=true\n{}",
                self.connector,
                Self::table(
                    &format!("connectors.{}.instances.{}", self.connector, self.instance),
                    &format!("alias='{}'\n{extra}{}", self.alias, self.scalars),
                    self,
                )
            )
        }

        fn table(table: &str, scalars: &str, kind: &Kind) -> String {
            format!(
                "[{table}]\n{scalars}[{table}.executable]\n{}[{table}.permissions]\n{}",
                kind.executable, kind.permissions
            )
        }
    }

    fn read(format: &str, body: &str) -> std::result::Result<Config, Refusal> {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        std::fs::write(
            &paths.config,
            format!("format='{format}'\nowner_uid={}\n{body}", fs::uid()),
        )
        .unwrap();
        Config::read(&paths.config)
    }

    fn value(adapter: &Adapter) -> serde_json::Value {
        serde_json::to_value(adapter).unwrap()
    }

    const INVALID: Option<Refusal> = Some(Refusal::Failure(Failure::InvalidConfiguration));

    #[test]
    fn a_pinned_instance_reads_as_the_legacy_entry_for_every_adapter_kind() {
        for kind in &KINDS {
            let legacy = read("connectors-local/4", &kind.legacy()).unwrap();
            let legacy = &legacy.adapters[kind.alias];
            let instance = read(
                "connectors-local/5",
                &kind.instance("description='a description'\n"),
            )
            .unwrap();
            assert_eq!(
                instance.adapters.keys().collect::<Vec<_>>(),
                [kind.alias],
                "{}",
                kind.alias
            );
            let instance = &instance.adapters[kind.alias];
            assert_eq!(instance.instance_id, legacy.instance_id, "{}", kind.alias);
            assert_eq!(
                instance.configuration_revision, legacy.configuration_revision,
                "{}",
                kind.alias
            );
            // The approval policy's `executable_selection`.
            assert_eq!(instance.selection(), legacy.selection(), "{}", kind.alias);
            assert_eq!(value(instance), value(legacy), "{}", kind.alias);
            // The legacy entry still loads in the new format, unchanged.
            let current = read("connectors-local/5", &kind.legacy()).unwrap();
            assert_eq!(
                current.adapters[kind.alias].selection(),
                legacy.selection(),
                "{}",
                kind.alias
            );
        }
    }

    #[test]
    fn both_tables_load_together_and_every_alias_reaches_its_entry() {
        let separate: BTreeMap<&str, serde_json::Value> = KINDS
            .iter()
            .map(|kind| {
                let config = read("connectors-local/4", &kind.legacy()).unwrap();
                (kind.alias, value(&config.adapters[kind.alias]))
            })
            .collect();
        // Two kinds stay legacy entries, three move to instances; an instance
        // with an endpoint and one of a connector named by no declaration
        // (`docs`) load all the same.
        let body = format!(
            "default_adapter='cluster'\n{}{}{}{}{}[consumers.probe.executable]\npath='/opt/consumer'\nsha256='{}'\n[consumers.probe.permissions]\nconnections=['gmail-read','slack-bot']\n",
            KINDS[0].legacy(),
            KINDS[3].legacy(),
            KINDS[1].instance(""),
            KINDS[2].instance(""),
            KINDS[4].instance("endpoint='https://cluster.example.com:6443'\n"),
            "6".repeat(64),
        );
        let config = read("connectors-local/5", &body).unwrap();
        let loaded: BTreeMap<&str, serde_json::Value> = config
            .adapters
            .iter()
            .map(|(alias, adapter)| (alias.as_str(), value(adapter)))
            .collect();
        assert_eq!(loaded, separate);
        // Serialising keeps each entry in the table it was read from.
        let text = toml::to_string(&config).unwrap();
        assert!(
            text.contains("[adapters.gmail-read]") && !text.contains("[adapters.slack-bot]"),
            "{text}"
        );
        let roundtrip: Config = toml::from_str(&text).unwrap();
        let reloaded: BTreeMap<&str, serde_json::Value> = roundtrip
            .adapters
            .iter()
            .map(|(alias, adapter)| (alias.as_str(), value(adapter)))
            .collect();
        assert_eq!(reloaded, separate, "{text}");
        assert_eq!(toml::to_string(&roundtrip).unwrap(), text);
    }

    #[test]
    fn an_instance_and_an_entry_with_one_instance_id_or_alias_are_refused() {
        let [catalog, credential, ..] = &KINDS;
        // The same entry twice, once in each table.
        let same = format!("{}{}", catalog.legacy(), catalog.instance(""));
        assert_eq!(read("connectors-local/5", &same).err(), INVALID);
        // One instance id under two aliases, by the rule that refuses it for
        // two legacy entries.
        let id = format!(
            "{}{}",
            catalog.legacy(),
            credential
                .instance("")
                .replace("instance_id='slack-bot'", "instance_id='gmail-read'")
        );
        assert_eq!(read("connectors-local/5", &id).err(), INVALID);
        let legacy_pair = format!(
            "{}{}",
            catalog.legacy(),
            credential
                .legacy()
                .replace("instance_id='slack-bot'", "instance_id='gmail-read'")
        );
        assert_eq!(read("connectors-local/5", &legacy_pair).err(), INVALID);
        // One alias for two instance ids.
        let alias = format!(
            "{}{}",
            catalog.legacy(),
            credential
                .instance("")
                .replace("alias='slack-bot'", "alias='gmail-read'")
        );
        assert_eq!(read("connectors-local/5", &alias).err(), INVALID);
        // Two instances naming one alias.
        let twice = format!(
            "{}{}",
            catalog.instance(""),
            credential
                .instance("")
                .replace("alias='slack-bot'", "alias='gmail-read'")
        );
        assert_eq!(read("connectors-local/5", &twice).err(), INVALID);
    }

    #[test]
    fn connector_tables_are_refused_outside_format_five_and_when_malformed() {
        let kind = &KINDS[4];
        let valid = kind.instance("");
        assert!(read("connectors-local/5", &valid).is_ok());
        for format in [
            "connectors-local/1",
            "connectors-local/2",
            "connectors-local/3",
            "connectors-local/4",
        ] {
            assert_eq!(read(format, &valid).err(), INVALID, "{format}");
        }
        let table = "[connectors.kubernetes.instances.work]\n";
        let mut malformed = vec![
            // Nothing derives this instance: no catalog provider is installed
            // beside the test binary, and none ships Kubernetes.
            "[connectors.kubernetes]\nenabled=true\n[connectors.kubernetes.instances.work]\ndescription='unpinned'\n".to_owned(),
            valid.replace("enabled=true", "enabled=false"),
            valid.replace("enabled=true\n", ""),
            valid.replace("connectors.kubernetes", "connectors.'not a selector'"),
            valid.replace("instances.work", "instances.'not a selector'"),
            valid.replace(table, &format!("{table}provider='kubernetes'\n")),
            valid.replace(table, &format!("{table}endpoint=''\n")),
            valid.replace(table, &format!("{table}endpoint='https://a b'\n")),
            valid.replace(table, &format!("{table}description=\"a\\u0007\"\n")),
            valid.replace(table, &format!("{table}description='{}'\n", "d".repeat(1025))),
            valid.replace("enabled=true", "enabled=true\nendpoint='https://x'"),
            // Legacy rules over the instance's pins.
            valid.replace("alias='cluster'", "alias='invoke'"),
            valid.replace("protocol='v1alpha1'", "protocol='v2'"),
            valid.replace("sha256='5555", "sha256='EEEE"),
        ];
        for pin in [
            "alias='cluster'\n",
            "instance_id='kubernetes-local'\n",
            "adapter_id='kubernetes'\n",
            "configuration_revision='kube-cfg-3'\n",
            "protocol='v1alpha1'\n",
        ] {
            malformed.push(valid.replace(pin, ""));
        }
        malformed.push(
            valid.replace(
                "[connectors.kubernetes.instances.work.executable]\npath='/opt/connectors/bin/connectors-kubernetes'\nsha256='5555555555555555555555555555555555555555555555555555555555555555'\nargs=['--local-config','/etc/connectors/kubernetes.json']\n",
                "",
            ),
        );
        for body in malformed {
            assert_eq!(read("connectors-local/5", &body).err(), INVALID, "{body}");
        }
        // A disabled connector without instances, and `/5` without any
        // connector, load.
        assert!(read("connectors-local/5", "[connectors.gmail]\nenabled=false\n").is_ok());
        assert!(read("connectors-local/5", "").is_ok());
    }

    #[test]
    fn format_five_keeps_the_private_protocol_rule_and_admits_custody() {
        let kind = &KINDS[4];
        let bare = kind
            .instance("")
            .replace("private_protocol='connectors-private/1'\n", "");
        assert_eq!(
            read("connectors-local/5", &bare).err(),
            Some(Refusal::PrivateProtocolMismatch {
                format: "connectors-local/5".into(),
                instance_id: "kubernetes-local".into(),
            })
        );
        #[cfg(target_os = "linux")]
        assert!(
            read(
                "connectors-local/5",
                &format!(
                    "[custody]\nbinding='mounted_secret'\ndirectory='/srv/custody'\n{}",
                    kind.instance("")
                )
            )
            .unwrap()
            .selection()
            .read_only()
        );
    }
}

// The fixtures write a POSIX shell provider.
#[cfg(all(test, unix))]
mod derived_instance_tests {
    //! story:connector-instance-derived-fields: an instance of a catalog
    //! connector that names nothing but itself loads to an entry whose
    //! executable, arguments, revision and permissions the host derives from
    //! the installed catalog provider and the bootstrap it prints.
    use super::*;
    use crate::local::runtime::{
        Bootstrap, Effect, EntryField, PrivateProtocol, Profile, Requirement,
    };
    use std::os::unix::fs::PermissionsExt as _;

    const INVALID: Option<Refusal> = Some(Refusal::Failure(Failure::InvalidConfiguration));
    const GMAIL: &str = "[connectors.gmail]\nenabled=true\n[connectors.gmail.instances.private]\n";

    /// The bootstrap of `instance`: the printed one has the read only, the one
    /// a `connectors-private/2` child reports (`writes`) has the write too.
    fn bootstrap(instance: &str, writes: bool) -> Bootstrap {
        let ids: &[&str] = if writes {
            &["users.messages.list", "users.drafts.create"]
        } else {
            &["users.messages.list"]
        };
        let operation = |id: &str| {
            serde_json::json!({"id": id, "description": "Fictional operation",
                "contract": "fixture/1", "profile": "google.oauth",
                "input_schema": {"type": "object"}, "output_schema": {"type": "object"}})
        };
        Bootstrap {
            bindings: None,
            credential_revision: None,
            executable: None,
            instance: instance.into(),
            adapter: "catalog".into(),
            protocol: "v1alpha1".into(),
            configuration_revision: "derived-revision-1".into(),
            provider_authority: "fixture-authority".into(),
            descriptor: serde_json::json!({
                "version": "v1alpha1", "instance": instance, "adapter": "catalog",
                "revision": "fixture-descriptor", "configuration_schema": {"type": "object"},
                "operations": ids.iter().map(|id| operation(id)).collect::<Vec<_>>(),
            })
            .to_string(),
            profiles: vec![Profile {
                id: "google.oauth".into(),
                revision: "fixture-profile".into(),
                purpose: crate::local::registry::Purpose::DelegatedUser,
                subject: crate::local::registry::Subject::User,
                scheme: "http_bearer".into(),
                capability: "http-bearer".into(),
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                fields: vec![EntryField {
                    name: "token".into(),
                    label: "Fictional credential".into(),
                    max_bytes: 100,
                }],
                acquisition: None,
            }],
            requirements: ids
                .iter()
                .map(|id| Requirement {
                    operation: (*id).into(),
                    custody: false,
                    profile: "google.oauth".into(),
                    scopes: BTreeSet::new(),
                    effect: if id.starts_with("users.drafts.") {
                        Effect::Write
                    } else {
                        Effect::Read
                    },
                })
                .collect(),
        }
    }

    /// What a `connectors-private/2` child of `adapter` reports: the write
    /// besides the read, under the printed instance and revision.
    fn reported(adapter: &Adapter) -> Result<Bootstrap> {
        let mut bootstrap = bootstrap(&adapter.instance_id, true);
        bootstrap.configuration_revision = adapter.configuration_revision.clone();
        Ok(bootstrap)
    }

    /// A configuration directory and a stand-in catalog provider that records
    /// its arguments and prints `printed` (or exits 3 when `None`, as the real
    /// provider does for a connector it does not ship). A shell script cannot
    /// be started from a sealed snapshot, so its child's handshake is
    /// `handshake`.
    struct Fixture {
        root: tempfile::TempDir,
        provider: PathBuf,
        handshake: fn(&Adapter) -> Result<Bootstrap>,
    }

    impl Fixture {
        fn new(printed: Option<&str>) -> Self {
            let root = tempfile::tempdir().unwrap();
            let bin = root.path().join("bin");
            std::fs::create_dir(&bin).unwrap();
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
            let provider = bin.join("connectors-catalog-provider");
            let printed_path = root.path().join("bootstrap.json");
            let body = match printed {
                Some(text) => {
                    std::fs::write(&printed_path, text).unwrap();
                    format!("cat '{}'", printed_path.display())
                }
                None => "exit 3".into(),
            };
            std::fs::write(
                &provider,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\n{body}\n",
                    root.path().join("args").display()
                ),
            )
            .unwrap();
            std::fs::set_permissions(&provider, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self {
                root,
                provider,
                handshake: reported,
            }
        }

        fn gmail(instance: &str) -> Self {
            Self::new(Some(
                &serde_json::to_string(&bootstrap(instance, false)).unwrap(),
            ))
        }

        /// The provider, after it records its arguments, runs `body`, in
        /// which `BOOTSTRAP` is the path of the bootstrap it prints.
        fn behave(&self, body: &str) {
            std::fs::write(
                &self.provider,
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\n{}\n",
                    self.root.path().join("args").display(),
                    body.replace(
                        "BOOTSTRAP",
                        &self
                            .root
                            .path()
                            .join("bootstrap.json")
                            .display()
                            .to_string()
                    )
                ),
            )
            .unwrap();
        }

        /// A provider of a connector declaring two profiles: it prints a
        /// bootstrap only for `--profile`, its declared profiles for
        /// `--print-profiles`, and otherwise refuses, as the catalog provider
        /// does.
        fn zendesk() -> Self {
            let fixture = Self::new(Some(
                &serde_json::to_string(&bootstrap("zendesk.support", false)).unwrap(),
            ));
            fixture.behave(
                "case \" $* \" in\n*' --print-profiles '*) echo '[\"zendesk.basic\",\"zendesk.oauth\"]';;\n*' --profile '*) cat 'BOOTSTRAP';;\n*) exit 1;;\nesac",
            );
            fixture
        }

        fn read(&self, body: &str) -> std::result::Result<Config, Refusal> {
            self.read_from(body, Some(&self.provider))
        }

        fn read_from(
            &self,
            body: &str,
            provider: Option<&Path>,
        ) -> std::result::Result<Config, Refusal> {
            let paths = Paths::resolve(
                Some(&self.root.path().join("config/config.toml")),
                Some(&self.root.path().join("state")),
            )
            .unwrap();
            if !paths.config.exists() {
                Config::initialize(&paths).unwrap();
            }
            std::fs::write(
                &paths.config,
                format!(
                    "format='connectors-local/5'\nowner_uid={}\n{body}",
                    fs::uid()
                ),
            )
            .unwrap();
            let installation = provider.map(|provider| Installation {
                provider: provider.to_owned(),
                handshake: self.handshake,
            });
            Config::read_with(&paths.config, installation.as_ref())
        }

        /// Every argument the provider was started with, one per line.
        fn args(&self) -> String {
            std::fs::read_to_string(self.root.path().join("args")).unwrap_or_default()
        }

        fn sha256(&self) -> String {
            hex::encode(Sha256::digest(std::fs::read(&self.provider).unwrap()))
        }
    }

    #[test]
    fn an_unpinned_gmail_instance_loads_to_an_entry_derived_from_the_installed_provider() {
        let fixture = Fixture::gmail("gmail.private");
        let config = fixture.read(GMAIL).unwrap();
        assert_eq!(
            config.adapters.keys().collect::<Vec<_>>(),
            ["gmail.private"]
        );
        let entry = &config.adapters["gmail.private"];
        assert_eq!(entry.instance_id, "gmail.private");
        assert_eq!(entry.adapter_id, "catalog");
        assert_eq!(entry.configuration_revision, "derived-revision-1");
        assert_eq!(entry.protocol, "v1alpha1");
        assert_eq!(entry.private_protocol, Some(PrivateProtocol::V2));
        assert!(matches!(entry.startup, Startup::OnDemand));
        assert!(matches!(entry.restart, Restart::Never));
        assert_eq!(
            entry.executable,
            Executable {
                path: fixture.provider.clone(),
                sha256: fixture.sha256(),
                args: ["--connector", "gmail", "--instance", "private"]
                    .map(str::to_owned)
                    .to_vec(),
            }
        );
        assert_eq!(
            entry.permissions.profiles,
            BTreeSet::from(["google.oauth".to_owned()])
        );
        assert_eq!(
            entry.permissions.operations,
            BTreeSet::from([
                "users.drafts.create".to_owned(),
                "users.messages.list".to_owned()
            ])
        );
        // The revision is the provider's own, read from its bootstrap.
        assert_eq!(
            fixture.args(),
            "--connector\ngmail\n--instance\nprivate\n--print-local-bootstrap\n"
        );
        let instance = &config.connectors.as_ref().unwrap()["gmail"].instances["private"];
        assert_eq!(instance.alias, "gmail.private");
        assert!(instance.derived);
    }

    /// An owner loads its configuration for every request: the provider is
    /// asked once per file and arguments, and again once the file changed.
    #[test]
    fn a_derivation_is_reused_until_the_provider_file_changes() {
        let fixture = Fixture::gmail("gmail.private");
        let first = fixture.read(GMAIL).unwrap().adapters["gmail.private"].clone();
        let again = fixture.read(GMAIL).unwrap().adapters["gmail.private"].clone();
        assert_eq!(
            serde_json::to_value(&again).unwrap(),
            serde_json::to_value(&first).unwrap()
        );
        assert_eq!(fixture.args().matches("--print-local-bootstrap").count(), 1);
        // Another endpoint is another selection.
        let body = GMAIL.replace(
            "instances.private]\n",
            "instances.private]\nendpoint='https://mail.example.test'\n",
        );
        fixture.read(&body).unwrap();
        assert_eq!(fixture.args().matches("--print-local-bootstrap").count(), 2);
        // The same path, rewritten: derived again, with the new digest.
        let script = std::fs::read_to_string(&fixture.provider).unwrap();
        std::fs::write(&fixture.provider, format!("{script}# rewritten\n")).unwrap();
        let rewritten = fixture.read(GMAIL).unwrap().adapters["gmail.private"].clone();
        assert_eq!(fixture.args().matches("--print-local-bootstrap").count(), 3);
        assert_eq!(rewritten.executable.sha256, fixture.sha256());
        assert_ne!(rewritten.executable.sha256, first.executable.sha256);
    }

    /// story:cli-connector-instance-selectors over a derived instance: it is
    /// selected by its connector and instance names, as `default_instance`,
    /// and its alias names them.
    #[test]
    fn a_derived_instance_is_selected_by_its_connector_and_instance() {
        let fixture = Fixture::gmail("gmail.private");
        let config = fixture.read(GMAIL).unwrap();
        let selected = config
            .select(Selector::Instance {
                connector: "gmail",
                instance: "private",
            })
            .unwrap();
        assert_eq!(
            (selected.alias, selected.connector, selected.instance),
            ("gmail.private", "gmail", "private")
        );
        assert_eq!(config.names("gmail.private"), Some(("gmail", "private")));
        let config = fixture
            .read(&format!("default_instance='gmail/private'\n{GMAIL}"))
            .unwrap();
        let selected = config.select(Selector::Default).unwrap();
        assert_eq!(selected.alias, "gmail.private");
        assert_eq!(
            selected.adapter.configuration_revision,
            "derived-revision-1"
        );
    }

    const ZENDESK: &str = "[connectors.zendesk]\nenabled=true\n[connectors.zendesk.instances.support]\nendpoint='https://example.zendesk.com'\n";

    /// An instance names one of its connector's profiles with `profile`; the
    /// provider derives the entry for that profile.
    #[test]
    fn a_stated_profile_is_passed_to_the_provider_and_written_back() {
        let fixture = Fixture::zendesk();
        let body = ZENDESK.replace("endpoint=", "profile='zendesk.basic'\nendpoint=");
        let config = fixture.read(&body).unwrap();
        assert_eq!(
            config.adapters["zendesk.support"].executable.args,
            [
                "--connector",
                "zendesk",
                "--instance",
                "support",
                "--endpoint",
                "https://example.zendesk.com",
                "--profile",
                "zendesk.basic"
            ]
        );
        assert_eq!(config.awaiting_profile("zendesk", "support"), None);
        let written: toml::Value = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(
            written["connectors"]["zendesk"]["instances"]["support"]["profile"].as_str(),
            Some("zendesk.basic")
        );
        // A profile is a selector.
        let invalid = body.replace("'zendesk.basic'", "'zendesk basic'");
        assert_eq!(fixture.read(&invalid).err(), INVALID);
    }

    /// The coordinator's rule for a connector declaring more than one profile
    /// and an instance that names none: the file loads, the instance is
    /// listed with its connector's profiles and has no entry, so nothing
    /// selects it, and every other entry loads as before.
    #[test]
    fn an_instance_of_a_two_profile_connector_naming_none_is_listed_without_an_entry() {
        let fixture = Fixture::zendesk();
        let legacy = "[adapters.tools]\ninstance_id='tools'\nadapter_id='mcp'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.tools.executable]\npath='/opt/connectors/bin/connectors-mcp'\nsha256='3333333333333333333333333333333333333333333333333333333333333333'\n";
        let config = fixture.read(&format!("{legacy}{ZENDESK}")).unwrap();
        assert_eq!(config.adapters.keys().collect::<Vec<_>>(), ["tools"]);
        assert_eq!(
            config.awaiting_profile("zendesk", "support"),
            Some(&BTreeSet::from([
                "zendesk.basic".to_owned(),
                "zendesk.oauth".to_owned()
            ]))
        );
        let instance = &config.connectors.as_ref().unwrap()["zendesk"].instances["support"];
        assert_eq!(instance.alias, "zendesk.support");
        assert!(
            config
                .select(Selector::Instance {
                    connector: "zendesk",
                    instance: "support"
                })
                .is_none()
        );
        assert!(
            fixture.args().contains("--print-profiles"),
            "{}",
            fixture.args()
        );
        // Written back as written: no pins and no profile.
        let written: toml::Value = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(
            written["connectors"]["zendesk"]["instances"]["support"],
            toml::Value::Table(toml::map::Map::from_iter([(
                "endpoint".to_owned(),
                toml::Value::String("https://example.zendesk.com".into())
            )]))
        );
        // A provider that refuses and declares fewer than two profiles still
        // refuses the file, as does a legacy entry under the listed alias.
        fixture.behave("case \" $* \" in\n*' --print-profiles '*) echo '[\"zendesk.basic\"]';;\n*) exit 1;;\nesac");
        assert_eq!(fixture.read(ZENDESK).err(), INVALID);
        let fixture = Fixture::zendesk();
        let taken = legacy.replace("adapters.tools", "adapters.'zendesk.support'");
        assert_eq!(fixture.read(&format!("{taken}{ZENDESK}")).err(), INVALID);
    }

    /// The provider is started in its own process group, which is killed when
    /// the provider ends, so a descendant holding the output open does not
    /// hold the load until the ten-second deadline, and nothing else is asked.
    #[test]
    fn printing_the_bootstrap_is_bounded_when_a_descendant_keeps_the_output() {
        let fixture = Fixture::gmail("gmail.private");
        fixture.behave("sleep 25 &\nexit 0");
        let started = std::time::Instant::now();
        assert_eq!(fixture.read(GMAIL).err(), INVALID);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        assert_eq!(fixture.args().matches("--connector").count(), 1);
    }

    /// Where no inode identifies a file (Windows), the cache key is its
    /// length and modification time, beside the path.
    #[test]
    fn the_portable_cache_key_moves_with_length_and_modification_time() {
        let at = |seconds| std::time::UNIX_EPOCH + std::time::Duration::from_nanos(seconds);
        let key = portable_identity(10, at(1_000_000_007));
        assert_eq!(key, portable_identity(10, at(1_000_000_007)));
        assert!(key.is_some());
        assert_ne!(key, portable_identity(11, at(1_000_000_007)));
        assert_ne!(key, portable_identity(10, at(1_000_000_008)));
        // Before the epoch nothing is remembered.
        assert_eq!(
            portable_identity(
                10,
                std::time::UNIX_EPOCH - std::time::Duration::from_secs(1)
            ),
            None
        );
    }

    /// The CLI and the owner it starts derive from one installation: the
    /// owner is handed the directory the CLI resolved.
    #[test]
    fn the_installation_is_the_handed_directory_else_the_executables() {
        let handed = Some(std::ffi::OsString::from("/opt/connectors/bin"));
        let running = Some(PathBuf::from("/usr/local/bin/connectors"));
        assert_eq!(
            installation_from(handed, running.clone()),
            Some(PathBuf::from("/opt/connectors/bin"))
        );
        assert_eq!(
            installation_from(None, running),
            Some(PathBuf::from("/usr/local/bin"))
        );
        assert_eq!(installation_from(None, None), None);
    }

    #[test]
    fn an_endpoint_is_passed_to_the_provider_and_digested_by_it() {
        let fixture = Fixture::gmail("gmail.private");
        let body = GMAIL.replace(
            "instances.private]\n",
            "instances.private]\nendpoint='https://mail.example.test'\ndescription='personal mailbox'\n",
        );
        let config = fixture.read(&body).unwrap();
        assert_eq!(
            config.adapters["gmail.private"].executable.args,
            [
                "--connector",
                "gmail",
                "--instance",
                "private",
                "--endpoint",
                "https://mail.example.test"
            ]
        );
    }

    #[test]
    fn an_unpinned_instance_is_refused_when_nothing_derives_it() {
        let fixture = Fixture::gmail("gmail.private");
        // No provider installed.
        assert_eq!(fixture.read_from(GMAIL, None).err(), INVALID);
        assert_eq!(
            fixture
                .read_from(GMAIL, Some(&fixture.root.path().join("bin/absent")))
                .err(),
            INVALID
        );
        // A connector the provider does not ship (Slack, Kubernetes, MCP, SQL)
        // keeps its pins: the provider refuses it, and so does the host.
        let unshipped = Fixture::new(None);
        let slack = "[connectors.slack]\nenabled=true\n[connectors.slack.instances.bot]\n";
        assert_eq!(unshipped.read(slack).err(), INVALID);
        // A bootstrap for another instance, or not a bootstrap at all.
        assert_eq!(Fixture::gmail("gmail.work").read(GMAIL).err(), INVALID);
        assert_eq!(Fixture::new(Some("{}")).read(GMAIL).err(), INVALID);
        // A child that does not start, or reports another revision or
        // instance than the provider printed.
        for handshake in [
            (|_| Err(Failure::InvalidConfiguration)) as fn(&Adapter) -> Result<Bootstrap>,
            |adapter| {
                let mut bootstrap = reported(adapter)?;
                bootstrap.configuration_revision = "another-revision".into();
                Ok(bootstrap)
            },
            |adapter| reported(adapter).map(|_| bootstrap("gmail.work", true)),
        ] {
            let mut refused = Fixture::gmail("gmail.private");
            refused.handshake = handshake;
            assert_eq!(refused.read(GMAIL).err(), INVALID);
        }
        // A provider whose content or mode the host does not admit.
        let writable = Fixture::gmail("gmail.private");
        std::fs::set_permissions(&writable.provider, std::fs::Permissions::from_mode(0o722))
            .unwrap();
        assert_eq!(writable.read(GMAIL).err(), INVALID);
        // The id `<connector>.<instance>` stays unambiguous: neither name
        // carries a `.`.
        for body in [
            GMAIL.replace("instances.private", "instances.'pri.vate'"),
            GMAIL.replace("connectors.gmail", "connectors.'gm.ail'"),
        ] {
            assert_eq!(fixture.read(&body).err(), INVALID, "{body}");
        }
        // The derived alias and instance id are taken by a legacy entry.
        let legacy = "[adapters.'gmail.private']\ninstance_id='other'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.'gmail.private'.executable]\npath='/opt/connectors/bin/connectors-catalog-provider'\nsha256='1111111111111111111111111111111111111111111111111111111111111111'\n";
        assert_eq!(fixture.read(&format!("{legacy}{GMAIL}")).err(), INVALID);
        let same_id = legacy
            .replace("'gmail.private'", "gmail-read")
            .replace("instance_id='other'", "instance_id='gmail.private'");
        assert_eq!(fixture.read(&format!("{same_id}{GMAIL}")).err(), INVALID);
        // Some pins but not all is neither pinned nor derived.
        let partial = GMAIL.replace(
            "instances.private]\n",
            "instances.private]\nadapter_id='catalog'\n",
        );
        assert_eq!(fixture.read(&partial).err(), INVALID);
    }

    #[test]
    fn a_derived_instance_is_written_back_without_pins() {
        let fixture = Fixture::gmail("gmail.private");
        let body = GMAIL.replace(
            "instances.private]\n",
            "instances.private]\ndescription='personal mailbox'\n",
        );
        let config = fixture.read(&body).unwrap();
        let text = toml::to_string(&config).unwrap();
        let written: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(
            written["connectors"]["gmail"]["instances"]["private"],
            toml::Value::Table(toml::map::Map::from_iter([(
                "description".to_owned(),
                toml::Value::String("personal mailbox".into())
            )])),
            "{text}"
        );
        assert!(written["adapters"].as_table().unwrap().is_empty(), "{text}");
    }
}

#[cfg(all(test, unix))]
mod instance_selector_tests {
    //! story:cli-connector-instance-selectors: `--connector` with `--instance`,
    //! `--adapter` and `default_instance` resolve to exactly one entry.
    use super::*;

    const PINS: &str = "configuration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n";
    const EXECUTABLE: &str = "path='/not-installed/adapter'\nsha256='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nargs=[]\n";

    /// An instance `<connector>/<instance>` carrying the pins of the legacy alias `alias`.
    fn instance(connector: &str, instance: &str, alias: &str) -> String {
        let table = format!("connectors.{connector}.instances.{instance}");
        format!(
            "[connectors.{connector}]\nenabled=true\n[{table}]\nalias='{alias}'\ninstance_id='{alias}-id'\nadapter_id='catalog'\n{PINS}[{table}.executable]\n{EXECUTABLE}"
        )
    }

    /// A legacy entry no instance names.
    fn legacy(alias: &str, adapter_id: &str) -> String {
        format!(
            "[adapters.{alias}]\ninstance_id='{alias}-id'\nadapter_id='{adapter_id}'\n{PINS}[adapters.{alias}.executable]\n{EXECUTABLE}"
        )
    }

    fn read(format: &str, body: &str) -> std::result::Result<Config, Refusal> {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        std::fs::write(
            &paths.config,
            format!("format='{format}'\nowner_uid={}\n{body}", fs::uid()),
        )
        .unwrap();
        Config::read(&paths.config)
    }

    fn alias(config: &Config, selector: Selector<'_>) -> Option<String> {
        config
            .select(selector)
            .map(|selected| selected.alias.to_owned())
    }

    #[test]
    fn both_selectors_name_one_entry_and_a_legacy_entry_by_its_adapter_id() {
        let config = read(
            "connectors-local/5",
            &format!(
                "{}{}",
                legacy("tools", "mcp"),
                instance("gmail", "private", "gmail-read")
            ),
        )
        .unwrap();
        let by_instance = |connector, instance| {
            alias(
                &config,
                Selector::Instance {
                    connector,
                    instance,
                },
            )
        };
        assert_eq!(
            by_instance("gmail", "private").as_deref(),
            Some("gmail-read")
        );
        assert_eq!(
            alias(&config, Selector::Alias("gmail-read")).as_deref(),
            Some("gmail-read")
        );
        assert_eq!(config.names("gmail-read"), Some(("gmail", "private")));
        assert_eq!(by_instance("mcp", "tools").as_deref(), Some("tools"));
        assert_eq!(config.names("tools"), Some(("mcp", "tools")));
        // An instance name is unique within its connector only.
        assert_eq!(by_instance("drive", "private"), None);
        assert_eq!(by_instance("gmail", "gmail-read"), None);
        assert_eq!(alias(&config, Selector::Alias("absent")), None);
        assert_eq!(config.names("absent"), None);
        // No selector and no default selects nothing.
        assert_eq!(alias(&config, Selector::Default), None);
    }

    #[test]
    fn default_instance_selects_and_counterparts_default_adapter() {
        let body = instance("gmail", "private", "gmail-read");
        let config = read(
            "connectors-local/5",
            &format!("default_instance='gmail/private'\n{body}"),
        )
        .unwrap();
        assert_eq!(config.default_instance.as_deref(), Some("gmail/private"));
        let selected = config.select(Selector::Default).unwrap();
        assert_eq!(
            (selected.alias, selected.connector, selected.instance),
            ("gmail-read", "gmail", "private")
        );
        let config = read(
            "connectors-local/5",
            &format!("default_adapter='gmail-read'\n{body}"),
        )
        .unwrap();
        assert_eq!(
            alias(&config, Selector::Default).as_deref(),
            Some("gmail-read")
        );
        // Written back as read.
        let written = toml::to_string(
            &read(
                "connectors-local/5",
                &format!("default_instance='gmail/private'\n{body}"),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(
            written.contains("default_instance = \"gmail/private\""),
            "{written}"
        );
    }

    #[test]
    fn a_default_instance_that_selects_no_single_entry_refuses_the_file() {
        let body = instance("gmail", "private", "gmail-read");
        let refused = Some(Refusal::Failure(Failure::InvalidConfiguration));
        for default in [
            "default_instance='gmail/absent'\n",
            "default_instance='gmail'\n",
            "default_instance='gmail/private/x'\n",
            "default_instance='/private'\n",
            "default_instance='gmail/private'\ndefault_adapter='gmail-read'\n",
        ] {
            assert_eq!(
                read("connectors-local/5", &format!("{default}{body}")).err(),
                refused,
                "{default}"
            );
        }
        // A `connectors-local/5` key: refused in an older format, where the
        // legacy entry would still be named.
        let legacy = legacy("tools", "mcp");
        assert!(read("connectors-local/4", &legacy).is_ok());
        assert_eq!(
            read(
                "connectors-local/4",
                &format!("default_instance='mcp/tools'\n{legacy}")
            )
            .err(),
            refused
        );
    }

    #[test]
    fn two_entries_answering_to_one_connector_and_instance_refuse_the_file() {
        // The legacy entry `private` of adapter_id `catalog` and the declared
        // instance `catalog/private` would answer to one pair.
        let refused = read(
            "connectors-local/5",
            &format!(
                "{}{}",
                legacy("private", "catalog"),
                instance("catalog", "private", "gmail-read")
            ),
        );
        assert_eq!(
            refused.err(),
            Some(Refusal::Failure(Failure::InvalidConfiguration))
        );
    }
}

// The fixtures configure Unix absolute executable paths and the mounted
// custody binding, which is Linux's.
#[cfg(all(test, target_os = "linux"))]
mod migration_tests {
    //! story:setup-migrate-adapter-entries: a legacy entry rewritten as a
    //! pinned instance reads back as exactly that entry, and nothing else of
    //! the file changes.
    use super::*;

    fn entry(alias: &str, adapter: &str, private_protocol: &str) -> String {
        format!(
            "[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='{adapter}'\nconfiguration_revision='cfg-{alias}'\nprotocol='v1alpha1'\n{private_protocol}startup='automatic'\n[adapters.{alias}.executable]\npath='/opt/connectors/bin/{adapter}'\nsha256='{}'\nargs=['--local-config','/etc/connectors/{alias}.json']\n[adapters.{alias}.permissions]\nprofiles=['p.token']\noperations=['item.read']\n",
            "1".repeat(64)
        )
    }

    /// Every top-level key a configuration may carry beside its entries.
    fn shared() -> String {
        format!(
            "default_adapter='mail'\n[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{k}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{k2}'\nversion='roughtime-0x8000000c'\n[custody]\nbinding='mounted_secret'\ndirectory='/run/connectors/custody'\n[consumers.probe]\npass_env=['EXAMPLE_']\n[consumers.probe.executable]\npath='/opt/connectors/bin/probe'\nsha256='{}'\nargs=['--x']\n[consumers.probe.permissions]\nconnections=['mail']\n",
            "2".repeat(64),
            k = base64_key(1),
            k2 = base64_key(2),
        )
    }

    fn base64_key(byte: u8) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode([byte; 32])
    }

    fn read(format: &str, body: &str) -> std::result::Result<(Config, String), Refusal> {
        let root = tempfile::tempdir().unwrap();
        let paths = Paths::resolve(
            Some(&root.path().join("config/config.toml")),
            Some(&root.path().join("state")),
        )
        .unwrap();
        Config::initialize(&paths).unwrap();
        std::fs::write(
            &paths.config,
            format!("format='{format}'\nowner_uid={}\n{body}", fs::uid()),
        )
        .unwrap();
        Config::read_source(&paths.config)
    }

    fn moved(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(alias, connector)| ((*alias).to_owned(), (*connector).to_owned()))
            .collect()
    }

    #[test]
    fn a_rewritten_entry_reads_back_as_the_same_entry_and_nothing_else_changes() {
        let body = format!(
            "{}{}{}",
            shared(),
            entry(
                "mail",
                "catalog",
                "private_protocol='connectors-private/2'\n"
            ),
            entry("tools", "mcp", "private_protocol='connectors-private/1'\n")
        );
        let (config, text) = read("connectors-local/4", &body).unwrap();
        assert!(text.contains("[adapters.mail]"));
        assert_eq!(config.legacy_aliases(), ["mail", "tools"]);
        let rewritten = config.with_instances(&moved(&[("mail", "gmail")])).unwrap();
        let written = rewritten.to_text().unwrap();
        assert!(
            written.contains("[connectors.gmail.instances.mail]"),
            "{written}"
        );
        assert!(!written.contains("[adapters.mail]"), "{written}");
        assert!(written.contains("[adapters.tools]"), "{written}");
        let after = Config::parse(&written).unwrap();
        assert_eq!(after.format, "connectors-local/5");
        assert_eq!(after.legacy_aliases(), ["tools"]);
        for alias in ["mail", "tools"] {
            assert_eq!(
                serde_json::to_value(&after.adapters[alias]).unwrap(),
                serde_json::to_value(&config.adapters[alias]).unwrap(),
                "{alias}"
            );
            assert_eq!(
                after.adapters[alias].selection(),
                config.adapters[alias].selection()
            );
        }
        assert_eq!(after.names("mail"), Some(("gmail", "mail")));
        assert_eq!(after.names("tools"), Some(("mcp", "tools")));
        let sha = |config: &Config| {
            config
                .approval_clock
                .as_ref()
                .unwrap()
                .configuration()
                .unwrap()
                .sha256()
        };
        assert_eq!(sha(&after), sha(&config));
        assert_eq!(after.custody, config.custody);
        assert_eq!(after.consumers, config.consumers);
        assert_eq!(after.default_adapter, config.default_adapter);
        assert_eq!(after.selection(), config.selection());
        // Nothing moved: the same configuration, format included.
        let same = config.with_instances(&BTreeMap::new()).unwrap();
        assert_eq!(same.format, "connectors-local/4");
    }

    #[test]
    fn a_connector_that_cannot_hold_the_instance_or_an_alias_that_is_no_legacy_entry_is_refused() {
        let pinned = "[connectors.gmail]\nenabled=true\n[connectors.gmail.instances.taken]\nalias='pinned'\ninstance_id='pinned-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-pinned'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[connectors.gmail.instances.taken.executable]\npath='/opt/connectors/bin/catalog'\nsha256='3333333333333333333333333333333333333333333333333333333333333333'\nargs=[]\n[connectors.drive]\nenabled=false\n";
        let body = format!(
            "{pinned}{}{}",
            entry(
                "taken",
                "catalog",
                "private_protocol='connectors-private/1'\n"
            ),
            entry(
                "files",
                "catalog",
                "private_protocol='connectors-private/1'\n"
            )
        );
        let (config, _) = read("connectors-local/5", &body).unwrap();
        assert_eq!(config.legacy_aliases(), ["files", "taken"]);
        assert!(!config.can_hold("gmail", "taken"));
        assert!(config.can_hold("gmail", "files"));
        assert!(!config.can_hold("drive", "files"));
        assert!(config.can_hold("calendar", "files"));
        assert!(!config.can_hold("bad name", "files"));
        for refused in [
            moved(&[("taken", "gmail")]),
            moved(&[("files", "drive")]),
            moved(&[("pinned", "calendar")]),
            moved(&[("absent", "calendar")]),
        ] {
            assert_eq!(
                config.with_instances(&refused).err(),
                Some(Failure::InvalidConfiguration),
                "{refused:?}"
            );
        }
        let after = Config::parse(
            &config
                .with_instances(&moved(&[("files", "gmail"), ("taken", "calendar")]))
                .unwrap()
                .to_text()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(after.names("files"), Some(("gmail", "files")));
        assert_eq!(after.names("taken"), Some(("calendar", "taken")));
        assert_eq!(after.names("pinned"), Some(("gmail", "taken")));
        assert!(after.legacy_aliases().is_empty());
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
