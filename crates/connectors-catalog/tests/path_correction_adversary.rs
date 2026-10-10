//! Adversary cases for cited path corrections
//! (story:catalog-path-correction-and-value-bound).
//!
//! The amendment model (`adapters/catalog/spec/ess/domains/amendment.yaml`,
//! `PathCorrection`) and the module documentation of
//! `connectors_catalog::amendment` both promise that a corrected path "keeps
//! every path parameter and cannot retarget the operation elsewhere". The
//! check only asks that each parenthesised group be removed or unwrapped, so a
//! group that is not GitLab's optional-segment notation but carries path
//! parameters, as the pinned GitLab document's NuGet OData routes do, can be
//! removed with its parameters.
use connectors_catalog::inventory::{Inventory, Location, Operation, Parameter, ValueType};

const FORMAT: &str = "connectors-source-amendments/1";

/// The route exactly as the pinned GitLab document declares it.
const NUGET: &str = "/api/v4/projects/{project_id}/packages/nuget/v2/Packages(Id='{package_name}',Version='{package_version}')";
const OPERATION: &str =
    "getApiV4ProjectsProjectIdPackagesNugetV2PackagesidPackageNameVersionPackageVersion";

fn path_parameter(name: &str) -> Parameter {
    Parameter {
        name: name.into(),
        location: Location::Path,
        required: true,
        value_type: Some(ValueType::String),
        repeated: false,
    }
}

fn inventory() -> Inventory {
    Inventory {
        operations: vec![Operation {
            method: "get".into(),
            path: NUGET.into(),
            operation_id: Some(OPERATION.into()),
            parameters: vec![
                path_parameter("project_id"),
                path_parameter("package_name"),
                path_parameter("package_version"),
            ],
            request_media_types: Vec::new(),
            responses: Vec::new(),
        }],
        unsupported: Vec::new(),
    }
}

#[test]
fn adversary_a_path_correction_may_not_drop_a_group_holding_path_parameters() {
    let source = connectors_catalog::ingest(
        "fixture.json",
        br#"{"openapi":"3.0.3","info":{"title":"t","version":"1"},"paths":{}}"#,
    )
    .unwrap();
    let bytes = format!(
        r#"{{"format":"{FORMAT}","source_sha256":"{}","amendments":[{{"operation_id":"{OPERATION}",
        "correct_path":{{"from":"{NUGET}","to":"/api/v4/projects/{{project_id}}/packages/nuget/v2/Packages"}},
        "cite":"https://docs.gitlab.example.test/api/packages/nuget/","reason":"constructed"}}]}}"#,
        source.source_sha256
    )
    .into_bytes();
    let mut inventory = inventory();
    let applied = connectors_catalog::amendment::apply("a.json", &bytes, &source, &mut inventory);
    assert!(
        applied.is_err(),
        "a correction dropped the path parameters `package_name` and `package_version`, \
         retargeting the operation to `{}`",
        inventory.operations[0].path
    );
    assert_eq!(inventory, self::inventory());
}
