mod legacy;
mod local;

/// Answers the root `--version` and `-V`, and nothing else.
///
/// The grouped command tree is generated from the ESS CLI binding, whose format
/// carries no version, so the version is declared here from this package's own.
/// Every other argument list is declined to the generated parser unchanged.
#[derive(clap::Parser)]
#[command(name = "connectors", version, disable_help_flag = true)]
struct VersionFlag;

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "__connectors-owner") {
        let result = if args.len() == 4 {
            connectors_host::local::config::Paths::resolve(
                Some(std::path::Path::new(&args[2])),
                Some(std::path::Path::new(&args[3])),
            )
            .map_err(connectors_host::local::owner::Error::from)
            .and_then(connectors_host::local::owner::serve)
        } else {
            Err(connectors_host::local::owner::Code::InvalidInput.into())
        };
        std::process::exit(if result.is_ok() { 0 } else { 1 });
    }
    if let Err(error) = <VersionFlag as clap::Parser>::try_parse_from(&args)
        && error.kind() == clap::error::ErrorKind::DisplayVersion
    {
        print!("{error}");
        return;
    }
    if legacy_route(&args) {
        let runtime = tokio::runtime::Runtime::new().expect("create compatibility runtime");
        runtime.block_on(legacy::main(args));
        return;
    }
    let output = local::run(args);
    use std::io::Write;
    let written = std::io::stdout()
        .lock()
        .write_all(output.stdout.as_bytes())
        .and_then(|_| std::io::stderr().lock().write_all(output.stderr.as_bytes()));
    std::process::exit(if written.is_ok() { output.exit_code } else { 1 });
}

/// Whether the command word, after any leading `--output` selections, is an
/// explicit service command. Only the output global applies to those commands.
fn legacy_route(args: &[std::ffi::OsString]) -> bool {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--output") if rest.next().is_some() => {}
            Some(arg) if arg.starts_with("--output=") => {}
            Some("describe" | "invoke" | "serve") => return true,
            _ => return false,
        }
    }
    false
}
