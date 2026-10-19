//! Shared helpers for generated and checked public files: hashing, canonical JSON,
//! file writes, the pinned ESS check and generated-tree reads.
use connectors_core::{Error, Result, canonical};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

pub fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn refuse(message: impl Into<String>) -> Error {
    Error::invalid(message)
}
pub fn json_bytes(v: &impl Serialize) -> Result<Vec<u8>> {
    let mut b = canonical(&serde_json::to_value(v).map_err(|_| Error::internal())?);
    b.push(b'\n');
    Ok(b)
}
pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::create_dir_all(path.parent().ok_or_else(Error::internal)?)
        .map_err(|e| refuse(format!("create {}: {e}", path.display())))?;
    std::fs::write(path, bytes).map_err(|e| refuse(format!("write {}: {e}", path.display())))
}
pub fn check_ess(executable: &Path) -> Result<()> {
    crate::toolchain::check(executable)
}

pub fn tree(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for entry in std::fs::read_dir(path).map_err(|_| Error::internal())? {
            let entry = entry.map_err(|_| Error::internal())?;
            let path = entry.path();
            let ty = entry.file_type().map_err(|_| Error::internal())?;
            if ty.is_symlink() {
                return Err(refuse("generated tree contains a symlink"));
            }
            if ty.is_dir() {
                visit(root, &path, files)?;
            } else {
                files.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    std::fs::read(path).map_err(|_| Error::internal())?,
                );
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}
