//! Tavily binding of `datasource.websearch/v1alpha1` under the native profile
//! `tavily/2026-10` (`contracts/websearch/v1alpha1/semantics.md` beside this
//! adapter). Every operation is a read Tavily serves as a POST; the adapter
//! reaches it only through `AuthenticatedHttp::post_json`, whose paths the
//! executable composition fixes to `READ_POSTS`.
use async_trait::async_trait;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{sync::Arc, time::Duration};

pub mod auth;

pub const PROFILE: &str = "tavily/2026-10";
/// The only paths this adapter POSTs to; the composition fixes exactly these.
pub const READ_POSTS: [&[&str]; 3] = [&["search"], &["extract"], &["crawl"]];
/// Tavily bounds a crawl at 150 s; the transport waits a little longer.
pub const READ_POST_TIMEOUT: Duration = Duration::from_secs(160);
/// Per-item content ceiling, clipped on a UTF-8 boundary.
pub const CONTENT_BYTES: usize = 128 * 1024;
/// Serialized-result ceiling; items past it are omitted and reported.
pub const RESULT_BYTES: usize = 3 * 1024 * 1024;
/// Tavily's own crawl deadline, in seconds, sent with every crawl.
const CRAWL_TIMEOUT_S: u64 = 150;
const OPERATIONS: [&str; 3] = ["websearch.search", "websearch.fetch", "websearch.crawl"];

pub struct Tavily {
    instance: String,
    http: Arc<dyn AuthenticatedHttp>,
    descriptor: Descriptor,
    clock: fn() -> u64,
}

impl Tavily {
    pub fn new(
        instance: &str,
        effective_configuration: &Value,
        http: Arc<dyn AuthenticatedHttp>,
    ) -> Result<Self> {
        let descriptor = connectors_sdk::instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            effective_configuration,
        )?;
        connectors_sdk::verify_handlers(&descriptor, &OPERATIONS)?;
        Ok(Self {
            instance: instance.to_owned(),
            http,
            descriptor,
            clock: connectors_sdk::now_ms,
        })
    }

    /// A new immutable HTTP capability for one host-admitted use.
    pub fn with_authenticated_http(&self, http: Arc<dyn AuthenticatedHttp>) -> Self {
        Self {
            instance: self.instance.clone(),
            http,
            descriptor: self.descriptor.clone(),
            clock: self.clock,
        }
    }

    /// For tests: a fixed clock, so provenance is deterministic.
    pub fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    fn input_schema(&self, operation: &str) -> Result<&Value> {
        self.descriptor
            .operations
            .iter()
            .find(|o| o.id == operation)
            .map(|o| &o.input_schema)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "operation is not provided"))
    }

    fn provenance(&self) -> Value {
        json!({
            "instance": self.instance,
            "profile": PROFILE,
            "received_at": rfc3339((self.clock)()),
        })
    }

    async fn post(&self, path: &'static str, body: &Value) -> Result<Value> {
        let response = self.http.post_json(&[path], &[], body).await?;
        answer(response)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
    max_results: u32,
    content: Option<String>,
    time_range: Option<String>,
    topic: Option<String>,
    include_domains: Option<Vec<String>>,
    exclude_domains: Option<Vec<String>>,
    country: Option<String>,
    language: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fetch {
    urls: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Crawl {
    url: String,
    limit: u32,
    max_depth: Option<u32>,
    max_breadth: Option<u32>,
    select_paths: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
    allow_external: Option<bool>,
    instructions: Option<String>,
}

fn decode<T: for<'de> Deserialize<'de>>(input: Value) -> Result<T> {
    serde_json::from_value(input).map_err(|_| Error::invalid("input does not match the operation"))
}

/// A provider status as the family's error, never carrying the provider's text.
fn answer(response: HttpResponse) -> Result<Value> {
    match response.status {
        200 => {}
        400 | 422 => return Err(Error::invalid("Tavily refused the request")),
        401 => {
            return Err(Error::new(
                ErrorCode::Unauthorized,
                "Tavily refused the API key",
            ));
        }
        403 => {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Tavily refused the target",
            ));
        }
        429 => {
            return Err(Error::new(
                ErrorCode::RateLimited,
                "Tavily rate limit reached",
            ));
        }
        432 | 433 => {
            return Err(Error::new(
                ErrorCode::Capacity,
                "Tavily plan or pay-as-you-go limit reached",
            ));
        }
        500..=599 => return Err(Error::unavailable()),
        _ => {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "unexpected Tavily status",
            ));
        }
    }
    connectors_core::read_json(&response.body)
        .map_err(|_| Error::new(ErrorCode::UpstreamProtocol, "malformed Tavily answer"))
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .map(str::to_owned)
}

/// Clips `content` to `CONTENT_BYTES` on a UTF-8 boundary.
fn bounded(content: Option<String>) -> (Option<String>, bool) {
    match content {
        Some(mut c) if c.len() > CONTENT_BYTES => {
            let mut end = CONTENT_BYTES;
            while !c.is_char_boundary(end) {
                end -= 1;
            }
            c.truncate(end);
            (Some(c), true)
        }
        other => (other, false),
    }
}

/// Keeps items until the serialized result would pass `RESULT_BYTES`.
fn within_result_bytes(items: Vec<Value>) -> (Vec<Value>, bool) {
    let mut kept = Vec::with_capacity(items.len());
    let mut bytes = 0;
    for item in items {
        let size = serde_json::to_vec(&item)
            .map(|b| b.len())
            .unwrap_or(usize::MAX);
        if bytes + size > RESULT_BYTES {
            return (kept, true);
        }
        bytes += size;
        kept.push(item);
    }
    (kept, false)
}

fn results(answer: &Value) -> Result<&Vec<Value>> {
    answer
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::new(ErrorCode::UpstreamProtocol, "Tavily answer has no results"))
}

fn page(item: &Value) -> Option<Value> {
    let url = text(item, "url")?;
    let (content, content_truncated) = bounded(text(item, "raw_content"));
    Some(json!({
        "url": url,
        "title": text(item, "title"),
        "content": content,
        "content_truncated": content_truncated,
    }))
}

impl Tavily {
    async fn search(&self, input: Search) -> Result<Value> {
        let content = input.content.as_deref().unwrap_or("snippet");
        let mut body = Map::new();
        body.insert("query".into(), json!(input.query));
        body.insert("max_results".into(), json!(input.max_results));
        body.insert("search_depth".into(), json!("basic"));
        body.insert("include_published_date".into(), json!(true));
        body.insert(
            "include_raw_content".into(),
            if content == "full" {
                json!("markdown")
            } else {
                json!(false)
            },
        );
        for (key, value) in [
            ("time_range", input.time_range.map(Value::from)),
            ("topic", input.topic.map(Value::from)),
            ("include_domains", input.include_domains.map(Value::from)),
            ("exclude_domains", input.exclude_domains.map(Value::from)),
            ("country", input.country.map(Value::from)),
            ("language", input.language.map(Value::from)),
        ] {
            if let Some(value) = value {
                body.insert(key.into(), value);
            }
        }
        let answer = self.post("search", &Value::Object(body)).await?;
        let mut clipped = false;
        let items: Vec<Value> = results(&answer)?
            .iter()
            .filter_map(|r| {
                let url = text(r, "url")?;
                let snippet = text(r, "content");
                let (content, content_truncated) = bounded(match content {
                    "none" => None,
                    "full" => text(r, "raw_content").or_else(|| snippet.clone()),
                    _ => snippet.clone(),
                });
                clipped |= content_truncated;
                Some(json!({
                    "url": url,
                    "title": text(r, "title"),
                    "description": snippet,
                    "content": content,
                    "content_truncated": content_truncated,
                    "published": text(r, "published_date"),
                    "score": r.get("score").and_then(Value::as_f64).map(|s| s.to_string()),
                }))
            })
            .collect();
        let returned = items.len();
        let (items, omitted) = within_result_bytes(items);
        let mut truncation = Vec::new();
        if returned < input.max_results as usize {
            truncation.push("provider_limit");
        }
        if omitted {
            truncation.push("response_bytes");
        }
        if clipped {
            truncation.push("content_bytes");
        }
        Ok(json!({
            "results": items,
            "complete": returned >= input.max_results as usize && !omitted,
            "truncation": truncation,
            "provenance": self.provenance(),
        }))
    }

    fn pages(
        &self,
        answer: &Value,
        requested: Option<&[String]>,
        limit: Option<u32>,
    ) -> Result<Value> {
        let mut pages: Vec<Value> = results(answer)?.iter().filter_map(page).collect();
        let clipped = pages.iter().any(|p| p["content_truncated"] == true);
        let failed: Vec<Value> = answer
            .get("failed_results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|f| Some(json!({"url": text(f, "url")?, "reason": "provider_refused"})))
            .collect();
        let reached = pages.len();
        let (kept, omitted) = within_result_bytes(std::mem::take(&mut pages));
        let mut truncation = Vec::new();
        let complete = match (requested, limit) {
            // fetch: every URL answered, as a page or a failure.
            (Some(urls), _) => urls.iter().all(|u| {
                kept.iter()
                    .chain(failed.iter())
                    .any(|p| p["url"].as_str() == Some(u))
            }),
            // crawl: it stopped before reaching its limit.
            (None, Some(limit)) => {
                if reached >= limit as usize {
                    truncation.push("result_limit");
                }
                reached < limit as usize
            }
            (None, None) => false,
        } && !omitted;
        if omitted {
            truncation.push("response_bytes");
        }
        if clipped {
            truncation.push("content_bytes");
        }
        Ok(json!({
            "pages": kept,
            "failed": failed,
            "complete": complete,
            "truncation": truncation,
            "provenance": self.provenance(),
        }))
    }

    async fn fetch(&self, input: Fetch) -> Result<Value> {
        let body = json!({"urls": input.urls, "format": "markdown", "extract_depth": "basic"});
        let answer = self.post("extract", &body).await?;
        self.pages(&answer, Some(&input.urls), None)
    }

    async fn crawl(&self, input: Crawl) -> Result<Value> {
        let mut body = Map::new();
        body.insert("url".into(), json!(input.url));
        body.insert("limit".into(), json!(input.limit));
        body.insert("format".into(), json!("markdown"));
        body.insert("extract_depth".into(), json!("basic"));
        body.insert("timeout".into(), json!(CRAWL_TIMEOUT_S));
        for (key, value) in [
            ("max_depth", input.max_depth.map(Value::from)),
            ("max_breadth", input.max_breadth.map(Value::from)),
            ("select_paths", input.select_paths.map(Value::from)),
            ("exclude_paths", input.exclude_paths.map(Value::from)),
            ("allow_external", input.allow_external.map(Value::from)),
            ("instructions", input.instructions.map(Value::from)),
        ] {
            if let Some(value) = value {
                body.insert(key.into(), value);
            }
        }
        let answer = self.post("crawl", &Value::Object(body)).await?;
        self.pages(&answer, None, Some(input.limit))
    }
}

#[async_trait]
impl Adapter for Tavily {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        connectors_sdk::validate(self.input_schema(operation)?, &input)?;
        match operation {
            "websearch.search" => self.search(decode(input)?).await,
            "websearch.fetch" => self.fetch(decode(input)?).await,
            "websearch.crawl" => self.crawl(decode(input)?).await,
            _ => Err(Error::new(ErrorCode::NotFound, "operation is not provided")),
        }
    }
}

/// `ms` since the Unix epoch as an RFC 3339 instant in UTC, millisecond precision.
pub fn rfc3339(ms: u64) -> String {
    let secs = ms / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn rfc3339_renders_utc_instants() {
        assert_eq!(super::rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            super::rfc3339(1_791_190_000_123),
            "2026-10-05T08:46:40.123Z"
        );
        assert_eq!(super::rfc3339(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }
}
