//! Audience metadata: curated catalogue discovery hints, never policy inputs.

use connector_spec::{Audience, Connector};

fn provider(body: &str) -> String {
    format!(
        r#"
id = "acme"
vendor = "Acme"
base_url = "https://api.acme.example"
description = "A provider that exists to be checked."
{body}
"#
    )
}

fn operation(id: &str, service: &str) -> String {
    format!(
        r#"
[[operations]]
id = "{id}"
service = "{service}"
method = "GET"
direction = "read"
path = "/v1/things"
description = "Fetch things."
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]
"#
    )
}

fn load(source: &str) -> connector_spec::Result<Connector> {
    connector_spec::provider::load("providers/acme.toml", source).map(|loaded| loaded.connector)
}

fn refusal(source: &str) -> String {
    load(source)
        .expect_err("the loader accepted audience metadata it must refuse")
        .to_string()
}

#[test]
fn a_service_may_carry_several_audiences_and_the_provider_derives_the_union() {
    let source = provider(&format!(
        r#"
[[services]]
name = "monitoring"
audiences = ["sre", "developer"]

[[services]]
name = "revenue"
audiences = ["sales-rep", "developer"]
{}{}"#,
        operation("acme-monitor-list", "monitoring"),
        operation("acme-account-list", "revenue"),
    ));

    let connector = load(&source).expect("several audiences on separate services must load");
    assert_eq!(
        connector.audiences(),
        vec![Audience::Sre, Audience::Developer, Audience::SalesRep]
    );
}

#[test]
fn an_unknown_audience_is_refused_and_names_the_known_set() {
    let source = provider(&format!(
        r#"
[[services]]
name = "monitoring"
audiences = ["site-reliabilty"]
{}"#,
        operation("acme-monitor-list", "monitoring")
    ));

    let error = refusal(&source);
    assert!(
        error.contains("site-reliabilty"),
        "rejected value missing: {error}"
    );
    assert!(error.contains("sre"), "known audience missing: {error}");
}

#[test]
fn a_repeated_audience_is_refused() {
    let source = provider(&format!(
        r#"
[[services]]
name = "monitoring"
audiences = ["sre", "sre"]
{}"#,
        operation("acme-monitor-list", "monitoring")
    ));

    let error = refusal(&source);
    assert!(
        error.contains("audience \"sre\" more than once"),
        "wrong refusal: {error}"
    );
}

#[test]
fn a_provider_level_audiences_key_is_refused() {
    let source = provider(&format!(
        r#"
audiences = ["developer"]
[[services]]
name = "default"
tags = ["developer-tools"]
{}"#,
        operation("acme-things-list", "default")
    ));

    assert!(refusal(&source).contains("audiences"));
}

#[test]
fn the_seeded_fleet_has_a_useful_cross_function_vocabulary() {
    let providers_dir = crate::shipped_provider::providers_dir();
    let mut distinct = Vec::new();

    for entry in std::fs::read_dir(providers_dir).expect("providers directory") {
        let path = entry.expect("provider entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("provider name");
        let connector = crate::shipped_provider::connector(name);
        for audience in connector.audiences() {
            if !distinct.contains(&audience) {
                distinct.push(audience);
            }
        }
    }

    assert!(
        distinct.len() >= 10,
        "audience filtering must span real functions, found only {distinct:?}"
    );
}
