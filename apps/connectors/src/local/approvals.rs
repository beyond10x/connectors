use connectors_cli_contract::Invocation;
use connectors_host::local::{
    config::Paths,
    owner::{self, Code, approval_issuance as issuance},
    protected,
};
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

pub(super) fn execute(call: &Invocation<'_>, until: Instant) -> owner::Result<Value> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )?;
    let text = |field: &str| call.input[field].as_str().ok_or(Code::InvalidInput);
    let alias = text("adapter")?;
    if call.callable == "approval-policy-status" {
        return Ok(json!({"adapter":alias,"policy":issuance::policy_status(&paths, alias)?}));
    }
    if call.callable == "approval-policy-set" {
        let document = protected::document_until(
            Some(Path::new(text("input_file")?)),
            until,
            issuance::REQUEST_LIMIT,
        )?;
        let revision = call
            .input
            .get("expected_revision")
            .filter(|v| !v.is_null())
            .map(|v| v.as_i64().ok_or(Code::InvalidInput))
            .transpose()?;
        return Ok(
            json!({"adapter":alias,"policy":issuance::policy_set(&paths, alias, document.as_bytes(), revision, until)?}),
        );
    }
    let request = issuance::Request {
        connection: text("connection")?,
        operation: text("operation")?,
        schema: text("schema")?,
        revision: text("revision")?,
        input: text("input")?,
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
                text("approve_subject")?,
                Path::new(text("proof_output")?),
                until,
            )?;
            Ok(
                json!({"adapter":alias,"reference":publication.reference,"subject_sha256":publication.subject_sha256,"disposition":publication.disposition}),
            )
        }
        _ => Err(Code::Unsupported.into()),
    }
}
