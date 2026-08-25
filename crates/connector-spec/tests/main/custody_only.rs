//! **A provider may hold a credential it cannot spend** — S-070.
//!
//! Every other declaration describes a request surface, and the loader refuses one that describes
//! neither `[spec]` nor `[[operations]]`. `custody_only` is the escape, for the case where that
//! refusal is wrong: a credential whose *use* belongs to another component still needs an owner
//! for its address, its store, its rotation and its revocation. Platform ADR 0014's 2026-08-25
//! amendment admits exactly that, and design 16 states the terms.
//!
//! The kind is defined by what it refuses. Each of `[spec]`, `[[operations]]`, `[[service]]`,
//! `base_url` and `verify` is rejected rather than ignored, so the flag cannot carry a
//! half-declared ordinary provider past review, and `[[auth]]` is mandatory, because a
//! custody-only provider that holds nothing has no reason to exist.
//!
//! A synthetic fixture throughout: the shipped `claude-code` provider is S-071's write set, and
//! the rule under test is the loader's.

use connector_spec::{provider, Connector};

/// `head` carries top-level scalars, which TOML requires before the first table; `tail` carries
/// tables. Splitting them is not decoration — appending `base_url` after `[[auth]]` puts it *in*
/// the auth table, and the loader then refuses an unknown auth key rather than the rule under test.
fn custody_provider(head: &str, tail: &str) -> String {
    format!(
        r#"
id = "acme-custody"
vendor = "Acme"
authority = "com.acme.custody"
description = "A fixture for the custody-only declaration kind."
custody_only = true
{head}

[[auth]]
name = "acme.subscription_token"
scheme = "bearer"
entry = "connect_session"
subject = "user"
description = "A token another component spends."

{tail}
"#
    )
}

fn load(source: &str) -> connector_spec::Result<Connector> {
    provider::load("providers/acme-custody.toml", source).map(|loaded| loaded.connector)
}

fn refusal(source: &str) -> String {
    load(source)
        .expect_err("a custody-only provider that describes a request surface must be refused")
        .to_string()
}

#[test]
fn a_credential_and_nothing_else_loads() {
    let connector = load(&custody_provider("", "")).expect("a custody-only provider loads");
    assert!(connector.custody_only);
    assert!(
        connector.base_url.is_empty(),
        "there is no request to build a URL for"
    );
    assert!(connector.operations.is_empty());
    assert!(connector.services.is_empty());
    assert!(connector.verify.is_none());
    assert_eq!(connector.auth.len(), 1);
}

#[test]
fn every_key_that_would_describe_a_request_surface_is_refused_by_name() {
    for (key, head, tail) in [
        ("`base_url`", r#"base_url = "https://api.acme.example""#, ""),
        ("`verify`", r#"verify = "acme-ping""#, ""),
        (
            "`[spec]`",
            "",
            "[spec]\npath = \"specs/acme/api.openapi.yaml\"\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"",
        ),
        (
            "`[[service]]`",
            "",
            "[[services]]\nname = \"api\"\ndescription = \"A surface.\"",
        ),
        (
            "`[[operations]]`",
            "",
            r#"[[operations]]
id = "acme-ping"
description = "Check service availability"
method = "GET"
direction = "read"
path = "/ping"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]"#,
        ),
    ] {
        let message = refusal(&custody_provider(head, tail));
        assert!(
            message.contains(key),
            "the refusal must name {key}, so an author knows which line to delete; got: {message}"
        );
        assert!(
            message.contains("custody_only"),
            "the refusal must name the kind that caused it; got: {message}"
        );
    }
}

#[test]
fn a_custody_only_provider_holding_nothing_is_refused() {
    let source = r#"
id = "acme-custody"
vendor = "Acme"
authority = "com.acme.custody"
description = "A fixture that declares the kind and then holds nothing."
custody_only = true
"#;
    let message = refusal(source);
    assert!(
        message.contains("no `[[auth]]`"),
        "a provider that holds nothing has no reason to exist; got: {message}"
    );
}

#[test]
fn an_ordinary_provider_still_needs_a_base_url_and_a_surface() {
    // The escape must not have widened the rule for everyone else. Both of these refused before
    // `custody_only` existed and must still refuse.
    let no_surface = r#"
id = "acme"
vendor = "Acme"
base_url = "https://api.acme.example"
description = "An ordinary provider that describes nothing."
"#;
    assert!(
        refusal(no_surface).contains("describes no operations at all"),
        "the pre-existing refusal is unchanged for a provider that did not opt in"
    );

    let no_base_url = r#"
id = "acme"
vendor = "Acme"
description = "An ordinary provider with no base URL."

[[operations]]
id = "acme-ping"
description = "Check service availability"
method = "GET"
direction = "read"
path = "/ping"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]
"#;
    assert!(
        refusal(no_base_url).contains("`base_url` must not be empty"),
        "`base_url` is optional only for the kind that refuses it outright"
    );
}

#[test]
fn the_flag_is_in_the_hash_domain_but_costs_absent_declarations_nothing() {
    let custody = load(&custody_provider("", "")).expect("custody-only loads");
    let mut ordinary = custody.clone();
    ordinary.custody_only = false;
    assert_ne!(
        custody
            .ir_sha256()
            .expect("hash the custody-only connector"),
        ordinary
            .ir_sha256()
            .expect("hash the same connector without the flag"),
        "two files differing only here are two different connectors"
    );

    // And the encoding is skipped when false, which is what keeps every provider that predates the
    // kind hashing to what it hashed before.
    let encoded = serde_json::to_string(&ordinary).expect("encode");
    assert!(
        !encoded.contains("custody_only"),
        "a false flag must not appear in the canonical bytes, or `connectors.lock` churns for 64 \
         providers nobody edited"
    );
}
