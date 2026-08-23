//! Public input/output projection for native SIP call establishment.

use serde::{Deserialize, Serialize};

/// Canonical catalog id. Connector tool projection renders this as `sip.dial`.
pub const SIP_DIAL_OPERATION: &str = "sip-dial";

/// Model/harness-facing operation reference derived from [`SIP_DIAL_OPERATION`].
pub const SIP_DIAL_TOOL_REF: &str = "sip.dial";

/// Stable Provider id for the B10x-owned native SIP capability.
pub const SIP_DIAL_PROVIDER: &str = "b10x";

/// Permanent Provider authority for B10x-owned Connector capabilities.
pub const SIP_DIAL_PROVIDER_AUTHORITY: &str = "io.b10x";

/// The longest dialled number accepted.
///
/// E.164 tops out at 15 digits; this allows more because an internal extension plan is not E.164
/// and may carry a prefix. It is a bound against abuse, not a numbering-plan opinion.
pub const MAX_DIALED_NUMBER_DIGITS: usize = 32;

/// Caller input for one SIP dial request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SipDialInput {
    /// Opaque alias owned and resolved by the selected Connection.
    ///
    /// **Absent selects the Connection's default trunk**, which is what makes `sip.dial` with only
    /// a number expressible. A Connection with several trunks and no default refuses rather than
    /// picking one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The party to dial through that trunk.
    ///
    /// # Why this is not a caller-selected destination
    ///
    /// The five-axis model forbids a caller naming where a request goes, and this does not: the
    /// **trunk** is the destination and stays Connection-owned — its host, port, transport,
    /// credentials and apertures are never nameable here. A number is a party reached *through* an
    /// already-admitted trunk, and it only ever becomes the user part of a URI whose host the
    /// Connection owns.
    ///
    /// Absent dials the trunk's own configured URI, which is how a fixed endpoint is reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
}

impl SipDialInput {
    /// Validate the catalog's closed destination-alias and dialled-number grammars.
    ///
    /// # Errors
    ///
    /// [`SipDialInputError::InvalidTargetAlias`] for an alias outside the closed grammar, and
    /// [`SipDialInputError::InvalidNumber`] for anything but 1..=[`MAX_DIALED_NUMBER_DIGITS`]
    /// ASCII digits.
    pub fn validate(&self) -> Result<(), SipDialInputError> {
        if let Some(target) = &self.target {
            let bytes = target.as_bytes();
            if bytes.is_empty()
                || bytes.len() > 64
                || !bytes[0].is_ascii_alphabetic()
                || !bytes
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            {
                return Err(SipDialInputError::InvalidTargetAlias);
            }
        }
        if let Some(number) = &self.number {
            // **Digits and nothing else.** This value becomes the user part of a SIP URI, so every
            // other character is a URI-injection vector rather than a formatting preference: `@`
            // redirects the call to another host, `;` appends URI parameters, `?` appends headers,
            // and CR/LF splits the request line into a forged message. Refusing the whole class is
            // what makes substitution safe, and it is why this is not a "strip bad characters"
            // filter -- a stripped number is a different number, dialled silently.
            let bytes = number.as_bytes();
            if bytes.is_empty()
                || bytes.len() > MAX_DIALED_NUMBER_DIGITS
                || !bytes.iter().all(u8::is_ascii_digit)
            {
                return Err(SipDialInputError::InvalidNumber);
            }
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
    /// The number is not 1..=32 ASCII digits.
    #[error("sip.dial number is not a valid dialled number")]
    InvalidNumber,
}

/// Successful call-establishment result exposed to the invoking harness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SipDialEstablished {
    /// Opaque call reference.
    pub call: String,
    /// Opaque voice-session reference.
    pub session: String,
    /// Opaque application-channel reference, when the call was carried onward to one.
    ///
    /// **Absent for a raw SIP call.** SIP terminates a call at the edge; carrying it onward to an
    /// application channel is a separate binding of the same neutral session contract. A receipt
    /// that always claimed a channel would name one that does not exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
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

    fn dial(target: Option<&str>, number: Option<&str>) -> SipDialInput {
        SipDialInput {
            target: target.map(ToOwned::to_owned),
            number: number.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn aliases_admit_names_and_refuse_network_destinations() {
        for target in ["asterisk-dev", "echo_1"] {
            dial(Some(target), None).validate().unwrap();
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
                dial(Some(target), None).validate(),
                Err(SipDialInputError::InvalidTargetAlias)
            );
        }
    }

    #[test]
    fn a_number_alone_is_admitted_because_the_trunk_supplies_the_destination() {
        // `sip.dial(number)` against the Connection's default trunk. The alias is what names a
        // destination, and omitting it selects a configured default rather than nothing.
        dial(None, Some("12341234")).validate().unwrap();
        dial(None, None).validate().unwrap();
        dial(Some("ivr"), Some("12341234")).validate().unwrap();
    }

    #[test]
    fn a_number_that_could_escape_the_uri_user_part_is_refused() {
        // Each of these is a real escape, not a formatting quibble. The value is substituted into
        // `sip:<number>@<connection-owned-host>`, so `@` redirects the call to a host the caller
        // chose, `;`/`?` append URI parameters and headers, and CR/LF split the request line into
        // a second forged message. Refusing the class is what makes substitution safe.
        for number in [
            "1234@evil.example",
            "1234;maddr=evil.example",
            "1234?Route=<sip:evil.example>",
            "1234\r\nINVITE sip:evil@x SIP/2.0",
            "1234 5678",
            "+4915112345678",
            "sip:1234@host",
            "",
            "12ab34",
        ] {
            assert_eq!(
                dial(None, Some(number)).validate(),
                Err(SipDialInputError::InvalidNumber),
                "{number:?} must not reach a SIP URI"
            );
        }
    }

    #[test]
    fn a_number_is_bounded_rather_than_truncated() {
        let longest = "9".repeat(MAX_DIALED_NUMBER_DIGITS);
        dial(None, Some(&longest)).validate().unwrap();
        let over = "9".repeat(MAX_DIALED_NUMBER_DIGITS + 1);
        assert_eq!(
            dial(None, Some(&over)).validate(),
            Err(SipDialInputError::InvalidNumber)
        );
    }

    #[test]
    fn an_absent_field_is_omitted_from_the_wire_rather_than_sent_as_null() {
        let rendered = serde_json::to_value(dial(None, Some("100"))).expect("input renders");
        assert!(rendered.get("target").is_none());
        assert_eq!(
            rendered.get("number").and_then(serde_json::Value::as_str),
            Some("100")
        );
    }
}
