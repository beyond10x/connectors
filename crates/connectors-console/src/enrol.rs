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
//! # Credential ownership
//!
//! The console collects the declared metadata and hidden input. The running daemon owns every
//! credential write and provider acquisition through one bounded owner-only socket exchange.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use connectors_config::PersonalConfig;
use protocol::local_setup::{CredentialSource, EnrollRequest, SecretValue};
use serde_json::Value;
use zeroize::Zeroizing;

/// Where one declared configuration value belongs, read from its `binds` grammar.
///
/// The catalogue's answer, not a guess from the field's name: `email` is a user half for Atlassian
/// because it declares `binds = "username.jira.api_token"`, and another connector's `email` that
/// declared an endpoint would be an endpoint.
enum Slot<'a> {
    /// A `{variable}` in the provider's declared base URL.
    Endpoint(&'a str),
    /// The non-secret user half of the named credential's Basic join.
    Username(&'a str),
}

/// What the caller fixed on the command line, so nothing already answered is asked again.
#[derive(Default)]
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
    pub instance: Option<String>,
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
    #[error("`{0}` is not in the catalogue; `connectors inspect providers` lists what is")]
    UnknownProvider(String),
    #[error("`{0}` declares no authority, so its credential has no address")]
    NoAuthority(String),
    #[error("`{provider}` declares no credential named `{credential}`")]
    UnknownCredential {
        provider: String,
        credential: String,
    },
    #[error("`{0}` declares no credential to supply")]
    NoCredential(String),
    #[error("`{0}` is already configured; pass --force to replace it")]
    AlreadyConfigured(String),
    #[error("a value is required for `{0}` and none was supplied")]
    MissingValue(String),
    #[error("the credential could not be read from the terminal: {0}")]
    Prompt(#[from] std::io::Error),
    #[error("{0}")]
    Acquisition(String),
    #[error("{0} is not an owner-only regular file")]
    UnsafeCredentialFile(String),
    #[error(transparent)]
    Client(#[from] connectors_client::ClientError),
    #[error(transparent)]
    Daemon(#[from] crate::daemon::DaemonError),
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
    mut options: Options,
) -> Result<Value, EnrolError> {
    let provider = catalog::provider(catalog::ProviderKey::id(provider_id))
        .ok_or_else(|| EnrolError::UnknownProvider(provider_id.to_owned()))?;
    let _authority = provider
        .authority
        .ok_or_else(|| EnrolError::NoAuthority(provider_id.to_owned()))?;

    let _existing = PersonalConfig::read(config_path)?;
    crate::daemon::start(config_path, state_root).await?;

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

    let credential_name = credential.name;
    // Everything the base URL needs and every user half a Basic credential joins, and nothing else.
    // A provider whose hosts are fixed and whose token is a bearer — most of the catalogue — asks
    // nothing here.
    //
    // **Both come from the same walk over `provider.config`**, because the catalogue already says
    // which is which: `binds = "endpoint.<variable>"` is a URL slot, `binds = "username.<credential>"`
    // is the non-secret user half of that credential's Basic join. Asking only for the first is
    // what made a stored Atlassian token unusable — the value had nowhere to be written down, so
    // every Jira and Confluence call refused a credential that was demonstrably there.
    let mut endpoints = BTreeMap::new();
    let mut usernames = BTreeMap::new();
    let mut approval_needed = Vec::new();
    for field in provider.config {
        let target = match (
            field.binds.strip_prefix("endpoint."),
            field.binds.strip_prefix("username."),
        ) {
            (Some(variable), _) => Slot::Endpoint(variable),
            // **Only the credential being supplied.** A provider may declare a Basic token and a
            // bearer service-account token; the Basic one needs an account name and the bearer one
            // has no user half at all. Asking for every declared user half would make connecting
            // the bearer credential demand an email that belongs to a credential this invocation
            // is not supplying — which is exactly what it did, and it refused rather than storing.
            (_, Some(credential)) if credential == credential_name => Slot::Username(credential),
            _ => continue,
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
        if matches!(field.approval, catalog::Approval::Operator)
            && Some(value.as_str()) != field.default
        {
            approval_needed.push(field.name.to_owned());
        }
        match target {
            Slot::Endpoint(variable) => endpoints.insert(variable.to_owned(), value),
            Slot::Username(credential) => usernames.insert(credential.to_owned(), value),
        };
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
    eprintln!("Input is hidden and sent only to the local Connector daemon.");
    let source = match options.credential_file.take() {
        Some(path) => CredentialSource::File {
            path: if path.is_absolute() {
                path
            } else {
                std::env::current_dir()?.join(path)
            },
        },
        None if acquires(provider_id) => {
            let origin = endpoints
                .get("origin")
                .ok_or_else(|| EnrolError::MissingValue("origin".into()))?
                .clone();
            let request = argocd_request(origin, options.allow_writes)?;
            CredentialSource::Argocd {
                username: request.username,
                password: SecretValue::new(request.password.to_string()),
                project: request.project,
                role: request.role,
                expires_in_seconds: request.expires_in_seconds,
            }
        }
        None => {
            let value = Zeroizing::new(rpassword::prompt_password(format!(
                "{}: ",
                credential.name
            ))?);
            if value.trim().is_empty() {
                return Err(EnrolError::MissingValue(credential.name.into()));
            }
            CredentialSource::Pasted {
                value: SecretValue::new(value.trim().to_owned()),
            }
        }
    };
    let mut outcome = connectors_client::LocalClient::new(state_root.join("connectors.sock"))
        .enroll(EnrollRequest {
            provider: provider_id.to_owned(),
            instance: options.instance,
            credential: credential.name.to_owned(),
            endpoints,
            usernames,
            allow_writes: options.allow_writes,
            operator_network: options.operator_network,
            source,
        })
        .await?;
    if outcome["reload_required"] == true {
        crate::daemon::stop(state_root).await?;
        outcome["daemon"] = crate::daemon::start(config_path, state_root).await?;
    }
    Ok(outcome)
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

/// Whether this provider issues its own credential rather than expecting a pasted one.
///
/// One name today, and deliberately a function rather than a catalogue lookup: acquiring is
/// hand-written per provider, so a provider is on this list exactly when code exists to do it.
/// The catalogue cannot answer "is there an implementation", and a declaration that claimed one
/// without it would fail at the moment a person is holding a password.
#[must_use]
pub fn acquires(provider: &str) -> bool {
    provider == "argocd"
}

/// Ask for the parts of an Argo CD acquisition the catalogue cannot supply.
///
/// The project and role are the operator's own nouns, and the password is the one value here that
/// must not outlive its use — it goes into the request as a `Zeroizing` and the acquisition drops
/// it after the single sign-in it pays for.
fn argocd_request(
    origin: String,
    allow_sync: bool,
) -> Result<integration_catalog::argocd::AcquireRequest, EnrolError> {
    eprintln!();
    eprintln!("Argo CD issues its own tokens, so there is nothing for you to fetch first.");
    eprintln!(
        "Sign in once and a scoped, expiring token is minted and stored; the password is not."
    );
    let project = ask("Argo CD project whose applications this connection reads")?;
    let role = ask_with_default("Role to create in that project", "b10x")?;
    let username = ask("Argo CD username with `projects, update` on it (often `admin`)")?;
    let password = Zeroizing::new(rpassword::prompt_password("Argo CD password: ")?);
    if password.trim().is_empty() {
        return Err(EnrolError::MissingValue("password".to_owned()));
    }
    Ok(integration_catalog::argocd::AcquireRequest {
        origin,
        username,
        password,
        project,
        role,
        allow_sync,
        expires_in_seconds: integration_catalog::argocd::DEFAULT_EXPIRES_IN_SECONDS,
    })
}

fn ask(prompt: &str) -> Result<String, EnrolError> {
    use std::io::{BufRead as _, Write as _};

    eprint!("{prompt}: ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    let answer = answer.trim().to_owned();
    if answer.is_empty() {
        return Err(EnrolError::MissingValue(prompt.to_owned()));
    }
    Ok(answer)
}

fn ask_with_default(prompt: &str, default: &str) -> Result<String, EnrolError> {
    use std::io::{BufRead as _, Write as _};

    eprint!("{prompt} [{default}]: ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    let answer = answer.trim();
    Ok(if answer.is_empty() {
        default.to_owned()
    } else {
        answer.to_owned()
    })
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
        assert_ne!(
            bot.leaf, user.leaf,
            "different leaves, so one instance holds both"
        );
        assert!(matches!(bot.subject, catalog::Subject::App));
        assert!(matches!(user.subject, catalog::Subject::User));
    }

    #[test]
    fn a_provider_outside_the_catalogue_is_named_rather_than_guessed_at() {
        let error = EnrolError::UnknownProvider("nosuch".to_owned());
        assert!(error.to_string().contains("nosuch"));
        assert!(
            error.to_string().contains("connectors inspect providers"),
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
