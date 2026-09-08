//! Local executor for the bounded Linux adapter service realization.
use clap::{Parser, Subcommand};
use connectors_spec::v2::{check_ess, generate, hash, json_bytes, tree, write};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Command, Output},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Parser)]
#[command(about = "Check generation or build an independently runnable local adapter image")]
struct Args {
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[arg(long, default_value = "ess")]
    ess: PathBuf,
    #[arg(long, default_value = "adapters/gitlab/realizations/local.json")]
    declaration: PathBuf,
    #[command(subcommand)]
    command: Action,
}
#[derive(Subcommand)]
enum Action {
    Check,
    Package {
        /// A new task-owned directory; existing paths are refused.
        #[arg(long)]
        output: PathBuf,
        /// Local Docker tag, never pushed by this executor.
        #[arg(long, default_value = "connectors-v2-gitlab:local")]
        image: String,
        #[arg(long, default_value_t = 2)]
        jobs: usize,
    },
}
fn run(command: &mut Command) -> Result<Output> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "command {:?} failed: {}{}",
            command.get_program(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(output)
}
fn save_json(path: &Path, value: &Value) -> Result<()> {
    Ok(write(path, &json_bytes(value)?)?)
}
fn main() -> Result<()> {
    let args = Args::parse();
    let root = args.root.canonicalize()?;
    check_ess(&args.ess)?;
    let declaration: LocalService =
        serde_json::from_slice(&std::fs::read(inside(&root, &args.declaration)?)?)?;
    declaration.validate()?;
    let generated = inside(&root, Path::new(&declaration.generated))?;
    let specification = inside(&root, Path::new(&declaration.specification))?;
    let spec = connectors_spec::v2::Spec::parse(&std::fs::read(&specification)?)?;
    if spec.id != declaration.adapter {
        return Err("realization adapter disagrees with specification".into());
    }
    generate(&specification, &generated, &args.ess, true)?;
    run(Command::new(&args.ess)
        .args(["validate", "--path"])
        .arg(root.join("ess")))?;
    if let Action::Package {
        output,
        image,
        jobs,
    } = args.command
    {
        if std::env::consts::OS != "linux" || std::env::consts::ARCH != "x86_64" {
            return Err("local packaging currently requires Linux x86_64".into());
        }
        if jobs == 0 || jobs > 8 {
            return Err("jobs must be between 1 and 8".into());
        }
        let output = if output.is_absolute() {
            output
        } else {
            root.join(output)
        };
        if output.exists() || output.symlink_metadata().is_ok() {
            return Err("output must be a new task-owned directory".into());
        }
        std::fs::create_dir_all(&output)?;
        let build = run(Command::new("cargo")
            .current_dir(&root)
            .args([
                "build",
                "--locked",
                "--offline",
                "-p",
                &declaration.package,
                "--bin",
                &declaration.binary,
                "--jobs",
            ])
            .arg(jobs.to_string()))?;
        write(&output.join("cargo-build.log"), &build.stderr)?;
        let metadata = run(Command::new("cargo").current_dir(&root).args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--locked",
            "--offline",
        ]))?;
        let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
        let binary = Path::new(
            metadata["target_directory"]
                .as_str()
                .ok_or("missing Cargo target directory")?,
        )
        .join("debug")
        .join(&declaration.binary);
        let rootfs = output.join("rootfs");
        copy_file(
            &binary,
            &rootfs.join(format!("usr/local/bin/{}", declaration.binary)),
        )?;
        let libraries = run(Command::new("ldd").arg(&binary))?;
        let libraries = String::from_utf8(libraries.stdout)?;
        if libraries.contains("not found") {
            return Err("binary has an unresolved shared library".into());
        }
        for path in libraries.split_whitespace().filter(|s| s.starts_with('/')) {
            copy_file(Path::new(path), &rootfs.join(path.trim_start_matches('/')))?;
        }
        copy_file(
            Path::new("/etc/ssl/certs/ca-certificates.crt"),
            &rootfs.join("etc/ssl/certs/ca-certificates.crt"),
        )?;
        // Retain relevant system redistribution notices when provided by this host.
        for (source, destination) in [
            (
                "/usr/share/licenses/glibc/COPYING.LIB",
                "usr/share/licenses/glibc/COPYING.LIB",
            ),
            (
                "/usr/share/licenses/gcc-libs/RUNTIME.LIBRARY.EXCEPTION",
                "usr/share/licenses/gcc-libs/RUNTIME.LIBRARY.EXCEPTION",
            ),
            (
                "/usr/share/licenses/gcc-libs/COPYING",
                "usr/share/licenses/gcc-libs/COPYING",
            ),
            (
                "/usr/share/doc/libc6/copyright",
                "usr/share/doc/libc6/copyright",
            ),
            (
                "/usr/share/doc/libgcc-s1/copyright",
                "usr/share/doc/libgcc-s1/copyright",
            ),
        ] {
            if Path::new(source).is_file() {
                copy_file(Path::new(source), &rootfs.join(destination))?;
            }
        }
        let files = tree(&rootfs)?;
        let contents: BTreeMap<_, _> = files.iter().map(|(p, b)| (p.clone(), hash(b))).collect();
        save_json(&output.join("rootfs.sha256.json"), &json!(contents))?;
        let graph = json!({"format":"ess-build/1","build":format!("{}-local",declaration.adapter),"platforms":[json!({"os":declaration.platform.os,"architecture":declaration.platform.architecture})],"nodes":[
            {"id":"payload","kind":"source","path":"rootfs","destination":"/"},
            {"id":"service","kind":"image","rootfs":"payload","config":{"entrypoint":declaration.entrypoint,"command":declaration.command,"user":declaration.user,"workdir":"/","environment":{"SSL_CERT_FILE":"/etc/ssl/certs/ca-certificates.crt"}}}
        ],"outputs":[{"name":declaration.adapter,"release_unit":declaration.package,"node":"service","kind":"oci_image","repository":declaration.repository}]});
        save_json(&output.join("build.json"), &graph)?;
        run(Command::new(&args.ess)
            .args(["build", "compile", "--path"])
            .arg(output.join("build.json"))
            .arg("--out")
            .arg(output.join("build-ir.json")))?;
        // ESS 0.9.2 omits empty secrets on serialization but requires the
        // field on IR input. Retain canonical bytes and make only this explicit
        // empty-default adaptation for the pinned reader.
        let mut projectable: Value =
            serde_json::from_slice(&std::fs::read(output.join("build-ir.json"))?)?;
        if projectable.get("secrets").is_none() {
            projectable["secrets"] = json!([]);
        }
        save_json(&output.join("build-ir.projectable.json"), &projectable)?;
        run(Command::new(&args.ess)
            .args(["project", "buildkit", "--ir"])
            .arg(output.join("build-ir.projectable.json"))
            .arg("--out")
            .arg(output.join("buildkit")))?;
        let image_build = run(Command::new("docker")
            .arg("build")
            .arg("--network=none")
            .arg("--tag")
            .arg(&image)
            .arg("--iidfile")
            .arg(output.join("image.id"))
            .arg("--file")
            .arg(output.join("buildkit/Dockerfile.ess"))
            .arg(&output))?;
        write(
            &output.join("docker-build.log"),
            &[image_build.stdout, image_build.stderr].concat(),
        )?;
        let image_id = std::fs::read_to_string(output.join("image.id"))?
            .trim()
            .to_owned();
        let inspection = run(Command::new("docker").args(["image", "inspect", &image_id]))?;
        let inspection: Value = serde_json::from_slice(&inspection.stdout)?;
        save_json(&output.join("image-inspect.json"), &inspection)?;
        let plan: Value =
            serde_json::from_slice(&std::fs::read(generated.join("rust/plan.json"))?)?;
        let requests: Value =
            serde_json::from_slice(&std::fs::read(generated.join("ess/domains/requests.yaml"))?)?;
        let surfaces: Vec<_> = requests["types"]
            .as_array()
            .ok_or("missing request types")?
            .iter()
            .map(|t| json!({"kind":"type","name":t["name"]}))
            .collect();
        let component = format!("{}-adapter", declaration.adapter);
        let realization = json!({"type":"ess-realization/1","id":format!("{}-local",declaration.adapter),"specification":{"system":declaration.adapter,"version":"v1","source_digest":format!("sha256:{}",plan["provenance"]["source_digest"].as_str().ok_or("missing ESS source digest")?)},"synthesis":{"target":"rust-linux-x86_64/1","generator":"ess/0.9.2"},"components":[component],"actors":[],"implementations":[{"id":"adapter-image","components":[component],"artifact":{"kind":"container","locator":format!("docker-daemon:{image}"),"identity":image_id}}],"entrypoints":[{"id":"http-service","title":format!("Configured {} HTTP service",declaration.adapter),"summary":"Generated typed requests and explicit provider bindings served by the asynchronous host.","primary":true,"interaction":"invoke","attachment":"network","availability":"internal","support":"preview","implementation":"adapter-image","actors":[],"surfaces":surfaces,"invocation":{"kind":"argv","argv":declaration.entrypoint.iter().chain(&declaration.command).collect::<Vec<_>>()},"requires":[{"kind":"filesystem","name":"configuration","summary":"Read-only configuration mounted at /config/service.json."},{"kind":"credential","name":"service-token","summary":"Owner-only caller credential mounted separately under /secrets."},{"kind":"network","name":"provider-api","summary":"Reachability to the explicitly configured provider HTTPS endpoint."}]}]});
        save_json(&output.join("realization.json"), &realization)?;
        run(Command::new(&args.ess)
            .args(["realization", "validate", "--path"])
            .arg(output.join("realization.json"))
            .arg("--spec")
            .arg(generated.join("ess")))?;
        let realized = run(Command::new(&args.ess)
            .args(["realization", "compile", "--path"])
            .arg(output.join("realization.json"))
            .arg("--spec")
            .arg(generated.join("ess"))
            .args(["--format", "json"]))?;
        write(&output.join("realization-ir.json"), &realized.stdout)?;
        let sources = source_hashes(&root)?;
        save_json(&output.join("source.sha256.json"), &json!(sources))?;
        let rustc = run(Command::new("rustc").arg("--version"))?;
        let git = run(Command::new("git")
            .current_dir(&root)
            .args(["rev-parse", "HEAD"]))?;
        let evidence = json!({"format":"connectors.local-build/v1","image":image,"image_id":image_id,"binary_sha256":hash(&std::fs::read(&binary)?),"source_head":String::from_utf8(git.stdout)?.trim(),"source_manifest_sha256":hash(&std::fs::read(output.join("source.sha256.json"))?),"rootfs_manifest_sha256":hash(&std::fs::read(output.join("rootfs.sha256.json"))?),"generation_manifest_sha256":hash(&std::fs::read(generated.join("manifest.json"))?),"rustc":String::from_utf8(rustc.stdout)?.trim(),"ess":"0.9.2","profile":"dev","platform":json!({"os":declaration.platform.os,"architecture":declaration.platform.architecture}),"validation":["generation drift check","ESS declarations","ESS build compile","ESS BuildKit projection","Docker build","ESS realization validate and compile"],"publication":"local only","runtime_acceptance":"separate conformance evidence required"});
        save_json(&output.join("build-evidence.json"), &evidence)?;
        println!("{}", serde_json::to_string_pretty(&evidence)?);
    } else {
        println!("generation and ESS declarations match");
    }
    Ok(())
}
fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination.parent().ok_or("missing destination parent")?)?;
    std::fs::copy(source, destination)?;
    Ok(())
}
fn source_hashes(root: &Path) -> Result<BTreeMap<String, String>> {
    let list = run(Command::new("git").current_dir(root).args([
        "ls-files",
        "-c",
        "-o",
        "--exclude-standard",
        "-z",
    ]))?;
    let mut sources = BTreeMap::new();
    for name in String::from_utf8(list.stdout)?
        .split('\0')
        .filter(|s| !s.is_empty())
    {
        sources.insert(name.to_owned(), hash(&std::fs::read(root.join(name))?));
    }
    Ok(sources)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Platform {
    os: String,
    architecture: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalService {
    format: String,
    adapter: String,
    package: String,
    binary: String,
    specification: String,
    generated: String,
    platform: Platform,
    entrypoint: Vec<String>,
    command: Vec<String>,
    user: String,
    repository: String,
}
impl LocalService {
    fn validate(&self) -> Result<()> {
        let name = |s: &str| {
            !s.is_empty()
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        };
        if self.format != "connectors.local-service/1"
            || ![&self.adapter, &self.package, &self.binary]
                .iter()
                .all(|s| name(s))
            || self.platform.os != "linux"
            || self.platform.architecture != "amd64"
            || self.entrypoint != [format!("/usr/local/bin/{}", self.binary)]
            || self.command != ["--config", "/config/service.json"]
            || self.user != "65532:65532"
            || self.repository.is_empty()
        {
            return Err("unsupported local-service declaration".into());
        }
        Ok(())
    }
}
fn inside(root: &Path, path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Err("declaration paths must be repository-relative".into());
    }
    let path = root.join(path).canonicalize()?;
    if !path.starts_with(root) {
        return Err("declaration path escapes the repository".into());
    }
    Ok(path)
}
