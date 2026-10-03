//! Shared public operation metadata projected from its owning ESS domain.
use super::Result;
use std::{collections::BTreeMap, path::Path, process::Command};

const OUTPUT: &str = "crates/connectors-core/generated/operation-types";

pub fn run(root: &Path, ess: &Path, check: bool) -> Result<()> {
    connectors_spec::toolchain::check(ess)?;
    if check {
        let base = root.join(".local/tmp");
        std::fs::create_dir_all(&base)?;
        let temp = tempfile::Builder::new()
            .prefix("operation-types-")
            .tempdir_in(base)?;
        let fresh = temp.path().join("types");
        generate(root, ess, &fresh)?;
        if artifacts(&fresh)? != artifacts(&root.join(OUTPUT))? {
            return Err("public operation metadata types drift; run service-metadata".into());
        }
    } else {
        generate(root, ess, &root.join(OUTPUT))?;
    }
    println!(
        "public operation metadata projection checked; admission remains a binding obligation"
    );
    Ok(())
}

fn generate(root: &Path, ess: &Path, output: &Path) -> Result<()> {
    super::run(
        Command::new(ess)
            .current_dir(root)
            .args([
                "generate",
                "types",
                "--path",
                "ess",
                "--root",
                "connectors.service_wire.OperationCuration",
                "--target",
                "rust",
                "--package",
                "connectors-operation-types",
                "--out",
            ])
            .arg(output),
    )?;
    Ok(())
}

fn artifacts(path: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = connectors_spec::v2::tree(path)?;
    files.retain(|name, _| !name.starts_with(".ess-output/"));
    Ok(files)
}
