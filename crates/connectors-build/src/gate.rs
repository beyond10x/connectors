use super::Result;
use std::{path::Path, process::Command, sync::OnceLock};

/// The host triple from `rustc -vV`, read once per process.
fn host_triple() -> Result<&'static str> {
    static HOST: OnceLock<std::result::Result<String, String>> = OnceLock::new();
    let host = HOST.get_or_init(|| {
        let output = Command::new("rustc")
            .arg("-vV")
            .output()
            .map_err(|error| format!("rustc -vV: {error}"))?;
        if !output.status.success() {
            return Err(format!("rustc -vV failed: {}", output.status));
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .map(str::to_owned)
            .ok_or_else(|| "rustc -vV names no host".to_owned())
    });
    host.as_deref().map_err(|error| error.clone().into())
}

/// Cargo as the gate runs it. Test binaries get the task-owned temporary root as `TMPDIR`
/// through a runner. Cargo itself, and so the compiler wrapper it starts, keeps the caller's
/// `TMPDIR`: sccache binds its startup socket there, and the checkout-local root of a long
/// checkout pushes that path past `SUN_LEN`. The runner passes the root as the shell's `$0`
/// and the binary and its arguments as `"$@"`, so no path is parsed as an assignment or as
/// shell text; it is keyed to the host triple so it outranks a user's host runner.
fn cargo(root: &Path, temp: &Path, ess: &Path, toolchain: Option<&str>) -> Result<Command> {
    let temp = temp
        .to_str()
        .ok_or("gate temporary root is not valid UTF-8")?;
    let runner = serde_json::to_string(&["sh", "-c", r#"export TMPDIR="$0"; exec "$@""#, temp])?;
    let host = serde_json::to_string(host_triple()?)?;
    let mut cmd = Command::new("cargo");
    if let Some(toolchain) = toolchain {
        cmd.arg(format!("+{toolchain}"));
    }
    cmd.current_dir(root)
        .arg("--config")
        .arg(format!("target.{host}.runner={runner}"))
        .env("CARGO_BUILD_JOBS", "2")
        .env("CONNECTORS_ESS", ess);
    Ok(cmd)
}

pub fn run(root: &Path, ess: &Path, aep: &Path, msrv: bool) -> Result<()> {
    connectors_spec::v2::check_ess(ess)?;
    super::metadata_entities::run(root, true)?;
    let base = root.join(".local/tmp");
    std::fs::create_dir_all(&base)?;
    let temp = tempfile::Builder::new().prefix("gate-").tempdir_in(base)?;
    super::ess_boundary::run(root, ess, temp.path())?;
    super::source_hashes::run(root)?;
    super::cli::run(root, ess, true)?;
    super::mcp::run(root, ess, true)?;
    // The website build's examples step, without its WASM build: a target refusing the
    // example model fails here rather than only in `npm run build`.
    super::docs::synthesize_examples(root, ess, &temp.path().join("examples"))?;
    println!("gate: website example model synthesizes for rust and web; exit=0");
    let command = |toolchain: Option<&str>| cargo(root, temp.path(), ess, toolchain);
    let execute = |cmd: &mut Command| -> Result<()> {
        println!("gate: {:?}", cmd.get_args().collect::<Vec<_>>());
        let status = cmd.status()?;
        println!(
            "gate: exit={status} {:?}",
            cmd.get_args().collect::<Vec<_>>()
        );
        if !status.success() {
            return Err(format!("gate command {:?} failed", cmd.get_program()).into());
        }
        Ok(())
    };
    // `fmt --all` follows excluded path dependencies too. Only authored workspace
    // members belong to rustfmt; generated dependencies retain their pinned bytes.
    let metadata: serde_json::Value = serde_json::from_slice(
        &super::run(command(None)?.args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--locked",
            "--offline",
        ]))?
        .stdout,
    )?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("missing Cargo workspace members")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing Cargo packages")?;
    let mut format = command(None)?;
    format.arg("fmt");
    let mut selected = 0;
    for package in packages.iter().filter(|p| members.contains(&p["id"])) {
        format.args([
            "--package",
            package["name"]
                .as_str()
                .ok_or("missing Cargo package name")?,
        ]);
        selected += 1;
    }
    if selected == 0 || selected != members.len() {
        return Err("incomplete Cargo formatting selection".into());
    }
    execute(format.args(["--", "--check"]))?;
    for adapter in ["kubernetes", "sql"] {
        let source = root.join(format!("adapters/{adapter}/spec/adapter.json"));
        let descriptor = connectors_spec::compile(&std::fs::read(source)?)?;
        if connectors_spec::descriptor_bytes(&descriptor)?
            != std::fs::read(root.join(format!("adapters/{adapter}/generated/descriptor.json")))?
        {
            return Err(format!("{adapter}: generated descriptor drift").into());
        }
        println!("gate: {adapter} descriptor matches; exit=0");
    }
    // Generation fixtures verify the full pinned bundle, reproducibility and refusals.
    execute(command(None)?.args(["build", "--workspace", "--locked", "--offline"]))?;
    execute(command(None)?.args(["test", "--workspace", "--locked", "--offline"]))?;
    execute(command(None)?.args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--offline",
        "--",
        "-D",
        "warnings",
    ]))?;
    for adapter in ["kubernetes", "sql", "catalog-provider", "mcp"] {
        let package = format!("connectors-{adapter}");
        execute(command(None)?.args([
            "build",
            "--locked",
            "--offline",
            "-p",
            &package,
            "--lib",
            "--no-default-features",
        ]))?;
        let output = super::run(command(None)?.args([
            "tree",
            "--locked",
            "--offline",
            "--edges",
            "normal",
            "--prefix",
            "none",
            "--no-default-features",
            "-p",
            &package,
        ]))?;
        let tree = String::from_utf8(output.stdout)?;
        for name in [
            "connectors-host",
            "connectors-client",
            "connectors-kubernetes",
            "connectors-mcp",
            "connectors-sql",
        ] {
            if name != package
                && tree
                    .lines()
                    .any(|line| line.split_whitespace().next() == Some(name))
            {
                return Err(format!("{package} library depends on forbidden {name}").into());
            }
        }
        println!("gate: {package} library boundary holds; exit=0");
    }
    let output = super::run(command(None)?.args([
        "tree",
        "--locked",
        "--offline",
        "--edges",
        "normal",
        "--prefix",
        "none",
        "-p",
        "connectors",
    ]))?;
    let tree = String::from_utf8(output.stdout)?;
    if tree.lines().any(|line| {
        matches!(
            line.split_whitespace().next(),
            Some("connectors-kubernetes" | "connectors-sql" | "tokio-postgres")
        )
    }) {
        return Err("generic CLI depends on a provider".into());
    }
    println!("gate: generic CLI boundary holds; exit=0");
    if msrv {
        // Keep compiler metadata separate. The Eventlog-backed host graph needs
        // Rust 1.91; libraries which do not select that graph retain 1.88.
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("target"));
        for package in [
            "connectors-catalog",
            "connectors-client",
            "connectors-contracts",
            "connectors-core",
            "connectors-mcp",
            "connectors-sdk",
        ] {
            execute(
                command(Some("1.88.0"))?
                    .args([
                        "check",
                        "--package",
                        package,
                        "--all-targets",
                        "--locked",
                        "--offline",
                    ])
                    .env("CARGO_TARGET_DIR", target.join("msrv-1.88")),
            )?;
        }
        for package in [
            "connectors-catalog-provider",
            "connectors-kubernetes",
            "connectors-sql",
        ] {
            execute(
                command(Some("1.88.0"))?
                    .args([
                        "check",
                        "--package",
                        package,
                        "--lib",
                        "--no-default-features",
                        "--locked",
                        "--offline",
                    ])
                    .env("CARGO_TARGET_DIR", target.join("msrv-1.88")),
            )?;
        }
        execute(
            command(Some("1.91.0"))?
                .args([
                    "check",
                    "--workspace",
                    "--all-targets",
                    "--locked",
                    "--offline",
                ])
                .env("CARGO_TARGET_DIR", target.join("msrv-1.91")),
        )?;
    }
    // The pinned ESS authoring reader is non-recursive. Collect each explicitly
    // registered directory with a unique prefix so none silently disappears.
    let scenarios = temp.path().join("authored-scenarios");
    std::fs::create_dir(&scenarios)?;
    for (index, directory) in [
        "contracts/operations/v1alpha1/scenarios",
        "contracts/auth/acquisition/v1alpha1/scenarios",
        "contracts/auth/evidence/v1alpha1/scenarios",
        // Inbound only. These are `ess-scenario/1` and ESS owns them, so synthesising them
        // against the shared IR is a real check.
        //
        // The outbound directory beside it is deliberately absent. Those files are
        // `mcp-outbound-lifecycle/1`, a format this repository defined and ESS does not
        // know — and ESS refuses an unknown key rather than ignoring it, so collecting
        // them here fails the gate on every one of them. They are held by
        // `mcp_outbound_connection_lifecycle`'s own package-scoped cases instead, which is
        // a weaker guarantee than the shared IR gives the inbound set, and saying so here
        // is the point: adding the directory would turn a stated gap into a broken gate.
        "adapters/mcp/contracts/server/v1alpha1/scenarios",
    ]
    .iter()
    .enumerate()
    {
        let mut files = std::fs::read_dir(root.join(directory))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        files.sort();
        let mut count = 0;
        for source in files {
            if source.is_file()
                && matches!(
                    source.extension().and_then(|ext| ext.to_str()),
                    Some("yaml" | "yml")
                )
            {
                let name = source.file_name().ok_or("scenario has no filename")?;
                std::fs::copy(
                    &source,
                    scenarios.join(format!("{index}-{}", name.to_string_lossy())),
                )?;
                count += 1;
            }
        }
        if count == 0 {
            return Err(format!("{directory}: no authored scenarios").into());
        }
        println!("gate: collected {count} authored scenarios from {directory}; exit=0");
    }
    // Compile generated obligations and authored traces. This checks the model.
    execute(
        Command::new(ess)
            .current_dir(root)
            .env("TMPDIR", temp.path())
            .args([
                "verify",
                "conform",
                "synthesize",
                "--path",
                "ess",
                "--target",
                "ir",
                "--scenarios",
            ])
            .arg(&scenarios)
            .arg("--out")
            .arg(temp.path().join("contract-conformance.json")),
    )?;
    // Execute the local metadata authority's synthesized scenarios against the lowered
    // definitions the host runs, not only the model.
    let metadata_conformance = temp.path().join("metadata-conformance");
    std::fs::create_dir_all(&metadata_conformance)?;
    super::metadata_conformance::gate(root, &metadata_conformance)?;
    let mut planning = Command::new(aep);
    planning.current_dir(root).env("TMPDIR", temp.path());
    execute(planning.args(["plan", "artifact", "validate"]))?;
    println!("gate: all checks passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cargo;
    use std::{ffi::OsStr, path::Path};

    const ROOT: &str = "/checkout/.local/tmp/gate-AbCdEf";

    fn build(toolchain: Option<&str>) -> std::process::Command {
        cargo(
            Path::new("/checkout"),
            Path::new(ROOT),
            Path::new("/ess"),
            toolchain,
        )
        .expect("gate cargo command")
    }

    /// The compiler wrapper cargo starts (sccache) inherits cargo's own `TMPDIR` and puts
    /// its startup socket there, so the gate's long root must not be cargo's `TMPDIR`.
    #[test]
    fn gate_cargo_leaves_the_compiler_wrapper_on_the_callers_tmpdir() {
        let command = build(None);
        assert!(
            command
                .get_envs()
                .all(|(name, _)| name != OsStr::new("TMPDIR")),
            "gate cargo sets or clears TMPDIR: {:?}",
            command.get_envs().collect::<Vec<_>>()
        );
    }

    /// The host triple as rustc reports it, read independently of the gate.
    fn rustc_host() -> String {
        let output = std::process::Command::new("rustc")
            .arg("-vV")
            .output()
            .expect("rustc -vV");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .expect("rustc -vV names its host")
            .to_owned()
    }

    /// Test binaries still run under the gate's task-owned root, through a runner that
    /// cargo applies to target executables and never to the compiler. The root is the
    /// shell's `$0`, never parsed as an assignment or as shell text.
    #[test]
    fn gate_cargo_hands_the_temporary_root_to_test_binaries_through_a_runner() {
        let args: Vec<_> = build(None).get_args().map(OsStr::to_owned).collect();
        let host = rustc_host();
        let expected = format!(
            r#"target."{host}".runner=["sh","-c","export TMPDIR=\"$0\"; exec \"$@\"","{ROOT}"]"#
        );
        assert_eq!(
            args,
            [OsStr::new("--config"), OsStr::new(&expected)],
            "gate cargo arguments"
        );
    }

    /// The runner is keyed to the host triple, so it outranks a user's
    /// `[target.<triple>] runner` and `CARGO_TARGET_<TRIPLE>_RUNNER`; a `cfg(...)` key
    /// would lose to either.
    #[test]
    fn gate_cargo_keys_the_runner_to_the_host_triple() {
        let host = rustc_host();
        assert!(
            host.starts_with(std::env::consts::ARCH) && host.contains(std::env::consts::OS),
            "rustc host {host} is not this machine"
        );
        let command = build(None);
        let config = command
            .get_args()
            .nth(1)
            .and_then(OsStr::to_str)
            .expect("--config value");
        assert!(
            config.starts_with(&format!(r#"target."{host}".runner="#)),
            "runner key is not the host triple {host}: {config}"
        );
    }

    /// A rustup toolchain selector must stay the first argument for the proxy to see it.
    #[test]
    fn gate_cargo_keeps_the_toolchain_selector_first() {
        let command = build(Some("1.88.0"));
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args.first(), Some(&OsStr::new("+1.88.0")));
        assert_eq!(args.get(1), Some(&OsStr::new("--config")));
    }
}

/// Adversary cases: the runner the gate configures is executed the way Cargo executes it,
/// `<runner...> <test binary> <args...>`, against real programs.
#[cfg(test)]
mod adversary_tests {
    use super::cargo;
    use std::{os::unix::fs::symlink, path::Path, process::Command};

    fn runner(root: &str) -> Vec<String> {
        let command = cargo(
            Path::new("/checkout"),
            Path::new(root),
            Path::new("/ess"),
            None,
        )
        .expect("gate cargo command");
        let config = command
            .get_args()
            .map(|arg| arg.to_str().expect("utf-8 argument").to_owned())
            .find_map(|arg| {
                arg.split_once(".runner=")
                    .map(|(_, runner)| runner.to_owned())
            })
            .expect("runner --config argument");
        serde_json::from_str(&config).expect("runner is a JSON/TOML string array")
    }

    /// Run `program` through the gate's runner as Cargo would, with an empty environment
    /// so nothing sensitive can be printed.
    fn through_runner(root: &str, program: &Path) -> std::process::Output {
        let argv = runner(root);
        Command::new(&argv[0])
            .args(&argv[1..])
            .arg(program)
            .env_clear()
            .output()
            .expect("spawn runner")
    }

    fn link(dir: &Path, name: &str, target: &str) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        symlink(target, &path).unwrap();
        path
    }

    /// A test binary under a target directory whose path contains `=` (a checkout or
    /// `CARGO_TARGET_DIR` such as `.../build=1/target`) must still be executed, and its
    /// failure must still fail. `env` reads every leading `NAME=VALUE` word as an
    /// assignment, so it never runs the binary, prints its environment and exits 0.
    #[test]
    fn runner_executes_a_failing_test_binary_whose_path_contains_an_equals_sign() {
        let scratch = tempfile::tempdir().unwrap();
        let binary = link(
            &scratch.path().join("build=1/debug/deps"),
            "fails",
            "/usr/bin/false",
        );
        let output = through_runner("/checkout/.local/tmp/gate-AbCdEf", &binary);
        assert!(
            !output.status.success(),
            "a failing test binary at {} reported success through the gate runner \
             (status {:?}, {} bytes of stdout instead of running it)",
            binary.display(),
            output.status,
            output.stdout.len()
        );
    }

    /// Same path shape, observable effect: the binary runs and sees the gate root.
    #[test]
    fn runner_hands_the_root_to_a_binary_whose_path_contains_an_equals_sign() {
        let scratch = tempfile::tempdir().unwrap();
        let binary = link(&scratch.path().join("a=b"), "printenv", "/usr/bin/printenv");
        let root = "/checkout/.local/tmp/gate-AbCdEf";
        let argv = runner(root);
        let output = Command::new(&argv[0])
            .args(&argv[1..])
            .arg(&binary)
            .arg("TMPDIR")
            .env_clear()
            .output()
            .expect("spawn runner");
        assert_eq!(String::from_utf8_lossy(&output.stdout), format!("{root}\n"));
    }

    /// Roots with spaces, quotes, a single quote, a backslash, `=` and shell metacharacters
    /// reach the binary byte for byte.
    #[test]
    fn runner_preserves_awkward_roots() {
        let scratch = tempfile::tempdir().unwrap();
        let binary = link(scratch.path(), "printenv", "/usr/bin/printenv");
        for root in [
            "/c/gate a b",
            "/c/gate\"q",
            "/c/gate'q",
            "/c/gate\\b",
            "/c/gate=e",
            "/c/gate$x`y`;|&",
            "/c/gaté",
        ] {
            let argv = runner(root);
            let output = Command::new(&argv[0])
                .args(&argv[1..])
                .arg(&binary)
                .arg("TMPDIR")
                .env_clear()
                .output()
                .expect("spawn runner");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{root}\n"),
                "root {root:?}"
            );
        }
    }
}

/// Adversary pass 2: the `sh -c` runner must be transparent to the test binary's exit
/// status, its signals and the arguments Cargo appends, and must hand over any root.
#[cfg(test)]
mod adversary_pass2_tests {
    use super::cargo;
    use std::{
        os::unix::process::ExitStatusExt,
        path::Path,
        process::{Command, Output},
    };

    fn runner(root: &str) -> Vec<String> {
        let command = cargo(
            Path::new("/checkout"),
            Path::new(root),
            Path::new("/ess"),
            None,
        )
        .expect("gate cargo command");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_str().expect("utf-8 argument").to_owned())
            .collect();
        assert_eq!(args.len(), 2, "exactly --config and its value: {args:?}");
        assert_eq!(args[0], "--config");
        let (_, value) = args[1].split_once("\".runner=").expect("host-keyed runner");
        serde_json::from_str(value).expect("runner is a string array")
    }

    /// `<runner...> <binary> <args...>`, exactly as Cargo spawns a test binary.
    fn run(root: &str, binary: &str, args: &[&str]) -> Output {
        let argv = runner(root);
        Command::new(&argv[0])
            .args(&argv[1..])
            .arg(binary)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .output()
            .expect("spawn runner")
    }

    const ROOT: &str = "/checkout/.local/tmp/gate-AbCdEf";

    /// A test binary exiting 101 (libtest's failure code) or any other code > 1 keeps it.
    #[test]
    fn runner_preserves_exit_codes_above_one() {
        for code in [2, 101, 255] {
            let output = run(ROOT, "/bin/sh", &["-c", &format!("exit {code}")]);
            assert_eq!(output.status.code(), Some(code), "{:?}", output.status);
        }
    }

    /// A test binary killed by a signal is reported as killed, not as a shell exit code.
    #[test]
    fn runner_preserves_a_killing_signal() {
        for signal in [9, 6, 11] {
            let output = run(ROOT, "/bin/sh", &["-c", &format!("kill -{signal} $$")]);
            assert_eq!(output.status.signal(), Some(signal), "{:?}", output.status);
        }
    }

    /// Filters with spaces and newlines, empty arguments, leading dashes and shell text
    /// reach the test binary unchanged and in order.
    #[test]
    fn runner_passes_test_arguments_byte_for_byte() {
        let args = [
            "--exact",
            "module::name with space",
            "line1\nline2",
            "--nocapture",
            "-",
            "--",
            "",
            "$HOME `id` *",
            "--test-threads=1",
        ];
        let mut argv = vec!["-c", r#"printf '%s\0' "$@""#, "argv0"];
        argv.extend(args);
        let output = run(ROOT, "/bin/sh", &argv);
        assert!(output.status.success(), "{:?}", output.status);
        let expected: String = args.iter().map(|arg| format!("{arg}\0")).collect();
        assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    }

    /// The binary is executed even when its path begins with a dash or holds a newline.
    #[test]
    fn runner_executes_awkward_binary_paths() {
        let scratch = tempfile::tempdir().unwrap();
        for name in ["-dash", "new\nline", "sp ace", "--"] {
            let path = scratch.path().join(name);
            std::os::unix::fs::symlink("/usr/bin/printenv", &path).unwrap();
            let output = run(ROOT, path.to_str().unwrap(), &["TMPDIR"]);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{ROOT}\n"),
                "binary {name:?}: {:?}",
                output.status
            );
        }
    }

    /// A root that begins with a dash or holds a newline still becomes `TMPDIR`.
    #[test]
    fn runner_hands_over_roots_with_a_leading_dash_or_newline() {
        for root in [
            "-x/gate",
            "--/gate",
            "-c",
            "+o/gate",
            "/c/new\nline",
            "/c/tab\tx",
        ] {
            let output = run(root, "/usr/bin/printenv", &["TMPDIR"]);
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{root}\n"),
                "root {root:?}: {:?}",
                output.status
            );
        }
    }
}
