//! `connectors-tool-contracts`: publish a tool contract, and write or check the committed ones.

use clap::{Parser, Subcommand};
use connectors_tool_contracts::{
    ADAPTERS, Declarations, generate, generated_path, publish, to_yaml,
};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    version,
    about = "Publishes tool-contract/1 documents for an instance's admitted operations"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the contract a descriptor and its declarations publish, or every refusal.
    Publish {
        /// The instance descriptor (JSON).
        #[arg(long)]
        descriptor: PathBuf,
        /// The declarations (`connectors.tool-profiles/1`, JSON).
        #[arg(long)]
        profiles: PathBuf,
    },
    /// Write each adapter's committed contract, or with `--check` fail on a difference.
    Generate {
        /// The repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Compare instead of writing.
        #[arg(long)]
        check: bool,
    },
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Publish {
            descriptor,
            profiles,
        } => {
            let read = |path: &PathBuf| {
                std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
            };
            let descriptor = serde_json::from_slice(&read(&descriptor)?)
                .map_err(|error| format!("descriptor: {error}"))?;
            let declarations = Declarations::from_json(&read(&profiles)?)?;
            let publication = publish(&descriptor, &declarations).map_err(|refusals| {
                refusals
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            })?;
            print!(
                "{}",
                to_yaml(&publication.contract, &profiles.display().to_string())?
            );
            Ok(())
        }
        Command::Generate { root, check } => {
            for adapter in ADAPTERS {
                let bytes = generate(&root, adapter)?;
                let path = root.join(generated_path(adapter));
                if check {
                    let committed = std::fs::read_to_string(&path)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    if committed != bytes {
                        return Err(format!(
                            "{}: generated tool contract drift; run connectors-tool-contracts generate",
                            generated_path(adapter)
                        ));
                    }
                    println!("{adapter}: tool contract matches");
                } else {
                    std::fs::write(&path, bytes)
                        .map_err(|error| format!("{}: {error}", path.display()))?;
                    println!("{adapter}: wrote {}", generated_path(adapter));
                }
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
