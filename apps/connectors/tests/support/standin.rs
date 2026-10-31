//! A stand-in catalog provider for the endpoint tests of
//! story:connectors-do-short-invocation, and the runner of the test binary it
//! lives in. Such a binary is built with `harness = false`, because the host
//! starts the installed provider as `--connector <c> --instance <i>
//! [--endpoint <e>] --print-local-bootstrap` and then its child from a sealed
//! snapshot with `--connectors-private-fd 3`: the standard test harness refuses
//! those arguments, and a script cannot be started from the snapshot.
//!
//! Printing, the stand-in appends its arguments to `args` beside its
//! installation's `bin` directory, so a test reads which endpoint the provider
//! was given, and prints a bootstrap for exactly that selection whose
//! configuration revision digests the endpoint. As a child it serves the same
//! bootstrap and validates nothing. An endpoint containing `refuse` is refused,
//! as a provider refuses an endpoint it cannot use.
//!
//! For story:migrate-writes-the-short-instance-form it also stands in for a
//! migrated GitLab instance, emulating the catalog provider's revision
//! ([`inputs`]): a selection naming a native configuration
//! (`--local-config <file>`) or an instance id (`--instance-id`) is described
//! from its digest input, printed per key for `--print-revision-inputs`, and
//! its child validates any credential and answers a read with its digest.
#![allow(dead_code)]
use connectors_host::local::{registry, runtime};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// The shipped GitLab declaration, which a derived instance is lowered from.
const GITLAB: &str = include_str!("../../../../adapters/catalog/providers/gitlab/connector.json");

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// The configuration revision digest input of a selection naming a native
/// configuration (its `instance`, `provider`, `api_base` and `auth`) or a
/// derived GitLab instance with an instance id (the declaration's provider,
/// the endpoint or default endpoint followed by `api_path`, and its single
/// profile as `auth`); `None` for any other selection.
pub fn inputs(args: &[String]) -> Option<BTreeMap<String, Value>> {
    let mut inputs = BTreeMap::new();
    if let Some(path) = option(args, "local-config") {
        let native: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        for key in ["instance", "provider", "api_base", "auth"] {
            inputs.insert(key.to_owned(), native.get(key)?.clone());
        }
    } else {
        let id = option(args, "instance-id")?;
        let declaration: Value = serde_json::from_str(GITLAB).unwrap();
        if option(args, "connector")? != declaration["connector_id"] {
            return None;
        }
        let endpoint = option(args, "endpoint")
            .or(declaration["default_endpoint"].as_str())?
            .trim_end_matches('/');
        let api_base = format!("{endpoint}{}", declaration["api_path"].as_str()?);
        inputs.insert("instance".into(), json!(id));
        inputs.insert("provider".into(), declaration["catalog_provider"].clone());
        inputs.insert("api_base".into(), json!(api_base));
        inputs.insert("auth".into(), declaration["profiles"][0].clone());
    }
    inputs.insert("format".into(), json!("standin/1"));
    Some(inputs)
}

/// The configuration revision of a digest input.
pub fn revision(inputs: &BTreeMap<String, Value>) -> String {
    format!(
        "cfg-{}",
        &sha256(serde_json::to_string(inputs).unwrap().as_bytes())[..16]
    )
}

/// The native configuration of the GitLab instance `instance` at `api_base`
/// whose profile requires `scopes`: with the declaration's scopes, the one a
/// derived instance of that id at that API base has.
pub fn native(instance: &str, api_base: &str, scopes: &[&str]) -> Value {
    let declaration: Value = serde_json::from_str(GITLAB).unwrap();
    let mut auth = declaration["profiles"][0].clone();
    auth["minimum_scopes"] = json!(scopes);
    json!({"instance": instance, "provider": declaration["catalog_provider"],
        "api_base": api_base, "auth": auth})
}

/// The revision a native configuration has.
pub fn native_revision(native: &Value) -> String {
    let mut inputs: BTreeMap<String, Value> = native
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    inputs.insert("format".into(), json!("standin/1"));
    revision(&inputs)
}

/// The value of `--<name>` in `args`.
fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let flag = format!("--{name}");
    args.iter()
        .position(|arg| *arg == flag)
        .and_then(|at| args.get(at + 1))
        .map(String::as_str)
}

/// The bootstrap the stand-in reports for the selection in `args`.
fn bootstrap(args: &[String]) -> runtime::Bootstrap {
    let (connector, instance, endpoint, revision) = match inputs(args) {
        Some(inputs) => (
            "gitlab".to_owned(),
            inputs["instance"].as_str().unwrap().to_owned(),
            inputs["api_base"].as_str().unwrap().to_owned(),
            revision(&inputs),
        ),
        None => {
            let connector = option(args, "connector").unwrap_or("none");
            let endpoint = option(args, "endpoint").unwrap_or("https://default.standin.invalid");
            (
                connector.to_owned(),
                format!("{connector}.{}", option(args, "instance").unwrap_or("none")),
                endpoint.to_owned(),
                format!(
                    "cfg-{}",
                    &hex::encode(Sha256::digest(endpoint.as_bytes()))[..16]
                ),
            )
        }
    };
    let endpoint = endpoint.as_str();
    let profile = format!("{connector}.pat");
    let operation = format!("{connector}.whoami");
    let descriptor = connectors_core::Descriptor {
        version: connectors_core::WIRE_VERSION.into(),
        instance: instance.clone(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: vec![connectors_core::Operation {
            id: operation.clone(),
            description: "a stand-in read".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","additionalProperties":false,"properties":{}}),
            output_schema: json!({"type":"object"}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance,
        adapter: "catalog".into(),
        protocol: connectors_core::WIRE_VERSION.into(),
        configuration_revision: revision,
        provider_authority: endpoint.into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: profile.clone(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
            acquisition: None,
        }],
        requirements: vec![runtime::Requirement {
            operation,
            custody: false,
            profile,
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

/// The child: its bootstrap, and whether it serves (a selection [`inputs`]
/// describes) rather than refusing every validation and invocation.
struct Child(runtime::Bootstrap, bool);

#[async_trait::async_trait]
impl runtime::Adapter for Child {
    fn bootstrap(&self) -> runtime::Bootstrap {
        self.0.clone()
    }
    /// A derived entry's child speaks `connectors-private/2`.
    fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
        Ok(self.0.clone())
    }
    async fn validate(
        &self,
        _: &str,
        _: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        if !self.1 {
            return Err(runtime::Failure::Unavailable);
        }
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            binding: None,
            identity: registry::ExternalIdentity {
                kind: "gitlab.user".into(),
                subject: "standin-subject-1".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 55_000,
        })
    }
    async fn invoke(
        &self,
        _: &str,
        _: &str,
        document: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        if !self.1 {
            return Err(runtime::Failure::Unsupported);
        }
        Ok(json!({"credential": sha256(&document.0)}))
    }
}

/// The schema digest and descriptor revision `invoke` names for the read of
/// the selection `args`.
pub fn read_schema(args: &[String]) -> (String, String) {
    let bootstrap = bootstrap(args);
    let operation = bootstrap.requirements[0].operation.clone();
    (
        connectors_host::local::owner::schema(&bootstrap, &operation).unwrap(),
        "desc-1".into(),
    )
}

/// Act as the provider when the arguments are a provider's, else run `tests`
/// (each named, filtered by a first free argument) and exit as a test binary
/// does: 0 when every selected test passed, 101 otherwise.
pub fn main(tests: &[(&str, fn())]) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--print-revision-inputs") {
        let Some(inputs) = inputs(&args) else {
            std::process::exit(1);
        };
        let digests: BTreeMap<&String, String> = inputs
            .iter()
            .map(|(key, value)| {
                (
                    key,
                    sha256(serde_json::to_string(value).unwrap().as_bytes()),
                )
            })
            .collect();
        println!("{}", serde_json::to_string(&digests).unwrap());
        return;
    }
    if args.iter().any(|arg| arg == "--print-local-bootstrap") {
        let exe = std::env::current_exe().unwrap();
        let record = exe.parent().unwrap().parent().unwrap().join("args");
        // A jailed provider may have no writable directory there.
        if let Ok(mut log) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(record)
        {
            writeln!(log, "{}", args.join("\n")).unwrap();
        }
        if option(&args, "endpoint").is_some_and(|endpoint| endpoint.contains("refuse")) {
            std::process::exit(1);
        }
        println!("{}", serde_json::to_string(&bootstrap(&args)).unwrap());
        return;
    }
    if args.iter().any(|arg| arg == "--print-profiles") {
        std::process::exit(1);
    }
    if args.iter().any(|arg| arg == "--connectors-private-fd") {
        let _ = runtime::serve(3, Child(bootstrap(&args), inputs(&args).is_some()));
        return;
    }
    let filter = args.iter().find(|arg| !arg.starts_with('-'));
    let (mut passed, mut failed) = (0, Vec::new());
    for (name, test) in tests {
        if filter.is_some_and(|filter| !name.contains(filter.as_str())) {
            continue;
        }
        let outcome = std::panic::catch_unwind(test);
        println!(
            "test {name} ... {}",
            if outcome.is_ok() { "ok" } else { "FAILED" }
        );
        match outcome {
            Ok(()) => passed += 1,
            Err(_) => failed.push(*name),
        }
    }
    let filtered = tests.len() - passed - failed.len();
    println!(
        "\ntest result: {}. {passed} passed; {} failed; 0 ignored; 0 measured; {filtered} filtered out",
        if failed.is_empty() { "ok" } else { "FAILED" },
        failed.len()
    );
    if !failed.is_empty() {
        println!("failures: {failed:?}");
        std::process::exit(101);
    }
}

fn private_file(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// An installation: a copy of the production CLI and, beside it, this test
/// binary as `connectors-catalog-provider`.
pub struct Installation {
    pub root: tempfile::TempDir,
}

impl Installation {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        for (source, name) in [
            (
                PathBuf::from(env!("CARGO_BIN_EXE_connectors")),
                "connectors",
            ),
            (
                std::env::current_exe().unwrap(),
                "connectors-catalog-provider",
            ),
        ] {
            let target = bin.join(name);
            fs::copy(source, &target).unwrap();
            fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self { root }
    }

    pub fn config(&self) -> PathBuf {
        self.root.path().join("config/config.toml")
    }

    pub fn run(&self, args: &[&str]) -> Output {
        let run = || {
            Command::new(self.root.path().join("bin/connectors"))
                .env_clear()
                .args(["--output", "json", "--config"])
                .arg(self.config())
                .arg("--state-dir")
                .arg(self.root.path().join("state"))
                .args(args)
                .stdin(std::process::Stdio::null())
                .output()
        };
        // A copied binary can be briefly busy (ETXTBSY) while it is written.
        for _ in 0..50 {
            match run() {
                Err(error) if error.raw_os_error() == Some(26) => {
                    std::thread::sleep(std::time::Duration::from_millis(100))
                }
                output => return output.unwrap(),
            }
        }
        run().unwrap()
    }

    /// `setup init`, then the configuration `connectors-local/5` of this
    /// user with `body` after its two first lines, owner-only.
    pub fn configure(&self, body: &str) -> String {
        let initialized = self.run(&["setup", "init"]);
        assert!(initialized.status.success(), "{initialized:?}");
        let text = format!(
            "format='connectors-local/5'\nowner_uid={}\n{body}",
            connectors_host::local::filesystem::uid()
        );
        private_file(&self.config(), text.as_bytes(), 0o600);
        text
    }

    /// The configuration file's text.
    pub fn text(&self) -> String {
        fs::read_to_string(self.config()).unwrap()
    }

    /// The configuration file's permission bits.
    pub fn mode(&self) -> u32 {
        fs::symlink_metadata(self.config())
            .unwrap()
            .permissions()
            .mode()
            & 0o7777
    }

    /// Each start of the provider to print a bootstrap: its arguments.
    pub fn calls(&self) -> Vec<Vec<String>> {
        fs::read_to_string(self.root.path().join("args"))
            .unwrap_or_default()
            .split_inclusive("--print-local-bootstrap\n")
            .map(|call| call.lines().map(str::to_owned).collect())
            .collect()
    }

    /// Forget the recorded provider starts.
    pub fn forget_calls(&self) {
        let _ = fs::remove_file(self.root.path().join("args"));
    }
}

/// The refusal's exit status and `error.data`.
pub fn refusal(output: &Output) -> (Option<i32>, Value) {
    assert!(!output.status.success(), "{output:?}");
    let value: Value =
        serde_json::from_slice(&output.stderr).unwrap_or_else(|_| panic!("{output:?}"));
    (output.status.code(), value["error"]["data"].clone())
}

/// The successful result.
pub fn success(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    value["result"].clone()
}

/// The configuration text parsed as TOML, for reading values.
pub fn parsed(text: &str) -> toml::Table {
    text.parse().unwrap()
}
