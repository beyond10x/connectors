// generated from connectors v1
// model digest ac9b84c2eccec357eb976d81eeb8dfdbf64fd211a4d1c5041cba9fc16aaae926
// contract digest 8aa4ca77c6867708024a981f32eeb60a71b924b35238d873e582e86fbf6858f5
// do not edit: regenerate with `cargo xtask synth --target clap`


//! The command tree, as the specification declares it.

/// The whole grammar: every group, every command, and every flag a command's input declares.
#[must_use]
pub fn command() -> ::clap::Command {
    ::clap::Command::new("connectors")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .about("Names which deployment a command reaches, instead of inferring it from a stored login.")
        .subcommand(
            ::clap::Command::new("setup")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Write a configuration, add a provider, install shell completions."),
        )
        .subcommand(
            ::clap::Command::new("inspect")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Read what is configured, what is connected, and what cannot work."),
        )
        .subcommand(
            ::clap::Command::new("session")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Sign in to a hosted Connector deployment, and sign out of it."),
        )
        .subcommand(
            ::clap::Command::new("serve")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Run a Connector for this machine, for a hosted deployment, or over stdio."),
        )
        .subcommand(
            ::clap::Command::new("connection")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Manage durable Connections through the credential-free control socket."),
        )
        .subcommand(
            ::clap::Command::new("event")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Search or receive durable normalized data events."),
        )
        .subcommand(
            ::clap::Command::new("operation")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Search, describe, or invoke admitted Connector operations."),
        )
        .subcommand(
            ::clap::Command::new("admin")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Operate an Identity-protected hosted Connectors instance."),
        )
        .subcommand(
            ::clap::Command::new("completions")
                .about("Print a completion script for one shell, from this same command tree")
                .arg(
                    ::clap::Arg::new("shell")
                        .required(true)
                        .value_parser(
                            ::clap::builder::EnumValueParser::<::clap_complete::Shell>::new(),
                        ),
                ),
        )
}
