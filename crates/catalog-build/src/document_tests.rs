#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_approved_origin_may_be_the_whole_base_url() {
        assert_eq!(origin_template("{origin}"), Some("origin"));
        assert_eq!(origin_template("{origin}/api/v1"), Some("origin"));
        assert_eq!(origin_template("prefix-{origin}"), None);
    }

    /// A complete hand-authored connector the loader accepts, parameterised on one operation body.
    fn connector_with(operations: &str) -> Connector {
        let definition = format!(
            r#"
id = "acme"
vendor = "Acme"
base_url = "https://api.acme.example"
default_auth = [{{ credentials = ["acme.token"] }}]

[[auth]]
name = "acme.token"
scheme = "bearer"
env = ["ACME_TOKEN"]

{operations}
"#
        );
        connector_spec::provider::load_with_spec("providers/acme.toml", &definition, &[])
            .expect("the fixture loads")
            .connector
    }

    /// **The closed-template refusal** (failing-first for C-536's third acceptance item): a
    /// construct `connector-pack`'s evaluator refuses today — a vendor's own brace-carrying
    /// syntax pinned as a body literal, the C-110 shape — is a **build error** in document
    /// lowering, never a silently degraded document.
    #[test]
    fn an_unclassifiable_brace_literal_is_a_build_error_never_a_degraded_document() {
        let connector = connector_with(
            r#"
[[operations]]
id = "acme-graph-query"
method = "POST"
direction = "read"
path = "/graphql"
description = "The C-110 shape: a pinned query document whose braces are vendor syntax"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]

[[operations.params.body]]
name = "query"
required = true
schema = { type = "string", const = "query { viewer { login } }" }
"#,
        );

        let error = render(&connector).expect_err("a C-110-shaped literal must not lower");
        let message = format!("{error:#}");
        assert!(
            message.contains("acme-graph-query") && message.contains("closed"),
            "the refusal names the operation and the closed vocabulary: {message}"
        );
    }

    /// The same closure, for a constant header whose value carries a brace.
    #[test]
    fn a_braced_constant_header_is_refused() {
        let connector = connector_with(
            r#"
[[operations]]
id = "acme-thing-get"
method = "GET"
direction = "read"
path = "/things"
description = "Get things"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]

[operations.params.const_headers]
"X-Weird" = "{templated}"
"#,
        );

        let error = render(&connector).expect_err("a braced constant header must not lower");
        assert!(
            format!("{error:#}").contains("X-Weird"),
            "the refusal names the header: {error:#}"
        );
    }

    /// A provider file declaring a non-empty OAuth2 registration value is a build error, not
    /// emitted data — the document has no field it could survive into.
    #[test]
    fn a_declared_registration_value_is_a_build_error() {
        let connector = connector_with(
            r#"
[[operations]]
id = "acme-thing-get"
method = "GET"
direction = "read"
path = "/things"
description = "Get things"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]

[[auth]]
name = "acme.oauth_token"
scheme = "bearer"
description = "OAuth2 access token"

[auth.oauth2]
token_path = "/oauth/token"
grants = ["authorization_code"]
client_id = "registered-elsewhere"
"#,
        );

        let error = render(&connector).expect_err("a registration value must not lower");
        let message = format!("{error:#}");
        assert!(
            message.contains("oauth.client_id"),
            "the refusal points at the `binds` grammar: {message}"
        );
    }

    /// A graph has no document lowering yet (deferred by the design's open questions); dropping
    /// a declared surface silently is the failure this artifact exists to end, so it refuses.
    #[test]
    fn a_declared_graph_is_refused_rather_than_dropped() {
        let mut connector = connector_with(
            r#"
[[operations]]
id = "acme-thing-get"
method = "GET"
direction = "read"
path = "/things"
description = "Get things"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]
"#,
        );
        connector.graphs.push(connector_spec::Graph {
            name: "flow".to_string(),
            service: connector_spec::DEFAULT_SERVICE.to_string(),
            description: String::new(),
            inputs: Vec::new(),
            output: None,
            expose: true,
            nodes: Vec::new(),
            edges: Vec::new(),
        });

        let error = render(&connector).expect_err("a graph must not vanish silently");
        assert!(
            format!("{error:#}").contains("graph"),
            "the refusal names the surface: {error:#}"
        );
    }

    /// A `form` body is representable: encoding, media type, pair order (always-sent pairs
    /// first, guarded ones after — `form_payload`'s partition), and a pinned `const` as a
    /// literal. Proven against a fixture because no shipped provider declares the encoding yet
    /// (C-144 landed the axis; `grep -rn 'body_encoding' providers/ | grep -v '#'` matches
    /// nothing) — the axis must not become sayable-in-Flux but unsayable-in-the-document.
    #[test]
    fn a_form_body_is_spelled_structurally() {
        let connector = connector_with(
            r#"
[[operations]]
id = "acme-message-send"
method = "POST"
direction = "write"
path = "/messages"
description = "Send a message"
risk = "medium"
idempotency = "non_idempotent"
effects = ["write", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]

[operations.params]
body_encoding = "form"

[[operations.params.body]]
name = "to"
required = true
schema = { type = "string" }

[[operations.params.body]]
name = "channel"
schema = { type = "string", const = "sms" }

[[operations.params.body]]
name = "note"
schema = { type = "string" }
"#,
        );

        let rendered = render(&connector).expect("the form fixture renders");
        let document: Value = serde_json::from_str(&rendered).expect("JSON");
        let request = &document["operations"][0]["request"];
        assert_eq!(
            request["headers"]["content-type"], "application/x-www-form-urlencoded",
            "the media type follows the declared encoding"
        );
        assert_eq!(request["body"]["encoding"], "form");
        assert_eq!(
            request["body"]["fields"],
            json!([
                { "name": "to", "value": { "$param": "to" }, "required": true },
                { "name": "channel", "value": "sms", "required": true },
                { "name": "note", "value": { "$param": "note" }, "required": false },
            ]),
            "always-sent pairs precede guarded ones, each group in declaration order"
        );
    }

    /// The rendered document is a fixed point: rendering twice yields identical bytes, and the
    /// text validates against the published schema (render already enforces it; this pins it).
    #[test]
    fn rendering_is_deterministic_and_validates() {
        let connector = connector_with(
            r#"
[[operations]]
id = "acme-thing-get"
method = "GET"
direction = "read"
path = "/things/{thing_id}"
description = "Get one thing"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]

[[operations.params.path]]
name = "thing_id"
required = true
schema = { type = "string" }

[[operations.params.query]]
name = "expand"
schema = { type = "string" }
"#,
        );

        let first = render(&connector).expect("the fixture renders");
        let second = render(&connector).expect("the fixture renders again");
        assert_eq!(first, second, "equal inputs must produce identical bytes");

        let document: Value = serde_json::from_str(&first).expect("the document is JSON");
        assert_eq!(document["schema_version"], SCHEMA_VERSION);
        let operation = &document["operations"][0];
        assert_eq!(operation["request"]["url"], "{base}/things/{thing_id}");
        assert_eq!(
            operation["request"]["query"][0]["value"],
            json!({ "$param": "expand" })
        );
    }

    /// **A custody-only provider renders** — S-070.
    ///
    /// Failing-first against the first implementation, which refused: `service_names` synthesises
    /// `DEFAULT_SERVICE` for a connector that declares none, that synthesised entry carries an
    /// empty `base_url`, and the document schema requires `minLength: 1`. The loader accepted the
    /// declaration and the build could not emit it, which is the worst of both — the refusal
    /// arrived one stage too late to name the file.
    ///
    /// The document publishes the flag rather than merely omitting everything. A consumer must be
    /// able to tell a provider that *happens* to have no operations from one whose declaration
    /// forbids ever having any; only the second is safe to hand a credential whose use belongs to
    /// another component.
    #[test]
    fn a_custody_only_connector_renders_a_document_with_no_surface() {
        let connector = connector_spec::provider::load(
            "providers/acme-custody.toml",
            r#"
id = "acme-custody"
vendor = "Acme"
authority = "com.acme.custody"
description = "A credential another component spends."
custody_only = true

[[auth]]
name = "acme.subscription_token"
scheme = "bearer"
entry = "connect_session"
subject = "user"
description = "A token another component spends."
"#,
        )
        .expect("the custody-only fixture loads")
        .connector;

        let rendered = render(&connector).expect("a custody-only provider renders");
        let document: Value = serde_json::from_str(&rendered).expect("the document is JSON");

        assert_eq!(document["custody_only"], json!(true));
        assert_eq!(
            document["services"],
            json!([]),
            "no service at all, rather than one implicit service with an empty base URL"
        );
        assert_eq!(document["operations"], json!([]));
        assert!(document.get("verify").is_none());
        assert_eq!(
            document["auth"][0]["name"], "acme.subscription_token",
            "the one thing it does publish is the credential it holds"
        );
        assert!(
            !rendered.contains("base_url"),
            "a document with no service states no base URL anywhere"
        );
    }

    /// The flag is absent from every ordinary document, so adding it moved no published byte.
    #[test]
    fn an_ordinary_connector_does_not_carry_the_flag() {
        let connector = connector_with(
            r#"
[[operations]]
id = "acme-thing-get"
description = "Read one thing"
method = "GET"
direction = "read"
path = "/things"
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["public_network"]
"#,
        );
        let rendered = render(&connector).expect("the fixture renders");
        assert!(
            !rendered.contains("custody_only"),
            "a false flag must not appear in the published bytes"
        );
    }
}
