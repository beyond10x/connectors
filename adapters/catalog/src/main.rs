//! One process per provider bundle, reached only through the host's private
//! protocol. There is no standalone service realization here.
use clap::Parser;
use std::path::PathBuf;
mod local;

#[derive(Parser)]
struct Args {
    /// Owner-only nonsecret configuration for the private host binding.
    #[arg(long)]
    local_config: PathBuf,
    /// Print the independently computed bootstrap and configuration revision.
    #[arg(long, conflicts_with = "connectors_private_fd")]
    print_local_bootstrap: bool,
    #[arg(long, hide = true)]
    connectors_private_fd: Option<i32>,
}
fn main() {
    let args = Args::parse();
    let result = (|| -> connectors_host::local::runtime::Result<()> {
        let adapter = local::Local::load(&args.local_config)?;
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
