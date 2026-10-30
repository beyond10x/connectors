//! story:mounted-secret-custody. The production CLI, owner and an adapter
//! child against the read-only `mounted_secret` binding, selected explicitly
//! by the owner configuration, in a jail with a read-only custody directory
//! (`support/mounted.rs`). Linux only: the binding is.
#![cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;
use mounted::*;
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

const ADAPTER_FIXTURE: &str = "mounted_custody_adapter_fixture";

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn mounted_custody_adapter_fixture() {
    serve_when_child();
}

/// A container with the item delivered, configured with `custody`, and state
/// initialized by `setup init`, which keeps the existing configuration.
fn container(format: &str, custody: &str) -> Container {
    let container = Container::new(ADAPTER_FIXTURE);
    container.deliver(PROFILE, SECRET);
    container.configure(format, custody);
    let init = success(&container.run(&["setup", "init"]));
    assert_eq!(init["disposition"], "state_initialized", "{init}");
    container
}

/// Selection is explicit and never a fallback: without `[custody]` the
/// platform binding answers `custody_unavailable` even beside a valid mounted
/// directory; `[custody]` belongs to `connectors-local/4` only and never names
/// the mounted binding beside a Secret Service socket; and `setup check`
/// reports the selected binding.
#[test]
fn mounted_selection_is_explicit() {
    let container = container("connectors-local/4", "");
    let check = success(&container.run(&["setup", "check"]));
    assert_eq!(check["custody_backend"], "secret_service", "{check}");
    // No session bus exists in the jail: the platform binding is unusable,
    // and the valid mounted directory is not consulted.
    let token = container.path("/srv/connectors/token");
    fs::write(&token, SECRET).unwrap();
    fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).unwrap();
    let (code, data) = refusal(&container.run(&[
        "connections",
        "connect",
        "--adapter",
        ALIAS,
        "--profile",
        PROFILE,
        "--credential-file",
        "/srv/connectors/token",
    ]));
    assert_eq!(
        (code, &data["code"], &data["stage"]),
        (Some(1), &json!("custody_unavailable"), &json!("custody")),
        "{data}"
    );
    assert!(container.seen("validations").is_empty());

    // `[custody]` in a `connectors-local/3` file refuses the configuration.
    container.configure("connectors-local/3", &Container::mounted());
    let (code, data) = refusal(&container.run(&["setup", "check"]));
    assert_eq!(
        (code, &data["code"]),
        (Some(2), &json!("invalid_configuration")),
        "{data}"
    );

    // The mounted binding beside a Secret Service socket refuses it too.
    container.configure(
        "connectors-local/4",
        &format!(
            "secret_service_socket='/run/user/1/bus'\n{}",
            Container::mounted()
        ),
    );
    let (code, data) = refusal(&container.run(&["setup", "check"]));
    assert_eq!(
        (code, &data["code"]),
        (Some(2), &json!("invalid_configuration")),
        "{data}"
    );

    // A directory without the mounted binding, and the mounted binding
    // without a directory, refuse it as well.
    for custody in [
        format!("[custody]\nbinding='secret_service'\ndirectory='{CUSTODY}'\n"),
        "[custody]\nbinding='mounted_secret'\n".to_owned(),
    ] {
        container.configure("connectors-local/4", &custody);
        let (code, data) = refusal(&container.run(&["setup", "check"]));
        assert_eq!(
            (code, &data["code"]),
            (Some(2), &json!("invalid_configuration")),
            "{custody}"
        );
    }

    // Selected, `setup check` names it and its prerequisite.
    container.configure("connectors-local/4", &Container::mounted());
    let check = success(&container.run(&["setup", "check"]));
    assert_eq!(check["custody_backend"], "mounted_secret", "{check}");
    let prerequisites = check["prerequisites"].as_array().unwrap();
    assert!(
        prerequisites
            .iter()
            .any(|p| p["name"] == "mounted_custody" && p["state"] == "ready"),
        "{check}"
    );
    assert!(
        !prerequisites.iter().any(|p| p["name"] == "keyring"),
        "the platform store is not a prerequisite of the mounted binding: {check}"
    );
}

/// `connect` takes no protected source under the read-only binding: the owner
/// reads the item itself, the provider validates those bytes and the
/// connection's reads use them. A source is `invalid_input`; a second
/// connect on the instance is `lifecycle_conflict` naming a reconnect. Neither the mount nor
/// the state directory gains a copy of the bytes.
#[test]
fn mounted_connect_reads_never_writes() {
    let container = container("connectors-local/4", &Container::mounted());
    let before = container.snapshot(SOURCE);
    let (code, data) = refusal(&container.run(&[
        "connections",
        "connect",
        "--adapter",
        ALIAS,
        "--profile",
        PROFILE,
        "--credential-stdin",
    ]));
    assert_eq!(
        (code, &data["code"], &data["stage"]),
        (Some(2), &json!("invalid_input"), &json!("arguments")),
        "{data}"
    );
    let connected = success(&container.connect(PROFILE));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        connected["connection"]["summary"]["state"], "ready",
        "{connected}"
    );
    assert_eq!(container.seen("validations"), [digest(SECRET)]);
    let listed = success(&container.run(&["connections", "list", "--adapter", ALIAS]));
    assert_eq!(
        listed["connections"][0]["connection"],
        connection.as_str(),
        "{listed}"
    );
    let read = success(&container.invoke(&connection));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");

    // The instance holds its connection: a second connect is refused before
    // any validation, naming the reconnect that replaces its credential
    // (story:connect-refused-while-instance-holds-connection).
    let (code, data) = refusal(&container.connect(PROFILE));
    assert_eq!(
        (code, &data["code"], &data["stage"], &data["next_action"]),
        (
            Some(1),
            &json!("lifecycle_conflict"),
            &json!("admission"),
            &json!("repair_connection")
        ),
        "{data}"
    );
    assert_eq!(
        container.seen("validations").len(),
        1,
        "no second validation"
    );
    assert_eq!(container.snapshot(SOURCE), before, "the mount is unchanged");
    assert!(
        !container.state_holds(SECRET),
        "no copy of the bytes in state"
    );
}

/// Rotation: the platform replaces the item, a read under the old version is
/// refused naming a revalidation and never sends the new bytes, and
/// `revalidate` pins them as a new version that the next read uses.
#[test]
fn mounted_rotation_requires_revalidate() {
    let container = container("connectors-local/4", &Container::mounted());
    let connected = success(&container.connect(PROFILE));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["connection"]["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();
    let rotated: &[u8] = b"fictional-mounted-token-0002";
    container.rotate("..2026_10_14_00_00_00.000000002", rotated);
    let (code, data) = refusal(&container.invoke(&connection));
    assert_eq!(
        (code, &data["code"], &data["next_action"]),
        (
            Some(1),
            &json!("custody_unavailable"),
            &json!("revalidate_connection")
        ),
        "{data}"
    );
    assert!(
        !container.seen("invocations").contains(&digest(rotated)),
        "the new bytes were sent before a revalidation"
    );
    let revalidated = success(&container.run(&[
        "connections",
        "revalidate",
        "--adapter",
        ALIAS,
        "--connection",
        &connection,
        "--expected-revision",
        &revision,
    ]));
    assert_eq!(
        revalidated["connection"]["summary"]["state"], "ready",
        "{revalidated}"
    );
    assert_eq!(container.seen("validations").last(), Some(&digest(rotated)));
    let read = success(&container.invoke(&connection));
    assert_eq!(read["result"]["credential"], digest(rotated), "{read}");
}

/// Revocation is metadata only: the mounted item stays in place and nothing
/// under the mount changes.
#[test]
fn mounted_revoke_leaves_the_item_in_place() {
    let container = container("connectors-local/4", &Container::mounted());
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    let before = container.snapshot(SOURCE);
    let revoked = success(&container.run(&[
        "connections",
        "revoke",
        "--adapter",
        ALIAS,
        "--connection",
        summary["connection"].as_str().unwrap(),
        "--expected-revision",
        summary["revision"].as_str().unwrap(),
    ]));
    assert_eq!(revoked["local_state"], "revoked", "{revoked}");
    assert_eq!(container.snapshot(SOURCE), before);
    // A new connection of the profile is admitted once the old one is revoked.
    success(&container.connect(PROFILE));
}

/// A refresh-rotating (OAuth-acquired) profile and approval-signing key
/// creation each answer `custody_read_only` before any provider exchange or
/// write.
#[test]
fn mounted_refuses_refresh_and_approval_keys() {
    let container = container("connectors-local/4", &Container::mounted());
    container.deliver(OAUTH_PROFILE, SECRET);
    let (code, data) = refusal(&container.connect(OAUTH_PROFILE));
    assert_eq!(
        (code, &data["code"], &data["stage"]),
        (Some(1), &json!("custody_read_only"), &json!("custody")),
        "{data}"
    );
    assert!(
        container.seen("validations").is_empty(),
        "no provider exchange"
    );
    let (code, data) = refusal(&container.run(&["approvals", "key-init", "--adapter", ALIAS]));
    assert_eq!(
        (code, &data["code"], &data["stage"]),
        (Some(1), &json!("custody_read_only"), &json!("custody")),
        "{data}"
    );
    let status = success(&container.run(&["approvals", "key-status", "--adapter", ALIAS]));
    assert!(
        status.get("issuer").is_none(),
        "no key was staged: {status}"
    );
}

/// story:catalog-github-app-installation-token: a GitHub App entry (the App
/// id, the installation id and the App's private key) rotates nothing in
/// custody, since its installation token lives in the adapter's memory only.
/// So the read-only mounted item is enough: connecting validates exactly the
/// mounted bytes and writes neither the mount nor a copy into state, where a
/// refresh-rotating profile is refused before any validation.
#[test]
fn mounted_secret_custody_admits_a_github_app_profile() {
    let container = container("connectors-local/4", &Container::mounted());
    let entry = br#"{"app_id":"424242","installation_id":4200017,"private_key":"fixture-app-key-material"}"#;
    container.deliver(GITHUB_APP_PROFILE, entry);
    let before = container.snapshot(SOURCE);
    let connected = success(&container.connect(GITHUB_APP_PROFILE));
    assert_eq!(
        connected["connection"]["summary"]["state"], "ready",
        "{connected}"
    );
    assert_eq!(container.seen("validations"), [digest(entry)]);
    assert_eq!(container.snapshot(SOURCE), before, "the mount is unchanged");
    assert!(
        !container.state_holds(entry),
        "no copy of the entry in state"
    );
}

/// The configuration stage names the rule that refused, never a path:
/// under an empty environment no path resolves; with an empty `HOME` no file
/// is there; a group-readable file is not private; and another `owner_uid`.
#[test]
fn configuration_refusal_names_rule() {
    let cli = env!("CARGO_BIN_EXE_connectors");
    let rule = |output: std::process::Output| {
        let (code, data) = refusal(&output);
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(
            (&data["code"], &data["stage"], &data["next_action"]),
            (
                &json!("invalid_configuration"),
                &json!("configuration"),
                &json!("check_configuration")
            ),
            "{data}"
        );
        data["configuration_rule"].as_str().unwrap_or("").to_owned()
    };
    for args in [
        &["--output", "json", "setup", "check"][..],
        &[
            "--output",
            "json",
            "connections",
            "list",
            "--adapter",
            "tavily",
        ][..],
    ] {
        let unresolved = Command::new(cli).args(args).env_clear().output().unwrap();
        assert_eq!(rule(unresolved), "config_path_unresolved", "{args:?}");
        let home = tempfile::tempdir().unwrap();
        let missing = Command::new(cli)
            .args(args)
            .env_clear()
            .env("HOME", home.path())
            .output()
            .unwrap();
        assert_eq!(rule(missing), "config_missing", "{args:?}");
    }
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    let state = root.path().join("state");
    let run = || {
        Command::new(cli)
            .args(["--output", "json", "--config"])
            .arg(&config)
            .arg("--state-dir")
            .arg(&state)
            .args(["setup", "check"])
            .env_clear()
            .output()
            .unwrap()
    };
    let uid = connectors_host::local::filesystem::uid();
    fs::write(
        &config,
        format!("format='connectors-local/2'\nowner_uid={uid}\n"),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(rule(run()), "config_not_private");
    fs::write(
        &config,
        format!("format='connectors-local/2'\nowner_uid={}\n", uid + 1),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(rule(run()), "owner_uid_mismatch");
    let text = String::from_utf8(run().stderr).unwrap();
    assert!(
        !text.contains(root.path().to_str().unwrap()),
        "no path: {text}"
    );
}

/// Security review, story:mounted-secret-custody: one mounted item backs one
/// connection, also when two `connections connect` for the same alias and
/// profile run at once. A check that read only published connections,
/// outside the transaction that publishes, would let two connects that both
/// pass it before either publishes each publish a connection over the same
/// item; `begin` refuses a connect beside one in flight, and the publishing
/// commit refuses a second one.
#[test]
fn mounted_concurrent_connects_publish_one_connection() {
    let container = container("connectors-local/4", &Container::mounted());
    // Start the owner first, so both connects race on the profile check and
    // not on owner startup.
    success(&container.run(&["connections", "list", "--adapter", ALIAS]));
    let outputs = std::thread::scope(|scope| {
        let first = scope.spawn(|| container.connect(PROFILE));
        let second = scope.spawn(|| container.connect(PROFILE));
        [first.join().unwrap(), second.join().unwrap()]
    });
    let succeeded = outputs.iter().filter(|o| o.status.success()).count();
    let listed = success(&container.run(&["connections", "list", "--adapter", ALIAS]));
    let connections = listed["connections"].as_array().unwrap().len();
    assert!(
        succeeded <= 1 && connections <= 1,
        "one item backs one connection: {succeeded} connects succeeded, \
         {connections} connections listed: {listed}"
    );
}

/// Copy the directory tree `from` to `to`, modes included. A socket (the
/// stopped owner's `owner.sock`) holds nothing and is left out.
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    use std::os::unix::fs::FileTypeExt;
    fs::create_dir(to).unwrap();
    fs::set_permissions(to, fs::metadata(from).unwrap().permissions()).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).unwrap();
        } else {
            assert!(kind.is_socket(), "{:?}", entry.path());
        }
    }
}

/// The envelope without observation times.
fn steady(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            for (key, field) in fields.iter_mut() {
                if key.ends_with("_ms") && field.is_i64() {
                    *field = json!(0);
                } else {
                    steady(field);
                }
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(steady),
        _ => {}
    }
}

/// story:connections-reconnect-verb: `connections reconnect` publishes the
/// item's new bytes on the existing connection, keeping its identity, and its
/// deprecated alias `connections repair` answers the same from the same state,
/// differing only by its one `deprecations` entry.
#[test]
fn mounted_reconnect_publishes_the_new_credential_as_repair_does() {
    let container = container("connectors-local/4", &Container::mounted());
    let connected = success(&container.connect(PROFILE));
    let summary = &connected["connection"]["summary"];
    let connection = summary["connection"].as_str().unwrap().to_owned();
    let revision = summary["revision"].as_str().unwrap().to_owned();
    let read = success(&container.invoke(&connection));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
    let rotated: &[u8] = b"fictional-mounted-token-0003";
    container.rotate("..2026_10_15_00_00_00.000000003", rotated);
    success(&container.run(&["setup", "owner-stop"]));
    let state = container.path(STATE);
    let saved = container.path("/srv/connectors/saved-state");
    copy_tree(&state, &saved);
    let mut answers = Vec::new();
    for verb in ["reconnect", "repair"] {
        fs::remove_dir_all(&state).unwrap();
        copy_tree(&saved, &state);
        let output = container.run(&[
            "connections",
            verb,
            "--connector",
            CONNECTOR,
            "--instance",
            ALIAS,
            "--connection",
            &connection,
            "--expected-revision",
            &revision,
        ]);
        assert!(output.stderr.is_empty(), "{verb}: {output:?}");
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{verb}: {output:?}"));
        assert_eq!(envelope["ok"], true, "{verb}: {envelope}");
        let result = &envelope["result"]["connection"]["summary"];
        assert_eq!(
            result["connection"],
            connection.as_str(),
            "{verb}: {envelope}"
        );
        assert_eq!(result["state"], "ready", "{verb}: {envelope}");
        assert_eq!(
            container.seen("validations").last(),
            Some(&digest(rotated)),
            "{verb}"
        );
        let read = success(&container.invoke(&connection));
        assert_eq!(
            read["result"]["credential"],
            digest(rotated),
            "{verb}: {read}"
        );
        success(&container.run(&["setup", "owner-stop"]));
        steady(&mut envelope);
        answers.push(envelope);
    }
    let deprecations = answers[1]
        .as_object_mut()
        .unwrap()
        .remove("deprecations")
        .expect("repair is deprecated");
    assert_eq!(
        deprecations,
        json!([{"command":"connections repair","replacement":["connections reconnect"]}])
    );
    assert_eq!(answers[0], answers[1]);
}
