use clap::Parser;
use connector_secrets::{CredentialScope, SecretStore};
use connectors_config::{HostedSecretsConfig, HostedVaultConfig};
use hosted_secrets::HostedSecretsStore;
use hosted_vault::HostedVaultStore;
use serde::Deserialize;
use std::{fs, path::PathBuf};

const MAX_CONFIG_BYTES: u64 = 256 * 1024;

#[derive(Parser)]
#[command(about = "Copy bounded Vault credential scopes into Secrets without deleting the source")]
struct Cli { #[arg(long)] config: PathBuf }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationConfig {
    vault: HostedVaultConfig,
    secrets: HostedSecretsConfig,
    scopes: Vec<Scope>,
    owner_subject: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope { tenant: String, authority: String }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let metadata = fs::metadata(&args.config)?;
    if metadata.len() > MAX_CONFIG_BYTES { return Err("migration configuration is too large".into()); }
    let config: MigrationConfig = toml::from_str(&fs::read_to_string(&args.config)?)?;
    if config.scopes.is_empty() || config.scopes.len() > 128 { return Err("migration needs 1..=128 scopes".into()); }
    let source = HostedVaultStore::new(&config.vault)?;
    source.initialize().await?;
    let destination = HostedSecretsStore::new(&config.secrets)?;
    destination.ready().await?;
    let mut copied = 0_u64;
    let mut existing = 0_u64;
    for scope in config.scopes {
        let scope = CredentialScope::new(&scope.tenant, &scope.authority).map_err(|_| "migration scope is invalid")?;
        for reference in source.references(&scope).await? {
            if destination.exists(&reference).await? { existing += 1; continue; }
            let value = source.get(&reference).await?;
            if let Some(owner) = config.owner_subject.as_deref() { destination.put_owned(&reference, owner, &value).await?; }
            else { destination.put(&reference, &value).await?; }
            copied += 1;
        }
    }
    println!("copied={copied} existing={existing}; source unchanged");
    Ok(())
}
