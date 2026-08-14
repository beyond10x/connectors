//! Public input/output projection for native SIP call establishment.

use serde::{Deserialize, Serialize};

/// Canonical catalog id. Connector tool projection renders this as `sip.dial`.
pub const SIP_DIAL_OPERATION: &str = "sip-dial";

/// Model/harness-facing operation reference derived from [`SIP_DIAL_OPERATION`].
pub const SIP_DIAL_TOOL_REF: &str = "sip.dial";

/// Caller input for one SIP dial request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SipDialInput {
    /// Opaque alias owned and resolved by the selected Connection.
    pub target: String,
}

impl SipDialInput {
    /// Validate the catalog's closed destination-alias grammar.
    pub fn validate(&self) -> Result<(), SipDialInputError> {
        let bytes = self.target.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 64
            || !bytes[0].is_ascii_alphabetic()
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(SipDialInputError::InvalidTargetAlias);
        }
        Ok(())
    }
}

/// Refusal before a target alias may reach Connection lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SipDialInputError {
    /// The target is not an opaque 1..=64 byte alias.
    #[error("sip.dial target is not a valid Connection-owned alias")]
    InvalidTargetAlias,
}

/// Successful call-establishment result exposed to the invoking harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SipDialEstablished {
    /// Opaque call reference.
    pub call: String,
    /// Opaque voice-session reference.
    pub session: String,
    /// Opaque application-channel reference.
    pub channel: String,
    /// Closed state token; always `established` in this response version.
    pub state: SipDialState,
}

/// State returned by successful `sip.dial` establishment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SipDialState {
    /// SIP and application sides have both completed establishment.
    Established,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_admit_names_and_refuse_network_destinations() {
        for target in ["asterisk-dev", "echo_1"] {
            SipDialInput {
                target: target.to_owned(),
            }
            .validate()
            .unwrap();
        }
        for target in [
            "",
            "sip:echo@127.0.0.1:5062",
            "127.0.0.1:5062",
            "919191",
            "pbx.local",
            "https://pbx.example",
            " space",
        ] {
            assert_eq!(
                SipDialInput {
                    target: target.to_owned(),
                }
                .validate(),
                Err(SipDialInputError::InvalidTargetAlias)
            );
        }
    }
}
