// generated from connectors v1
// model digest 9465040634bb366dd25958f1cdc7a6f96cf15eb7beffcdeac76eb2d1f9506c51
// contract digest 04fcd536a3c100325904181a779615bd4f192f2ce7fcefcca5f8fd46ddd2b362
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
            ::clap::Command::new("admin")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Operate an Identity-protected hosted Connectors instance."),
        )
        .subcommand(
            ::clap::Command::new("auth")
                .subcommand_required(true)
                .arg_required_else_help(true)
                .about("Which configured providers have their credential stored. Never reads one."),
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
