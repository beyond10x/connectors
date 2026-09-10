mod legacy;
mod local;

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args
        .get(1)
        .is_some_and(|arg| matches!(arg.to_str(), Some("describe" | "invoke" | "serve")))
    {
        let runtime = tokio::runtime::Runtime::new().expect("create compatibility runtime");
        runtime.block_on(legacy::main());
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
