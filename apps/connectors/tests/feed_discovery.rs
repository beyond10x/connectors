//! A consumer finds a connection's `datasource.feed/v1alpha1` operations at run time.
//!
//! `operations list --family` answers only the operations whose descriptor binds the family,
//! each with its contract and native profile. [`consumer`] knows the family id and its two
//! fixed operation ids and nothing about any provider. The fixture adapter is configured and
//! described while the test runs, after the `connectors` binary and the consumer were built, and
//! the consumer finds and describes its binding with no change. No adapter executable exists and
//! none is launched: invoking needs a saved connection in custody, which this suite does not have.
use connectors_host::local::{
    config::{Config, Paths},
    owner, registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    process::{Command, Output},
};

/// A consumer of the feed family. Everything it knows is in this module.
mod consumer {
    use serde_json::Value;

    pub const FAMILY: &str = "datasource.feed/v1alpha1";
    pub const CONTAINERS: &str = "feed.containers";
    pub const ITEMS: &str = "feed.items";

    /// What `operations invoke` selects one operation by.
    #[derive(Debug)]
    pub struct Selection {
        pub operation: String,
        pub schema: String,
        pub revision: String,
    }

    #[derive(Debug)]
    pub struct Binding {
        pub profile: String,
        pub containers: Selection,
        pub items: Selection,
    }

    /// The connection's feed binding, if its adapter has one. `cli` runs `connectors` and
    /// answers the success `result`, or the failure `data`.
    pub fn find(
        cli: &dyn Fn(&[&str]) -> Result<Value, Value>,
        adapter: &str,
    ) -> Result<Option<Binding>, String> {
        let mut listed = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut args = vec![
                "operations",
                "list",
                "--adapter",
                adapter,
                "--family",
                FAMILY,
            ];
            if let Some(cursor) = &cursor {
                args.extend(["--cursor", cursor.as_str()]);
            }
            let page = cli(&args).map_err(|data| format!("list refused: {data}"))?;
            listed.extend(page["operations"].as_array().cloned().unwrap_or_default());
            match page["next_cursor"].as_str() {
                Some(next) => cursor = Some(next.to_owned()),
                None => break,
            }
        }
        if listed.is_empty() {
            return Ok(None);
        }
        let mut ids: Vec<&str> = listed.iter().filter_map(|o| o["id"].as_str()).collect();
        ids.sort_unstable();
        if ids != [CONTAINERS, ITEMS] {
            return Err(format!("not the family's two operations: {ids:?}"));
        }
        let profile = listed[0]["profile"].as_str().unwrap_or_default().to_owned();
        if profile.is_empty()
            || listed
                .iter()
                .any(|o| o["contract"] != FAMILY || o["profile"] != profile.as_str())
        {
            return Err("the family's operations disagree on contract or profile".into());
        }
        let select = |operation: &str| -> Result<Selection, String> {
            let described = cli(&[
                "operations",
                "describe",
                "--adapter",
                adapter,
                "--operation",
                operation,
            ])
            .map_err(|data| format!("describe refused: {data}"))?;
            let description = &described["operation"];
            if description["contract"] != FAMILY || description["profile"] != profile.as_str() {
                return Err(format!("{operation} changed between list and describe"));
            }
            Ok(Selection {
                operation: operation.to_owned(),
                schema: described["schema"].as_str().unwrap_or_default().to_owned(),
                revision: described["revision"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            })
        };
        Ok(Some(Binding {
            containers: select(CONTAINERS)?,
            items: select(ITEMS)?,
            profile,
        }))
    }
}

const PROFILE: &str = "fixture.chat.feed/v1";
const REVISION: &str = "desc-7";

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

fn answer(output: &Output) -> Result<Value, Value> {
    if output.status.success() {
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        Ok(value["result"].clone())
    } else {
        assert!(output.stdout.is_empty(), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        Err(error["error"]["data"].clone())
    }
}

fn success(output: &Output) -> Value {
    answer(output).unwrap_or_else(|data| panic!("refused: {data}"))
}

fn refusal(output: &Output) -> String {
    let data = answer(output).expect_err("a refusal");
    data["code"].as_str().unwrap().to_owned()
}

fn paths(root: &tempfile::TempDir) -> Paths {
    Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    }
}

fn initialized() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = command(&root, &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    root
}

/// One operation of a fixture description: `(id, contract, profile)`.
type Declared = (&'static str, &'static str, &'static str);

/// The fixture chat adapter: the feed binding, a records read and an operation whose id merely
/// looks like the family's.
const CHAT: [Declared; 4] = [
    (
        "message.get",
        "datasource.records/v1alpha1",
        "fixture.chat.records/v1",
    ),
    ("feed.containers", consumer::FAMILY, PROFILE),
    (
        "feed.export",
        "operations/v1alpha1",
        "fixture.chat.export/v1",
    ),
    ("feed.items", consumer::FAMILY, PROFILE),
];

/// An adapter with no feed binding.
const FORGE: [Declared; 2] = [
    ("project.get", "operations/v1alpha1", "resource"),
    ("project.list", "datasource.records/v1alpha1", "resource"),
];

fn bootstrap(instance: &str, adapter: &str, declared: &[Declared]) -> runtime::Bootstrap {
    let operations = declared
        .iter()
        .map(|(id, contract, profile)| connectors_core::Operation {
            id: (*id).into(),
            description: "fixture operation".into(),
            contract: (*contract).into(),
            profile: (*profile).into(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
        })
        .collect();
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: instance.into(),
        adapter: adapter.into(),
        revision: REVISION.into(),
        operations,
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance: instance.into(),
        adapter: adapter.into(),
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
            acquisition: None,
        }],
        requirements: declared
            .iter()
            .map(|(id, _, _)| runtime::Requirement {
                operation: (*id).into(),
                profile: "token".into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            })
            .collect(),
    }
}

/// Configures `alias` with every declared operation granted. With `described`, its description
/// is cached as a launch would leave it.
fn add_adapter(
    root: &tempfile::TempDir,
    alias: &str,
    adapter_id: &str,
    declared: &[Declared],
    described: bool,
) -> runtime::Bootstrap {
    let paths = paths(root);
    let granted: Vec<String> = declared
        .iter()
        .map(|(id, _, _)| format!("'{id}'"))
        .collect();
    let config = fs::read_to_string(&paths.config).unwrap()
        + &format!(
            "\n[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='{adapter_id}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.executable]\npath='/not-installed/connectors-{adapter_id}'\nsha256='{}'\nargs=[]\n[adapters.{alias}.permissions]\nprofiles=['token']\noperations=[{}]\n",
            "a".repeat(64),
            granted.join(",")
        );
    fs::write(&paths.config, config).unwrap();
    let bootstrap = bootstrap(&format!("{alias}-local"), adapter_id, declared);
    if described {
        let adapter = Config::load(&paths.config).unwrap().adapters[alias].clone();
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap)
            .unwrap();
    }
    bootstrap
}

fn listed(page: &Value) -> Vec<(String, String, String)> {
    page["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["id"].as_str().unwrap().to_owned(),
                o["contract"].as_str().unwrap().to_owned(),
                o["profile"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn family(alias: &str, value: &str) -> Vec<String> {
    ["operations", "list", "--adapter", alias, "--family", value]
        .map(str::to_owned)
        .to_vec()
}

fn run(root: &tempfile::TempDir, args: &[String]) -> Output {
    command(root, &args.iter().map(String::as_str).collect::<Vec<_>>())
}

#[test]
fn operations_list_by_family_answers_the_family_operations_with_contract_and_profile() {
    let root = initialized();
    add_adapter(&root, "chat", "fixture-chat", &CHAT, true);
    let page = success(&run(&root, &family("chat", consumer::FAMILY)));
    assert_eq!(
        listed(&page),
        [
            (
                "feed.containers".into(),
                consumer::FAMILY.into(),
                PROFILE.into()
            ),
            ("feed.items".into(), consumer::FAMILY.into(), PROFILE.into()),
        ]
    );
    assert_eq!(page["revision"], REVISION);
    assert_eq!(
        (&page["source"], &page["stale"]),
        (&json!("cached"), &json!(true))
    );
    assert!(page["next_cursor"].is_null(), "{page}");

    // Without the filter the listing is unchanged: every granted operation.
    let all = success(&command(
        &root,
        &["operations", "list", "--adapter", "chat"],
    ));
    assert_eq!(listed(&all).len(), CHAT.len());

    // A family nothing binds is a truthful empty page, not a refusal.
    let none = success(&run(&root, &family("chat", "datasource.logs/v1alpha1")));
    assert_eq!(listed(&none), []);
    // The match is exact: a prefix of the family is another family.
    let prefix = success(&run(&root, &family("chat", "datasource.feed")));
    assert_eq!(listed(&prefix), []);
}

#[test]
fn a_consumer_built_before_the_adapter_finds_and_describes_its_feed_binding() {
    let root = initialized();
    add_adapter(&root, "forge", "fixture-forge", &FORGE, true);
    let cli = |args: &[&str]| answer(&command(&root, args));

    // Before the chat adapter exists, the consumer finds no binding on the other adapter.
    assert!(consumer::find(&cli, "forge").unwrap().is_none());

    // Added now, while the test runs: the consumer is unchanged.
    let bootstrap = add_adapter(&root, "chat", "fixture-chat", &CHAT, true);
    let binding = consumer::find(&cli, "chat")
        .unwrap()
        .expect("the chat adapter's feed binding");
    assert_eq!(binding.profile, PROFILE);
    assert_eq!(binding.containers.operation, "feed.containers");
    assert_eq!(binding.items.operation, "feed.items");

    // Each selection is exactly what `operations invoke` admission compares: the operation is
    // exposed and granted, and the schema identity and revision are current.
    let paths = paths(&root);
    for selection in [&binding.containers, &binding.items] {
        assert_eq!(selection.revision, REVISION);
        assert_eq!(
            selection.schema,
            owner::schema(&bootstrap, &selection.operation).unwrap()
        );
        let call = json!({"operation":selection.operation,"schema":selection.schema,
            "revision":selection.revision});
        owner::operation_snapshot(&paths, "chat", &call)
            .unwrap_or_else(|e| panic!("{}: {:?}", selection.operation, e.code));
    }
}

#[test]
fn a_family_cursor_is_bound_to_the_family() {
    let root = initialized();
    add_adapter(&root, "chat", "fixture-chat", &CHAT, true);
    let mut first = family("chat", consumer::FAMILY);
    first.extend(["--limit".into(), "1".into()]);
    let page = success(&run(&root, &first));
    assert_eq!(listed(&page)[0].0, "feed.containers");
    let cursor = page["next_cursor"]
        .as_str()
        .expect("a next_cursor")
        .to_owned();

    // The same cursor without the filter, or under another family, selects another list.
    let unfiltered = command(
        &root,
        &[
            "operations",
            "list",
            "--adapter",
            "chat",
            "--limit",
            "1",
            "--cursor",
            &cursor,
        ],
    );
    assert_eq!(refusal(&unfiltered), "stale_cursor");
    let mut other = family("chat", "datasource.records/v1alpha1");
    other.extend([
        "--limit".into(),
        "1".into(),
        "--cursor".into(),
        cursor.clone(),
    ]);
    assert_eq!(refusal(&run(&root, &other)), "stale_cursor");

    let mut rest = first.clone();
    rest.extend(["--cursor".into(), cursor]);
    let rest = success(&run(&root, &rest));
    assert_eq!(listed(&rest)[0].0, "feed.items");
    assert!(rest["next_cursor"].is_null(), "{rest}");
}

#[test]
fn an_invalid_family_refuses_before_the_cached_description_is_read() {
    let root = initialized();
    // Configured but never described: reading the description would answer
    // `description_unavailable`.
    add_adapter(&root, "chat", "fixture-chat", &CHAT, false);
    assert_eq!(
        refusal(&run(&root, &family("chat", consumer::FAMILY))),
        "description_unavailable"
    );
    let long = "f".repeat(129);
    for value in [
        "",
        "datasource feed/v1alpha1",
        "datasource.feed/v1alpha1\n",
        &long,
    ] {
        assert_eq!(
            refusal(&run(&root, &family("chat", value))),
            "invalid_input",
            "{value:?}"
        );
    }
    let bounded = "f".repeat(128);
    assert_eq!(
        refusal(&run(&root, &family("chat", &bounded))),
        "description_unavailable"
    );
}
