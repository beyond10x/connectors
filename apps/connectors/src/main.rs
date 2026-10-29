mod local;
mod mcp;

/// Answers the root `--version` and `-V`, and nothing else.
///
/// The grouped command tree is generated from the ESS CLI binding, whose format
/// carries no version, so the version is declared here from this package's own.
/// Every other argument list is declined to the generated parser unchanged.
#[derive(clap::Parser)]
#[command(name = "connectors", version, disable_help_flag = true)]
struct VersionFlag;

fn main() {
    let mut args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "__connectors-owner") {
        let result = if args.len() == 4 {
            connectors_host::local::config::Paths::resolve(
                Some(std::path::Path::new(&args[2])),
                Some(std::path::Path::new(&args[3])),
            )
            .map_err(connectors_host::local::owner::Error::from)
            .and_then(|paths| {
                connectors_host::local::owner::serve_with_read_policy(
                    paths,
                    std::sync::Arc::new(mcp::ProjectionPolicy),
                )
            })
        } else {
            Err(connectors_host::local::owner::Code::InvalidInput.into())
        };
        // The refusal's code alone, on standard error; the CLI starts its owner
        // with standard error discarded (contracts/cli/v1alpha1/owner.md).
        if let Err(error) = &result
            && let Ok(code) = serde_json::to_string(&error.code)
        {
            eprintln!("connectors owner: {code}");
        }
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    // Every usage line names the binding's binary, `connectors` (help.txt). clap takes
    // the displayed name from argv[0], which on Windows is `connectors.exe`.
    if let Some(name) = args.first_mut() {
        *name = "connectors".into();
    }
    if let Err(error) = <VersionFlag as clap::Parser>::try_parse_from(&args)
        && error.kind() == clap::error::ErrorKind::DisplayVersion
    {
        print!("{error}");
        return;
    }
    if mcp::selected(&args) {
        std::process::exit(mcp::run(args));
    }
    let output = local::run(args);
    use std::io::Write;
    let written = std::io::stdout()
        .lock()
        .write_all(output.stdout.as_bytes())
        .and_then(|_| std::io::stderr().lock().write_all(output.stderr.as_bytes()));
    std::process::exit(if written.is_ok() { output.exit_code } else { 1 });
}
