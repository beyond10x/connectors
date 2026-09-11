//! Version-two mutation exchange. Version-one enums remain closed.
use super::*;
use connectors_sdk::WriteOutcome;
use serde_json::Value;
use std::{os::unix::net::UnixStream, time::Instant};

/// Trusted executable composition retains the immutable native request and its
/// command-local credential. Preparation may read; only consumption may write.
/// There is no Clone, serialization, replacement input or retry operation.
#[async_trait::async_trait]
pub trait PreparedWrite: Send {
    async fn execute(self: Box<Self>) -> WriteOutcome<Value>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteEffect {
    Applied,
    Refused,
    Unknown,
}

/// Effect evidence survives failure to validate or disclose the safe result.
/// A lost or uncorrelated reply supplies only Unknown, never permission to retry.
#[derive(Debug)]
pub struct WriteResult {
    pub effect: WriteEffect,
    pub result: Result<Value>,
}
impl WriteResult {
    pub(super) fn unknown(code: Failure) -> Self {
        Self {
            effect: WriteEffect::Unknown,
            result: Err(code),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum RequestV2 {
    Legacy(Request),
    Write(WriteRequest),
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum WriteRequest {
    #[serde(rename = "prepare_write")]
    Prepare {
        id: String,
        operation: String,
        revision: String,
        partition: String,
        deadline_ms: u64,
    },
    #[serde(rename = "commit_write")]
    Commit { id: String, preparation_id: String },
    #[serde(rename = "cancel_write")]
    Cancel { id: String, preparation_id: String },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum WriteReply {
    PreparedWrite { id: String, preparation_id: String },
    CancelledWrite { id: String, preparation_id: String },
    WriteResult { id: String, effect: WriteEffect },
    Failed { request_id: String, code: Failure },
}

// Exactly one bounded document follows WriteResult, including on failure. No
// provider error text, credentials or private request material enter this codec.
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ResultDocument {
    Success { value: Value },
    Failure { code: Failure },
}

pub(super) fn canonical_id(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| !id.is_nil() && id.to_string() == value)
}
pub(super) fn budget(deadline_ms: u64) -> Result<Instant> {
    let ms = deadline_ms
        .checked_sub(connectors_sdk::now_ms())
        .filter(|ms| (1..=120_000).contains(ms))
        .ok_or(Failure::Timeout)?;
    Ok(Instant::now() + std::time::Duration::from_millis(ms))
}
pub(super) fn remaining(until: Instant, deadline_ms: u64) -> Result<()> {
    if Instant::now() >= until || connectors_sdk::now_ms() >= deadline_ms {
        Err(Failure::Timeout)
    } else {
        Ok(())
    }
}

pub(super) fn serve_write(
    channel: &mut UnixStream,
    executor: &tokio::runtime::Runtime,
    adapter: &impl Adapter,
    bootstrap: &Bootstrap,
    frame: channel::Frame<WriteRequest>,
) -> Result<()> {
    let WriteRequest::Prepare {
        id,
        operation,
        revision,
        partition,
        deadline_ms,
    } = frame.control
    else {
        // Includes a second commit, even after a successful first result.
        return Err(Failure::Protocol);
    };
    if !canonical_id(&id) || frame.secret.0.is_empty() {
        return Err(Failure::Protocol);
    }
    let until = budget(deadline_ms)?;
    let prepared = executor.block_on(async {
        tokio::time::timeout_at(until.into(), async {
            let descriptor = bootstrap.descriptor()?;
            let declaration =
                write_declaration(bootstrap, &descriptor, &operation, &revision, &partition)?;
            channel::depth(&frame.document)?;
            let input =
                connectors_core::read_json(&frame.document).map_err(|_| Failure::InvalidInput)?;
            connectors_sdk::validate_write_value(&declaration.input_schema, &input)
                .map_err(Failure::from_service)?;
            let pending = adapter
                .prepare_write(&operation, &partition, frame.secret, input)
                .await?;
            Ok((pending, declaration.output_schema.clone()))
        })
        .await
        .map_err(|_| Failure::Timeout)?
    });
    let (pending, output_schema) = match prepared {
        Ok(prepared) => prepared,
        Err(code) => {
            return channel::write(
                channel,
                &Reply::Failed {
                    request_id: id,
                    code,
                },
                None,
                &[],
                until,
            );
        }
    };
    remaining(until, deadline_ms)?;
    let preparation_id = uuid::Uuid::new_v4().to_string();
    channel::write(
        channel,
        &WriteReply::PreparedWrite {
            id: id.clone(),
            preparation_id: preparation_id.clone(),
        },
        None,
        &[],
        until,
    )?;

    // This stack frame owns the sole pending value. EOF, malformed/replacement
    // frames, cancellation and expiry destroy it. No idle wait resets the budget.
    let command = channel::read::<RequestV2>(channel, until, false, 0)?;
    remaining(until, deadline_ms)?;
    match command.control {
        RequestV2::Write(WriteRequest::Cancel {
            id: returned,
            preparation_id: returned_preparation,
        }) if returned == id && returned_preparation == preparation_id => {
            drop(pending);
            channel::write(
                channel,
                &WriteReply::CancelledWrite { id, preparation_id },
                None,
                &[],
                until,
            )
        }
        RequestV2::Write(WriteRequest::Commit {
            id: returned,
            preparation_id: returned_preparation,
        }) if returned == id && returned_preparation == preparation_id => {
            // Ownership moves before any write. Timeout drops the future;
            // neither this exchange nor the host can recreate its authority.
            let outcome = executor.block_on(tokio_timeout(until, pending));
            let (effect, result) = match outcome {
                Ok(WriteOutcome::Applied(value)) => {
                    (WriteEffect::Applied, value.map_err(Failure::from_provider))
                }
                Ok(WriteOutcome::Refused(error)) => {
                    (WriteEffect::Refused, Err(Failure::from_provider(error)))
                }
                Ok(WriteOutcome::Unknown(error)) => {
                    (WriteEffect::Unknown, Err(Failure::from_provider(error)))
                }
                Err(error) => (WriteEffect::Unknown, Err(error)),
            };
            let result = result.and_then(|value| {
                connectors_sdk::validate_write_value(&output_schema, &value)
                    .map_err(|_| Failure::Protocol)?;
                Ok(value)
            });
            let document = encode_result(result)?;
            channel::write(
                channel,
                &WriteReply::WriteResult { id, effect },
                None,
                &document,
                until,
            )
        }
        _ => Err(Failure::Protocol),
    }
}

async fn tokio_timeout(
    until: Instant,
    pending: Box<dyn PreparedWrite>,
) -> Result<WriteOutcome<Value>> {
    tokio::time::timeout_at(until.into(), pending.execute())
        .await
        .map_err(|_| Failure::Timeout)
}

pub(super) fn write_declaration<'a>(
    bootstrap: &Bootstrap,
    descriptor: &'a Descriptor,
    operation: &str,
    revision: &str,
    partition: &str,
) -> Result<&'a connectors_core::Operation> {
    if !connectors_core::valid_id(partition) {
        return Err(Failure::InvalidInput);
    }
    if descriptor.revision != revision {
        return Err(Failure::StaleDescription);
    }
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|r| r.operation == operation)
        .ok_or(Failure::NotFound)?;
    if requirement.effect != Effect::Write {
        return Err(Failure::Unsupported);
    }
    descriptor
        .operation(operation)
        .map_err(Failure::from_service)
}

fn encode_result(result: Result<Value>) -> Result<Vec<u8>> {
    let document = match result {
        Ok(value) => ResultDocument::Success { value },
        Err(code) => ResultDocument::Failure { code },
    };
    let bytes = serde_json::to_vec(&document).map_err(|_| Failure::Protocol)?;
    if bytes.len() > RESULT_LIMIT || channel::depth(&bytes).is_err() {
        serde_json::to_vec(&ResultDocument::Failure {
            code: Failure::Capacity,
        })
        .map_err(|_| Failure::Protocol)
    } else {
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_codecs_refuse_new_variants_and_result_documents_remain_closed_and_bounded() {
        for bytes in [
            br#"{"kind":"prepare_write","id":"id","operation":"write","revision":"rev","partition":"partition","deadline_ms":1}"#.as_slice(),
            br#"{"kind":"commit_write","id":"id","preparation_id":"preparation"}"#,
            br#"{"kind":"cancel_write","id":"id","preparation_id":"preparation"}"#,
        ] {
            assert!(connectors_core::read_json::<Request>(bytes).is_err());
            assert!(connectors_core::read_json::<RequestV2>(bytes).is_ok());
        }
        assert!(
            connectors_core::read_json::<Reply>(
                br#"{"kind":"prepared_write","id":"id","preparation_id":"preparation"}"#
            )
            .is_err()
        );
        for bytes in [
            br#"{"kind":"commit_write","id":"id","preparation_id":"preparation","deadline_ms":2}"#.as_slice(),
            br#"{"kind":"commit_write","id":"id","id":"different","preparation_id":"preparation"}"#,
            br#"{"kind":"invoke","request_id":"id","operation":"read","revision":"revision","partition":"partition","deadline_ms":1,"approval":"extra"}"#,
        ] {
            assert!(connectors_core::read_json::<RequestV2>(bytes).is_err());
        }
        let bytes = encode_result(Ok(Value::String("x".repeat(RESULT_LIMIT)))).unwrap();
        assert!(matches!(
            connectors_core::read_json::<ResultDocument>(&bytes).unwrap(),
            ResultDocument::Failure {
                code: Failure::Capacity
            }
        ));
        assert!(bytes.len() < 100);
        assert!(canonical_id(&uuid::Uuid::new_v4().to_string()));
        for invalid in [
            uuid::Uuid::nil().to_string(),
            uuid::Uuid::new_v4().simple().to_string(),
            "not-a-uuid".into(),
        ] {
            assert!(!canonical_id(&invalid));
        }
    }
}
