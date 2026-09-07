//! Pure verification and normalization of the declared Slack and Twilio webhooks.
//!
//! The caller supplies the receiver's admitted identity, event set, named signing secret, raw
//! request bytes and configured public URL. This module opens no listener, reads no secret store,
//! creates no identities and acknowledges nothing. An `Event` result must be attributed and
//! persisted by the receiver before HTTP acknowledgement; a challenge is a separate control result.
//!
//! Verification follows the catalog's closed HMAC axes. The envelope adapters cover Slack's
//! Events API and Twilio's two status callbacks only. They do not imply general webhook support.
//! Slack's envelope/challenge contract is documented at
//! <https://docs.slack.dev/apis/events-api/using-http-request-urls/>; request verification is at
//! <https://docs.slack.dev/authentication/verifying-requests-from-slack/>. Twilio's configured URL
//! and decoded, sorted form signature are at <https://www.twilio.com/docs/usage/security>.
//! Attribution fields come from <https://docs.slack.dev/apis/events-api/>,
//! <https://www.twilio.com/docs/messaging/guides/track-outbound-message-status>, and
//! <https://www.twilio.com/docs/voice/api/call-resource#statuscallback>.
//! The native schemas remain the shipped catalog's selected coverage, even if vendor documentation
//! has since added values; this runtime does not silently widen a pinned provider schema.

use std::collections::BTreeMap;

use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha1::Sha1;
use sha2::Sha256;

/// Maximum unparsed body accepted by this pure boundary. Listeners must also bound their reads.
pub const MAX_WEBHOOK_BODY_BYTES: usize = 256 * 1024;
const MAX_HEADERS: usize = 64;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_FORM_FIELDS: usize = 256;
const MAX_REF_BYTES: usize = 512;

/// A refusal deliberately carrying no raw request, signature, URL, or secret value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WebhookError {
    /// The catalog binding or one of its declared semantics is not supported.
    #[error("unsupported webhook declaration")]
    Declaration,
    /// The caller must supply the declared signing credential and a matching identity kind.
    #[error("webhook receiver context is invalid")]
    Context,
    /// A body, header collection, URL, or decoded form exceeds its bound.
    #[error("webhook request exceeds its bounds")]
    Bounds,
    /// A required header is absent, duplicated, or malformed.
    #[error("webhook header is absent or ambiguous")]
    Header,
    /// The declared signature cannot authenticate these bytes.
    #[error("webhook signature is invalid")]
    Signature,
    /// A declared timestamp is malformed or outside the declared window.
    #[error("webhook timestamp is invalid or outside its window")]
    Timestamp,
    /// The request encoding is outside the supported envelope's declared shape.
    #[error("unsupported webhook content type")]
    ContentType,
    /// The verified envelope or native event does not satisfy its shape/schema.
    #[error("webhook payload is invalid")]
    Payload,
    /// The authenticated upstream identity differs from this receiver's expected identity.
    #[error("webhook identity does not match the receiver")]
    Attribution,
}

/// Original bytes and transport facts. No `Debug` implementation exposes their contents.
pub struct WebhookRequest<'a> {
    /// Original header values, retaining duplicates so ambiguous authentication can be refused.
    pub headers: &'a [(&'a str, &'a str)],
    /// Original body, before JSON/form normalization.
    pub body: &'a [u8],
    /// Exact public callback URL configured with the provider, including its query string.
    /// Never reconstruct this from an untrusted Host or Forwarded header.
    pub public_url: &'a str,
    /// Receiver clock supplied explicitly for deterministic timestamp-window verification.
    pub now_unix_seconds: u64,
}

/// A credential already resolved within the receiver's custody boundary. It is never retained.
pub struct WebhookSecret<'a> {
    /// Must equal [`WebhookPlan::secret_name`].
    pub name: &'a str,
    /// Signing secret bytes, never an API token chosen by fallback.
    pub value: &'a [u8],
}

/// Expected provider identity from admitted receiver configuration, never inferred from a request.
#[derive(Clone, Copy)]
pub enum WebhookIdentity<'a> {
    /// Slack app registration and workspace installation served by this receiver.
    Slack {
        /// Expected workspace ID.
        team_id: &'a str,
        /// Expected app registration ID.
        api_app_id: &'a str,
    },
    /// Account owning a Twilio status callback.
    Twilio {
        /// Expected account SID.
        account_sid: &'a str,
    },
}

/// One authenticated, admitted native event, awaiting durable receiver attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct WebhookEvent {
    /// Exact catalog event name, preserving its declared split from the wire discriminator.
    pub event_type: &'static str,
    /// Declared vendor redelivery identity. `None` never fabricates a deduplication guarantee.
    pub delivery_id: Option<String>,
    /// Native event payload validated against its vendor schema, without the transport envelope.
    /// Slack's `channel` and `thread_ts` remain provider fields.
    pub payload: Value,
    /// Present fields from the declaration's payload map, for separately authorized business replies.
    /// Missing optional native fields remain absent; they are not invented as null or empty strings.
    pub reply_fields: BTreeMap<String, Value>,
}

/// Verification alone never means an event was persisted or a business reply was authorized.
#[derive(Debug, Clone, PartialEq)]
pub enum WebhookOutcome {
    /// Authenticated Slack URL verification. The caller can return this string as a challenge reply.
    Challenge(String),
    /// Authentic delivery excluded by the declared event set, admission, narrowing, or loop guard.
    Ignored,
    /// An event the caller must persist before acknowledging the provider.
    Event(WebhookEvent),
}

#[derive(Clone, Copy)]
enum Envelope {
    Slack,
    Twilio,
}

/// Compiled catalog verification and event rules. Contains no credentials or live transport state.
pub struct WebhookPlan {
    binding: &'static catalog::Channel,
    envelope: Envelope,
    hmac: HmacRule,
    segments: Vec<Segment>,
    tolerance: Option<u64>,
    events: Vec<EventRule>,
}

// Typed projections of existing canonical declaration fields, like document.rs. The outer catalog
// declaration has other responsibilities; unknown verification fields are refused, not interpreted.
#[derive(Deserialize)]
struct BindingDeclaration {
    verification: Verification,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Verification {
    kind: String,
    verified: bool,
    hmac: Option<HmacRule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HmacRule {
    algorithm: Algorithm,
    encoding: Encoding,
    header: String,
    prefix: Option<String>,
    signed: String,
    secret: String,
    timestamp: Option<Selector>,
    timestamp_format: Option<String>,
    tolerance: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Algorithm {
    Sha1,
    Sha256,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Encoding {
    Hex,
    Base64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selector {
    source: String,
    name: String,
}

#[derive(Deserialize)]
struct EventDeclaration {
    #[serde(default)]
    when: BTreeMap<String, Value>,
}

struct EventRule {
    name: &'static str,
    wire_value: &'static str,
    schema: jsonschema::Validator,
    when: Vec<(String, jsonschema::Validator)>,
}

enum Segment {
    Literal(String),
    Body,
    SortedForm,
    Timestamp,
    Url,
}

impl WebhookPlan {
    /// Compile an installed webhook binding. Socket/poll/session bindings and unknown envelope
    /// adapters are refused. Vendor verification facts are read from the canonical catalog.
    ///
    /// # Errors
    /// Returns [`WebhookError::Declaration`] for unsupported or inconsistent declarations.
    pub fn from_catalog(
        provider: &'static catalog::Provider,
        binding_name: &str,
    ) -> Result<Self, WebhookError> {
        let binding = provider
            .channel(binding_name)
            .ok_or(WebhookError::Declaration)?;
        if binding.transport != catalog::ChannelTransport::Webhook || binding.events.is_empty() {
            return Err(WebhookError::Declaration);
        }
        let envelope = match (provider.id, binding.name) {
            ("slack", "events-api") => Envelope::Slack,
            ("twilio", "message-status-callback" | "call-status-callback") => Envelope::Twilio,
            _ => return Err(WebhookError::Declaration),
        };
        let declaration: BindingDeclaration = serde_json::from_str(binding.declaration_json)
            .map_err(|_| WebhookError::Declaration)?;
        if declaration.verification.kind != "hmac" || !declaration.verification.verified {
            return Err(WebhookError::Declaration);
        }
        let hmac = declaration
            .verification
            .hmac
            .ok_or(WebhookError::Declaration)?;
        let segments = compile_template(&hmac.signed)?;
        let timestamped = segments
            .iter()
            .any(|part| matches!(part, Segment::Timestamp));
        if hmac.header.is_empty()
            || hmac.secret.is_empty()
            || timestamped != hmac.timestamp.is_some()
            || timestamped != hmac.tolerance.is_some()
            || hmac
                .timestamp
                .as_ref()
                .is_some_and(|selector| selector.source != "header" || selector.name.is_empty())
            || hmac
                .timestamp_format
                .as_deref()
                .is_some_and(|format| !timestamped || format != "unix_seconds")
        {
            return Err(WebhookError::Declaration);
        }
        let tolerance = hmac.tolerance.as_deref().map(parse_tolerance).transpose()?;
        let events = binding
            .events
            .iter()
            .map(|name| {
                let event = provider
                    .events
                    .iter()
                    .find(|event| event.name == *name && event.service == binding.service)
                    .ok_or(WebhookError::Declaration)?;
                let declaration: EventDeclaration = serde_json::from_str(event.declaration_json)
                    .map_err(|_| WebhookError::Declaration)?;
                let schema: Value =
                    serde_json::from_str(event.schema.ok_or(WebhookError::Declaration)?)
                        .map_err(|_| WebhookError::Declaration)?;
                Ok(EventRule {
                    name: event.name,
                    wire_value: event.wire_value.unwrap_or(event.name),
                    schema: jsonschema::validator_for(&schema)
                        .map_err(|_| WebhookError::Declaration)?,
                    when: declaration
                        .when
                        .into_iter()
                        .map(|(path, schema)| {
                            Ok((
                                path,
                                jsonschema::validator_for(&schema)
                                    .map_err(|_| WebhookError::Declaration)?,
                            ))
                        })
                        .collect::<Result<_, WebhookError>>()?,
                })
            })
            .collect::<Result<_, WebhookError>>()?;
        Ok(Self {
            binding,
            envelope,
            hmac,
            segments,
            tolerance,
            events,
        })
    }

    /// Credential name a host must resolve within its existing receiver custody boundary.
    #[must_use]
    pub fn secret_name(&self) -> &str {
        &self.hmac.secret
    }

    /// Verify raw bytes, prove provider attribution, apply the closed event admission set, validate
    /// the native event, and project optional reply fields. Does not persist or acknowledge.
    ///
    /// # Errors
    /// Returns a bounded, value-free refusal for invalid authority, request, signature or payload.
    pub fn receive(
        &self,
        request: &WebhookRequest<'_>,
        secret: WebhookSecret<'_>,
        identity: WebhookIdentity<'_>,
        allowed_events: &[&str],
    ) -> Result<WebhookOutcome, WebhookError> {
        self.context(identity, allowed_events)?;
        self.verify(request, secret)?;
        let envelope = match self.envelope {
            Envelope::Slack => {
                require_content_type(request, "application/json")?;
                serde_json::from_slice::<Value>(request.body).map_err(|_| WebhookError::Payload)?
            }
            Envelope::Twilio => {
                require_content_type(request, "application/x-www-form-urlencoded")?;
                Value::Object(
                    parse_form(request.body)?
                        .into_iter()
                        .map(|(key, value)| (key, Value::String(value)))
                        .collect(),
                )
            }
        };
        let payload = match identity {
            WebhookIdentity::Slack {
                team_id,
                api_app_id,
            } => {
                match string(&envelope, "type")? {
                    "url_verification" => {
                        return Ok(WebhookOutcome::Challenge(
                            string(&envelope, "challenge")?.to_owned(),
                        ))
                    }
                    "event_callback" => {}
                    _ => return Ok(WebhookOutcome::Ignored),
                }
                if string(&envelope, "team_id")? != team_id
                    || string(&envelope, "api_app_id")? != api_app_id
                {
                    return Err(WebhookError::Attribution);
                }
                envelope
                    .get("event")
                    .filter(|value| value.is_object())
                    .ok_or(WebhookError::Payload)?
            }
            WebhookIdentity::Twilio { account_sid } => {
                if string(&envelope, "AccountSid")? != account_sid {
                    return Err(WebhookError::Attribution);
                }
                &envelope
            }
        };
        let wire = match self.binding.discriminator {
            Some(selector) => selected_string(&envelope, request, selector)?,
            None if self.events.len() == 1 => self.events[0].wire_value,
            None => return Err(WebhookError::Declaration),
        };
        let candidates = self
            .events
            .iter()
            .filter(|event| event.wire_value == wire)
            .filter(|event| {
                event.when.iter().all(|(path, schema)| {
                    select(payload, path).is_some_and(|value| schema.is_valid(value))
                })
            })
            .collect::<Vec<_>>();
        let rule = match candidates.as_slice() {
            [] => return Ok(WebhookOutcome::Ignored),
            [rule] => *rule,
            _ => return Err(WebhookError::Declaration),
        };
        if !allowed_events.contains(&rule.name) {
            return Ok(WebhookOutcome::Ignored);
        }
        // Preserve the existing Socket Mode loop policy; channel is provider data, not a receiver.
        if matches!(self.envelope, Envelope::Slack)
            && wire == "message"
            && (payload.get("bot_id").is_some() || payload.get("subtype").is_some())
        {
            return Ok(WebhookOutcome::Ignored);
        }
        if !rule.schema.is_valid(payload) {
            return Err(WebhookError::Payload);
        }
        let delivery_id = self
            .binding
            .delivery_id
            .map(|selector| selected_string(&envelope, request, selector).map(str::to_owned))
            .transpose()?;
        let reply_fields = self
            .binding
            .payload
            .iter()
            .filter_map(|pair| {
                select(&envelope, pair.value).map(|value| (pair.name.to_owned(), value.clone()))
            })
            .collect();
        Ok(WebhookOutcome::Event(WebhookEvent {
            event_type: rule.name,
            delivery_id,
            payload: if self.binding.payload_root {
                envelope.clone()
            } else {
                payload.clone()
            },
            reply_fields,
        }))
    }

    fn context(&self, identity: WebhookIdentity<'_>, allowed: &[&str]) -> Result<(), WebhookError> {
        let valid_identity = match (self.envelope, identity) {
            (
                Envelope::Slack,
                WebhookIdentity::Slack {
                    team_id,
                    api_app_id,
                },
            ) => valid_ref(team_id) && valid_ref(api_app_id),
            (Envelope::Twilio, WebhookIdentity::Twilio { account_sid }) => valid_ref(account_sid),
            _ => false,
        };
        if !valid_identity
            || allowed.len() > self.events.len()
            || allowed.iter().enumerate().any(|(index, name)| {
                !self.events.iter().any(|event| event.name == *name)
                    || allowed[..index].contains(name)
            })
        {
            return Err(WebhookError::Context);
        }
        Ok(())
    }

    fn verify(
        &self,
        request: &WebhookRequest<'_>,
        secret: WebhookSecret<'_>,
    ) -> Result<(), WebhookError> {
        if secret.name != self.hmac.secret || secret.value.is_empty() {
            return Err(WebhookError::Context);
        }
        if request.body.len() > MAX_WEBHOOK_BODY_BYTES
            || request.headers.len() > MAX_HEADERS
            || request
                .headers
                .iter()
                .map(|(key, value)| key.len().saturating_add(value.len()))
                .sum::<usize>()
                > MAX_HEADER_BYTES
            || request.public_url.len() > 4096
        {
            return Err(WebhookError::Bounds);
        }
        let raw = header(request, &self.hmac.header)?;
        let encoded = raw
            .strip_prefix(self.hmac.prefix.as_deref().unwrap_or(""))
            .ok_or(WebhookError::Signature)?;
        let provided = match self.hmac.encoding {
            Encoding::Hex => hex::decode(encoded).map_err(|_| WebhookError::Signature)?,
            Encoding::Base64 => base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|_| WebhookError::Signature)?,
        };
        let timestamp = self
            .hmac
            .timestamp
            .as_ref()
            .map(|selector| header(request, &selector.name))
            .transpose()?;
        if let (Some(timestamp), Some(tolerance)) = (timestamp, self.tolerance) {
            if timestamp.is_empty() || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(WebhookError::Timestamp);
            }
            let signed_at = timestamp
                .parse::<u64>()
                .map_err(|_| WebhookError::Timestamp)?;
            if request.now_unix_seconds.abs_diff(signed_at) > tolerance {
                return Err(WebhookError::Timestamp);
            }
        }
        let form = if self
            .segments
            .iter()
            .any(|part| matches!(part, Segment::SortedForm))
        {
            require_content_type(request, "application/x-www-form-urlencoded")?;
            Some(parse_form(request.body)?)
        } else {
            None
        };
        if self
            .segments
            .iter()
            .any(|part| matches!(part, Segment::Url))
            && !(request.public_url.starts_with("https://")
                || request.public_url.starts_with("http://"))
        {
            return Err(WebhookError::Context);
        }
        // RustCrypto's verify_slice uses a constant-time tag comparison. Neither expected tag nor
        // submitted signature enters an error; the reference fixture's handwritten crypto is not used.
        macro_rules! verify {
            ($digest:ty) => {{
                let mut mac = Hmac::<$digest>::new_from_slice(secret.value)
                    .map_err(|_| WebhookError::Context)?;
                for part in &self.segments {
                    match part {
                        Segment::Literal(value) => mac.update(value.as_bytes()),
                        Segment::Body => mac.update(request.body),
                        Segment::Timestamp => {
                            mac.update(timestamp.ok_or(WebhookError::Declaration)?.as_bytes())
                        }
                        Segment::Url => mac.update(request.public_url.as_bytes()),
                        Segment::SortedForm => {
                            for (key, value) in form.as_ref().ok_or(WebhookError::Declaration)? {
                                mac.update(key.as_bytes());
                                mac.update(value.as_bytes());
                            }
                        }
                    }
                }
                mac.verify_slice(&provided)
                    .map_err(|_| WebhookError::Signature)
            }};
        }
        match self.hmac.algorithm {
            Algorithm::Sha1 => verify!(Sha1),
            Algorithm::Sha256 => verify!(Sha256),
        }
    }
}

fn compile_template(template: &str) -> Result<Vec<Segment>, WebhookError> {
    if template.len() > 4096 {
        return Err(WebhookError::Declaration);
    }
    let mut segments = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let literal = &rest[..open];
        if literal.contains('}') {
            return Err(WebhookError::Declaration);
        }
        segments.push(Segment::Literal(literal.to_owned()));
        let after = &rest[open + 1..];
        let close = after.find('}').ok_or(WebhookError::Declaration)?;
        segments.push(match &after[..close] {
            "body" => Segment::Body,
            "sorted_form" => Segment::SortedForm,
            "timestamp" => Segment::Timestamp,
            "url" => Segment::Url,
            _ => return Err(WebhookError::Declaration),
        });
        rest = &after[close + 1..];
    }
    if rest.contains('}')
        || !segments
            .iter()
            .any(|part| matches!(part, Segment::Body | Segment::SortedForm))
    {
        return Err(WebhookError::Declaration);
    }
    segments.push(Segment::Literal(rest.to_owned()));
    Ok(segments)
}

fn parse_tolerance(value: &str) -> Result<u64, WebhookError> {
    let (digits, scale) = if let Some(digits) = value.strip_suffix('s') {
        (digits, 1)
    } else if let Some(digits) = value.strip_suffix('m') {
        (digits, 60)
    } else if let Some(digits) = value.strip_suffix('h') {
        (digits, 3600)
    } else {
        return Err(WebhookError::Declaration);
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(WebhookError::Declaration);
    }
    digits
        .parse::<u64>()
        .ok()
        .and_then(|seconds| seconds.checked_mul(scale))
        .filter(|seconds| (1..=3600).contains(seconds))
        .ok_or(WebhookError::Declaration)
}

fn header<'a>(request: &'a WebhookRequest<'_>, name: &str) -> Result<&'a str, WebhookError> {
    let mut values = request
        .headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| *value);
    match (values.next(), values.next()) {
        (Some(value), None)
            if !value.is_empty() && !value.bytes().any(|byte| byte.is_ascii_control()) =>
        {
            Ok(value)
        }
        _ => Err(WebhookError::Header),
    }
}

fn require_content_type(request: &WebhookRequest<'_>, expected: &str) -> Result<(), WebhookError> {
    let value = header(request, "content-type")?;
    if !value
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case(expected)
    {
        return Err(WebhookError::ContentType);
    }
    Ok(())
}

fn parse_form(body: &[u8]) -> Result<BTreeMap<String, String>, WebhookError> {
    let mut fields = BTreeMap::new();
    for field in body.split(|byte| *byte == b'&') {
        if fields.len() == MAX_FORM_FIELDS {
            return Err(WebhookError::Bounds);
        }
        let split = field
            .iter()
            .position(|byte| *byte == b'=')
            .ok_or(WebhookError::Payload)?;
        let key = form_decode(&field[..split])?;
        let value = form_decode(&field[split + 1..])?;
        if key.is_empty() || fields.insert(key, value).is_some() {
            return Err(WebhookError::Payload);
        }
    }
    Ok(fields)
}

fn form_decode(bytes: &[u8]) -> Result<String, WebhookError> {
    let mut result = Vec::with_capacity(bytes.len());
    let mut position = 0;
    while position < bytes.len() {
        match bytes[position] {
            b'+' => result.push(b' '),
            b'%' => {
                let encoded = bytes
                    .get(position + 1..position + 3)
                    .ok_or(WebhookError::Payload)?;
                let mut decoded = [0];
                hex::decode_to_slice(encoded, &mut decoded).map_err(|_| WebhookError::Payload)?;
                result.push(decoded[0]);
                position += 2;
            }
            byte => result.push(byte),
        }
        position += 1;
    }
    String::from_utf8(result).map_err(|_| WebhookError::Payload)
}

fn select<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(value, |value, key| value.as_object()?.get(key))
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_REF_BYTES && !value.chars().any(char::is_control)
}

fn string<'a>(value: &'a Value, path: &str) -> Result<&'a str, WebhookError> {
    select(value, path)
        .and_then(Value::as_str)
        .filter(|value| valid_ref(value))
        .ok_or(WebhookError::Payload)
}

fn selected_string<'a>(
    value: &'a Value,
    request: &'a WebhookRequest<'_>,
    selector: catalog::Selector,
) -> Result<&'a str, WebhookError> {
    match selector.source {
        "body" => string(value, selector.name),
        "header" => header(request, selector.name).and_then(|value| {
            if valid_ref(value) {
                Ok(value)
            } else {
                Err(WebhookError::Payload)
            }
        }),
        _ => Err(WebhookError::Declaration),
    }
}

#[cfg(test)]
mod tests;
