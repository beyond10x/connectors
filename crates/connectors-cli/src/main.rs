#![forbid(unsafe_code)]

//! The `connectors` binary: an async runtime, and the command line the library owns.

#[tokio::main]
async fn main() -> std::process::ExitCode {
    connectors_cli::run_from(std::env::args_os()).await
}
