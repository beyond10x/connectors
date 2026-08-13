//! `catalog` — the repository-maintenance binary.
//!
//! ```text
//! catalog build | diff | check | scaffold
//! ```
//!
//! **This is not the product's CLI, and it deliberately does not take its name.** `connectors` is
//! the user-facing binary of design 02 §2 — `serve`, `admin`, the client verbs — and it arrives
//! with M2. This one exists so that the catalogue can be rebuilt and reviewed *here*: it is a
//! workspace member, never a release artifact, and the only thing it knows how to do is compile
//! committed text into the catalog artifacts.
//!
//! # The surface is a declaration, not a parser
//!
//! Argument parsing is clap's derive API and nothing else. Hand-rolled argv parsing is banned in
//! this repository: the predecessor hand-rolled its own, and the cost was that the surface's rules
//! — "`--select` is `scaffold` only" — lived in `match` guards, with `--help` maintained by hand
//! beside them and free to drift. Here the verbs *are* the enum, the flags *are* the fields, and
//! `--help` is generated from them.
//!
//! Everything the binary can do is a library call over [`catalog_build::cli::Invocation`], so an
//! integration test drives the same code path `main` does.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

/// Compile the reviewed connector declarations into the catalog artifacts.
///
/// Every verb is hermetic and offline: it reads committed bytes and contacts no vendor.
#[derive(Debug, Parser)]
#[command(
    name = "catalog",
    version,
    about = "Compile the reviewed connector declarations into the catalog artifacts",
    long_about = "The repository-maintenance CLI for b10x/connectors.\n\n\
                  Every verb is hermetic and offline: it reads committed bytes and contacts no \
                  vendor. `scaffold` writes to stdout and never over a file in place — the author \
                  diffs and pastes, so a bad run costs nothing."
)]
struct Cli {
    #[command(subcommand)]
    command: Verb,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// Compile providers/*.toml plus the vendored spec cache into catalog/, the catalog pack,
    /// connectors.lock and the site projection.
    Build {
        #[command(flatten)]
        scope: Scope,
    },
    /// Show what `build` would change, without writing anything.
    Diff {
        #[command(flatten)]
        scope: Scope,
    },
    /// Recompute every hash in connectors.lock and refuse on drift.
    Check {
        /// Repository root (default: the current directory).
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
    /// Write the provider TOML that references a vendored document, to stdout.
    Scaffold {
        /// The connector to scaffold.
        ///
        /// Positional because a scaffold of nothing in particular has no meaning: it is not a
        /// *restriction* on a whole-catalogue run the way `build --provider` is.
        provider: String,

        /// One `[[patch.select]]` statement, repeatable, in the order written.
        ///
        /// Spelled `<service>:<path_prefix>:<METHOD,METHOD>` — for example
        /// `manager:/api/v2/agents:GET,POST`. Fields may be dropped from the right
        /// (`manager:/api/v2`, `manager`) and left empty (`:/api/v2:GET`); an empty field states
        /// nothing. Without any `--select`, every document is selected whole, split by method
        /// class.
        #[arg(long = "select", value_name = "SEL")]
        selects: Vec<String>,

        /// Report what the documents declare that this connector does not publish, and the
        /// reverse, instead of emitting TOML.
        #[arg(long)]
        diff: bool,

        /// Repository root (default: the current directory).
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
}

/// How much of the catalogue a run covers.
///
/// One flattened group rather than three repeated fields: the two selectors and the root mean the
/// same thing to every verb that compiles, and a group is how clap says that once.
#[derive(Debug, Args)]
struct Scope {
    /// Restrict the run to one connector.
    #[arg(long, short = 'p', value_name = "NAME")]
    provider: Option<String>,

    /// Restrict the run to one whole service of that connector.
    ///
    /// By service name (`s3`) or by its address (`com.amazonaws/s3:2006-03-01`). A provider with a
    /// single API surface has one service, `default`, and needs no flag.
    #[arg(long, short = 's', value_name = "NAME")]
    service: Option<String>,

    /// Repository root (default: the current directory).
    #[arg(long, value_name = "DIR")]
    root: Option<PathBuf>,
}

impl From<Cli> for catalog_build::cli::Invocation {
    fn from(cli: Cli) -> Self {
        use catalog_build::cli::{Command, Invocation};
        match cli.command {
            Verb::Build { scope } => Invocation {
                command: Command::Build,
                ..scope.into()
            },
            Verb::Diff { scope } => Invocation {
                command: Command::Diff,
                ..scope.into()
            },
            Verb::Check { root } => Invocation {
                command: Command::Check,
                root,
                ..Default::default()
            },
            Verb::Scaffold {
                provider,
                selects,
                diff,
                root,
            } => Invocation {
                command: Command::Scaffold,
                root,
                provider: Some(provider),
                service: None,
                selects,
                diff,
            },
        }
    }
}

impl From<Scope> for catalog_build::cli::Invocation {
    fn from(scope: Scope) -> Self {
        catalog_build::cli::Invocation {
            root: scope.root,
            provider: scope.provider,
            service: scope.service,
            ..Default::default()
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    match catalog_build::run(&cli.into(), &mut out) {
        Ok(()) => {
            // A failure to flush is a failure to produce the output the exit code claims: a broken
            // pipe is ordinary, and anything else means the caller did not get what we said we
            // wrote.
            if let Err(error) = out.flush() {
                if error.kind() != io::ErrorKind::BrokenPipe {
                    eprintln!("catalog: {error}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ = out.flush();
            // `{:#}` prints the whole `anyhow` context chain on one line — the provider a refusal
            // came from, then the refusal.
            eprintln!("catalog: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog_build::cli::{Command, Invocation};
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Invocation, clap::Error> {
        Cli::try_parse_from(std::iter::once("catalog").chain(args.iter().copied()))
            .map(Invocation::from)
    }

    /// clap's own contract check: an ambiguous or malformed declaration is a panic at startup, and
    /// this is the one call that surfaces it in a test rather than in a user's terminal.
    #[test]
    fn the_declared_surface_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_verb_maps_to_its_command() {
        for (args, command) in [
            (vec!["build"], Command::Build),
            (vec!["diff"], Command::Diff),
            (vec!["check"], Command::Check),
            (vec!["scaffold", "babelforce"], Command::Scaffold),
        ] {
            assert_eq!(parse(&args).expect("the verb parses").command, command);
        }
    }

    #[test]
    fn the_scope_flags_reach_the_invocation() {
        let invocation = parse(&["build", "--provider", "zendesk", "--root", "/tmp/x"])
            .expect("build takes a scope");
        assert_eq!(invocation.provider.as_deref(), Some("zendesk"));
        assert_eq!(
            invocation.root.as_deref(),
            Some(std::path::Path::new("/tmp/x"))
        );
        assert_eq!(
            parse(&["build", "-p", "zendesk"])
                .unwrap()
                .provider
                .as_deref(),
            Some("zendesk")
        );
    }

    /// The surface's rules are the *types*, not `match` guards: `--select` and `--diff` exist on
    /// `scaffold` alone, and a provider is required there rather than optional.
    #[test]
    fn selection_belongs_to_scaffold_alone() {
        assert!(parse(&["build", "--select", "manager"]).is_err());
        assert!(parse(&["diff", "--diff"]).is_err());
        assert!(
            parse(&["scaffold"]).is_err(),
            "scaffold requires a provider"
        );

        let invocation = parse(&[
            "scaffold",
            "babelforce",
            "--select",
            "manager:/v2:GET",
            "--select",
            "user",
            "--diff",
        ])
        .expect("scaffold takes a repeated selection");
        assert_eq!(invocation.provider.as_deref(), Some("babelforce"));
        assert_eq!(invocation.selects, ["manager:/v2:GET", "user"]);
        assert!(invocation.diff);
    }

    /// The predecessor's binary carried verbs this one does not, and the platform's verbs do not
    /// live here at all. Each is a parse failure rather than a silent success.
    #[test]
    fn a_verb_outside_this_surface_is_refused() {
        for verb in ["fetch", "install", "migration-check", "serve", "admin"] {
            assert!(
                parse(&[verb]).is_err(),
                "`{verb}` is not in this surface and must not parse"
            );
        }
    }

    /// `check` is whole-catalogue: a partial verification could miss the deleted row it exists to
    /// detect and then report a truncated lock clean.
    #[test]
    fn check_accepts_no_scope() {
        assert!(parse(&["check", "--provider", "zendesk"]).is_err());
        assert!(parse(&["check", "--service", "default"]).is_err());
        assert_eq!(
            parse(&["check", "--root", "/tmp/x"])
                .expect("check accepts a workspace root")
                .root
                .as_deref(),
            Some(std::path::Path::new("/tmp/x"))
        );
    }
}
