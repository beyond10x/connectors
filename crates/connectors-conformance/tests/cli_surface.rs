//! Execute the generated process adapter with fictional, bounded application seams.
//! These tests establish no real keyring durability, supervisor or provider effects.

use connectors_cli_contract::{
    AcquireError, Context, DynamicError, DynamicPhase, DynamicValidator, Handler, HandlerReply,
    Invocation, ProcessOutput, ProtectedSource, Sources,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, ffi::OsString};

fn fixture(name: &str) -> Value {
    let data: Value = serde_json::from_str(include_str!(
        "../../../contracts/cli/v1alpha1/fixtures/values.json"
    ))
    .unwrap();
    data["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["valid"] == true && case["type"] == format!("connectors.cli.{name}"))
        .unwrap_or_else(|| panic!("no valid fixture for {name}"))["value"]
        .clone()
}

#[derive(Default)]
struct InputSources {
    value: String,
    calls: Vec<ProtectedSource>,
}
impl Sources for InputSources {
    fn acquire(&mut self, source: ProtectedSource) -> Result<String, AcquireError> {
        self.calls.push(source);
        Ok(self.value.clone())
    }
}

struct Recorder {
    result: Value,
    calls: usize,
    last: Option<(String, Value, Context)>,
}
impl Recorder {
    fn new(result: Value) -> Self {
        Self {
            result,
            calls: 0,
            last: None,
        }
    }
}
impl Handler for Recorder {
    fn call(&mut self, invocation: &Invocation<'_>) -> HandlerReply {
        self.calls += 1;
        self.last = Some((
            invocation.callable.to_owned(),
            invocation.input.clone(),
            invocation.context.clone(),
        ));
        HandlerReply::Success(self.result.clone())
    }
}

fn run(
    args: &[&str],
    sources: &mut dyn Sources,
    handler: &mut dyn Handler,
    validator: Option<&mut dyn DynamicValidator>,
) -> ProcessOutput {
    connectors_cli_contract::run(
        std::iter::once("connectors")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect(),
        sources,
        handler,
        validator,
    )
}

fn refusal(output: &ProcessOutput, exit: i32) -> Value {
    assert_eq!(output.exit_code, exit, "{output:?}");
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_str(&output.stderr).unwrap();
    assert_eq!(value["ok"], false);
    value
}

#[test]
fn the_selected_inventory_has_twenty_three_grouped_commands_and_explicit_runtime_obligations() {
    let plan = connectors_cli_contract::plan();
    let actual: BTreeSet<_> = plan
        .commands
        .iter()
        .map(|command| command.path.join(" "))
        .collect();
    let expected: BTreeSet<_> = [
        "setup init",
        "setup check",
        "adapters list",
        "adapters describe",
        "adapters status",
        "adapters stop",
        "connections list",
        "connections describe",
        "connections connect",
        "connections repair",
        "connections revalidate",
        "connections status",
        "connections revoke",
        "operations list",
        "operations describe",
        "operations invoke",
        "approvals key-init",
        "approvals key-status",
        "approvals key-rotate",
        "approvals key-recover",
        "approvals key-revoke",
        "approvals key-retire",
        "approvals clock-check",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(actual, expected);
    assert!(!plan.obligations.is_empty());
    let mut sources = InputSources::default();
    let mut handler = connectors_cli_contract::UnavailableHandler;
    refusal(
        &run(
            &["setup", "check", "--output", "json"],
            &mut sources,
            &mut handler,
            None,
        ),
        1,
    );
    assert!(sources.calls.is_empty());
}

#[test]
fn inputless_setup_keeps_configuration_in_context_and_emits_one_typed_result() {
    let result = fixture("SetupCheckResult");
    let mut handler = Recorder::new(result.clone());
    let mut sources = InputSources::default();
    let output = run(
        &[
            "--config",
            "/fixture/config.toml",
            "setup",
            "check",
            "--state-dir",
            "/fixture/state",
            "--output",
            "json",
        ],
        &mut sources,
        &mut handler,
        None,
    );
    assert_eq!(output.exit_code, 0);
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_str::<Value>(&output.stdout).unwrap(),
        json!({"ok":true,"result":result})
    );
    let (callable, input, context) = handler.last.unwrap();
    assert_eq!(callable, "setup-check");
    assert_eq!(input, json!({}));
    assert_eq!(
        context.config.unwrap().to_str().unwrap(),
        "/fixture/config.toml"
    );
    assert_eq!(
        context.state_dir.unwrap().to_str().unwrap(),
        "/fixture/state"
    );
    assert!(sources.calls.is_empty());
}

#[test]
fn every_local_action_dispatches_once_with_its_declared_result_type() {
    let cases: &[(&[&str], &str)] = &[
        (
            &["approvals", "clock-check", "--adapter", "forge"],
            "ApprovalClockCheckResult",
        ),
        (
            &["approvals", "key-init", "--adapter", "forge"],
            "ApprovalKeysResult",
        ),
        (
            &["approvals", "key-status", "--adapter", "forge"],
            "ApprovalKeysResult",
        ),
        (
            &[
                "approvals",
                "key-rotate",
                "--adapter",
                "forge",
                "--expected-revision",
                "r1",
                "--expected-key",
                "k1",
            ],
            "ApprovalKeysResult",
        ),
        (
            &[
                "approvals",
                "key-recover",
                "--adapter",
                "forge",
                "--expected-revision",
                "r1",
                "--candidate",
                "k1",
            ],
            "ApprovalKeysResult",
        ),
        (
            &[
                "approvals",
                "key-revoke",
                "--adapter",
                "forge",
                "--expected-revision",
                "r1",
                "--expected-key",
                "k1",
            ],
            "ApprovalKeysResult",
        ),
        (
            &[
                "approvals",
                "key-retire",
                "--adapter",
                "forge",
                "--expected-revision",
                "r1",
                "--key",
                "k1",
            ],
            "ApprovalKeysResult",
        ),
        (&["setup", "init"], "SetupInitResult"),
        (&["setup", "check"], "SetupCheckResult"),
        (&["adapters", "list", "--limit", "10"], "AdapterListResult"),
        (
            &["adapters", "describe", "--adapter", "forge"],
            "AdapterDescribeResult",
        ),
        (
            &["adapters", "status", "--adapter", "forge"],
            "AdapterStatusResult",
        ),
        (
            &[
                "adapters",
                "stop",
                "--adapter",
                "forge",
                "--expected-revision",
                "cfg-1",
                "--host-incarnation",
                "h1",
                "--child-incarnation",
                "p1",
            ],
            "AdapterStopResult",
        ),
        (
            &["connections", "list", "--adapter", "forge"],
            "ConnectionListResult",
        ),
        (
            &[
                "connections",
                "describe",
                "--adapter",
                "forge",
                "--connection",
                "c1",
            ],
            "ConnectionDescribeResult",
        ),
        (
            &[
                "connections",
                "connect",
                "--adapter",
                "forge",
                "--profile",
                "static",
                "--credential-stdin",
            ],
            "ConnectionConnectResult",
        ),
        (
            &[
                "connections",
                "repair",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--expected-revision",
                "r1",
                "--credential-stdin",
            ],
            "ConnectionRepairResult",
        ),
        (
            &[
                "connections",
                "revalidate",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--expected-revision",
                "r1",
            ],
            "ConnectionRevalidateResult",
        ),
        (
            &[
                "connections",
                "status",
                "--adapter",
                "forge",
                "--connection",
                "c1",
            ],
            "ConnectionStatusResult",
        ),
        (
            &[
                "connections",
                "revoke",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--expected-revision",
                "r1",
            ],
            "ConnectionRevokeResult",
        ),
        (
            &["operations", "list", "--adapter", "forge"],
            "OperationListResult",
        ),
        (
            &[
                "operations",
                "describe",
                "--adapter",
                "forge",
                "--operation",
                "read",
            ],
            "OperationDescribeResult",
        ),
    ];
    for (args, result_type) in cases {
        let result = fixture(result_type);
        let mut handler = Recorder::new(result.clone());
        let mut sources = InputSources {
            value: r#"{"credential":"fictional-credential-sentinel"}"#.into(),
            ..Default::default()
        };
        let output = run(
            &[&["--output", "json"][..], args].concat(),
            &mut sources,
            &mut handler,
            None,
        );
        assert_eq!(output.exit_code, 0, "{args:?}: {output:?}");
        assert_eq!(handler.calls, 1);
        if matches!(
            *result_type,
            "ConnectionRevalidateResult" | "ApprovalClockCheckResult"
        ) {
            assert!(sources.calls.is_empty());
        }
        assert_eq!(
            serde_json::from_str::<Value>(&output.stdout).unwrap()["result"],
            result
        );
        assert!(!output.stdout.contains("fictional-credential-sentinel"));
        assert!(!output.stderr.contains("fictional-credential-sentinel"));
    }
}

#[test]
fn protected_source_conflicts_and_parser_errors_never_capture_or_echo_values() {
    for extra in [
        vec![
            "--credential-file",
            "/private/fictional-sentinel",
            "--credential-stdin",
        ],
        vec!["--credential-stdin", "--credential-prompt"],
        vec!["--credential-document", "fictional-sentinel"],
        vec![
            "--credential-file",
            "/private/fictional-sentinel",
            "--unknown",
        ],
        vec![],
    ] {
        let mut handler = Recorder::new(fixture("ConnectionConnectResult"));
        let mut sources = InputSources::default();
        let args = [
            &[
                "connections",
                "connect",
                "--adapter",
                "forge",
                "--profile",
                "static",
            ][..],
            &extra,
            &["--output", "json"],
        ]
        .concat();
        let output = run(&args, &mut sources, &mut handler, None);
        refusal(&output, 2);
        assert!(sources.calls.is_empty());
        assert_eq!(handler.calls, 0);
        assert!(!output.stderr.contains("fictional-sentinel"));
        assert!(!output.stderr.contains("/private/"));
    }
}

#[derive(Default)]
struct FixtureSchemas {
    phases: Vec<&'static str>,
}
impl DynamicValidator for FixtureSchemas {
    fn validate(
        &mut self,
        invocation: &Invocation<'_>,
        phase: DynamicPhase,
        value: &Value,
    ) -> Result<(), DynamicError> {
        if invocation.input["schema"] != "fixture-input" || invocation.input["revision"] != "r1" {
            return Err(DynamicError::Unavailable);
        }
        let (decoded, schema) = match phase {
            DynamicPhase::Input => {
                self.phases.push("input");
                // The generic adapter supplies a parsed value. Inspect the original
                // carrier too, so duplicate members cannot disappear before native
                // validation; this fixture proves source acquisition, not parse count.
                let decoded: Value = connectors_core::read_json(
                    invocation.input["input"].as_str().unwrap().as_bytes(),
                )
                .map_err(|_| DynamicError::InvalidValue)?;
                if &decoded != value {
                    return Err(DynamicError::InvalidValue);
                }
                (
                    decoded,
                    json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"],"additionalProperties":false}),
                )
            }
            DynamicPhase::Result => {
                self.phases.push("result");
                let decoded: Value = connectors_core::read_json(
                    value["result"]
                        .as_str()
                        .ok_or(DynamicError::InvalidValue)?
                        .as_bytes(),
                )
                .map_err(|_| DynamicError::InvalidValue)?;
                (
                    decoded,
                    json!({"type":"object","properties":{"count":{"type":"integer"}},"required":["count"],"additionalProperties":false}),
                )
            }
            DynamicPhase::Error(_) => return Ok(()),
        };
        if jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&decoded)
        {
            Ok(())
        } else {
            Err(DynamicError::InvalidValue)
        }
    }
}

const INVOKE: &[&str] = &[
    "operations",
    "invoke",
    "--adapter",
    "forge",
    "--connection",
    "c1",
    "--operation",
    "read",
    "--schema",
    "fixture-input",
    "--revision",
    "r1",
    "--output",
    "json",
];
fn invocation_result(result: &str) -> Value {
    json!({"adapter":"forge","operation":"read","revision":"r1","result":result})
}

#[test]
fn dynamic_business_sources_are_acquired_once_and_validate_both_directions() {
    for source in [
        vec!["--input-json", r#"{"selector":"fixture"}"#],
        vec!["--input-file", "/fixture/input.json"],
        vec!["--input-stdin"],
    ] {
        let mut sources = InputSources {
            value: r#"{"selector":"fixture"}"#.into(),
            ..Default::default()
        };
        let mut handler = Recorder::new(invocation_result(r#"{"count":1}"#));
        let mut validator = FixtureSchemas::default();
        let output = run(
            &[INVOKE, &source].concat(),
            &mut sources,
            &mut handler,
            Some(&mut validator),
        );
        assert_eq!(output.exit_code, 0, "{output:?}");
        assert_eq!(handler.calls, 1);
        assert_eq!(validator.phases, ["input", "result"]);
        assert_eq!(
            handler.last.as_ref().unwrap().1["input"],
            r#"{"selector":"fixture"}"#
        );
        assert_eq!(
            sources.calls.len(),
            usize::from(source[0] != "--input-json")
        );
    }
}

#[test]
fn native_schema_refusals_prevent_handler_dispatch_and_bad_results_never_reach_stdout() {
    let good_args = [INVOKE, &["--input-json", r#"{"selector":"fixture"}"#]].concat();
    let mut sources = InputSources::default();
    let mut handler = Recorder::new(invocation_result(r#"{"count":1}"#));
    refusal(&run(&good_args, &mut sources, &mut handler, None), 1);
    assert_eq!(handler.calls, 0);
    for document in [
        r#"{"selector":1}"#,
        r#"{"selector":"a","selector":"b"}"#,
        r#"{"selector":"a"} {}"#,
        r#"{"unknown":"a"}"#,
        "not-json",
    ] {
        let args = [INVOKE, &["--input-json", document]].concat();
        refusal(
            &run(
                &args,
                &mut sources,
                &mut handler,
                Some(&mut FixtureSchemas::default()),
            ),
            2,
        );
        assert_eq!(handler.calls, 0);
    }
    let stale: Vec<_> = good_args
        .iter()
        .map(|s| if *s == "r1" { "r0" } else { *s })
        .collect();
    refusal(
        &run(
            &stale,
            &mut sources,
            &mut handler,
            Some(&mut FixtureSchemas::default()),
        ),
        1,
    );
    assert_eq!(handler.calls, 0);
    let mut bad_output = Recorder::new(invocation_result(r#"{"count":"wrong"}"#));
    refusal(
        &run(
            &good_args,
            &mut sources,
            &mut bad_output,
            Some(&mut FixtureSchemas::default()),
        ),
        1,
    );
    assert_eq!(bad_output.calls, 1);
}

struct ReplyOnce(Option<HandlerReply>);
impl Handler for ReplyOnce {
    fn call(&mut self, _: &Invocation<'_>) -> HandlerReply {
        self.0.take().unwrap()
    }
}

#[test]
fn typed_application_usage_operational_and_interrupt_outcomes_use_distinct_exits() {
    let operational = fixture("Failure");
    let mut usage = operational.clone();
    usage["kind"] = "usage".into();
    usage["code"] = "invalid_configuration".into();
    usage["stage"] = "configuration".into();
    usage["next_action"] = "check_configuration".into();
    let mut sources = InputSources::default();
    let args = ["setup", "check", "--output", "json"];
    let usage_output = refusal(
        &run(
            &args,
            &mut sources,
            &mut ReplyOnce(Some(HandlerReply::UsageError {
                code: "failure".into(),
                data: usage.clone(),
            })),
            None,
        ),
        2,
    );
    assert_eq!(
        usage_output["error"],
        json!({"code":"failure","data":usage})
    );
    let operational_output = refusal(
        &run(
            &args,
            &mut sources,
            &mut ReplyOnce(Some(HandlerReply::Error {
                code: "failure".into(),
                data: operational.clone(),
            })),
            None,
        ),
        1,
    );
    assert_eq!(
        operational_output["error"],
        json!({"code":"failure","data":operational})
    );
    refusal(
        &run(
            &args,
            &mut sources,
            &mut ReplyOnce(Some(HandlerReply::Interrupted)),
            None,
        ),
        130,
    );
    let malformed = run(
        &args,
        &mut sources,
        &mut ReplyOnce(Some(HandlerReply::Success(
            json!({"secret":"fictional-sentinel"}),
        ))),
        None,
    );
    refusal(&malformed, 1);
    assert!(!malformed.stderr.contains("fictional-sentinel"));
}
