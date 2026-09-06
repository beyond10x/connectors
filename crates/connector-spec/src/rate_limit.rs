//! Published rate declarations, separate from fixed throttle settings.

use serde::{Deserialize, Deserializer, Serialize};

/// A vendor's published rate limit, compiled into a Flux `throttle` by C-12.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateLimit {
    /// Requests allowed per window.
    pub requests: u32,
    /// The window, in seconds.
    pub per_seconds: u32,
    /// The throttle bucket name. Buckets collide if they are not unique within a session, so when
    /// this is `None` codegen derives one from the connector and operation ids.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
}

/// Whether a stated rate is a minimum tier allowance or a ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitBasis {
    /// At least this allowance; burst capacity is not established.
    MinimumAllowance,
    /// The provider's published maximum.
    Ceiling,
}

/// A positive vendor rate with an explicit interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedRate {
    /// Number of requests in the stated window.
    #[serde(deserialize_with = "positive")]
    pub requests: u32,
    /// Window duration in seconds.
    #[serde(deserialize_with = "positive")]
    pub per_seconds: u32,
    /// Meaning of the stated number.
    pub basis: RateLimitBasis,
}

/// One documented applicability category; an absent rate establishes no numeric allowance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalRateLimit {
    /// Conditions that must hold before this declaration applies.
    #[serde(deserialize_with = "applicability")]
    pub applies_when: String,
    /// A number only when the source establishes one.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_rate"
    )]
    pub rate: Option<PublishedRate>,
    /// Public HTTPS source, without embedded credentials or fragments.
    #[serde(deserialize_with = "source")]
    pub source_url: String,
}

fn positive<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let value = u32::deserialize(deserializer)?;
    if value == 0 {
        return Err(serde::de::Error::custom("rate must be positive"));
    }
    Ok(value)
}

fn present_rate<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PublishedRate>, D::Error> {
    PublishedRate::deserialize(deserializer).map(Some)
}

fn bounded(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.chars().count() <= maximum && !value.chars().any(char::is_control)
}

fn applicability<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if !bounded(&value, 4096) {
        return Err(serde::de::Error::custom("invalid rate applicability"));
    }
    Ok(value)
}

fn source<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if !bounded(&value, 2048)
        || !url::Url::parse(&value).is_ok_and(|url| {
            url.scheme() == "https"
                && url.has_host()
                && url.username().is_empty()
                && url.password().is_none()
                && url.fragment().is_none()
        })
    {
        return Err(serde::de::Error::custom("invalid public rate source URL"));
    }
    Ok(value)
}

pub(crate) fn alternatives<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ConditionalRateLimit>, D::Error> {
    let values = Vec::<ConditionalRateLimit>::deserialize(deserializer)?;
    if values.len() > 16 {
        return Err(serde::de::Error::custom("too many rate alternatives"));
    }
    Ok(values)
}
