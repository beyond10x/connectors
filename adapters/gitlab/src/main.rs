use clap::Parser;
use connectors_gitlab::{Config, GitLab};
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    server::ServiceConfig,
};
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
    http: HttpConfig,
    adapter: Config,
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
    let adapter = GitLab::new(
        &config.service.instance,
        config.adapter,
        effective,
        Arc::new(ScopedHttp::from_config(&config.http)?),
    )?;
    connectors_host::server::serve(config.service, Arc::new(adapter)).await
}
