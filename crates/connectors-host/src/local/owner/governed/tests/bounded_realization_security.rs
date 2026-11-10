//! Conformance of the owner's bounded-read port to the MCP projection's
//! realization rule, on both owner-side admissions a `BoundedRead` frame
//! reaches: the audited coordinator (`bounded::read`) and the serialized worker
//! that admits the read again before any provider process starts. An
//! `implemented` realization is served only for the `resource` profile over
//! self-contained input and output schemas, a `generic` one only for the
//! generic HTTP profiles, likewise over self-contained schemas; every other
//! declaration is `unsupported`, recorded as an early refusal that claims no
//! attempt, and never dispatched.
#![cfg(unix)]
use super::*;
use crate::local::owner::supervisor::{Pool, Task};
use connectors_core::operation_metadata::Metadata;
use std::{collections::BTreeMap, sync::Arc};

const PROFILES: [&str; 5] = [
    "resource",
    "generic-http",
    "generic-http-page",
    "rows-by-number",
    "postgresql-native-text",
];

/// A low-risk network read declaring `realization` (`None`: none declared)
/// with the limits the owner selects for it.
fn declaration(realization: Option<&str>) -> Value {
    let generic = realization == Some("generic");
    let mut value = json!({"effects":["network"],"semantic_effects":[],"risk":"low",
        "idempotency":{"kind":"none"},"approval":"not_required",
        "limits":{"execution_ms":if generic {40000} else {20000},
            "provider_ms":if generic {30000} else {15000},"connect_ms":5000,
            "request_bytes":if generic {262144} else {65536},"result_bytes":4194304}});
    if let Some(realization) = realization {
        value["realization"] = json!(realization);
    }
    value
}

fn budget(realization: Option<&str>) -> runtime::ReadBudget {
    if realization == Some("generic") {
        runtime::ReadBudget::start(40000, 30000, 262144, 4194304)
    } else {
        runtime::ReadBudget::start(20000, 15000, 65536, 4194304)
    }
    .unwrap()
}

/// One schema per reference, identifier or anchor keyword the projection
/// refuses. Each compiles locally and admits the fixture's input, so only the
/// realization rule can refuse it.
fn relocated() -> Vec<(&'static str, Value)> {
    vec![
        (
            "$ref",
            json!({"$ref":"#/$defs/value","$defs":{"value":{"type":"object"}}}),
        ),
        (
            "$id",
            json!({"$id":"https://schema.invalid/root","type":"object"}),
        ),
        ("$anchor", json!({"$anchor":"root","type":"object"})),
        (
            "$dynamicRef",
            json!({"$dynamicAnchor":"node","type":"object",
                "properties":{"nested":{"$dynamicRef":"#node"}}}),
        ),
        (
            "$recursiveRef",
            json!({"$schema":"https://json-schema.org/draft/2019-09/schema",
                "$recursiveAnchor":true,"type":"object",
                "properties":{"nested":{"$recursiveRef":"#"}}}),
        ),
    ]
}

/// The cached descriptor of `item.read` with `profile` and the given schemas,
/// under the V3 private protocol a bounded read requires.
fn prepare(target: &mut Target, profile: &str, input: &Value, output: &Value) {
    let mut config = Config::load(&target.paths.config).unwrap();
    let old = config.adapters["fixture"].clone();
    let mut bootstrap = runtime::state::State::new(&target.paths.state)
        .cached(&old.instance_id, &old.selection())
        .unwrap()
        .unwrap();
    config.adapters.get_mut("fixture").unwrap().private_protocol =
        Some(runtime::PrivateProtocol::V3);
    let mut descriptor = bootstrap.descriptor().unwrap();
    descriptor.operations[0].profile = profile.into();
    descriptor.operations[0].input_schema = input.clone();
    descriptor.operations[0].output_schema = output.clone();
    bootstrap.descriptor = serde_json::to_string(&descriptor).unwrap();
    target.schema = crate::local::owner::schema(&bootstrap, "item.read").unwrap();
    std::fs::write(&target.paths.config, toml::to_string(&config).unwrap()).unwrap();
    runtime::state::State::new(&target.paths.state)
        .remember(&config.adapters["fixture"].selection(), &bootstrap)
        .unwrap_or_else(|error| panic!("the cache refuses in {input} out {output}: {error:?}"));
}

/// The fixture's precondition: every relocated schema compiles as the cache
/// compiles a descriptor's schemas, so the owner, not the cache, decides it.
#[test]
fn relocated_schemas_are_admitted_by_the_descriptor_cache() {
    let refused: Vec<_> = relocated()
        .into_iter()
        .filter(|(_, schema)| {
            connectors_sdk::validate(schema, &Value::Null)
                .is_err_and(|error| error.code != connectors_core::ErrorCode::InvalidInput)
        })
        .map(|(keyword, schema)| format!("{keyword}: {schema}"))
        .collect();
    assert!(refused.is_empty(), "{refused:?}");
}

/// A projection that admits every request and declares `declaration` for
/// `item.read`, parsed against the operation's current profile.
struct Projection(Value);
impl ReadPolicy for Projection {
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

/// The projection's rule as the acceptance states it, for the combinations
/// this module drives.
fn projection_serves(realization: Option<&str>, profile: &str, inline: bool) -> bool {
    match realization {
        Some("implemented") => profile == "resource" && inline,
        Some("generic") => matches!(profile, "generic-http" | "generic-http-page") && inline,
        _ => false,
    }
}

/// Each combination of realization, read profile and input or output schema
/// the projection would not serve is refused by the owner's audited
/// coordinator as `unsupported`, with an early-refusal audit that names no
/// operation and no provider request; each combination it would serve is
/// dispatched once. The implemented combinations vary one condition at a time
/// from the served one, and once both together.
#[test]
fn bounded_owner_serves_exactly_the_realizations_the_projection_serves() {
    let inline = json!({"type":"object"});
    let implemented = Some("implemented");
    let mut cases = vec![(
        implemented,
        "resource",
        "inline",
        inline.clone(),
        inline.clone(),
    )];
    for (keyword, schema) in relocated() {
        cases.push((
            implemented,
            "resource",
            keyword,
            schema.clone(),
            inline.clone(),
        ));
        cases.push((implemented, "resource", keyword, inline.clone(), schema));
    }
    for profile in PROFILES.into_iter().skip(1) {
        cases.push((
            implemented,
            profile,
            "inline",
            inline.clone(),
            inline.clone(),
        ));
    }
    let (keyword, schema) = relocated().swap_remove(0);
    cases.push((
        implemented,
        "rows-by-number",
        keyword,
        schema,
        inline.clone(),
    ));
    for profile in PROFILES {
        cases.push((
            Some("generic"),
            profile,
            "inline",
            inline.clone(),
            inline.clone(),
        ));
    }
    for profile in ["generic-http", "generic-http-page"] {
        for (keyword, schema) in relocated() {
            cases.push((
                Some("generic"),
                profile,
                keyword,
                schema.clone(),
                inline.clone(),
            ));
            cases.push((Some("generic"), profile, keyword, inline.clone(), schema));
        }
    }
    for profile in ["resource", "generic-http"] {
        cases.push((None, profile, "inline", inline.clone(), inline.clone()));
    }
    let mut served = 0;
    let mut refused = 0;
    for (realization, profile, keyword, input, output) in &cases {
        let (realization, profile) = (*realization, *profile);
        let name = format!("{realization:?} {profile} {keyword} in {input} out {output}");
        let mut target = Target::new();
        prepare(&mut target, profile, input, output);
        let policy = Projection(declaration(realization));
        let resolved = bounded::resolve(
            &target.paths,
            "fixture",
            &target.request(),
            Some((&policy, "projection")),
            Some(budget(realization)),
        )
        .err()
        .map(|error| error.code);
        let calls = Cell::new(0);
        let value = bounded::read(
            &target.paths,
            "owner-authority",
            "fixture",
            &target.request(),
            (&policy, "projection"),
            budget(realization),
            |_| {
                calls.set(calls.get() + 1);
                Ok(br#"{"observed":true}"#.to_vec())
            },
        )
        .unwrap();
        if projection_serves(realization, profile, *keyword == "inline") {
            served += 1;
            assert_eq!(resolved, None, "{name}");
            assert_eq!(calls.get(), 1, "{name}: {value}");
            assert_eq!(value["status"], "success", "{name}");
            continue;
        }
        refused += 1;
        assert_eq!(calls.get(), 0, "{name}: dispatched {value}");
        assert_eq!(resolved, Some(Code::Unsupported), "{name}");
        assert_eq!(value["error"]["code"], "unsupported", "{name}");
        assert_eq!(value["audit_status"], "complete", "{name}");
        let record = target.record(&target.audits(), &value);
        assert_eq!(record.anchor.kind, audit::Kind::EarlyRefusal, "{name}");
        assert!(record.anchor.operation_id.is_none(), "{name}");
        assert!(record.anchor.access.is_none(), "{name}");
        assert_eq!(
            record.final_observation.unwrap().outcome,
            audit::Outcome::Refused,
            "{name}"
        );
    }
    // Served: implemented `resource` inline, generic on both generic profiles.
    assert_eq!((served, refused), (3, 40));
}

/// The serialized worker admits a bounded read again before it starts any
/// provider process. Driven directly, past the owner's audited coordinator,
/// an implemented realization outside the `resource` profile, or over a
/// relocated input or output schema, is `unsupported` there; the served
/// combination passes that admission and fails later, at provider startup.
#[test]
fn worker_admission_refuses_an_implemented_realization_the_projection_would_not_serve() {
    let inline = json!({"type":"object"});
    let run = |profile: &str, input: &Value, output: &Value| {
        let mut target = Target::new();
        prepare(&mut target, profile, input, output);
        let adapter = Config::load(&target.paths.config).unwrap().adapters["fixture"].clone();
        let pool = Pool::shared_with_read_policy(
            Arc::new(Paths {
                config: target.paths.config.clone(),
                state: target.paths.state.clone(),
            }),
            "owner-authority".into(),
            Arc::new(Projection(declaration(Some("implemented")))),
        );
        let request = target.request();
        let result = pool.run(
            "fixture",
            &adapter,
            Task::Invoke {
                connection: request.connection.into(),
                operation: request.operation.into(),
                schema: request.schema.into(),
                revision: request.revision.into(),
                document: request.input.as_bytes().to_vec(),
                governed: true,
                projection_revision: Some("projection".into()),
                budget: Some(budget(Some("implemented"))),
            },
            connectors_sdk::now_ms() + 5000,
        );
        pool.shutdown().unwrap();
        assert!(!adapter.executable.path.exists());
        result.err().map(|error| error.code)
    };
    let control = run("resource", &inline, &inline);
    assert!(
        control.is_some_and(|code| code != Code::Unsupported),
        "the served combination must pass worker admission: {control:?}"
    );
    for profile in PROFILES.into_iter().skip(1) {
        assert_eq!(
            run(profile, &inline, &inline),
            Some(Code::Unsupported),
            "{profile}"
        );
    }
    for (keyword, schema) in relocated() {
        assert_eq!(
            run("resource", &schema, &inline),
            Some(Code::Unsupported),
            "input {keyword}"
        );
        assert_eq!(
            run("resource", &inline, &schema),
            Some(Code::Unsupported),
            "output {keyword}"
        );
    }
}
