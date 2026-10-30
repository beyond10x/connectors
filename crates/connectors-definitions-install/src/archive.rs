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

/// Tar header room allowed per indexed file: its own header block, a pax or GNU
/// long-name record, and the directories above it.
const HEADER_ROOM_PER_FILE: u64 = 8 * 1024;
/// Tar header room allowed once: the pax global header, the top directory and the
/// end-of-archive record padding.
const HEADER_ROOM: u64 = 1024 * 1024;

/// The most bytes the asset may inflate to: the files the index names, each padded
/// to a 512-byte block, and bounded header room. Nothing an archive declares (a
/// long-name or pax record of any size) is read past it.
fn inflated_bound(index: &Index) -> u64 {
    let files = index.files.len() as u64;
    let content: u64 = index
        .files
        .iter()
        .map(|file| file.size.div_ceil(512) * 512)
        .sum();
    content + files * HEADER_ROOM_PER_FILE + HEADER_ROOM
}

/// A reader that fails once more than `left` bytes were read from `inner`.
struct Capped<R> {
    inner: R,
    left: u64,
}

impl<R: Read> Read for Capped<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.left = self.left.checked_sub(read as u64).ok_or_else(|| {
            std::io::Error::other("the asset inflates past the files its index names")
        })?;
        Ok(read)
    }
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
    let inflated = Capped {
        inner: flate2::read::GzDecoder::new(asset),
        left: inflated_bound(index),
    };
    let mut archive = tar::Archive::new(inflated);
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
