use connectors_spec::{compile, descriptor_bytes};
use serde_json::{Value, json};

#[test]
fn all_adapter_descriptors_are_reproducible_and_schema_valid() {
    for (source, generated) in [
        (
            include_bytes!("../../../adapters/gitlab/spec/adapter.json").as_slice(),
            include_bytes!("../../../adapters/gitlab/generated/descriptor.json").as_slice(),
        ),
        (
            include_bytes!("../../../adapters/kubernetes/spec/adapter.json").as_slice(),
            include_bytes!("../../../adapters/kubernetes/generated/descriptor.json").as_slice(),
        ),
        (
            include_bytes!("../../../adapters/sql/spec/adapter.json").as_slice(),
            include_bytes!("../../../adapters/sql/generated/descriptor.json").as_slice(),
        ),
    ] {
        let descriptor = compile(source).unwrap();
        assert_eq!(descriptor_bytes(&descriptor).unwrap(), generated);
        let value: Value = serde_json::from_slice(source).unwrap();
        let compact = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            descriptor_bytes(&compile(&compact).unwrap()).unwrap(),
            generated
        );
    }
}

#[test]
fn ambiguous_unmapped_and_duplicate_declarations_are_refused() {
    let source = include_bytes!("../../../adapters/gitlab/spec/adapter.json");
    let original: Value = serde_json::from_slice(source).unwrap();
    let mut duplicate = original.clone();
    let operation = duplicate["operations"][0].clone();
    duplicate["operations"]
        .as_array_mut()
        .unwrap()
        .push(operation);
    assert!(compile(&serde_json::to_vec(&duplicate).unwrap()).is_err());
    let mut unknown = original.clone();
    unknown["extensions"] = json!({"callable_stub":true});
    assert!(compile(&serde_json::to_vec(&unknown).unwrap()).is_err());
    let mut unsupported = original;
    unsupported["operations"][0]["input_schema"] =
        json!({"$ref":"https://unconfigured.invalid/schema"});
    assert!(compile(&serde_json::to_vec(&unsupported).unwrap()).is_err());
    assert!(compile(br#"{"kind":"connectors.adapter/v1","kind":"other"}"#).is_err());
}

#[test]
fn shared_configuration_imports_keep_constraints_and_reject_unknown_versions() {
    let mut value: Value =
        serde_json::from_slice(include_bytes!("../../../adapters/sql/spec/adapter.json")).unwrap();
    let descriptor = compile(&serde_json::to_vec(&value).unwrap()).unwrap();
    let schema = &descriptor.configuration_schema["properties"]["service"];
    let validator = jsonschema::validator_for(schema).unwrap();
    assert!(validator.is_valid(&json!({"instance":"sql","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":"token"}})));
    assert!(!validator.is_valid(
        &json!({"instance":"bad\nname","listen":"x","service_credential":{"kind":"file","path":""}})
    ));
    value["configuration_schema"]["properties"]["service"]["$ref"] =
        json!("urn:connectors:config:v99:service");
    assert!(compile(&serde_json::to_vec(&value).unwrap()).is_err());
    value["configuration_schema"]["properties"]["service"] =
        json!({"$ref":"urn:connectors:config:v1:service","required":["extra"]});
    let descriptor = compile(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(!jsonschema::validator_for(&descriptor.configuration_schema["properties"]["service"]).unwrap().is_valid(&json!({"instance":"sql","listen":"x","service_credential":{"kind":"file","path":"token"}})));
}

#[test]
fn configuration_examples_are_data_not_imports() {
    let mut schema = json!({"type":"object", "examples":[{"$ref":"urn:connectors:config:v99:service"}], "const":{"$ref":"urn:connectors:config:v99:service"}});
    let original = schema.clone();
    connectors_host::schema::expand(&mut schema).unwrap();
    assert_eq!(schema, original);
}
