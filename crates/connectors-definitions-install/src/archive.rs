//! The release asset: a gzip tar archive, as `git archive --prefix` writes it, whose
//! regular files under its one top directory are exactly the files the index names.
use super::{
    Refusal,
    index::{Index, relative_path, sha256},
};
use std::{collections::BTreeMap, io::Read};

fn invalid(reason: &str) -> Refusal {
    Refusal::ReleaseInvalid(reason.to_owned())
}

/// Checks `asset` against `index` and returns each file's bytes by path, in the
/// index's order. The asset's own digest first, then every entry.
pub(crate) fn verify(index: &Index, asset: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Refusal> {
    if sha256(asset) != index.asset.sha256 {
        return Err(Refusal::DigestMismatch(index.asset.name.clone()));
    }
    let expected: BTreeMap<&str, (&str, u64)> = index
        .files
        .iter()
        .map(|file| (file.path.as_str(), (file.sha256.as_str(), file.size)))
        .collect();
    let mut files = BTreeMap::new();
    let mut top: Option<String> = None;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(asset));
    let entries = archive
        .entries()
        .map_err(|_| invalid("the asset is not a gzip tar archive"))?;
    for entry in entries {
        let mut entry = entry.map_err(|_| invalid("the asset is not a gzip tar archive"))?;
        let kind = entry.header().entry_type();
        // `git archive` puts the commit in a pax global header first.
        if kind.is_pax_global_extensions() {
            continue;
        }
        let path = entry.path_bytes().into_owned();
        let path = String::from_utf8(path).map_err(|_| invalid("an entry path is not UTF-8"))?;
        let path = path.trim_end_matches('/');
        let (head, rest) = match path.split_once('/') {
            Some((head, rest)) => (head, Some(rest)),
            None => (path, None),
        };
        if !relative_path(head) {
            return Err(invalid("an entry is not under one top directory"));
        }
        match &top {
            Some(top) if top != head => {
                return Err(invalid("the entries are not under one top directory"));
            }
            Some(_) => {}
            None => top = Some(head.to_owned()),
        }
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            return Err(invalid(
                "an entry is neither a regular file nor a directory",
            ));
        }
        let Some(rest) = rest else {
            return Err(invalid("a file is not under the top directory"));
        };
        let Some((digest, size)) = expected.get(rest) else {
            return Err(invalid("the asset holds a file the index does not name"));
        };
        if files.contains_key(rest) {
            return Err(invalid("the asset holds a file twice"));
        }
        if entry.header().size().ok() != Some(*size) {
            return Err(Refusal::DigestMismatch(rest.to_owned()));
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(*size + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid("the asset is not a gzip tar archive"))?;
        if bytes.len() as u64 != *size || sha256(&bytes) != *digest {
            return Err(Refusal::DigestMismatch(rest.to_owned()));
        }
        files.insert(rest.to_owned(), bytes);
    }
    if files.len() != expected.len() {
        return Err(invalid("the asset lacks a file the index names"));
    }
    Ok(files)
}
