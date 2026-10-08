//! The first v1alpha2 invoke binding (`contracts/service/compatibility.md`
//! § 2.1, § 5) against the vectors under `tests/vectors/v1alpha2/`.
use connectors_core::canonical;
use connectors_core::v1alpha2::{
    self, AuditStatus, ErrorCode, InvokeRequest, InvokeResponse, ResponseStatus,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Vectors {
    format: String,
    #[serde(rename = "type")]
    type_name: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    /// Expected verdict of the ESS-generated JSON Schema; absent for `raw`
    /// bytes, which are not a JSON value a schema can read.
    #[serde(default)]
    schema_valid: Option<bool>,
    expect: Expect,
    #[serde(default)]
    code: Option<ErrorCode>,
    #[serde(default, deserialize_with = "present")]
    request_id: Option<Option<String>>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    raw: Option<String>,
}

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

#[derive(Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
enum Expect {
    RoundTrip,
    Refuse,
}

fn load(name: &str, type_name: &str) -> Vec<Case> {
    let path = format!(
        "{}/tests/vectors/v1alpha2/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let vectors: Vectors = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(vectors.format, "connectors-service-wire-vectors/1");
    assert_eq!(vectors.type_name, type_name);
    let mut ids = BTreeSet::new();
    for case in &vectors.cases {
        assert!(
            ids.insert(case.id.clone()),
            "duplicate vector id {}",
            case.id
        );
        assert!(
            case.value.is_some() != case.raw.is_some(),
            "{}: exactly one of value/raw",
            case.id
        );
        assert_eq!(
            case.schema_valid.is_some(),
            case.value.is_some(),
            "{}: schema_valid accompanies every value and no raw bytes",
            case.id
        );
        if case.schema_valid == Some(false) {
            assert_eq!(
                case.expect,
                Expect::Refuse,
                "{}: schema-invalid but accepted",
                case.id
            );
        }
    }
    for expect in [Expect::RoundTrip, Expect::Refuse] {
        assert!(vectors.cases.iter().any(|c| c.expect == expect));
    }
    vectors.cases
}

fn bytes(case: &Case) -> Vec<u8> {
    match (&case.value, &case.raw) {
        (Some(value), None) => serde_json::to_vec(value).unwrap(),
        (None, Some(raw)) => raw.as_bytes().to_vec(),
        _ => unreachable!(),
    }
}

#[test]
fn request_vectors_round_trip_or_refuse_with_the_contract_code() {
    let cases = load(
        "invoke-request.json",
        "connectors.service_wire.InvokeRequest",
    );
    for case in &cases {
        let decoded = InvokeRequest::decode(&bytes(case));
        match case.expect {
            Expect::RoundTrip => {
                let request = decoded.unwrap_or_else(|r| panic!("{}: refused {r:?}", case.id));
                assert_eq!(request.version, v1alpha2::VERSION, "{}", case.id);
                assert!(
                    case.code.is_none() && case.request_id.is_none(),
                    "{}",
                    case.id
                );
                let written = request.encode();
                let reread: Value = serde_json::from_slice(&written).unwrap();
                assert_eq!(
                    canonical(&reread),
                    canonical(case.value.as_ref().unwrap()),
                    "{}: round trip changed the envelope",
                    case.id
                );
            }
            Expect::Refuse => {
                let refusal = match decoded {
                    Ok(_) => panic!("{}: accepted", case.id),
                    Err(refusal) => refusal,
                };
                let code = case.code.expect("request refusal names its code");
                let request_id = case.request_id.clone().expect("refusal names request_id");
                assert_eq!(refusal.error.code, code, "{}", case.id);
                assert_eq!(refusal.request_id, request_id, "{}", case.id);
                // The refusal is a valid § 5 Response that echoes no caller
                // bytes beyond a decoded request ID and records no audit.
                let response = refusal.into_response();
                assert_eq!(response.status, ResponseStatus::Error);
                assert_eq!(response.audit_status, AuditStatus::Unavailable);
                assert!(response.audit_ref.is_none() && response.mutation.is_none());
                let encoded = response.encode().unwrap();
                InvokeResponse::decode(&encoded).unwrap();
            }
        }
    }
}

#[test]
fn response_vectors_round_trip_or_refuse() {
    let cases = load(
        "invoke-response.json",
        "connectors.service_wire.InvokeResponse",
    );
    for case in &cases {
        assert!(
            case.code.is_none() && case.request_id.is_none(),
            "{}",
            case.id
        );
        let decoded = InvokeResponse::decode(&bytes(case));
        // `serde_json` alone reaches the same verdict for every JSON value:
        // the rules hold for any reader of the type, not only `decode`.
        if let Some(value) = &case.value {
            assert_eq!(
                serde_json::from_value::<InvokeResponse>(value.clone()).is_ok(),
                decoded.is_ok(),
                "{}: serde and decode disagree",
                case.id
            );
        }
        match case.expect {
            Expect::RoundTrip => {
                let response = decoded.unwrap_or_else(|e| panic!("{}: refused {e:?}", case.id));
                let reread: Value = serde_json::from_slice(&response.encode().unwrap()).unwrap();
                assert_eq!(
                    canonical(&reread),
                    canonical(case.value.as_ref().unwrap()),
                    "{}: round trip changed the envelope",
                    case.id
                );
            }
            Expect::Refuse => {
                assert!(decoded.is_err(), "{}: accepted", case.id);
            }
        }
    }
}

#[test]
fn encode_refuses_a_response_that_breaks_a_rule() {
    let valid = json!({"version":"v1alpha2","request_id":"read-1","status":"success","result":{},"audit_ref":"aud-1","audit_status":"complete"});
    let mut response: InvokeResponse = serde_json::from_value(valid).unwrap();
    response.audit_ref = None;
    assert!(response.encode().is_err());
    response.audit_status = AuditStatus::Unavailable;
    assert!(response.encode().is_ok());
    response.error = Some(v1alpha2::Error::new(ErrorCode::Internal, "m"));
    assert!(response.encode().is_err());
}

#[test]
fn the_v1alpha1_envelope_does_not_decode_as_v1alpha2() {
    let legacy = json!({"version":"v1alpha1","request_id":"one","status":"success","result":{}});
    assert!(InvokeResponse::decode(&serde_json::to_vec(&legacy).unwrap()).is_err());
}
