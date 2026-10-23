//! Production stdio composition. Only complete MCP frames cross stdout.
use super::*;
use clap::{CommandFactory, FromArgMatches, Parser};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(name = "connectors", disable_help_subcommand = true)]
struct Selectors {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long = "state-dir", global = true)]
    state: Option<PathBuf>,
}
pub(crate) fn selected(args: &[OsString]) -> bool {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--config" | "--state-dir" | "--output") => {
                rest.next();
            }
            Some(a)
                if a.starts_with("--config=")
                    || a.starts_with("--state-dir=")
                    || a.starts_with("--output=") => {}
            Some("server") => return true,
            _ => return false,
        }
    }
    false
}
pub(crate) fn run(args: Vec<OsString>) -> i32 {
    let result = (|| {
        let matches = match Selectors::command()
            .subcommand_required(true)
            .subcommand(connectors_mcp::launch::command())
            .try_get_matches_from(args)
        {
            Ok(matches) => matches,
            Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
                io::stdout()
                    .write_all(error.to_string().as_bytes())
                    .map_err(|_| ())?;
                return Ok(());
            }
            Err(_) => return Err(()),
        };
        let selectors = Selectors::from_arg_matches(&matches).map_err(|_| ())?;
        connectors_mcp::launch::input(matches.subcommand_matches("server").ok_or(())?)
            .map_err(|_| ())?;
        let paths = Paths::resolve(selectors.config.as_deref(), selectors.state.as_deref())
            .map_err(|_| ())?;
        serve(paths)
    })();
    if result.is_ok() {
        0
    } else {
        let _ = writeln!(io::stderr(), "mcp_unavailable");
        1
    }
}

#[cfg(target_os = "linux")]
fn serve(paths: Paths) -> Result<(), ()> {
    let signals = connectors_host::local::protected::Signals::install().map_err(|_| ())?;
    super::stdio::serve(paths, &signals).map_err(|_| ())
}

/// The stdio server's data lease runs on Linux's boot clock; elsewhere a valid
/// launch is refused before any protocol byte, like any unavailable server.
#[cfg(not(target_os = "linux"))]
fn serve(_paths: Paths) -> Result<(), ()> {
    Err(())
}
