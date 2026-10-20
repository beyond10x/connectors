use connectors_cli_contract::Invocation;
use connectors_host::local::{
    config::Paths,
    owner::{self, Argument, Code, approval_issuance as issuance},
    protected,
};
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

pub(super) fn execute(call: &Invocation<'_>, until: Instant) -> owner::Result<Value> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )?;
    // Every refusal of an argument names it, as the CLI input model does.
    let text = |argument: Argument| {
        call.input[argument.as_str()]
            .as_str()
            .ok_or_else(|| owner::Error::invalid_input(argument))
    };
    let alias = text(Argument::Adapter)?;
    if call.callable == "approval-policy-status" {
        return Ok(json!({"adapter":alias,"policy":issuance::policy_status(&paths, alias)?}));
    }
    if call.callable == "approval-policy-set" {
        let document = protected::document_until(
            Some(Path::new(text(Argument::InputFile)?)),
            until,
            issuance::REQUEST_LIMIT,
        )
        .map_err(|mut error| {
            if error.code == Code::InvalidInput && error.field.is_none() {
                error.field = Some(Argument::InputFile);
            }
            error
        })?;
        let revision = call
            .input
            .get(Argument::ExpectedRevision.as_str())
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_i64()
                    .ok_or_else(|| owner::Error::invalid_input(Argument::ExpectedRevision))
            })
            .transpose()?;
        return Ok(
            json!({"adapter":alias,"policy":issuance::policy_set(&paths, alias, document.as_bytes(), revision, until)?}),
        );
    }
    let request = issuance::Request {
        connection: text(Argument::Connection)?,
        operation: text(Argument::Operation)?,
        schema: text(Argument::Schema)?,
        revision: text(Argument::Revision)?,
        input: text(Argument::Input)?,
    };
    match call.callable {
        "approval-prepare" => Ok(
            json!({"adapter":alias,"preparation":issuance::prepare(&paths, alias, &request, until)?}),
        ),
        "approval-issue" => {
            let publication = issuance::issue(
                &paths,
                alias,
                &request,
                text(Argument::ApproveSubject)?,
                Path::new(text(Argument::ProofOutput)?),
                until,
            )?;
            Ok(
                json!({"adapter":alias,"reference":publication.reference,"subject_sha256":publication.subject_sha256,"disposition":publication.disposition}),
            )
        }
        _ => Err(Code::Unsupported.into()),
    }
}
