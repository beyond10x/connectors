//! Personal admission is explicit, per flow, and contains no deployment registration values.

fn provider(admission: &str, grants: &str) -> String {
    format!(
        r#"
id = "acme"
vendor = "Acme"
authority = "com.acme.api"
base_url = "https://api.acme.example"
description = "Synthetic provider for personal admission validation."
[[services]]
name = "login"
base_url = "https://login.acme.example"
[[auth]]
name = "acme.oauth_token"
scheme = "bearer"
subject = "user"
[auth.oauth2]
endpoint = "login"
authorize_path = "/oauth/authorize"
token_path = "/oauth/token"
grants = [{grants}]
{admission}
[[operations]]
id = "acme-read"
service = "login"
method = "GET"
path = "/things"
description = "Read synthetic things."
direction = "read"
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

const PKCE: &str = r#"
[[auth.oauth2.personal_flows]]
flow = "authorization_code_pkce"
client_authentication = "public"
redirect_shape = "loopback_ipv4_http"
registration_use = "development_only"
refresh_policy = "required"
[auth.oauth2.personal_flows.token_evidence]
endpoint = { service = "login", path = "/oauth/token/info" }
scopes_pointer = "/scope"
scope_encoding = "string_array"
subject_pointer = "/resource_owner_id"
client_id_pointer = "/application/uid"
"#;

const DEVICE: &str = r#"
[[auth.oauth2.personal_flows]]
flow = "device_authorization"
client_authentication = "public"
registration_use = "production_allowed"
refresh_policy = "if_issued"
device_authorization_endpoint = { service = "login", path = "/oauth/authorize_device" }
[auth.oauth2.personal_flows.token_evidence]
endpoint = { service = "login", path = "/oauth/token/info" }
scopes_pointer = "/scope"
scope_encoding = "string_array"
subject_pointer = "/resource_owner_id"
client_id_pointer = "/application/uid"
"#;

fn load(source: &str) -> connector_spec::Result<connector_spec::Connector> {
    connector_spec::provider::load("providers/acme.toml", source).map(|p| p.connector)
}

#[test]
fn explicit_personal_pkce_admission_survives_loading_without_changing_legacy_public_client() {
    let connector = load(&provider(PKCE, "\"authorization_code\", \"refresh_token\""))
        .expect("explicit public PKCE admission must load");
    let oauth = connector.auth[0].oauth2.as_ref().unwrap();
    assert!(!oauth.public_client);
    assert!(oauth.client_id.is_empty());
    let value = serde_json::to_value(oauth).unwrap();
    assert_eq!(
        value["personal_flows"][0]["flow"],
        "authorization_code_pkce"
    );
    assert_eq!(
        value["personal_flows"][0]["token_evidence"]["client_id_pointer"],
        "/application/uid"
    );
}

#[test]
fn explicit_personal_device_admission_survives_without_a_redirect_or_refresh_promise() {
    let connector = load(&provider(
        DEVICE,
        "\"device_authorization\", \"refresh_token\"",
    ))
    .expect("explicit public device admission must load");
    let value = serde_json::to_value(connector.auth[0].oauth2.as_ref().unwrap()).unwrap();
    let flow = &value["personal_flows"][0];
    assert!(flow.get("redirect_shape").is_none());
    assert_eq!(flow["refresh_policy"], "if_issued");
    assert_eq!(flow["device_authorization_endpoint"]["service"], "login");
}

#[test]
fn personal_admission_refuses_ambiguous_flow_endpoints_evidence_and_registration_values() {
    for malformed in [
        format!("{PKCE}{PKCE}"),
        PKCE.replace("authorization_code_pkce", "device_authorization"),
        PKCE.replace("redirect_shape = \"loopback_ipv4_http\"", ""),
        PKCE.replace("service = \"login\"", "service = \"undeclared\""),
        PKCE.replace(
            "path = \"/oauth/token/info\"",
            "path = \"//outside.example/info\"",
        ),
        PKCE.replace("\"/scope\"", "\"/access_token\""),
        PKCE.replace("\"/application/uid\"", "\"application/uid\""),
        PKCE.replace(
            "refresh_policy = \"required\"",
            "refresh_policy = \"required\"\nclient_id = \"not-catalog-data\"",
        ),
        DEVICE.replace(
            "flow = \"device_authorization\"",
            "flow = \"device_authorization\"\nredirect_shape = \"loopback_ipv4_http\"",
        ),
    ] {
        assert!(load(&provider(
            &malformed,
            "\"authorization_code\", \"device_authorization\", \"refresh_token\""
        ))
        .is_err());
    }
    assert!(load(&provider(PKCE, "\"refresh_token\"")).is_err());
    assert!(load(&provider(DEVICE, "\"authorization_code\"")).is_err());
}

#[test]
fn legacy_public_client_does_not_invent_personal_admission() {
    let connector = load(&provider("public_client = true", "\"authorization_code\""))
        .expect("legacy OAuth still loads");
    let value = serde_json::to_value(connector.auth[0].oauth2.as_ref().unwrap()).unwrap();
    assert!(value.get("personal_flows").is_none());
}

#[test]
fn oauth_pass1_explicit_empty_personal_admission_obeys_the_authoring_contract() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../schema/provider-toml.schema.json")).unwrap();
    assert_eq!(
        schema["$defs"]["oauth2"]["properties"]["personal_flows"]["minItems"],
        1
    );
    let omitted = provider("", "\"authorization_code\", \"refresh_token\"");
    assert!(
        load(&omitted).is_ok(),
        "omission keeps the legacy acquisition"
    );
    let explicit = provider(
        "personal_flows = []",
        "\"authorization_code\", \"refresh_token\"",
    );
    assert!(
        load(&explicit).is_err(),
        "the published minItems=1 contract rejects explicit empty admission; the actual provider loader must retain that presence distinction"
    );
}

#[test]
fn oauth_pass2_presence_round_trips_across_formats_without_changing_legacy_arrays() {
    use connector_spec::OAuth2Spec;
    for admission in ["", PKCE, DEVICE] {
        let loaded = load(&provider(
            admission,
            "\"authorization_code\", \"device_authorization\", \"refresh_token\"",
        ))
        .unwrap();
        let original = loaded.auth[0].oauth2.as_ref().unwrap();
        let json = serde_json::to_string(original).unwrap();
        let toml = toml::to_string(original).unwrap();
        let yaml = serde_norway::to_string(original).unwrap();
        assert_eq!(
            &serde_json::from_str::<OAuth2Spec>(&json).unwrap(),
            original
        );
        assert_eq!(&toml::from_str::<OAuth2Spec>(&toml).unwrap(), original);
        assert_eq!(
            &serde_norway::from_str::<OAuth2Spec>(&yaml).unwrap(),
            original
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json)
                .unwrap()
                .get("personal_flows")
                .is_none(),
            admission.is_empty()
        );
    }
    for empty in [
        "personal_flows = []",
        "'personal_flows' = [\n# explicitly supplied\n]",
        "personal_flows = [ ]\nscopes = []",
    ] {
        assert!(load(&provider(empty, "\"authorization_code\"")).is_err());
    }
    let legacy: OAuth2Spec =
        serde_json::from_str(r#"{"grants":[],"scopes":[],"public_client":true}"#).unwrap();
    assert!(legacy.personal_flows.is_empty());
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::json!({"public_client":true})
    );
    for bad in ["[]", "null", "{}", "true", "\"[]\"", "[null]", "[{}]"] {
        let json = format!(r#"{{"personal_flows":{bad}}}"#);
        assert!(serde_json::from_str::<OAuth2Spec>(&json).is_err(), "{bad}");
        assert!(
            serde_norway::from_str::<OAuth2Spec>(&json).is_err(),
            "{bad}"
        );
    }
}
