use super::*;
use connectors_core::operation_metadata::Metadata;
use std::collections::BTreeMap;

pub(super) fn declaration(generic: bool) -> Value {
    json!({"effects":["network"],"semantic_effects":[],"risk":"low",
        "idempotency":{"kind":"none"},"approval":"not_required",
        "realization":if generic {"generic"} else {"implemented"},
        "limits":{"execution_ms":if generic {40000} else {20000},
            "provider_ms":if generic {30000} else {15000},"connect_ms":5000,
            "request_bytes":if generic {262144} else {65536},"result_bytes":4194304}})
}
pub(super) fn prepare(target: &mut Target, generic: bool) {
    prepare_operation(target, |operation| {
        if generic {
            operation.profile = "generic-http".into();
        }
    });
}
/// `prepare` for an implemented read, with `edit` applied to the selected
/// operation's descriptor entry before the owner caches it.
fn prepare_operation(target: &mut Target, edit: impl FnOnce(&mut connectors_core::Operation)) {
    let mut config = Config::load(&target.paths.config).unwrap();
    let old = config.adapters["fixture"].clone();
    let mut bootstrap = runtime::state::State::new(&target.paths.state)
        .cached(&old.instance_id, &old.selection())
        .unwrap()
        .unwrap();
    config.adapters.get_mut("fixture").unwrap().private_protocol =
        Some(runtime::PrivateProtocol::V3);
    let mut descriptor = bootstrap.descriptor().unwrap();
    descriptor.operations[0].input_schema = json!({"type":"object"});
    edit(&mut descriptor.operations[0]);
    bootstrap.descriptor = serde_json::to_string(&descriptor).unwrap();
    target.schema = crate::local::owner::schema(&bootstrap, "item.read").unwrap();
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    runtime::state::State::new(&target.paths.state)
        .remember(&config.adapters["fixture"].selection(), &bootstrap)
        .unwrap();
}
struct Policy(Value);
impl ReadPolicy for Policy {
    fn admit(&self, _: &Paths, _: &str, _: &Request<'_>, _: &str) -> Result<()> {
        Ok(())
    }
    fn metadata(
        &self,
        _: &Paths,
        _: &str,
        _: &config::Adapter,
        bootstrap: &runtime::Bootstrap,
    ) -> Result<ProjectionMetadata> {
        let descriptor = bootstrap.descriptor()?;
        let profile = &descriptor.operation("item.read").unwrap().profile;
        Ok(ProjectionMetadata {
            revision: "projection".into(),
            operations: BTreeMap::from([(
                "item.read".into(),
                Metadata::parse(&serde_json::to_vec(&self.0).unwrap(), profile, 65536).unwrap(),
            )]),
        })
    }
}
fn budget(generic: bool) -> runtime::ReadBudget {
    if generic {
        runtime::ReadBudget::start(40000, 30000, 262144, 4194304)
    } else {
        runtime::ReadBudget::start(20000, 15000, 65536, 4194304)
    }
    .unwrap()
}
fn resolve(target: &Target, policy: &Policy, budget: runtime::ReadBudget) -> Result<ReadPlan> {
    bounded::resolve(
        &target.paths,
        "fixture",
        &target.request(),
        Some((policy, "projection")),
        Some(budget),
    )
}

#[test]
fn bounded_owner_selects_declared_profiles_and_audits_the_service_response() {
    for generic in [false, true] {
        let mut target = Target::new();
        prepare(&mut target, generic);
        let policy = Policy(declaration(generic));
        resolve(&target, &policy, budget(generic)).unwrap();
        let calls = Cell::new(0);
        let value = bounded::read(
            &target.paths,
            "owner-authority",
            "fixture",
            &target.request(),
            (&policy, "projection"),
            budget(generic),
            |_| {
                calls.set(calls.get() + 1);
                Ok(br#"{"observed":true}"#.to_vec())
            },
        )
        .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(value["status"], "success");
        assert_eq!(value["audit_status"], "complete");
        assert!(value["audit_ref"].as_str().is_some());
    }
}

#[test]
fn bounded_owner_refuses_caller_limits_and_unimplemented_requirements() {
    let mut target = Target::new();
    prepare(&mut target, false);
    let policy = Policy(declaration(false));
    let oversized = runtime::ReadBudget::start(20000, 15000, 262144, 4194304).unwrap();
    assert!(matches!(
        resolve(&target, &policy, oversized),
        Err(Error {
            code: Code::Unsupported,
            ..
        })
    ));
    for field in ["approval", "connect", "realization"] {
        let mut value = declaration(false);
        match field {
            "approval" => value["approval"] = json!("required"),
            "connect" => value["limits"]["connect_ms"] = json!(1000),
            _ => value
                .as_object_mut()
                .unwrap()
                .remove("realization")
                .map(|_| ())
                .unwrap(),
        }
        assert!(
            matches!(
                resolve(&target, &Policy(value), budget(false)),
                Err(Error {
                    code: Code::Unsupported,
                    ..
                })
            ),
            "{field}"
        );
    }
    let mut config = Config::load(&target.paths.config).unwrap();
    let bootstrap = crate::local::owner::cached(&target.paths, "fixture").unwrap();
    config.adapters.get_mut("fixture").unwrap().private_protocol =
        Some(runtime::PrivateProtocol::V2);
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    runtime::state::State::new(&target.paths.state)
        .remember(&config.adapters["fixture"].selection(), &bootstrap)
        .unwrap();
    assert!(matches!(
        resolve(&target, &policy, budget(false)),
        Err(Error {
            code: Code::Unsupported,
            ..
        })
    ));
}

#[test]
fn bounded_owner_refuses_semantic_and_non_network_effects_before_any_request() {
    let mut target = Target::new();
    prepare(&mut target, false);
    let read = |declaration: Value| {
        let calls = Cell::new(0);
        let value = bounded::read(
            &target.paths,
            "owner-authority",
            "fixture",
            &target.request(),
            (&Policy(declaration), "projection"),
            budget(false),
            |_| {
                calls.set(calls.get() + 1);
                Ok(br#"{"observed":true}"#.to_vec())
            },
        )
        .unwrap();
        (value, calls.get())
    };
    let (admitted, requests) = read(declaration(false));
    assert_eq!(admitted["status"], "success");
    assert_eq!(requests, 1);
    for (field, effects) in [
        ("semantic_effects", json!(["human_visible"])),
        ("effects", json!(["network", "process"])),
    ] {
        let mut value = declaration(false);
        value[field] = effects;
        assert!(
            matches!(
                resolve(&target, &Policy(value.clone()), budget(false)),
                Err(Error {
                    code: Code::Unsupported,
                    ..
                })
            ),
            "{field}"
        );
        let (refused, requests) = read(value);
        assert_eq!(refused["error"]["code"], "unsupported", "{field}");
        assert_eq!(refused["audit_status"], "complete", "{field}");
        assert_eq!(requests, 0, "{field}");
    }
}

/// Effects a bounded or projected read refuses.
const REFUSED_EFFECTS: [(&str, &str); 4] = [
    ("semantic_effects", r#"["human_visible"]"#),
    ("semantic_effects", r#"["billable"]"#),
    ("effects", r#"["network","process"]"#),
    ("effects", r#"["local_system"]"#),
];

fn declared(field: &str, effects: &str) -> Value {
    let mut value = declaration(false);
    value[field] = serde_json::from_str(effects).unwrap();
    value
}

/// The protected operation curation (`<config>.operation-curation.json`),
/// pinned to the fixture's selection and cache, declaring `metadata` for
/// `item.read`: the one place an owner reads an operation's declared effects.
fn write_curation(target: &Target, metadata: &Value) {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let adapter = Config::load(&target.paths.config).unwrap().adapters["fixture"].clone();
    let bootstrap = crate::local::owner::cached(&target.paths, "fixture").unwrap();
    let document = json!({"format":"connectors-operation-curation/1","adapters":[{
        "adapter_alias":"fixture","executable_selection":adapter.selection(),
        "bootstrap_sha256":connectors_core::digest(&serde_json::to_value(&bootstrap).unwrap()),
        "operations":[{"operation":"item.read","metadata":metadata}]}]});
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(crate::local::operation_curation::path(&target.paths))
        .unwrap()
        .write_all(document.to_string().as_bytes())
        .unwrap();
}

/// The native projection policy's metadata source: the protected curation.
struct CurationPolicy;
impl ReadPolicy for CurationPolicy {
    fn admit(&self, _: &Paths, _: &str, _: &Request<'_>, _: &str) -> Result<()> {
        Ok(())
    }
    fn metadata(
        &self,
        paths: &Paths,
        alias: &str,
        adapter: &config::Adapter,
        bootstrap: &runtime::Bootstrap,
    ) -> Result<ProjectionMetadata> {
        let snapshot = crate::local::operation_curation::load(paths, alias, adapter, bootstrap)?;
        Ok(ProjectionMetadata {
            revision: "projection".into(),
            operations: snapshot.operations,
        })
    }
}

/// Publish one connection of the fixture's `token` profile and answer its
/// reference, through the registry as the owner's own connect does.
fn connect(target: &Target) -> String {
    let now = connectors_sdk::now_ms();
    let bootstrap = crate::local::owner::cached(&target.paths, "fixture").unwrap();
    let binding = bootstrap.binding("token").unwrap();
    let registry = registry::Registry::new(&target.paths.state);
    let acquisition = registry.begin_unchecked(&binding, now).unwrap();
    let claim = registry.consume(acquisition, now).unwrap();
    let prepared = registry
        .prepare(
            &claim,
            registry::ValidatedBaseline {
                binding: None,
                identity: registry::ExternalIdentity {
                    kind: "fixture-account".into(),
                    subject: "fixture-subject".into(),
                },
                granted_scopes: Some(BTreeSet::new()),
                credential_expires_at_ms: Some(now + 3_600_000),
                collected_at_ms: now,
                valid_until_ms: now + 60_000,
            },
            12,
            now,
        )
        .unwrap();
    let receipt = crate::local::keyring::custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, now).unwrap();
    registry.publish(stored, now).unwrap()
}

/// The effects rule binds the bounded and projected read paths, not the plain
/// `invoke` frame (story:owner-invoke-frame-effects-boundary). The declaration
/// lives in the protected curation, where the bounded read finds it; a granted,
/// connected read whose curation that read refuses is admitted on `invoke` all
/// the same, as it is with no curation at all.
#[test]
fn invoke_admission_is_unchanged_by_a_curation_the_bounded_read_refuses() {
    let mut target = Target::new();
    prepare(&mut target, false);
    let reference = connect(&target);
    let call = json!({"connection":reference,"operation":"item.read",
        "schema":target.schema,"revision":"desc-1"});
    // Admission's last step asks the custody binding whether it is available,
    // which needs a live Secret Service or a read-only mount the fixture has
    // not got. Every step before it (the grant, the revision, the read
    // requirement, the input and the connection) must pass: the answer is `Ok`
    // where a Secret Service runs and `custody_unavailable` where not, and
    // never a refusal of the declared effects.
    let admit = || match crate::local::owner::admit_invoke(
        &target.paths,
        "fixture",
        &call,
        br#"{"n":1}"#,
    ) {
        Err(error) if error.code != Code::CustodyUnavailable => Err(error),
        _ => Ok(()),
    };
    admit().expect("a granted, connected read is admitted without a curation");
    write_curation(&target, &declaration(false));
    admit().expect("a network-only curation is admitted");
    for (field, effects) in REFUSED_EFFECTS {
        write_curation(&target, &declared(field, effects));
        let request = Request {
            connection: &reference,
            ..target.request()
        };
        assert!(
            matches!(
                bounded::resolve(
                    &target.paths,
                    "fixture",
                    &request,
                    Some((&CurationPolicy, "projection")),
                    Some(budget(false)),
                ),
                Err(Error {
                    code: Code::Unsupported,
                    ..
                })
            ),
            "bounded path admitted {field} {effects}"
        );
        admit().unwrap_or_else(|error| panic!("{field} {effects}: invoke refused {error:?}"));
    }
}

/// The refusal `invoke` does hold for what it is not for: a requirement that is
/// not a read is `unsupported`, whatever the grant (writes take the write frames).
#[test]
fn invoke_refuses_a_write_requirement_unsupported() {
    let mut target = Target::new();
    prepare(&mut target, false);
    let reference = connect(&target);
    let call = json!({"connection":reference,"operation":"item.read",
        "schema":target.schema,"revision":"desc-1"});
    let adapter = Config::load(&target.paths.config).unwrap().adapters["fixture"].clone();
    let mut bootstrap = runtime::state::State::new(&target.paths.state)
        .cached(&adapter.instance_id, &adapter.selection())
        .unwrap()
        .unwrap();
    bootstrap.requirements[0].effect = runtime::Effect::Write;
    runtime::state::State::new(&target.paths.state)
        .remember(&adapter.selection(), &bootstrap)
        .unwrap();
    assert_eq!(
        crate::local::owner::admit_invoke(&target.paths, "fixture", &call, br#"{"n":1}"#)
            .unwrap_err()
            .code,
        Code::Unsupported
    );
}

/// A bounded read of `target` under the implemented declaration: the
/// resolution's refusal code, the served value and the provider request count.
fn implemented_read(target: &Target) -> (Option<Code>, Value, usize) {
    let policy = Policy(declaration(false));
    let refusal = resolve(target, &policy, budget(false))
        .err()
        .map(|error| error.code);
    let calls = Cell::new(0);
    let value = bounded::read(
        &target.paths,
        "owner-authority",
        "fixture",
        &target.request(),
        (&policy, "projection"),
        budget(false),
        |_| {
            calls.set(calls.get() + 1);
            Ok(br#"{"observed":true}"#.to_vec())
        },
    )
    .unwrap();
    (refusal, value, calls.get())
}

/// The MCP projection advertises an implemented read only for the `resource`
/// profile. Driven directly, past the projection, the owner holds the same
/// rule: an implemented realization on any other read profile is
/// `unsupported` before any provider request.
#[test]
fn bounded_owner_refuses_an_implemented_realization_outside_the_resource_profile() {
    for profile in [
        "generic-http",
        "generic-http-page",
        "rows-by-number",
        "postgresql-native-text",
    ] {
        let mut target = Target::new();
        prepare_operation(&mut target, |operation| operation.profile = profile.into());
        let (refusal, value, requests) = implemented_read(&target);
        assert_eq!(requests, 0, "{profile}: dispatched {value}");
        assert_eq!(refusal, Some(Code::Unsupported), "{profile}");
        assert_eq!(value["error"]["code"], "unsupported", "{profile}");
        assert_eq!(value["audit_status"], "complete", "{profile}");
    }
}

/// The projection advertises an implemented `resource` read only when its
/// input and output schemas are self-contained. The owner refuses a schema the
/// projection would not advertise, before any provider request.
#[test]
fn bounded_owner_refuses_an_implemented_resource_read_without_inline_schemas() {
    let relocated = [
        json!({"$ref":"#/$defs/value","$defs":{"value":{"type":"object"}}}),
        json!({"$dynamicAnchor":"node","type":"object",
            "properties":{"nested":{"$dynamicRef":"#node"}}}),
        json!({"type":"object","$anchor":"root"}),
    ];
    for (side, schema) in relocated
        .iter()
        .flat_map(|schema| [("input", schema), ("output", schema)])
    {
        let mut target = Target::new();
        prepare_operation(&mut target, |operation| match side {
            "input" => operation.input_schema = schema.clone(),
            _ => operation.output_schema = schema.clone(),
        });
        let (refusal, value, requests) = implemented_read(&target);
        assert_eq!(requests, 0, "{side} {schema}: dispatched {value}");
        assert_eq!(refusal, Some(Code::Unsupported), "{side} {schema}");
        assert_eq!(value["error"]["code"], "unsupported", "{side} {schema}");
    }
    let mut target = Target::new();
    prepare_operation(&mut target, |_| {});
    let (refusal, value, requests) = implemented_read(&target);
    assert_eq!((refusal, requests), (None, 1), "inline control: {value}");
    assert_eq!(value["status"], "success");
}

#[test]
fn bounded_owner_counts_the_whole_request_and_never_dispatches_oversize() {
    let mut target = Target::new();
    prepare(&mut target, false);
    let policy = Policy(declaration(false));
    let input = json!({"text":"x".repeat(65536-80)}).to_string();
    assert!(input.len() < 65536);
    let request = Request {
        input: &input,
        ..target.request()
    };
    let value = bounded::read(
        &target.paths,
        "owner-authority",
        "fixture",
        &request,
        (&policy, "projection"),
        budget(false),
        |_| panic!("oversized service envelope dispatched"),
    )
    .unwrap();
    assert_eq!(value["error"]["code"], "invalid_input");
    assert_eq!(value["audit_status"], "complete");
}

#[test]
fn bounded_owner_counts_the_whole_result_and_returns_capacity_without_truncation() {
    let mut target = Target::new();
    prepare(&mut target, false);
    let policy = Policy(declaration(false));
    let payload = serde_json::to_vec(&json!({"text":"x".repeat(4194304-80)})).unwrap();
    assert!(payload.len() < 4194304);
    let value = bounded::read(
        &target.paths,
        "owner-authority",
        "fixture",
        &target.request(),
        (&policy, "projection"),
        budget(false),
        |_| Ok(payload),
    )
    .unwrap();
    assert_eq!(value["error"]["code"], "capacity");
    assert!(value.get("result").is_none());
    assert_eq!(value["audit_status"], "complete");
}

// Adversary: the GovernedRead frame (transport.rs governed_action, budget None)
// reaches governed::read with the composition's projection selected. The owner
// must hold the read rule there too, or a same-uid caller swaps frame kinds.
#[test]
fn adversary_projected_governed_read_refuses_semantic_and_non_network_effects() {
    let mut target = Target::new();
    prepare(&mut target, false);
    for (field, effects) in [
        ("semantic_effects", json!(["human_visible"])),
        ("effects", json!(["network", "process"])),
    ] {
        let mut value = declaration(false);
        value[field] = effects;
        let policy = Policy(value);
        let calls = Cell::new(0);
        let refused = super::super::read(
            (
                &target.paths,
                "owner-authority",
                Instant::now() + Duration::from_secs(20),
            ),
            "fixture",
            &target.request(),
            Some((&policy, "projection")),
            |_| {
                calls.set(calls.get() + 1);
                Ok(br#"{"observed":true}"#.to_vec())
            },
        )
        .unwrap();
        assert_eq!(
            calls.get(),
            0,
            "{field}: projected GovernedRead dispatched {refused}"
        );
        assert_eq!(refused["error"]["code"], "unsupported", "{field}");
    }
}

// Adversary: every semantic effect and every non-network effect that parses
// for a read profile, not only the two the unit's case names.
#[test]
fn adversary_bounded_owner_refuses_every_parseable_semantic_and_non_network_effect() {
    let mut target = Target::new();
    prepare(&mut target, false);
    for (field, effects) in [
        ("semantic_effects", json!(["billable"])),
        ("semantic_effects", json!(["irreversible"])),
        ("semantic_effects", json!(["human_visible"])),
        ("semantic_effects", json!(["irreversible", "billable"])),
        ("effects", json!(["process"])),
        ("effects", json!(["local_system"])),
        ("effects", json!(["network", "local_system"])),
        ("effects", json!(["local_system", "network"])),
    ] {
        let mut value = declaration(false);
        value[field] = effects.clone();
        assert!(
            matches!(
                resolve(&target, &Policy(value), budget(false)),
                Err(Error {
                    code: Code::Unsupported,
                    ..
                })
            ),
            "{field} {effects}"
        );
    }
}

// Adversary: field shapes the owner's check would read differently from the
// projection never reach either check: both parse the typed metadata first.
#[test]
fn adversary_effect_shapes_outside_the_typed_metadata_never_parse() {
    let mut cases: Vec<(&str, Option<Value>)> = vec![];
    for shape in [
        json!(null),
        json!("x"),
        json!({}),
        json!([""]),
        json!(["HUMAN_VISIBLE"]),
    ] {
        cases.push(("semantic_effects", Some(shape)));
    }
    for shape in [
        json!(null),
        json!("network"),
        json!(["NETWORK"]),
        json!(["network", "network"]),
        json!(["telepathy"]),
        json!([""]),
    ] {
        cases.push(("effects", Some(shape)));
    }
    cases.push(("semantic_effects", None));
    cases.push(("effects", None));
    for (field, shape) in cases {
        let mut value = declaration(false);
        match &shape {
            Some(shape) => value[field] = shape.clone(),
            None => {
                value.as_object_mut().unwrap().remove(field);
            }
        }
        assert!(
            Metadata::parse(&serde_json::to_vec(&value).unwrap(), "resource", 65536).is_err(),
            "{field} {shape:?}"
        );
    }
}
