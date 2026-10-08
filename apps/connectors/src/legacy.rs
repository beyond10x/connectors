use clap::{Parser, Subcommand};
use connectors_host::{
    credentials::CredentialRef,
    federation::{Federation, FederationConfig},
};
use connectors_sdk::Credential;
use std::{ffi::OsString, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(about = "Discover and invoke independently hosted connector contracts")]
struct Args {
    /// Output for parse refusals; successful results stay raw JSON.
    #[arg(long, global = true, value_parser = ["human", "json"], default_value = "human")]
    output: String,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
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

/// Bash completion for the explicit service commands, appended to the generated
/// script. The generated function `_connectors` stays verbatim; a routing
/// function registered for `connectors` sends a command line whose command word,
/// after any leading `--output` selections, is `describe`, `invoke` or `serve` to
/// `_connectors_explicit`, built from this parser, and otherwise calls
/// `_connectors` and adds those words where they may stand: at the root and
/// as the word after a leading `help`.
pub fn completion() -> String {
    let mut bytes = Vec::new();
    clap_complete::generate(
        clap_complete::Shell::Bash,
        &mut <Args as clap::CommandFactory>::command(),
        "connectors_explicit",
        &mut bytes,
    );
    let script = String::from_utf8(bytes).expect("bash completion is UTF-8");
    let function = script
        .find(REGISTRATION)
        .map_or(script.as_str(), |end| &script[..end]);
    format!("{function}\n{ROUTE}")
}

const REGISTRATION: &str = "\nif [[ \"${BASH_VERSINFO[0]}\" -eq 4";

const ROUTE: &str = r#"_connectors_route() {
    local i word="" cur="${COMP_WORDS[COMP_CWORD]}"
    for (( i = 1; i < COMP_CWORD; i++ )); do
        case "${COMP_WORDS[i]}" in
            --output) (( i++ )) ;;
            --output=*) ;;
            *) word="${COMP_WORDS[i]}"; break ;;
        esac
    done
    case "${word}" in
        describe|invoke|serve)
            _connectors_explicit "$@"
            return
            ;;
    esac
    _connectors "$@"
    if [[ ${cur} == -* ]]; then
        return 0
    fi
    if [[ -z ${word} && ${COMP_WORDS[COMP_CWORD-1]} != --output ]] \
        || [[ ${word} == help && ${i} -eq 1 && ${COMP_CWORD} -eq 2 ]]; then
        COMPREPLY+=( $(compgen -W "describe invoke serve" -- "${cur}") )
    fi
    return 0
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F _connectors_route -o nosort -o bashdefault -o default connectors
else
    complete -F _connectors_route -o bashdefault -o default connectors
fi
"#;

pub async fn main(argv: Vec<OsString>) {
    let args = match Args::try_parse_from(&argv) {
        Ok(args) => args,
        Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => error.exit(),
        Err(_) => parse_refusal(&argv),
    };
    connectors_host::logging();
    if let Err(error) = run(args).await {
        eprintln!(
            "{}",
            serde_json::to_string(&error).unwrap_or_else(|_| "{\"code\":\"internal\"}".into())
        );
        std::process::exit(1);
    }
}
/// The contract's fixed parser refusal: exit 2, empty stdout, no argv echo.
fn parse_refusal(argv: &[OsString]) -> ! {
    let mut json = false;
    let mut previous = false;
    for arg in argv.iter().skip(1) {
        if arg == "--" {
            break;
        }
        if arg == "--output=json" || (previous && arg == "json") {
            json = true;
            break;
        }
        previous = arg == "--output";
    }
    if json {
        eprintln!(
            "{}",
            serde_json::json!({"ok": false, "error": {"code": "cli_parse", "data": {}}})
        );
    } else {
        eprintln!("cli_parse");
    }
    std::process::exit(2);
}
async fn client(
    endpoint: String,
    token_file: PathBuf,
    allow_plaintext: bool,
) -> connectors_core::Result<connectors_client::Client> {
    let secret = CredentialRef::File { path: token_file }.resolve().await?;
    let token = std::str::from_utf8(&secret.0)
        .map(str::to_owned)
        .map_err(|_| connectors_core::Error::invalid("invalid service token"))?;
    connectors_client::Client::new(&endpoint, token, allow_plaintext)
}
async fn run(args: Args) -> connectors_core::Result<()> {
    match args.command {
        Command::Describe {
            endpoint,
            token_file,
            allow_plaintext,
        } => {
            let descriptor = client(endpoint, token_file, allow_plaintext)
                .await?
                .describe()
                .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&descriptor)
                    .map_err(|_| connectors_core::Error::internal())?
            );
        }
        Command::Invoke {
            endpoint,
            token_file,
            allow_plaintext,
            operation,
            input,
        } => {
            let bytes = std::fs::read(input)
                .map_err(|_| connectors_core::Error::invalid("input file is not readable"))?;
            if bytes.len() > connectors_core::REQUEST_LIMIT {
                return Err(connectors_core::Error::invalid("input exceeds byte limit"));
            }
            let input = connectors_core::read_json(&bytes)
                .map_err(|_| connectors_core::Error::invalid("invalid JSON input"))?;
            let client = client(endpoint, token_file, allow_plaintext).await?;
            let descriptor = client.describe().await?;
            let result = client.invoke(&descriptor, &operation, input).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result)
                    .map_err(|_| connectors_core::Error::internal())?
            );
        }
        Command::Serve { config } => {
            let config: FederationConfig = connectors_host::read_config(&config)?;
            let federation = Federation::connect(&config).await?;
            connectors_host::server::serve(config.service, Arc::new(federation)).await?;
        }
    }
    Ok(())
}
