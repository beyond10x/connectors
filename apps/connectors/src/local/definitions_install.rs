//! `connectors definitions install` (story:definitions-install;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Installing a definitions release"):
//! fetches a connectors-definitions release by tag, checks every file against its
//! `index.json`, and records it in `definitions.lock` beside the configuration
//! file and a read-only cache entry under `<state>/installed-definitions/`. The
//! configuration was read and checked before this runs; nothing here reads it, and
//! nothing here starts a process or grants anything.
use super::{failure, resolved};
use connectors_cli_contract::{HandlerReply, Invocation};
use connectors_definitions_install::{CACHE_DIRECTORY, LOCK_FILE, Refusal, Source, install};
use serde_json::{Value, json};

pub(super) fn execute(call: &Invocation<'_>) -> Result<Value, HandlerReply> {
    let paths = resolved(call)?;
    let text = |field: &str| call.input[field].as_str().unwrap_or_default().to_owned();
    let source = Source::parse(&text("source")).map_err(refused)?;
    let lock_path = paths
        .config
        .parent()
        .map(|directory| directory.join(LOCK_FILE))
        .ok_or_else(|| refused(Refusal::WriteFailed))?;
    let cache_root = paths.state.join(CACHE_DIRECTORY);
    let installed = install(&source, &text("tag"), &cache_root, &lock_path).map_err(refused)?;
    Ok(json!({
        "source": installed.lock.source,
        "tag": installed.lock.tag,
        "commit": installed.lock.commit,
        "index_sha256": installed.lock.index_sha256,
        "features": installed.features,
        "files": installed.files,
        "lock_path": installed.lock_path,
        "cache_path": installed.cache_path,
        "cache_entry": if installed.created { "created" } else { "existing" },
    }))
}

/// The failure each refusal is reported as; `feature` and `file` name what an
/// unmet feature and a digest mismatch refer to.
fn refused(refusal: Refusal) -> HandlerReply {
    let (stage, next_action, usage) = match &refusal {
        Refusal::InvalidSource | Refusal::InvalidTag => ("arguments", "none", true),
        Refusal::NotFound => ("dispatch", "none", false),
        Refusal::Unavailable => ("dispatch", "retry_explicitly", false),
        Refusal::UnknownFormatVersion | Refusal::UnmetFeature(_) => ("admission", "upgrade", false),
        Refusal::ReleaseInvalid(_) | Refusal::DigestMismatch(_) => ("admission", "none", false),
        Refusal::WriteFailed => ("publication", "retry_explicitly", false),
    };
    let mut reply = failure(refusal.code(), stage, next_action, usage);
    if let HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. } = &mut reply {
        match refusal {
            Refusal::UnmetFeature(feature) => data["feature"] = json!(feature),
            Refusal::DigestMismatch(file) => data["file"] = json!(file),
            _ => {}
        }
    }
    reply
}
