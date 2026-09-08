use clap::{Parser, Subcommand};
use connectors_host::{
    credentials::CredentialRef,
    federation::{Federation, FederationConfig},
};
use connectors_sdk::Credential;
use std::{path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(about = "Discover and invoke independently hosted connector contracts")]
struct Args {
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

#[tokio::main]
async fn main() {
    connectors_host::logging();
    if let Err(error) = run(Args::parse()).await {
        eprintln!(
            "{}",
            serde_json::to_string(&error).unwrap_or_else(|_| "{\"code\":\"internal\"}".into())
        );
        std::process::exit(1);
    }
}
async fn client(
    endpoint: String,
    token_file: PathBuf,
    allow_plaintext: bool,
) -> connectors_core::Result<connectors_client::Client> {
    let secret = CredentialRef::File { path: token_file }.resolve().await?;
    let token = String::from_utf8(secret.0)
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
