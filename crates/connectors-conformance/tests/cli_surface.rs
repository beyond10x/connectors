//! Execute the generated process adapter with fictional, bounded application seams.
//! These tests establish no real keyring durability, supervisor or provider effects.

use connectors_cli_contract::{
    AcquireError, Context, DynamicError, DynamicPhase, DynamicValidator, Handler, HandlerReply,
    Invocation, ProcessOutput, ProtectedSource, Sources,
    wire::{ArgumentSource, Shape},
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
fn the_selected_inventory_has_thirty_grouped_commands_and_explicit_runtime_obligations() {
    let plan = connectors_cli_contract::plan();
    let actual: BTreeSet<_> = plan
        .commands
        .iter()
        .map(|command| command.path.join(" "))
        .collect();
    let expected: BTreeSet<_> = [
        "setup init",
        "setup check",
        "setup checkpoints-enable",
        "setup owner-stop",
        "instances list",
        "instances describe",
        "instances status",
        "instances stop",
        "connections list",
        "connections describe",
        "connections connect",
        "connections reconnect",
        "connections revalidate",
        "connections launch",
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
        "approvals policy-status",
        "approvals policy-set",
        "approvals prepare",
        "approvals issue",
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
            &["approvals", "policy-status", "--adapter", "forge"],
            "ApprovalPolicyResult",
        ),
        (
            &[
                "approvals",
                "policy-set",
                "--adapter",
                "forge",
                "--input-file",
                "/fixture/policy.json",
                "--expected-revision",
                "1",
            ],
            "ApprovalPolicyResult",
        ),
        (
            &[
                "approvals",
                "prepare",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--operation",
                "item.write",
                "--schema",
                "s1",
                "--revision",
                "r1",
                "--input-json",
                "{}",
            ],
            "ApprovalPrepareResult",
        ),
        (
            &[
                "approvals",
                "issue",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--operation",
                "item.write",
                "--schema",
                "s1",
                "--revision",
                "r1",
                "--input-json",
                "{}",
                "--approve-subject",
                "digest",
                "--proof-output",
                "/fixture/proof.json",
            ],
            "ApprovalIssueResult",
        ),
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
        (&["instances", "list", "--limit", "10"], "AdapterListResult"),
        (
            &["instances", "describe", "--adapter", "forge"],
            "AdapterDescribeResult",
        ),
        (
            &["instances", "status", "--adapter", "forge"],
            "AdapterStatusResult",
        ),
        (
            &[
                "instances",
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
                "reconnect",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--expected-revision",
                "r1",
                "--credential-stdin",
            ],
            "ConnectionReconnectResult",
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
                "launch",
                "--adapter",
                "forge",
                "--connection",
                "c1",
                "--consumer",
                "probe",
                "--args",
                "[\"head\",\"--json\"]",
            ],
            "ConnectionLaunchResult",
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
            "ConnectionRevalidateResult" | "ConnectionLaunchResult" | "ApprovalClockCheckResult"
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

/// A connect with no protected source is the read-only binding's form: the
/// contract reads no source and hands the handler no credential document,
/// and the runtime decides by the selected binding (a platform binding answers
/// `protected_entry_unavailable`).
#[test]
fn a_connect_without_a_protected_source_reads_none_and_passes_no_document() {
    let mut handler = Recorder::new(fixture("ConnectionConnectResult"));
    let mut sources = InputSources::default();
    let output = run(
        &[
            "connections",
            "connect",
            "--adapter",
            "forge",
            "--profile",
            "static",
            "--output",
            "json",
        ],
        &mut sources,
        &mut handler,
        None,
    );
    assert_eq!(output.exit_code, 0, "{output:?}");
    assert!(sources.calls.is_empty());
    assert_eq!(handler.calls, 1);
    let (callable, input, _) = handler.last.unwrap();
    assert_eq!(callable, "connections-connect");
    assert!(
        input.get("credential_document").is_none_or(Value::is_null),
        "{input}"
    );
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
                (
                    value["result"].clone(),
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
fn invocation_result(result: Value) -> Value {
    json!({"adapter":"forge","connector":"fixture-forge","instance":"forge","operation":"read","revision":"r1","result":result})
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
        let mut handler = Recorder::new(invocation_result(json!({"count":1})));
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

/// beyond10x/connectors#105: schemas and the provider result are JSON values in
/// the answer, so a caller decodes the output once.
#[test]
fn describe_and_invoke_answer_json_values_not_json_text() {
    let input_schema = json!({"type":"object","properties":{"selector":{"type":"string"}}});
    let output_schema = json!({"type":"object","properties":{"count":{"type":"integer"}}});
    let operation = json!({"id":"read","description":"Read","contract":"operations/v1alpha1","profile":"read","input_schema":input_schema,"output_schema":output_schema});
    let described = json!({"adapter":"forge","connector":"fixture-forge","instance":"forge","revision":"r1","schema":"fixture-input","operation":operation,"source":"cached","stale":true});
    let mut adapter = fixture("AdapterDescribeResult");
    adapter["descriptor"]["operations"] = json!([operation]);
    let cases: [(&[&str], Value); 2] = [
        (
            &[
                "operations",
                "describe",
                "--adapter",
                "forge",
                "--operation",
                "read",
            ],
            described,
        ),
        (&["instances", "describe", "--adapter", "forge"], adapter),
    ];
    for (args, result) in cases {
        let mut handler = Recorder::new(result.clone());
        let output = run(
            &[&["--output", "json"][..], args].concat(),
            &mut InputSources::default(),
            &mut handler,
            None,
        );
        assert_eq!(output.exit_code, 0, "{args:?}: {output:?}");
        let answer = serde_json::from_str::<Value>(&output.stdout).unwrap()["result"].clone();
        assert_eq!(answer, result);
        let answered = answer["operation"]
            .as_object()
            .or_else(|| answer["descriptor"]["operations"][0].as_object())
            .unwrap();
        assert_eq!(answered["input_schema"], input_schema);
        assert_eq!(answered["output_schema"], output_schema);
    }
    let mut handler = Recorder::new(invocation_result(json!({"count":1})));
    let output = run(
        &[INVOKE, &["--input-json", r#"{"selector":"fixture"}"#]].concat(),
        &mut InputSources::default(),
        &mut handler,
        Some(&mut FixtureSchemas::default()),
    );
    assert_eq!(output.exit_code, 0, "{output:?}");
    assert_eq!(
        serde_json::from_str::<Value>(&output.stdout).unwrap()["result"]["result"],
        json!({"count":1})
    );
}

#[test]
fn native_schema_refusals_prevent_handler_dispatch_and_bad_results_never_reach_stdout() {
    let good_args = [INVOKE, &["--input-json", r#"{"selector":"fixture"}"#]].concat();
    let mut sources = InputSources::default();
    let mut handler = Recorder::new(invocation_result(json!({"count":1})));
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
    let mut bad_output = Recorder::new(invocation_result(json!({"count":"wrong"})));
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

#[test]
fn a_configuration_refusal_may_name_the_format_and_entry_and_nothing_else() {
    let named = json!({"kind":"usage","code":"invalid_configuration","stage":"configuration",
        "next_action":"check_configuration","configuration_format":"connectors-local/1",
        "instance_id":"forge-local"});
    let mut sources = InputSources::default();
    let args = ["instances", "list", "--output", "json"];
    let output = refusal(
        &run(
            &args,
            &mut sources,
            &mut ReplyOnce(Some(HandlerReply::UsageError {
                code: "failure".into(),
                data: named.clone(),
            })),
            None,
        ),
        2,
    );
    assert_eq!(output["error"], json!({"code":"failure","data":named}));
    let mut parser_text = named.clone();
    parser_text["parser"] = "TOML parse error at line 1".into();
    let refused = run(
        &args,
        &mut sources,
        &mut ReplyOnce(Some(HandlerReply::UsageError {
            code: "failure".into(),
            data: parser_text,
        })),
        None,
    );
    refusal(&refused, 1);
    assert!(!refused.stderr.contains("TOML"));
}

/// Whether every struct of the shape that carries `adapter` requires
/// `connector` and `instance` beside it. A service descriptor's `adapter`
/// (`configuration_schema` beside it) is the implementation it describes, not
/// a selector.
fn names_selection(shape: &Shape) -> bool {
    match shape.required() {
        Shape::Struct { fields } => {
            (!fields.contains_key("adapter")
                || fields.contains_key("configuration_schema")
                || ["adapter", "connector", "instance"]
                    .iter()
                    .all(|name| fields.get(*name) == Some(&Shape::String)))
                && fields.values().all(names_selection)
        }
        Shape::List { of } => names_selection(of),
        Shape::Map { value } => names_selection(value),
        _ => true,
    }
}

/// story:cli-connector-instance-selectors: every command that takes
/// `--adapter` takes `--connector` and `--instance`, each an optional field of
/// its input, so the parser requires none of them and the handler decides
/// which one selected. Every result type that names a selected alias requires
/// its connector and instance beside it.
#[test]
fn every_adapter_command_also_selects_by_connector_and_instance() {
    let plan = connectors_cli_contract::plan();
    let optional = Shape::Optional {
        of: Box::new(Shape::String),
    };
    let mut selecting = Vec::new();
    for command in &plan.commands {
        let longs: BTreeSet<&str> = command
            .arguments
            .iter()
            .filter_map(|argument| match &argument.source {
                ArgumentSource::Option { long } => Some(long.as_str()),
                _ => None,
            })
            .collect();
        if !longs.contains("adapter") {
            continue;
        }
        selecting.push(command.path.join(" "));
        assert!(
            longs.contains("connector") && longs.contains("instance"),
            "{:?}",
            command.path
        );
        let callable = &plan.callables[&command.callable];
        let Some(Shape::Struct { fields }) = callable.input.as_ref().map(|input| &input.shape)
        else {
            panic!("{:?}: no struct input", command.path);
        };
        for name in ["adapter", "connector", "instance"] {
            assert_eq!(
                fields.get(name),
                Some(&optional),
                "{:?} {name}",
                command.path
            );
        }
    }
    assert_eq!(selecting.len(), 25, "{selecting:?}");
    for (name, callable) in &plan.callables {
        assert!(names_selection(&callable.result.shape), "{name}");
    }
    for selector in [
        &["--connector", "fixture", "--instance", "main"][..],
        &["--adapter", "forge"],
        &[],
    ] {
        let mut handler = Recorder::new(fixture("ApprovalPolicyResult"));
        let output = run(
            &[
                &["approvals", "policy-status", "--output", "json"][..],
                selector,
            ]
            .concat(),
            &mut InputSources::default(),
            &mut handler,
            None,
        );
        assert_eq!(output.exit_code, 0, "{selector:?}: {output:?}");
        let (_, input, _) = handler.last.unwrap();
        let expected: serde_json::Map<String, Value> = selector
            .chunks(2)
            .map(|pair| (pair[0].trim_start_matches("--").to_owned(), json!(pair[1])))
            .collect();
        assert_eq!(input, Value::Object(expected), "{selector:?}");
    }
}

/// story:connections-reconnect-verb: `connections reconnect` replaces
/// `connections repair` and the `instances` group replaces `adapters`. Each old
/// path is a path alias of exactly its new one, so the same arguments reach
/// the same callable with the same input and answer the same output.
#[test]
fn every_renamed_path_is_an_alias_of_its_new_name() {
    let plan = connectors_cli_contract::plan();
    let aliases: BTreeSet<(String, String)> = plan
        .commands
        .iter()
        .flat_map(|command| {
            command
                .aliases
                .iter()
                .map(|alias| (alias.join(" "), command.path.join(" ")))
        })
        .collect();
    let renamed = [
        ("connections repair", "connections reconnect"),
        ("adapters list", "instances list"),
        ("adapters describe", "instances describe"),
        ("adapters status", "instances status"),
        ("adapters stop", "instances stop"),
    ];
    assert_eq!(
        aliases,
        renamed
            .iter()
            .map(|(old, new)| ((*old).to_owned(), (*new).to_owned()))
            .collect()
    );
    let arguments: &[&str] = &[
        "--connector",
        "fixture",
        "--instance",
        "main",
        "--connection",
        "c1",
        "--expected-revision",
        "r1",
        "--host-incarnation",
        "h1",
        "--child-incarnation",
        "p1",
        "--credential-stdin",
    ];
    for (old, new) in renamed {
        let declaration = plan
            .commands
            .iter()
            .find(|command| command.path.join(" ") == new)
            .unwrap();
        let longs: BTreeSet<String> = declaration
            .arguments
            .iter()
            .flat_map(|argument| match &argument.source {
                ArgumentSource::Option { long } => vec![format!("--{long}")],
                ArgumentSource::Protected { stdin, .. } => vec![format!("--{stdin}")],
                _ => Vec::new(),
            })
            .collect();
        let args: Vec<&str> = arguments
            .chunks(2)
            .filter(|pair| pair.len() == 2 && longs.contains(pair[0]))
            .flatten()
            .copied()
            .chain(
                arguments
                    .last()
                    .copied()
                    .filter(|flag| longs.contains(*flag)),
            )
            .collect();
        let result = fixture(match declaration.callable.as_str() {
            "connections-reconnect" => "ConnectionReconnectResult",
            "adapters-list" => "AdapterListResult",
            "adapters-describe" => "AdapterDescribeResult",
            "adapters-status" => "AdapterStatusResult",
            "adapters-stop" => "AdapterStopResult",
            other => panic!("{other}"),
        });
        let answer = |path: &str| {
            let mut handler = Recorder::new(result.clone());
            let mut sources = InputSources {
                value: r#"{"credential":"fictional-credential-sentinel"}"#.into(),
                ..Default::default()
            };
            let output = run(
                &[
                    &["--output", "json"][..],
                    &path.split(' ').collect::<Vec<_>>(),
                    &args,
                ]
                .concat(),
                &mut sources,
                &mut handler,
                None,
            );
            assert_eq!(output.exit_code, 0, "{path}: {output:?}");
            assert_eq!(handler.calls, 1, "{path}");
            (output, handler.last.unwrap(), sources.calls)
        };
        let (by_new, by_old) = (answer(new), answer(old));
        assert_eq!(by_new.0, by_old.0, "{old} -> {new}");
        assert_eq!(by_new.1.0, declaration.callable, "{new}");
        assert_eq!(by_new.1, by_old.1, "{old} -> {new}");
        assert_eq!(by_new.2, by_old.2, "{old} -> {new}");
    }
}
