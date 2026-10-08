//! The argument definitions of the explicit service commands `describe`, `invoke` and
//! `serve`. The binary parses them; the documentation generator walks them for the CLI
//! reference, so the reference cannot describe an option the binary does not have.
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// The one-line purpose of each explicit service command, in the order the root help lists
/// them below the generated commands.
pub const SUMMARIES: [(&str, &str); 3] = [
    ("describe", "Read a complete service descriptor"),
    ("invoke", "Invoke an explicit service operation"),
    ("serve", "Run the configured federation service"),
];

#[derive(Parser)]
#[command(about = "Discover and invoke independently hosted connector contracts")]
pub struct Args {
    /// Output for parse refusals; successful results stay raw JSON.
    #[arg(long, global = true, value_parser = ["human", "json"], default_value = "human")]
    pub output: String,
    #[command(subcommand)]
    pub command: Command,
}
#[derive(Subcommand)]
pub enum Command {
    Describe {
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        token_file: PathBuf,
        #[arg(long)]
        allow_plaintext: bool,
    },
    Invoke {
        #[arg(long)]
        endpoint: String,
        #[arg(long)]
        token_file: PathBuf,
        #[arg(long)]
        allow_plaintext: bool,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        input: PathBuf,
    },
    Serve {
        #[arg(long)]
        config: PathBuf,
    },
}
