//! story:invoke-definition-file, security review: the owner-only cache under
//! `<state>/definitions/` holds one directory per lowering, keyed by digest,
//! and `instances.toml`; "an entry holds the lowering of its key"
//! (`contracts/cli/v1alpha1/semantics.md` §3, "Invoking a definition file").
//! A definition is data: no field of it places files outside its own cache
//! entry. These cases start no provider (`CONNECTORS_INSTALLATION` names an
//! empty directory, so the command stops after lowering) and have no terminal.
#![cfg(unix)]
use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

/// `openapi.json.gz` (gzip, mtime 0) of a three-path fixture OpenAPI document
/// whose servers name `https://gitlab.example.com/api/v4`, and the SHA-256 of
/// the uncompressed document.
const SOURCE_GZ: &str = "1f8b08000000000002ffcd52b16ac33010fd9572b3b1dc2693b72c856e85b45d4a088a7c7194d892aa3b8714e37fef494920d02153a15e243dbdf7ee9dce23f8804e070b35cccaaa9c4101d66d3dd423b0e50e057fb6271e223e6c7d6c51ee8f18c97a27378f301540181302f5e70843ec04de3107aa956ac5406f4a3ce93e74581adf2ba9a48e7398560504cd3b4a75d4201669d322a7451245cd52e1a511330117c17eccdf13a9808814bc23cccaa7aa4a4b8364a20d7c0ee50f4233de31ba6ca743e8acc9866a4f893202991df63a37f91d528f7eb347c3220c3195677b2e609b1b8e15cb56424cd7af00255cb1d4dd5a1b83446bf60774a408bbeddd8e5e2fda4596be65e53209ffbacb9bf8d12784d4689be97ee00b5bc034bea87be4ebe49d1cd21b35f907925d1a6feee46bb011c586e380c5ef4cc4d1ba5632adfed770a7e9075b994af41c030000";
const SOURCE_SHA256: &str = "09a3f7ddf8a60e5e7dd547270129bd9d5ad84793942518bca977c3eded696b3c";

fn definition(catalog: Option<&str>) -> String {
    let mut text = String::from("schema: urn:beyond10x:connectors-definition:1\n");
    if let Some(catalog) = catalog {
        text.push_str(&format!("catalog: '{catalog}'\n"));
    }
    text.push_str(&format!(
        "source:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: {SOURCE_SHA256}\n\
         base_url: https://{{hostname}}/api/v4\n\
         config: {{hostname: {{type: string, default: gitlab.example.com}}}}\n\
         auth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n\
         \x20   label: Fixture personal access token\n\
         \x20   identity: {{path: user, kind: forge.user, subject: /id}}\n\
         \x20   scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}\n\
         operations:\n  project.get:\n    op: getApiV4ProjectsId\n    description: Read one project\n"
    ));
    text
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        let init = fixture.run(&["setup", "init"]);
        assert!(init.status.success(), "{init:?}");
        let defs = fixture.defs();
        fs::create_dir_all(defs.join("connectors")).unwrap();
        fs::create_dir_all(defs.join("upstream/forge")).unwrap();
        fs::write(
            defs.join("upstream/forge/openapi.json.gz"),
            hex::decode(SOURCE_GZ).unwrap(),
        )
        .unwrap();
        fs::create_dir_all(fixture.root.path().join("not-installed")).unwrap();
        fixture
    }

    fn defs(&self) -> PathBuf {
        self.root.path().join("defs")
    }

    fn definitions(&self) -> PathBuf {
        self.root.path().join("state/definitions")
    }

    /// The CLI with no controlling terminal and no installed provider.
    fn run(&self, args: &[&str]) -> Output {
        self.command(&self.root.path().join("not-installed"), args)
            .output()
            .unwrap()
    }

    /// The CLI with no controlling terminal, in its own session, deriving
    /// from the installation directory `installation`.
    fn command(&self, installation: &Path, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .env("CONNECTORS_INSTALLATION", installation)
            .args(["--output", "json", "--config"])
            .arg(self.root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .stdin(Stdio::null());
        // SAFETY: setsid in the child between fork and exec; no allocation.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command
    }

    /// `operations invoke --definition` of `connectors/<stem>.yaml`.
    fn invoke(&self, stem: &str, text: &str) -> Output {
        let file = self.write(stem, text);
        self.run(&invoke_args(&file))
    }

    fn write(&self, stem: &str, text: &str) -> String {
        let file = self.defs().join(format!("connectors/{stem}.yaml"));
        fs::write(&file, text).unwrap();
        file.to_str().unwrap().to_owned()
    }

    /// Every name directly under `<state>/definitions/` that is neither a
    /// cache entry directory nor `instances.toml`, with its mode.
    fn strays(&self) -> Vec<(String, u32)> {
        let Ok(names) = fs::read_dir(self.definitions()) else {
            return Vec::new();
        };
        names
            .map(|name| name.unwrap())
            .filter(|name| {
                let file_name = name.file_name().into_string().unwrap();
                let entry = file_name.len() == 64
                    && file_name.bytes().all(|b| b.is_ascii_hexdigit())
                    && name.file_type().unwrap().is_dir();
                !entry && file_name != "instances.toml"
            })
            .map(|name| {
                (
                    name.file_name().into_string().unwrap(),
                    name.metadata().unwrap().permissions().mode() & 0o777,
                )
            })
            .collect()
    }
}

/// Every path a native configuration of a cache entry names lies under that
/// entry's directory.
fn escaping(entry: &Path) -> Vec<String> {
    let native: serde_json::Value =
        serde_json::from_slice(&fs::read(entry.join("native.json")).unwrap()).unwrap();
    ["bundle_directory", "operations_file"]
        .iter()
        .filter_map(|key| native[key].as_str())
        .filter(|path| {
            let path = Path::new(path);
            !path.starts_with(entry)
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
        })
        .map(str::to_owned)
        .collect()
}

/// `catalog` is a provider id, and `..` passes `connectors_core::valid_id`.
/// The lowerer writes the declaration and selection to `<out>/<catalog>/`, so
/// a definition naming `catalog: '..'` would write them beside the cache
/// entries in `<state>/definitions/`, outside its own entry, where every such
/// definition shares (and replaces) them. The fixture itself lowers: the same
/// definition without `catalog` publishes one entry and leaves nothing beside it.
#[test]
fn security_a_catalog_name_never_places_lowered_files_outside_its_cache_entry() {
    let fixture = Fixture::new();
    let control = fixture.invoke("forge", &definition(None));
    let entries: Vec<PathBuf> = fs::read_dir(fixture.definitions())
        .unwrap_or_else(|error| panic!("{error}: {control:?}"))
        .map(|name| name.unwrap().path())
        .filter(|path| path.join("native.json").is_file())
        .collect();
    assert_eq!(entries.len(), 1, "the fixture lowers: {control:?}");
    assert!(fixture.strays().is_empty(), "{:?}", fixture.strays());

    let output = fixture.invoke("forge-up", &definition(Some("..")));
    let strays = fixture.strays();
    let escaped: Vec<(PathBuf, Vec<String>)> = fs::read_dir(fixture.definitions())
        .unwrap()
        .map(|name| name.unwrap().path())
        .filter(|path| path.join("native.json").is_file())
        .map(|path| {
            let escaping = escaping(&path);
            (path, escaping)
        })
        .filter(|(_, escaping)| !escaping.is_empty())
        .collect();
    assert!(
        strays.is_empty() && escaped.is_empty(),
        "beside the cache entries: {strays:?}; native configurations naming paths outside their entry: {escaped:?}; {output:?}"
    );
}

fn invoke_args(file: &str) -> Vec<&str> {
    vec![
        "operations",
        "invoke",
        "--definition",
        file,
        "--operation",
        "project.get",
        "--input-json",
        r#"{"id":"org/project"}"#,
    ]
}

/// Interruption is exit 130 with kind `interrupted`
/// (`contracts/cli/v1alpha1/semantics.md`; `protected::Signals`). Preparing a
/// definition installs the interrupt handler before it lowers and pins the
/// provider, so a SIGINT received then is caught rather than ending the
/// process; it must still end the command as interrupted, not be dropped
/// while preparation goes on (with a held connection, on to the invoke). The
/// window is widened deterministically with an installed provider stand-in
/// that marks its start and prints nothing for two seconds; the signal goes to
/// the CLI alone (the provider runs in its own process group). `/bin/sleep` is
/// where both Linux and macOS keep it (macOS has no `/usr/bin/sleep`, and a
/// stand-in that exits at once lets the refusal win the race).
#[test]
fn security_an_interrupt_while_a_definition_is_prepared_is_exit_130() {
    let fixture = Fixture::new();
    let installation = fixture.root.path().join("installation");
    fs::create_dir_all(&installation).unwrap();
    let provider = installation.join("connectors-catalog-provider");
    fs::write(
        &provider,
        "#!/bin/sh\n: > \"${0%/*}/started\"\nexec /bin/sleep 2\n",
    )
    .unwrap();
    fs::set_permissions(&provider, fs::Permissions::from_mode(0o700)).unwrap();
    let file = fixture.write("forge", &definition(None));
    let mut child = fixture
        .command(&installation, &invoke_args(&file))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = installation.join("started");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !started.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            panic!("the CLI ended before the provider started: {status:?} {output:?}");
        }
        assert!(
            std::time::Instant::now() < until,
            "the provider never started"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // SAFETY: the CLI child this test spawned and has not reaped.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGINT) }, 0);
    let output = child.wait_with_output().unwrap();
    let envelope: serde_json::Value =
        serde_json::from_slice(&output.stderr).unwrap_or(serde_json::Value::Null);
    assert!(
        output.status.code() == Some(130) && envelope["error"]["data"]["kind"] == "interrupted",
        "{output:?}"
    );
}

/// The connector and catalog are now plain names, but with an `id` the file
/// stem still names the bundle when `catalog` is absent: `connectors/...yaml`
/// has the stem `..`. The lowerer then writes its declaration and selection to
/// `<staging>/..`, beside the cache entries. The containment check refuses the
/// lowering only after those writes, and the cleanup removes only the staging
/// and copy directories, so nothing may be left beside the cache entries.
#[test]
fn security_a_dot_dot_file_stem_leaves_nothing_beside_the_cache_entries() {
    let fixture = Fixture::new();
    let text = definition(None).replacen('\n', "\nid: forge\n", 1);
    let output = fixture.invoke("..", &text);
    let strays = fixture.strays();
    assert!(
        strays.is_empty(),
        "beside the cache entries: {strays:?}; {output:?}"
    );
}

/// The CLI as [`Fixture::run`] starts it, with `--state-dir` spelled `state`.
fn run_in(fixture: &Fixture, state: &str, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command
        .env(
            "CONNECTORS_INSTALLATION",
            fixture.root.path().join("not-installed"),
        )
        .args(["--output", "json", "--config"])
        .arg(fixture.root.path().join("config/config.toml"))
        .args(["--state-dir", state])
        .args(args)
        .stdin(Stdio::null());
    // SAFETY: setsid in the child between fork and exec; no allocation.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.output().unwrap()
}

/// "A run whose inputs are unchanged reuses the entry." `--state-dir` is taken
/// as given and `/x/./state` is admitted as `/x/state`; a cache hit now
/// requires the native configuration to name the entry by the same string, so
/// a run under the other spelling removes the published entry
/// (`remove_dir_all`) and lowers it again.
#[test]
fn security_an_unchanged_definition_reuses_its_entry_under_another_spelling_of_the_state_directory()
{
    let fixture = Fixture::new();
    let file = fixture.write("forge", &definition(None));
    let identity = || -> Vec<(String, u64, i64)> {
        use std::os::unix::fs::MetadataExt;
        fs::read_dir(fixture.definitions())
            .unwrap()
            .map(|name| name.unwrap())
            .filter(|name| name.path().join("native.json").is_file())
            .map(|name| {
                let native = fs::metadata(name.path().join("native.json")).unwrap();
                (
                    name.file_name().into_string().unwrap(),
                    native.ino(),
                    native.mtime_nsec(),
                )
            })
            .collect()
    };
    let state = fixture.root.path().join("state");
    let first = run_in(&fixture, state.to_str().unwrap(), &invoke_args(&file));
    let lowered = identity();
    assert_eq!(lowered.len(), 1, "the fixture lowers: {first:?}");
    let respelled = format!("{}/./state", fixture.root.path().to_str().unwrap());
    let second = run_in(&fixture, &respelled, &invoke_args(&file));
    assert_eq!(identity(), lowered, "{second:?}");
}
