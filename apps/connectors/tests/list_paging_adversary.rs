//! Adversary cases for list cursors: encoding edges, forged offsets, foreign
//! cursors, page coverage around the limit and `--limit` bounds.
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    process::{Command, Output},
};

const OPERATIONS: [&str; 3] = ["item.read", "item.list", "item.write"];

fn command(root: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.path().join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.path().join("state"))
        .args(args)
        .output()
        .unwrap()
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    value["result"].clone()
}

fn refusal(output: &Output) -> Value {
    assert!(
        !output.status.success(),
        "expected a refusal, got {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    error["error"]["data"].clone()
}

fn adapter_entry(alias: &str, operations: &[&str]) -> String {
    let granted = operations
        .iter()
        .map(|o| format!("'{o}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "\n[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.{alias}.permissions]\nprofiles=['token']\noperations=[{granted}]\n",
        "a".repeat(64),
    )
}

fn initialized(aliases: &[String]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config_path = root.path().join("config/config.toml");
    let mut config = fs::read_to_string(&config_path).unwrap();
    for alias in aliases {
        config += &adapter_entry(alias, &OPERATIONS);
    }
    fs::write(&config_path, config).unwrap();
    root
}

fn bootstrap(instance: &str) -> runtime::Bootstrap {
    let operations = OPERATIONS
        .iter()
        .map(|id| connectors_core::Operation {
            id: (*id).into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        })
        .collect();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: instance.into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations,
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: instance.into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: "token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
        }],
        requirements: OPERATIONS
            .iter()
            .map(|id| runtime::Requirement {
                operation: (*id).into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            })
            .collect(),
    }
}

/// Adapters with a cached description of the same three granted operations.
fn described(aliases: &[&str]) -> tempfile::TempDir {
    let root = initialized(&aliases.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>());
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    let config = Config::load(&paths.config).unwrap();
    for alias in aliases {
        let adapter = config.adapters[*alias].clone();
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap(&format!("{alias}-local")))
            .unwrap();
    }
    root
}

fn names(page: &Value, key: &str, field: &str) -> Vec<String> {
    page[key]
        .as_array()
        .unwrap_or_else(|| panic!("no {key}: {page}"))
        .iter()
        .map(|entry| entry[field].as_str().unwrap().to_owned())
        .collect()
}

fn body(cursor: &str) -> Value {
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(cursor).unwrap()).unwrap()
}

fn rebuilt(body: &Value) -> String {
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(body).unwrap())
}

fn first_adapters_cursor(root: &tempfile::TempDir, limit: &str) -> String {
    let first = success(&command(root, &["adapters", "list", "--limit", limit]));
    first["next_cursor"]
        .as_str()
        .expect("a next_cursor")
        .to_owned()
}

fn adapters_page(root: &tempfile::TempDir, limit: &str, cursor: &str) -> Output {
    command(
        root,
        &["adapters", "list", "--limit", limit, "--cursor", cursor],
    )
}

fn assert_stale(output: &Output, what: &str) {
    let refused = refusal(output);
    assert_eq!(refused["code"], "stale_cursor", "{what}: {refused}");
    assert_eq!(refused["stage"], "observation", "{what}: {refused}");
}

/// Follows every cursor and returns the concatenated pages.
fn walk_adapters(root: &tempfile::TempDir, limit: usize) -> Vec<String> {
    let limit_arg = limit.to_string();
    let mut all = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..200 {
        let mut args = vec!["adapters", "list", "--limit", &limit_arg];
        if let Some(c) = &cursor {
            args.extend(["--cursor", c.as_str()]);
        }
        let page = success(&command(root, &args));
        let got = names(&page, "adapters", "adapter");
        assert!(got.len() <= limit, "page longer than limit {limit}: {page}");
        match page["next_cursor"].as_str() {
            Some(next) => {
                assert_eq!(got.len(), limit, "short page with a cursor: {page}");
                all.extend(got);
                cursor = Some(next.to_owned());
            }
            None => {
                all.extend(got);
                return all;
            }
        }
    }
    panic!("paging did not terminate");
}

#[test]
fn every_entry_appears_exactly_once_around_the_limit() {
    for limit in [1usize, 3] {
        for n in [
            0usize,
            1,
            limit - 1,
            limit,
            limit + 1,
            2 * limit,
            2 * limit + 1,
        ] {
            let aliases = (0..n).map(|i| format!("a{i:02}")).collect::<Vec<_>>();
            let root = initialized(&aliases);
            assert_eq!(
                walk_adapters(&root, limit),
                aliases,
                "limit {limit}, {n} entries"
            );
        }
    }
    // 64 configured entries is the documented maximum; --limit 500 is one page.
    let aliases = (0..64).map(|i| format!("a{i:02}")).collect::<Vec<_>>();
    let root = initialized(&aliases);
    let page = success(&command(&root, &["adapters", "list", "--limit", "500"]));
    assert_eq!(names(&page, "adapters", "adapter"), aliases);
    assert!(page["next_cursor"].is_null(), "{page}");
    assert_eq!(walk_adapters(&root, 1), aliases);
}

#[test]
fn a_limit_outside_one_to_five_hundred_is_invalid_input() {
    let root = initialized(&["alpha".to_owned()]);
    for limit in ["0", "501", "-1"] {
        let out = command(&root, &["adapters", "list", "--limit", limit]);
        let refused = refusal(&out);
        assert_eq!(
            refused["code"], "invalid_input",
            "--limit {limit}: {refused}"
        );
    }
    // Beyond i64 the argument parser refuses; it must never fall back to the
    // default limit and answer a page.
    for limit in ["9223372036854775808", "18446744073709551616"] {
        let out = command(&root, &["adapters", "list", "--limit", limit]);
        assert!(!out.status.success(), "--limit {limit} answered a page");
    }
}

#[test]
fn the_encoding_admits_exactly_one_spelling_of_a_cursor() {
    let aliases = (0..3).map(|i| format!("a{i}")).collect::<Vec<_>>();
    let root = initialized(&aliases);
    let cursor = first_adapters_cursor(&root, "1");
    // The genuine cursor is accepted.
    success(&adapters_page(&root, "1", &cursor));

    let bytes = URL_SAFE_NO_PAD.decode(&cursor).unwrap();
    let mut variants = vec![
        (
            "padded",
            STANDARD.encode(&bytes).replace('+', "-").replace('/', "_"),
        ),
        ("standard alphabet", STANDARD.encode(&bytes)),
        ("trailing newline", format!("{cursor}\n")),
        ("leading space", format!(" {cursor}")),
        (
            "embedded space",
            format!("{} {}", &cursor[..4], &cursor[4..]),
        ),
        ("empty", String::new()),
        ("one char", "A".to_owned()),
        ("huge", "A".repeat(120_000)),
    ];
    // Non-canonical trailing bits: set the lowest bit of the last symbol when
    // the length leaves unused bits.
    if !cursor.len().is_multiple_of(4) {
        let last = cursor.as_bytes()[cursor.len() - 1];
        let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let index = alphabet.iter().position(|c| *c == last).unwrap();
        let flipped = alphabet[index ^ 1] as char;
        variants.push((
            "non-canonical trailing bits",
            format!("{}{flipped}", &cursor[..cursor.len() - 1]),
        ));
    }
    for (what, variant) in variants {
        if variant == cursor {
            continue;
        }
        assert_stale(&adapters_page(&root, "1", &variant), what);
    }
    // Every body length mod 3 decodes: pad the JSON with spaces.
    let parsed = body(&cursor);
    let compact = serde_json::to_vec(&parsed).unwrap();
    for extra in 0..3 {
        let mut padded = compact.clone();
        padded.extend(std::iter::repeat_n(b' ', extra));
        let spelled = URL_SAFE_NO_PAD.encode(&padded);
        success(&adapters_page(&root, "1", &spelled));
    }
}

#[test]
fn a_forged_offset_outside_the_list_is_stale() {
    let aliases = (0..3).map(|i| format!("a{i}")).collect::<Vec<_>>();
    let root = initialized(&aliases);
    let cursor = first_adapters_cursor(&root, "1");
    let original = body(&cursor);
    for offset in [
        json!(0),
        json!(3),
        json!(4),
        json!(-1),
        json!(u64::MAX),
        json!(1.5),
        json!("1"),
        json!(null),
    ] {
        let mut forged = original.clone();
        forged["offset"] = offset.clone();
        assert_stale(
            &adapters_page(&root, "1", &rebuilt(&forged)),
            &format!("offset {offset}"),
        );
    }
    for (key, value) in [("version", json!(2)), ("extra", json!(1))] {
        let mut forged = original.clone();
        forged[key] = value;
        assert_stale(&adapters_page(&root, "1", &rebuilt(&forged)), key);
    }
    let mut future = original.clone();
    future["issued_at"] = json!(original["issued_at"].as_u64().unwrap() + 3600);
    assert_stale(&adapters_page(&root, "1", &rebuilt(&future)), "future");
}

#[test]
fn a_cursor_does_not_cross_between_the_two_lists() {
    let root = described(&["forge", "smith", "tongs"]);
    let adapters_cursor = first_adapters_cursor(&root, "1");
    let ops = success(&command(
        &root,
        &["operations", "list", "--adapter", "forge", "--limit", "1"],
    ));
    let ops_cursor = ops["next_cursor"].as_str().unwrap().to_owned();
    let refused = refusal(&command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "forge",
            "--limit",
            "1",
            "--cursor",
            &adapters_cursor,
        ],
    ));
    assert_eq!(refused["code"], "stale_cursor", "{refused}");
    assert_stale(
        &adapters_page(&root, "1", &ops_cursor),
        "operations cursor to adapters list",
    );
}

#[test]
fn an_operations_cursor_is_bound_to_the_adapter_it_was_issued_for() {
    // Two configured instances of one adapter with the same grants: the
    // contract binds a cursor to its selection (semantics.md:322-323), and
    // the selection names the adapter.
    let root = described(&["forge", "smith"]);
    let first = success(&command(
        &root,
        &["operations", "list", "--adapter", "forge", "--limit", "2"],
    ));
    let cursor = first["next_cursor"]
        .as_str()
        .expect("a next_cursor")
        .to_owned();
    let foreign = command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "smith",
            "--limit",
            "2",
            "--cursor",
            &cursor,
        ],
    );
    let refused = refusal(&foreign);
    assert_eq!(refused["code"], "stale_cursor", "{refused}");
}

#[test]
fn operations_pages_cover_every_permitted_operation_once_and_nothing_else() {
    let root = described(&["forge"]);
    // Grant only two of the three described operations.
    let config_path = root.path().join("config/config.toml");
    let config = fs::read_to_string(&config_path).unwrap().replace(
        "operations=['item.read','item.list','item.write']",
        "operations=['item.read','item.write']",
    );
    fs::write(&config_path, config).unwrap();
    let first = success(&command(
        &root,
        &["operations", "list", "--adapter", "forge", "--limit", "1"],
    ));
    let cursor = first["next_cursor"]
        .as_str()
        .expect("a next_cursor")
        .to_owned();
    let decoded = body(&cursor);
    let keys = decoded
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        ["digest", "issued_at", "offset", "version"],
        "{decoded}"
    );
    assert!(!decoded.to_string().contains("item."), "{decoded}");
    let rest = success(&command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "forge",
            "--limit",
            "1",
            "--cursor",
            &cursor,
        ],
    ));
    assert!(rest["next_cursor"].is_null(), "{rest}");
    let mut all = names(&first, "operations", "id");
    all.extend(names(&rest, "operations", "id"));
    assert_eq!(all, ["item.read", "item.write"]);
    // An offset forged past the permitted list is stale, not a view of the
    // ungranted operation.
    let mut forged = decoded.clone();
    forged["offset"] = json!(2);
    let refused = refusal(&command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "forge",
            "--limit",
            "1",
            "--cursor",
            &rebuilt(&forged),
        ],
    ));
    assert_eq!(refused["code"], "stale_cursor", "{refused}");
}
