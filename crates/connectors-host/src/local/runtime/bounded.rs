//! Explicit native read budgets; no admission authority is conveyed.
use super::{Failure, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadBudget {
    deadline_ticks: u64,
    execution_ms: u64,
    provider_ms: u64,
    input_bytes: usize,
    result_bytes: usize,
}
impl ReadBudget {
    pub fn start(
        execution_ms: u64,
        provider_ms: u64,
        input_bytes: usize,
        result_bytes: usize,
    ) -> Result<Self> {
        let mut value = Self {
            deadline_ticks: 0,
            execution_ms,
            provider_ms,
            input_bytes,
            result_bytes,
        };
        value.validate()?;
        value.deadline_ticks = ticks()?
            .checked_add(execution_ms * 1_000_000)
            .ok_or(Failure::Unavailable)?;
        Ok(value)
    }
    fn validate(self) -> Result<()> {
        if !(1..=40_000).contains(&self.execution_ms)
            || !(1..=30_000).contains(&self.provider_ms)
            || self.provider_ms > self.execution_ms
            || !(1..=262_144).contains(&self.input_bytes)
            || !(1..=4_194_304).contains(&self.result_bytes)
        {
            return Err(Failure::InvalidInput);
        }
        Ok(())
    }
    pub fn until(self) -> Result<Instant> {
        self.validate()?;
        // Sample Instant first: conversion can only shorten, never extend.
        let start = Instant::now();
        let left = self
            .deadline_ticks
            .checked_sub(ticks()?)
            .filter(|left| (1..=self.execution_ms * 1_000_000).contains(left))
            .ok_or(Failure::Timeout)?;
        start
            .checked_add(Duration::from_nanos(left))
            .ok_or(Failure::Timeout)
    }
    /// Call once at the start of a native provider sequence; share the returned
    /// cutoff with every HTTP capability, including the token-exchange client.
    pub fn provider_until(self) -> Result<Instant> {
        Ok(self
            .until()?
            .min(Instant::now() + Duration::from_millis(self.provider_ms)))
    }
    pub fn input_bytes(self) -> usize {
        self.input_bytes
    }
    pub fn result_bytes(self) -> usize {
        self.result_bytes
    }
}

fn ticks() -> Result<u64> {
    let mut stamp = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: the syscall receives a valid timespec of the declared size.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut stamp) } != 0
        || stamp.tv_sec < 0
        || !(0..1_000_000_000).contains(&stamp.tv_nsec)
    {
        return Err(Failure::Unavailable);
    }
    (stamp.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(stamp.tv_nsec as u64))
        .ok_or(Failure::Unavailable)
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum RequestV3 {
    Bounded(ReadRequest),
    Legacy(super::Request),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ReadRequest {
    InvokeBounded {
        request_id: String,
        operation: String,
        revision: String,
        partition: String,
        budget: ReadBudget,
    },
}

pub(super) fn serve_read(
    channel: &mut std::os::unix::net::UnixStream,
    executor: &tokio::runtime::Runtime,
    adapter: &impl super::Adapter,
    bootstrap: &super::Bootstrap,
    frame: super::channel::Frame<ReadRequest>,
) -> Result<()> {
    use super::{Effect, Reply, channel};
    let ReadRequest::InvokeBounded {
        request_id,
        operation,
        revision,
        partition,
        budget,
    } = frame.control;
    if !super::writes::canonical_id(&request_id) || frame.secret.0.is_empty() {
        return Err(Failure::Protocol);
    }
    let until = budget.until()?;
    let result = executor.block_on(async {
        tokio::time::timeout_at(until.into(), async {
            if !connectors_core::valid_id(&partition) || frame.document.len() > budget.input_bytes()
            {
                return Err(Failure::InvalidInput);
            }
            let descriptor = bootstrap.descriptor()?;
            if descriptor.revision != revision {
                return Err(Failure::StaleDescription);
            }
            let requirement = bootstrap
                .requirements
                .iter()
                .find(|r| r.operation == operation)
                .ok_or(Failure::NotFound)?;
            if requirement.effect != Effect::Read {
                return Err(Failure::Unsupported);
            }
            let declaration = descriptor
                .operation(&operation)
                .map_err(Failure::from_service)?;
            let input = connectors_core::json::decode(&frame.document, 64)
                .map_err(|_| Failure::InvalidInput)?;
            connectors_sdk::validate(&declaration.input_schema, &input)
                .map_err(Failure::from_service)?;
            budget.until()?;
            let value = adapter
                .invoke_bounded(&operation, &partition, frame.secret, input, budget)
                .await?;
            connectors_sdk::validate(&declaration.output_schema, &value)
                .map_err(|_| Failure::Protocol)?;
            let bytes = serde_json::to_vec(&value).map_err(|_| Failure::Protocol)?;
            if bytes.len() > budget.result_bytes() {
                return Err(Failure::Capacity);
            }
            budget.until()?;
            Ok(bytes)
        })
        .await
        .map_err(|_| Failure::Timeout)?
    });
    let (reply, document) = match result {
        Ok(bytes) => (Reply::Success { request_id }, bytes),
        Err(code) => (Reply::Failed { request_id, code }, Vec::new()),
    };
    channel::write(channel, &reply, None, &document, until)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_limits_are_checked_without_clipping() {
        let selected = ReadBudget::start(40_000, 30_000, 262_144, 4_194_304).unwrap();
        assert_eq!(selected.input_bytes(), 262_144);
        assert_eq!(selected.result_bytes(), 4_194_304);
        for (execution, provider, input, result) in [
            (0, 1, 1, 1),
            (40_001, 1, 1, 1),
            (1, 0, 1, 1),
            (40_000, 30_001, 1, 1),
            (10, 11, 1, 1),
            (1, 1, 0, 1),
            (1, 1, 262_145, 1),
            (1, 1, 1, 0),
            (1, 1, 1, 4_194_305),
        ] {
            assert_eq!(
                ReadBudget::start(execution, provider, input, result).unwrap_err(),
                Failure::InvalidInput
            );
        }
    }
    #[test]
    fn serialized_deadline_does_not_restart_and_expiry_refuses() {
        let selected = ReadBudget::start(5000, 2500, 100, 100).unwrap();
        let original = selected.until().unwrap();
        std::thread::sleep(Duration::from_millis(30));
        let mut restored: ReadBudget =
            serde_json::from_slice(&serde_json::to_vec(&selected).unwrap()).unwrap();
        assert!(restored.until().unwrap() <= original + Duration::from_millis(1));
        let provider = restored.provider_until().unwrap();
        assert!(provider <= original && provider > Instant::now());
        restored.deadline_ticks = 0;
        assert_eq!(restored.until().unwrap_err(), Failure::Timeout);
        assert_eq!(restored.provider_until().unwrap_err(), Failure::Timeout);
    }
    #[test]
    fn decoded_forgery_cannot_bypass_budget_validation() {
        let mut value =
            serde_json::to_value(ReadBudget::start(100, 50, 100, 100).unwrap()).unwrap();
        value["execution_ms"] = serde_json::json!(40_001);
        let invalid: ReadBudget = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(invalid.until().unwrap_err(), Failure::InvalidInput);
        value["execution_ms"] = serde_json::json!(100);
        value["deadline_ticks"] = serde_json::json!(u64::MAX);
        let invalid: ReadBudget = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(invalid.until().unwrap_err(), Failure::Timeout);
        value["unreviewed"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ReadBudget>(value).is_err());
    }
}
