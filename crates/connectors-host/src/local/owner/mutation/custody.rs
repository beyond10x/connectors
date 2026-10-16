//! A write whose provider answer carried a credential it created, shown once
//! (`connectors.mutations.CredentialCustody`). The target the configuration
//! names is admitted before any provider work, and the bytes are held as a new
//! connection of that target before the attempt settles, exactly as a
//! protected-prompt connect would hold them (docs/local-mutation-ledger.md,
//! "Created credentials"). The bytes never reach a result, the ledger, an
//! audit record or an error.
use super::super::supervisor::{Output, Pool, Task};
use super::*;
use crate::local::{config::CustodyTarget, keyring::custody, registry, runtime};
use connectors_sdk::Secret;
use std::sync::Weak;

/// The custody target the configuration names for `operation`, if any.
pub(super) fn target<'a>(
    adapter: &'a crate::local::config::Adapter,
    operation: &str,
) -> Option<&'a CustodyTarget> {
    adapter.permissions.custody.get(operation)
}

/// Before any provider work: the target alias is configured, grants the
/// profile, is not stopped, and its custody binding is available and writable.
/// A created credential that could not be held is otherwise foreseeable.
pub(super) fn admit(paths: &Paths, target: &CustodyTarget) -> Result<(), Error> {
    let (config, adapter) = super::super::selected(paths, &target.adapter)?;
    if !adapter.permissions.profiles.contains(&target.profile) {
        return Err(Code::Forbidden.into());
    }
    let selection = config.selection();
    if !custody::available_for(&selection) {
        return Err(Code::CustodyUnavailable.into());
    }
    if selection.read_only() {
        return Err(Code::CustodyReadOnly.into());
    }
    if runtime::state::State::new(&paths.state).suppressed(&adapter.instance_id)? {
        return Err(Code::Unavailable.into());
    }
    Ok(())
}

/// After an applied commit whose result is `value` (the selection's public
/// members) and whose protected slot carried `secret`: the result the caller
/// receives, `{custody: stored, connection, <value>}`, or the error
/// `custody_unavailable` naming `<value>` as `created`, so the operator
/// can revoke the credential at the provider by its public identifier.
pub(super) fn hold(
    paths: &Paths,
    pool: &Weak<Pool>,
    target: &CustodyTarget,
    secret: Secret,
    value: Value,
    until: Instant,
) -> Result<Value, Failure> {
    let mut members = match value {
        Value::Object(members) => members,
        _ => serde_json::Map::new(),
    };
    match store(paths, pool, target, secret, until) {
        Ok(connection) => {
            members.insert("custody".into(), Value::from("stored"));
            members.insert("connection".into(), Value::from(connection));
            Ok(Value::Object(members))
        }
        Err(_) => {
            let created = members
                .into_iter()
                .filter_map(|(name, value)| Some((name, value.as_str()?.to_owned())))
                .collect();
            Err(Failure {
                created: Some(created),
                ..Failure::from(Code::CustodyUnavailable)
            })
        }
    }
}

/// The milliseconds of `until` left, at most `cap`, as a wall deadline.
fn deadline(until: Instant, cap: u64) -> u64 {
    let left = until.saturating_duration_since(Instant::now()).as_millis() as u64;
    connectors_sdk::now_ms() + left.min(cap)
}

/// Validate the bytes through the target alias, on its own worker, and publish
/// them as a new connection of the target profile.
fn store(
    paths: &Paths,
    pool: &Weak<Pool>,
    target: &CustodyTarget,
    secret: Secret,
    until: Instant,
) -> Result<String, Error> {
    let pool = pool.upgrade().ok_or(Code::Unavailable)?;
    issuance::check(until)?;
    admit(paths, target)?;
    let (config, adapter) = super::super::selected(paths, &target.adapter)?;
    let Output::Bootstrap(bootstrap, capture_epoch) = pool.run(
        &target.adapter,
        &adapter,
        Task::Ensure { resume: false },
        deadline(until, 30_000),
    )?
    else {
        return Err(Code::Unavailable.into());
    };
    let profile = bootstrap.profile(&target.profile)?;
    let [field] = profile.fields.as_slice() else {
        return Err(Code::InvalidConfiguration.into());
    };
    if profile.acquisition.is_some() {
        return Err(Code::InvalidConfiguration.into());
    }
    let document = document(field, &secret)?;
    drop(secret);
    let binding = bootstrap.binding(&target.profile)?;
    let registry = registry::Registry::with_system_clock(&paths.state);
    let acquired = registry.begin(&binding, connectors_sdk::now_ms())?;
    let claim = registry.consume(acquired, connectors_sdk::now_ms())?;
    let stored = (|| {
        let Output::Baseline(baseline) = pool.run(
            &target.adapter,
            &adapter,
            Task::Validate {
                profile: target.profile.clone(),
                secret: Secret(document.0.clone()),
                capture_epoch,
            },
            deadline(until, 30_000),
        )?
        else {
            return Err(Error::from(Code::Unavailable));
        };
        let (latest_config, latest) = super::super::selected(paths, &target.adapter)?;
        if latest.selection() != adapter.selection()
            || latest_config.selection() != config.selection()
        {
            return Err(Code::LifecycleConflict.into());
        }
        if !latest.permissions.profiles.contains(&target.profile) {
            return Err(Code::Forbidden.into());
        }
        let prepared = registry.prepare(
            &claim,
            baseline.into_registry(),
            document.0.len(),
            connectors_sdk::now_ms(),
        )?;
        let store = custody::Store::open_for(
            prepared.version().scope(),
            &config.selection(),
            &paths.state,
        )
        .map_err(|_| Code::CustodyUnavailable)?;
        Ok(registry.store_and_publish(prepared, &document, &store, connectors_sdk::now_ms)?)
    })();
    if let Err(error) = &stored
        && error.code != Code::OutcomeUnknown
    {
        let _ = registry.fail(&claim, connectors_sdk::now_ms());
    }
    stored
}

/// The protected document a prompt builds for a profile of one entry field:
/// `{"<field>": "<secret>"}`, sized up front so no copy of the bytes is left
/// behind by a reallocation, and zeroized on drop.
fn document(field: &runtime::EntryField, secret: &Secret) -> Result<Secret, Error> {
    let value = std::str::from_utf8(&secret.0).map_err(|_| Code::InvalidInput)?;
    if value.is_empty() || value.len() > field.max_bytes as usize {
        return Err(Code::InvalidInput.into());
    }
    // A JSON string escapes a byte in at most six.
    let mut document = Secret(Vec::with_capacity(
        field.name.len() * 6 + value.len() * 6 + 8,
    ));
    document.0.push(b'{');
    serde_json::to_writer(&mut document.0, &field.name).map_err(|_| Code::InvalidInput)?;
    document.0.push(b':');
    serde_json::to_writer(&mut document.0, value).map_err(|_| Code::InvalidInput)?;
    document.0.push(b'}');
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_document_is_the_one_a_prompt_builds_for_a_single_field() {
        let field = runtime::EntryField {
            name: "token".into(),
            label: "Token".into(),
            max_bytes: 32,
        };
        let document = document(&field, &Secret(b"fictional-\"quoted\"".to_vec())).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&document.0).unwrap(),
            serde_json::json!({"token": "fictional-\"quoted\""})
        );
        assert!(document.0.capacity() >= document.0.len());
        for refused in [b"".to_vec(), vec![b'a'; 33], vec![0xff]] {
            assert!(super::document(&field, &Secret(refused)).is_err());
        }
    }
}
