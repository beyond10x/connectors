//! `setup custody-orphans` (contracts/cli/v1alpha1/semantics.md §3, "Custody
//! orphans"): lists the Secret Service custody items the store of this state
//! directory does not name, grouped by authority, and with `--mode remove
//! --authority <id>` deletes the items of that one authority. Another store of
//! this machine may hold a listed authority; the command cannot see it, so it
//! never removes more than the one authority it is told. Item attributes are
//! read; no secret value ever is.
use super::failure;
use connectors_cli_contract::{HandlerReply, Invocation};
use connectors_host::local::config::Paths;
use serde_json::{Value, json};

/// `invalid_input` at `arguments`, naming the refused argument.
fn refused(field: &str) -> HandlerReply {
    let mut reply = failure("invalid_input", "arguments", "none", true);
    if let HandlerReply::UsageError { data, .. } = &mut reply {
        data["field"] = json!(field);
    }
    reply
}

const NOTICE: &str = "An authority listed here is not the store of this state directory. It may belong to another store on this machine that this command cannot see. Remove one authority only after checking that no store holds it: setup custody-orphans --mode remove --authority <id>.";

#[cfg(target_os = "linux")]
pub(super) fn execute(call: &Invocation<'_>, paths: &Paths) -> Result<Value, HandlerReply> {
    use super::{Config, configuration_refusal, host_failure};
    use connectors_host::local::{
        keyring::{self, CustodyItem},
        metadata::Metadata,
    };
    use std::collections::{BTreeMap, HashSet};
    use uuid::Uuid;
    let mode = match call.input.get("mode").and_then(Value::as_str) {
        None | Some("list") => "list",
        Some("remove") => "remove",
        Some(_) => return Err(refused("mode")),
    };
    let authority = call.input.get("authority").and_then(Value::as_str);
    let target = match (mode, authority) {
        ("list", Some(_)) => return Err(refused("authority")),
        ("remove", None) => return Err(refused("authority")),
        (_, Some(text)) => Some(text.parse::<Uuid>().map_err(|_| refused("authority"))?),
        (_, None) => None,
    };
    let config = Config::read(&paths.config).map_err(configuration_refusal)?;
    // The store that names the authority must exist, and its own authority is
    // never removed: its custody is live.
    let own = Metadata::authority_at(&paths.state).map_err(host_failure)?;
    if target == Some(own) {
        return Err(refused("authority"));
    }
    let socket = config.secret_service_socket.as_deref();
    let unavailable = |_| failure("custody_unavailable", "custody", "unlock_keyring", false);
    let row = |item: &CustodyItem| {
        json!({
            "item": item.path,
            "purpose": item.purpose.label(),
            "authority": item.authority,
            "scope": item.allocation,
            "version": item.version,
        })
    };
    let (mut groups, mut removed, mut failed) = (Vec::new(), Vec::new(), Vec::new());
    if let Some(target) = target {
        let removal = keyring::custody_remove_authority(socket, target).map_err(unavailable)?;
        removed = removal.deleted.iter().map(row).collect();
        failed = removal
            .failed
            .iter()
            .map(|(item, why)| json!({"item": item.path, "failure": format!("{why:?}")}))
            .collect();
        // The deletes ran but could not be made durable: `removed` still names
        // them, and this row names the failure.
        if let Some(why) = removal.synchronization {
            failed.push(json!({"item": "synchronization", "failure": format!("{why:?}")}));
        }
    } else {
        let found =
            keyring::custody_orphans(socket, &HashSet::from([own]), false).map_err(unavailable)?;
        let mut counts: BTreeMap<(String, &str), u64> = BTreeMap::new();
        for item in &found.orphans {
            *counts
                .entry((item.authority.clone(), item.purpose.label()))
                .or_default() += 1;
        }
        groups = counts
            .into_iter()
            .map(|((authority, purpose), count)| {
                json!({"authority": authority, "purpose": purpose, "count": count})
            })
            .collect();
    }
    Ok(json!({
        "mode": mode,
        "state_path": paths.state,
        "groups": groups,
        "notice": NOTICE,
        "removed": removed,
        "failed": failed,
    }))
}

#[cfg(not(target_os = "linux"))]
pub(super) fn execute(_: &Invocation<'_>, _: &Paths) -> Result<Value, HandlerReply> {
    let _ = (NOTICE, refused);
    Err(failure(
        "custody_unavailable",
        "custody",
        "unlock_keyring",
        false,
    ))
}
