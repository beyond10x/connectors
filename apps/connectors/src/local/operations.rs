use super::*;
use connectors_host::local::{config::Adapter, owner::Code, runtime::Bootstrap};

fn summary(operation: &connectors_core::Operation) -> Value {
    json!({"id":operation.id,"description":operation.description,"contract":operation.contract,"profile":operation.profile})
}
fn operation(operation: &connectors_core::Operation) -> Value {
    let mut value = summary(operation);
    value["input_schema"] = operation.input_schema.clone();
    value["output_schema"] = operation.output_schema.clone();
    value
}
/// The optional `--family` filter: a contract id matched exactly against each
/// operation's descriptor `contract`. `None` when the value is malformed.
fn family(input: &Value) -> Option<Option<&str>> {
    match input.get("family") {
        None | Some(Value::Null) => Some(None),
        Some(value) => value
            .as_str()
            .filter(|family| {
                !family.is_empty()
                    && family.len() <= 128
                    && family
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-_./".contains(&c))
            })
            .map(Some),
    }
}
pub(super) fn descriptor(bootstrap: &Bootstrap) -> owner::Result<Value> {
    let descriptor = bootstrap.descriptor()?;
    Ok(
        json!({"version":descriptor.version,"instance":descriptor.instance,"adapter":descriptor.adapter,"revision":descriptor.revision,"configuration_schema":descriptor.configuration_schema.to_string(),"operations":descriptor.operations.iter().map(operation).collect::<Vec<_>>()}),
    )
}
pub(super) fn describe(
    call: &Invocation<'_>,
    paths: &Paths,
    alias: &str,
    adapter: &Adapter,
) -> owner::Result<Value> {
    let refused = |refusal| match refusal {
        super::cursor::Refusal::InvalidInput => Code::InvalidInput,
        super::cursor::Refusal::StaleCursor => Code::StaleCursor,
    };
    // `--family`, `--limit` and the cursor's encoding are checked before the
    // cached description is read.
    let request = (call.callable == "operations-list")
        .then(|| {
            let family = family(&call.input).ok_or(Code::InvalidInput)?;
            let request = super::cursor::Request::parse(&call.input).map_err(refused)?;
            Ok::<_, owner::Error>((family, request))
        })
        .transpose()?;
    let bootstrap = owner::cached(paths, alias)?;
    let descriptor = bootstrap.descriptor()?;
    if let Some((family, request)) = request {
        let operations = descriptor
            .operations
            .iter()
            .filter(|o| family.is_none_or(|family| o.contract == family))
            .filter(|o| owner::admit_operation(adapter, &bootstrap, &o.id).is_ok())
            .collect::<Vec<_>>();
        // The selection a cursor is bound to: the adapter it lists, the
        // descriptor revision, the family it is filtered to and the operations
        // this configuration permits.
        let mut source = json!({"adapter":alias,"instance_id":adapter.instance_id,"revision":descriptor.revision,"operations":operations.iter().map(|o| &o.id).collect::<Vec<_>>()});
        if let Some(family) = family {
            source["family"] = json!(family);
        }
        let (range, next) = request.page(&source, operations.len()).map_err(refused)?;
        let mut result = json!({"adapter":alias,"revision":descriptor.revision,"operations":operations[range].iter().map(|o| summary(o)).collect::<Vec<_>>(),"source":"cached","stale":true});
        if let Some(next) = next {
            result["next_cursor"] = json!(next);
        }
        Ok(result)
    } else {
        let name = call.input["operation"]
            .as_str()
            .filter(|name| connectors_core::valid_id(name))
            .ok_or(Code::InvalidInput)?;
        owner::admit_operation(adapter, &bootstrap, name)?;
        let selected = descriptor.operation(name).map_err(|_| Code::NotFound)?;
        Ok(
            json!({"adapter":alias,"revision":descriptor.revision,"schema":owner::schema(&bootstrap,name)?,"operation":operation(selected),"source":"cached","stale":true}),
        )
    }
}
pub(super) fn invoke(call: &Invocation<'_>, deadline: u64) -> owner::Result<Value> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )?;
    let alias = call.input["adapter"].as_str().ok_or(Code::InvalidInput)?;
    let document = call.input["input"]
        .as_str()
        .ok_or(Code::InvalidInput)?
        .as_bytes();
    // A running owner of this build admits the invoke itself, from its held
    // metadata handle, before it dispatches. Without one the CLI admits it
    // directly, which verifies the whole store, and only then starts an owner.
    let client = match owner::Client::connect(&paths, false) {
        Ok(client) if client.is_same_build() => client,
        _ => {
            owner::admit_invoke(&paths, alias, &call.input, document)?;
            if connectors_sdk::now_ms() >= deadline {
                return Err(Code::Timeout.into());
            }
            owner::Client::connect(&paths, true)?
        }
    };
    if connectors_sdk::now_ms() >= deadline {
        return Err(Code::Timeout.into());
    }
    client.invoke(alias, &call.input, document, deadline)
}

pub(super) fn dispatch(
    call: &Invocation<'_>,
    read_deadline: u64,
    deadline: Option<owner::mutation::Deadline>,
) -> owner::Result<HandlerReply> {
    use connectors_host::local::{protected, runtime::Effect};
    use owner::{approval_issuance::Request, mutation};
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )?;
    let text = |field: &str| call.input[field].as_str().ok_or(Code::InvalidInput);
    let alias = text("adapter")?;
    let bootstrap = owner::operation_snapshot(&paths, alias, &call.input)?;
    let operation = text("operation")?;
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|r| r.operation == operation)
        .ok_or(Code::NotFound)?;
    let key = call.input["idempotency_key"].as_str();
    let proof_path = call.input["approval_file"].as_str();
    if requirement.effect == Effect::Read {
        if key.is_some() || proof_path.is_some() {
            return Err(Code::InvalidInput.into());
        }
        return invoke(call, read_deadline).map(HandlerReply::Success);
    }
    if requirement.effect != Effect::Write {
        return Err(Code::Unsupported.into());
    }
    let deadline = deadline.ok_or(Code::Unavailable)?;
    let until = deadline.until()?;
    let request = Request {
        connection: text("connection")?,
        operation,
        schema: text("schema")?,
        revision: text("revision")?,
        input: text("input")?,
    };
    // Current result-access admission and exact-key observation happen before
    // even opening the proof file, unlocking custody or starting the owner.
    let original = mutation::observe_original(&paths, alias, &request, key, until)?;
    if let Some(original) = &original
        && !original.pending
    {
        return project(call, &bootstrap, original.delivery.clone());
    }
    let pending = original.is_some();
    let value = {
        // Pending recovery is metadata-only. A deleted/locked proof source
        // cannot block it, and it never supplies a new execution proof.
        let proof = if pending {
            Ok(None)
        } else {
            proof_path
                .map(|path| protected::file_until(std::path::Path::new(path), until, 20 * 1024))
                .transpose()
        };
        let proof = match proof {
            Ok(proof) => proof,
            Err(error) => {
                // Another caller may have reserved the same key while protected
                // entry failed. Reveal only a currently admitted exact winner.
                if let Some(value) = mutation::observe(&paths, alias, &request, key, until)? {
                    return project(call, &bootstrap, value);
                }
                return Err(error);
            }
        };
        let result = owner::WriteClient::connect(&paths, true, deadline)
            .and_then(|client| client.invoke(alias, &request, key, proof.as_ref(), deadline))
            .map_err(|mut error| {
                if pending
                    && matches!(
                        error.code,
                        Code::Unavailable | Code::Timeout | Code::Interrupted | Code::Capacity
                    )
                {
                    error.code = Code::OutcomeUnknown;
                }
                error
            });
        match result {
            Ok(value) => value,
            Err(error) if error.code == Code::OutcomeUnknown => {
                let mut reply = owner_failure(error);
                if let HandlerReply::Error { data, .. } = &mut reply {
                    data["mutation"] = json!({"classification":"unknown","attempt":null,"original_request_id":null,"replayed":false,"cause":{"code":"unavailable","stage":"response"}});
                    data["source_audit"] = json!({"instance":bootstrap.instance,"audit_ref":null,"audit_status":"unavailable"});
                }
                return Ok(reply);
            }
            Err(error) => return Err(error),
        }
    };
    // No payload enters the generated success carrier without native validation.
    // The owner is the authority for current admission and effect classification.
    project(call, &bootstrap, value)
}

fn project(
    call: &Invocation<'_>,
    bootstrap: &Bootstrap,
    mut value: owner::mutation::Delivery,
) -> owner::Result<HandlerReply> {
    use owner::mutation::{Cause, FailureCode, Stage};
    if let Some(payload) = &value.result {
        let descriptor = bootstrap.descriptor()?;
        let operation = descriptor
            .operation(call.input["operation"].as_str().ok_or(Code::InvalidInput)?)
            .map_err(|_| Code::NotFound)?;
        if connectors_sdk::validate_write_value(&operation.output_schema, payload).is_err() {
            value.result = None;
            value.error = Some(owner::mutation::Failure {
                code: FailureCode::Owner(Code::ServiceFailure),
                service_code: Some(connectors_core::ErrorCode::UpstreamProtocol),
                origin: owner::Origin::Host,
            });
            value.mutation.cause = Some(Cause {
                code: connectors_core::ErrorCode::UpstreamProtocol,
                stage: Stage::Response,
            });
        }
    }
    let mut output = if let Some(payload) = value.result {
        json!({"adapter":call.input["adapter"],"operation":call.input["operation"],"revision":call.input["revision"],"result":payload})
    } else {
        let error = value.error.as_ref().ok_or(Code::OutcomeUnknown)?;
        let classification = value.mutation.classification;
        let mut data = json!({"kind":"operational","code":error.code,"stage":error.stage(classification),"next_action":error.next_action(classification)});
        if let Some(code) = &error.service_code {
            data["service_code"] = json!(code);
        }
        data
    };
    output["request_id"] = json!(value.request_id);
    output["mutation"] = json!(value.mutation);
    output["source_audit"] = json!(value.source_audit);
    Ok(if value.error.is_some() {
        HandlerReply::Error {
            code: "failure".into(),
            data: output,
        }
    } else {
        HandlerReply::Success(output)
    })
}

/// story:owner-read-answers-json-value: every `operations invoke` answer
/// carries the provider result as a JSON value. A read's answer comes from the
/// owner (`owner/supervisor.rs`) and passes the native result check here; a
/// write's is projected here from the owner's delivery, fresh, replayed or
/// observed alike.
#[cfg(test)]
mod json_value_tests {
    use super::*;
    use connectors_cli_contract::{
        Context, DynamicError, DynamicPhase, DynamicValidator, OutputMode, wire::Target,
    };
    use connectors_host::local::{config::Config, registry, runtime};
    use std::{collections::BTreeSet, path::PathBuf};

    const READ: &str = "item.read";
    const WRITE: &str = "item.write";

    fn output_schema() -> Value {
        json!({"type":"object","additionalProperties":false,
               "properties":{"id":{"type":"integer"}},"required":["id"]})
    }

    fn bootstrap() -> runtime::Bootstrap {
        let operation = |id: &str| connectors_core::Operation {
            id: id.into(),
            description: "fixture operation".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object"}),
            output_schema: output_schema(),
        };
        let descriptor = connectors_core::Descriptor {
            version: "v1alpha1".into(),
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            revision: "desc-1".into(),
            operations: vec![operation(READ), operation(WRITE)],
            configuration_schema: json!({"type":"object"}),
        };
        let requirement = |id: &str, effect| runtime::Requirement {
            operation: id.into(),
            profile: "token".into(),
            scopes: BTreeSet::new(),
            effect,
        };
        runtime::Bootstrap {
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            protocol: "v1alpha1".into(),
            configuration_revision: "cfg-1".into(),
            provider_authority: "https://fixture.invalid".into(),
            descriptor: serde_json::to_string(&descriptor).unwrap(),
            profiles: vec![runtime::Profile {
                id: "token".into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                scheme: "http_bearer".into(),
                capability: "http-bearer".into(),
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                fields: vec![runtime::EntryField {
                    name: "token".into(),
                    label: "Token".into(),
                    max_bytes: 1024,
                }],
                acquisition: None,
            }],
            requirements: vec![
                requirement(READ, runtime::Effect::Read),
                requirement(WRITE, runtime::Effect::Write),
            ],
        }
    }

    /// A configured adapter `forge` with a cached description of one read and
    /// one write; no adapter executable exists and none is launched.
    fn configured(root: &std::path::Path) -> (Context, String) {
        let config = root.join("config/config.toml");
        let state = root.join("state");
        let arg = |path: &PathBuf| path.as_os_str().to_owned();
        let init = super::super::run(vec![
            "connectors".into(),
            "--output".into(),
            "json".into(),
            "--config".into(),
            arg(&config),
            "--state-dir".into(),
            arg(&state),
            "setup".into(),
            "init".into(),
        ]);
        assert_eq!(init.exit_code, 0, "{}", init.stderr);
        let text = std::fs::read_to_string(&config).unwrap()
            + &format!(
                "\n[adapters.forge]\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.forge.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[adapters.forge.permissions]\nprofiles=['token']\noperations=['{READ}','{WRITE}']\n",
                "a".repeat(64)
            );
        std::fs::write(&config, text).unwrap();
        let adapter = Config::load(&config).unwrap().adapters["forge"].clone();
        let bootstrap = bootstrap();
        runtime::state::State::new(&state)
            .remember(&adapter.selection(), &bootstrap)
            .unwrap();
        let schema = owner::schema(&bootstrap, READ).unwrap();
        (
            Context {
                config: Some(config),
                state_dir: Some(state),
                output: OutputMode::Json,
            },
            schema,
        )
    }

    fn target() -> Target {
        Target::Local {
            owner: "connectors.cli".into(),
            action: "operations-invoke".into(),
        }
    }

    /// The owner read path: the native result check holds the answer's
    /// `result` to the selected output schema as a JSON value. The provider's
    /// object passes; the same object as a string holding its JSON text, which
    /// the owner answered before, is refused, so it never reaches stdout.
    #[test]
    fn a_read_answer_passes_the_native_check_only_as_a_json_value() {
        let root = tempfile::tempdir().unwrap();
        let (context, schema) = configured(root.path());
        let target = target();
        let call = Invocation {
            callable: "operations-invoke",
            target: &target,
            context,
            input: json!({"adapter":"forge","connection":"connection","operation":READ,
                          "schema":schema,"revision":"desc-1","input":"{}"}),
        };
        let mut validator =
            super::super::session::NativeValidator(super::super::session::Session::new(&[]));
        let answer = |result: Value| json!({"adapter":"forge","operation":READ,"revision":"desc-1","result":result});
        assert!(
            validator
                .validate(&call, DynamicPhase::Result, &answer(json!({"id": 7})))
                .is_ok(),
            "a JSON value result is refused"
        );
        assert!(matches!(
            validator.validate(&call, DynamicPhase::Result, &answer(json!(r#"{"id":7}"#))),
            Err(DynamicError::InvalidValue)
        ));
        assert!(matches!(
            validator.validate(&call, DynamicPhase::Result, &answer(json!({"id": "7"}))),
            Err(DynamicError::InvalidValue)
        ));
    }

    /// The owner write path: whatever delivery the owner answers (a fresh
    /// execution, a replayed original, an observed concurrent winner), the
    /// projected answer carries its result as the JSON value it holds.
    #[test]
    fn a_write_answer_projects_the_delivered_result_as_a_json_value() {
        let root = tempfile::tempdir().unwrap();
        let (context, _) = configured(root.path());
        let target = target();
        let call = Invocation {
            callable: "operations-invoke",
            target: &target,
            context,
            input: json!({"adapter":"forge","connection":"connection","operation":WRITE,
                          "revision":"desc-1","input":"{}"}),
        };
        for replayed in [false, true] {
            let delivery: owner::mutation::Delivery = serde_json::from_value(json!({
                "request_id": "00000000-0000-4000-8000-000000000001",
                "result": {"id": 7},
                "error": null,
                "mutation": {"classification":"applied","attempt":null,
                             "original_request_id":null,"replayed":replayed,"cause":null},
                "source_audit": {"instance":"forge-local","audit_ref":null,
                                 "audit_status":"complete"}
            }))
            .unwrap();
            let HandlerReply::Success(answer) = project(&call, &bootstrap(), delivery).unwrap()
            else {
                panic!("a delivered result is not a success");
            };
            assert_eq!(answer["result"], json!({"id": 7}), "{answer}");
            assert_eq!(answer["mutation"]["replayed"], replayed);
        }
    }
}
