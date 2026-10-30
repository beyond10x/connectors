//! Adversary cases for story:invoke-shapes-its-result, driven from the documents the unit
//! wrote about itself: `contracts/cli/v1alpha1/semantics.md` "Shaping a result" and
//! `connectors.cli.InvokeShaped` in `ess/domains/cli.yaml`.
use connectors::shape::{Fields, answer};
use serde_json::{Value, json};

fn catalog(status: u64, body: Value) -> Value {
    json!({"status": status, "body": body,
           "provenance": {"instance": "forge-local", "resource": "/api/v2/tickets/{ticket_id}",
                          "observed_at_unix_ms": 1, "source_revision": null}})
}

/// `ess/domains/cli.yaml` declares `connectors.cli.InvokeShaped.records` as
/// `Optional<Integer>`, "the number of elements --each printed", and says of the members
/// `--fields` adds that no field may make its output name one of InvokeShaped's members.
/// `records` is one of them, yet `--fields records:ticket.subject` is admitted and prints the
/// subject string as `result.records`.
#[test]
fn adversary_no_field_takes_the_records_member_of_invoke_shaped() {
    let full = json!({"connector": "zendesk", "instance": "support", "operation": "ticket.show",
        "revision": "desc-1",
        "result": catalog(200, json!({"ticket": {"id": 101, "subject": "fixture ticket 101"}}))});
    for paths in ["records:ticket.subject", "records"] {
        let shaped = Fields::parse(paths)
            .ok()
            .and_then(|fields| answer(&full, Some(&fields), None));
        let records = shaped
            .as_ref()
            .and_then(|shaped| shaped.result.get("records"));
        assert!(
            records.is_none_or(Value::is_u64),
            "story:invoke-shapes-its-result: InvokeShaped.records is Optional<Integer>, yet \
             --fields {paths:?} prints {shaped:?}"
        );
    }
}

/// semantics.md: "a name holds no `.`, `[`, `]`, `,`, `:`, `/`, whitespace or control
/// character. A key holding one of them is written as an RFC 6901 pointer". The pointer for
/// the key `a,b` is `/a,b`, but the path list is split at every comma first, so it becomes the
/// two paths `/a` and `b` and the key cannot be named at all.
#[test]
fn adversary_a_pointer_names_a_key_holding_a_comma() {
    let value = json!({"a,b": 1, "a": 2, "b": 3});
    let fields = Fields::parse("/a,b").expect("the pointer `/a,b` parses");
    let shaped = fields.apply(&value);
    assert_eq!(
        (Value::Object(shaped.value), shaped.missing),
        (json!({"a,b": 1}), Vec::<String>::new()),
        "story:invoke-shapes-its-result: a key holding `,` is written as a pointer"
    );
}

/// semantics.md: a key holding whitespace is written as an RFC 6901 pointer. Each item is
/// trimmed before it is read as a pointer, so `/a ` names the key `a`, not `a `.
#[test]
fn adversary_a_pointer_names_a_key_ending_in_whitespace() {
    let value = json!({"a ": 1, "a": 2});
    let fields = Fields::parse("/a ").expect("the pointer `/a ` parses");
    let shaped = fields.apply(&value);
    assert_eq!(
        (Value::Object(shaped.value), shaped.missing),
        (json!({"a ": 1}), Vec::<String>::new()),
        "story:invoke-shapes-its-result: a key holding whitespace is written as a pointer"
    );
}
