//! Attacks against the published schema and real embedded schedule requests.

use std::collections::BTreeMap;

use connector_resolve::{build_request, document::Document};
use serde_json::{json, Value};

fn request(id: &str, input: Value) -> connector_resolve::Request {
    let document = connector_resolve::document::provider("gitlab").unwrap();
    let operation = document.operation(id).unwrap();
    build_request(
        operation,
        document.base_url(&operation.service).unwrap(),
        &input,
        &BTreeMap::from([("origin".to_owned(), "https://gitlab.example".to_owned())]),
    )
    .unwrap()
}

#[test]
fn adversary_gitlab_pass1_schema_versions_and_profiles_fail_closed() {
    let schema3: Value = serde_json::from_str(include_str!(
        "../../../catalog/connector-document-v3.schema.json"
    ))
    .unwrap();
    let schema2: Value = serde_json::from_str(include_str!(
        "../../../catalog/connector-document.schema.json"
    ))
    .unwrap();
    let v3 = jsonschema::validator_for(&schema3).unwrap();
    let v2 = jsonschema::validator_for(&schema2).unwrap();
    let original: Value =
        serde_json::from_str(include_str!("../../../catalog/github.catalog.json")).unwrap();
    assert!(v3.is_valid(&original));
    assert!(Document::parse(&original.to_string()).is_ok());
    let mut old = original.clone();
    old["$schema"] = schema2["$id"].clone();
    old["schema_version"] = json!(2);
    for operation in old["operations"].as_array_mut().unwrap() {
        assert_eq!(operation["request_semantics"], "legacy_v1");
        operation
            .as_object_mut()
            .unwrap()
            .remove("request_semantics");
    }
    assert!(
        v2.is_valid(&old),
        "the frozen old schema still validates its legacy contract"
    );
    assert!(!v3.is_valid(&old));
    assert!(Document::parse(&old.to_string()).is_err());
    for version in [
        json!(0),
        json!(2),
        json!(4),
        json!(null),
        json!("3"),
        json!(-1),
    ] {
        let mut mutated = original.clone();
        mutated["schema_version"] = version;
        assert!(!v3.is_valid(&mutated));
        assert!(Document::parse(&mutated.to_string()).is_err());
    }
    for profile in [
        None,
        Some(json!(null)),
        Some(json!("openapi_3_0_json_v2")),
        Some(json!("")),
    ] {
        let mut mutated = original.clone();
        let operation = mutated["operations"][0].as_object_mut().unwrap();
        if let Some(profile) = profile {
            operation.insert("request_semantics".to_owned(), profile);
        } else {
            operation.remove("request_semantics");
        }
        assert!(!v3.is_valid(&mutated));
        assert!(Document::parse(&mutated.to_string()).is_err());
    }
}

#[test]
fn adversary_gitlab_pass1_path_and_integer_boundaries_preserve_caller_values() {
    for (logical, encoded) in [
        ("group/project", "group%2Fproject"),
        ("a%2Fb", "a%252Fb"),
        ("a?scope=active#x", "a%3Fscope%3Dactive%23x"),
        ("a/{origin}/b", "a%2F%7Borigin%7D%2Fb"),
        ("a\\b c", "a%5Cb%20c"),
        ("é/项目", "%C3%A9%2F%E9%A1%B9%E7%9B%AE"),
    ] {
        let built = request("gitlab-pipeline-schedule-list", json!({"id":logical}));
        assert_eq!(
            built.url,
            format!("https://gitlab.example/api/v4/projects/{encoded}/pipeline_schedules")
        );
    }
    for integer in [0_u64, 1, 9_007_199_254_740_993, u64::MAX] {
        let built = request(
            "gitlab-pipeline-schedule-list",
            json!({"id":integer,"page":integer}),
        );
        assert_eq!(built.url, format!("https://gitlab.example/api/v4/projects/{integer}/pipeline_schedules?page={integer}"));
    }
    let built = request(
        "gitlab-pipeline-schedule-list",
        json!({"id":12.0,"page":-0.0}),
    );
    assert!(built
        .url
        .ends_with("/projects/12/pipeline_schedules?page=0"));
}

#[test]
fn adversary_gitlab_pass1_body_literals_do_not_become_template_instructions() {
    let body = json!({
        "description":"{origin} {$param} \"null\"", "ref":"null", "cron":"0 * * * *",
        "active":false, "inputs":[{"name":"value","value":[null,false,0,"{origin}",{"$param":"id"}]}],
        "additional":{"$param":"id","$ref":"#/literal","example":{"values":[]}}
    });
    let built = request(
        "gitlab-pipeline-schedule-create",
        json!({"id":12,"body":body}),
    );
    assert_eq!(
        serde_json::from_str::<Value>(built.body.as_deref().unwrap()).unwrap(),
        body
    );
    let absent = request(
        "gitlab-pipeline-schedule-update",
        json!({"id":12,"pipeline_schedule_id":7}),
    );
    assert!(absent.body.is_none());
    let empty = request(
        "gitlab-pipeline-schedule-update",
        json!({"id":12,"pipeline_schedule_id":7,"body":{}}),
    );
    assert_eq!(empty.body.as_deref(), Some("{}"));
}
