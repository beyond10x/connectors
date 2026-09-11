use connectors_catalog::inventory::{Location, Operation, Parameter};
use connectors_catalog::template::{Refusal, Template};
use std::collections::BTreeMap;

fn parameter(name: &str, location: Location, required: bool) -> Parameter {
    Parameter {
        name: name.to_owned(),
        location,
        required,
    }
}

/// One path parameter, two query parameters declared out of alphabetical order,
/// and one header: enough to pin the order the binding emits.
fn operation() -> Operation {
    Operation {
        method: "get".into(),
        path: "/projects/{id}/issues".into(),
        operation_id: Some("listIssues".into()),
        parameters: vec![
            parameter("id", Location::Path, true),
            parameter("page", Location::Query, false),
            parameter("after", Location::Query, false),
            parameter("tenant", Location::Header, true),
        ],
        request_media_types: vec!["application/json".into(), "text/plain".into()],
        responses: Vec::new(),
    }
}

fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn template() -> Template {
    Template::from_operation(&operation()).expect("the fixture operation is representable")
}

#[test]
fn a_supplied_map_binds_to_an_exact_path_query_and_header_set() {
    let binding = template()
        .bind(
            &values(&[
                ("id", "42"),
                ("page", "2"),
                ("after", "9"),
                ("tenant", "acme"),
            ]),
            None,
        )
        .expect("every declared parameter has a value");
    assert_eq!(binding.path, "/projects/42/issues");
    // Declared order, not the supplied map's order: the map never reaches the output.
    assert_eq!(
        binding.query,
        vec![
            ("page".to_owned(), "2".to_owned()),
            ("after".to_owned(), "9".to_owned())
        ]
    );
    assert_eq!(binding.query_string(), "page=2&after=9");
    assert_eq!(
        binding.headers,
        vec![("tenant".to_owned(), "acme".to_owned())]
    );
    assert_eq!(binding.media_type.as_deref(), Some("application/json"));
}

#[test]
fn an_optional_parameter_without_a_value_is_simply_absent() {
    let binding = template()
        .bind(&values(&[("id", "42"), ("tenant", "acme")]), None)
        .expect("the optional query parameters may be omitted");
    assert_eq!(binding.path, "/projects/42/issues");
    assert!(binding.query.is_empty());
    assert_eq!(binding.query_string(), "");
}

#[test]
fn a_required_parameter_without_a_value_is_refused_by_name() {
    let refusal = template()
        .bind(&values(&[("id", "42")]), None)
        .expect_err("the required header has no value");
    assert_eq!(refusal, Refusal::ValueAbsent("tenant".into()));
    assert_eq!(refusal.subject(), "tenant");
    assert!(refusal.reason().contains("tenant"), "{}", refusal.reason());
}

#[test]
fn a_value_for_an_undeclared_parameter_is_refused_by_name() {
    let refusal = template()
        .bind(
            &values(&[("id", "42"), ("tenant", "acme"), ("sudo", "root")]),
            None,
        )
        .expect_err("the operation declares no `sudo`");
    assert_eq!(refusal, Refusal::ParameterUndeclared("sudo".into()));
    assert_eq!(refusal.subject(), "sudo");
    assert!(refusal.reason().contains("sudo"), "{}", refusal.reason());
}

#[test]
fn a_path_placeholder_with_no_parameter_is_refused_by_name() {
    let mut operation = operation();
    operation.path = "/projects/{id}/issues/{iid}".into();
    let refusal =
        Template::from_operation(&operation).expect_err("`iid` has no declared parameter");
    assert_eq!(refusal, Refusal::PlaceholderUndeclared("iid".into()));
    assert_eq!(refusal.subject(), "iid");
    assert!(refusal.reason().contains("iid"), "{}", refusal.reason());
}

#[test]
fn an_empty_path_value_is_refused_by_name() {
    let refusal = template()
        .bind(&values(&[("id", ""), ("tenant", "acme")]), None)
        .expect_err("an empty path value would collapse the segment");
    assert_eq!(refusal, Refusal::PathValueEmpty("id".into()));
    assert_eq!(refusal.subject(), "id");
    assert!(refusal.reason().contains("id"), "{}", refusal.reason());
}

#[test]
fn a_media_type_the_operation_does_not_offer_is_refused_by_name() {
    let refusal = template()
        .bind(
            &values(&[("id", "42"), ("tenant", "acme")]),
            Some("application/xml"),
        )
        .expect_err("the operation offers no XML");
    assert_eq!(
        refusal,
        Refusal::MediaTypeUnoffered("application/xml".into())
    );
    assert_eq!(refusal.subject(), "application/xml");
    assert!(
        refusal.reason().contains("application/xml"),
        "{}",
        refusal.reason()
    );
}

#[test]
fn an_offered_media_type_is_selected_as_asked() {
    let binding = template()
        .bind(
            &values(&[("id", "42"), ("tenant", "acme")]),
            Some("text/plain"),
        )
        .expect("text/plain is offered");
    assert_eq!(binding.media_type.as_deref(), Some("text/plain"));
}

#[test]
fn an_operation_offering_no_request_media_type_binds_without_one() {
    let mut operation = operation();
    operation.request_media_types.clear();
    let binding = Template::from_operation(&operation)
        .expect("representable")
        .bind(&values(&[("id", "42"), ("tenant", "acme")]), None)
        .expect("a GET need not carry a body");
    assert!(binding.media_type.is_none());
}

#[test]
fn a_cookie_parameter_is_declared_unsupported_rather_than_dropped() {
    let mut operation = operation();
    operation
        .parameters
        .push(parameter("session", Location::Cookie, false));
    let refusal = Template::from_operation(&operation).expect_err("cookies are not carried");
    assert_eq!(refusal, Refusal::CookieUnsupported("session".into()));
    assert_eq!(refusal.subject(), "session");
    assert!(refusal.reason().contains("session"), "{}", refusal.reason());
}

#[test]
fn a_slash_or_a_space_is_encoded_rather_than_changing_the_path() {
    let binding = template()
        .bind(
            &values(&[
                ("id", "feature/new thing"),
                ("page", "a b&c=d"),
                ("tenant", "acme"),
            ]),
            None,
        )
        .expect("values encode rather than refuse");
    assert_eq!(binding.path, "/projects/feature%2Fnew%20thing/issues");
    // The same three segments the template declares, whatever the value held.
    assert_eq!(binding.path.split('/').count(), 4);
    assert_eq!(binding.query_string(), "page=a%20b%26c%3Dd");
}

#[test]
fn the_same_inputs_bind_byte_identically_twice() {
    let supplied = values(&[
        ("id", "42"),
        ("page", "2"),
        ("after", "9"),
        ("tenant", "acme"),
    ]);
    let first = serde_json::to_string(&template().bind(&supplied, None).expect("binds"))
        .expect("serialisable");
    let second = serde_json::to_string(&template().bind(&supplied, None).expect("binds"))
        .expect("serialisable");
    assert_eq!(first, second);
    // The bytes themselves, so a reordering or a renaming has to fail here rather
    // than pass by agreeing with itself.
    assert_eq!(
        first,
        r#"{"path":"/projects/42/issues","query":[["page","2"],["after","9"]],"headers":[["tenant","acme"]],"media_type":"application/json"}"#
    );
}

#[test]
fn a_dot_segment_path_value_is_refused_by_name() {
    let refusal = template()
        .bind(&values(&[("id", ".."), ("tenant", "acme")]), None)
        .expect_err("`..` resolves the segment away rather than filling it");
    assert_eq!(refusal, Refusal::PathValueDotSegment("id".into()));
    assert_eq!(refusal.subject(), "id");
    assert!(refusal.reason().contains("id"), "{}", refusal.reason());
}

#[test]
fn a_header_value_holding_a_line_break_is_refused_by_name() {
    let refusal = template()
        .bind(
            &values(&[("id", "42"), ("tenant", "acme\r\nX-Admin: true")]),
            None,
        )
        .expect_err("a line break in a header value starts a second header");
    assert_eq!(refusal, Refusal::HeaderUnsafe("tenant".into()));
    assert_eq!(refusal.subject(), "tenant");
    assert!(refusal.reason().contains("tenant"), "{}", refusal.reason());
}

#[test]
fn a_header_name_the_document_wrote_with_a_control_byte_is_refused_by_name() {
    let mut operation = operation();
    operation.parameters.push(parameter(
        "X-Trace\r\nX-Admin: true",
        Location::Header,
        false,
    ));
    let refusal = Template::from_operation(&operation).expect_err("the name ends the header line");
    assert_eq!(
        refusal,
        Refusal::HeaderUnsafe("X-Trace\r\nX-Admin: true".into())
    );
}

#[test]
fn a_request_media_type_holding_a_control_byte_is_refused_by_name() {
    let mut operation = operation();
    operation.request_media_types = vec!["application/json\r\nX-Admin: true".into()];
    let refusal =
        Template::from_operation(&operation).expect_err("the media type ends the header line");
    assert_eq!(
        refusal,
        Refusal::MediaTypeUnsafe("application/json\r\nX-Admin: true".into())
    );
    assert_eq!(refusal.subject(), "application/json\r\nX-Admin: true");
}

#[test]
fn a_declared_path_parameter_with_no_placeholder_is_refused_by_name() {
    let mut operation = operation();
    operation
        .parameters
        .push(parameter("scope", Location::Path, true));
    let refusal =
        Template::from_operation(&operation).expect_err("`scope` templates no part of the path");
    assert_eq!(refusal, Refusal::PathParameterUnplaced("scope".into()));
    assert_eq!(refusal.subject(), "scope");
    assert!(refusal.reason().contains("scope"), "{}", refusal.reason());
}

#[test]
fn an_unterminated_placeholder_refuses_as_a_malformed_path() {
    let mut operation = operation();
    operation.path = "/projects/{id".into();
    let refusal = Template::from_operation(&operation).expect_err("the path is not bindable");
    assert_eq!(refusal, Refusal::PathMalformed("/projects/{id".into()));
    assert_eq!(refusal.subject(), "/projects/{id");
    assert!(
        refusal.reason().contains("/projects/{id"),
        "{}",
        refusal.reason()
    );
}

#[test]
fn a_name_in_two_locations_is_reached_by_qualifying_the_key() {
    let mut operation = operation();
    operation
        .parameters
        .push(parameter("id", Location::Query, false));
    let binding = Template::from_operation(&operation)
        .expect("two locations are two parameters, and both are representable")
        .bind(
            &values(&[("id", "42"), ("query:id", "7"), ("tenant", "acme")]),
            None,
        )
        .expect("the bare key fills the path, the qualified one the query");
    assert_eq!(binding.path, "/projects/42/issues");
    assert_eq!(binding.query, vec![("id".to_owned(), "7".to_owned())]);
}

#[test]
fn a_parameter_declared_twice_in_one_location_binds_once() {
    let mut operation = operation();
    // Whatever built this operation, only the last declaration of a (name,
    // location) is in force; the earlier one is not emitted beside it.
    operation
        .parameters
        .push(parameter("page", Location::Query, true));
    operation
        .parameters
        .push(parameter("page", Location::Query, false));
    let binding = Template::from_operation(&operation)
        .expect("representable")
        .bind(&values(&[("id", "42"), ("tenant", "acme")]), None)
        .expect("the last declaration makes `page` optional");
    assert!(binding.query.is_empty(), "{:?}", binding.query);
}

/// Two parameters of one name, so the bare key can only reach the first by rank.
fn twinned() -> Operation {
    Operation {
        method: "get".into(),
        path: "/things".into(),
        operation_id: Some("listThings".into()),
        parameters: vec![
            parameter("trace", Location::Query, false),
            parameter("trace", Location::Header, true),
        ],
        request_media_types: Vec::new(),
        responses: Vec::new(),
    }
}

#[test]
fn a_required_parameter_a_bare_key_cannot_reach_is_named_by_its_qualified_key() {
    let refusal = Template::from_operation(&twinned())
        .expect("representable")
        .bind(&values(&[("trace", "abc")]), None)
        .expect_err("the bare key reaches the query twin, not the required header");
    assert_eq!(refusal, Refusal::ValueAbsent("header:trace".into()));
    // The key the caller must supply, not one already supplied.
    assert_eq!(refusal.subject(), "header:trace");
}

#[test]
fn a_qualified_key_claims_its_parameter_and_the_bare_key_falls_to_the_next() {
    let binding = Template::from_operation(&twinned())
        .expect("representable")
        .bind(
            &values(&[("trace", "bare"), ("query:trace", "qualified")]),
            None,
        )
        .expect("two keys, two parameters");
    assert_eq!(
        binding.query,
        vec![("trace".to_owned(), "qualified".to_owned())]
    );
    assert_eq!(
        binding.headers,
        vec![("trace".to_owned(), "bare".to_owned())]
    );
}

#[test]
fn a_key_no_parameter_is_left_to_read_is_refused_by_name() {
    let refusal = Template::from_operation(&twinned())
        .expect("representable")
        .bind(
            &values(&[
                ("trace", "bare"),
                ("query:trace", "q"),
                ("header:trace", "h"),
            ]),
            None,
        )
        .expect_err("both parameters are claimed, so the bare key reaches nothing");
    assert_eq!(refusal, Refusal::KeyUnread("trace".into()));
    assert_eq!(refusal.subject(), "trace");
    assert!(refusal.reason().contains("trace"), "{}", refusal.reason());
}

#[test]
fn a_parameter_named_like_a_qualified_key_takes_that_key_for_itself() {
    let mut operation = twinned();
    operation.parameters = vec![
        parameter("trace", Location::Query, false),
        parameter("query:trace", Location::Query, false),
    ];
    let binding = Template::from_operation(&operation)
        .expect("representable")
        .bind(&values(&[("query:trace", "V")]), None)
        .expect("the key is the second parameter's own name");
    // A parameter's own name always wins over another's qualified spelling, so
    // one value reaches exactly one parameter. A colon is legal in a query, so
    // the name is carried as written.
    assert_eq!(
        binding.query,
        vec![("query:trace".to_owned(), "V".to_owned())]
    );
}

#[test]
fn a_parameter_no_key_can_reach_is_refused_at_construction() {
    let mut operation = twinned();
    operation
        .parameters
        .push(parameter("header:trace", Location::Query, false));
    let refusal = Template::from_operation(&operation)
        .expect_err("the header twin's qualified key is another parameter's own name");
    assert_eq!(
        refusal,
        Refusal::ParameterUnreachable("header:trace".into())
    );
    assert!(
        refusal.reason().contains("header:trace"),
        "{}",
        refusal.reason()
    );
}

#[test]
fn a_document_header_name_that_is_not_a_token_is_refused_by_name() {
    for name in [
        "X-Trace: injected",
        "X Trace",
        "",
        "X-Trace,Other",
        "@X-Trace",
    ] {
        let mut operation = operation();
        operation
            .parameters
            .push(parameter(name, Location::Header, false));
        let Err(refusal) = Template::from_operation(&operation) else {
            panic!("`{name}` is not an HTTP token, yet it is accepted");
        };
        assert_eq!(refusal, Refusal::HeaderUnsafe(name.to_owned()));
    }
}

#[test]
fn a_request_media_type_that_is_not_a_type_and_subtype_is_refused_by_name() {
    for media_type in ["application", "application/json, text/plain", "/json", "x/"] {
        let mut operation = operation();
        operation.request_media_types = vec![media_type.to_owned()];
        let Err(refusal) = Template::from_operation(&operation) else {
            panic!("`{media_type}` is not a media type, yet it is accepted");
        };
        assert_eq!(refusal, Refusal::MediaTypeUnsafe(media_type.to_owned()));
    }
}

#[test]
fn the_media_types_a_document_really_writes_are_accepted() {
    for media_type in [
        "application/json",
        "*/*",
        "application/vnd.api+json",
        "multipart/form-data; boundary=abc",
    ] {
        let mut operation = operation();
        operation.request_media_types = vec![media_type.to_owned()];
        if let Err(refusal) = Template::from_operation(&operation) {
            panic!("`{media_type}` refused: {}", refusal.reason());
        }
    }
}
