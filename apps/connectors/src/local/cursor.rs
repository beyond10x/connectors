//! Self-contained list cursors for `adapters list` and `operations list`.
//!
//! A cursor is base64url (no padding) of `{version, digest, offset, issued_at}`.
//! The digest covers the list's source and the page limit, so a cursor answers
//! only for the selection it was issued over. There is no stored cursor state:
//! altering a cursor can only select entries the caller may already list.
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::ops::Range;

const VERSION: u64 = 1;
/// Cursor lifetime in seconds (contracts/cli/v1alpha1/semantics.md, list bounds).
const LIFETIME_SECONDS: u64 = 300;
const DEFAULT_LIMIT: i64 = 100;
const MAXIMUM_LIMIT: i64 = 500;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    InvalidInput,
    StaleCursor,
}

struct Cursor {
    digest: String,
    offset: usize,
    issued_at: u64,
}

/// A list request's validated `--limit` and decoded `--cursor`.
pub(super) struct Request {
    limit: usize,
    cursor: Option<Cursor>,
}

impl Request {
    /// Checks `--limit` first, then decodes `--cursor`. Neither reads the
    /// list's source.
    pub(super) fn parse(input: &Value) -> Result<Self, Refusal> {
        let limit = input
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(DEFAULT_LIMIT);
        if !(1..=MAXIMUM_LIMIT).contains(&limit) {
            return Err(Refusal::InvalidInput);
        }
        let cursor = match input.get("cursor") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_str()
                    .and_then(decode)
                    .ok_or(Refusal::StaleCursor)?,
            ),
        };
        Ok(Self {
            limit: limit as usize,
            cursor,
        })
    }

    /// The range of a `len`-entry list this request selects, and the cursor for
    /// the entries after it. `source` is what the list was built from.
    pub(super) fn page(
        &self,
        source: &Value,
        len: usize,
    ) -> Result<(Range<usize>, Option<String>), Refusal> {
        self.page_at(source, len, connectors_sdk::now_ms() / 1000)
    }

    fn page_at(
        &self,
        source: &Value,
        len: usize,
        now: u64,
    ) -> Result<(Range<usize>, Option<String>), Refusal> {
        let digest = connectors_core::digest(&json!({"source":source,"limit":self.limit}));
        let start = match &self.cursor {
            None => 0,
            Some(cursor) => {
                let fresh = cursor.issued_at <= now && now - cursor.issued_at <= LIFETIME_SECONDS;
                if !fresh || cursor.digest != digest || cursor.offset == 0 || cursor.offset >= len {
                    return Err(Refusal::StaleCursor);
                }
                cursor.offset
            }
        };
        let end = start.saturating_add(self.limit).min(len);
        let next = (end < len).then(|| {
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(
                    &json!({"version":VERSION,"digest":digest,"offset":end,"issued_at":now}),
                )
                .expect("JSON Value is serializable"),
            )
        });
        Ok((start..end, next))
    }
}

fn decode(text: &str) -> Option<Cursor> {
    let body: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(text).ok()?).ok()?;
    let object = body.as_object()?;
    if object.len() != 4 || object.get("version")?.as_u64()? != VERSION {
        return None;
    }
    Some(Cursor {
        digest: object.get("digest")?.as_str()?.to_owned(),
        offset: usize::try_from(object.get("offset")?.as_u64()?).ok()?,
        issued_at: object.get("issued_at")?.as_u64()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(limit: i64, cursor: Option<&str>) -> Result<Request, Refusal> {
        Request::parse(&json!({"limit":limit,"cursor":cursor}))
    }

    #[test]
    fn the_encoding_rejects_padding_other_alphabets_and_non_canonical_bits() {
        // URL_SAFE_NO_PAD refuses unused trailing bits that are not zero, so a
        // cursor has exactly one spelling.
        assert_eq!(URL_SAFE_NO_PAD.encode(b"\xfb\xff"), "-_8");
        assert_eq!(URL_SAFE_NO_PAD.decode("-_8").unwrap(), b"\xfb\xff");
        for spelling in ["-_9", "-_8=", "+/8"] {
            assert!(URL_SAFE_NO_PAD.decode(spelling).is_err(), "{spelling}");
        }
    }

    #[test]
    fn a_cursor_expires_after_its_lifetime_and_never_before_its_issue() {
        let source = json!(["a", "b", "c"]);
        let (range, next) = request(2, None).unwrap().page_at(&source, 3, 1000).unwrap();
        assert_eq!(range, 0..2);
        let next = next.unwrap();
        let follow = request(2, Some(&next)).unwrap();
        assert_eq!(follow.page_at(&source, 3, 1300).unwrap(), (2..3, None));
        assert_eq!(follow.page_at(&source, 3, 1301), Err(Refusal::StaleCursor));
        assert_eq!(follow.page_at(&source, 3, 999), Err(Refusal::StaleCursor));
    }

    #[test]
    fn the_limit_is_checked_before_the_cursor() {
        assert_eq!(request(0, Some("!")).err(), Some(Refusal::InvalidInput));
        assert_eq!(request(501, None).err(), Some(Refusal::InvalidInput));
        assert_eq!(request(500, Some("!")).err(), Some(Refusal::StaleCursor));
    }
}
