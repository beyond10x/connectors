// generated from connectors v1
// model digest 5849d9d17106bb6c8f9f7331f9c1fcf8d8addd0506851273b950db073eacb81b
// contract digest ec905bc6d63ad79ca30a539c0a27a4d8c83bd41b4d0909594faa9ae6f34ab91e
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
