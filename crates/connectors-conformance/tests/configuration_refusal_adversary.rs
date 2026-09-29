// Adversary cases for story:configuration-refusal-names-the-entry.
// semantics.md: both fields appear only on invalid_configuration, and are
// "always a supported format and a valid selector the operator wrote"; "no
// parser, OS or database text is ever carried". The generated contract is the
// only thing between a handler and stderr, so it has to refuse the rest.
use connectors_cli_contract::{
    AcquireError, Handler, HandlerReply, Invocation, ProtectedSource, Sources,
};
use serde_json::{Value, json};
use std::ffi::OsString;

struct NoSources;
impl Sources for NoSources {
    fn acquire(&mut self, _: ProtectedSource) -> Result<String, AcquireError> {
        Err(AcquireError::Unavailable)
    }
}

struct ReplyOnce(Option<HandlerReply>);
impl Handler for ReplyOnce {
    fn call(&mut self, _: &Invocation<'_>) -> HandlerReply {
        self.0.take().unwrap()
    }
}

fn exit_for(data: Value, usage: bool) -> (i32, String) {
    let reply = if usage {
        HandlerReply::UsageError {
            code: "failure".into(),
            data,
        }
    } else {
        HandlerReply::Error {
            code: "failure".into(),
            data,
        }
    };
    let output = connectors_cli_contract::run(
        ["connectors", "adapters", "list", "--output", "json"]
            .iter()
            .map(OsString::from)
            .collect(),
        &mut NoSources,
        &mut ReplyOnce(Some(reply)),
        None,
    );
    (output.exit_code, output.stderr)
}

fn declared() -> Value {
    json!({"kind":"usage","code":"invalid_configuration","stage":"configuration",
        "next_action":"check_configuration","configuration_format":"connectors-local/1",
        "instance_id":"forge-local"})
}

#[test]
fn the_contract_refuses_parser_text_as_the_configuration_format() {
    let mut parser = declared();
    parser["configuration_format"] = "TOML parse error at line 1, column 1".into();
    let (exit, stderr) = exit_for(parser, true);
    assert_eq!(
        exit,
        1,
        "the contract admitted parser text: {}",
        stderr.trim()
    );
}

/// ESS 0.40 refuses invariants on CLI types (`ess generate cli`: "CLI type
/// 'connectors.cli.Failure' has unsupported invariants or union semantics"), so the
/// contract cannot bound `instance_id` or tie both fields to `invalid_configuration`.
/// The host enforces both: an entry is named only after its selector check passed,
/// and only the CLI's configuration refusal sets the fields.
#[test]
#[ignore = "ESS-LIMIT: CLI types carry no invariants in ESS 0.40; host-enforced"]
fn the_contract_refuses_configuration_coordinates_that_are_not_the_declared_ones() {
    let base = declared();
    let mut admitted = Vec::new();
    // A non-selector instance id: quote, control character, far past 256 bytes.
    let mut control = base.clone();
    control["instance_id"] = "forge\u{1b}[31m\"x".into();
    let mut long = base.clone();
    long["instance_id"] = "a".repeat(5000).into();
    for (name, data) in [("control", control), ("long", long)] {
        let (exit, stderr) = exit_for(data, true);
        if exit != 1 {
            admitted.push(format!(
                "{name}: exit {exit} {}",
                stderr.trim().chars().take(160).collect::<String>()
            ));
        }
    }
    // The coordinates on an unrelated operational code.
    let other = json!({"kind":"operational","code":"outcome_unknown","stage":"publication",
        "next_action":"retry_status","configuration_format":"connectors-local/1",
        "instance_id":"forge-local"});
    let (exit, stderr) = exit_for(other, false);
    if !stderr.contains("cli_error") {
        admitted.push(format!("outcome_unknown: exit {exit} {}", stderr.trim()));
    }
    assert!(
        admitted.is_empty(),
        "the contract admitted undeclared configuration coordinates:\n{}",
        admitted.join("\n")
    );
}
