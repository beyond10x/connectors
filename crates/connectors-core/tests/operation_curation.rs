use connectors_core::operation_metadata::{CURATION_FORMAT, Curation};
use serde_json::{Value, json};

fn document() -> Value {
    json!({"format":CURATION_FORMAT,"adapters":[{
        "adapter_alias":"selected", "executable_selection":"a".repeat(64),
        "bootstrap_sha256":"b".repeat(64), "operations":[{
            "operation":"list", "metadata":{
                "effects":["network"], "semantic_effects":[], "risk":"low",
                "idempotency":{"kind":"none"}, "approval":"not_required",
                "limits":{"request_bytes":65536,"result_bytes":4194304,
                    "execution_ms":20000,"provider_ms":15000,"connect_ms":5000}
            }
        }]
    }]})
}
fn parse(value: &Value) -> bool {
    Curation::parse(&serde_json::to_vec(value).unwrap()).is_ok()
}

#[test]
fn curation_declarations_roundtrip_without_creating_operation_defaults() {
    let value = document();
    let parsed = Curation::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(parsed.document()).unwrap(), value);
    assert!(parse(&json!({"format":CURATION_FORMAT,"adapters":[]})));
}

#[test]
fn curation_document_refuses_duplicates_unknown_members_and_forged_numbers() {
    let mut invalid = vec![];
    for pointer in ["", "/adapters/0", "/adapters/0/operations/0"] {
        let mut value = document();
        value.pointer_mut(pointer).unwrap()["credential"] = json!("private");
        invalid.push(value);
    }
    for digest in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        let mut value = document();
        value["adapters"][0]["executable_selection"] = json!(digest);
        invalid.push(value);
    }
    for pointer in ["/adapters", "/adapters/0/operations"] {
        let mut value = document();
        let entries = value.pointer_mut(pointer).unwrap().as_array_mut().unwrap();
        entries.push(entries[0].clone());
        invalid.push(value);
    }
    for value in [
        json!(0),
        json!(1.5),
        json!({"$serde_json::private::Number":"15000"}),
    ] {
        let mut bad = document();
        bad["adapters"][0]["operations"][0]["metadata"]["limits"]["provider_ms"] = value;
        invalid.push(bad);
    }
    for value in invalid {
        assert!(!parse(&value), "{value}");
    }
    let duplicate =
        serde_json::to_string(&document())
            .unwrap()
            .replacen('{', "{\"format\":\"bad\",", 1);
    assert!(Curation::parse(duplicate.as_bytes()).is_err());
}
