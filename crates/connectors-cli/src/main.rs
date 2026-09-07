#![forbid(unsafe_code)]

//! The `connectors` binary: the library's command line, with an async runtime when needed.

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args_os().collect();
    if let Some(exit) = connectors_cli::run_without_runtime_from(arguments.clone()) {
        return exit;
    }

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("the Connectors async runtime starts")
        .block_on(connectors_cli::run_from(arguments))
}
