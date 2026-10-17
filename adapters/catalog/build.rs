//! Compiles every shipped connector declaration into the provider binary, with
//! the selection files beside it and the bundle it runs on, so an instance
//! configured by connector name reads no file the user names
//! (`src/local.rs`, `Local::derive`). Every `providers/<name>/connector.json`
//! is shipped; a directory without one, such as Slack's, ships nothing here.
use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

fn sorted(directory: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    paths.sort();
    paths
}
fn name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .expect("a UTF-8 file name")
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let providers = root.join("providers");
    let bundles = root.join("generated/bundles");
    println!("cargo:rerun-if-changed=providers");
    println!("cargo:rerun-if-changed=generated/bundles");
    // A file by name and bytes; a connector by directory, declaration and the
    // files beside it.
    let mut out = String::from(
        "pub type File = (&'static str, &'static [u8]);\npub type Connector = (&'static str, &'static str, &'static [File]);\n",
    );
    writeln!(
        out,
        "pub const BUNDLE_INDEX: &[u8] = include_bytes!({:?});",
        bundles.join("index.json")
    )
    .unwrap();
    let mut connectors = String::new();
    let mut shipped_bundles = String::new();
    for directory in sorted(&providers) {
        let declaration = directory.join("connector.json");
        if !declaration.is_file() {
            continue;
        }
        let provider = name(&directory);
        let mut files = String::new();
        for file in sorted(&directory) {
            if file.is_file() && file != declaration && name(&file).ends_with(".json") {
                writeln!(files, "({:?}, include_bytes!({file:?})),", name(&file)).unwrap();
            }
        }
        writeln!(
            connectors,
            "({provider:?}, include_str!({declaration:?}), &[{files}]),"
        )
        .unwrap();
        let bundle = bundles.join(format!("{provider}.bundle.json"));
        if bundle.is_file() {
            writeln!(
                shipped_bundles,
                "({:?}, include_bytes!({bundle:?})),",
                name(&bundle)
            )
            .unwrap();
        }
    }
    writeln!(out, "pub const BUNDLES: &[File] = &[{shipped_bundles}];").unwrap();
    writeln!(out, "pub const CONNECTORS: &[Connector] = &[{connectors}];").unwrap();
    let target = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("shipped.rs");
    fs::write(target, out).expect("shipped.rs is written");
}
