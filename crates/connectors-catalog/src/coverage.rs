//! What a bundle covers and what it does not, in one report a person reads and a
//! command compares. Coverage here is never a single number: the inventoried
//! count, the unsupported count and the source they were derived from travel
//! together, so no reader can quote a figure that hides the gaps beside it. No
//! ratio is computed — a ratio is the one rendering that can be quoted alone.
//!
//! Every string this module renders came out of a document somebody else wrote.
//! The text form is therefore built through one escape at the render boundary
//! (`escaped`, `quoted`); nothing is interpolated raw. A document that reaches
//! this module has already been accepted by `ingest`, and it is accepted because
//! it is a legal OpenAPI document, not because its strings are safe to print.

use crate::bundle::Bundle;
use crate::inventory::METHODS;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;

/// One reason a document could not be represented, with every operation it was
/// raised against. Grouped rather than listed per entry, because the same gap
/// across twenty operations is one thing to decide about, not twenty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonGroup {
    pub reason: String,
    /// The designations under this reason, in a stable order, one entry each —
    /// two operations raising the same gap are two entries, not one.
    pub designations: Vec<String>,
}

/// A method the inventory carries, and how many operations use it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MethodCount {
    pub method: String,
    pub operations: usize,
}

/// What a bundle covers. Every field is derived from the bundle alone; nothing
/// here reads the source document again or contacts anything.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub provider: String,
    /// The digest of the exact source bytes, so two reports can be compared
    /// without either one being trusted about which document it describes.
    pub source_sha256: String,
    /// The file the digest is of. It is rendered delimited beside the digest,
    /// because a file name is a document-supplied string like any other and an
    /// undelimited one could spell the rest of the provenance line itself.
    pub source_file_name: String,
    /// The document's own `openapi` string, as written.
    pub openapi: String,
    /// `info.version`, when the document carries one.
    pub info_version: Option<String>,
    pub inventoried: usize,
    pub unsupported: usize,
    pub reasons: Vec<ReasonGroup>,
    pub methods: Vec<MethodCount>,
    /// Operations the document identifies by method and path alone. They are
    /// inventoried like any other; the count says how many a caller cannot name.
    pub without_operation_id: usize,
}

/// Build the report for this bundle. An empty bundle reports zeroes: nothing
/// covered is an answer, and refusing to state it would leave the gap unsaid.
pub fn report(bundle: &Bundle) -> Report {
    let inventory = &bundle.inventory;
    let (inventoried, unsupported) = inventory.coverage();

    let mut grouped: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for gap in &inventory.unsupported {
        grouped
            .entry(gap.reason.as_str())
            .or_default()
            .push(gap.designation.as_str());
    }
    let reasons = grouped
        .into_iter()
        .map(|(reason, mut designations)| {
            // The inventory hands these over in the order the document declared
            // them; the report imposes its own, so two builds of one document
            // that differ only in declaration order still compare equal.
            designations.sort_unstable();
            ReasonGroup {
                reason: reason.to_owned(),
                designations: designations.into_iter().map(str::to_owned).collect(),
            }
        })
        .collect();

    let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
    for operation in &inventory.operations {
        *counted.entry(operation.method.as_str()).or_default() += 1;
    }
    let mut methods: Vec<MethodCount> = METHODS
        .iter()
        .filter_map(|method| {
            counted.remove(method).map(|operations| MethodCount {
                method: (*method).to_owned(),
                operations,
            })
        })
        .collect();
    // A bundle carrying a method outside `METHODS` is not reachable through this
    // crate's own producers — `inventory::extract` emits only `METHODS`, the
    // authored reader refuses anything else, and `bundle::load` checks the bytes
    // against the digest `bundle::write` recorded. This branch is for a bundle a
    // later build writes: such a method is reported after the ones this build
    // reads rather than dropped from a report whose purpose is naming gaps.
    methods.extend(counted.into_iter().map(|(method, operations)| MethodCount {
        method: method.to_owned(),
        operations,
    }));

    Report {
        provider: bundle.provider.clone(),
        source_sha256: bundle.source.source_sha256.clone(),
        source_file_name: bundle.source.file_name.clone(),
        openapi: bundle.source.openapi.clone(),
        info_version: bundle.source.info_version.clone(),
        inventoried,
        unsupported,
        reasons,
        methods,
        without_operation_id: inventory
            .operations
            .iter()
            .filter(|operation| operation.operation_id.is_none())
            .count(),
    }
}

/// A document-supplied string, made safe to put on a line of a report.
///
/// `ingest` and `inventory::extract` carry a document's bytes through unchanged,
/// and they are right to: a newline in a parameter name is legal JSON, a legal
/// parameter name, and a legal file name on Linux, so refusing one at ingest
/// would make a lawful OpenAPI document un-ingestible to buy a property that
/// belongs to the rendering. The escape is therefore here, at the one boundary
/// where the strings stop being data and start being lines a person reads. The
/// serialised form still carries the exact bytes.
fn escaped(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            // The escape character first, so the escaping stays reversible: a
            // document that writes the two characters `\` `n` and one that
            // writes a newline must not render alike.
            '\\' => out.push_str("\\\\"),
            // The delimiter `quoted` puts around a value, so no value can end
            // its own quoting and spell the rest of the line itself.
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // A percent sign read back out of this report cannot be told apart
            // from the report stating a ratio, and a stated ratio is the one
            // rendering this report exists in order not to produce.
            '%' => out.push_str("\\u{25}"),
            // Everything else a line can be broken or hidden with: the C0 and C1
            // controls, and the two separators `str::lines` does not split on
            // but a terminal or an editor may.
            '\u{2028}' | '\u{2029}' => {
                let _ = write!(out, "\\u{{{:x}}}", character as u32);
            }
            _ if character.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", character as u32);
            }
            _ => out.push(character),
        }
    }
    out
}

/// An escaped value with its extent marked. Two values share the provenance line
/// and the methods line; without delimiters one of them could spell the
/// separator and forge the other.
fn quoted(value: &str) -> String {
    format!("\"{}\"", escaped(value))
}

impl Report {
    /// The same value rendered for a person. The two counts share one line: a
    /// line that states either one states both, so neither can be read out of
    /// the report without the other coming with it. Every document-supplied
    /// string on every line goes through `quoted`, so no document can add a
    /// line, and the line count is a function of the value alone.
    pub fn text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "provider: {}", quoted(&self.provider));
        let _ = writeln!(
            out,
            "source: {} sha256 {}",
            quoted(&self.source_file_name),
            quoted(&self.source_sha256)
        );
        let _ = writeln!(out, "openapi: {}", quoted(&self.openapi));
        // The absent marker is the one unquoted value on these lines, so a
        // document declaring the version `not declared` is still distinguishable
        // from a document declaring none.
        let _ = writeln!(
            out,
            "info version: {}",
            match &self.info_version {
                Some(version) => quoted(version),
                None => "not declared".to_owned(),
            }
        );
        let _ = writeln!(
            out,
            "inventoried: {} operations; unsupported: {} entries",
            self.inventoried, self.unsupported
        );
        let _ = writeln!(
            out,
            "operations declaring no operationId: {}",
            self.without_operation_id
        );
        if self.methods.is_empty() {
            let _ = writeln!(out, "methods: none");
        } else {
            let listed: Vec<String> = self
                .methods
                .iter()
                .map(|m| format!("{} {}", quoted(&m.method), m.operations))
                .collect();
            let _ = writeln!(out, "methods: {}", listed.join(", "));
        }
        if self.reasons.is_empty() {
            let _ = writeln!(out, "reasons: none");
        } else {
            let _ = writeln!(out, "reasons:");
            for group in &self.reasons {
                // Two spaces for a reason, four for a designation under it: the
                // indentation is the block's structure, so neither value is
                // allowed to carry any of its own.
                let _ = writeln!(out, "  {}", quoted(&group.reason));
                for designation in &group.designations {
                    let _ = writeln!(out, "    {}", quoted(designation));
                }
            }
        }
        out
    }
}
