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

/// semantics.md: "A key holding `,` cannot be named in `--fields`: `,` separates the items".
/// The pointer `/a,b` is therefore the two items `/a` and `b`, not the key `a,b`.
#[test]
fn adversary_a_pointer_names_a_key_holding_a_comma() {
    let value = json!({"a,b": 1, "a": 2, "b": 3});
    let fields = Fields::parse("/a,b").expect("`/a,b` parses as two items");
    let shaped = fields.apply(&value);
    assert_eq!(
        (Value::Object(shaped.value), shaped.missing),
        (json!({"a": 2, "b": 3}), Vec::<String>::new()),
        "story:invoke-shapes-its-result: a key holding `,` cannot be named in --fields, so \
         `/a,b` is the two items `/a` and `b` (decided by the coordinator in correction round 1)"
    );
}

/// semantics.md: "Each item is trimmed, so a key with leading or trailing whitespace cannot be
/// named in `--fields`". The pointer `/a ` is trimmed to `/a` and names the key `a`.
#[test]
fn adversary_a_pointer_names_a_key_ending_in_whitespace() {
    let value = json!({"a ": 1, "a": 2});
    let fields = Fields::parse("/a ").expect("`/a ` parses");
    let shaped = fields.apply(&value);
    assert_eq!(
        (Value::Object(shaped.value), shaped.missing),
        (json!({"a": 2}), Vec::<String>::new()),
        "story:invoke-shapes-its-result: items are trimmed, so a key with leading or trailing \
         whitespace cannot be named in --fields and `/a ` names `a` (decided by the coordinator \
         in correction round 1)"
    );
}
