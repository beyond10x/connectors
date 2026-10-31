//! One process per provider bundle, reached through the host's private
//! protocol, or one catalog instance on the served HTTP host (`--config`,
//! reads only; see `served`).
use clap::{ArgGroup, Parser};
use std::path::PathBuf;
mod local;
mod served;

#[derive(Parser)]
#[command(version)]
#[command(group(ArgGroup::new("configuration").required(true).args(["config", "local_config", "connector"])))]
struct Args {
    /// Serve one catalog instance on the HTTP host: the service, the
    /// instance and its mounted credential (docs/running-services.md).
    #[arg(long, conflicts_with_all = ["print_local_bootstrap", "print_profiles", "connectors_private_fd"])]
    config: Option<PathBuf>,
    /// Owner-only nonsecret configuration for the private host binding.
    #[arg(long)]
    local_config: Option<PathBuf>,
    /// A connector this binary ships, configured from its declaration instead
    /// of a configuration file.
    #[arg(long, requires = "instance")]
    connector: Option<String>,
    /// The instance's name; its id is `<connector>.<instance>` unless
    /// `--instance-id` names it.
    #[arg(long, requires = "connector")]
    instance: Option<String>,
    /// The instance's id in place of `<connector>.<instance>`: the id a
    /// migrated instance keeps.
    #[arg(long, requires = "connector")]
    instance_id: Option<String>,
    /// The instance's API base, in place of the connector's default endpoint.
    #[arg(long, requires = "connector")]
    endpoint: Option<String>,
    /// The auth profile, when the connector declares more than one.
    #[arg(long, requires = "connector")]
    profile: Option<String>,
    /// Print the independently computed bootstrap and configuration revision.
    #[arg(long, conflicts_with = "connectors_private_fd")]
    print_local_bootstrap: bool,
    /// Print, per key of the configuration revision's digest input, the
    /// digest of its value, as a JSON object.
    #[arg(long, conflicts_with_all = ["print_local_bootstrap", "print_profiles", "connectors_private_fd"])]
    print_revision_inputs: bool,
    /// Print the auth profiles the connector declares, as a JSON array.
    #[arg(long, requires = "connector", conflicts_with_all = ["print_local_bootstrap", "connectors_private_fd"])]
    print_profiles: bool,
    #[arg(long, hide = true)]
    connectors_private_fd: Option<i32>,
}
fn main() {
    let args = Args::parse();
    if let Some(path) = &args.config {
        connectors_host::logging();
        let runtime = tokio::runtime::Runtime::new().expect("create service runtime");
        if let Err(error) = runtime.block_on(served::run(path)) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
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
                instance_id: args.instance_id.as_deref(),
                endpoint: args.endpoint.as_deref(),
                profile: args.profile.as_deref(),
            })?,
            _ => return Err(connectors_host::local::runtime::Failure::InvalidInput),
        };
        if args.print_revision_inputs {
            println!(
                "{}",
                serde_json::to_string(adapter.revision_inputs())
                    .map_err(|_| connectors_host::local::runtime::Failure::Protocol)?
            );
            Ok(())
        } else if args.print_local_bootstrap {
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
