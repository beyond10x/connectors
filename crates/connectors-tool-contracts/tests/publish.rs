//! `publish` over the Tavily descriptor and its declarations, and each refusal it makes.
use connectors_core::Descriptor;
use connectors_tool_contracts::{
    Declarations, RefusalKind, generate, generated_path, publish, to_yaml, tool_contract,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn descriptor() -> Descriptor {
    serde_json::from_slice(
        &std::fs::read(root().join("adapters/tavily/generated/descriptor.json")).unwrap(),
    )
    .unwrap()
}

fn declarations_value() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("adapters/tavily/spec/tool-profiles.json")).unwrap(),
    )
    .unwrap()
}

fn declarations(value: &Value) -> Declarations {
    Declarations::from_json(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn refusals(descriptor: &Descriptor, value: &Value) -> Vec<(RefusalKind, String, String)> {
    publish(descriptor, &declarations(value))
        .expect_err("refused")
        .into_iter()
        .map(|r| (r.kind, r.operation, r.detail))
        .collect()
}

#[test]
fn tavily_contract_validates() {
    let publication = publish(&descriptor(), &declarations(&declarations_value())).unwrap();
    let names: Vec<&str> = publication
        .contract
        .operations
        .iter()
        .map(|o| o.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "connector-tavily.search",
            "connector-tavily.fetch",
            "connector-tavily.crawl"
        ]
    );
    assert_eq!(publication.contract.toolset, "connector-tavily");
    assert_eq!(publication.contract.version, "v1alpha1");
    tool_contract::validate(&publication.contract).unwrap();
    // The committed document parses back to the same contract.
    let yaml = to_yaml(&publication.contract, "test").unwrap();
    assert_eq!(
        tool_contract::Contract::from_yaml(&yaml).unwrap(),
        publication.contract
    );
}

#[test]
fn profile_is_the_declared_profile() {
    let value = declarations_value();
    let publication = publish(&descriptor(), &declarations(&value)).unwrap();
    for (published, operation) in publication
        .operations
        .iter()
        .zip(&publication.contract.operations)
    {
        let declared: tool_contract::Profile =
            serde_json::from_value(value["operations"][&published.operation]["profile"].clone())
                .unwrap();
        assert_eq!(operation.profile, declared, "{}", published.name);
    }
}

#[test]
fn operation_without_profile_is_refused() {
    let mut value = declarations_value();
    value["operations"]["websearch.fetch"]
        .as_object_mut()
        .unwrap()
        .remove("profile");
    let found = refusals(&descriptor(), &value);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, RefusalKind::ProfileMissing);
    assert_eq!(found[0].1, "websearch.fetch");
    assert!(found[0].2.contains("tavily/2026-10"), "{}", found[0].2);

    let mut value = declarations_value();
    value["operations"]
        .as_object_mut()
        .unwrap()
        .remove("websearch.crawl");
    let found = refusals(&descriptor(), &value);
    assert_eq!(
        found,
        [(
            RefusalKind::ProfileMissing,
            "websearch.crawl".to_owned(),
            "admitted, but no declaration names it".to_owned()
        )]
    );
}

#[test]
fn declaration_for_unknown_operation_is_refused() {
    let mut value = declarations_value();
    value["operations"]["websearch.map"] = value["operations"]["websearch.crawl"].clone();
    let found = refusals(&descriptor(), &value);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].0, found[0].1.as_str()),
        (RefusalKind::ProfileMissing, "websearch.map")
    );
}

#[test]
fn risk_under_the_floor_is_refused() {
    let mut value = declarations_value();
    value["operations"]["websearch.search"]["profile"]["risk"] = json!("low");
    let found = refusals(&descriptor(), &value);
    assert!(!found.is_empty());
    assert!(
        found.iter().all(
            |(kind, operation, detail)| *kind == RefusalKind::RiskBelowFloor
                && operation == "connector-tavily.search"
                && detail.starts_with("risk:")
        ),
        "{found:?}"
    );
}

#[test]
fn other_validator_rules_are_profile_invalid() {
    let mut value = declarations_value();
    value["operations"]["websearch.crawl"]["profile"]["capability"] = json!("Crawl");
    let found = refusals(&descriptor(), &value);
    assert!(
        found
            .iter()
            .any(|(kind, op, _)| *kind == RefusalKind::ProfileInvalid
                && op == "connector-tavily.crawl"),
        "{found:?}"
    );
}

#[test]
fn a_profile_that_names_a_secret_is_refused() {
    let mut value = declarations_value();
    value["operations"]["websearch.search"]["profile"]["requires"] =
        json!({"secrets": ["TAVILY_API_KEY"]});
    let found = refusals(&descriptor(), &value);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].0, found[0].1.as_str()),
        (RefusalKind::ProfileInvalid, "websearch.search")
    );
}

#[test]
fn instance_name_outside_the_grammar_is_refused() {
    let mut descriptor = descriptor();
    descriptor.instance = "Tavily.Prod".into();
    let found = refusals(&descriptor, &declarations_value());
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].0, found[0].1.as_str()),
        (RefusalKind::NameInvalid, "document")
    );
    let mut value = declarations_value();
    value["operations"]["websearch.fetch"]["name"] = json!("Fetch");
    let found = refusals(&self::descriptor(), &value);
    assert_eq!(
        (found[0].0, found[0].1.as_str()),
        (RefusalKind::NameInvalid, "websearch.fetch")
    );
}

#[test]
fn number_field_is_refused_with_its_path() {
    for (shape, says) in [
        (json!({"type": "number"}), "type number"),
        (
            json!({"oneOf": [{"type": "string"}, {"type": "integer"}]}),
            "oneOf",
        ),
        (json!({}), "without a type"),
        (json!({"type": ["string", "integer"]}), "several non-null"),
    ] {
        let mut descriptor = descriptor();
        descriptor.operations[0].input_schema["properties"]["weight"] = shape;
        let found = refusals(&descriptor, &declarations_value());
        assert_eq!(found.len(), 1, "{found:?}");
        let (kind, operation, detail) = &found[0];
        assert_eq!(*kind, RefusalKind::SchemaUnrepresentable);
        assert_eq!(operation, "websearch.search");
        assert!(
            detail.starts_with("input_schema/properties/weight: ") && detail.contains(says),
            "{detail}"
        );
    }
}

#[test]
fn nullable_and_map_shapes_are_typed() {
    let mut descriptor = descriptor();
    let properties = &mut descriptor.operations[0].input_schema["properties"];
    properties["labels"] = json!({"type": "object", "additionalProperties": {"type": "string"}});
    properties["note"] = json!({"anyOf": [{"type": "string"}, {"type": "null"}]});
    properties["mode"] = json!({"enum": ["a", "b", null]});
    let publication = publish(&descriptor, &declarations(&declarations_value())).unwrap();
    let input = &publication.contract.operations[0].input;
    let ty = |name: &str| {
        input
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.ty.as_str())
            .unwrap()
    };
    assert_eq!(ty("labels"), "Optional<Map<String, String>>");
    assert_eq!(ty("note"), "Optional<String>");
    assert_eq!(ty("mode"), "Optional<search.Mode>");
    assert_eq!(ty("query"), "String");
}

#[test]
fn omitted_field_leaves_the_input() {
    let mut value = declarations_value();
    value["operations"]["websearch.search"]["omit"] = json!(["country"]);
    let publication = publish(&descriptor(), &declarations(&value)).unwrap();
    let search = &publication.contract.operations[0];
    assert!(search.input.iter().all(|f| f.name != "country"));
    assert!(search.names_error("connector.FieldNotAllowed"));
    assert!(
        publication
            .contract
            .error_decl("connector.FieldNotAllowed")
            .is_some()
    );
    assert!(!publication.contract.operations[1].names_error("connector.FieldNotAllowed"));
    assert_eq!(publication.operations[0].omit, ["country"]);
}

#[test]
fn omit_of_unknown_field_is_refused() {
    let mut value = declarations_value();
    value["operations"]["websearch.search"]["omit"] = json!(["region"]);
    let found = refusals(&descriptor(), &value);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, RefusalKind::SchemaUnrepresentable);
    assert!(found[0].2.contains("/properties/region"), "{}", found[0].2);
}

#[test]
fn contract_names_no_secret() {
    let yaml = generate(&root(), "tavily").unwrap();
    let lower = yaml.to_ascii_lowercase();
    for word in [
        "api_key",
        "apikey",
        "authorization",
        "bearer",
        "token",
        "tvly-",
        "api_base",
    ] {
        assert!(!lower.contains(word), "the contract names {word}");
    }
    let publication = publish(&descriptor(), &declarations(&declarations_value())).unwrap();
    assert!(
        publication
            .contract
            .operations
            .iter()
            .all(|o| o.profile.requires.secrets.is_empty() && o.profile.requires.env.is_empty())
    );
}

#[test]
fn generated_contract_has_no_drift() {
    let committed = std::fs::read_to_string(root().join(generated_path("tavily"))).unwrap();
    assert_eq!(
        generate(&root(), "tavily").unwrap(),
        committed,
        "run `cargo run -p connectors-tool-contracts -- generate`"
    );
    // A changed declaration changes the document the check compares against.
    let copy = tempfile::tempdir().unwrap();
    for path in [
        "adapters/tavily/generated/descriptor.json",
        "adapters/tavily/spec/tool-profiles.json",
    ] {
        let target = copy.path().join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(root().join(path), &target).unwrap();
    }
    let mut value = declarations_value();
    value["operations"]["websearch.crawl"]["summary"] = json!("Crawl.");
    std::fs::write(
        copy.path().join("adapters/tavily/spec/tool-profiles.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    assert_ne!(generate(copy.path(), "tavily").unwrap(), committed);
}

#[test]
fn every_error_code_is_an_enum_variant() {
    let publication = publish(&descriptor(), &declarations(&declarations_value())).unwrap();
    let codes = publication
        .contract
        .type_decl("connector.ErrorCode")
        .unwrap()
        .variants
        .clone();
    assert_eq!(codes, connectors_tool_contracts::error_codes());
    assert!(codes.contains(&"invalid_input".to_owned()));
    assert!(codes.contains(&"unauthorized".to_owned()));
}
