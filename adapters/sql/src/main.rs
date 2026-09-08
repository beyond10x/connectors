use clap::Parser;
use connectors_host::{credentials::CredentialRef, server::ServiceConfig};
use connectors_sql::{Config, Sql};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: PathBuf,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    service: ServiceConfig,
    adapter: Config,
    password: CredentialRef,
}
#[tokio::main]
async fn main() {
    connectors_host::logging();
    if let Err(error) = run(Args::parse()).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
async fn run(args: Args) -> connectors_core::Result<()> {
    let config: Configuration = connectors_host::read_config(&args.config)?;
    let effective =
        serde_json::to_value(&config).map_err(|_| connectors_core::Error::internal())?;
    let adapter = Sql::new(
        &config.service.instance,
        config.adapter,
        effective,
        Arc::new(config.password),
    )?;
    connectors_host::server::serve(config.service, Arc::new(adapter)).await
}
