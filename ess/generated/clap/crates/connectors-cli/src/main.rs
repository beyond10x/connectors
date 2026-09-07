// generated from connectors v1
// model digest 28b7f4588abf0f2922e31dd0e2731da2dedc3b13e15798da7658566d5e9f489c
// contract digest dad28d01b32983092de2e0ba4fd965b5c88a7c200d85f9185debc6d844d8d838
// do not edit: regenerate with `cargo xtask synth --target clap`


//! The binary: parse the tree, answer `completions` from it, dispatch the rest.

mod handler;
mod tree;

pub use self::handler::{Handler, Unimplemented};

fn main() -> ::std::process::ExitCode {
    let matches = self::tree::command().get_matches();
    if let Some(completions) = matches.subcommand_matches("completions") {
        let shell = *completions
            .get_one::<::clap_complete::Shell>("shell")
            .expect("the shell is required");
        let mut command = self::tree::command();
        let name = command.get_name().to_owned();
        ::clap_complete::generate(shell, &mut command, name, &mut ::std::io::stdout());
        return ::std::process::ExitCode::SUCCESS;
    }
    dispatch(&Unimplemented, &matches)
}

/// Routes one parsed invocation to the handler that owes it.
///
/// Exhaustive over the tree by construction: every arm is a command the `cli:` block places,
/// and a command it places nowhere is a specification `ess validate` refuses.
fn dispatch<H: Handler>(handler: &H, matches: &::clap::ArgMatches) -> ::std::process::ExitCode {
    match matches.subcommand() {
        Some(("setup", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("inspect", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("session", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("serve", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("daemon", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("endpoint", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("connection", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("event", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("operation", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("admin", sub)) => match sub.subcommand() {
            _ => ::std::process::ExitCode::FAILURE,
        },
        Some(("completions", _)) => ::std::process::ExitCode::SUCCESS,
        _ => ::std::process::ExitCode::FAILURE,
    }
}
