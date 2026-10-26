//! The refresh ledger (`ess/domains/refresh.yaml`, story:owner-rotating-refresh).
//!
//! One `connectors.refresh.RefreshAttempt` per attempt, recorded and never
//! projected to SQL, like the store writer marker. The registry reads the
//! attempts before its transaction and stages each transition beside the SQL
//! change it commits with; the persist that records that change records the
//! staged transitions in the same batch, so both commit together or neither
//! does. A custody version a refresh stored names its attempt as its
//! acquisition (`refresh:<attempt>`): `registry_materials` requires an
//! acquisition row, which [`insert_acquisition`] supplies in SQL only.
use super::{
    Baseline, CommandEntry, ErAuthority, Failure, Result, RowImage, command_action, create_action,
};
use entity_core::Registry;
use entity_executor::BatchAction;
use rusqlite::Connection;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

pub(in crate::local) const ENTITY: &str = "connectors.refresh.RefreshAttempt";
/// The acquisition reference of a custody version a refresh stored, before the
/// attempt's id.
pub(in crate::local) const ACQUISITION: &str = "refresh:";

pub(super) fn key(key: &(String, String)) -> bool {
    key.0 == ENTITY
}

/// `connectors.refresh.RefreshAttempt.State`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptState {
    Reserved,
    Authorized,
    ResponseStored,
    Recovering,
    Published,
    Fenced,
    Uncertain,
    Released,
    Discarded,
}

impl AttemptState {
    fn parse(state: &str) -> Result<Self> {
        Ok(match state {
            "Reserved" => Self::Reserved,
            "Authorized" => Self::Authorized,
            "ResponseStored" => Self::ResponseStored,
            "Recovering" => Self::Recovering,
            "Published" => Self::Published,
            "Fenced" => Self::Fenced,
            "Uncertain" => Self::Uncertain,
            "Released" => Self::Released,
            "Discarded" => Self::Discarded,
            _ => return Err(Failure::MetadataUnavailable),
        })
    }
    fn name(self) -> &'static str {
        match self {
            Self::Reserved => "Reserved",
            Self::Authorized => "Authorized",
            Self::ResponseStored => "ResponseStored",
            Self::Recovering => "Recovering",
            Self::Published => "Published",
            Self::Fenced => "Fenced",
            Self::Uncertain => "Uncertain",
            Self::Released => "Released",
            Self::Discarded => "Discarded",
        }
    }
    /// No transition leaves it.
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Published | Self::Fenced | Self::Uncertain | Self::Discarded | Self::Released
        )
    }
}

/// One recorded attempt. `owner` and `token` are the initial owner's; the
/// model records no owner a recovery names (`ess/domains/refresh.yaml`,
/// ESS-LIMIT), so a recovery reserves and publishes in one batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub id: String,
    pub state: AttemptState,
    pub source_generation: String,
    pub owner: String,
    pub token: String,
    /// The connection's private publication fence the attempt expects.
    pub expected_fence: String,
    pub candidate_generation: Option<String>,
}

/// One transition the registry stages, named by its `connectors.refresh` command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Reserve {
        attempt: String,
        source_generation: String,
        owner: String,
        token: String,
        expected_fence: String,
    },
    Authorize {
        attempt: String,
        owner: String,
        token: String,
    },
    Fence {
        attempt: String,
    },
    Store {
        attempt: String,
        owner: String,
        token: String,
        candidate_generation: String,
    },
    Quarantine {
        attempt: String,
    },
    Release {
        attempt: String,
        owner: String,
        token: String,
    },
    Recover {
        attempt: String,
        owner: String,
        token: String,
    },
    Publish {
        attempt: String,
        owner: String,
        token: String,
        candidate_generation: String,
    },
    Discard {
        attempt: String,
    },
}

impl Step {
    pub fn attempt(&self) -> &str {
        match self {
            Self::Reserve { attempt, .. }
            | Self::Authorize { attempt, .. }
            | Self::Fence { attempt }
            | Self::Store { attempt, .. }
            | Self::Quarantine { attempt }
            | Self::Release { attempt, .. }
            | Self::Recover { attempt, .. }
            | Self::Publish { attempt, .. }
            | Self::Discard { attempt } => attempt,
        }
    }

    /// The command, the states it moves from and the state it moves to.
    fn transition(&self) -> (&'static str, &'static [AttemptState], AttemptState) {
        use AttemptState as S;
        match self {
            Self::Reserve { .. } => ("connectors.refresh.ReserveRefresh", &[], S::Reserved),
            Self::Authorize { .. } => (
                "connectors.refresh.AuthorizeExchange",
                &[S::Reserved],
                S::Authorized,
            ),
            Self::Fence { .. } => (
                "connectors.refresh.FenceReserved",
                &[S::Reserved],
                S::Fenced,
            ),
            Self::Store { .. } => (
                "connectors.refresh.StoreResponse",
                &[S::Authorized],
                S::ResponseStored,
            ),
            Self::Quarantine { .. } => (
                "connectors.refresh.QuarantineRefresh",
                &[S::Authorized],
                S::Uncertain,
            ),
            Self::Release { .. } => (
                "connectors.refresh.ReleaseRateLimited",
                &[S::Authorized],
                S::Released,
            ),
            Self::Recover { .. } => (
                "connectors.refresh.RecoverStoredResponse",
                &[S::ResponseStored],
                S::Recovering,
            ),
            Self::Publish { .. } => (
                "connectors.refresh.PublishRefresh",
                &[S::ResponseStored, S::Recovering],
                S::Published,
            ),
            Self::Discard { .. } => (
                "connectors.refresh.DiscardCandidate",
                &[S::ResponseStored, S::Recovering],
                S::Discarded,
            ),
        }
    }

    /// The command input this step supplies beyond the record's own fields.
    fn input(&self) -> Map<String, Value> {
        let mut input = Map::new();
        match self {
            Self::Reserve { owner, token, .. }
            | Self::Authorize { owner, token, .. }
            | Self::Store { owner, token, .. }
            | Self::Release { owner, token, .. }
            | Self::Publish { owner, token, .. } => {
                input.insert("owner".into(), json!(owner));
                input.insert("ownership_token".into(), json!(token));
            }
            Self::Recover { owner, token, .. } => {
                input.insert("new_owner".into(), json!(owner));
                input.insert("new_ownership_token".into(), json!(token));
            }
            Self::Fence { .. } | Self::Quarantine { .. } | Self::Discard { .. } => {}
        }
        input
    }
}

/// Every attempt `er` accounts for.
pub(in crate::local::metadata) fn attempts(er: &ErAuthority) -> Result<Vec<Attempt>> {
    er.baseline
        .values()
        .filter(|row| row.entity == ENTITY)
        .map(attempt)
        .collect()
}

fn attempt(row: &RowImage) -> Result<Attempt> {
    let text = |name: &str| {
        row.fields
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(Failure::MetadataUnavailable)
    };
    Ok(Attempt {
        id: row
            .id
            .strip_prefix("s:")
            .ok_or(Failure::MetadataUnavailable)?
            .to_owned(),
        state: AttemptState::parse(&row.lifecycle_state)?,
        source_generation: text("source_generation_id")?,
        owner: text("initial_owner")?,
        token: text("initial_ownership_token")?,
        expected_fence: text("expected_binding_revision")?,
        candidate_generation: text("candidate_generation_id").ok(),
    })
}

/// The batch actions recording `steps`, in order, against `baseline`. Steps
/// on one attempt chain: each names the revision the one before it produced.
pub(super) fn actions(
    baseline: &Baseline,
    registry: &Registry,
    commands: &BTreeMap<String, CommandEntry>,
    steps: &[Step],
) -> Result<Vec<BatchAction>> {
    let mut current: BTreeMap<String, RowImage> = BTreeMap::new();
    let mut actions = Vec::new();
    for step in steps {
        let id = format!("s:{}", step.attempt());
        let before = current
            .get(step.attempt())
            .or_else(|| baseline.get(&(ENTITY.to_owned(), id.clone())))
            .cloned();
        let (command, from, to) = step.transition();
        let next = match (step, before) {
            (
                Step::Reserve {
                    source_generation,
                    owner,
                    token,
                    expected_fence,
                    ..
                },
                None,
            ) => {
                let mut fields = Map::new();
                fields.insert("attempt_id".into(), json!(step.attempt()));
                fields.insert("source_generation_id".into(), json!(source_generation));
                fields.insert("initial_owner".into(), json!(owner));
                fields.insert("initial_ownership_token".into(), json!(token));
                fields.insert("expected_binding_revision".into(), json!(expected_fence));
                let next = RowImage {
                    entity: ENTITY.into(),
                    id,
                    revision: 1,
                    lifecycle_state: to.name().into(),
                    fields,
                };
                let mut carrier = next.clone();
                carrier.fields.extend(step.input());
                actions.push(create_action(registry, commands, &carrier, command)?);
                next
            }
            (Step::Reserve { .. }, Some(_)) | (_, None) => {
                return Err(Failure::MetadataUnavailable);
            }
            (step, Some(before)) => {
                if !from.contains(&AttemptState::parse(&before.lifecycle_state)?) {
                    return Err(Failure::MetadataUnavailable);
                }
                let mut next = before.clone();
                next.revision = before.revision + 1;
                next.lifecycle_state = to.name().into();
                if let Step::Store {
                    candidate_generation,
                    ..
                }
                | Step::Publish {
                    candidate_generation,
                    ..
                } = step
                {
                    if matches!(step, Step::Publish { .. })
                        && next.fields.get("candidate_generation_id")
                            != Some(&json!(candidate_generation))
                    {
                        return Err(Failure::MetadataUnavailable);
                    }
                    next.fields.insert(
                        "candidate_generation_id".into(),
                        json!(candidate_generation),
                    );
                }
                let mut carrier = next.clone();
                carrier.fields.extend(step.input());
                actions.push(command_action(
                    registry, commands, baseline, &before, &carrier, command,
                )?);
                next
            }
        };
        current.insert(step.attempt().to_owned(), next);
    }
    Ok(actions)
}

/// The `registry_acquisitions` row of the custody version a refresh stored:
/// completed, owned by nothing, expiring at once. Its values follow from the
/// version alone, so a projection rebuilt from the recorded versions inserts
/// exactly the row the registry inserted.
pub(in crate::local) fn insert_acquisition(
    connection: &Connection,
    acquisition: &str,
    connection_ref: &str,
    generation: &str,
) -> Result<()> {
    if !acquisition.starts_with(ACQUISITION) {
        return Err(Failure::MetadataUnavailable);
    }
    connection
        .execute(
            "INSERT INTO registry_acquisitions (acquisition_ref,connection_ref,owner_token,expected_fence,state,created_at_ms,expires_at_ms,generation_id,capture_id) VALUES (?1,?2,?1,?1,'completed',1,2,?3,?1)",
            rusqlite::params![acquisition, connection_ref, generation],
        )
        .map_err(super::unavailable)?;
    Ok(())
}

/// Whether `table`'s captured row is a refresh's acquisition row, which is
/// never recorded.
pub(super) fn acquisition_row(table: &str, fields: &Map<String, Value>) -> bool {
    table == "registry_acquisitions"
        && fields
            .get("acquisition_ref")
            .and_then(Value::as_str)
            .is_some_and(|reference| reference.starts_with(ACQUISITION))
}

/// Inserts the acquisition row of every recorded custody version a refresh stored.
pub(super) fn project_acquisitions(connection: &Connection, baseline: &Baseline) -> Result<()> {
    for row in baseline
        .values()
        .filter(|row| row.entity == "connectors.auth_bindings.CustodyVersion")
    {
        let text = |name: &str| {
            row.fields
                .get(name)
                .and_then(Value::as_str)
                .ok_or(Failure::MetadataUnavailable)
        };
        let acquisition = text("acquisition_ref")?;
        if acquisition.starts_with(ACQUISITION) {
            insert_acquisition(
                connection,
                acquisition,
                text("connection_ref")?,
                text("generation_id")?,
            )?;
        }
    }
    Ok(())
}

/// Each attempt's recorded commands in order, read from its subject history:
/// the command name its recording carries (`connectors-<command>-<uuid>`) and,
/// for an operation, the operation the decision executed.
#[cfg(test)]
pub(in crate::local::metadata) fn recorded_commands(
    er: &ErAuthority,
) -> Result<BTreeMap<String, Vec<String>>> {
    use entity_core::DecisionCommand;
    use entity_store::asynchronous::RecordedEntry;
    let mut commands = BTreeMap::new();
    for (entity, id) in er.baseline.keys().filter(|subject| key(subject)) {
        let history = super::read_history(&er.facade, entity, id)?;
        let mut names = Vec::new();
        for record in &history.records {
            let RecordedEntry::Decision(commit) = &record.entry else {
                continue;
            };
            let label = commit
                .envelope
                .record_id
                .strip_prefix("connectors-")
                .and_then(|rest| rest.get(..rest.len().checked_sub(37)?))
                .ok_or(Failure::MetadataUnavailable)?
                .to_owned();
            match &commit.envelope.record.command {
                DecisionCommand::Execute { operation, .. } if *operation != label => {
                    return Err(Failure::MetadataUnavailable);
                }
                _ => {}
            }
            names.push(label);
        }
        commands.insert(
            id.strip_prefix("s:")
                .ok_or(Failure::MetadataUnavailable)?
                .to_owned(),
            names,
        );
    }
    Ok(commands)
}
