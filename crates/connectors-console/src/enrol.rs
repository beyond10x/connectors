//! `connectors connect <provider>` for anything the catalogue declares.
//!
//! # What this removes
//!
//! Adding a provider used to mean: write the credential into an owner-only file by hand, hand-write
//! a `[[catalog]]` block with the right grant reference and endpoint variables, restart the daemon,
//! and find out at the first invocation whether any of it was right. Every one of those steps is
//! answerable from the catalogue, so none of them should be a person's job.
//!
//! The catalogue already declares, per provider: which credentials exist and what each is for, which
//! configuration variables its base URL carries and what they mean, whether a value needs operator
//! approval, and which operation verifies the result. This walks exactly that and asks only what the
//! catalogue cannot answer — the values themselves.
//!
//! # Where the credential goes
//!
//! Straight from a non-echoing prompt into the [`SecretStore`], at the address
//! [`connector_resolve`] will look it up under. It is never written to the configuration, never
//! passed as an argument, and never printed. The configuration gets policy — which provider, which
//! credential name, which endpoint values, what the grant admits — and no value.
//!
//! # Why it does not need the daemon
//!
//! The curated guided flows ([`crate::connect`]) drive a running Connector because they complete a
//! Connect Session, which is how a provider-initiated OAuth or Socket Mode credential arrives. A
//! catalogued provider with a pasted credential needs none of that: the store and the configuration
//! are both files this process can write. So `connect` works before `serve` has ever run, which is
//! the order a person actually does things in.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use connector_secrets::Secret;
use connectors_config::PersonalConfig;
use serde_json::{json, Value};
use zeroize::Zeroizing;

/// What the caller fixed on the command line, so nothing already answered is asked again.
#[derive(Debug, Default)]
pub struct Options {
    /// Which declared credential to supply. Defaults to the provider's first.
    pub credential: Option<String>,
    /// Configuration values, by declared field name.
    pub values: BTreeMap<String, String>,
    /// Raise the grant ceiling past reads.
    pub allow_writes: bool,
    /// Admit private destinations, for a self-hosted instance on the operator's own network.
    pub operator_network: bool,
    /// Replace an existing entry for this provider.
    pub force: bool,
    /// A stable name, when this placement holds the same provider more than once.
    ///
    /// Two Slack identities — a workspace bot and a personal companion — are the same provider,
    /// tenant and credential name, so only an instance separates their stored credentials. Naming
    /// one here is what puts an instance segment in its address.
    pub name: Option<String>,
    /// Read the credential from an owner-only file instead of prompting.
    ///
    /// The scriptable path, and deliberately a **file rather than an environment variable or an
    /// argument**: design 07 rules out an environment fallback, and an argument would put the value
    /// in `ps` output and shell history. The file is read once, its bytes go to the store, and it
    /// can be deleted afterwards — the same import the runtime performs for a declared instance.
    pub credential_file: Option<std::path::PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum EnrolError {
    #[error("`{0}` is not in the catalogue; `connectors providers` lists what is")]
    UnknownProvider(String),
    #[error("`{0}` declares no authority, so its credential has no address")]
    NoAuthority(String),
    #[error("`{provider}` declares no credential named `{credential}`")]
    UnknownCredential { provider: String, credential: String },
    #[error("`{0}` declares no credential to supply")]
    NoCredential(String),
    #[error("`{0}` is already configured; pass --force to replace it")]
    AlreadyConfigured(String),
    #[error("a value is required for `{0}` and none was supplied")]
    MissingValue(String),
    #[error("the credential could not be read from the terminal: {0}")]
    Prompt(#[from] std::io::Error),
    #[error("{0} is not an owner-only regular file")]
    UnsafeCredentialFile(String),
    #[error("the credential could not be stored: {0}")]
    Store(#[from] connector_secrets::StoreError),
    #[error("the configuration could not be written: {0}")]
    Config(#[from] connectors_config::ConfigError),
}

/// Add one catalogued provider: ask what the catalogue cannot answer, store the credential, and
/// record the policy.
///
/// # Errors
///
/// A provider outside the catalogue, one already configured without `--force`, a required value
/// with no answer, or a store or configuration that refused the write.
pub async fn run(
    provider_id: &str,
    config_path: &Path,
    state_root: &Path,
    options: &Options,
) -> Result<Value, EnrolError> {
    let provider = catalog::provider(catalog::ProviderKey::id(provider_id))
        .ok_or_else(|| EnrolError::UnknownProvider(provider_id.to_owned()))?;
    let authority = provider
        .authority
        .ok_or_else(|| EnrolError::NoAuthority(provider_id.to_owned()))?;

    let existing = PersonalConfig::read(config_path)?;
    // Named entries are distinct Connections of one provider, so a clash is on the *name*, not on
    // the provider: connecting a second Slack identity must not read as connecting Slack twice.
    let identity = options.name.as_deref().unwrap_or(provider_id);
    let already = existing
        .catalog
        .iter()
        .any(|entry| entry.provider == provider_id && entry.name() == identity);

    let credential = match options.credential.as_deref() {
        Some(name) => provider
            .auth
            .iter()
            .find(|item| item.name == name)
            .ok_or_else(|| EnrolError::UnknownCredential {
                provider: provider_id.to_owned(),
                credential: name.to_owned(),
            })?,
        None => provider
            .auth
            .first()
            .ok_or_else(|| EnrolError::NoCredential(provider_id.to_owned()))?,
    };

    // Everything the base URL needs, and nothing else. A provider whose hosts are fixed — most of
    // the catalogue — asks nothing here.
    let mut endpoints = BTreeMap::new();
    let mut approval_needed = Vec::new();
    for field in provider.config {
        let Some(variable) = field.binds.strip_prefix("endpoint.") else {
            continue;
        };
        let value = match options.values.get(field.name) {
            Some(supplied) => supplied.clone(),
            None => match prompt_value(field)? {
                Some(answered) => answered,
                None if field.required => {
                    return Err(EnrolError::MissingValue(field.name.to_owned()))
                }
                None => continue,
            },
        };
        // A value differing from the catalogue's default is the operator pointing this credential
        // at an instance the catalogue did not name. That is the case the approval flag exists for.
        if matches!(field.approval, catalog::Approval::Operator) && Some(value.as_str()) != field.default
        {
            approval_needed.push(field.name.to_owned());
        }
        endpoints.insert(variable.to_owned(), value);
    }

    // Whose authority the value will carry, from the catalogue's own declaration. It is the fact an
    // operator most needs before pasting: a user token acts as them and sees everything they see,
    // an app token acts as a bot and is bounded by its own memberships. Printed before the prompt
    // rather than discovered at the first surprising result.
    eprintln!("Connect {} ({})", provider.vendor, provider.id);
    eprintln!("Credential: {}", credential.name);
    eprintln!("  {}", subject_sentence(credential.subject));
    if let Some(hazard) = credential.hazard {
        // A declared weakness in *obtaining* this credential. Naming it at the prompt is the only
        // moment it can change what someone does.
        eprintln!("  Declared hazard: {hazard:?}");
    }
    if !approval_needed.is_empty() {
        eprintln!(
            "  Operator approval: {} differs from the catalogue default, so this Connection records \
             your approval of it.",
            approval_needed.join(", ")
        );
    }
    eprintln!("Input is hidden and goes straight to the credential store.");

    let value = match options.credential_file.as_deref() {
        Some(path) => read_credential_file(path)?,
        None => Zeroizing::new(rpassword::prompt_password(format!("{}: ", credential.name))?),
    };
    if value.trim().is_empty() {
        return Err(EnrolError::MissingValue(credential.name.to_owned()));
    }

    // Addressed exactly as the runtime will read it: the same function, so a credential this
    // command stores cannot land somewhere the backend does not look.
    let entry = connectors_config::CatalogIntegrationConfig {
        provider: provider_id.to_owned(),
        name: options.name.clone(),
        label: None,
        grant_ref: format!("grant:{provider_id}:local"),
        initiation: connectors_config::InitiationConfig::B10x,
        allow_writes: options.allow_writes,
        endpoints: endpoints.clone(),
        operator_approved: true,
        network: connectors_config::NetworkScopeConfig::Public,
        credential: Some(credential.name.to_owned()),
        credential_file: None,
    };
    let reference = integration_catalog::credential_address(
        existing.owner.tenant_id.as_str(),
        authority,
        &entry,
        credential.leaf,
    )
    .map_err(|_| EnrolError::NoAuthority(provider_id.to_owned()))?;
    let (store, backend) = crate::auth::open_store(state_root)?;
    store.put(&reference, &Secret::new(value.trim())).await?;
    drop(value);

    // **One identity, several credentials.** A companion bot holds a bot token *and* a user token:
    // same provider, same instance, different leaf, so they are already distinct addresses. Adding
    // the second is storing a value against an identity that exists, not declaring a second
    // Connection — and `assemble_credentials` then picks whichever declared mechanism resolves,
    // so a user-token operation starts working the moment its credential is there.
    //
    // The first cut refused this as "already configured", which pushed an operator into inventing
    // a second identity (`timo-ai-user`) for what is one actor holding two tokens.
    if already {
        return Ok(json!({
            "provider": provider_id,
            "name": identity,
            "credential": credential.name,
            "store": backend,
            "added_to_existing_identity": true,
            "verify": provider.verify,
        }));
    }

    append_entry(config_path, provider_id, credential.name, &endpoints, options)?;

    Ok(json!({
        "provider": provider_id,
        "name": identity,
        "credential": credential.name,
        "store": backend,
        "endpoints": endpoints,
        "grant": if options.allow_writes { "read and write" } else { "read only" },
        "verify": provider.verify,
        "operations": provider.operations.len(),
        "next": "connectors serve, then connectors operation search",
    }))
}

/// Read a credential from an owner-only file.
///
/// The same checks the runtime applies to a declared instance's `credential_file`: a regular file,
/// owned by this user, no group or other bits, and bounded — a credential file readable by anyone
/// else is a credential that has already leaked.
fn read_credential_file(path: &Path) -> Result<Zeroizing<String>, EnrolError> {
    use std::os::unix::fs::MetadataExt as _;
    use std::os::unix::fs::PermissionsExt as _;

    const MAX_CREDENTIAL_FILE_BYTES: u64 = 8 * 1024;

    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > MAX_CREDENTIAL_FILE_BYTES
    {
        return Err(EnrolError::UnsafeCredentialFile(path.display().to_string()));
    }
    Ok(Zeroizing::new(std::fs::read_to_string(path)?))
}

/// Parse one `field=value` setting.
///
/// # Errors
///
/// A string with no `=`, naming what was expected and what arrived.
pub fn parse_setting(raw: &str) -> Result<(String, String), String> {
    raw.split_once('=')
        .map(|(field, value)| (field.to_owned(), value.to_owned()))
        .ok_or_else(|| format!("expected `field=value`, got `{raw}`"))
}

/// What the catalogue says this credential will act as.
const fn subject_sentence(subject: catalog::Subject) -> &'static str {
    match subject {
        catalog::Subject::User => {
            "acts as the person who issued it, and sees everything that person can see"
        }
        catalog::Subject::App => {
            "acts as an application identity, bounded by its own memberships and scopes"
        }
        // Every connector shipped before the subject axis was added is in this state. Saying so is
        // better than picking one of the other two and being wrong half the time.
        catalog::Subject::Unstated => {
            "does not declare whose authority it carries — check the provider's own documentation"
        }
    }
}

/// Ask for one declared configuration value, showing what the catalogue says about it.
fn prompt_value(field: &catalog::ConfigField) -> Result<Option<String>, EnrolError> {
    use std::io::{BufRead as _, Write as _};

    let mut prompt = String::new();
    let _ = write!(prompt, "{}", field.label);
    if let Some(default) = field.default {
        let _ = write!(prompt, " [{default}]");
    }
    let _ = write!(prompt, ": ");

    eprintln!("  {}", field.help);
    if let Some(example) = field.example {
        eprintln!("  e.g. {example}");
    }
    eprint!("{prompt}");
    std::io::stderr().flush()?;

    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    let answer = answer.trim();
    if answer.is_empty() {
        return Ok(field.default.map(ToOwned::to_owned));
    }
    Ok(Some(answer.to_owned()))
}

/// Append the `[[catalog]]` block, then prove the file still reads.
///
/// Appended textually rather than by re-serializing the whole configuration, so an operator's
/// comments and ordering survive being connected to a new provider. Validated by reading the
/// result back through the daemon's own reader, exactly as `init` does — a configuration this
/// command wrote must never be one the daemon then refuses.
fn append_entry(
    config_path: &Path,
    provider: &str,
    credential: &str,
    endpoints: &BTreeMap<String, String>,
    options: &Options,
) -> Result<(), EnrolError> {
    let previous = std::fs::read_to_string(config_path)?;
    let mut block = String::new();
    let _ = write!(
        block,
        "\n[[catalog]]\nprovider = \"{provider}\"\ngrant_ref = \"grant:{provider}:local\"\n\
         initiation = \"b10x\"\nallow_writes = {}\ncredential = \"{credential}\"\n\
         operator_approved = true\n",
        options.allow_writes
    );
    if let Some(name) = options.name.as_deref() {
        let _ = write!(block, "name = \"{name}\"\n");
    }
    if options.operator_network {
        let _ = write!(block, "network = \"operator\"\n");
    }
    if !endpoints.is_empty() {
        let _ = write!(block, "\n[catalog.endpoints]\n");
        for (name, value) in endpoints {
            let _ = write!(block, "{name} = \"{value}\"\n");
        }
    }

    let mut next = previous.clone();
    next.push_str(&block);
    std::fs::write(config_path, &next)?;

    if let Err(error) = PersonalConfig::read(config_path) {
        // Put the file back exactly as it was. A half-connected provider that stops the daemon
        // reading its configuration would take every other provider down with it.
        std::fs::write(config_path, previous)?;
        return Err(EnrolError::Config(error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold() {
        // The shape Timo's companion bot needs: `timo-ai` is one actor with two tokens, not two
        // actors. They differ only by leaf, so one instance addresses both without collision.
        let slack = catalog::provider(catalog::ProviderKey::id("slack")).expect("slack");
        let bot = slack
            .auth
            .iter()
            .find(|c| c.name == "slack.bot_token")
            .expect("a bot credential");
        let user = slack
            .auth
            .iter()
            .find(|c| c.name == "slack.user_token")
            .expect("a user credential");
        assert_ne!(bot.leaf, user.leaf, "different leaves, so one instance holds both");
        assert!(matches!(bot.subject, catalog::Subject::App));
        assert!(matches!(user.subject, catalog::Subject::User));
    }

    #[test]
    fn a_provider_outside_the_catalogue_is_named_rather_than_guessed_at() {
        let error = EnrolError::UnknownProvider("nosuch".to_owned());
        assert!(error.to_string().contains("nosuch"));
        assert!(
            error.to_string().contains("connectors providers"),
            "the refusal points at the command that lists what is available"
        );
    }

    #[test]
    fn gitlab_asks_for_nothing_when_its_default_origin_is_wanted() {
        // The catalogue's `origin` field carries a default of `https://gitlab.com`, so connecting
        // gitlab.com is a credential prompt and nothing else. That is the property that makes
        // "adding a provider is a row, not a program" true in practice.
        let provider = catalog::provider(catalog::ProviderKey::id("gitlab")).expect("gitlab");
        let endpoint_fields: Vec<_> = provider
            .config
            .iter()
            .filter(|field| field.binds.starts_with("endpoint."))
            .collect();
        assert_eq!(endpoint_fields.len(), 1);
        assert_eq!(endpoint_fields[0].default, Some("https://gitlab.com"));
        assert!(!endpoint_fields[0].required);
    }

    #[test]
    fn a_self_hosted_origin_is_the_case_operator_approval_exists_for() {
        let provider = catalog::provider(catalog::ProviderKey::id("gitlab")).expect("gitlab");
        let origin = provider
            .config
            .iter()
            .find(|field| field.binds == "endpoint.origin")
            .expect("gitlab declares an origin");
        assert!(
            matches!(origin.approval, catalog::Approval::Operator),
            "pointing a credential at a host the catalogue did not name is an operator decision"
        );
    }

    #[test]
    fn most_of_the_catalogue_asks_no_configuration_question_at_all() {
        // A SaaS provider has fixed hosts, so connecting it is one prompt. Measured rather than
        // asserted, because it is the claim the whole command rests on.
        let askless = catalog::providers()
            .iter()
            .filter(|provider| {
                provider
                    .config
                    .iter()
                    .filter(|field| field.binds.starts_with("endpoint."))
                    .all(|field| field.default.is_some() || !field.required)
            })
            .count();
        assert!(
            askless > 40,
            "only {askless} providers can be connected without answering an endpoint question"
        );
    }
}
