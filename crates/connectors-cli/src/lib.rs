#![forbid(unsafe_code)]

//! The Connectors command line, as a library.
//!
//! It is a library because it has two front doors: the `connectors` binary, and `zwirn connectors …`
//! in a Zwirn build that carries the local placement. One implementation, so the alias cannot drift
//! from the tool it aliases — a second parser would be a second product with the same name.

mod error;
use error::MainError;

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use connectors_client::{AuthenticatedHostedClient, IdentityError, LocalClient, LoginOptions};
use connectors_runtime::{
    default_config_path, default_state_root, local_socket_absent, validate_state_root,
    HostedRuntime, OneShotOperationV3Outcome, PersonalConfig, PersonalRuntime, RuntimeError,
};
use protocol::connection::{
    CandidateActivateRequest, CandidateSearchRequest, ConnectionRequest, MaterializeRequest,
    ObservationSearchRequest, SearchRequest as ConnectionSearchRequest,
};
use protocol::event::{
    EventRequest, ReceiveRequest, ReplayRequest, SearchRequest as EventSearchRequest,
};
use protocol::operation::{
    DescribeRequest as OperationDescribeRequest, InvokeRequest as OperationInvokeRequest,
    OperationRequest, SearchRequest as OperationSearchRequest,
};

use connectors_console::{
    admin, auth, connect, doctor, enrol, init, input, output, reduce_envelope, remediation, Format,
};

#[derive(Debug, Parser)]
#[command(name = "connectors", version, about = "b10x Connectors service")]
struct Cli {
    /// How results are rendered. `json` and `yaml` also carry failures on stdout, so a pipe reads
    /// the refusal instead of an empty stream.
    #[arg(long, short = 'o', value_enum, default_value_t = Format::Text, global = true)]
    output: Format,
    #[command(subcommand)]
    command: Command,
}

/// The parser's own command tree, built once and read by everything that needs the surface rather
/// than a parse: `connectors setup completions <shell>` renders it, and `tests/cli_surface.rs`
/// holds it
/// against the `cli:` block of `ess/system/components.yaml`.
#[must_use]
pub fn command() -> clap::Command {
    Cli::command()
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write a configuration, add a provider, install shell completions.
    Setup {
        #[command(subcommand)]
        command: SetupCommand,
    },
    /// Read what is configured, what is connected, and what cannot work.
    Inspect {
        #[command(subcommand)]
        command: InspectCommand,
    },
    /// Sign in to a hosted Connector deployment, and sign out of it.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Run a Connector for this machine, for a hosted deployment, or over stdio.
    Serve {
        #[command(subcommand)]
        command: ServeCommand,
    },
    /// Manage durable Connections through the credential-free control socket.
    Connection {
        /// Deployment to reach. A saved login never changes the local default.
        #[arg(long, value_enum, default_value_t = Target::Local, global = true)]
        target: Target,
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    /// Search or receive durable normalized data events.
    Event {
        /// Deployment to reach. A saved login never changes the local default.
        #[arg(long, value_enum, default_value_t = Target::Local, global = true)]
        target: Target,
        #[command(subcommand)]
        command: EventCommand,
    },
    /// Search, describe, or invoke admitted Connector operations.
    Operation {
        /// Deployment to reach. A saved login never changes the local default.
        #[arg(long, value_enum, default_value_t = Target::Local, global = true)]
        target: Target,
        /// Exact operation contract to send. There is no negotiation or fallback.
        #[arg(long, value_enum, default_value_t = OperationVersion::V3, global = true)]
        protocol_version: OperationVersion,
        #[command(subcommand)]
        command: OperationCommand,
    },
    /// Operate an Identity-protected hosted Connectors instance.
    Admin(admin::CommandOptions),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum OperationVersion {
    V2,
    V3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Target {
    Local,
    Hosted,
}

impl Target {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Hosted => "hosted",
        }
    }

    fn validate(self, config: &Option<PathBuf>, state: &Option<PathBuf>) -> Result<(), MainError> {
        if self == Self::Hosted && (config.is_some() || state.is_some()) {
            return Err(MainError::TargetConflict);
        }
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum SetupCommand {
    /// Write a usable configuration for this machine, so nothing has to be authored by hand.
    Init {
        /// Where to write. Defaults below XDG_CONFIG_HOME.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Owner-only state root it will be served with.
        #[arg(long)]
        state_root: Option<PathBuf>,
        /// Integration to declare. Repeatable. Omit to admit whatever this machine supports.
        #[arg(long = "integration", value_enum)]
        integrations: Vec<init::Integration>,
        /// Admit kubeconfig contexts authenticated by a credential plugin. Every EKS context is one.
        #[arg(long)]
        allow_exec_auth: bool,
        /// Replace an existing configuration.
        #[arg(long)]
        force: bool,
    },
    /// Add a provider through one guided, secret-safe flow.
    Connect {
        /// Deployment to reach. Hosted token setup uses the current Identity login.
        #[arg(long, value_enum, default_value_t = Target::Local)]
        target: Target,
        /// Provider to add. `connectors inspect providers` lists every one the catalogue declares.
        #[arg(required_unless_present = "operation", conflicts_with = "operation")]
        provider: Option<String>,
        /// Strict value-free deployment configuration.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Human label for the resulting Connection.
        #[arg(long)]
        label: Option<String>,
        /// Exact detected kubeconfig context when connecting Kubernetes.
        #[arg(long)]
        context: Option<String>,
        /// Owner-only state root used by the running Connector.
        #[arg(long)]
        state_root: Option<PathBuf>,
        /// Which declared credential to supply.
        #[arg(long = "as")]
        credential: Option<String>,
        #[command(flatten)]
        oauth: Box<PersonalOAuthArgs>,
        /// A declared configuration value, as `field=value`. Repeatable.
        #[arg(long = "set", value_parser = enrol::parse_setting)]
        settings: Vec<(String, String)>,
        /// Admit write operations. Reads only without it.
        #[arg(long = "allow", value_parser = ["writes"])]
        allow: Option<String>,
        /// Admit private destinations, for a self-hosted instance on your own network.
        #[arg(long)]
        operator_network: bool,
        /// Read the credential from an owner-only file rather than prompting.
        /// Hosted mode supports declared catalog token profiles with one secret entry;
        /// browser OAuth and native flows with multiple fields keep their acquisition routes.
        #[arg(long)]
        credential_file: Option<PathBuf>,
        /// Which instance, when this placement holds one provider more than once.
        #[arg(long)]
        instance: Option<String>,
    },
    /// Print a completion script for one shell, generated from this same command tree.
    ///
    /// Install it where the shell reads completions at start-up, for example
    /// `connectors setup completions fish > ~/.config/fish/completions/connectors.fish` or
    /// `connectors setup completions bash > ~/.local/share/bash-completion/completions/connectors`.
    Completions {
        /// Which shell's syntax to print.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, clap::Args)]
struct PersonalOAuthArgs {
    /// Exact hosted token profile, or the configured personal OAuth credential purpose.
    #[arg(long, conflicts_with = "credential")]
    auth_profile: Option<String>,
    /// Private OAuth instructions in a new owner-only file; otherwise use the controlling terminal.
    #[arg(long)]
    instruction_file: Option<PathBuf>,
    /// Repair the configured Connection for this admitted operation; never invoke it automatically.
    #[arg(long, requires_all = ["connection", "remediation-input"], conflicts_with_all = ["provider", "label", "context", "credential", "auth_profile", "settings", "allow", "operator_network", "credential_file", "instance"])]
    operation: Option<String>,
    /// Exact configured Connection to repair, paired with --operation.
    #[arg(long, requires = "operation")]
    connection: Option<String>,
    /// Intended input, checked again against the fresh operation schema after acknowledgement.
    #[arg(long, group = "remediation-input", requires = "operation")]
    input_json: Option<String>,
    /// Read the bounded intended input from a file.
    #[arg(long, group = "remediation-input", requires = "operation")]
    input_file: Option<PathBuf>,
    /// Read bounded intended input from stdin; the only accepted value is `-`.
    #[arg(long, group = "remediation-input", requires = "operation", value_parser = ["-"])]
    input: Option<String>,
}

#[derive(Debug, Subcommand)]
enum InspectCommand {
    /// What is configured, what is running, and what cannot work.
    Doctor {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// What every catalogued provider needs before it can be connected.
    Providers {
        /// Narrow to providers whose id or vendor contains this text.
        #[arg(long, default_value = "")]
        query: String,
    },
    /// Which configured providers have their credential stored. Never reads one.
    Auth {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    /// Sign in through this hosted Connector deployment's Identity authority.
    Login {
        /// Exact hosted Connector API base.
        base: String,
        /// Print the authorization URL instead of opening the system browser.
        #[arg(long)]
        no_browser: bool,
        /// Maximum time to wait for the loopback browser callback.
        #[arg(long, default_value_t = 300)]
        timeout_seconds: u64,
    },
    /// Remove the active hosted Connector login from local state and the OS keyring.
    Logout,
}

#[derive(Debug, Subcommand)]
enum ServeCommand {
    /// Serve the owner-permissioned personal-local Connector protocols.
    Local {
        /// Strict value-free deployment configuration.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Owner-only state root. Defaults below XDG_STATE_HOME (or ~/.local/state).
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Serve the Identity-authenticated hosted Operation and Connection APIs.
    Hosted {
        /// Strict value-free server and Integration configuration.
        ///
        /// Required, unlike every personal-local command: a hosted deployment's configuration is
        /// installed by whoever operates it, and defaulting to a path in the invoking user's home
        /// directory would be the wrong file every time.
        #[arg(long)]
        config: PathBuf,
    },
    /// Serve the active hosted Connector MCP endpoint over local stdio.
    Mcp,
}

#[derive(Debug, Subcommand)]
enum ConnectionCommand {
    /// Passively list potential direct Connections without contacting their providers.
    Candidates {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        integration: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(
            long,
            default_value_t = protocol::connection::MAX_SEARCH_RESULTS,
            value_parser = clap::value_parser!(u16).range(1..=i64::from(protocol::connection::MAX_SEARCH_RESULTS)),
            help = format!("Maximum results (1..={})", protocol::connection::MAX_SEARCH_RESULTS)
        )]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Explicitly contact and activate one opaque direct-Connection candidate.
    Activate {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        candidate: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// List non-secret Connection summaries.
    List {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(
            long,
            default_value_t = protocol::connection::MAX_SEARCH_RESULTS,
            value_parser = clap::value_parser!(u16).range(1..=i64::from(protocol::connection::MAX_SEARCH_RESULTS)),
            help = format!("Maximum results (1..={})", protocol::connection::MAX_SEARCH_RESULTS)
        )]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// List the latest stored discovery observations for a source Connection.
    Observations {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        source: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(
            long,
            default_value_t = protocol::connection::MAX_SEARCH_RESULTS,
            value_parser = clap::value_parser!(u16).range(1..=i64::from(protocol::connection::MAX_SEARCH_RESULTS)),
            help = format!("Maximum results (1..={})", protocol::connection::MAX_SEARCH_RESULTS)
        )]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Materialize one recognized observation as a callable mediated Connection.
    Materialize {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum OperationCommand {
    /// List currently callable operations and their admitted Connections.
    Search {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(
            long,
            default_value_t = protocol::operation::MAX_SEARCH_RESULTS,
            value_parser = clap::value_parser!(u16).range(1..=i64::from(protocol::operation::MAX_SEARCH_RESULTS)),
            help = format!("Maximum results (1..={})", protocol::operation::MAX_SEARCH_RESULTS)
        )]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Read a fresh operation description and its opaque lease.
    Describe {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Send DTMF into a session that is already established.
    Signal {
        #[arg(long)]
        config: Option<PathBuf>,
        /// The `execution_ref` the invocation that placed the call returned.
        #[arg(long)]
        execution_ref: String,
        /// Digits to send, from `0123456789*#ABCD`.
        #[arg(long)]
        dtmf: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Invoke an operation using a fresh description lease.
    Invoke {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        connection: String,
        #[arg(long)]
        description_ref: String,
        /// Strict JSON object of catalog-declared caller input. See also --input-file and --input.
        #[arg(long, group = "caller-input")]
        input_json: Option<String>,
        /// Read the input object from a file.
        #[arg(long, group = "caller-input")]
        input_file: Option<PathBuf>,
        /// Read the input object from stdin. The only accepted value is `-`.
        #[arg(long, group = "caller-input")]
        input: Option<String>,
        #[arg(long)]
        approval_evidence_ref: Option<String>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum EventCommand {
    /// List admitted Connector channels.
    Search {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(
            long,
            default_value_t = protocol::event::MAX_SEARCH_RESULTS,
            value_parser = clap::value_parser!(u16).range(1..=i64::from(protocol::event::MAX_SEARCH_RESULTS)),
            help = format!("Maximum results (1..={})", protocol::event::MAX_SEARCH_RESULTS)
        )]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Pull durable events, optionally waiting for the next one.
    Receive {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        after: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u16,
        #[arg(long, default_value_t = 30_000)]
        wait_ms: u32,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Replay one stored event by its opaque Connector event reference.
    Replay {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        event: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

/// **Old top-level paths, and where each moved.** Applied by [`moved`] before clap is handed the
/// argv, and removed one release after `story:cli-first-level-groups`.
///
/// `auth` is here twice on purpose. It was a first-level group whose one command moved, so both the
/// group a person typed and the two-word path they typed under it land on `connectors inspect
/// auth`. `serve` is both a word that moved and the group it moved into: its row fires only when
/// what follows it is an option the group refuses and `serve local` declares — `--config`,
/// `--state-root` — so a bare `serve`, `serve -o json`, `serve --help`, `serve local` and a
/// `serve --bogus` are the group's own, exactly as they are for `setup`, `inspect` and `session`.
///
/// Public because a copy of it drifted: an adversary suite restated eleven rows against the twelve
/// shipped, and the row it lacked was the one its cases therefore never drove. The tests read this.
pub const MOVED: &[(&[&str], &[&str])] = &[
    (&["auth", "status"], &["inspect", "auth"]),
    (&["auth"], &["inspect", "auth"]),
    (&["completions"], &["setup", "completions"]),
    (&["connect"], &["setup", "connect"]),
    (&["doctor"], &["inspect", "doctor"]),
    (&["init"], &["setup", "init"]),
    (&["login"], &["session", "login"]),
    (&["logout"], &["session", "logout"]),
    (&["mcp"], &["serve", "mcp"]),
    (&["providers"], &["inspect", "providers"]),
    (&["serve-hosted"], &["serve", "hosted"]),
    (&["serve"], &["serve", "local"]),
];

/// How many leading words of argv are looked at before deciding the table cannot apply.
///
/// A word that moved stands behind at most `help` and the root's global options, each one or two
/// words, so it is within the first few; one further along is under a group of this release and
/// is clap's. Generous rather than exact: a false hit costs one parse, a miss would be a path the
/// promise covers and this does not.
const LEGACY_WINDOW: usize = 8;

/// One read of a legacy argv, as the release before this one would have read it: the root's global
/// options in front, then one word, then everything after that word, verbatim.
///
/// A clap tree of the real global options and nothing else, with the first word that is not one
/// of them admitted as an external subcommand — which is how clap hands back the word *and* the
/// tail untouched. It has no positional on purpose: a positional that admits hyphenated values
/// makes clap read `-ojson` as one, because `j` is no short option, and the four spellings of the
/// format flag stop being one case. Returns the global options as they were read, re-spelt
/// `--long value` for the parse that follows, the word, and the tail; `None` when clap refuses the
/// front of the argv, which leaves it to the real parse to refuse.
///
/// `None`, too, when the front holds the escape `--`. A word behind it is a value, not a path:
/// the release before this one refused `connectors -- doctor` exactly as this one refuses
/// `connectors -- inspect`. Clap consumes the escape without recording it and, with no positional
/// to hand what follows to, admits the word behind it as the external subcommand all the same — a
/// `last(true)` positional would not help, because clap refuses every word in front of the escape
/// before the external subcommand is reached. So the escape is looked for among the tokens clap
/// took in front of the word: the tail is the argv after the word verbatim, so those are
/// everything before it. This is the one token read here without clap, and it is clap's own.
fn read_one_word(tree: &clap::Command, words: &[OsString]) -> Option<ReadWord> {
    let globals: Vec<&clap::Arg> = tree
        .get_arguments()
        .filter(|argument| argument.is_global_set())
        .collect();
    let mut reader = clap::Command::new("connectors")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .allow_external_subcommands(true)
        .external_subcommand_value_parser(clap::value_parser!(OsString));
    for argument in &globals {
        reader = reader.arg((*argument).clone());
    }
    let matches = reader
        .try_get_matches_from(
            std::iter::once(OsString::from("connectors")).chain(words.iter().cloned()),
        )
        .ok()?;

    let mut read = Vec::new();
    for argument in globals {
        let id = argument.get_id().as_str();
        if matches.value_source(id) != Some(clap::parser::ValueSource::CommandLine) {
            continue;
        }
        let flag = match (argument.get_long(), argument.get_short()) {
            (Some(long), _) => OsString::from(format!("--{long}")),
            (None, Some(short)) => OsString::from(format!("-{short}")),
            (None, None) => continue,
        };
        if argument.get_action().takes_values() {
            for value in matches.get_raw(id).into_iter().flatten() {
                read.push(flag.clone());
                read.push(value.to_os_string());
            }
        } else {
            read.push(flag);
        }
    }
    let word = matches.subcommand().map(|(word, inner)| {
        let tail: Vec<OsString> = inner
            .get_many::<OsString>("")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();
        (word.to_owned(), tail)
    });
    if let Some((_, tail)) = &word {
        let front = &words[..words.len() - tail.len() - 1];
        if front.iter().any(|token| token == "--") {
            return None;
        }
    }
    Some((read, word))
}

type ReadWord = (Vec<OsString>, Option<(String, Vec<OsString>)>);

/// The row of [`MOVED`] the words walked so far name: the longest whose old path they begin with.
fn row(walked: &[&str]) -> Option<(&'static [&'static str], &'static [&'static str])> {
    MOVED
        .iter()
        .copied()
        .filter(|(old, _)| {
            old.len() <= walked.len() && old.iter().zip(walked).all(|(word, given)| word == given)
        })
        .max_by_key(|(old, _)| old.len())
}

/// This release's command at the new path of the words walked so far, once there is a row for
/// them: `inspect auth` for `auth`, `session login` for `login`.
fn at_new_path<'a>(tree: &'a clap::Command, walked: &[&str]) -> Option<&'a clap::Command> {
    let (old, new) = row(walked)?;
    new.iter()
        .chain(&walked[old.len()..])
        .try_fold(tree, |command, word| command.find_subcommand(word))
}

/// Rewrite one legacy path, and report the pair so the caller can name where it went.
///
/// Nothing here reads an option's syntax. Clap decides everything about argv, and each parse it
/// performs answers one question:
///
/// 1. **Is a word that moved even present?** None of the first [`LEGACY_WINDOW`] words is the
///    first word of a `MOVED` row: the argv is this release's, and no tree is built.
/// 2. **Does this release already read it?** The real tree is asked to parse the argv. Only two
///    refusals mean "a word that moved": an unrecognized subcommand — `doctor`, `auth status`,
///    `help doctor` — and an unexpected argument, which is `serve --config` at the group that
///    took the old leaf's name. Anything else is clap's to answer as it stands: a parse, a help or
///    version request, a missing value. So a bare `serve` prints the group, as a bare `setup`
///    does, and `help serve` prints it too.
/// 3. **Which path was typed?** [`read_one_word`] reads the argv one word at a time, each read
///    stepping over the global options in front of that word. A word is taken while it is `help`,
///    or continues a row of the table from the words already taken; the first that does neither
///    ends the path, and the tail after the last word taken is kept verbatim. `help` is taken in
///    front of a path, and after a word whose new leaf takes no positional — `auth help` — but
///    not after one whose new leaf does: `login help` is a login to the base named `help`, as it
///    was. A word behind the escape `--` is never taken; the reader says so. The argv handed on
///    is rebuilt from that: the globals as clap read them, `help` if it was there, the new words,
///    the tail. `auth -o json status --help`, `auth --output=json status --help`, `-ojson` and
///    `--output json` are one case, because clap's option syntax is clap's.
///
/// A path this release reads that the real parse could not accept for a reason *behind* the word is
/// still left alone. `serve` is the one word that is both a row and a group of this release, so
/// its row fires only when the first word of the tail is an option its new leaf declares, and that
/// is asked of the built leaf: `serve --config x` is the old leaf, because the group refused
/// `--config` and `serve local` takes it; `serve local --bogus` and `serve foo` are the group's,
/// because a word after a group's name is the group's to answer; `serve --hlep` and `serve -V` are
/// the group's, because `serve local` refuses them exactly as the group does. The tree is built
/// first, because a global option reaches a leaf, and `help` becomes a subcommand of a group,
/// only after `clap::Command::build`.
///
/// Public so that `tests/moved_paths_are_not_taught.rs` asks this exact function whether a written
/// invocation names a path that moved, rather than restating its rules.
pub fn moved(
    arguments: &mut Vec<OsString>,
) -> Option<(&'static [&'static str], &'static [&'static str])> {
    let names_a_legacy_word =
        |word: &OsString| MOVED.iter().any(|(old, _)| word.as_os_str() == old[0]);
    if !arguments
        .iter()
        .skip(1)
        .take(LEGACY_WINDOW)
        .any(names_a_legacy_word)
    {
        return None;
    }

    match command().try_get_matches_from(arguments.iter()) {
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::InvalidSubcommand | clap::error::ErrorKind::UnknownArgument
            ) => {}
        _ => return None,
    }

    let mut tree = command();
    tree.build();

    // The path, one word per read: `help` and up to the longest row, plus one word past it for
    // the guard below, which needs to see what followed a group's name.
    let longest = MOVED.iter().map(|(old, _)| old.len()).max().unwrap_or(0);
    let mut globals: Vec<OsString> = Vec::new();
    let mut walked: Vec<&'static str> = Vec::new();
    let mut under_help = false;
    let mut tail: Vec<OsString> = arguments[1..].to_vec();
    let mut next: Option<String> = None;
    for _ in 0..longest + 2 {
        let Some((read, word)) = read_one_word(&tree, &tail) else {
            break;
        };
        let Some((word, after)) = word else {
            break;
        };
        let continues = MOVED.iter().find_map(|(old, _)| {
            (old.len() > walked.len()
                && old[..walked.len()] == walked[..]
                && old[walked.len()] == word)
                .then(|| old[walked.len()])
        });
        // `help` after a word whose new leaf takes a positional is that positional's value.
        let takes_a_value =
            at_new_path(&tree, &walked).is_some_and(|leaf| leaf.get_positionals().next().is_some());
        match continues {
            _ if word == "help" && !under_help && !takes_a_value => under_help = true,
            Some(word) => walked.push(word),
            None => {
                next = Some(word);
                break;
            }
        }
        globals.extend(read);
        tail = after;
    }
    if walked.is_empty() {
        return None;
    }
    let (old, new) = row(&walked)?;

    // A word that is also a group of this release — `serve` — is the group's unless the first
    // token of the tail is an option its new leaf declares. A word after the group's name is the
    // group's to answer, one of its own (`local`) or not (`foo`); so is `help`; so is nothing at
    // all; and so is an option the leaf refuses too — `--hlep`, `-V` — which is asked of clap, one
    // token against the built leaf, because that token's syntax is clap's. The escape parses clean
    // and declares nothing, so it is not one either.
    if let [word] = walked.as_slice() {
        if tree.find_subcommand(word).is_some() {
            if under_help || next.is_some() {
                return None;
            }
            let first = tail.first()?;
            let leaf = at_new_path(&tree, &walked)?;
            let declared = match leaf
                .clone()
                .try_get_matches_from([OsString::from(leaf.get_name()), first.clone()])
            {
                Ok(read) => leaf.get_arguments().any(|argument| {
                    read.value_source(argument.get_id().as_str())
                        == Some(clap::parser::ValueSource::CommandLine)
                }),
                Err(error) => error.kind() != clap::error::ErrorKind::UnknownArgument,
            };
            if !declared {
                return None;
            }
        }
    }

    let mut rebuilt: Vec<OsString> = Vec::with_capacity(arguments.len() + 2);
    rebuilt.push(arguments[0].clone());
    rebuilt.extend(globals);
    if under_help {
        rebuilt.push(OsString::from("help"));
    }
    rebuilt.extend(new.iter().map(OsString::from));
    rebuilt.extend(walked[old.len()..].iter().map(OsString::from));
    rebuilt.extend(tail);
    *arguments = rebuilt;
    Some((old, new))
}

/// Run the Connectors command line over the arguments given, argv[0] included.
///
/// The caller owns the async runtime: the `connectors` binary makes one with `#[tokio::main]`, and
/// Zwirn makes one for the duration of the call.
pub async fn run_from<I, T>(arguments: I) -> std::process::ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let mut arguments: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    if let Some((old, new)) = moved(&mut arguments) {
        eprintln!(
            "note: `connectors {}` is now `connectors {}`, and the old path works for one more release",
            old.join(" "),
            new.join(" ")
        );
    }
    let cli = Cli::parse_from(arguments);
    // Captured before dispatch: a failure must be rendered in the format the caller asked for, and
    // the command that failed is no longer available to ask.
    let format = cli.output;
    let target = match &cli.command {
        Command::Connection { target, .. }
        | Command::Event { target, .. }
        | Command::Operation { target, .. } => Some(target.as_str()),
        _ => None,
    };
    match run(cli).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if let MainError::Refused(refusal) = &error {
                output::emit_refusal_with_target(format, refusal, target);
            } else {
                output::emit_error_with_target(format, error.code(), &error.to_string(), target);
            }
            std::process::ExitCode::FAILURE
        }
    }
}

/// Finish result delivery without deciding the command's semantic status. In particular,
/// doctor must still evaluate its health report after a consumer stops reading that report.
fn complete_output(result: Result<(), MainError>) -> Result<(), MainError> {
    match result {
        Err(MainError::Output(output::OutputError::Io(error)))
        | Err(MainError::Admin(admin::AdminError::Output(output::OutputError::Io(error))))
            if error.kind() == io::ErrorKind::BrokenPipe =>
        {
            Ok(())
        }
        result => result,
    }
}

fn emit(format: Format, value: &serde_json::Value) -> Result<(), MainError> {
    complete_output(output::emit(format, value).map_err(Into::into))
}

fn emit_targeted(format: Format, value: &serde_json::Value, target: &str) -> Result<(), MainError> {
    complete_output(output::emit_targeted(format, value, target).map_err(Into::into))
}

async fn run(cli: Cli) -> Result<(), MainError> {
    let format = cli.output;
    match cli.command {
        Command::Setup { command } => match command {
            // A shell program rather than a result, so `--output` does not apply to it. Rendered to a
            // buffer first: `clap_complete::generate` panics on a write error, and a reader that stops
            // early (`| head`) is not an error at all.
            SetupCommand::Completions { shell } => {
                let mut script = Vec::new();
                clap_complete::generate(shell, &mut Cli::command(), "connectors", &mut script);
                match io::Write::write_all(&mut io::stdout().lock(), &script) {
                    Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
                    result => result.map_err(MainError::Completions),
                }
            }
            SetupCommand::Init {
                config,
                state_root,
                integrations,
                allow_exec_auth,
                force,
            } => initialize(
                format,
                config,
                state_root,
                &integrations,
                allow_exec_auth,
                force,
            ),
            SetupCommand::Connect {
                target,
                provider,
                config,
                label,
                context,
                state_root,
                credential,
                oauth,
                settings,
                allow,
                operator_network,
                credential_file,
                instance,
            } => {
                target.validate(&config, &state_root)?;
                if target == Target::Hosted {
                    if context.is_some()
                        || credential.is_some()
                        || oauth.instruction_file.is_some()
                        || oauth.operation.is_some()
                        || !settings.is_empty()
                        || allow.is_some()
                        || operator_network
                        || instance.is_some()
                    {
                        return Err(MainError::HostedSetupOptions);
                    }
                    let provider = provider.ok_or(MainError::HostedSetupOptions)?;
                    let auth_profile = oauth.auth_profile.ok_or(MainError::HostedSetupOptions)?;
                    let credential_file = credential_file.ok_or(MainError::HostedSetupOptions)?;
                    connect::validate_hosted_token_profile(&provider, &auth_profile)?;
                    let description = AuthenticatedHostedClient::active()?
                        .connect_with_credential_file(
                            protocol::connection::ConnectSessionCreateRequest {
                                integration_ref: provider.clone(),
                                label: label.unwrap_or_else(|| provider.clone()),
                                auth_profile: Some(auth_profile),
                            },
                            &credential_file,
                        )
                        .await?;
                    emit_targeted(
                        format,
                        &serde_json::json!({
                            "provider": provider,
                            "connected": true,
                            "connection": description.summary.label,
                            "connection_ref": description.summary.connection_ref,
                            "auth_profile": description.summary.auth_profile,
                        }),
                        target.as_str(),
                    )?;
                    return Ok(());
                }
                let config_path = config.map_or_else(default_config_path, Ok)?;
                let state_root = state_root.map_or_else(default_state_root, Ok)?;
                validate_state_root(&state_root)?;
                let personal = PersonalConfig::read(&config_path)?;
                if let (Some(operation_ref), Some(connection_ref)) =
                    (oauth.operation, oauth.connection)
                {
                    remediation::require_daemon(&state_root)?;
                    let input =
                        remediation::read_input(oauth.input_json, oauth.input_file, oauth.input)?;
                    let outcome = remediation::run(
                        &personal,
                        &state_root,
                        protocol::connection_v2::RemediationStartRequest {
                            operation_ref,
                            connection_ref,
                            input,
                        },
                        oauth.instruction_file.as_deref(),
                    )
                    .await?;
                    emit(format, &outcome)?;
                    return Ok(());
                }
                let provider = provider.ok_or(remediation::RemediationError::Configuration)?;
                let options = enrol::Options {
                    credential,
                    values: settings.into_iter().collect(),
                    allow_writes: allow.as_deref() == Some("writes"),
                    operator_network,
                    force: false,
                    credential_file,
                    instance,
                    acquire: enrol::acquires(&provider)
                        .then(|| connectors_runtime::argocd_acquisition(operator_network)),
                };
                let outcome = connect::dispatch_with_personal_oauth(
                    &provider,
                    &personal,
                    &config_path,
                    &state_root,
                    label,
                    context,
                    connect::PersonalOAuthOptions {
                        enrol: options,
                        auth_profile: oauth.auth_profile,
                        instruction_file: oauth.instruction_file,
                    },
                )
                .await?;
                emit(format, &outcome)?;
                Ok(())
            }
        },
        Command::Inspect { command } => match command {
            InspectCommand::Doctor { config, state_root } => diagnose(format, config, state_root),
            InspectCommand::Providers { query } => {
                emit(format, &connectors_console::providers::run(&query))?;
                Ok(())
            }
            InspectCommand::Auth { config, state_root } => {
                let config = read_config(config)?;
                let state_root = state_root.map_or_else(default_state_root, Ok)?;
                emit(format, &auth::status(&config, &state_root).await?)?;
                Ok(())
            }
        },
        Command::Session { command } => match command {
            SessionCommand::Login {
                base,
                no_browser,
                timeout_seconds,
            } => {
                let session = connectors_client::login(&LoginOptions {
                    connectors_base: base,
                    no_browser,
                    timeout: std::time::Duration::from_secs(timeout_seconds),
                })
                .await?;
                emit(
                    format,
                    &serde_json::json!({
                        "signed_in_as": session.display_identity(),
                        "connectors_base": session.connectors_base,
                        "identity_origin": session.identity_origin,
                        "tenant_id": session.tenant_id,
                        "default_target": "local",
                        "note": "Login never changes the default local target; choose --target hosted to use this deployment.",
                    }),
                )?;
                Ok(())
            }
            SessionCommand::Logout => {
                let session = connectors_client::logout()?;
                emit(
                    format,
                    &serde_json::json!({
                        "logged_out": session.as_ref().map(|session| session.connectors_base.as_str())
                    }),
                )?;
                Ok(())
            }
        },
        Command::Serve { command } => match command {
            ServeCommand::Local { config, state_root } => serve(config, state_root).await,
            ServeCommand::Hosted { config } => serve_hosted(&config).await,
            ServeCommand::Mcp => {
                if format != Format::Text {
                    return Err(MainError::McpOutput);
                }
                connectors_client::run_mcp_bridge().await?;
                Ok(())
            }
        },
        Command::Connection { target, command } => connection(format, target, command).await,
        Command::Event { target, command } => event(format, target, command).await,
        Command::Operation {
            target,
            protocol_version,
            command,
        } => operation(format, target, protocol_version, command).await,
        Command::Admin(command) => {
            complete_output(admin::run(format, command).await.map_err(Into::into))
        }
    }
}

/// Report on this installation, and exit non-zero when something in it cannot work.
fn diagnose(
    format: Format,
    config: Option<PathBuf>,
    state_root: Option<PathBuf>,
) -> Result<(), MainError> {
    let config_path = config.map_or_else(default_config_path, Ok)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let report = doctor::run(&config_path, &state_root);
    emit(format, &report.to_value())?;
    if report.healthy() {
        Ok(())
    } else {
        // The report was already rendered; this only sets the exit code, so a script can branch on
        // `connectors inspect doctor` without parsing it.
        Err(MainError::Unhealthy)
    }
}

fn initialize(
    format: Format,
    config: Option<PathBuf>,
    state_root: Option<PathBuf>,
    integrations: &[init::Integration],
    allow_exec_auth: bool,
    force: bool,
) -> Result<(), MainError> {
    let config_path = config.map_or_else(default_config_path, Ok)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let written = init::run(
        &config_path,
        &state_root,
        integrations,
        allow_exec_auth,
        force,
    )?;
    emit(
        format,
        &serde_json::json!({
            "config": written.config_path.display().to_string(),
            "state_root": written.state_root.display().to_string(),
            "authority_snapshot_id": written.snapshot_id,
            "integrations": written.integrations,
            "notes": written.notes,
        }),
    )?;
    Ok(())
}

async fn serve_hosted(config_path: &Path) -> Result<(), MainError> {
    let runtime = HostedRuntime::bind(config_path).await?;
    println!("{}", runtime.readiness());
    runtime
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn serve(config_path: Option<PathBuf>, state_root: Option<PathBuf>) -> Result<(), MainError> {
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    // **An omitted `--config` means the well-known path, not "no configuration".** Passing `None`
    // straight through composed a daemon with an empty registry: `connectors setup init` followed
    // by `connectors serve local` published a readiness document reporting every integration as
    // absent, and
    // the operator's next command found nothing callable with no error anywhere to explain it.
    //
    // A genuinely absent file still composes the refusing protocol skeleton, which is what a
    // machine that has never run `init` should get — so the fallback is "the default path if it
    // exists", not "the default path, and fail if it does not".
    let config_path = match config_path {
        Some(explicit) => Some(explicit),
        None => default_config_path().ok().filter(|path| path.exists()),
    };
    let runtime = PersonalRuntime::bind(config_path.as_deref(), state_root).await?;
    println!("{}", runtime.readiness());
    runtime
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn connection(
    format: Format,
    target: Target,
    command: ConnectionCommand,
) -> Result<(), MainError> {
    let (config_path, state_root, request) = match command {
        ConnectionCommand::Candidates {
            config,
            integration,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::CandidateSearch(CandidateSearchRequest {
                integration_ref: integration,
                query,
                limit,
            }),
        ),
        ConnectionCommand::Activate {
            config,
            candidate,
            label,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::CandidateActivate(CandidateActivateRequest {
                candidate_ref: candidate,
                label,
            }),
        ),
        ConnectionCommand::List {
            config,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::Search(ConnectionSearchRequest { query, limit }),
        ),
        ConnectionCommand::Observations {
            config,
            source,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::ObservationSearch(ObservationSearchRequest {
                source_connection_ref: source,
                query,
                limit,
            }),
        ),
        ConnectionCommand::Materialize {
            config,
            observation,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::Materialize(MaterializeRequest {
                observation_ref: observation,
            }),
        ),
    };
    target.validate(&config_path, &state_root)?;
    if target == Target::Hosted {
        let response = AuthenticatedHostedClient::active()?
            .connection(request)
            .await?;
        emit_targeted(
            format,
            &reduce_envelope!(response, connection)?,
            target.as_str(),
        )?;
        return Ok(());
    }
    let config_path = config_path.map_or_else(default_config_path, Ok)?;
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let response = if local_socket_absent(&state_root)? {
        PersonalRuntime::one_shot_connection(
            &config_path,
            state_root,
            config.owner_context(),
            request,
        )
        .await?
    } else {
        validate_state_root(&state_root)?;
        LocalClient::new(state_root.join("connectors.sock"))
            .connection(&config.owner_context(), request)
            .await?
    };
    emit_targeted(
        format,
        &reduce_envelope!(response, connection)?,
        target.as_str(),
    )?;
    Ok(())
}

async fn event(format: Format, target: Target, command: EventCommand) -> Result<(), MainError> {
    let (config_path, state_root, request) = match command {
        EventCommand::Search {
            config,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            EventRequest::Search(EventSearchRequest { query, limit }),
        ),
        EventCommand::Receive {
            config,
            channel,
            after,
            limit,
            wait_ms,
            state_root,
        } => (
            config,
            state_root,
            EventRequest::Receive(ReceiveRequest {
                channel_ref: channel,
                after,
                limit,
                wait_ms,
            }),
        ),
        EventCommand::Replay {
            config,
            event,
            state_root,
        } => (
            config,
            state_root,
            EventRequest::Replay(ReplayRequest { event_ref: event }),
        ),
    };
    target.validate(&config_path, &state_root)?;
    if target == Target::Hosted {
        let response = AuthenticatedHostedClient::active()?.event(request).await?;
        emit_targeted(format, &reduce_envelope!(response)?, target.as_str())?;
        return Ok(());
    }
    let config = read_config(config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    if local_socket_absent(&state_root)? {
        return Err(MainError::DaemonRequired);
    }
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .event(&config.owner_context(), request)
        .await?;
    emit_targeted(format, &reduce_envelope!(response)?, target.as_str())?;
    Ok(())
}

async fn operation(
    format: Format,
    target: Target,
    version: OperationVersion,
    command: OperationCommand,
) -> Result<(), MainError> {
    // Resolve conflicting placement options before any branch can acquire caller input. This
    // exhaustive match also makes each future operation variant participate in that check.
    let (config_path, state_root) = match &command {
        OperationCommand::Search {
            config, state_root, ..
        }
        | OperationCommand::Describe {
            config, state_root, ..
        }
        | OperationCommand::Signal {
            config, state_root, ..
        }
        | OperationCommand::Invoke {
            config, state_root, ..
        } => (config, state_root),
    };
    target.validate(config_path, state_root)?;
    let (config_path, state_root, request) = match command {
        OperationCommand::Search {
            config,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            OperationRequest::Search(OperationSearchRequest { query, limit }),
        ),
        OperationCommand::Describe {
            config,
            operation,
            state_root,
        } => (
            config,
            state_root,
            OperationRequest::Describe(OperationDescribeRequest {
                operation_ref: operation,
            }),
        ),
        OperationCommand::Signal {
            config,
            execution_ref,
            dtmf,
            state_root,
        } => (
            config,
            state_root,
            OperationRequest::SessionSignal(protocol::operation::SessionSignalRequest {
                execution_ref,
                signal: protocol::operation::ChannelSignal::Dtmf { digits: dtmf },
            }),
        ),
        OperationCommand::Invoke {
            config,
            operation,
            connection,
            description_ref,
            input_json,
            input_file,
            input,
            approval_evidence_ref,
            state_root,
        } => {
            let input = input::read(input_json, input_file, input)?;
            (
                config,
                state_root,
                OperationRequest::Invoke(OperationInvokeRequest {
                    operation_ref: operation,
                    connection_ref: connection,
                    description_ref,
                    input,
                    approval_evidence_ref,
                }),
            )
        }
    };
    if target == Target::Hosted {
        let client = AuthenticatedHostedClient::active()?;
        let value = match version {
            OperationVersion::V2 => {
                reduce_envelope!(client.operation_v2(request).await?, operation)?
            }
            OperationVersion::V3 => reduce_envelope!(client.operation(request).await?, operation)?,
        };
        emit_targeted(format, &value, target.as_str())?;
        return Ok(());
    }
    let config_path = config_path.map_or_else(default_config_path, Ok)?;
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let value = if local_socket_absent(&state_root)? {
        match version {
            OperationVersion::V2 => reduce_envelope!(
                PersonalRuntime::one_shot_operation(
                    &config_path,
                    state_root,
                    config.owner_context(),
                    request,
                )
                .await?,
                operation
            )?,
            OperationVersion::V3 => match PersonalRuntime::one_shot_operation_v3_outcome(
                &config_path,
                state_root,
                config.owner_context(),
                request,
            )
            .await?
            {
                OneShotOperationV3Outcome::Reply(response) => {
                    reduce_envelope!(response, operation)?
                }
                OneShotOperationV3Outcome::RequiresDaemon(_) => {
                    return Err(connectors_console::envelope::ReducedError {
                        code: "unavailable".into(),
                        message: "this operation requires a persistent daemon; run `connectors serve local` with the same --config and --state-root".into(),
                        retriable: false,
                        retry_after_seconds: None,
                        authentication: None,
                    }.into());
                }
            },
        }
    } else {
        validate_state_root(&state_root)?;
        let client = LocalClient::new(state_root.join("connectors.sock"));
        match version {
            OperationVersion::V2 => reduce_envelope!(
                client
                    .operation_v2(&config.owner_context(), request)
                    .await?,
                operation
            )?,
            OperationVersion::V3 => reduce_envelope!(
                client.operation(&config.owner_context(), request).await?,
                operation
            )?,
        }
    };
    emit_targeted(format, &value, target.as_str())?;
    Ok(())
}

/// Read the personal configuration, defaulting to the well-known path.
///
/// Every personal-local command takes `--config` as an option rather than a requirement. Before
/// this, each one demanded an explicit path, so the shortest working invocation of the product
/// carried a path the person had to know and retype.
fn read_config(path: Option<PathBuf>) -> Result<PersonalConfig, MainError> {
    let path = path.map_or_else(default_config_path, Ok)?;
    Ok(PersonalConfig::read(&path)?)
}

#[cfg(test)]
mod tests;
