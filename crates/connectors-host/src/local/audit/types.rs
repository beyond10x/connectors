use super::{Failure, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TAG: &[u8] = b"connectors.execution-audit-record/v1";

/// Only this pair and compatibility status may be projected publicly. The
/// audit owner allocates audit_ref; request input is never its source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub instance: String,
    pub audit_ref: String,
}
impl Reference {
    /// Private storage identity, not a public audit id or diagnostic field.
    pub fn encode_private(&self) -> Result<String> {
        bounded(&self.instance, 128)?;
        bounded(&self.audit_ref, 128)?;
        let mut bytes = TAG.to_vec();
        for value in [&self.instance, &self.audit_ref] {
            bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
            bytes.extend_from_slice(value.as_bytes());
        }
        let encoded = URL_SAFE_NO_PAD.encode(bytes);
        if encoded.len() > 512 {
            return Err(Failure::InvalidInput);
        }
        Ok(encoded)
    }

    pub fn decode_private(encoded: &str) -> Result<Self> {
        if encoded.len() > 512 {
            return Err(Failure::InvalidInput);
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| Failure::InvalidInput)?;
        let mut rest = bytes.strip_prefix(TAG).ok_or(Failure::InvalidInput)?;
        fn field(rest: &mut &[u8]) -> Result<String> {
            let length: [u8; 4] = rest
                .get(..4)
                .ok_or(Failure::InvalidInput)?
                .try_into()
                .map_err(|_| Failure::InvalidInput)?;
            let size = u32::from_be_bytes(length) as usize;
            if size == 0 || size > 128 {
                return Err(Failure::InvalidInput);
            }
            let value = std::str::from_utf8(rest.get(4..4 + size).ok_or(Failure::InvalidInput)?)
                .map_err(|_| Failure::InvalidInput)?
                .to_owned();
            *rest = &rest[4 + size..];
            Ok(value)
        }
        let value = Self {
            instance: field(&mut rest)?,
            audit_ref: field(&mut rest)?,
        };
        if !rest.is_empty() || value.encode_private()? != encoded {
            return Err(Failure::InvalidInput);
        }
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    AdmittedExecution,
    EarlyRefusal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Describe,
    Invoke,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hop {
    Gateway,
    Execution,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Authentication,
    Decoding,
    Admission,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Refused,
    Error,
    Unknown,
}

/// Trusted host facts, not caller-supplied or a public request codec. Optional
/// coordinates may be supplied only when independently verified by the owner.
/// Timestamps describe observations; they never prove authorization or expiry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub instance_id: String,
    pub kind: Kind,
    pub activity: Option<Activity>,
    pub hop: Hop,
    pub stage: Stage,
    pub request_id: Option<String>,
    pub principal_ref: Option<String>,
    pub operation_id: Option<String>,
    pub connection_ref: Option<String>,
    pub descriptor_revision: Option<String>,
    pub recorded_at_ms: i64,
    pub attempt_id: Option<Uuid>,
}
impl Anchor {
    pub(super) fn validate(&self) -> Result<()> {
        bounded(&self.instance_id, 128)?;
        for field in [
            &self.request_id,
            &self.principal_ref,
            &self.connection_ref,
            &self.descriptor_revision,
        ]
        .into_iter()
        .flatten()
        {
            bounded(field, 128)?;
        }
        if let Some(operation) = &self.operation_id {
            bounded(operation, 256)?;
        }
        if self.recorded_at_ms < 0 || self.attempt_id.is_some_and(|id| id.is_nil()) {
            return Err(Failure::InvalidInput);
        }
        if self.activity == Some(Activity::Describe)
            && (self.operation_id.is_some() || self.attempt_id.is_some())
        {
            return Err(Failure::InvalidInput);
        }
        if self.kind == Kind::AdmittedExecution
            && (self.stage != Stage::Admission
                || self.principal_ref.is_none()
                || self.activity.is_none()
                || (self.activity == Some(Activity::Invoke) && self.operation_id.is_none()))
        {
            return Err(Failure::InvalidInput);
        }
        Ok(())
    }
}

/// The owner allocates this UUID before the first append and retains these exact
/// fields through uncertainty. No business result or native message is accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinalObservation {
    pub observation_id: Uuid,
    pub outcome: Outcome,
    pub code: Option<String>,
    pub recorded_at_ms: i64,
}
impl FinalObservation {
    pub(super) fn validate(&self) -> Result<()> {
        if self.observation_id.is_nil() || self.recorded_at_ms < 0 {
            return Err(Failure::InvalidInput);
        }
        if let Some(code) = &self.code {
            bounded(code, 64)?;
            if !code.is_ascii() || code.bytes().any(|b| b.is_ascii_control()) {
                return Err(Failure::InvalidInput);
            }
        }
        Ok(())
    }
}

/// Internal owner observation only. Reading it grants neither admission nor
/// permission to disclose its contents and cannot reconstruct a receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub reference: Reference,
    pub anchor: Anchor,
    pub final_observation: Option<FinalObservation>,
}
impl Record {
    pub(super) fn encode(&self) -> Result<String> {
        self.anchor.validate()?;
        if self.reference.instance != self.anchor.instance_id {
            return Err(Failure::InvalidInput);
        }
        let private = self.reference.encode_private()?;
        if let Some(value) = &self.final_observation {
            value.validate()?;
        }
        let encoded = serde_json::to_string(self).map_err(|_| Failure::InvalidInput)?;
        // Include the separate stored private key in the aggregate budget.
        if encoded.len() + private.len() > 4096 {
            return Err(Failure::Capacity);
        }
        Ok(encoded)
    }
}

pub enum Acknowledgement {
    Execution(Admission),
    Refusal(Reference),
}

/// Non-Clone and not serializable. Only definite anchor acknowledgement creates
/// this live receipt; a coordinator still needs every other execution gate.
pub struct Admission {
    pub(super) record: Box<Record>,
    pub(super) authority: Uuid,
    pub(super) process: u32,
}
impl Admission {
    pub fn reference(&self) -> &Reference {
        &self.record.reference
    }
    pub fn facts(&self) -> &Anchor {
        &self.record.anchor
    }
}

pub(super) fn bounded(value: &str, limit: usize) -> Result<()> {
    if value.is_empty() || value.len() > limit {
        Err(Failure::InvalidInput)
    } else {
        Ok(())
    }
}
