//! One run from a pinned document to an indexed bundle. This module sequences
//! the four that do the work — ingest, inventory, coverage, bundle — and adds no
//! behaviour of its own beyond the order and the record.

use crate::bundle::{self, Bundle, Entry};
use crate::coverage::{self, Report};
use crate::inventory;
use crate::{Refusal, SourceRecord, ingest};
use connectors_core::Error;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The steps of a run, in the order they are taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    Read,
    Ingest,
    Extract,
    Report,
    Write,
}

impl Step {
    /// How a step is named when a refusal is rendered. The same five words the
    /// serialised form uses, so a record read as text and one read as JSON name
    /// the step alike.
    pub fn label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Ingest => "ingest",
            Self::Extract => "extract",
            Self::Report => "report",
            Self::Write => "write",
        }
    }
}

/// The order, stated once. A record lists every step, including the ones a
/// refusal never reached.
pub const ORDER: [Step; 5] = [
    Step::Read,
    Step::Ingest,
    Step::Extract,
    Step::Report,
    Step::Write,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Progress {
    Completed,
    Refused,
    NotReached,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepRecord {
    pub step: Step,
    pub progress: Progress,
}

/// What to run over what.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub provider: &'a str,
    pub source: &'a Path,
    pub directory: &'a Path,
    pub auth_profile: &'a str,
    pub replace: bool,
}

/// What a completed run did.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub source: SourceRecord,
    pub coverage: Report,
    pub entry: Entry,
    pub bundle_path: PathBuf,
    pub steps: Vec<StepRecord>,
}

/// A refusal, as the refusing step's own error with the step beside it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub step: Step,
    pub error: Error,
    pub source: Option<SourceRecord>,
    pub coverage: Option<Report>,
    pub steps: Vec<StepRecord>,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} step refused: {}", self.step.label(), self.error)
    }
}

impl std::error::Error for Failure {
    /// The refusing module's own error, reachable as the cause. A caller that
    /// composes this run with anything else reaches it with `?`, and what it
    /// finds there is that module's error rather than a rendering of it.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// The five steps with their outcome. A refusal names its step; every step after
/// it is reported unreached rather than omitted, so a reader of a failed record
/// sees the same five entries a completed one carries.
fn progression(refused: Option<Step>) -> Vec<StepRecord> {
    let stop = refused.and_then(|step| ORDER.iter().position(|candidate| *candidate == step));
    ORDER
        .iter()
        .enumerate()
        .map(|(position, step)| StepRecord {
            step: *step,
            progress: match stop {
                None => Progress::Completed,
                Some(stop) if position < stop => Progress::Completed,
                Some(stop) if position == stop => Progress::Refused,
                Some(_) => Progress::NotReached,
            },
        })
        .collect()
}

/// The refusing step's error, carried out with whatever the earlier steps
/// produced. The error is the one that module raised; nothing here rewrites it,
/// because a second vocabulary for the same refusal is a second answer to why a
/// bundle was not written.
///
/// It travels boxed: a refusal's record carries as much as a completed run's, and
/// an unboxed one would widen every `Ok` this function returns by the size of a
/// failure that did not happen.
fn refused(
    step: Step,
    error: Error,
    source: Option<SourceRecord>,
    coverage: Option<Report>,
) -> Box<Failure> {
    Box::new(Failure {
        step,
        error,
        source,
        coverage,
        steps: progression(Some(step)),
    })
}

/// Take the document at `request.source` to an indexed bundle.
///
/// The write is last, and it is the only step that touches `request.directory`:
/// a refusal anywhere before it has not created the directory, written a bundle
/// or read — let alone rewritten — the index, so the directory is left exactly
/// as it was. The ordering is the whole guarantee; there is no cleanup path here
/// to undo a partial write, because no earlier step can make one.
pub fn run(request: &Request<'_>) -> std::result::Result<Run, Box<Failure>> {
    let unreadable = || refused(Step::Read, Refusal::Unreadable.into(), None, None);
    let bytes = std::fs::read(request.source).map_err(|_| unreadable())?;
    // The recorded name is the file's own, not the path it was read through, so
    // a record does not carry a home directory — the same rule `ingest_file`
    // states, applied here because this run needs the bytes as well.
    let file_name = request
        .source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(unreadable)?;

    let source = ingest(file_name, &bytes)
        .map_err(|refusal| refused(Step::Ingest, refusal.into(), None, None))?;

    // The same bytes, read once more: `inventory::extract` reads the document
    // rather than a shape cached by the ingest.
    //
    // This arm cannot fire. `ingest` above put these exact bytes — the ones in
    // `bytes`, not a re-read of the file — through this same
    // `connectors_core::read_json` and returned `Ok`, and `read_json` is a pure
    // function of the bytes. It is written as a refusal rather than an unwrap
    // because an unwrap would make an impossibility a panic, and it is attributed
    // to its step like every other, so no refusal can reach a caller without one.
    let document = connectors_core::read_json(&bytes)
        .map_err(|error| refused(Step::Extract, error, Some(source.clone()), None))?;
    let bundle = Bundle {
        provider: request.provider.to_owned(),
        source: source.clone(),
        inventory: inventory::extract(&document),
        auth_profile: request.auth_profile.to_owned(),
    };

    let report = coverage::report(&bundle);

    let entry = bundle::write(request.directory, &bundle, request.replace).map_err(|error| {
        refused(
            Step::Write,
            error,
            Some(source.clone()),
            Some(report.clone()),
        )
    })?;

    Ok(Run {
        bundle_path: request.directory.join(&entry.file_name),
        source,
        coverage: report,
        entry,
        steps: progression(None),
    })
}
