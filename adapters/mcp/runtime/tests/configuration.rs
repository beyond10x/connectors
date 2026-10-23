use connectors_mcp::configuration::{Configuration, Error};
use serde_json::{Value, json};
fn valid() -> Value {
    json!({"format":"connectors-mcp-local/1","limits":{"frame_octets":1048576,"response_octets":33554432,"concurrent_requests":16,"request_milliseconds":120000},"exposures":[{"adapter_alias":"selected","operation_ref":"read","connection_ref":"connection","families":["tools","resources"],"enabled":true}]})
}
fn parse(value: &Value) -> Result<Configuration, Error> {
    Configuration::parse(&serde_json::to_vec(value).unwrap())
}
#[test]
fn accepts_explicit_selection_and_empty_projection_without_opening_approval_source() {
    let mut value = valid();
    value["exposures"][0]["approval_file"] =
        json!("/nonexistent-private-owner-directory/approval.json");
    let admitted = parse(&value).unwrap();
    assert_eq!(admitted.document().exposures.len(), 1);
    value["exposures"] = json!([]);
    assert!(parse(&value).is_ok());
}
#[test]
fn duplicate_and_unknown_members_do_not_gain_last_value_wins_meaning() {
    let mut bytes = serde_json::to_string(&valid()).unwrap();
    bytes.insert_str(1, "\"format\":\"invalid\",");
    assert!(matches!(
        Configuration::parse(bytes.as_bytes()),
        Err(Error::Json)
    ));
    let mut value = valid();
    value["grant"] = json!("all");
    assert!(matches!(parse(&value), Err(Error::Shape)));
    let mut value = valid();
    value["exposures"][0]["approval_file"] = Value::Null;
    assert!(matches!(parse(&value), Err(Error::Shape)));
}
#[test]
fn limits_refuse_overflow_fraction_exponent_and_private_number_token_objects() {
    for number in [
        "0",
        "-1",
        "17",
        "1.0",
        "1e0",
        "1844674407370955161701",
        r#"{"$serde_json::private::Number":"1"}"#,
    ] {
        let value = serde_json::to_string(&valid()).unwrap().replace(
            "\"concurrent_requests\":16",
            &format!("\"concurrent_requests\":{number}"),
        );
        assert!(
            matches!(Configuration::parse(value.as_bytes()), Err(Error::Limits)),
            "{number}"
        );
    }
    for (key, limit) in [
        ("frame_octets", 255),
        ("response_octets", 1023),
        ("request_milliseconds", 120001),
    ] {
        let mut value = valid();
        value["limits"][key] = json!(limit);
        assert!(matches!(parse(&value), Err(Error::Limits)));
    }
}
#[test]
fn duplicate_operation_cannot_select_another_connection_or_repeated_family() {
    let mut value = valid();
    let mut other = value["exposures"][0].clone();
    other["connection_ref"] = json!("another");
    value["exposures"].as_array_mut().unwrap().push(other);
    assert!(matches!(parse(&value), Err(Error::Exposure)));
    let mut value = valid();
    value["exposures"][0]["families"] = json!(["tools", "tools"]);
    assert!(matches!(parse(&value), Err(Error::Exposure)));
    value["exposures"][0]["families"] = json!([]);
    assert!(matches!(parse(&value), Err(Error::Exposure)));
}
#[test]
fn selectors_and_approval_paths_stay_in_the_explicit_local_binding() {
    for path in [
        "/",
        "relative/file",
        "/private/../approval",
        "/private/approval\n",
    ] {
        let mut value = valid();
        value["exposures"][0]["approval_file"] = json!(path);
        assert!(matches!(parse(&value), Err(Error::ApprovalPath)));
    }
    for field in ["adapter_alias", "operation_ref", "connection_ref"] {
        let mut value = valid();
        value["exposures"][0][field] = json!("");
        assert!(matches!(parse(&value), Err(Error::Exposure)));
    }
}
#[test]
fn byte_and_exposure_limits_are_checked_before_admission() {
    assert!(matches!(
        Configuration::parse(&vec![b' '; 1048577]),
        Err(Error::Size)
    ));
    let mut value = valid();
    value["exposures"]=json!((0..1001).map(|n|json!({"adapter_alias":"selected","operation_ref":format!("op{n}"),"connection_ref":"connection","families":["tools"],"enabled":true})).collect::<Vec<_>>());
    assert!(matches!(parse(&value), Err(Error::Exposure)));
}
