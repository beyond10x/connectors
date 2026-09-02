//! Secret-safe operator access to the hosted Integration administration API.

use std::borrow::Cow;
use std::io::{self, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use connectors_client::{AdminIdentityClient, AdminLoginMetadata, HostedClient};
use sha2::{Digest as _, Sha256};
use url::Url;
use zeroize::{Zeroize as _, Zeroizing};

const MAX_SECRET_BYTES: u64 = 8 * 1024;
const MAX_CALLBACK_BYTES: usize = 16 * 1024;
const LOGIN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error(transparent)]
    Client(#[from] connectors_client::ClientError),
    #[error("administrative authentication failed: {0}")]
    Authentication(String),
    #[error("secret input failed: {0}")]
    SecretInput(String),
    #[error(transparent)]
    Output(#[from] crate::output::OutputError),
    #[error("administrative response could not be rendered")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, clap::Args)]
pub struct CommandOptions {
    /// Exact hosted API base, including `/api/connectors/v1` when deployed below it.
    #[arg(long, global = true)]
    endpoint: Option<String>,
    /// Read a short-lived Identity access token from an owner-only file.
    #[arg(long, global = true, conflicts_with = "access_token_stdin")]
    access_token_file: Option<std::path::PathBuf>,
    /// Read a short-lived Identity access token from stdin instead of browser login.
    #[arg(long, global = true)]
    access_token_stdin: bool,
    /// Print the Identity authorization URL instead of opening a browser.
    #[arg(long, global = true)]
    no_browser: bool,
    #[command(subcommand)]
    action: AdminCommand,
}

#[derive(Debug, clap::Subcommand)]
enum AdminCommand {
    /// Inspect activated Integrations and their value-free readiness.
    Integrations {
        #[command(subcommand)]
        command: IntegrationCommand,
    },
    /// Supply credentials required by activated hosted Integrations.
    Credentials {
        #[command(subcommand)]
        command: CredentialCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum IntegrationCommand {
    /// Show configured fields and whether each required credential is present.
    Status,
}

#[derive(Debug, clap::Subcommand)]
enum CredentialCommand {
    /// Write one credential directly into hosted Connector custody.
    Set {
        integration: String,
        credential: String,
        /// Read the secret from stdin.
        #[arg(long, conflicts_with = "secret_file")]
        secret_stdin: bool,
        /// Read the secret from an owner-only regular file.
        #[arg(long)]
        secret_file: Option<std::path::PathBuf>,
        /// Replace an already-present credential.
        #[arg(long)]
        replace: bool,
    },
}

/// Executes one hosted administrative command without exposing credential bytes to the frontend.
pub async fn run(format: crate::output::Format, options: CommandOptions) -> Result<(), AdminError> {
    let endpoint = options
        .endpoint
        .ok_or_else(|| AdminError::Authentication("--endpoint is required".to_owned()))?;
    let client = HostedClient::new(&endpoint)?;
    let secret_stdin = matches!(
        &options.action,
        AdminCommand::Credentials {
            command: CredentialCommand::Set {
                secret_stdin: true,
                ..
            }
        }
    );
    if options.access_token_stdin && secret_stdin {
        return Err(AdminError::SecretInput(
            "--access-token-stdin and --secret-stdin cannot consume the same stdin".to_owned(),
        ));
    }
    let bearer = access_token(
        &client,
        AccessOptions {
            token_file: options.access_token_file,
            token_stdin: options.access_token_stdin,
            no_browser: options.no_browser,
        },
    )
    .await?;
    let value = match options.action {
        AdminCommand::Integrations {
            command: IntegrationCommand::Status,
        } => serde_json::to_value(client.admin_integrations_status(bearer.as_str()).await?)?,
        AdminCommand::Credentials {
            command:
                CredentialCommand::Set {
                    integration,
                    credential,
                    secret_stdin,
                    secret_file,
                    replace,
                },
        } => {
            let value = secret(
                secret_stdin,
                secret_file.as_deref(),
                &format!("{integration} {credential}"),
            )?;
            serde_json::to_value(
                client
                    .set_admin_credential(
                        bearer.as_str(),
                        &integration,
                        &credential,
                        &value,
                        replace,
                    )
                    .await?,
            )?
        }
    };
    crate::output::emit(format, &value)?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct AccessOptions {
    pub token_file: Option<std::path::PathBuf>,
    pub token_stdin: bool,
    pub no_browser: bool,
}

/// Obtains a short-lived exact-audience Identity access credential.
pub async fn access_token(
    client: &HostedClient,
    options: AccessOptions,
) -> Result<Zeroizing<String>, AdminError> {
    if let Some(path) = options.token_file {
        return read_owner_only(&path, 512).map_err(AdminError::SecretInput);
    }
    if options.token_stdin {
        return read_stdin(512).map_err(AdminError::SecretInput);
    }
    let target = client.admin_auth_metadata().await?;
    let identity = AdminIdentityClient::new(&target)?;
    let metadata = identity.login_metadata().await?;
    let grant = tokio::task::spawn_blocking(move || browser_grant(&metadata, options.no_browser))
        .await
        .map_err(|_| AdminError::Authentication("browser login task failed".to_owned()))??;
    identity
        .exchange_access_token(
            &grant.metadata,
            &grant.code,
            &grant.redirect_uri,
            &grant.verifier,
        )
        .await
        .map_err(AdminError::from)
}

/// Reads one value from stdin or an owner-only file, otherwise uses a hidden terminal prompt.
pub fn secret(
    stdin: bool,
    file: Option<&Path>,
    prompt: &str,
) -> Result<Zeroizing<String>, AdminError> {
    let value = if let Some(path) = file {
        read_owner_only(path, MAX_SECRET_BYTES).map_err(AdminError::SecretInput)?
    } else if stdin {
        read_stdin(MAX_SECRET_BYTES).map_err(AdminError::SecretInput)?
    } else {
        Zeroizing::new(
            rpassword::prompt_password(format!("{prompt}: "))
                .map_err(|error| AdminError::SecretInput(error.to_string()))?,
        )
    };
    if value.is_empty() || value.len() > MAX_SECRET_BYTES as usize {
        return Err(AdminError::SecretInput(
            "the secret is empty or exceeds 8 KiB".to_owned(),
        ));
    }
    Ok(value)
}

fn read_owner_only(path: &Path, maximum: u64) -> Result<Zeroizing<String>, String> {
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > maximum
    {
        return Err(format!(
            "{} is not an owner-only bounded file",
            path.display()
        ));
    }
    let value = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    normalize(value, maximum)
}

fn read_stdin(maximum: u64) -> Result<Zeroizing<String>, String> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read stdin: {error}"))?;
    if bytes.len() > maximum as usize {
        bytes.zeroize();
        return Err("stdin exceeds the allowed secret size".to_owned());
    }
    let decoded = String::from_utf8(bytes).map_err(|error| {
        let mut bytes = error.into_bytes();
        bytes.zeroize();
        "stdin is not UTF-8".to_owned()
    })?;
    normalize(decoded, maximum)
}

fn normalize(mut value: String, maximum: u64) -> Result<Zeroizing<String>, String> {
    while value.ends_with(['\n', '\r']) {
        value.pop();
    }
    if value.is_empty()
        || value.len() > maximum as usize
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        value.zeroize();
        return Err("the supplied value is empty, oversized, or contains controls".to_owned());
    }
    Ok(Zeroizing::new(value))
}

struct BrowserGrant {
    metadata: AdminLoginMetadata,
    code: Zeroizing<String>,
    verifier: Zeroizing<String>,
    redirect_uri: String,
}

fn browser_grant(
    metadata: &AdminLoginMetadata,
    no_browser: bool,
) -> Result<BrowserGrant, AdminError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| {
        AdminError::Authentication(format!("cannot bind loopback callback: {error}"))
    })?;
    let redirect_uri = format!(
        "http://127.0.0.1:{}/callback",
        listener
            .local_addr()
            .map_err(|error| AdminError::Authentication(format!(
                "cannot inspect loopback callback: {error}"
            )))?
            .port()
    );
    let state = random_token(32).map_err(AdminError::Authentication)?;
    let nonce = random_token(32).map_err(AdminError::Authentication)?;
    let verifier = Zeroizing::new(random_token(64).map_err(AdminError::Authentication)?);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut authorization = Url::parse(&metadata.authorization_endpoint).map_err(|_| {
        AdminError::Authentication("Identity authorization endpoint is invalid".to_owned())
    })?;
    authorization
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &metadata.cli_client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", "openid profile email")
        .append_pair("state", &state)
        .append_pair("nonce", &nonce)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    if no_browser {
        eprintln!("Open this URL to authenticate:\n{authorization}");
    } else if let Err(error) = open_browser(&authorization) {
        eprintln!("{error}\nOpen this URL to authenticate:\n{authorization}");
    }
    eprintln!("Waiting for Identity authentication ...");
    let code =
        Zeroizing::new(wait_for_callback(&listener, &state).map_err(AdminError::Authentication)?);
    Ok(BrowserGrant {
        metadata: metadata.clone(),
        code,
        verifier,
        redirect_uri,
    })
}

fn wait_for_callback(listener: &TcpListener, expected_state: &str) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("cannot configure loopback callback: {error}"))?;
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let result = read_callback(&mut stream, expected_state);
                write_callback_response(&mut stream, result.is_ok());
                return result;
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err("timed out waiting for Identity login".to_owned());
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(format!("loopback callback failed: {error}")),
        }
    }
}

fn read_callback(stream: &mut TcpStream, expected_state: &str) -> Result<String, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("cannot configure callback: {error}"))?;
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    while request.len() < MAX_CALLBACK_BYTES && !request.windows(4).any(|part| part == b"\r\n\r\n")
    {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("cannot read callback: {error}"))?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
    }
    if request.len() >= MAX_CALLBACK_BYTES {
        return Err("browser callback is too large".to_owned());
    }
    let line = std::str::from_utf8(&request)
        .map_err(|_| "browser callback is not UTF-8".to_owned())?
        .lines()
        .next()
        .ok_or_else(|| "browser callback is empty".to_owned())?;
    let mut words = line.split_ascii_whitespace();
    if words.next() != Some("GET") {
        return Err("browser callback must use GET".to_owned());
    }
    let target = words
        .next()
        .ok_or_else(|| "browser callback has no target".to_owned())?;
    if words.next() != Some("HTTP/1.1") || words.next().is_some() {
        return Err("browser callback request line is invalid".to_owned());
    }
    let callback = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "browser callback target is invalid".to_owned())?;
    if callback.path() != "/callback" {
        return Err("browser callback path is invalid".to_owned());
    }
    let values = callback.query_pairs().collect::<Vec<_>>();
    if let Some(error) = one_query_value(&values, "error")? {
        return Err(format!("Identity refused login: {error}"));
    }
    if one_query_value(&values, "state")?.as_deref() != Some(expected_state) {
        return Err("browser callback state does not match".to_owned());
    }
    let code = one_query_value(&values, "code")?
        .ok_or_else(|| "browser callback omitted its code".to_owned())?;
    if code.is_empty() || code.len() > 2048 || code.contains(char::is_whitespace) {
        return Err("browser callback code is invalid".to_owned());
    }
    Ok(code)
}

fn one_query_value(
    values: &[(Cow<'_, str>, Cow<'_, str>)],
    name: &str,
) -> Result<Option<String>, String> {
    let found = values
        .iter()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.to_string())
        .collect::<Vec<_>>();
    match found.as_slice() {
        [] => Ok(None),
        [value] => Ok(Some(value.clone())),
        _ => Err(format!("browser callback repeated `{name}`")),
    }
}

fn write_callback_response(stream: &mut TcpStream, success: bool) {
    let (status, title) = if success {
        ("200 OK", "Authentication complete")
    } else {
        ("400 Bad Request", "Authentication failed")
    };
    let body = format!("<!doctype html><meta charset=utf-8><title>{title}</title><h1>{title}</h1><p>You can close this window.</p>");
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}", body.len());
    let _ = stream.write_all(response.as_bytes());
}

fn open_browser(url: &Url) -> Result<(), String> {
    let program = std::env::var_os("B10X_BROWSER").unwrap_or_else(|| "xdg-open".into());
    Command::new(program)
        .arg(url.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot open the system browser: {error}"))
}

fn random_token(bytes: usize) -> Result<String, String> {
    let mut value = vec![0_u8; bytes];
    getrandom::fill(&mut value)
        .map_err(|error| format!("cannot obtain operating-system randomness: {error}"))?;
    Ok(URL_SAFE_NO_PAD.encode(value))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use clap::{Parser, Subcommand};

    use super::*;

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(subcommand)]
        command: TestCommand,
    }

    #[derive(Debug, Subcommand)]
    enum TestCommand {
        Admin(CommandOptions),
    }

    #[test]
    fn command_shape_accepts_secret_stdin_without_a_secret_argument() {
        let parsed = TestCli::try_parse_from([
            "connectors",
            "admin",
            "credentials",
            "set",
            "gitlab",
            "oauth_client_secret",
            "--endpoint",
            "https://connectors.example/api/connectors/v1",
            "--secret-stdin",
        ]);
        assert!(parsed.is_ok());
        assert!(TestCli::try_parse_from([
            "connectors",
            "admin",
            "credentials",
            "set",
            "gitlab",
            "oauth_client_secret",
            "--secret",
            "must-not-be-argv",
        ])
        .is_err());
    }

    #[test]
    fn explicit_secret_file_must_be_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("secret");
        std::fs::write(&path, "synthetic-secret\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_owner_only(&path, MAX_SECRET_BYTES).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            read_owner_only(&path, MAX_SECRET_BYTES).unwrap().as_str(),
            "synthetic-secret"
        );
    }
}
