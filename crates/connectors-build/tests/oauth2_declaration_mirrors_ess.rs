//! `connectors-oauth2`'s declaration types mirror `connectors.oauth2` in the compiled shared
//! specification (`ess/domains/oauth2.yaml`, `story:oauth2-grant-core`): each struct's serde
//! field names, in declaration order, with the same optionality and JSON shape, and each enum's
//! variants, in declaration order. A field or variant added on either side fails here.
use connectors_oauth2::{
    ClientKind, GrantType, Parameter, RedirectKind, RequestEncoding, Rotation, ServerDeclaration,
};
use ess_compiler::{
    ir::{EssIr, ResolvedBody, ResolvedField, ResolvedTypeRef},
    resolve::compile_locating,
    source::SourceMap,
};
use ess_domain::{
    name::QualifiedName,
    spec::{RawSpecFile, Specification},
    system::Source,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const DOMAIN: &str = "connectors.oauth2";

fn collect_yaml(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_yaml(&path, paths);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
        {
            paths.push(path);
        }
    }
}

/// The shared specification, compiled as the gate compiles it.
fn compiled() -> EssIr {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ess");
    let mut paths = Vec::new();
    collect_yaml(&source, &mut paths);
    paths.sort();
    let mut documents = Vec::new();
    let mut labels = Vec::new();
    let mut sources = SourceMap::new();
    for path in paths {
        let label = path.strip_prefix(&source).unwrap().display().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        sources.insert(label.clone(), text.clone());
        documents.push((
            Source::new(label.clone()),
            RawSpecFile::parse(&text).unwrap_or_else(|error| panic!("{label}: {error}")),
        ));
        labels.push(label);
    }
    let specification = Specification::assemble(documents).expect("specification assembles");
    compile_locating(&specification, &sources, &labels)
        .unwrap_or_else(|diagnostics| panic!("ESS compile diagnostics: {diagnostics:?}"))
}

fn body<'a>(ir: &'a EssIr, name: &str) -> &'a ResolvedBody {
    let qualified = QualifiedName::new(format!("{DOMAIN}.{name}")).unwrap();
    &ir.types()
        .get(&qualified)
        .unwrap_or_else(|| panic!("{DOMAIN}.{name} is not in the compiled specification"))
        .body
}

/// The name ESS puts on the wire: its declared wire name, else its name.
fn wire(name: &str, naming: &ess_domain::name::Naming) -> String {
    naming.wire.clone().unwrap_or_else(|| name.to_owned())
}

fn fields<'a>(ir: &'a EssIr, name: &str) -> &'a [ResolvedField] {
    match body(ir, name) {
        ResolvedBody::Struct { fields, .. } => fields,
        _ => panic!("{DOMAIN}.{name} is not a struct"),
    }
}

fn variants(ir: &EssIr, name: &str) -> Vec<String> {
    match body(ir, name) {
        ResolvedBody::Enum { variants } => variants
            .iter()
            .map(|variant| wire(&variant.name, &variant.naming))
            .collect(),
        _ => panic!("{DOMAIN}.{name} is not an enum"),
    }
}

/// Every variant of a Rust enum, by its serde name, against the ESS enum of the same name; and
/// every ESS variant deserializes back to the same name.
fn assert_enum<T: Serialize + DeserializeOwned>(ir: &EssIr, name: &str, all: &[T]) {
    let rust: Vec<String> = all
        .iter()
        .map(|variant| {
            serde_json::to_value(variant)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    let ess = variants(ir, name);
    assert_eq!(rust, ess, "{name} variants");
    for variant in &ess {
        let parsed: T = serde_json::from_value(json!(variant))
            .unwrap_or_else(|error| panic!("{name}::{variant} does not deserialize: {error}"));
        assert_eq!(serde_json::to_value(parsed).unwrap(), json!(variant));
    }
    assert!(
        serde_json::from_value::<T>(json!("not_a_variant")).is_err(),
        "{name} accepts an undeclared variant"
    );
}

/// The keys of a serialized JSON object in the order serde wrote them.
fn key_order(document: &str) -> Vec<String> {
    struct Keys(Vec<String>);
    impl<'de> serde::Deserialize<'de> for Keys {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Keys;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("an object")
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut map: A,
                ) -> Result<Keys, A::Error> {
                    let mut keys = Vec::new();
                    while let Some(key) = map.next_key::<String>()? {
                        map.next_value::<serde::de::IgnoredAny>()?;
                        keys.push(key);
                    }
                    Ok(Keys(keys))
                }
            }
            deserializer.deserialize_map(Visitor)
        }
    }
    serde_json::from_str::<Keys>(document).unwrap().0
}

/// Whether `value` has the JSON shape of `type_ref`, with every ESS enum read from the model.
fn shaped(ir: &EssIr, type_ref: &ResolvedTypeRef, value: &Value) -> bool {
    match type_ref {
        ResolvedTypeRef::Optional { of } => value.is_null() || shaped(ir, of, value),
        ResolvedTypeRef::List { of } => value
            .as_array()
            .is_some_and(|items| items.iter().all(|item| shaped(ir, of, item))),
        ResolvedTypeRef::Primitive { .. } => match type_ref.to_string().as_str() {
            "String" => value.is_string(),
            "Integer" => value.is_i64(),
            "Boolean" => value.is_boolean(),
            other => panic!("{other}: a primitive this test does not read"),
        },
        ResolvedTypeRef::Declared { .. } => {
            let name = type_ref.to_string();
            let local = name
                .strip_prefix(&format!("{DOMAIN}."))
                .unwrap_or_else(|| panic!("{name} is outside {DOMAIN}"));
            match body(ir, local) {
                ResolvedBody::Enum { .. } => value
                    .as_str()
                    .is_some_and(|text| variants(ir, local).iter().any(|v| v == text)),
                ResolvedBody::Struct { fields, .. } => value.as_object().is_some_and(|object| {
                    object.len() == fields.len()
                        && fields.iter().all(|field| {
                            object
                                .get(&wire(&field.name, &field.naming))
                                .is_some_and(|value| shaped(ir, &field.type_ref, value))
                        })
                }),
                _ => panic!("{name}: a body this test does not read"),
            }
        }
        ResolvedTypeRef::Map { .. } => panic!("{type_ref}: a map this test does not read"),
    }
}

/// A Rust struct, serialized with every optional field present, against the ESS struct of the
/// same name: field names in order, JSON shapes, optionality, and closure to unknown fields.
fn assert_struct<T: Serialize + DeserializeOwned>(ir: &EssIr, name: &str, full: &T) {
    let ess = fields(ir, name);
    let document = serde_json::to_string(full).unwrap();
    let expected: Vec<String> = ess
        .iter()
        .map(|field| wire(&field.name, &field.naming))
        .collect();
    assert_eq!(key_order(&document), expected, "{name} fields in order");
    let value: Value = serde_json::from_str(&document).unwrap();
    for field in ess {
        let key = wire(&field.name, &field.naming);
        assert!(
            !value[&key].is_null(),
            "{name}.{key}: the sample must fill every field"
        );
        assert!(
            shaped(ir, &field.type_ref, &value[&key]),
            "{name}.{key} serializes as {} but ESS declares {}",
            value[&key],
            field.type_ref
        );
        let mut without = value.clone();
        without.as_object_mut().unwrap().remove(&key);
        assert_eq!(
            serde_json::from_value::<T>(without).is_ok(),
            field.type_ref.is_optional(),
            "{name}.{key}: omitting it must succeed exactly when ESS declares {}",
            field.type_ref
        );
    }
    let mut extended = value.clone();
    extended
        .as_object_mut()
        .unwrap()
        .insert("undeclared_field".into(), json!("x"));
    assert!(
        serde_json::from_value::<T>(extended).is_err(),
        "{name} accepts a field ESS does not declare"
    );
    assert!(
        !ir.undeclared_fields_ignored()
            .contains(&QualifiedName::new(format!("{DOMAIN}.{name}")).unwrap()),
        "{name}: ESS ignores undeclared fields, the crate refuses them"
    );
}

fn full_declaration() -> ServerDeclaration {
    ServerDeclaration {
        authorize_url: Some("https://auth.example.test/authorize".into()),
        token_url: "https://auth.example.test/token".into(),
        device_authorization_url: Some("https://auth.example.test/device".into()),
        client_id: Some("fixture-client-id".into()),
        client_kind: ClientKind::Public,
        request_encoding: RequestEncoding::Form,
        refresh_encoding: RequestEncoding::Json,
        redirect_kind: RedirectKind::LoopbackAnyPort,
        redirect_uri: Some("http://127.0.0.1/callback".into()),
        authorize_parameters: vec![Parameter {
            name: "prompt".into(),
            value: "login".into(),
        }],
        requested_scopes: vec!["fixture.read".into()],
        required_scopes: vec!["fixture.read".into()],
        rotation: Rotation::Rotating,
        refresh_lead_seconds: 300,
        grant_types: vec![GrantType::AuthorizationCode, GrantType::DeviceCode],
        sources: vec!["https://example.test/reference".into()],
        state_in_exchange: true,
    }
}

#[test]
fn server_declaration_and_its_enums_mirror_the_compiled_specification() {
    let ir = compiled();
    assert_struct(&ir, "ServerDeclaration", &full_declaration());
    assert_struct(
        &ir,
        "Parameter",
        &Parameter {
            name: "prompt".into(),
            value: "login".into(),
        },
    );
    assert_enum(&ir, "GrantType", GrantType::ALL);
    assert_enum(&ir, "ClientKind", ClientKind::ALL);
    assert_enum(&ir, "RequestEncoding", RequestEncoding::ALL);
    assert_enum(&ir, "RedirectKind", RedirectKind::ALL);
    assert_enum(&ir, "Rotation", Rotation::ALL);
}
