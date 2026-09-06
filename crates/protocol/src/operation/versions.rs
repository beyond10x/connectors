//! Explicit version selection on original bytes; no dispatch, fallback or resend.

use super::{legacy, v3, wire};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Requested external identity. The internal operation API remains v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// Frozen/deployed v1 reader, retaining its documented schema distinctions.
    V0Alpha1,
    /// Rate metadata and definite rate-limit refusal.
    V0Alpha2,
    /// Safe pre-dispatch authentication facts.
    V0Alpha3,
}

#[derive(Deserialize)]
struct Identity {
    protocol: String,
}

fn selected(bytes: &[u8], maximum: usize) -> Result<Version, v3::OperationError> {
    if bytes.len() > maximum {
        return Err(v3::protocol_refusal());
    }
    let identity: Identity = read(bytes)?;
    match identity.protocol.as_str() {
        legacy::CONTRACT => Ok(Version::V0Alpha1),
        wire::CONTRACT => Ok(Version::V0Alpha2),
        v3::CONTRACT => Ok(Version::V0Alpha3),
        _ => Err(v3::protocol_refusal()),
    }
}
fn read<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, v3::OperationError> {
    serde_json::from_slice(bytes).map_err(|_| v3::protocol_refusal())
}
fn write<T: Serialize>(value: &T, maximum: usize) -> Result<Vec<u8>, v3::OperationError> {
    let bytes = serde_json::to_vec(value).map_err(|_| v3::protocol_refusal())?;
    if bytes.len() > maximum {
        return Err(v3::protocol_refusal());
    }
    Ok(bytes)
}

/// Decode the selected strict request from original bytes and return its ordinary v2 projection.
pub fn decode_request(
    bytes: &[u8],
) -> Result<(Version, wire::RequestEnvelope), v3::OperationError> {
    let version = selected(bytes, wire::MAX_FRAME_BYTES)?;
    let request = match version {
        Version::V0Alpha1 => {
            let value: legacy::RequestEnvelope = read(bytes)?;
            value.validate().map_err(wire::OperationError::from)?;
            value.into()
        }
        Version::V0Alpha2 => {
            let value: wire::RequestEnvelope = read(bytes)?;
            value.validate()?;
            value
        }
        Version::V0Alpha3 => {
            let value: v3::RequestEnvelope = read(bytes)?;
            value.validate()?;
            value.into_v2()
        }
    };
    Ok((version, request))
}

/// Decode every selected response directly; v3 is a lossless lift of ordinary predecessor results.
pub fn decode_response(
    bytes: &[u8],
) -> Result<(Version, v3::ResponseEnvelope), v3::OperationError> {
    let version = selected(bytes, wire::MAX_RESULT_BYTES)?;
    let response = match version {
        Version::V0Alpha1 => {
            let value: legacy::ResponseEnvelope = read(bytes)?;
            value.validate().map_err(wire::OperationError::from)?;
            wire::ResponseEnvelope::from(value).into()
        }
        Version::V0Alpha2 => {
            let value: wire::ResponseEnvelope = read(bytes)?;
            value.validate()?;
            value.into()
        }
        Version::V0Alpha3 => {
            let value: v3::ResponseEnvelope = read(bytes)?;
            value.validate()?;
            value
        }
    };
    Ok((version, response))
}

impl Version {
    /// Serialize a validated ordinary request to this exact identity.
    pub fn encode_request(
        self,
        request: wire::RequestEnvelope,
    ) -> Result<Vec<u8>, v3::OperationError> {
        request.validate()?;
        match self {
            Self::V0Alpha1 => write(&request.into_legacy(), wire::MAX_FRAME_BYTES),
            Self::V0Alpha2 => write(&request, wire::MAX_FRAME_BYTES),
            Self::V0Alpha3 => write(&v3::RequestEnvelope::from(request), wire::MAX_FRAME_BYTES),
        }
    }
    /// Serialize a validated response to the requested identity, applying only its defined loss.
    pub fn encode_response(
        self,
        response: v3::ResponseEnvelope,
    ) -> Result<Vec<u8>, v3::OperationError> {
        response.validate()?;
        match self {
            Self::V0Alpha1 => wire::Version::V0Alpha1
                .encode_response(response.into_v2())
                .map_err(Into::into),
            Self::V0Alpha2 => wire::Version::V0Alpha2
                .encode_response(response.into_v2())
                .map_err(Into::into),
            Self::V0Alpha3 => write(&response, wire::MAX_RESULT_BYTES),
        }
    }
}
