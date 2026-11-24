use connectors_cli_contract::Invocation;
use connectors_host::local::{
    config::Paths,
    owner::{self, Argument, Code, approval_issuance as issuance},
    protected, standing,
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
    // The three standing-grant commands (contracts/cli/v1alpha1/semantics.md,
    // "Standing grants"): they read and write the instance's signed grant
    // file in this process and start no owner.
    if call.callable == "approval-grant-standing" {
        let document = protected::document_until(
            Some(Path::new(text(Argument::InputFile)?)),
            until,
            standing::FILE_LIMIT,
        )
        .map_err(|mut error| {
            if error.code == Code::InvalidInput && error.field.is_none() {
                error.field = Some(Argument::InputFile);
            }
            error
        })?;
        let granted = issuance::grant_standing(&paths, alias, document.as_bytes(), until)?;
        return Ok(json!({"grant_id":granted.grant_id,"path":granted.path}));
    }
    if call.callable == "approval-revoke-standing" {
        // The presentation requires the positional; no other value is refused
        // as input: an id the file does not hold is `not_found`.
        let grant_id = call.input["grant_id"].as_str().ok_or(Code::InvalidInput)?;
        let revoked = issuance::revoke_standing(&paths, alias, grant_id, until)?;
        return Ok(json!({"grant_id":revoked.grant_id,"path":revoked.path}));
    }
    if call.callable == "approval-list-standing" {
        let grants: Vec<Value> = issuance::list_standing(&paths, alias, until)?
            .into_iter()
            .map(standing_view)
            .collect();
        return Ok(json!({"grants":grants}));
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

/// One grant as `approvals list-standing` shows it.
fn standing_view(grant: standing::Grant) -> Value {
    // A pattern member is shown with `max_value_bytes`.
    let target: Vec<Value> = grant.target.iter().map(|member| member.listed()).collect();
    let mut view = json!({
        "grant_id": grant.id,
        "operation": grant.operation,
        "target": target,
        "not_after_unix_ms": grant.not_after_unix_ms,
        "max_per_hour": grant.max_per_hour,
    });
    // Absent for a grant that leaves the rest of its input open.
    if let Some(keys) = grant.input_keys {
        view["input_keys"] = json!(keys);
    }
    view
}

#[cfg(test)]
mod tests {
    use super::*;

    /// contracts/cli/v1alpha1/semantics.md, "Standing grants": the answer of
    /// `approvals list-standing` shows each target member with the key it was
    /// written with, and a pattern member also with `max_value_bytes`, which
    /// the file does not hold.
    #[test]
    fn a_listed_standing_grant_shows_max_value_bytes_on_its_pattern_member_only() {
        let target = json!([
            {"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$"},
            {"pointer": "/project", "value": "DEV"},
            {"pointer": "/target_status", "one_of": ["10000", "10235"]}
        ]);
        let file = json!({
            "format": standing::FORMAT,
            "grants": [{
                "id": "g1", "instance": "leaf", "operation": "issue.transition.run",
                "target": target, "input_keys": ["/body"],
                "not_after_unix_ms": 2_000_000_000_000_i64, "max_per_hour": 3
            }],
            "revoked": []
        });
        let grants = standing::parse(&serde_json::to_vec(&file).unwrap()).unwrap();
        assert_eq!(
            standing_view(grants.grants()[0].clone()),
            json!({
                "grant_id": "g1",
                "operation": "issue.transition.run",
                "target": [
                    {"pointer": "/issueIdOrKey", "pattern": "^DEV-[0-9]+$", "max_value_bytes": 22055},
                    {"pointer": "/project", "value": "DEV"},
                    {"pointer": "/target_status", "one_of": ["10000", "10235"]}
                ],
                "input_keys": ["/body"],
                "not_after_unix_ms": 2_000_000_000_000_i64,
                "max_per_hour": 3
            })
        );
    }
}
