use super::*;

/// The keys the loader actually accepts, per documented object, **as serde reports them**.
///
/// This is the machinery behind "a JSON Schema kept in sync by a test". Hand-written schemas rot;
/// the only cure is to ask the code rather than the author. Each entry is produced by handing the
/// type a key it cannot possibly know and reading the field list out of `deny_unknown_fields`'
/// own error — so the answer is derived from the `Deserialize` impl that will parse real provider
/// files, not from a second list that could disagree with it.
///
/// `tests/provider_schema.rs` asserts that this map and the schema's `$defs` describe the same
/// objects with the same properties. Adding a field to any IR type therefore fails that test until
/// the schema documents it.
///
/// The object names are the schema's `$defs` keys, not Rust type names — the schema is the
/// published artifact, so it gets to choose the vocabulary.
pub fn accepted_keys() -> Vec<(&'static str, Vec<String>)> {
    vec![
        ("provider", probe::<ProviderFile>()),
        ("service", probe::<Service>()),
        ("spec", probe::<SpecSource>()),
        ("patch", probe::<Patch>()),
        ("operationSelector", probe::<OperationSelector>()),
        ("naming", probe::<Naming>()),
        ("operationPatch", probe::<OperationPatch>()),
        ("eventPatch", probe::<EventPatch>()),
        ("paramPatch", probe::<ParamPatch>()),
        ("paramOmission", probe::<ParamOmission>()),
        ("authMethod", probe::<AuthMethod>()),
        ("oauth2", probe::<crate::OAuth2Spec>()),
        ("oauthRedirect", probe::<crate::OAuthRedirect>()),
        ("authWorkarounds", probe::<crate::AuthWorkarounds>()),
        (
            "tokenEndpointWorkaround",
            probe::<crate::TokenEndpointWorkaround>(),
        ),
        ("authRequirement", probe::<AuthRequirement>()),
        ("operation", probe::<Operation>()),
        ("producedCredential", probe::<crate::ProducedCredential>()),
        ("event", probe::<EventDecl>()),
        ("channel", probe::<ChannelBinding>()),
        ("discovery", probe::<Discovery>()),
        ("discoveryMapping", probe::<DiscoveryMapping>()),
        ("sessionBinding", probe::<SessionBinding>()),
        ("socketConnect", probe::<SocketConnectSpec>()),
        ("configField", probe::<ConfigField>()),
        ("choice", probe::<crate::Choice>()),
        ("graph", probe::<Graph>()),
        ("graphNode", probe::<GraphNode>()),
        ("port", probe::<crate::graph::Port>()),
        ("portRef", probe::<PortRef>()),
        ("edge", probe::<crate::graph::Edge>()),
        ("condition", probe::<crate::graph::Condition>()),
        ("subscription", probe::<Subscription>()),
        ("manualSetup", probe::<ManualSetup>()),
        ("hmac", probe::<HmacSpec>()),
        ("selector", probe::<Selector>()),
        ("reply", probe::<Reply>()),
        ("paramSet", probe::<ParamSet>()),
        ("param", probe::<Param>()),
        ("pagination", probe::<Pagination>()),
        ("rateLimit", probe::<crate::RateLimit>()),
        ("errorEnvelope", probe::<crate::ErrorEnvelope>()),
        ("provenance", probe::<Provenance>()),
        ("operationSpecSource", probe::<OperationSpecSource>()),
    ]
}

/// A key no provider TOML will ever contain, used to make `deny_unknown_fields` name its alternatives.
const UNKNOWN_KEY_PROBE: &str = "__connector_spec_unknown_key_probe__";

/// Asks `T` which keys it accepts, by feeding it one it does not.
///
/// Panics if `T` accepts the probe key or reports no alternatives — either would mean the type is
/// not `deny_unknown_fields`, which is the very property this crate's strictness rests on, so a
/// panic in a test helper is the right loudness.
fn probe<T: serde::de::DeserializeOwned>() -> Vec<String> {
    let document = format!("{{\"{UNKNOWN_KEY_PROBE}\": null}}");
    let error = serde_json::from_str::<T>(&document)
        .err()
        .unwrap_or_else(|| {
            panic!(
                "{} accepted an unknown key — it is missing `deny_unknown_fields`",
                std::any::type_name::<T>()
            )
        })
        .to_string();

    let keys = expected_fields(&error);
    assert!(
        !keys.is_empty(),
        "could not read the accepted keys of {} out of: {error}",
        std::any::type_name::<T>()
    );
    keys
}

/// Extracts the backtick-quoted field names serde lists after "expected one of" (or "expected", for
/// a single-field struct).
fn expected_fields(error: &str) -> Vec<String> {
    let Some(offset) = error.find("expected") else {
        return Vec::new();
    };
    error[offset..]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}
