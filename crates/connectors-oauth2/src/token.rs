//! Token-endpoint requests and responses: the body encodings, the status and error-code
//! mapping onto [`Failure`], and the validated token response.
use crate::{
    Failure, RESPONSE_LIMIT, Request, RequestEncoding, Response, Secret, Transport,
    declaration::is_https,
};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use std::time::{Duration, SystemTime};
use zeroize::Zeroizing;

/// The tokens a code, device or dialect exchange granted.
#[derive(Debug)]
pub struct Granted {
    pub access_token: Secret,
    /// Absent when the server issued none.
    pub refresh_token: Option<Secret>,
    /// The access token's lifetime from the response, always positive.
    pub expires_in: Duration,
    /// The granted scopes: the response's `scope`, or the requested scopes when it sent none
    /// (RFC 6749 §5.1).
    pub scopes: Vec<String>,
}

/// The tokens a refresh exchange returned.
#[derive(Debug)]
pub struct Refreshed {
    pub access_token: Secret,
    /// The refresh token to keep: the new one when the response carried one, else the old one.
    pub refresh_token: Secret,
    /// Whether the response carried a refresh token different from the one sent.
    pub rotated: bool,
    pub expires_in: Duration,
    /// The response's `scope`, or the previous granted scopes when it sent none.
    pub scopes: Vec<String>,
}

/// How a token-endpoint request ended when it returned no usable body. The device grant reads
/// `authorization_pending` and `slow_down`; every other caller sees them as
/// [`Failure::Rejected`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Failed(Failure),
    Pending,
    SlowDown,
}

impl From<Failure> for Answer {
    fn from(failure: Failure) -> Self {
        Self::Failed(failure)
    }
}

impl Answer {
    /// The failure a caller other than the device grant reports.
    pub(crate) fn failure(self) -> Failure {
        match self {
            Self::Failed(failure) => failure,
            Self::Pending | Self::SlowDown => Failure::Rejected,
        }
    }
}

/// Fields serialized as one flat JSON object, in the order given.
struct Fields<'a>(&'a [(&'a str, &'a str)]);

impl Serialize for Fields<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (name, value) in self.0 {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

/// The request body for `fields` and its content type. The buffer is sized for the worst case
/// up front, so no reallocation leaves an unzeroized copy of a secret behind.
fn encode(
    encoding: RequestEncoding,
    fields: &[(&str, &str)],
) -> Result<(&'static str, Zeroizing<Vec<u8>>), Failure> {
    let size: usize = fields
        .iter()
        .map(|(name, value)| name.len() + value.len())
        .sum();
    match encoding {
        RequestEncoding::Form => {
            let mut form = url::form_urlencoded::Serializer::new(String::with_capacity(
                3 * size + 2 * fields.len(),
            ));
            form.extend_pairs(fields.iter().copied());
            Ok((
                "application/x-www-form-urlencoded",
                Zeroizing::new(form.finish().into_bytes()),
            ))
        }
        RequestEncoding::Json => {
            let mut body = Zeroizing::new(Vec::with_capacity(6 * size + 6 * fields.len() + 2));
            serde_json::to_writer(&mut *body, &Fields(fields))
                .map_err(|_| Failure::InvalidDeclaration)?;
            Ok(("application/json", body))
        }
    }
}

/// POSTs `fields` to the HTTPS endpoint `url` and returns a successful response's body.
pub(crate) fn post(
    transport: &dyn Transport,
    url: &str,
    encoding: RequestEncoding,
    fields: &[(&str, &str)],
) -> Result<Zeroizing<Vec<u8>>, Answer> {
    if !is_https(url) {
        return Err(Failure::InvalidDeclaration.into());
    }
    let (content_type, body) = encode(encoding, fields)?;
    let response = transport
        .post(&Request {
            url,
            content_type,
            body: &body,
            limit: RESPONSE_LIMIT,
        })
        .map_err(|_| Failure::Unavailable)?;
    classify(response)
}

/// A response's body when its status is 2xx; otherwise what the status and the RFC 6749 §5.2
/// (RFC 8628 §3.5) error code mean. An oversized body is refused before its status is read.
fn classify(response: Response) -> Result<Zeroizing<Vec<u8>>, Answer> {
    if response.body.len() > RESPONSE_LIMIT {
        return Err(Failure::InvalidResponse.into());
    }
    match response.status {
        200..=299 => Ok(response.body),
        401 => Err(Failure::ReauthorizationRequired.into()),
        429 => Err(Failure::RateLimited {
            retry_after: retry_after(response.retry_after.as_deref()),
        }
        .into()),
        400..=499 => Err(match error_code(&response.body).as_deref() {
            Some("invalid_grant" | "invalid_client") => Failure::ReauthorizationRequired.into(),
            Some("access_denied") => Failure::AccessDenied.into(),
            Some("expired_token") => Failure::ExpiredToken.into(),
            Some("authorization_pending") => Answer::Pending,
            Some("slow_down") => Answer::SlowDown,
            _ => Failure::Rejected.into(),
        }),
        _ => Err(Failure::Unavailable.into()),
    }
}

/// The `error` code of an error response, when it is one.
fn error_code(body: &[u8]) -> Option<String> {
    #[derive(Deserialize)]
    struct ErrorBody {
        error: String,
    }
    serde_json::from_slice::<ErrorBody>(body)
        .ok()
        .map(|body| body.error)
}

/// `Retry-After` as delta-seconds or an HTTP-date (RFC 9110 §10.2.3); anything else is no
/// delay.
fn retry_after(value: Option<&str>) -> Option<Duration> {
    let value = value?.trim();
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return value.parse().ok().map(Duration::from_secs);
    }
    let at = httpdate::parse_http_date(value).ok()?;
    Some(
        at.duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO),
    )
}

/// A token as RFC 6750 sends it: not empty, and visible ASCII only.
pub(crate) fn is_token(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_graphic())
}

/// A validated RFC 6749 §5.1 token response.
pub(crate) struct TokenResponse {
    pub(crate) access_token: Secret,
    pub(crate) refresh_token: Option<Secret>,
    pub(crate) expires_in: Duration,
    /// The response's `scope`, when it sent one.
    pub(crate) scopes: Option<Vec<String>>,
}

impl TokenResponse {
    /// Reads a successful body: `access_token` required, `token_type` bearer in any case,
    /// `expires_in` required and positive.
    pub(crate) fn parse(body: &[u8]) -> Result<Self, Failure> {
        #[derive(Deserialize)]
        struct Body {
            access_token: Option<Secret>,
            token_type: Option<String>,
            expires_in: Option<u64>,
            refresh_token: Option<Secret>,
            scope: Option<String>,
        }
        let body: Body = serde_json::from_slice(body).map_err(|_| Failure::InvalidResponse)?;
        let access_token = body
            .access_token
            .filter(|token| is_token(token.expose()))
            .ok_or(Failure::InvalidResponse)?;
        if !body
            .token_type
            .is_some_and(|kind| kind.eq_ignore_ascii_case("bearer"))
        {
            return Err(Failure::InvalidResponse);
        }
        let expires_in = body
            .expires_in
            .filter(|&seconds| seconds > 0)
            .map(Duration::from_secs)
            .ok_or(Failure::InvalidResponse)?;
        if body
            .refresh_token
            .as_ref()
            .is_some_and(|token| !is_token(token.expose()))
        {
            return Err(Failure::InvalidResponse);
        }
        Ok(Self {
            access_token,
            refresh_token: body.refresh_token,
            expires_in,
            scopes: body.scope.map(|scope| {
                scope
                    .split(' ')
                    .filter(|scope| !scope.is_empty())
                    .map(str::to_owned)
                    .collect()
            }),
        })
    }
}

/// [`Failure::InsufficientScope`] when `granted` lacks one of `required`.
pub(crate) fn require(required: &[String], granted: &[String]) -> Result<(), Failure> {
    if required.iter().all(|scope| granted.contains(scope)) {
        Ok(())
    } else {
        Err(Failure::InsufficientScope)
    }
}

#[cfg(test)]
mod tests {
    use super::{RequestEncoding, encode};

    /// Both encodings keep the fields' order and escape what needs escaping.
    #[test]
    fn encodings_keep_order_and_escape() {
        let fields = [("b", "x y&z"), ("a", "q\"\\\u{1}")];
        let (kind, form) = encode(RequestEncoding::Form, &fields).unwrap();
        assert_eq!(kind, "application/x-www-form-urlencoded");
        assert_eq!(&form[..], b"b=x+y%26z&a=q%22%5C%01");
        let (kind, json) = encode(RequestEncoding::Json, &fields).unwrap();
        assert_eq!(kind, "application/json");
        assert_eq!(&json[..], br#"{"b":"x y&z","a":"q\"\\\u0001"}"#);
    }
}
