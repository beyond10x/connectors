//! The hosted server's own contract surface (S-067 + S-068).
//!
//! `GET {base_path}/openapi.json` serves a committed OpenAPI 3.1 document, unauthenticated
//! and verbatim: the artifact beside this module is the contract, embedded at compile time
//! and never derived from the running code. Drift is caught by the test suite instead —
//! every request example in the document must be accepted by the exact `protocol` types the
//! routes deserialize with, and every documented route must exist in the real router
//! (`hosted/tests/docs.rs`).
//!
//! `GET {base_path}/docs` renders that same document as one self-contained HTML page for a
//! person: authentication, the envelope endpoints, MCP, datasources, and the refusal codes.
//! Everything the page states — the version, the audience and scopes, every example, every
//! refusal code — is extracted from the embedded document at first render, never repeated
//! in this source, so the artifact stays the single source of truth. The page is static and
//! tenant-free: the handler reads no state, takes no auth, and renders nothing derived from
//! the request.
//!
//! Both answers are immutable per build, so each `ETag` is the SHA-256 of the exact served
//! bytes.

use std::sync::OnceLock;

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use sha2::Digest as _;

/// The committed skeleton: paths, info, security, and the schemas that have no
/// struct source (the hand-rolled MCP transport and the axum-level error body).
/// Every envelope schema is generated from the `protocol` payload structs at first
/// use — the structs are the contract; the skeleton only says where they bind.
const OPENAPI_SKELETON: &str = include_str!("docs/openapi.json");

/// The served document: skeleton plus the ten envelope schemas generated from the
/// exact `protocol` types the routes deserialize with, inlined so no name can
/// collide across modules. Built once per process.
pub(in crate::hosted) fn document_json() -> &'static str {
    static DOCUMENT: OnceLock<String> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        let mut document: Value =
            serde_json::from_str(OPENAPI_SKELETON).expect("the committed OpenAPI skeleton is JSON");
        let schemas = document
            .pointer_mut("/components/schemas")
            .expect("the skeleton declares components.schemas");
        let mut settings = schemars::generate::SchemaSettings::draft2020_12();
        settings.inline_subschemas = true;
        fn generated<T: schemars::JsonSchema>(
            settings: &schemars::generate::SchemaSettings,
        ) -> Value {
            let mut schema = settings.clone().into_generator().root_schema_for::<T>();
            let object = schema.ensure_object();
            object.remove("$schema");
            Value::Object(std::mem::take(object))
        }
        let entries: [(&str, Value); 10] = [
            (
                "operation.requestEnvelope",
                generated::<protocol::operation::RequestEnvelope>(&settings),
            ),
            (
                "operation.responseEnvelope",
                generated::<protocol::operation::ResponseEnvelope>(&settings),
            ),
            (
                "connection.request_envelope",
                generated::<protocol::connection::RequestEnvelope>(&settings),
            ),
            (
                "connection.response_envelope",
                generated::<protocol::connection::ResponseEnvelope>(&settings),
            ),
            (
                "catalog.requestEnvelope",
                generated::<protocol::catalog::RequestEnvelope>(&settings),
            ),
            (
                "catalog.responseEnvelope",
                generated::<protocol::catalog::ResponseEnvelope>(&settings),
            ),
            (
                "event.request_envelope",
                generated::<protocol::event::RequestEnvelope>(&settings),
            ),
            (
                "event.response_envelope",
                generated::<protocol::event::ResponseEnvelope>(&settings),
            ),
            (
                "datasource.request_envelope",
                generated::<protocol::datasource::RequestEnvelope>(&settings),
            ),
            (
                "datasource.response_envelope",
                generated::<protocol::datasource::ResponseEnvelope>(&settings),
            ),
        ];
        let contracts: [(&str, &str); 10] = [
            ("operation.requestEnvelope", protocol::operation::CONTRACT),
            ("operation.responseEnvelope", protocol::operation::CONTRACT),
            (
                "connection.request_envelope",
                protocol::connection::CONTRACT,
            ),
            (
                "connection.response_envelope",
                protocol::connection::CONTRACT,
            ),
            ("catalog.requestEnvelope", protocol::catalog::CONTRACT),
            ("catalog.responseEnvelope", protocol::catalog::CONTRACT),
            ("event.request_envelope", protocol::event::CONTRACT),
            ("event.response_envelope", protocol::event::CONTRACT),
            (
                "datasource.request_envelope",
                protocol::datasource::CONTRACT,
            ),
            (
                "datasource.response_envelope",
                protocol::datasource::CONTRACT,
            ),
        ];
        let table = schemas
            .as_object_mut()
            .expect("components.schemas is an object");
        for (name, mut schema) in entries {
            // The `protocol` field is an open string on the struct; `validate()` refuses
            // anything but the module's contract identity, and the document says so.
            let contract = contracts
                .iter()
                .find(|(entry, _)| *entry == name)
                .map(|(_, contract)| *contract)
                .expect("every envelope root has a contract identity");
            if let Some(field) = schema.pointer_mut("/properties/protocol") {
                *field = serde_json::json!({ "type": "string", "const": contract });
            }
            table.insert(name.to_owned(), schema);
        }
        serde_json::to_string_pretty(&document).expect("the merged document serializes")
    })
}

/// The five envelope endpoints in reading order, each with the request and response
/// example name the page leads with. Every name must exist in the document; a rename
/// there panics the first render, which the drift tests exercise on every run.
const ENVELOPE_SECTIONS: [(&str, &str, &str); 5] = [
    ("/operations", "search", "success"),
    ("/connections", "search", "success"),
    ("/catalog", "search", "success"),
    ("/events", "search", "success"),
    ("/datasources", "search", "success"),
];

/// The rest of the datasource method set gets its own blocks — `read` in both its
/// shapes is what makes the endpoint useful to a person, and the shapes differ.
const DATASOURCE_READ_EXAMPLES: [&str; 4] = ["describe", "bindings", "read_get", "read_list"];

/// The unauthenticated health probes, summarized from the document.
const HEALTH_PATHS: [&str; 3] = ["/livez", "/readyz", "/healthz"];

/// Inline stylesheet: brand palette from `web/public/brand/` (violet→cyan on deep
/// navy), no `url()`, no `@import` — the page must trigger zero external requests.
const STYLE: &str = "\
:root{--bg:#0F1629;--panel:#151d33;--edge:#26304d;--ink:#e8ebf4;--muted:#9aa4c0;\
--brand:#7C5CFF;--accent:#22D3EE}\
*{box-sizing:border-box}\
body{margin:0 auto;max-width:64rem;padding:2rem 1.25rem 4rem;background:var(--bg);\
color:var(--ink);font:16px/1.6 system-ui,sans-serif}\
a{color:var(--accent)}\
h1{font-size:1.6rem;margin:.25rem 0 0}\
h2{margin-top:2.5rem;border-bottom:1px solid var(--edge);padding-bottom:.35rem}\
h3{margin-top:2rem}\
h4{margin:1.25rem 0 .4rem;color:var(--muted);font-size:.95rem}\
.wordmark{font-weight:800;font-size:1.05rem;letter-spacing:.04em;margin:0;\
background:linear-gradient(90deg,var(--brand),var(--accent));\
-webkit-background-clip:text;background-clip:text;color:transparent}\
.meta{color:var(--muted);margin:.35rem 0 0}\
.version{border:1px solid var(--edge);border-radius:.5rem;padding:.05rem .5rem;\
background:var(--panel);color:var(--ink)}\
nav{display:flex;flex-wrap:wrap;gap:.4rem .9rem;margin-top:1.25rem}\
pre{background:var(--panel);border:1px solid var(--edge);border-radius:.6rem;\
padding:.9rem 1rem;overflow-x:auto;font-size:.84rem;line-height:1.5}\
code{background:var(--panel);border:1px solid var(--edge);border-radius:.35rem;\
padding:.05rem .3rem;font-size:.85em}\
pre code{border:0;padding:0;background:none}\
table{border-collapse:collapse;width:100%;margin-top:.75rem}\
th,td{border:1px solid var(--edge);padding:.5rem .65rem;text-align:left;vertical-align:top}\
th{color:var(--muted);font-weight:600}\
.lede{color:var(--muted)}\
.derived{color:var(--muted);font-size:.92rem}\
footer{margin-top:3rem;border-top:1px solid var(--edge);padding-top:1rem;\
color:var(--muted);font-size:.9rem}";

/// Quoted lowercase-hex SHA-256 of the exact served bytes.
fn quoted_content_hash(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    let mut etag = String::with_capacity(66);
    etag.push('"');
    for byte in digest {
        etag.push_str(&format!("{byte:02x}"));
    }
    etag.push('"');
    etag
}

/// [`quoted_content_hash`] of the generated document, computed once per process.
fn content_hash_etag() -> &'static str {
    static ETAG: OnceLock<String> = OnceLock::new();
    ETAG.get_or_init(|| quoted_content_hash(document_json().as_bytes()))
}

/// [`quoted_content_hash`] of the rendered page, computed once per process.
fn page_etag() -> &'static str {
    static ETAG: OnceLock<String> = OnceLock::new();
    ETAG.get_or_init(|| quoted_content_hash(page_html().as_bytes()))
}

/// The documentation page, rendered once per process from the committed document so
/// the page cannot state anything the contract does not.
fn page_html() -> &'static str {
    static PAGE: OnceLock<String> = OnceLock::new();
    PAGE.get_or_init(|| {
        let doc: Value =
            serde_json::from_str(document_json()).expect("the served OpenAPI document is JSON");
        render(&doc)
    })
}

/// `GET /openapi.json`: the committed contract, no authentication, content-hash `ETag`.
pub(super) async fn openapi() -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::ETAG, content_hash_etag()),
        ],
        document_json(),
    )
        .into_response()
}

/// `GET /docs`: the public documentation page — static, unauthenticated, tenant-free.
/// The strict CSP turns the self-containment promise into a browser-enforced rule:
/// no scripts, no remote fetches, styles inline only.
pub(super) async fn page() -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::ETAG, page_etag()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; \
                 form-action 'none'",
            ),
        ],
        page_html(),
    )
        .into_response()
}

/// Escape text for HTML element and attribute content.
fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// One named example value from the document. Unreachable panics are acceptable here:
/// the drift tests render the page on every run, so a missing name cannot ship.
fn example<'a>(doc: &'a Value, path: &str, kind: &str, name: &str) -> &'a Value {
    let operation = &doc["paths"][path]["post"];
    let container = match kind {
        "request" => &operation["requestBody"],
        "response" => &operation["responses"]["200"],
        other => panic!("unknown example kind `{other}`"),
    };
    let value = &container["content"]["application/json"]["examples"][name]["value"];
    assert!(
        !value.is_null(),
        "the document carries `{path}` {kind} example `{name}`"
    );
    value
}

/// One example JSON block, marked with `data-example="<path> <kind> <name>"` so the
/// drift tests can trace every shown value back to the document byte for byte.
fn example_block(doc: &Value, path: &str, kind: &str, name: &str, heading: &str) -> String {
    let json = serde_json::to_string_pretty(example(doc, path, kind, name))
        .expect("a document example re-serializes");
    format!(
        "<h4>{heading}</h4>\n<pre class=\"json\" data-example=\"{path} {kind} {name}\">{}</pre>\n",
        escape_html(&json)
    )
}

/// A copy-pastable curl for one documented request example. The body travels through a
/// quoted heredoc so no shell escaping can corrupt it, and the embedded JSON carries the
/// same drift-test marker as every other example block.
fn curl_block(doc: &Value, path: &str, name: &str) -> String {
    let json = serde_json::to_string_pretty(example(doc, path, "request", name))
        .expect("a document example re-serializes");
    format!(
        "<pre class=\"shell\">curl -sS \"$BASE{path}\" \\\n  \
         -H \"authorization: Bearer $ACCESS_TOKEN\" \\\n  \
         -H \"content-type: application/json\" \\\n  \
         --data-binary @- &lt;&lt;'JSON'\n\
         <span data-example=\"{path} request {name}\">{}</span>\nJSON</pre>\n",
        escape_html(&json)
    )
}

/// The audience the bearer description names (`urn:…`), extracted rather than repeated.
fn audience(bearer_description: &str) -> &str {
    bearer_description
        .split_whitespace()
        .find(|token| token.starts_with("urn:"))
        .map(|token| token.trim_end_matches(['.', ',']))
        .expect("the bearer scheme names its audience")
}

/// Every scope name the bearer description lists, in the order it lists them.
fn scopes(bearer_description: &str) -> Vec<&str> {
    let mut scopes = Vec::new();
    for token in bearer_description.split_whitespace() {
        let token = token
            .trim_start_matches('(')
            .trim_end_matches(['.', ',', ')']);
        if token.starts_with("connectors.") && !scopes.contains(&token) {
            scopes.push(token);
        }
    }
    assert!(!scopes.is_empty(), "the bearer scheme names its scopes");
    scopes
}

/// Every closed refusal-code vocabulary in the document, as `(family, codes)`.
/// Test seam: the drift suite asserts the page against the same extraction.
#[cfg(test)]
pub(in crate::hosted) fn refusal_rows_for_tests(doc: &Value) -> Vec<(String, Vec<String>)> {
    refusal_rows(doc)
}

fn refusal_rows(doc: &Value) -> Vec<(String, Vec<String>)> {
    let schemas = doc["components"]["schemas"]
        .as_object()
        .expect("the document declares schemas");
    let mut rows = Vec::new();
    for (family, envelope) in [
        ("operation", "operation.responseEnvelope"),
        ("connection", "connection.response_envelope"),
        ("catalog", "catalog.responseEnvelope"),
        ("event", "event.response_envelope"),
        ("datasource", "datasource.response_envelope"),
    ] {
        let codes = match find_code_enum(&schemas[envelope]) {
            Some(codes) => codes,
            // The catalog error code is an open string on the wire; the documented
            // refusal examples carry the codes the hosted route actually produces.
            None => catalog_example_codes(doc),
        };
        rows.push((family.to_owned(), codes));
    }
    rows
}

/// Depth-first search for the generated error object's `code` enum inside an inlined
/// envelope schema. The closed Rust error enums surface here; an open string does not.
fn find_code_enum(schema: &Value) -> Option<Vec<String>> {
    if let Some(codes) = schema
        .pointer("/properties/code/enum")
        .and_then(Value::as_array)
    {
        return Some(
            codes
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        );
    }
    match schema {
        Value::Object(map) => map.values().find_map(find_code_enum),
        Value::Array(items) => items.iter().find_map(find_code_enum),
        _ => None,
    }
}

/// The catalog refusal codes, read from the documented refusal examples on `/catalog`.
fn catalog_example_codes(doc: &Value) -> Vec<String> {
    let mut codes: Vec<String> = doc["paths"]["/catalog"]["post"]["responses"]
        .as_object()
        .expect("the catalog path documents responses")
        .values()
        .filter_map(|response| {
            response
                .pointer("/content/application~1json/examples")?
                .as_object()
        })
        .flat_map(|examples| examples.values())
        .filter_map(|example| example.pointer("/value/error/code")?.as_str())
        .map(str::to_owned)
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

/// Render the whole page from the parsed document. Every dynamic string is extracted
/// from `doc` and HTML-escaped; the only authored content is connective prose.
fn render(doc: &Value) -> String {
    let title = escape_html(
        doc["info"]["title"]
            .as_str()
            .expect("the document has a title"),
    );
    let version = escape_html(
        doc["info"]["version"]
            .as_str()
            .expect("the document has a version"),
    );
    let lede = escape_html(doc["info"]["description"].as_str().unwrap_or_default());
    let bearer = doc["components"]["securitySchemes"]["identityAccessToken"]["description"]
        .as_str()
        .expect("the bearer scheme carries a description");
    let audience = escape_html(audience(bearer));
    let scopes = scopes(bearer);
    let mint_scope = escape_html(&scopes[..scopes.len().min(2)].join(" "));

    let mut page = String::with_capacity(96 * 1024);
    page.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    page.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    page.push_str(&format!("<title>{title}</title>\n"));
    page.push_str("<style>");
    page.push_str(STYLE);
    page.push_str("</style>\n</head>\n<body>\n<header>\n<p class=\"wordmark\">b10x</p>\n");
    page.push_str(&format!("<h1>{title}</h1>\n"));
    page.push_str(&format!(
        "<p class=\"meta\"><span class=\"version\">{version}</span> · \
         <a href=\"openapi.json\">openapi.json</a> — the machine-readable contract this \
         page is rendered from.</p>\n"
    ));
    page.push_str(
        "</header>\n<nav>\n\
         <a href=\"#authentication\">Authentication</a>\n\
         <a href=\"#endpoints\">Envelope endpoints</a>\n\
         <a href=\"#mcp\">MCP</a>\n\
         <a href=\"#refusals\">Refusal codes</a>\n\
         <a href=\"#health\">Health</a>\n\
         </nav>\n<main>\n",
    );
    page.push_str(&format!("<p class=\"lede\">{lede}</p>\n"));

    // Authentication: identity login, then the access-token mint, then the bearer call.
    page.push_str(
        "<section id=\"authentication\">\n<h2>Authentication</h2>\n\
         <p>Every request except the health probes, <code>openapi.json</code> and this \
         page carries an identity-issued bearer token.</p>\n<ol>\n\
         <li>Log in against your deployment&#39;s identity authority (browser or CLI \
         login); the login leaves you with a short-lived identity session \
         credential.</li>\n",
    );
    page.push_str(&format!(
        "<li>Mint an access token for this service — audience <code>{audience}</code> — \
         with exactly the scopes you need:</li>\n</ol>\n\
         <pre class=\"shell\">curl -sS -X POST \"$IDENTITY_ORIGIN/v1/access-token\" \\\n  \
         -H \"authorization: Bearer $IDENTITY_SESSION\" \\\n  \
         -H \"content-type: application/json\" \\\n  \
         --data-binary @- &lt;&lt;'JSON'\n\
         {{\"audience\": \"{audience}\", \"scope\": \"{mint_scope}\"}}\nJSON</pre>\n\
         <p>The answer carries an <code>access_token</code>; tokens are short-lived, so \
         mint again when a request answers 401. Every call below passes it as \
         <code>authorization: Bearer</code>, spelled <code>$ACCESS_TOKEN</code>.</p>\n"
    ));
    page.push_str(&format!(
        "<p class=\"derived\">{}</p>\n</section>\n",
        escape_html(bearer)
    ));

    // The five envelope endpoints, each led by a working curl and a success response.
    page.push_str(
        "<section id=\"endpoints\">\n<h2>Envelope endpoints</h2>\n\
         <p>Five POST endpoints, one closed wire contract each: an unknown field \
         anywhere in a request is refused. <code>$BASE</code> is the deployment&#39;s \
         base URL.</p>\n",
    );
    for (path, request_name, response_name) in ENVELOPE_SECTIONS {
        let operation = &doc["paths"][path]["post"];
        let summary = escape_html(operation["summary"].as_str().unwrap_or_default());
        let description = escape_html(operation["description"].as_str().unwrap_or_default());
        let slug = path.trim_start_matches('/');
        page.push_str(&format!(
            "<section id=\"{slug}\">\n<h3><code>POST {path}</code></h3>\n\
             <p><strong>{summary}.</strong> {description}</p>\n"
        ));
        page.push_str(&format!("<h4>Request — {request_name}</h4>\n"));
        page.push_str(&curl_block(doc, path, request_name));
        page.push_str(&example_block(
            doc,
            path,
            "response",
            response_name,
            &format!("Response — {response_name}"),
        ));
        if path == "/datasources" {
            page.push_str(
                "<p>The read verbs and the rest of the closed method set, as request \
                 bodies for the same endpoint:</p>\n",
            );
            for name in DATASOURCE_READ_EXAMPLES {
                page.push_str(&example_block(
                    doc,
                    path,
                    "request",
                    name,
                    &format!("Request — {name}"),
                ));
            }
        }
        page.push_str("</section>\n");
    }
    page.push_str("</section>\n");

    // The MCP entry point: initialize and tools/call, from the document's own examples.
    {
        let operation = &doc["paths"]["/mcp"]["post"];
        let summary = escape_html(operation["summary"].as_str().unwrap_or_default());
        let description = escape_html(operation["description"].as_str().unwrap_or_default());
        page.push_str(&format!(
            "<section id=\"mcp\">\n<h2>MCP</h2>\n<h3><code>POST /mcp</code></h3>\n\
             <p><strong>{summary}.</strong> {description}</p>\n"
        ));
        page.push_str("<h4>Request — initialize</h4>\n");
        page.push_str(&curl_block(doc, "/mcp", "initialize"));
        page.push_str(&example_block(
            doc,
            "/mcp",
            "response",
            "initialize",
            "Response — initialize",
        ));
        page.push_str("<h4>Request — tools/call</h4>\n");
        page.push_str(&curl_block(doc, "/mcp", "tools_call"));
        page.push_str("</section>\n");
    }

    // The refusal-code table, from the document's own closed enums.
    page.push_str(
        "<section id=\"refusals\">\n<h2>Refusal codes</h2>\n\
         <p>A refusal is structured: <code>status</code> is <code>error</code> and the \
         error carries a code from the endpoint&#39;s closed vocabulary. The \
         vocabularies, from the document&#39;s own enums:</p>\n\
         <table>\n<thead><tr><th>envelope</th><th>codes</th></tr></thead>\n<tbody>\n",
    );
    for (family, codes) in refusal_rows(doc) {
        let codes = codes
            .iter()
            .map(|code| format!("<code>{}</code>", escape_html(code)))
            .collect::<Vec<_>>()
            .join(" ");
        page.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{codes}</td></tr>\n",
            escape_html(&family)
        ));
    }
    page.push_str("</tbody>\n</table>\n</section>\n");

    // Health probes: unauthenticated, summarized from the document.
    page.push_str("<section id=\"health\">\n<h2>Health</h2>\n<ul>\n");
    for path in HEALTH_PATHS {
        let summary = escape_html(
            doc["paths"][path]["get"]["summary"]
                .as_str()
                .unwrap_or_default(),
        );
        page.push_str(&format!("<li><code>GET {path}</code> — {summary}.</li>\n"));
    }
    page.push_str("</ul>\n<pre class=\"shell\">curl -sS \"$BASE/readyz\"</pre>\n</section>\n");

    page.push_str(&format!(
        "</main>\n<footer>\n<p>b10x Connectors — rendered from \
         <a href=\"openapi.json\">the OpenAPI document</a>, version {version}. One page, \
         zero external requests.</p>\n</footer>\n</body>\n</html>\n"
    ));
    page
}
