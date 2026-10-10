//! `pods.logs`, profile `kubernetes-pod-logs`: one bounded log GET turned into
//! lines. The typed model is `connectors_kubernetes.logs` in the adapter's ESS
//! specification; the normative rules are `contracts/logs/v1alpha1/semantics.md`.
//! Everything here is a pure function of the selection and the bytes received,
//! so the decoder is tested without a cluster.
use connectors_core::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};

/// The longest line, in bytes, a result carries; a longer one is clipped.
pub const MAX_LINE_BYTES: usize = 8192;
pub const DEFAULT_SINCE_SECONDS: u32 = 86_400;
pub const DEFAULT_TAIL_LINES: u32 = 200;
pub const DEFAULT_MAX_BYTES: u32 = 131_072;

/// The closed input of `pods.logs`, before defaults.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub namespace: String,
    pub pod: String,
    #[serde(default)]
    pub container: Option<String>,
    #[serde(default)]
    pub since_seconds: Option<u32>,
    #[serde(default)]
    pub tail_lines: Option<u32>,
    #[serde(default)]
    pub max_bytes: Option<u32>,
}

/// `PodLogSelection` after defaults and bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
    pub since_seconds: u32,
    pub tail_lines: u32,
    pub max_bytes: u32,
}

impl Input {
    /// Apply the profile's defaults and its integer bounds. Name grammar and
    /// the configured-namespace check belong to the caller, which holds the
    /// configuration.
    pub fn selection(self) -> Result<Selection> {
        let since_seconds = self.since_seconds.unwrap_or(DEFAULT_SINCE_SECONDS);
        let tail_lines = self.tail_lines.unwrap_or(DEFAULT_TAIL_LINES);
        let max_bytes = self.max_bytes.unwrap_or(DEFAULT_MAX_BYTES);
        if !(1..=86_400).contains(&since_seconds)
            || !(1..=1_000).contains(&tail_lines)
            || !(1..=131_072).contains(&max_bytes)
        {
            return Err(Error::invalid(
                "since_seconds, tail_lines or max_bytes is outside its bound",
            ));
        }
        Ok(Selection {
            namespace: self.namespace,
            pod: self.pod,
            container: self.container,
            since_seconds,
            tail_lines,
            max_bytes,
        })
    }
}

impl Selection {
    /// The one provider request: follow, previous and timestamps are fixed,
    /// and provider `limitBytes` is deliberately never sent (the pinned API may
    /// return more or fewer bytes and split the last line, so it proves no
    /// exhaustion); `max_bytes` is enforced while consuming instead.
    pub fn query(&self) -> Vec<(&'static str, String)> {
        let mut query = Vec::with_capacity(6);
        if let Some(container) = &self.container {
            query.push(("container", container.clone()));
        }
        query.push(("follow", "false".into()));
        query.push(("previous", "false".into()));
        query.push(("sinceSeconds", self.since_seconds.to_string()));
        query.push(("tailLines", self.tail_lines.to_string()));
        query.push(("timestamps", "true".into()));
        query
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Stream {
    pub namespace: String,
    pub pod: String,
    pub container: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Line {
    pub timestamp_unix_ns: Option<String>,
    pub stream: Stream,
    pub line: String,
    pub line_truncated: bool,
    pub redacted: bool,
    pub source: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    ProviderLimit,
    SourceBytes,
    LineBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Truncation {
    pub causes: Vec<Cause>,
    pub occurrences_dropped: Option<u64>,
    pub stream_groups_dropped: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SelectionEcho {
    pub kind: &'static str,
    pub since_seconds: u32,
    pub tail_lines: u32,
}

/// Everything of the public result except its provenance, which the adapter
/// adds because it owns the instance identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoded {
    pub lines: Vec<Line>,
    pub selection: SelectionEcho,
    pub complete: bool,
    pub truncation: Truncation,
}

/// Decode the bytes one log GET returned. `body` holds at most `max_bytes`
/// bytes; `eof` says the response ended cleanly. A body that reached
/// `max_bytes` is a deliberate receiver cutoff whatever `eof` says: at the
/// exact ceiling no lookahead may claim exhaustion. A short body without a
/// clean end is not a cutoff the binding chose and refuses as unavailable.
pub fn decode(selection: &Selection, body: &[u8], eof: bool) -> Result<Decoded> {
    let max_bytes = selection.max_bytes as usize;
    if body.len() > max_bytes {
        return Err(Error::new(
            ErrorCode::UpstreamProtocol,
            "log response exceeded the requested byte ceiling",
        ));
    }
    let cutoff = body.len() == max_bytes;
    if !cutoff && !eof {
        return Err(Error::unavailable());
    }
    let text = if cutoff {
        // Withhold only an incomplete trailing scalar at the cutoff; malformed
        // bytes before it are not a cutoff artifact and refuse.
        match std::str::from_utf8(body) {
            Ok(text) => text,
            Err(error) if error.error_len().is_none() => {
                std::str::from_utf8(&body[..error.valid_up_to()]).map_err(|_| unavailable())?
            }
            Err(_) => return Err(unavailable()),
        }
    } else {
        std::str::from_utf8(body).map_err(|_| unavailable())?
    };
    let stream = Stream {
        namespace: selection.namespace.clone(),
        pod: selection.pod.clone(),
        container: selection.container.clone(),
    };
    let tail = selection.tail_lines as usize;
    let mut segments: Vec<&str> = text.split('\n').collect();
    // The text after the last LF: empty when the text ended on a line break.
    let remainder = segments.pop().unwrap_or("");
    let mut observed = segments.len();
    let mut lines = Vec::new();
    let mut clipped = false;
    for raw in segments.into_iter().take(tail) {
        let line = line(raw, &stream, false);
        clipped |= line.line_truncated;
        lines.push(line);
    }
    // At clean EOF a nonempty unterminated remainder is the last line. At the
    // cutoff it is the valid prefix of a line the source did not finish, so it
    // is emitted as clipped.
    if !remainder.is_empty() {
        observed += 1;
        if lines.len() < tail {
            let line = line(remainder, &stream, cutoff);
            clipped |= line.line_truncated;
            lines.push(line);
        }
    }
    let mut causes = Vec::new();
    if observed >= tail {
        causes.push(Cause::ProviderLimit);
    }
    if cutoff {
        causes.push(Cause::SourceBytes);
    }
    if clipped {
        causes.push(Cause::LineBytes);
    }
    let saturated = causes
        .iter()
        .any(|cause| matches!(cause, Cause::ProviderLimit | Cause::SourceBytes));
    Ok(Decoded {
        lines,
        selection: SelectionEcho {
            kind: "kubernetes-relative-tail",
            since_seconds: selection.since_seconds,
            tail_lines: selection.tail_lines,
        },
        complete: !saturated,
        truncation: Truncation {
            causes,
            // Saturation leaves unseen occurrences unknown; otherwise nothing
            // observed was omitted.
            occurrences_dropped: if saturated { None } else { Some(0) },
            stream_groups_dropped: 0,
        },
    })
}

fn unavailable() -> Error {
    Error::new(
        ErrorCode::Unavailable,
        "log response is not valid UTF-8 text",
    )
}

/// One received line: a valid representable RFC 3339 prefix and its
/// separating space become the timestamp; anything else keeps its text and a
/// null timestamp. The rest is clipped at [`MAX_LINE_BYTES`] on a scalar
/// boundary.
fn line(raw: &str, stream: &Stream, unfinished: bool) -> Line {
    let (timestamp_unix_ns, text) = match raw.split_once(' ') {
        Some((prefix, rest)) => match rfc3339_unix_ns(prefix) {
            Some(ns) => (Some(ns.to_string()), rest),
            None => (None, raw),
        },
        None => (None, raw),
    };
    let (text, clipped) = clip(text, MAX_LINE_BYTES);
    Line {
        timestamp_unix_ns,
        stream: stream.clone(),
        line: text.to_owned(),
        line_truncated: clipped || unfinished,
        redacted: false,
        source: None,
    }
}

/// The longest prefix of at most `limit` bytes that ends on a scalar boundary.
pub fn clip(text: &str, limit: usize) -> (&str, bool) {
    if text.len() <= limit {
        return (text, false);
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}

/// Nanoseconds since the Unix epoch of an RFC 3339 date-time with a `Z` or
/// numeric offset and at most nine fractional digits, the forms the provider
/// writes with `timestamps=true`. Anything else, including a leap second, is
/// not a timestamp this binding can represent exactly.
pub fn rfc3339_unix_ns(text: &str) -> Option<i128> {
    let bytes = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let slice = bytes.get(range)?;
        if slice.is_empty() || !slice.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(slice).ok()?.parse().ok()
    };
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let year = digits(0..4)?;
    let month = digits(5..7)?;
    let day = digits(8..10)?;
    let hour = digits(11..13)?;
    let minute = digits(14..16)?;
    let second = digits(17..19)?;
    if !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let mut index = 19;
    let mut nanos: i64 = 0;
    if bytes[index] == b'.' {
        let start = index + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        let count = end - start;
        if count == 0 || count > 9 {
            return None;
        }
        nanos = digits(start..end)? * 10_i64.pow((9 - count) as u32);
        index = end;
    }
    let offset_seconds = match bytes.get(index)? {
        b'Z' | b'z' if index + 1 == bytes.len() => 0,
        sign @ (b'+' | b'-') if index + 6 == bytes.len() && bytes[index + 3] == b':' => {
            let hours = digits(index + 1..index + 3)?;
            let minutes = digits(index + 4..index + 6)?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            let offset = hours * 3600 + minutes * 60;
            if *sign == b'+' { offset } else { -offset }
        }
        _ => return None,
    };
    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_seconds;
    Some(i128::from(seconds) * 1_000_000_000 + i128::from(nanos))
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// Days from 1970-01-01 to a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_prefixes_convert_exactly_and_refuse_what_they_cannot_represent() {
        assert_eq!(rfc3339_unix_ns("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            rfc3339_unix_ns("2026-10-10T12:00:00.123456789Z"),
            Some(1_791_633_600_123_456_789)
        );
        assert_eq!(
            rfc3339_unix_ns("2026-10-10T14:00:00.5+02:00"),
            Some(1_791_633_600_500_000_000)
        );
        assert_eq!(
            rfc3339_unix_ns("2024-02-29T00:00:00Z"),
            Some(1_709_164_800_000_000_000)
        );
        for invalid in [
            "2023-02-29T00:00:00Z",
            "2026-10-10T12:00:60Z",
            "2026-10-10T12:00:00.1234567891Z",
            "2026-10-10 12:00:00Z",
            "2026-10-10T12:00:00",
            "not-a-timestamp",
        ] {
            assert_eq!(rfc3339_unix_ns(invalid), None, "{invalid}");
        }
    }
}
