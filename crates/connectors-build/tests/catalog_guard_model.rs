//! Every guard a shipped selection declares is a value of the catalog adapter's own model in
//! `adapters/catalog/spec/ess` (`connectors_catalog.guard.Guard`, compiled to JSON Schema by the
//! pinned `ess`), and the engine's reader carries every field of it: read and written back, it is
//! the same document, so a guard keeps its bytes and its configuration revision. A guard with a
//! member the model does not name fails here, and so does a model member the engine drops.
//!
//! The same holds for the members of a selection that close, type, require and fix a write's body
//! (`connectors_catalog.selection.ClosedBody`): every shipped selection's projection onto them is
//! a value of the model, and the whole selection reads and writes back unchanged.
use connectors_catalog_provider::{Guard, Selection};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every operation in every shipped selection file, by file and id.
fn shipped() -> Vec<(String, Value)> {
    let providers = root().join("adapters/catalog/providers");
    let mut files: Vec<PathBuf> = std::fs::read_dir(providers)
        .unwrap()
        .flat_map(|entry| std::fs::read_dir(entry.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let raw: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        for operation in raw["operations"].as_array().into_iter().flatten() {
            found.push((
                format!("{}: {}", file.display(), operation["id"]),
                operation.clone(),
            ));
        }
    }
    found
}

/// Every operation with a `guard` in every shipped selection file, by file and id.
fn guarded() -> Vec<(String, Value)> {
    shipped()
        .into_iter()
        .filter_map(|(name, operation)| Some((name, operation.get("guard")?.clone())))
        .collect()
}

/// The JSON Schema the pinned `ess` compiles for one type of the catalog model.
fn compiled(name: &str) -> Value {
    let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
    let out = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(ess)
        .current_dir(root())
        .args([
            "generate",
            "--path",
            "adapters/catalog/spec/ess",
            "--kind",
            "schema",
            "--out",
        ])
        .arg(out.path().join("schemas"))
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    serde_json::from_slice(
        &std::fs::read(
            out.path()
                .join(format!("schemas/schema/types/{name}.schema.json")),
        )
        .unwrap(),
    )
    .unwrap()
}

fn assert_valid(validator: &jsonschema::Validator, name: &str, raw: &Value) {
    let errors: Vec<String> = validator
        .iter_errors(raw)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{name}: {errors:?}");
}

#[test]
fn every_shipped_guard_is_a_value_of_the_catalog_guard_model() {
    let schema = compiled("connectors_catalog.guard.Guard");
    // JSON Schema carries the model's invariants without enforcing them; the engine refuses
    // them when the selection loads (`adapters/catalog/tests/jira_transition_run.rs`,
    // `adapters/catalog/tests/gitlab_mr_variants.rs`).
    let invariants =
        schema["$defs"]["connectors_catalog.guard.Postflight"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("defined(read)"),
        "the model no longer requires a check of a postflight read: {invariants}"
    );
    assert!(
        invariants.contains("any_of.count"),
        "the model no longer requires two comparisons of an any_of: {invariants}"
    );
    assert!(
        invariants.contains("absent == true")
            && invariants.contains("not (defined(absent)) or defined(read)"),
        "the model no longer holds absent to true beside a read: {invariants}"
    );
    assert!(
        invariants.contains("defined(refusals)"),
        "the model no longer holds refusals to the write's own answer beside a check: {invariants}"
    );
    // A boolean is compared with `boolean`, never a literal of its text; the engine refuses
    // those two literals when the selection loads (`adapters/catalog/tests/engine.rs`).
    let expectation =
        schema["$defs"]["connectors_catalog.guard.Expectation"]["x-ess-invariants"].to_string();
    assert!(
        expectation.contains("literal != true") && expectation.contains("literal != false"),
        "the model no longer refuses a literal boolean spelling: {expectation}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let guarded = guarded();
    for member in [
        "further_preflights",
        "postflight/read",
        "postflight/any_of",
        "postflight/absent",
        "postflight/refusals",
    ] {
        assert!(
            guarded
                .iter()
                .any(|(_, guard)| guard.pointer(&format!("/{member}")).is_some()),
            "no shipped guard declares `{member}`, so this check would not cover it"
        );
    }
    for (name, raw) in &guarded {
        assert_valid(&validator, name, raw);
        let guard: Guard =
            serde_json::from_value(raw.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(&serde_json::to_value(&guard).unwrap(), raw, "{name}");
    }
}

#[test]
fn every_shipped_closed_body_is_a_value_of_the_catalog_selection_model() {
    const MEMBERS: [&str; 5] = [
        "body_keys",
        "body_types",
        "body_required",
        "body_nonempty",
        "body_fixed",
    ];
    let schema = compiled("connectors_catalog.selection.ClosedBody");
    let invariants =
        schema["$defs"]["connectors_catalog.selection.ClosedBody"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("defined(body_fixed)"),
        "the model no longer requires a fixed key's body to be closed: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let shipped = shipped();
    for member in MEMBERS {
        assert!(
            shipped.iter().any(|(_, s)| s.get(member).is_some()),
            "no shipped selection declares `{member}`, so this check would not cover it"
        );
    }
    for (name, raw) in &shipped {
        let projection: serde_json::Map<String, Value> = MEMBERS
            .iter()
            .filter_map(|member| Some((member.to_string(), raw.get(*member)?.clone())))
            .collect();
        assert_valid(&validator, name, &Value::Object(projection));
        // The whole selection reads and writes back member for member: nothing the file
        // carries is dropped, and an omitted member is not written back. `description`,
        // `guard` and `response` are written as `null` when absent.
        let selection: Selection =
            serde_json::from_value(raw.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        let mut written = serde_json::to_value(&selection).unwrap();
        written
            .as_object_mut()
            .unwrap()
            .retain(|key, value| !value.is_null() || raw.get(key).is_some());
        assert_eq!(&written, raw, "{name}");
    }
}

/// Adversary: the engine's reader and the compiled model agree on the shape of a guard. For each
/// variation of the shipped Jira guard, the reader admits it exactly when the compiled schema
/// does. A postflight `read`, or `absent`, written as `null` is not a value of the model (`read` is an
/// `Optional<Read>`, compiled as an object or absent), and the reader admits it as absent.
#[test]
fn the_guard_reader_admits_exactly_the_shapes_the_compiled_model_admits() {
    let schema = compiled("connectors_catalog.guard.Guard");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let (_, shipped) = guarded()
        .into_iter()
        .find(|(_, guard)| guard["postflight"].get("read").is_some())
        .expect("a shipped guard with a postflight read");
    type Change<'a> = &'a dyn Fn(&mut Value);
    let variations: [(&str, Change); 6] = [
        ("as shipped", &|_| {}),
        ("postflight read null", &|g| {
            g["postflight"]["read"] = Value::Null
        }),
        ("further_preflights null", &|g| {
            g["further_preflights"] = Value::Null
        }),
        ("any_of null", &|g| g["postflight"]["any_of"] = Value::Null),
        ("absent null", &|g| g["postflight"]["absent"] = Value::Null),
        ("refusals null", &|g| {
            g["postflight"]["refusals"] = Value::Null
        }),
    ];
    for (name, change) in variations {
        let mut raw = shipped.clone();
        change(&mut raw);
        let model = validator.is_valid(&raw);
        let reader = serde_json::from_value::<Guard>(raw.clone()).is_ok();
        assert_eq!(
            reader, model,
            "{name}: reader admits {reader}, model admits {model}"
        );
    }
}

/// Every shipped selection's `bounds` is a value of `connectors_catalog.selection.QueryBounds`:
/// a range (`maximum`, optional `minimum`) or a `values` set, never both, and both forms are
/// shipped, so the check covers each (story:catalog-path-correction-and-value-bound).
#[test]
fn every_shipped_bound_is_a_value_of_the_catalog_selection_model() {
    let schema = compiled("connectors_catalog.selection.QueryBounds");
    let invariants =
        schema["$defs"]["connectors_catalog.selection.Bound"]["x-ess-invariants"].to_string();
    for clause in ["defined(maximum)", "defined(values)", "values.count"] {
        assert!(
            invariants.contains(clause),
            "the model no longer states `{clause}` of a bound: {invariants}"
        );
    }
    let validator = jsonschema::validator_for(&schema).unwrap();
    let bounded: Vec<(String, Value)> = shipped()
        .into_iter()
        .filter_map(|(name, s)| Some((name, s.get("bounds")?.clone())))
        .collect();
    for form in ["maximum", "values"] {
        assert!(
            bounded.iter().any(|(_, bounds)| bounds
                .as_object()
                .unwrap()
                .values()
                .any(|bound| bound.get(form).is_some())),
            "no shipped bound declares `{form}`, so this check would not cover it"
        );
    }
    for (name, bounds) in &bounded {
        assert_valid(&validator, name, &serde_json::json!({"bounds": bounds}));
        for (parameter, bound) in bounds.as_object().unwrap() {
            assert!(
                bound.get("maximum").is_some() != bound.get("values").is_some(),
                "{name}: `{parameter}` is not exactly one form of bound"
            );
        }
    }
}

/// Every shipped read's `envelope` is a value of `connectors_catalog.selection.ReadEnvelope`, and
/// the engine's reader admits exactly the envelope shapes the compiled model admits: an unknown
/// code or member, or a refusal of no value, is neither (story:slack-ok-false-is-a-refusal).
#[test]
fn every_shipped_envelope_is_a_value_of_the_catalog_selection_model() {
    let schema = compiled("connectors_catalog.selection.ReadEnvelope");
    let invariants =
        schema["$defs"]["connectors_catalog.selection.Envelope"]["x-ess-invariants"].to_string();
    for clause in ["ok.count", "error.count", "refusals.count"] {
        assert!(
            invariants.contains(clause),
            "the model no longer states `{clause}` of an envelope: {invariants}"
        );
    }
    let validator = jsonschema::validator_for(&schema).unwrap();
    let enveloped: Vec<(String, Value)> = shipped()
        .into_iter()
        .filter_map(|(name, s)| Some((name, s.get("envelope")?.clone())))
        .collect();
    assert!(
        !enveloped.is_empty(),
        "no shipped selection declares `envelope`, so this check would not cover it"
    );
    for (name, envelope) in &enveloped {
        assert_valid(
            &validator,
            name,
            &serde_json::json!({ "envelope": envelope }),
        );
    }
    let (_, shipped) = &enveloped[0];
    type Change<'a> = &'a dyn Fn(&mut Value);
    let variations: [(&str, Change); 5] = [
        ("as shipped", &|_| {}),
        ("unknown otherwise", &|e| e["otherwise"] = "capacity".into()),
        ("unknown member", &|e| e["extra"] = true.into()),
        ("unknown refusal code", &|e| {
            e["refusals"][0]["code"] = "capacity".into()
        }),
        ("no otherwise", &|e| {
            e.as_object_mut().unwrap().remove("otherwise");
        }),
    ];
    for (name, change) in variations {
        let mut envelope = shipped.clone();
        change(&mut envelope);
        let model = validator.is_valid(&serde_json::json!({ "envelope": envelope }));
        let reader = serde_json::from_value::<Selection>(serde_json::json!({
            "id": "users.list", "operation_id": "users_list", "effect": "read",
            "envelope": envelope
        }))
        .is_ok();
        assert_eq!(
            reader, model,
            "{name}: reader admits {reader}, model admits {model}"
        );
    }
}

/// Every shipped write's `refusal_texts` is a value of
/// `connectors_catalog.selection.WriteRefusalTexts`, and the engine's reader admits exactly the
/// shapes the compiled model admits: an unknown code or member, or a refusal without a code, is
/// neither. The model states a refusal's non-empty `pointers` and `contains` as invariants,
/// which the engine enforces when the selection loads (story:runpod-create-capacity-refusal).
#[test]
fn every_shipped_refusal_text_is_a_value_of_the_catalog_selection_model() {
    let schema = compiled("connectors_catalog.selection.WriteRefusalTexts");
    let invariants =
        schema["$defs"]["connectors_catalog.selection.TextRefusal"]["x-ess-invariants"].to_string();
    for clause in ["pointers.count", "contains.count"] {
        assert!(
            invariants.contains(clause),
            "the model no longer states `{clause}` of a refusal text: {invariants}"
        );
    }
    let validator = jsonschema::validator_for(&schema).unwrap();
    let declared: Vec<(String, Value)> = shipped()
        .into_iter()
        .filter_map(|(name, s)| Some((name, s.get("refusal_texts")?.clone())))
        .collect();
    assert!(
        !declared.is_empty(),
        "no shipped selection declares `refusal_texts`, so this check would not cover it"
    );
    for (name, texts) in &declared {
        assert_valid(
            &validator,
            name,
            &serde_json::json!({ "refusal_texts": texts }),
        );
    }
    let (_, shipped) = &declared[0];
    type Change<'a> = &'a dyn Fn(&mut Value);
    let variations: [(&str, Change); 4] = [
        ("as shipped", &|_| {}),
        ("unknown code", &|t| t[0]["code"] = "invalid_input".into()),
        ("unknown member", &|t| t[0]["extra"] = true.into()),
        ("no code", &|t| {
            t[0].as_object_mut().unwrap().remove("code");
        }),
    ];
    for (name, change) in variations {
        let mut texts = shipped.clone();
        change(&mut texts);
        let model = validator.is_valid(&serde_json::json!({ "refusal_texts": texts }));
        let reader = serde_json::from_value::<Selection>(serde_json::json!({
            "id": "thing.create", "operation_id": "createThing", "effect": "write",
            "refusal_texts": texts
        }))
        .is_ok();
        assert_eq!(
            reader, model,
            "{name}: reader admits {reader}, model admits {model}"
        );
    }
}

/// A named change to a shipped value, run against the model and the reader alike.
type Variation<'a> = (&'a str, &'a dyn Fn(&mut Value));

/// One selection member checked against the type of the catalog selection model that declares
/// it: every shipped value is a value of the model, at least one shipped selection declares it,
/// the model still states the named invariant clauses, and the engine's reader admits exactly
/// what the compiled model admits for each variation of the first shipped value
/// (story:catalog-github-reads).
fn member_matches_model(
    member: &str,
    holder: &str,
    inner: &str,
    clauses: &[&str],
    selection: Value,
    variations: &[Variation],
) {
    let schema = compiled(holder);
    let invariants = schema["$defs"][inner]["x-ess-invariants"].to_string();
    for clause in clauses {
        assert!(
            invariants.contains(clause),
            "the model no longer states `{clause}` of `{member}`: {invariants}"
        );
    }
    let validator = jsonschema::validator_for(&schema).unwrap();
    let declared: Vec<(String, Value)> = shipped()
        .into_iter()
        .filter_map(|(name, s)| Some((name, s.get(member)?.clone())))
        .collect();
    assert!(
        !declared.is_empty(),
        "no shipped selection declares `{member}`, so this check would not cover it"
    );
    for (name, value) in &declared {
        assert_valid(&validator, name, &serde_json::json!({ member: value }));
    }
    let (_, first) = &declared[0];
    for (name, change) in variations {
        let mut value = first.clone();
        change(&mut value);
        let model = validator.is_valid(&serde_json::json!({ member: value }));
        let mut written = selection.clone();
        written[member] = value;
        let reader = serde_json::from_value::<Selection>(written).is_ok();
        assert_eq!(
            reader, model,
            "`{member}` {name}: reader admits {reader}, model admits {model}"
        );
    }
}

type Change<'a> = &'a dyn Fn(&mut Value);

#[test]
fn every_shipped_rate_limit_header_pair_is_a_value_of_the_catalog_selection_model() {
    let variations: [(&str, Change); 4] = [
        ("as shipped", &|_| {}),
        ("unknown member", &|h| {
            h["limit"] = "x-ratelimit-limit".into()
        }),
        ("no reset", &|h| {
            h.as_object_mut().unwrap().remove("reset");
        }),
        ("a number for a name", &|h| h["remaining"] = 0.into()),
    ];
    member_matches_model(
        "rate_limit_headers",
        "connectors_catalog.selection.QuotaHeaders",
        "connectors_catalog.selection.RateLimitHeaders",
        &["remaining.count", "reset.count"],
        serde_json::json!({"id": "user.me", "operation_id": "users/get-authenticated", "effect": "read"}),
        &variations,
    );
}

#[test]
fn every_shipped_fixed_query_is_a_value_of_the_catalog_selection_model() {
    let variations: [(&str, Change); 3] = [
        ("as shipped", &|_| {}),
        ("a number for a value", &|q| {
            let key = q.as_object().unwrap().keys().next().unwrap().clone();
            q[key] = 1.into();
        }),
        ("a list", &|q| *q = serde_json::json!(["q"])),
    ];
    member_matches_model(
        "query_fixed",
        "connectors_catalog.selection.FixedQuery",
        "connectors_catalog.selection.FixedQuery",
        &["defined(query_folds)"],
        serde_json::json!({"id": "review_requests.list", "operation_id": "search/issues-and-pull-requests", "effect": "read"}),
        &variations,
    );
    let fold_variations: [(&str, Change); 4] = [
        ("as shipped", &|_| {}),
        ("unknown format", &|f| f["since"]["format"] = "date".into()),
        ("unknown member", &|f| f["since"]["suffix"] = "x".into()),
        ("no prefix", &|f| {
            f["since"].as_object_mut().unwrap().remove("prefix");
        }),
    ];
    member_matches_model(
        "query_folds",
        "connectors_catalog.selection.FixedQuery",
        "connectors_catalog.selection.QueryFold",
        &["into.count", "prefix.count", "contains"],
        serde_json::json!({"id": "review_requests.list", "operation_id": "search/issues-and-pull-requests", "effect": "read",
            "query_fixed": {"q": "is:pr is:open review-requested:@me"}}),
        &fold_variations,
    );
}

#[test]
fn every_shipped_embedded_text_is_a_value_of_the_catalog_selection_model() {
    let variations: [(&str, Change); 4] = [
        ("as shipped", &|_| {}),
        ("unknown member", &|e| e["media_type"] = "text/plain".into()),
        ("no content", &|e| {
            e.as_object_mut().unwrap().remove("content");
        }),
        ("a string for max_bytes", &|e| e["max_bytes"] = "1".into()),
    ];
    member_matches_model(
        "embedded_text",
        "connectors_catalog.selection.ReadEmbeddedText",
        "connectors_catalog.selection.EmbeddedText",
        &[
            "content.count",
            "encoding.count",
            "max_bytes >= 1",
            "max_bytes <= 2097152",
        ],
        serde_json::json!({"id": "readme.raw", "operation_id": "repos/get-readme", "effect": "read"}),
        &variations,
    );
}
