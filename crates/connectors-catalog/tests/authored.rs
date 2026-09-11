use connectors_catalog::authored::{DOCUMENT_LIMIT, Refusal, read, read_file};
use connectors_catalog::inventory::{Location, METHODS, Operation, Parameter, extract};
use connectors_catalog::template::{self, Template};
use connectors_core::ErrorCode;
use std::collections::BTreeMap;

/// The authored file the acceptance case is about: one path parameter, one query
/// parameter, one offered request media type.
const AUTHORED: &str = r#"
provider = "acme"
auth_profile = "acme.token"

[[action]]
name = "listIssues"
method = "get"
path = "/projects/{id}/issues"
request_media_types = ["application/json"]

[[action.parameter]]
name = "id"
location = "path"
required = true

[[action.parameter]]
name = "page"
location = "query"
"#;

/// The same action as an OpenAPI document, read through the ingest path's own
/// inventory. Hand-writing the equivalent operation would only compare the
/// authored reader against this test's idea of the inventory; extracting it
/// compares the two readers.
fn imported() -> Operation {
    let document = serde_json::json!({
        "openapi": "3.1.0",
        "info": {"title": "Acme", "version": "1"},
        "paths": {
            "/projects/{id}/issues": {
                "get": {
                    "operationId": "listIssues",
                    "parameters": [
                        {"name": "id", "in": "path", "required": true},
                        {"name": "page", "in": "query"}
                    ],
                    "requestBody": {"content": {"application/json": {}}}
                }
            }
        }
    });
    let inventory = extract(&document);
    assert_eq!(inventory.coverage(), (1, 0));
    inventory.operations[0].clone()
}

/// The same action written the way the OpenAPI inventory emits it.
fn equivalent() -> Operation {
    Operation {
        method: "get".into(),
        path: "/projects/{id}/issues".into(),
        operation_id: Some("listIssues".into()),
        parameters: vec![
            Parameter {
                name: "id".into(),
                location: Location::Path,
                required: true,
            },
            Parameter {
                name: "page".into(),
                location: Location::Query,
                required: false,
            },
        ],
        request_media_types: vec!["application/json".into()],
        responses: Vec::new(),
    }
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn refusal(text: &str) -> Refusal {
    read(text.as_bytes()).expect_err("this document must be refused")
}

#[test]
fn an_authored_action_binds_exactly_as_the_equivalent_operation() {
    let document = read(AUTHORED.as_bytes()).expect("the fixture document is representable");
    assert_eq!(document.provider, "acme");
    assert_eq!(document.actions.len(), 1);
    let action = &document.actions[0];
    assert_eq!(action.name, "listIssues");

    // The authored path is not a second engine: it produces the inventory's own
    // operation, and the template is built from that one place.
    assert_eq!(action.operation, equivalent());
    assert_eq!(action.operation, imported());

    let supplied = values(&[("id", "42"), ("page", "2")]);
    let authored = action
        .template
        .bind(&supplied, None)
        .expect("every declared parameter has a value");
    let from_openapi = Template::from_operation(&imported())
        .expect("the imported operation is representable")
        .bind(&supplied, None)
        .expect("every declared parameter has a value");
    assert_eq!(authored, from_openapi);
    assert_eq!(authored.path, "/projects/42/issues");
    assert_eq!(authored.query_string(), "page=2");
    assert_eq!(authored.media_type.as_deref(), Some("application/json"));
}

#[test]
fn an_action_names_the_auth_profile_it_authenticates_under() {
    let document = read(AUTHORED.as_bytes()).expect("the fixture document is representable");
    assert_eq!(
        document.actions[0].auth_profile.as_deref(),
        Some("acme.token")
    );

    let own = read(
        br#"
provider = "acme"
auth_profile = "acme.token"

[[action]]
name = "listIssues"
method = "get"
path = "/issues"
auth_profile = "acme.admin"
"#,
    )
    .expect("an action may name its own profile");
    assert_eq!(own.actions[0].auth_profile.as_deref(), Some("acme.admin"));
}

#[test]
fn an_unknown_method_is_refused_by_the_value_as_written() {
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "fetch"
path = "/issues"
"#,
    );
    assert_eq!(refused, Refusal::MethodUnknown("fetch".into()));
    assert_eq!(refused.subject(), "fetch");
    assert!(refused.reason().contains("fetch"), "{}", refused.reason());
}

#[test]
fn a_method_is_read_regardless_of_the_case_it_is_written_in() {
    let document = read(
        br#"
provider = "acme"
[[action]]
name = "listIssues"
method = "GET"
path = "/issues"
"#,
    )
    .expect("an uppercase method names the same method");
    assert_eq!(document.actions[0].operation.method, "get");
}

#[test]
fn a_path_that_does_not_begin_with_a_slash_is_refused_by_its_path() {
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "issues"
"#,
    );
    assert_eq!(refused, Refusal::PathRelative("issues".into()));
    assert!(refused.reason().contains("issues"), "{}", refused.reason());
}

#[test]
fn a_parameter_location_the_model_does_not_carry_is_refused_by_its_value() {
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "body"
location = "formData"
"#,
    );
    assert_eq!(refused, Refusal::LocationUnsupported("formData".into()));
    // Unsupported, not invalid input: the document is well formed and says
    // something this model does not carry.
    assert_eq!(
        connectors_core::Error::from(refused).code,
        ErrorCode::Unsupported
    );
}

#[test]
fn two_actions_with_one_name_are_refused_rather_than_the_last_kept() {
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"

[[action]]
name = "listIssues"
method = "post"
path = "/issues"
"#,
    );
    assert_eq!(refused, Refusal::ActionDuplicate("listIssues".into()));
    assert!(
        refused.reason().contains("listIssues"),
        "{}",
        refused.reason()
    );
}

#[test]
fn a_missing_required_field_is_refused_by_its_dotted_name() {
    for (document, field) in [
        (
            r#"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
"#,
            "provider",
        ),
        (
            r#"
provider = "acme"
"#,
            "action",
        ),
        (
            r#"
provider = "acme"
[[action]]
method = "get"
path = "/issues"
"#,
            "action.name",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
path = "/issues"
"#,
            "action.method",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
"#,
            "action.path",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
location = "query"
"#,
            "action.parameter.name",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "page"
"#,
            "action.parameter.location",
        ),
    ] {
        assert_eq!(refusal(document), Refusal::FieldAbsent(field.into()));
    }
}

#[test]
fn an_unknown_field_is_refused_rather_than_ignored() {
    for (document, field) in [
        (
            r#"
provider = "acme"
providers = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
"#,
            "providers",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
response_media_types = ["application/json"]
"#,
            "action.response_media_types",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "page"
location = "query"
optional = true
"#,
            "action.parameter.optional",
        ),
    ] {
        assert_eq!(refusal(document), Refusal::FieldUnknown(field.into()));
    }
}

#[test]
fn a_field_of_the_wrong_type_is_refused_by_its_dotted_name() {
    for (document, field) in [
        (
            r#"
provider = 7
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
"#,
            "provider",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
request_media_types = "application/json"
"#,
            "action.request_media_types",
        ),
        (
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "page"
location = "query"
required = "yes"
"#,
            "action.parameter.required",
        ),
    ] {
        assert_eq!(refusal(document), Refusal::FieldMalformed(field.into()));
    }
}

#[test]
fn a_document_over_the_bounded_size_is_refused_rather_than_read_in_part() {
    let mut oversized = AUTHORED.to_owned();
    while oversized.len() <= DOCUMENT_LIMIT {
        oversized.push_str("\n# padding to pass the authored limit\n");
    }
    let refused = read(oversized.as_bytes()).expect_err("an oversized document must be refused");
    assert_eq!(refused, Refusal::TooLarge);
    assert_eq!(
        connectors_core::Error::from(refused).code,
        ErrorCode::Capacity
    );
}

#[test]
fn a_document_that_is_not_toml_is_refused_as_malformed() {
    let refused = refusal("provider = \n[[action]\n");
    assert!(
        matches!(refused, Refusal::Malformed(_)),
        "unexpected refusal: {refused:?}"
    );
}

#[test]
fn a_template_refusal_is_reported_as_itself_rather_than_rewritten() {
    // A cookie parameter is representable in the inventory and refused by the
    // template pass. The authored path must report that refusal unchanged.
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "session"
location = "cookie"
"#,
    );
    let inner = template::Refusal::CookieUnsupported("session".into());
    assert_eq!(refused, Refusal::Template(inner.clone()));
    assert_eq!(refused.subject(), "session");
    assert_eq!(refused.reason(), inner.reason());
    let error = connectors_core::Error::from(refused);
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert_eq!(error.message, connectors_core::Error::from(inner).message);
}

#[test]
fn a_placeholder_with_no_declared_parameter_refuses_as_the_template_says() {
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "showIssue"
method = "get"
path = "/issues/{id}"
"#,
    );
    assert_eq!(
        refused,
        Refusal::Template(template::Refusal::PlaceholderUndeclared("id".into()))
    );
}

#[test]
fn a_file_reads_as_its_bytes_do_and_an_absent_one_is_not_found() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("acme.actions.toml");
    std::fs::write(&path, AUTHORED).expect("write the authored file");
    let from_file = read_file(&path).expect("the fixture document is representable");
    assert_eq!(
        from_file,
        read(AUTHORED.as_bytes()).expect("the fixture document is representable")
    );

    let absent = read_file(&directory.path().join("absent.toml")).expect_err("must refuse");
    assert_eq!(absent.code, ErrorCode::NotFound);
}

/// Everything an operation the inventory emits satisfies, checked over the
/// operation the authored reader publishes. The two readers feed one template
/// pass, so an authored operation the inventory could never emit is the whole
/// failure this unit exists to avoid — and checking the shape catches the next
/// field as well as the two that were wrong.
fn assert_inventory_shaped(operation: &Operation) {
    assert!(
        METHODS.contains(&operation.method.as_str()),
        "`{}` is not a method the inventory emits",
        operation.method
    );
    assert!(
        operation.path.starts_with('/'),
        "`{}` is not a path the inventory emits",
        operation.path
    );
    for (index, parameter) in operation.parameters.iter().enumerate() {
        let twin = operation
            .parameters
            .iter()
            .skip(index + 1)
            .any(|other| other.name == parameter.name && other.location == parameter.location);
        assert!(
            !twin,
            "`{}` is declared twice in one operation: {:?}",
            parameter.name, operation.parameters
        );
    }
    // An imported operation's media types come out of a JSON object, which is a
    // sorted map of unique keys. An authored list that is neither does not offer
    // the same default, and `bind(_, None)` takes the first offered.
    let mut ordered = operation.request_media_types.clone();
    ordered.sort();
    ordered.dedup();
    assert_eq!(
        operation.request_media_types, ordered,
        "the offered media types are not the sorted unique set an import offers"
    );
    assert!(
        operation.responses.is_empty(),
        "an authored action describes no response: {:?}",
        operation.responses
    );
}

#[test]
fn every_published_operation_is_one_the_inventory_could_have_emitted() {
    for document in [
        AUTHORED,
        r#"
provider = "acme"
[[action]]
name = "createIssue"
method = "POST"
path = "/issues"
request_media_types = ["application/xml", "application/json"]
[[action.parameter]]
name = "trace"
location = "header"
[[action.parameter]]
name = "trace"
location = "query"
"#,
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action]]
name = "showIssue"
method = "get"
path = "/issues/{id}"
[[action.parameter]]
name = "id"
location = "path"
required = true
"#,
    ] {
        let read = read(document.as_bytes()).expect("the fixture document is representable");
        for action in &read.actions {
            assert_inventory_shaped(&action.operation);
        }
    }
}

#[test]
fn a_parameter_repeated_under_one_name_and_location_is_refused() {
    // The template's merge would keep the last and drop the `required = true`
    // this document wrote, leaving the action's operation and its template
    // disagreeing about whether a value is needed.
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "page"
location = "query"
required = true
[[action.parameter]]
name = "page"
location = "query"
"#,
    );
    assert_eq!(refused, Refusal::ParameterDuplicate("query:page".into()));
    assert!(refused.reason().contains("page"), "{}", refused.reason());
}

#[test]
fn one_name_in_two_locations_is_not_a_repeat() {
    let document = read(
        br#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "trace"
location = "query"
[[action.parameter]]
name = "trace"
location = "header"
"#,
    )
    .expect("a name may be declared once per location");
    assert_eq!(document.actions[0].operation.parameters.len(), 2);
}

#[test]
fn the_offered_media_types_are_the_set_the_equivalent_import_offers() {
    let document = read(
        br#"
provider = "acme"
[[action]]
name = "createIssue"
method = "post"
path = "/issues"
request_media_types = ["application/xml", "application/json"]
"#,
    )
    .expect("the fixture document is representable");
    let action = &document.actions[0];
    assert_eq!(
        action.operation.request_media_types,
        vec!["application/json".to_owned(), "application/xml".to_owned()]
    );

    let imported = extract(&serde_json::json!({
        "openapi": "3.1.0",
        "info": {"title": "Acme", "version": "1"},
        "paths": {"/issues": {"post": {
            "operationId": "createIssue",
            "requestBody": {"content": {"application/xml": {}, "application/json": {}}}
        }}}
    }));
    assert_eq!(action.operation, imported.operations[0]);
    // `bind(_, None)` takes the first offered: the authored order must not choose
    // a different default from the one the equivalent import chooses.
    assert_eq!(
        action
            .template
            .bind(&BTreeMap::new(), None)
            .expect("the action declares no parameter"),
        Template::from_operation(&imported.operations[0])
            .expect("the imported operation is representable")
            .bind(&BTreeMap::new(), None)
            .expect("the operation declares no parameter")
    );
}

#[test]
fn a_media_type_offered_twice_is_refused() {
    let refused = refusal(
        r#"
provider = "acme"
[[action]]
name = "createIssue"
method = "post"
path = "/issues"
request_media_types = ["application/json", "application/json"]
"#,
    );
    assert_eq!(
        refused,
        Refusal::MediaTypeDuplicate("application/json".into())
    );
}

#[test]
fn each_location_the_model_carries_reaches_the_part_of_the_request_it_names() {
    let document = read(
        br#"
provider = "acme"
[[action]]
name = "showIssue"
method = "get"
path = "/issues/{id}"
[[action.parameter]]
name = "id"
location = "path"
required = true
[[action.parameter]]
name = "page"
location = "query"
[[action.parameter]]
name = "tenant"
location = "header"
"#,
    )
    .expect("the fixture document is representable");
    let action = &document.actions[0];
    assert_eq!(
        action.operation.parameters,
        vec![
            Parameter {
                name: "id".into(),
                location: Location::Path,
                required: true
            },
            Parameter {
                name: "page".into(),
                location: Location::Query,
                required: false
            },
            Parameter {
                name: "tenant".into(),
                location: Location::Header,
                required: false
            },
        ]
    );
    // Read through the binding as well: a location read as the wrong one puts a
    // value somewhere the document never said it goes.
    let bound = action
        .template
        .bind(
            &values(&[("id", "7"), ("page", "2"), ("tenant", "acme")]),
            None,
        )
        .expect("every declared parameter has a value");
    assert_eq!(bound.path, "/issues/7");
    assert_eq!(bound.query_string(), "page=2");
    assert_eq!(
        bound.headers,
        vec![("tenant".to_owned(), "acme".to_owned())]
    );
}

#[test]
fn a_cookie_is_read_as_a_cookie_and_left_to_the_template_to_refuse() {
    // The fourth spelling, which the model carries and the template pass refuses:
    // reading it as anything else would bind a request carrying it.
    assert_eq!(
        refusal(
            r#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action.parameter]]
name = "session"
location = "cookie"
"#
        ),
        Refusal::Template(template::Refusal::CookieUnsupported("session".into()))
    );
}

#[test]
fn an_empty_action_array_is_refused_by_what_the_document_actually_did() {
    let refused = refusal(
        r#"
provider = "acme"
action = []
"#,
    );
    assert_eq!(refused, Refusal::NoAction);
    // The document declared `action`; a reason saying it must declare one names
    // a correction the author cannot make.
    assert!(
        !refused.reason().contains("must declare"),
        "{}",
        refused.reason()
    );
    assert!(
        refused.reason().contains("no action"),
        "{}",
        refused.reason()
    );
}

#[test]
fn a_document_of_exactly_the_limit_is_read_and_one_byte_more_is_not() {
    // The bound is strict: the largest document this reader accepts is exactly
    // `DOCUMENT_LIMIT` bytes, and nothing pins that but a document of that size.
    let padding = DOCUMENT_LIMIT - AUTHORED.len() - 2;
    let exact = format!("{AUTHORED}\n#{}", "x".repeat(padding));
    assert_eq!(exact.len(), DOCUMENT_LIMIT);
    assert_eq!(
        read(exact.as_bytes()).expect("a document of exactly the limit is read"),
        read(AUTHORED.as_bytes()).expect("the fixture document is representable")
    );

    let over = format!("{exact}x");
    assert_eq!(over.len(), DOCUMENT_LIMIT + 1);
    assert_eq!(
        read(over.as_bytes()).expect_err("one byte past the limit is refused"),
        Refusal::TooLarge
    );
}

#[test]
fn find_names_one_action_and_reports_the_rest_absent() {
    let document = read(
        br#"
provider = "acme"
[[action]]
name = "listIssues"
method = "get"
path = "/issues"
[[action]]
name = "createIssue"
method = "post"
path = "/issues"
"#,
    )
    .expect("the fixture document is representable");
    assert_eq!(
        document.find("createIssue").map(|action| &action.operation),
        Some(&document.actions[1].operation)
    );
    assert_eq!(
        document
            .find("listIssues")
            .map(|action| action.name.as_str()),
        Some("listIssues")
    );
    assert!(document.find("deleteIssue").is_none());
}

#[test]
fn a_path_that_exists_and_is_not_a_document_is_not_reported_as_absent() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("acme.actions.toml");
    std::fs::create_dir(&path).expect("create a directory where a file is named");
    let refused = read_file(&path).expect_err("a directory is not an authored document");
    assert_eq!(refused.code, ErrorCode::InvalidInput);
    assert!(
        refused.message.contains("acme.actions.toml"),
        "{}",
        refused.message
    );
}

#[cfg(unix)]
#[test]
fn a_file_this_reader_may_not_open_is_refused_as_forbidden() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    // The variant's own classification, which holds for every runner.
    assert_eq!(
        connectors_core::Error::from(Refusal::FileForbidden("acme.actions.toml".into())).code,
        ErrorCode::Forbidden
    );

    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("acme.actions.toml");
    std::fs::write(&path, AUTHORED).expect("write the authored file");
    // A file's owner is this process's effective user; root reads a file whose
    // mode forbids it, so only an unprivileged runner can produce the refusal.
    if std::fs::metadata(&path).expect("metadata").uid() != 0 {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000))
            .expect("forbid reading the file");
        let refused = read_file(&path).expect_err("a file this reader may not open");
        assert_eq!(refused.code, ErrorCode::Forbidden);
    }
}

#[test]
fn every_refusal_names_its_subject_in_the_reason_it_gives() {
    // A reason that does not name what it is about sends the author back to a
    // file with nothing to look for. Table-driven, so a refusal added later is
    // one row rather than a case nobody writes.
    let documents = [
        r#"providers = "acme""#,
        r#"provider = 7"#,
        r#"provider = "acme""#,
        "provider = \"acme\"\naction = []\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"fetch\"\npath = \"/i\"\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"get\"\npath = \"issues\"\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"get\"\npath = \"/i\"\n[[action.parameter]]\nname = \"p\"\nlocation = \"formData\"\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"get\"\npath = \"/i\"\n[[action]]\nname = \"a\"\nmethod = \"post\"\npath = \"/i\"\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"get\"\npath = \"/i\"\n[[action.parameter]]\nname = \"p\"\nlocation = \"query\"\n[[action.parameter]]\nname = \"p\"\nlocation = \"query\"\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"post\"\npath = \"/i\"\nrequest_media_types = [\"application/json\", \"application/json\"]\n",
        "provider = \"acme\"\n[[action]]\nname = \"a\"\nmethod = \"get\"\npath = \"/i/{id}\"\n",
        "provider = ",
    ];
    let mut seen: Vec<Refusal> = Vec::new();
    for document in documents {
        let refused = refusal(document);
        assert!(
            refused.reason().contains(refused.subject()),
            "`{}` does not name `{}`",
            refused.reason(),
            refused.subject()
        );
        seen.push(refused);
    }
    for refused in [
        Refusal::TooLarge,
        Refusal::FileAbsent("acme.actions.toml".into()),
        Refusal::FileForbidden("acme.actions.toml".into()),
        Refusal::Unreadable("acme.actions.toml".into()),
    ] {
        assert!(
            refused.reason().contains(refused.subject()),
            "`{}` does not name `{}`",
            refused.reason(),
            refused.subject()
        );
        seen.push(refused);
    }
    // One row per variant, each raised exactly once: with `ordinal` exhaustive,
    // a variant added to `Refusal` cannot reach a reader until this table grows
    // a row that produces it.
    let mut ordinals: Vec<u8> = seen.iter().map(ordinal).collect();
    ordinals.sort_unstable();
    assert_eq!(ordinals, (0..VARIANTS).collect::<Vec<u8>>());
}

/// How many variants `Refusal` has, as [`ordinal`] counts them.
const VARIANTS: u8 = 16;

/// Exhaustive by construction: a variant added to `Refusal` stops this file
/// compiling until it is given a number, and the case above then demands a
/// document that produces it.
fn ordinal(refusal: &Refusal) -> u8 {
    match refusal {
        Refusal::FileAbsent(_) => 0,
        Refusal::FileForbidden(_) => 1,
        Refusal::Unreadable(_) => 2,
        Refusal::TooLarge => 3,
        Refusal::Malformed(_) => 4,
        Refusal::FieldAbsent(_) => 5,
        Refusal::FieldUnknown(_) => 6,
        Refusal::FieldMalformed(_) => 7,
        Refusal::NoAction => 8,
        Refusal::MethodUnknown(_) => 9,
        Refusal::PathRelative(_) => 10,
        Refusal::LocationUnsupported(_) => 11,
        Refusal::ActionDuplicate(_) => 12,
        Refusal::ParameterDuplicate(_) => 13,
        Refusal::MediaTypeDuplicate(_) => 14,
        Refusal::Template(_) => 15,
    }
}
