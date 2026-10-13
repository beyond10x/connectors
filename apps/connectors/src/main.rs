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
    let mut args: Vec<_> = std::env::args_os().collect();
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
    let args = legacy_help(args);
    if legacy_route(&args) {
        let runtime = tokio::runtime::Runtime::new().expect("create compatibility runtime");
        runtime.block_on(legacy::main(args));
        return;
    }
    let completions = is_bash_completions(&args);
    let mut output = local::run(args);
    if completions && output.exit_code == 0 {
        output.stdout.push_str(&legacy::completion());
    }
    use std::io::Write;
    let written = std::io::stdout()
        .lock()
        .write_all(output.stdout.as_bytes())
        .and_then(|_| std::io::stderr().lock().write_all(output.stderr.as_bytes()));
    std::process::exit(if written.is_ok() { output.exit_code } else { 1 });
}

/// Rewrites exactly `help COMMAND` to `COMMAND --help` when COMMAND is an
/// explicit service command. Every other argument list, including one with a
/// leading global, is returned unchanged.
fn legacy_help(mut args: Vec<std::ffi::OsString>) -> Vec<std::ffi::OsString> {
    if args.len() == 3
        && args[1] == "help"
        && matches!(args[2].to_str(), Some("describe" | "invoke" | "serve"))
    {
        args.remove(1);
        args.push("--help".into());
    }
    args
}

/// Whether the words other than process globals are exactly `completions bash`.
/// The generated parser accepts its globals anywhere, so they are skipped anywhere.
fn is_bash_completions(args: &[std::ffi::OsString]) -> bool {
    let mut words = Vec::new();
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--output" | "--config" | "--state-dir") => {
                rest.next();
            }
            Some(arg)
                if ["--output=", "--config=", "--state-dir="]
                    .iter()
                    .any(|prefix| arg.starts_with(prefix)) => {}
            word => words.push(word),
        }
    }
    words == [Some("completions"), Some("bash")]
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
