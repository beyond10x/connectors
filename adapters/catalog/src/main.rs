//! One process per provider bundle, reached only through the host's private
//! protocol. There is no standalone service realization here.
use clap::{ArgGroup, Parser};
use std::path::PathBuf;
mod local;

#[derive(Parser)]
#[command(version)]
#[command(group(ArgGroup::new("configuration").required(true).args(["local_config", "connector"])))]
struct Args {
    /// Owner-only nonsecret configuration for the private host binding.
    #[arg(long)]
    local_config: Option<PathBuf>,
    /// A connector this binary ships, configured from its declaration instead
    /// of a configuration file.
    #[arg(long, requires = "instance")]
    connector: Option<String>,
    /// The instance's name; its id is `<connector>.<instance>`.
    #[arg(long, requires = "connector")]
    instance: Option<String>,
    /// The instance's API base, in place of the connector's default endpoint.
    #[arg(long, requires = "connector")]
    endpoint: Option<String>,
    /// The auth profile, when the connector declares more than one.
    #[arg(long, requires = "connector")]
    profile: Option<String>,
    /// Print the independently computed bootstrap and configuration revision.
    #[arg(long, conflicts_with = "connectors_private_fd")]
    print_local_bootstrap: bool,
    /// Print the auth profiles the connector declares, as a JSON array.
    #[arg(long, requires = "connector", conflicts_with_all = ["print_local_bootstrap", "connectors_private_fd"])]
    print_profiles: bool,
    #[arg(long, hide = true)]
    connectors_private_fd: Option<i32>,
}
fn main() {
    let args = Args::parse();
    let result = (|| -> connectors_host::local::runtime::Result<()> {
        if args.print_profiles {
            let connector = args
                .connector
                .as_deref()
                .ok_or(connectors_host::local::runtime::Failure::InvalidInput)?;
            println!(
                "{}",
                serde_json::to_string(&local::profiles(connector)?)
                    .map_err(|_| connectors_host::local::runtime::Failure::Protocol)?
            );
            return Ok(());
        }
        let adapter = match (&args.local_config, &args.connector, &args.instance) {
            (Some(path), _, _) => local::Local::load(path)?,
            (None, Some(connector), Some(name)) => local::Local::derive(&local::Instance {
                connector,
                name,
                endpoint: args.endpoint.as_deref(),
                profile: args.profile.as_deref(),
            })?,
            _ => return Err(connectors_host::local::runtime::Failure::InvalidInput),
        };
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
}
