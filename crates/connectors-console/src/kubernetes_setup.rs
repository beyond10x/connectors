//! Explicit local Kubernetes setup: passive context selection, read policy, and daemon startup.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead as _, IsTerminal as _, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use connectors_config::{
    InitiationConfig, KubernetesIntegrationConfig, OwnerConfig, PersonalConfig,
};
use integration_kubernetes::LocalContextSummary;
use serde_json::{json, Value};

use crate::{daemon, init};

const DEFAULT_PROVIDERS: &[&str] = &[
    "alertmanager",
    "asterisk",
    "grafana",
    "loki",
    "mysql",
    "postgresql",
    "prometheus",
];

#[derive(Debug, Default)]
pub struct Options {
    pub context: Option<String>,
    pub namespaces: Vec<String>,
    pub all_namespaces: bool,
    pub read_providers: Vec<String>,
    pub allow_exec_auth: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("no usable Kubernetes context was found in the local kubeconfig")]
    NoContext,
    #[error("select a Kubernetes context with --context; available contexts: {0}")]
    ContextRequired(String),
    #[error("the selected Kubernetes context was not found")]
    UnknownContext,
    #[error("this context runs a credential helper; explicitly pass --allow-exec-auth")]
    ExecConsent,
    #[error("the existing configuration has ambiguous namespace policy; select --namespace or --all-namespaces")]
    NamespaceRequired,
    #[error("select either --namespace or --all-namespaces")]
    NamespaceConflict,
    #[error("the read-provider selection contains an unknown catalog provider")]
    UnknownProvider,
    #[error("could not read Kubernetes context metadata")]
    ContextMetadata,
    #[error(transparent)]
    Config(#[from] connectors_config::ConfigError),
    #[error(transparent)]
    Init(#[from] init::InitError),
    #[error(transparent)]
    Daemon(#[from] daemon::DaemonError),
    #[error("Kubernetes setup I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("could not render Kubernetes configuration: {0}")]
    Render(#[from] toml::ser::Error),
}

/// Prepare configuration and explicitly start its daemon. This helper never runs auth plugins.
pub async fn prepare(
    config_path: &Path,
    state_root: &Path,
    options: Options,
) -> Result<Value, SetupError> {
    let contexts =
        integration_kubernetes::local_contexts().map_err(|_| SetupError::ContextMetadata)?;
    let existing = match fs::symlink_metadata(config_path) {
        Ok(_) => Some(PersonalConfig::read(config_path)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let mut terminal = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()
        .filter(|terminal| terminal.is_terminal());
    // Redirected stdin is an explicit noninteractive setup, even if a controlling terminal exists.
    if !io::stdin().is_terminal() {
        terminal = None;
    }
    let selected = select_context(&contexts, &options, existing.as_ref())?;
    let mut options = options;
    let previously_allowed = existing
        .as_ref()
        .and_then(|value| value.kubernetes.as_ref())
        .is_some_and(|value| value.allow_exec_auth);
    if selected.requires_exec_auth && !options.allow_exec_auth && !previously_allowed {
        let allowed = if let Some(terminal) = terminal.as_mut() {
            prompt(
                terminal,
                "Run this context's kubeconfig credential helper? [y/N] ",
            )?
            .eq_ignore_ascii_case("y")
        } else {
            false
        };
        if !allowed {
            return Err(SetupError::ExecConsent);
        }
        options.allow_exec_auth = true;
    }
    let old_policy = existing
        .as_ref()
        .and_then(|value| value.kubernetes.as_ref());
    let ambiguous =
        old_policy.is_some_and(|value| value.namespaces.is_empty() && !value.all_namespaces);
    if options.namespaces.is_empty()
        && !options.all_namespaces
        && (old_policy.is_none() || ambiguous)
    {
        let default = selected.namespace.as_deref().unwrap_or("default");
        if let Some(terminal) = terminal.as_mut() {
            let answer = prompt(
                terminal,
                &format!("Namespaces for service discovery [{default}]: "),
            )?;
            options.namespaces = if answer.is_empty() {
                vec![default.to_owned()]
            } else {
                answer
                    .split(',')
                    .map(|value| value.trim().to_owned())
                    .collect()
            };
        } else if ambiguous {
            return Err(SetupError::NamespaceRequired);
        }
    }
    if options.read_providers.is_empty() && old_policy.is_none() {
        if let Some(terminal) = terminal.as_mut() {
            let answer = prompt(
                terminal,
                &format!(
                    "Providers admitted for reads [{}]: ",
                    DEFAULT_PROVIDERS.join(",")
                ),
            )?;
            if !answer.is_empty() {
                options.read_providers = answer
                    .split(',')
                    .map(|value| value.trim().to_owned())
                    .collect();
            }
        }
    }
    let mut config = existing.clone().unwrap_or_else(empty_configuration);
    let policy = configure_policy(config.kubernetes.as_ref(), selected, &options)?;
    config.kubernetes = Some(policy);
    let rendered = toml::to_string_pretty(&config)?;
    let changed = existing
        .as_ref()
        .map(toml::to_string_pretty)
        .transpose()?
        .as_deref()
        != Some(&rendered);
    let mut backup = None;
    if changed {
        let running = daemon::inspect(state_root).await?;
        if let Some(running) = &running {
            let canonical = config_path.canonicalize()?;
            if running.configuration.as_deref() != canonical.to_str() {
                return Err(daemon::DaemonError::ConfigurationConflict.into());
            }
        }
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if existing.is_some() {
            backup = Some(backup_configuration(config_path)?);
        }
        init::write_validated(config_path, rendered.as_bytes(), existing.is_some())?;
        if running.is_some() {
            daemon::stop(state_root).await?;
        }
    }
    let lifecycle = daemon::start(config_path, state_root).await?;
    Ok(
        json!({"context": selected.name, "namespaces": config.kubernetes.as_ref().map(|value| &value.namespaces),
        "all_namespaces": config.kubernetes.as_ref().is_some_and(|value| value.all_namespaces),
        "read_providers": config.kubernetes.as_ref().map(|value| value.target_grants.keys().collect::<Vec<_>>()),
        "configuration_changed": changed, "configuration_backup": backup, "daemon": lifecycle}),
    )
}

fn select_context<'a>(
    contexts: &'a [LocalContextSummary],
    options: &Options,
    existing: Option<&PersonalConfig>,
) -> Result<&'a LocalContextSummary, SetupError> {
    let selected = options.context.as_deref().or_else(|| {
        existing
            .and_then(|value| value.kubernetes.as_ref())
            .and_then(|value| value.selected_context.as_deref())
    });
    if let Some(name) = selected {
        return contexts
            .iter()
            .find(|context| context.name == name)
            .ok_or(SetupError::UnknownContext);
    }
    if let Some(context) = contexts.iter().find(|context| context.current) {
        return Ok(context);
    }
    match contexts {
        [] => Err(SetupError::NoContext),
        [context] => Ok(context),
        contexts => Err(SetupError::ContextRequired(
            contexts
                .iter()
                .map(|context| context.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )),
    }
}

fn configure_policy(
    existing: Option<&KubernetesIntegrationConfig>,
    context: &LocalContextSummary,
    options: &Options,
) -> Result<KubernetesIntegrationConfig, SetupError> {
    if options.all_namespaces && !options.namespaces.is_empty() {
        return Err(SetupError::NamespaceConflict);
    }
    let explicit_scope = options.all_namespaces || !options.namespaces.is_empty();
    let all_namespaces = options.all_namespaces
        || !explicit_scope && existing.is_some_and(|value| value.all_namespaces);
    let mut namespaces = if all_namespaces {
        Vec::new()
    } else if !options.namespaces.is_empty() {
        options.namespaces.clone()
    } else {
        existing
            .filter(|value| !value.namespaces.is_empty())
            .map(|value| value.namespaces.clone())
            .unwrap_or_else(|| {
                vec![context
                    .namespace
                    .clone()
                    .unwrap_or_else(|| "default".into())]
            })
    };
    namespaces.sort();
    namespaces.dedup();
    let providers = if !options.read_providers.is_empty() {
        options.read_providers.clone()
    } else if let Some(existing) = existing {
        existing.target_grants.keys().cloned().collect()
    } else {
        DEFAULT_PROVIDERS
            .iter()
            .map(|value| (*value).to_owned())
            .collect()
    };
    if providers
        .iter()
        .any(|provider| catalog::provider(catalog::ProviderKey::id(provider)).is_none())
    {
        return Err(SetupError::UnknownProvider);
    }
    let target_grants = providers
        .into_iter()
        .map(|provider| {
            let grant = existing
                .and_then(|value| value.target_grants.get(&provider))
                .cloned()
                .unwrap_or_else(|| format!("grant:kubernetes:{provider}:read:local"));
            (provider, grant)
        })
        .collect::<BTreeMap<_, _>>();
    Ok(KubernetesIntegrationConfig {
        grant_ref: existing
            .map(|value| value.grant_ref.clone())
            .unwrap_or_else(|| "grant:kubernetes:local".into()),
        initiation: InitiationConfig::Platform,
        namespaces,
        all_namespaces,
        selected_context: Some(context.name.clone()),
        target_grants,
        allow_exec_auth: options.allow_exec_auth
            || existing.is_some_and(|value| value.allow_exec_auth),
        resource_limit: existing.map_or(256, |value| value.resource_limit),
    })
}

fn empty_configuration() -> PersonalConfig {
    let digest = init::derive_snapshot(&["kubernetes"]);
    PersonalConfig {
        owner: OwnerConfig {
            tenant_id: "local".into(),
            agent_id: init::local_agent_id(),
            agent_revision: 1,
            authority_snapshot_id: format!("snapshot:local:{}", &digest[..16]),
            authority_snapshot_sha256: digest,
        },
        connection: None,
        authority: None,
        application: None,
        sip: None,
        slack: None,
        grafana: None,
        kubernetes: None,
        platform: None,
        catalog: Vec::new(),
    }
}

fn backup_configuration(path: &Path) -> Result<PathBuf, io::Error> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let backup = path.with_extension(format!("toml.backup-{stamp}"));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&backup)?;
    let mut input = fs::File::open(path)?;
    io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    Ok(backup)
}

fn prompt(terminal: &mut fs::File, message: &str) -> Result<String, io::Error> {
    terminal.write_all(message.as_bytes())?;
    terminal.flush()?;
    let mut answer = String::new();
    io::BufReader::new(terminal).read_line(&mut answer)?;
    Ok(answer.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn context() -> LocalContextSummary {
        LocalContextSummary {
            name: "fixture".into(),
            namespace: Some("team".into()),
            current: true,
            requires_exec_auth: false,
        }
    }

    #[test]
    fn fresh_policy_uses_context_namespace_and_explicit_selected_context() {
        let policy = configure_policy(None, &context(), &Options::default()).unwrap();
        assert_eq!(policy.namespaces, ["team"]);
        assert_eq!(policy.selected_context.as_deref(), Some("fixture"));
        assert!(!policy.all_namespaces);
        assert!(!policy.allow_exec_auth);
        assert!(policy.target_grants.contains_key("asterisk"));
    }

    #[test]
    fn explicit_namespace_replaces_existing_cluster_wide_policy() {
        let prior = configure_policy(
            None,
            &context(),
            &Options {
                all_namespaces: true,
                ..Options::default()
            },
        )
        .unwrap();
        let next = configure_policy(
            Some(&prior),
            &context(),
            &Options {
                namespaces: vec!["narrow".into()],
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(next.namespaces, ["narrow"]);
        assert!(!next.all_namespaces);
    }

    #[test]
    fn context_change_in_kubeconfig_does_not_retarget_an_existing_source() {
        let mut existing = empty_configuration();
        existing.kubernetes =
            Some(configure_policy(None, &context(), &Options::default()).unwrap());
        let mut contexts = vec![
            context(),
            LocalContextSummary {
                name: "another".into(),
                ..context()
            },
        ];
        contexts[0].current = false;
        assert_eq!(
            select_context(&contexts, &Options::default(), Some(&existing))
                .unwrap()
                .name,
            "fixture"
        );
    }
}
