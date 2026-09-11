use clap::Parser;
use connectors_host::{
    http::{HttpConfig, ScopedHttp},
    server::ServiceConfig,
};
use connectors_kubernetes::{Config, Kubernetes};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
mod local;

#[derive(Parser)]
#[command(group(clap::ArgGroup::new("mode").args(["config","local_config"]).required(true)))]
struct Args {
    #[arg(long)]
    config: Option<PathBuf>,
    /// Owner-only nonsecret configuration for the private host binding.
    #[arg(long)]
    local_config: Option<PathBuf>,
    /// Print the independently computed bootstrap and configuration revision.
    #[arg(
        long,
        requires = "local_config",
        conflicts_with = "connectors_private_fd"
    )]
    print_local_bootstrap: bool,
    #[arg(long, hide = true, requires = "local_config")]
    connectors_private_fd: Option<i32>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    service: ServiceConfig,
    http: HttpConfig,
    adapter: Config,
}
fn main() {
    let args = Args::parse();
    if let Some(path) = args.local_config {
        let result = (|| -> connectors_host::local::runtime::Result<()> {
            let adapter = local::Local::load(&path)?;
            if args.print_local_bootstrap {
                println!(
                    "{}",
                    serde_json::to_string(adapter.description())
                        .map_err(|_| connectors_host::local::runtime::Failure::Protocol)?
                );
                Ok(())
            } else {
                connectors_host::local::runtime::serve(
                    args.connectors_private_fd
                        .ok_or(connectors_host::local::runtime::Failure::InvalidInput)?,
                    adapter,
                )
            }
        })();
        if let Err(error) = result {
            eprintln!("{error:?}");
            std::process::exit(1);
        }
        return;
    }
    connectors_host::logging();
    let runtime = tokio::runtime::Runtime::new().expect("create service runtime");
    if let Err(error) = runtime.block_on(run(args.config.expect("validated configuration mode"))) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
async fn run(path: PathBuf) -> connectors_core::Result<()> {
    let config: Configuration = connectors_host::read_config(&path)?;
    let effective =
        serde_json::to_value(&config).map_err(|_| connectors_core::Error::internal())?;
    let adapter = Kubernetes::new(
        &config.service.instance,
        config.adapter,
        effective,
        Arc::new(ScopedHttp::from_config(&config.http)?),
    )?;
    connectors_host::server::serve(config.service, Arc::new(adapter)).await
}
