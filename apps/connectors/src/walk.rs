//! Walking a read's declared pages (story:invoke-walks-declared-pages;
//! `contracts/cli/v1alpha1/semantics.md`, "Walking pages").
//!
//! [`run`] is the loop: the input of each page is the caller's with the declared `query` and,
//! from the second page on, the position the page before named ([`Walk::page_input`]); each page
//! is one read the caller performs ([`Page`]); its records, the array at the declared `records`
//! pointer, are each shaped by `--fields` when given and handed, one page at a time, to a sink;
//! the declared evaluator ([`Walker`]) decides whether the walk goes on, ends by the name of an
//! end, or cannot go on. Nothing here reads, writes or calls anything: `invoke --walk` reads each
//! page through the same admitted path a plain invoke takes and writes NDJSON, and a caller that
//! keeps the records in memory passes a sink that refuses a page past its byte bound.
//!
//! The walk stops before its end at `max_pages`, on a refused read (a `rate_limited` one
//! included), on a catalog result whose status is not 2xx, when the sink refuses a page, or when
//! the evaluator names a page it cannot go on from. Records already handed over stay handed over;
//! `resume` is then the input of the page that would have been read next, when there is one.
use crate::shape::Fields;
use connectors_catalog_provider::walk::{Step, Walker};
use serde_json::{Map, Value, json};

pub use connectors_catalog_provider::walk::Walk;

/// The pages a walk reads when `--max-pages` is not given.
pub const DEFAULT_PAGES: u32 = 20;
/// The most pages one walk may be asked to read.
pub const MAX_PAGES: u32 = 10_000;

/// A `--max-pages` that is not a whole number from 1 to [`MAX_PAGES`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutOfRange;

/// `--max-pages` as given, [`DEFAULT_PAGES`] when it is not.
pub fn max_pages(text: Option<&str>) -> Result<u32, OutOfRange> {
    let Some(text) = text else {
        return Ok(DEFAULT_PAGES);
    };
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(OutOfRange);
    }
    match text.parse::<u32>() {
        Ok(pages) if (1..=MAX_PAGES).contains(&pages) => Ok(pages),
        _ => Err(OutOfRange),
    }
}

/// What one page's read answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Page {
    /// The provider result of an admitted read (for the catalog `{status, body, provenance}`).
    Answered(Value),
    /// The read was refused: the failure envelope's `error`, `{code, data}`.
    Refused(Value),
}

/// Why a walk stopped before one of its ends held.
#[derive(Clone, Debug, PartialEq)]
pub enum Stopped {
    /// `max_pages` pages were read and the last named a next position.
    MaxPages,
    /// A read was refused: the failure envelope's `error`.
    Refused(Value),
    /// A catalog result whose status is not 2xx: its records are not read.
    Status(u64),
    /// The evaluator cannot go on from the page (`Broken::name`); there is no resume.
    Broken(&'static str),
    /// The sink refused a page's records, by the name it gave.
    Sink(&'static str),
}

impl Stopped {
    /// The name the summary's `stopped` reports.
    pub fn name(&self) -> &'static str {
        match self {
            Self::MaxPages => "max_pages",
            Self::Refused(_) => "refused",
            Self::Status(_) => "status",
            Self::Broken(name) | Self::Sink(name) => name,
        }
    }
}

/// A walk's outcome.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Walked {
    /// The pages whose records the sink took.
    pub pages: u64,
    /// The records the sink took.
    pub records: u64,
    /// The name of the end that held; `None` when the walk stopped before one did.
    pub end: Option<String>,
    pub stopped: Option<Stopped>,
    /// The input of the page that would have been read next, when the walk can go on from it.
    pub resume: Option<Value>,
    /// The `--fields` paths no record had, as written, when there was a record.
    pub missing: Vec<String>,
}

impl Walked {
    /// Whether one of the declared ends held.
    pub fn complete(&self) -> bool {
        self.end.is_some()
    }

    /// The summary (`connectors.cli.InvokeWalked`), without the `out` the caller names.
    pub fn summary(&self) -> Map<String, Value> {
        let mut summary = Map::new();
        summary.insert("pages".into(), json!(self.pages));
        summary.insert("records".into(), json!(self.records));
        summary.insert("complete".into(), json!(self.complete()));
        if let Some(end) = &self.end {
            summary.insert("end".into(), json!(end));
        }
        if let Some(stopped) = &self.stopped {
            summary.insert("stopped".into(), json!(stopped.name()));
            match stopped {
                Stopped::Refused(error) => {
                    summary.insert("error".into(), error.clone());
                }
                Stopped::Status(status) => {
                    summary.insert("status".into(), json!(status));
                }
                _ => {}
            }
        }
        if let Some(resume) = &self.resume {
            summary.insert("resume".into(), resume.clone());
        }
        if !self.missing.is_empty() {
            summary.insert("missing".into(), json!(self.missing));
        }
        summary
    }
}

/// The document a walk's pointers read in a provider result: a catalog result's `body`, or
/// `Err` with its status when that is not 2xx; any other result is read whole.
fn page_body(result: &Value) -> Result<&Value, u64> {
    match (
        result.get("status").and_then(Value::as_u64),
        result.get("body"),
    ) {
        (Some(status), Some(_)) if !(200..300).contains(&status) => Err(status),
        (Some(_), Some(body)) => Ok(body),
        _ => Ok(result),
    }
}

/// Walk `walk` from the caller's input `caller`, reading at most `max_pages` pages.
///
/// `read` performs one page's read with the input it is given. `sink` takes one page's
/// records, each shaped by `fields` when given, all or nothing: an `Err` names why it took
/// none, and the walk stops with that page as `resume`.
pub fn run(
    walk: &Walk,
    caller: &Value,
    max_pages: u32,
    fields: Option<&Fields>,
    mut read: impl FnMut(&Value) -> Page,
    mut sink: impl FnMut(Vec<Value>) -> Result<(), &'static str>,
) -> Walked {
    let mut walker = Walker::new(walk);
    let mut walked = Walked::default();
    let mut position: Option<Value> = None;
    // The fields every record so far missed; `None` before the first record.
    let mut everywhere: Option<Vec<String>> = None;
    loop {
        let input = walk.page_input(caller, position.as_ref());
        if walked.pages >= u64::from(max_pages) {
            walked.stopped = Some(Stopped::MaxPages);
            walked.resume = Some(input);
            break;
        }
        let result = match read(&input) {
            Page::Answered(result) => result,
            Page::Refused(error) => {
                walked.stopped = Some(Stopped::Refused(error));
                walked.resume = Some(input);
                break;
            }
        };
        let body = match page_body(&result) {
            Ok(body) => body,
            Err(status) => {
                walked.stopped = Some(Stopped::Status(status));
                walked.resume = Some(input);
                break;
            }
        };
        let records = match walk.records(body) {
            Ok(records) => records,
            Err(broken) => {
                walked.stopped = Some(Stopped::Broken(broken.name()));
                break;
            }
        };
        let shaped: Vec<Value> = records
            .iter()
            .map(|record| match fields {
                Some(fields) => {
                    let shaped = fields.apply(record);
                    everywhere = Some(match everywhere.take() {
                        None => shaped.missing,
                        Some(before) => before
                            .into_iter()
                            .filter(|path| shaped.missing.contains(path))
                            .collect(),
                    });
                    Value::Object(shaped.value)
                }
                None => record.clone(),
            })
            .collect();
        let count = shaped.len() as u64;
        let step = walker.step(body);
        if let Err(name) = sink(shaped) {
            walked.stopped = Some(Stopped::Sink(name));
            walked.resume = Some(input);
            break;
        }
        walked.pages += 1;
        walked.records += count;
        match step {
            Ok(Step::End(name)) => {
                walked.end = Some(name);
                break;
            }
            Ok(Step::Next(next)) => position = Some(next),
            Err(broken) => {
                walked.stopped = Some(Stopped::Broken(broken.name()));
                break;
            }
        }
    }
    walked.missing = everywhere.unwrap_or_default();
    walked
}
