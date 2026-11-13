//! HubSpot CRM records and owners through the catalog provider.
//!
//! The shipped selection set is pinned by id and source operation, resolves
//! against the committed bundle compiled from the pinned CRM Objects `2026-09`
//! document and the two owner reads cited from the pinned Owners `2026-09`
//! document, and is cited row by row in `docs/catalog-hubspot.md`. Each read
//! runs through the provider child against a disposable HTTPS fixture: the
//! exact request (path, query, `Authorization: Bearer …`) and the returned body
//! are asserted, the list walks two pages to the end condition the guide
//! documents, and the account-details identity read yields the portal id. No
//! live credential and no network.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

/// The document's server is `https://api.hubapi.com` with no path, so every
/// operation and the identity read sit below the authority's root.
const BASE: &str = "/";
/// A fictional private-app access token.
const TOKEN: &str = "fixture-hubspot-private-app-token";
const HEADER: &str = "Bearer fixture-hubspot-private-app-token";
const PROFILE: &str = "hubspot.private-app";
/// The pinned source, its manifest and README, relative to this crate.
const UPSTREAM: &str = "../hubspot/upstream";
const SOURCE: &str = "crm-objects-2026-09.json";
/// The owners document and the cited amendments that add its two reads to the
/// bundle, which has one source.
const OWNERS: &str = "crm-owners-2026-09.json";
const AMENDMENTS: &str = "hubspot-owners.amendments.json";

/// The shipped ids, their pinned `operationId` and the path the bundle
/// records. A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 4] = [
    (
        "object.get",
        "get-/crm/objects/2026-09/{objectType}/{objectId}_/crm/objects/2026-03/{objectType}/{objectId}",
        "/crm/objects/2026-09/{objectType}/{objectId}",
    ),
    (
        "objects.list",
        "get-/crm/objects/2026-09/{objectType}_/crm/objects/2026-03/{objectType}",
        "/crm/objects/2026-09/{objectType}",
    ),
    (
        "owner.get",
        "get-/crm/owners/2026-09/{ownerId}_getById",
        "/crm/owners/2026-09/{ownerId}",
    ),
    (
        "owners.list",
        "get-/crm/owners/2026-09_/crm/owners/v3",
        "/crm/owners/2026-09",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/hubspot/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "hubspot");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM).join(SOURCE)).unwrap()).unwrap()
}

#[test]
fn shipped_hubspot_selections_are_exactly_the_four_reads() {
    let selections = shipped();
    let bundle = bundle::load(&root().join("generated/bundles"), "hubspot").unwrap();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(declared, expected);
    assert!(engine.declarations(&[Effect::Write]).is_empty());
    for (id, operation_id, path) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the pinned source lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
}

/// Search is a POST in the pinned document, and the engine admits a read only
/// through GET, so it cannot ship as a read (`story:catalog-post-reads`).
#[test]
fn search_cannot_ship_as_a_read() {
    let bundle = bundle::load(&root().join("generated/bundles"), "hubspot").unwrap();
    let mut selections = shipped();
    selections[0].operation_id =
        "post-/crm/objects/2026-09/{objectType}/search_/crm/objects/2026-03/{objectType}/search"
            .into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
    selections[0].operation_id = "getPages".into();
    assert!(Engine::new(&bundle, BASE, &selections).is_err());
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("hubspot").unwrap();
    assert_eq!(entry.source_sha256, digest);
    // The 11 operations of the CRM Objects document and the two owner reads
    // the amendments add.
    assert_eq!(entry.operations, 13);
    assert_eq!(entry.unsupported, 0);
    let bundle = bundle::load(&root().join("generated/bundles"), "hubspot").unwrap();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    let amendments = fs::read(upstream.join(AMENDMENTS)).unwrap();
    let applied = bundle
        .source
        .amendments
        .as_ref()
        .expect("the bundle records its amendments");
    assert_eq!(applied.file_name, AMENDMENTS);
    assert_eq!(applied.sha256, hex::encode(Sha256::digest(&amendments)));
    assert_eq!(applied.count, 2);
    let owners = fs::read(upstream.join(OWNERS)).unwrap();
    let owners_digest = hex::encode(Sha256::digest(&owners));
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("hubspot-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 2, "two pinned HubSpot documents");
    assert_eq!(records[0]["file"], SOURCE);
    assert_eq!(records[0]["sha256"], digest.as_str());
    assert_eq!(records[0]["bytes"], bytes.len());
    assert_eq!(records[1]["file"], OWNERS);
    assert_eq!(records[1]["sha256"], owners_digest.as_str());
    assert_eq!(records[1]["bytes"], owners.len());
    assert_eq!(records[1]["archive"], "vendor/crm-owners-2026-09.json.gz");
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    assert!(readme.contains(&digest), "README does not name the digest");
    assert!(
        readme.contains(&owners_digest),
        "README does not name the owners digest"
    );
    // The README says the documents declare neither licence nor terms.
    assert_eq!(pinned()["info"]["license"], Value::Null);
    assert_eq!(pinned()["info"]["termsOfService"], Value::Null);
    let owners: Value = serde_json::from_slice(&owners).unwrap();
    assert_eq!(owners["info"]["license"], Value::Null);
    assert_eq!(owners["info"]["termsOfService"], Value::Null);
}

/// The one amendment parameter object the Owners document's parameter says, in
/// every field the amendment format carries: `name`, `location`, `required`,
/// `type`, `repeated` (an array, sent one pair per element, whose `type` is
/// the element's), `enum`, `format` (only `date-time`) and `pattern`. The
/// amendment must equal it exactly, so an extra field or a changed value fails.
fn amended_parameter(declared: &Value) -> Value {
    let schema = &declared["schema"];
    let repeated = schema["type"] == "array";
    let scalar = if repeated { &schema["items"] } else { schema };
    let mut expected = json!({
        "name": declared["name"],
        "location": declared["in"],
        "required": declared["required"].as_bool().unwrap(),
        "type": scalar["type"],
    });
    let fields = expected.as_object_mut().unwrap();
    if repeated {
        fields.insert("repeated".into(), json!(true));
    }
    if scalar.get("enum").is_some() {
        fields.insert("enum".into(), scalar["enum"].clone());
    }
    if scalar["format"] == "date-time" {
        fields.insert("format".into(), json!("date-time"));
    }
    if scalar.get("pattern").is_some() {
        fields.insert("pattern".into(), scalar["pattern"].clone());
    }
    expected
}

/// The comparison above fails on any field the document does not say: a
/// `repeated`, a `pattern`, a `format`, a changed `type` or `enum`, a changed
/// requirement or location, or a field the amendment format does not carry.
#[test]
fn the_amendment_comparison_fails_on_any_field_the_document_does_not_say() {
    let document: Value =
        serde_json::from_slice(&fs::read(root().join(UPSTREAM).join(OWNERS)).unwrap()).unwrap();
    let declared = &document["paths"]["/crm/owners/2026-09/{ownerId}"]["get"]["parameters"][2];
    assert_eq!(declared["name"], "idProperty");
    let faithful = amended_parameter(declared);
    assert_eq!(
        faithful,
        json!({"name": "idProperty", "location": "query", "required": false,
               "type": "string", "enum": ["id", "userId"]})
    );
    for (field, value) in [
        ("repeated", json!(true)),
        ("pattern", json!("^[a-z]+$")),
        ("format", json!("date-time")),
        ("type", json!("integer")),
        ("enum", json!(["id"])),
        ("required", json!(true)),
        ("location", json!("path")),
        ("example", json!("id")),
    ] {
        let mut changed = faithful.clone();
        changed[field] = value;
        assert_ne!(changed, amended_parameter(declared), "`{field}`");
    }
    let array = json!({"name": "ids", "in": "query", "required": false,
                       "schema": {"type": "array", "items": {"type": "integer"}}});
    assert_eq!(
        amended_parameter(&array),
        json!({"name": "ids", "location": "query", "required": false,
               "type": "integer", "repeated": true})
    );
}

/// The bundle has one source, so the owner reads enter it as cited amendments
/// bound to the CRM Objects digest. Each must say what the pinned Owners
/// document says: operation id, path, and every parameter exactly
/// ([`amended_parameter`]). The document declares exactly these two reads.
#[test]
fn the_owner_amendments_say_what_the_pinned_owners_document_says() {
    let upstream = root().join(UPSTREAM);
    let document: Value =
        serde_json::from_slice(&fs::read(upstream.join(OWNERS)).unwrap()).unwrap();
    assert_eq!(document["openapi"], "3.0.1");
    assert_eq!(document["info"]["version"], "2026-09");
    assert_eq!(
        document["servers"],
        json!([{"url": "https://api.hubapi.com"}])
    );
    let paths = document["paths"].as_object().unwrap();
    assert_eq!(paths.len(), 2);
    for item in paths.values() {
        assert_eq!(
            item.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["get"]
        );
    }
    let amendments: Value =
        serde_json::from_slice(&fs::read(upstream.join(AMENDMENTS)).unwrap()).unwrap();
    assert_eq!(amendments["format"], "connectors-source-amendments/1");
    let source = hex::encode(Sha256::digest(fs::read(upstream.join(SOURCE)).unwrap()));
    assert_eq!(amendments["source_sha256"], source.as_str());
    let added = amendments["amendments"].as_array().unwrap();
    assert_eq!(added.len(), 2);
    for amendment in added {
        let added = &amendment["add_operation"];
        assert_eq!(added["method"], "get");
        let path = added["path"].as_str().unwrap();
        let operation = &document["paths"][path]["get"];
        assert_eq!(
            amendment["operation_id"], operation["operationId"],
            "`{path}`"
        );
        let cite = amendment["cite"].as_str().unwrap();
        assert!(cite.starts_with("https://raw.githubusercontent.com/HubSpot/HubSpot-public-api-spec-collection/55d9bcaaf1c208516f11a33aa43fb6bba7aac823/"), "`{cite}`");
        let declared = operation["parameters"].as_array().unwrap();
        let amended = added["parameters"].as_array().unwrap();
        assert_eq!(amended.len(), declared.len(), "`{path}` parameters");
        for parameter in declared {
            let name = parameter["name"].as_str().unwrap();
            let found = amended
                .iter()
                .find(|p| p["name"] == name)
                .unwrap_or_else(|| panic!("`{path}` lacks `{name}`"));
            assert_eq!(
                found,
                &amended_parameter(parameter),
                "`{path}` `{name}` differs from the document"
            );
        }
        // Both reads need `crm.objects.owners.read` (or `automation`).
        assert!(
            operation["security"]
                .to_string()
                .contains("crm.objects.owners.read")
        );
    }
}

/// The owners list pages by `paging.next.after`, as the CRM list does.
#[test]
fn the_owner_paging_cursor_is_in_the_pinned_owners_schema() {
    let document: Value =
        serde_json::from_slice(&fs::read(root().join(UPSTREAM).join(OWNERS)).unwrap()).unwrap();
    let schemas = &document["components"]["schemas"];
    let list = &schemas["CollectionResponsePublicOwnerForwardPaging"];
    assert_eq!(list["required"], json!(["results"]));
    assert_eq!(
        list["properties"]["paging"]["$ref"],
        "#/components/schemas/ForwardPaging"
    );
    assert_eq!(
        schemas["ForwardPaging"]["properties"]["next"]["$ref"],
        "#/components/schemas/NextPage"
    );
    assert_eq!(schemas["NextPage"]["properties"]["after"]["type"], "string");
    assert_eq!(schemas["NextPage"]["required"], json!(["after"]));
}

/// The list's end condition is `paging.next.after`, and the pinned document
/// says so in the list schema and in the `after` parameter.
#[test]
fn the_paging_cursor_is_in_the_pinned_list_schema() {
    let document = pinned();
    let resolve = |schema: &Value| -> Value {
        match schema.get("$ref").and_then(Value::as_str) {
            Some(reference) => document
                .pointer(reference.trim_start_matches('#'))
                .unwrap()
                .clone(),
            None => schema.clone(),
        }
    };
    let get = &document["paths"]["/crm/objects/2026-09/{objectType}"]["get"];
    let parameters = get["parameters"].as_array().unwrap();
    let names: Vec<&str> = parameters
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    for name in [
        "objectType",
        "after",
        "limit",
        "properties",
        "associations",
        "archived",
    ] {
        assert!(names.contains(&name), "the list read lacks `{name}`");
    }
    let limit = &parameters.iter().find(|p| p["name"] == "limit").unwrap()["schema"];
    // No maximum is stated, so the selection carries no bound.
    assert_eq!(limit["maximum"], Value::Null);
    assert!(shipped().iter().all(|s| s.bounds.is_empty()));
    let list = resolve(&get["responses"]["200"]["content"]["application/json"]["schema"]);
    let paging = resolve(&list["properties"]["paging"]);
    let next = resolve(&paging["properties"]["next"]);
    assert_eq!(next["properties"]["after"]["type"], "string");
    assert_eq!(next["required"], json!(["after"]));
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-hubspot.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, path, paging, end, deltas) in [
        (
            "objects.list",
            "/crm/objects/2026-09/{objectType}",
            "`after`, `limit`",
            "`paging.next` absent",
            "none",
        ),
        (
            "object.get",
            "/crm/objects/2026-09/{objectType}/{objectId}",
            "single item",
            "n/a",
            "none",
        ),
        (
            "owners.list",
            "/crm/owners/2026-09",
            "`after`, `limit`",
            "`paging.next` absent",
            "none",
        ),
        (
            "owner.get",
            "/crm/owners/2026-09/{ownerId}",
            "single item",
            "n/a",
            "none",
        ),
    ] {
        let cited = rows
            .iter()
            .any(|row| row.contains(&format!("`{id}`")) && row.contains(&format!("`GET {path}`")));
        assert!(cited, "no row cites `{id}` as `{path}`");
        let paged = rows.iter().any(|row| {
            row.contains(&format!("`{id}`"))
                && row.contains(paging)
                && row.contains(end)
                && row.contains(deltas)
        });
        assert!(paged, "no row states the paging of `{id}`");
    }
}

/// The `auth` profile the `hubspot` connector declares
/// (`providers/hubspot/connector.json`), which the guide's instance uses.
fn documented_auth() -> Value {
    let declaration: Value =
        serde_json::from_slice(&fs::read(root().join("providers/hubspot/connector.json")).unwrap())
            .unwrap();
    declaration["profiles"][0].clone()
}

#[test]
fn the_documented_profile_is_a_bearer_token_with_the_portal_as_identity() {
    let auth = documented_auth();
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], Value::Null);
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["identity"]["path"], "account-info/2026-09/details");
    assert_eq!(auth["identity"]["kind"], "hubspot.portal");
    assert_eq!(auth["identity"]["subject_pointer"], "/portalId");
    assert_eq!(auth["scopes"], Value::Null);
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

fn record(id: u64) -> Value {
    json!({"id": id.to_string(),
           "properties": {"email": format!("person{id}@example.test"),
                          "firstname": format!("Person {id}"),
                          "hs_object_id": id.to_string(),
                          "lastmodifieddate": "2026-09-20T10:00:00.000Z"},
           "createdAt": "2026-08-01T08:00:00.000Z",
           "updatedAt": "2026-09-20T10:00:00.000Z",
           "archived": false})
}

/// A deal or a company: money or terms, and the owner it carries.
fn crm_record(id: u64, owner: &str, properties: Value) -> Value {
    let mut all = json!({"hs_object_id": id.to_string(), "hubspot_owner_id": owner});
    all.as_object_mut()
        .unwrap()
        .extend(properties.as_object().unwrap().clone());
    json!({"id": id.to_string(), "properties": all,
           "createdAt": "2026-08-01T08:00:00.000Z",
           "updatedAt": "2026-09-20T10:00:00.000Z",
           "archived": false})
}

fn owner(id: u64) -> Value {
    json!({"id": id.to_string(), "email": format!("owner{id}@example.test"),
           "firstName": "Owner", "lastName": id.to_string(), "userId": id + 7000,
           "type": "PERSON", "archived": false,
           "createdAt": "2026-01-05T09:00:00.000Z", "updatedAt": "2026-09-01T09:00:00.000Z",
           "teams": [{"id": "11", "name": "Customer Success", "primary": true}]})
}

/// The recorded bodies the fixture serves, keyed by route and paging position.
/// `None` for anything else, which the fixture answers 404.
fn page(path: &str) -> Option<Value> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    Some(match route {
        "/account-info/2026-09/details" => json!({
            "portalId": 146_000_001u64, "accountType": "STANDARD", "timeZone": "Europe/Berlin",
            "companyCurrency": "EUR", "additionalCurrencies": [], "utcOffset": "+02:00",
            "utcOffsetMilliseconds": 7_200_000, "uiDomain": "app-eu1.hubspot.com",
            "dataHostingLocation": "eu1"}),
        // The last page carries no `paging`.
        "/crm/objects/2026-09/contacts" if has("after=fixture-after-2") => {
            json!({"results": [record(103)]})
        }
        "/crm/objects/2026-09/contacts" => json!({
            "results": [record(101), record(102)],
            "paging": {"next": {"after": "fixture-after-2",
                                "link": "https://api.hubapi.com/crm/objects/2026-09/contacts?after=fixture-after-2"}}}),
        "/crm/objects/2026-09/contacts/101" => record(101),
        "/crm/objects/2026-09/deals" => json!({"results": [
            crm_record(201, "501", json!({"dealname": "Annual plan", "amount": "12000.00",
                                          "dealstage": "closedwon", "closedate": "2026-09-15T00:00:00.000Z"}))]}),
        "/crm/objects/2026-09/deals/201" => crm_record(
            201,
            "501",
            json!({"dealname": "Annual plan", "amount": "12000.00", "dealstage": "closedwon"}),
        ),
        "/crm/objects/2026-09/companies" => json!({"results": [
            crm_record(301, "502", json!({"name": "Example Industries", "domain": "example.test"}))]}),
        "/crm/objects/2026-09/companies/301" => crm_record(
            301,
            "502",
            json!({"name": "Example Industries", "domain": "example.test"}),
        ),
        "/crm/objects/2026-09/line_items" => json!({"results": [
            crm_record(401, "501", json!({"name": "Seat licence", "quantity": "10",
                                          "price": "100.00", "amount": "1000.00",
                                          "recurringbillingfrequency": "annually"}))]}),
        "/crm/objects/2026-09/line_items/401" => crm_record(
            401,
            "501",
            json!({"name": "Seat licence", "quantity": "10", "price": "100.00"}),
        ),
        // The owner's last page carries no `paging`.
        "/crm/owners/2026-09" if has("after=fixture-owner-after-2") => {
            json!({"results": [owner(503)]})
        }
        "/crm/owners/2026-09" => json!({
            "results": [owner(501), owner(502)],
            "paging": {"next": {"after": "fixture-owner-after-2",
                                "link": "https://api.hubapi.com/crm/owners/2026-09?after=fixture-owner-after-2"}}}),
        "/crm/owners/2026-09/501" => owner(501),
        "/crm/owners/2026-09/502" => owner(502),
        _ => return None,
    })
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}
impl Provider {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.join("ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let requests: Requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
                        assert!(header.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8(header).unwrap();
                    let path = request
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or_default()
                        .to_owned();
                    let authorization = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    // Only the fictional header is accepted; raw headers are
                    // compared, never printed.
                    let (status, body) = if authorization.as_deref() != Some(HEADER) {
                        (401, json!({"message": "fixture refusal"}))
                    } else {
                        match page(&path) {
                            Some(body) => (200, body),
                            None => (404, json!({"message": "no fixture"})),
                        }
                    };
                    observed.lock().unwrap().push((path, authorization));
                    let body = serde_json::to_vec(&body).unwrap();
                    let header = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "fixture-hubspot",
                "provider": "hubspot",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}", address.port()),
                "ca_file": ca,
                "auth": documented_auth(),
                "operations_file": root_path("providers/hubspot/operations.json"),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            requests,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "bootstrap inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-hubspot".into(),
            adapter_id: "catalog".into(),
            configuration_revision: bootstrap.configuration_revision,
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec![
                    "--local-config".into(),
                    self.config.to_str().unwrap().into(),
                ],
            },
        }
    }
    fn requests(&self) -> Vec<(String, Option<String>)> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn invoke(child: &mut Child, operation: &str, input: Value) -> Value {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
    serde_json::from_slice(&output).unwrap()
}

#[test]
fn connecting_records_the_portal_as_the_identity() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "hubspot.portal");
    assert_eq!(baseline.identity.subject, "146000001");
    assert_eq!(baseline.granted_scopes, None);
    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    // The identity probe carries no query.
    assert_eq!(requests[0].0, "/account-info/2026-09/details");
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
    // A basic credential document is not this profile's entry.
    let basic = Secret(br#"{"account":"a@example.test","token":"t"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &basic, deadline()),
        Err(Failure::InvalidInput)
    ));
    assert!(matches!(
        child.validate("atlassian.basic", &secret(), deadline()),
        Err(Failure::Unsupported)
    ));
}

/// Each read's first request: the input, and the exact request the fixture
/// must observe.
fn first_pages() -> [(&'static str, Value, &'static str); 5] {
    [
        // A deactivated owner is read only with `archived`: the document says
        // the parameter returns "only results that have been archived".
        (
            "owner.get",
            json!({"ownerId": 502, "archived": true}),
            "/crm/owners/2026-09/502?archived=true",
        ),
        (
            "owners.list",
            json!({"limit": 2, "email": "owner501@example.test"}),
            "/crm/owners/2026-09?email=owner501%40example.test&limit=2",
        ),
        (
            "owner.get",
            json!({"ownerId": 501, "idProperty": "id"}),
            "/crm/owners/2026-09/501?idProperty=id",
        ),
        (
            "objects.list",
            json!({"objectType": "contacts", "limit": 2, "properties": "email,firstname",
                   "associations": "companies"}),
            "/crm/objects/2026-09/contacts?associations=companies&limit=2&properties=email%2Cfirstname",
        ),
        (
            "object.get",
            json!({"objectType": "contacts", "objectId": "101", "properties": "email,firstname"}),
            "/crm/objects/2026-09/contacts/101?properties=email%2Cfirstname",
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_with_bearer_auth_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in first_pages() {
        let before = provider.requests().len();
        let result = invoke(&mut child, operation, input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        assert_eq!(requests[before].0, expected, "`{operation}` request");
        assert_eq!(
            requests[before].1.as_deref(),
            Some(HEADER),
            "`{operation}` authorization"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        // The engine re-serialises the body, so the recorded body is compared
        // as JSON, not as bytes.
        assert_eq!(
            Some(&result["body"]),
            page(expected).as_ref(),
            "`{operation}` body"
        );
        assert_eq!(result["provenance"]["instance"], "fixture-hubspot");
    }
}

#[test]
fn an_undeclared_parameter_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let input = json!({"objectType": "contacts", "filterGroups": "x"});
    let outcome = child.invoke(
        "objects.list",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(&input).unwrap(),
        deadline(),
    );
    assert!(matches!(outcome, Err(Failure::InvalidInput)), "{outcome:?}");
    assert!(provider.requests().is_empty());
}

/// The list walks by `paging.next.after` and ends on a page without
/// `paging.next`, read from the body alone.
#[test]
fn the_list_walks_two_pages_and_stops_without_paging_next() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = json!({"objectType": "contacts", "limit": 2});
    let mut ids = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(&mut child, "objects.list", input.clone())["body"].clone();
        pages += 1;
        for item in body["results"].as_array().unwrap() {
            ids.push(item["id"].as_str().unwrap().to_owned());
        }
        match body.pointer("/paging/next/after").and_then(Value::as_str) {
            Some(after) => input["after"] = json!(after),
            None => break,
        }
        assert!(pages < 3, "the walk did not stop");
    }
    assert_eq!(pages, 2);
    assert_eq!(ids, ["101", "102", "103"]);
    let requests: Vec<String> = provider
        .requests()
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        requests,
        [
            "/crm/objects/2026-09/contacts?limit=2",
            "/crm/objects/2026-09/contacts?after=fixture-after-2&limit=2",
        ]
    );
}

/// The owners list walks the same way: `paging.next.after` as `after`, to a
/// page without `paging.next`.
#[test]
fn the_owner_list_walks_two_pages_and_stops_without_paging_next() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut input = json!({"limit": 2});
    let mut ids = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(&mut child, "owners.list", input.clone())["body"].clone();
        pages += 1;
        for item in body["results"].as_array().unwrap() {
            ids.push(item["id"].as_str().unwrap().to_owned());
        }
        match body.pointer("/paging/next/after").and_then(Value::as_str) {
            Some(after) => input["after"] = json!(after),
            None => break,
        }
        assert!(pages < 3, "the walk did not stop");
    }
    assert_eq!(pages, 2);
    assert_eq!(ids, ["501", "502", "503"]);
    let requests: Vec<String> = provider
        .requests()
        .into_iter()
        .map(|(path, _)| path)
        .collect();
    assert_eq!(
        requests,
        [
            "/crm/owners/2026-09?limit=2",
            "/crm/owners/2026-09?after=fixture-owner-after-2&limit=2",
        ]
    );
}

/// Finance reads deals, companies and line items, and turns the
/// `hubspot_owner_id` a deal or a company carries into a name and an email,
/// through the shipped reads alone. Each request is the exact one the fixture
/// must observe, each with the bearer header, and the owner is asked for by
/// the id the record carried, never by one the test knows.
#[test]
fn deals_companies_line_items_and_the_owner_a_record_carries_are_read() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let deals = invoke(
        &mut child,
        "objects.list",
        json!({"objectType": "deals", "limit": 10,
               "properties": "dealname,amount,dealstage,closedate,hubspot_owner_id",
               "associations": "line_items,companies"}),
    );
    let deal = &deals["body"]["results"][0];
    assert_eq!(deal["properties"]["amount"], "12000.00");
    let companies = invoke(
        &mut child,
        "objects.list",
        json!({"objectType": "companies", "properties": "name,domain,hubspot_owner_id"}),
    );
    let company = &companies["body"]["results"][0];
    assert_eq!(company["properties"]["name"], "Example Industries");
    let items = invoke(
        &mut child,
        "objects.list",
        json!({"objectType": "line_items",
               "properties": "name,quantity,price,amount,recurringbillingfrequency"}),
    );
    assert_eq!(items["body"]["results"][0]["properties"]["quantity"], "10");
    let one = invoke(
        &mut child,
        "object.get",
        json!({"objectType": "line_items", "objectId": "401", "properties": "name,quantity,price"}),
    );
    assert_eq!(one["body"]["properties"]["price"], "100.00");
    let deal_get = invoke(
        &mut child,
        "object.get",
        json!({"objectType": "deals", "objectId": deal["id"], "properties": "dealname,amount,dealstage"}),
    );
    assert_eq!(deal_get["body"]["properties"]["dealstage"], "closedwon");
    let company_get = invoke(
        &mut child,
        "object.get",
        json!({"objectType": "companies", "objectId": company["id"]}),
    );
    assert_eq!(company_get["body"]["id"], company["id"]);
    let mut owners = Vec::new();
    for record in [deal, company] {
        let id = record["properties"]["hubspot_owner_id"].clone();
        let read = invoke(&mut child, "owner.get", json!({"ownerId": id}));
        assert_eq!(read["status"], 200);
        assert_eq!(read["body"]["id"], id);
        owners.push(read["body"]["email"].as_str().unwrap().to_owned());
    }
    assert_eq!(owners, ["owner501@example.test", "owner502@example.test"]);
    let requests = provider.requests();
    assert!(
        requests
            .iter()
            .all(|(_, header)| header.as_deref() == Some(HEADER))
    );
    let paths: Vec<&str> = requests.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/crm/objects/2026-09/deals?associations=line_items%2Ccompanies&limit=10&properties=dealname%2Camount%2Cdealstage%2Cclosedate%2Chubspot_owner_id",
            "/crm/objects/2026-09/companies?properties=name%2Cdomain%2Chubspot_owner_id",
            "/crm/objects/2026-09/line_items?properties=name%2Cquantity%2Cprice%2Camount%2Crecurringbillingfrequency",
            "/crm/objects/2026-09/line_items/401?properties=name%2Cquantity%2Cprice",
            "/crm/objects/2026-09/deals/201?properties=dealname%2Camount%2Cdealstage",
            "/crm/objects/2026-09/companies/301",
            "/crm/owners/2026-09/501",
            "/crm/owners/2026-09/502",
        ]
    );
}

/// The owner reads accept only what the pinned Owners document declares.
#[test]
fn an_undeclared_owner_parameter_or_idproperty_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    for (operation, input) in [
        ("owners.list", json!({"filter": "x"})),
        ("owner.get", json!({"ownerId": 501, "idProperty": "email"})),
        ("owner.get", json!({"ownerId": "owner501"})),
    ] {
        let outcome = child.invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        );
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{operation}: {outcome:?}"
        );
    }
    assert!(provider.requests().is_empty());
}

/// Adversary: an `ownerId` that is not one integer's decimal text never leaves
/// the provider, so no owner read can be steered off `/crm/owners/2026-09/{id}`
/// by a dot segment, a query, a fragment, an escape or a look-alike digit.
#[test]
fn adversary_an_owner_id_that_is_not_an_integer_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    for input in [
        json!({"ownerId": "501/../../objects/2026-09/contacts"}),
        json!({"ownerId": "../501"}),
        json!({"ownerId": ".."}),
        json!({"ownerId": "501?archived=true"}),
        json!({"ownerId": "501#x"}),
        json!({"ownerId": "501%2F.."}),
        json!({"ownerId": " 501"}),
        json!({"ownerId": "501\n"}),
        json!({"ownerId": "\u{ff15}01"}),
        json!({"ownerId": 1.5}),
        json!({"ownerId": 5.01e2}),
        json!({"ownerId": true}),
        json!({"ownerId": ""}),
        json!({"ownerId": null}),
        json!({}),
        json!({"ownerId": [501, 502]}),
        json!({"ownerId": 501, "archived": "yes"}),
        json!({"ownerId": 501, "idProperty": "ID"}),
        json!({"ownerId": 501, "idProperty": "id&archived=true"}),
    ] {
        let outcome = child.invoke(
            "owner.get",
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        );
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "{input}: {outcome:?}"
        );
    }
    assert_eq!(provider.requests(), Vec::new());
}
