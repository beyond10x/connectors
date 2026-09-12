//! Adversarial cases for `story:mcp-outbound-connection-lifecycle`, written 2026-09-12
//! against commit `c1c5e16` of branch `unit/mcp-outbound-connection-lifecycle-20260912`.
//!
//! The unit's own check derives its state list from the story's acceptance sentence, so
//! it can see every state that sentence names and no state it does not. These cases
//! derive from the other two authorities instead — the pinned archives and the selection
//! matrix — and drive
//! `adapters/mcp/contracts/client/v1alpha1/semantics.md` against them.
//!
//! The load-bearing fact none of the unit's cases reads: the selection matrix
//! dispositions **two** revisions `supported` for this direction, `revision:2026-07-28`
//! and `revision:2025-11-25`. Sections 3, 10 and 11 of the document say which revision
//! they hold for. Sections 6 to 9 do not, and where the two revisions disagree the
//! document states the primary one's rule as if it were the transport's.
//!
//! Every premise below is asserted against an archive or against the matrix before the
//! claim that rests on it, so a case that goes green because a source moved says so
//! rather than passing quietly. No case restates a rule the document states; each names
//! a source that says something else.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const INTEROP: &str = "2025-11-25";

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn client_dir() -> PathBuf {
    repository_root().join("adapters/mcp/contracts/client/v1alpha1")
}

fn evidence_dir() -> PathBuf {
    repository_root().join("adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn semantics() -> String {
    read(&client_dir().join("semantics.md"))
}

fn selection_matrix() -> String {
    read(&repository_root().join("adapters/mcp/contracts/protocol/v1alpha1/selection.md"))
}

fn model() -> String {
    read(&repository_root().join("adapters/mcp/spec/ess/domains/state.yaml"))
}

/// One archived specification file, uncompressed, with every run of whitespace collapsed
/// so a probe phrase can straddle the archive's hard wrap.
fn archive(file: &str) -> String {
    let path = evidence_dir().join("vendor").join(format!("{file}.gz"));
    let output = Command::new("gzip")
        .arg("-dc")
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("run gzip -dc {}: {e}", path.display()));
    assert!(
        output.status.success(),
        "gzip -dc {} exited {}",
        path.display(),
        output.status
    );
    flat(&String::from_utf8(output.stdout).expect("archived source is not UTF-8"))
}

fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A section of `document` from `heading` up to the next heading of the same or a
/// shallower level. The heading line is part of it.
fn section(document: &str, heading: &str) -> String {
    let depth = heading.chars().take_while(|c| *c == '#').count();
    assert!(depth > 0, "`{heading}` is not a heading");
    let start = document
        .find(heading)
        .unwrap_or_else(|| panic!("semantics.md carries no section `{heading}`"));
    let mut collected = String::new();
    for (index, line) in document[start..].lines().enumerate() {
        let level = line.chars().take_while(|c| *c == '#').count();
        if index > 0 && level > 0 && level <= depth && line[level..].starts_with(' ') {
            break;
        }
        collected.push_str(line);
        collected.push('\n');
    }
    collected
}

/// Paragraphs of a document as `(first line number, text with the wrap collapsed)`.
fn paragraphs(document: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut buffer: Vec<&str> = Vec::new();
    let mut start = 0usize;
    for (index, line) in document.lines().enumerate() {
        if line.trim().is_empty() {
            if !buffer.is_empty() {
                found.push((start + 1, flat(&buffer.join(" "))));
                buffer.clear();
            }
        } else {
            if buffer.is_empty() {
                start = index;
            }
            buffer.push(line);
        }
    }
    if !buffer.is_empty() {
        found.push((start + 1, flat(&buffer.join(" "))));
    }
    found
}

/// The fields `connectors_mcp.state.McpServerBinding` declares, as `(name, type)`, read
/// out of the model rather than written here.
fn binding_fields() -> Vec<(String, String)> {
    let text = model();
    let start = text
        .find("- name: connectors_mcp.state.McpServerBinding")
        .expect("the MCP model declares no `McpServerBinding`");
    let rest = &text[start..];
    let end = rest
        .find("\n    lifecycle:")
        .expect("`McpServerBinding` declares no lifecycle, so its field block has no end");
    let field = regex::Regex::new(r#"\{name: ([a-z_]+), type: "?([A-Za-z0-9_.<>]+)"?\}"#).unwrap();
    field
        .captures_iter(&rest[..end])
        .map(|capture| (capture[1].to_string(), capture[2].to_string()))
        .collect()
}

/// Whether the selection matrix dispositions `key` `supported` in the outbound direction.
fn supported_outbound(key: &str) -> bool {
    selection_matrix().lines().any(|line| {
        line.contains(key) && line.contains("| outbound |") && line.contains("| supported |")
    })
}

/// The scenario file a register row names for `outcome`, from the register itself.
fn scenario_of(outcome: &str) -> String {
    let link = regex::Regex::new(r"\(scenarios/([A-Za-z0-9._-]+\.yaml)\)").unwrap();
    let document = semantics();
    for line in document.lines() {
        if !line.trim_start().starts_with('|') || !line.contains(outcome) {
            continue;
        }
        if let Some(capture) = link.captures(line) {
            return capture[1].to_string();
        }
    }
    panic!("no register row names `{outcome}` with a scenario file");
}

fn scenario_value(file: &str) -> serde_yaml_ng::Value {
    let path = client_dir().join("scenarios").join(file);
    let text = read(&path);
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("{} is not YAML: {e}", path.display()))
}

fn joined(value: &serde_yaml_ng::Value, key: &str) -> String {
    match &value[key] {
        serde_yaml_ng::Value::String(text) => flat(text),
        serde_yaml_ng::Value::Sequence(items) => flat(
            &items
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        ),
        _ => String::new(),
    }
}

/// Whether a passage resolves which pinned revision it holds for. The document's own
/// name for the second one is "the interoperability revision", and sections 3, 10 and 11
/// use it; a passage that names either that word or the version string has answered the
/// question, whichever answer it gives.
fn names_the_second_revision(passage: &str) -> bool {
    passage.contains(INTEROP) || passage.to_lowercase().contains("interoperability")
}

// ── Cases ───────────────────────────────────────────────────────────────────────────

/// Section 9 states one cancellation signal for "Streamable HTTP" and names no revision.
/// The two revisions the matrix dispositions `supported` outbound give opposite signals:
/// under the primary one, closing the stream **is** cancellation; under the
/// interoperability one, disconnection **SHOULD NOT** be read as cancellation and the
/// client **SHOULD** send `notifications/cancelled` — which the document says is "not
/// expected".
#[test]
fn cancellation_says_which_revision_its_signal_holds_for() {
    let primary = archive("mcp-2026-07-28-basic-transports-streamable-http.mdx");
    assert!(
        primary.contains(
            "Closing the SSE response stream **MUST** be treated by the server as \
             cancellation of that request."
        ),
        "the primary revision's archive no longer states the signal this case compares \
         against, so this comparison has drifted from the pin"
    );
    let interop = archive("mcp-2025-11-25-basic-transports.mdx");
    for contrary in [
        "Disconnection **SHOULD NOT** be interpreted as the client cancelling its request.",
        "To cancel, the client **SHOULD** explicitly send an MCP `CancelledNotification`.",
    ] {
        assert!(
            interop.contains(contrary),
            "the interoperability revision's archive no longer says {contrary:?}, so this \
             case's premise has drifted from the pin"
        );
    }
    assert!(
        supported_outbound(&format!("`revision:{INTEROP}`")),
        "the selection matrix no longer dispositions `revision:{INTEROP}` supported \
         outbound, so no binding reaches the contrary rule"
    );

    let document = semantics();
    let cancellation = section(&document, "## 9. Cancellation");
    assert!(
        cancellation.contains("no `notifications/cancelled`"),
        "section 9 no longer carries the claim this case drives; re-read it"
    );

    let scenario = scenario_of("mcp.outbound.cancelled-by-closing-the-stream");
    let value = scenario_value(&scenario);
    let trace = format!(
        "{} {} {} {}",
        joined(&value, "summary"),
        joined(&value, "given"),
        joined(&value, "when"),
        joined(&value, "then")
    );

    let mut unresolved = Vec::new();
    if !names_the_second_revision(&cancellation) {
        unresolved.push(
            "semantics.md:357 `## 9. Cancellation` states the primary revision's signal \
             for \"Streamable HTTP\" and names no revision: it says at :361 that no \
             `notifications/cancelled` message is expected, while \
             mcp-2025-11-25-basic-transports.mdx:130-131 says disconnection SHOULD NOT be \
             read as cancellation and the client SHOULD send one"
                .to_string(),
        );
    }
    if !names_the_second_revision(&trace) {
        unresolved.push(format!(
            "{scenario} asserts that closing the stream is cancellation with no revision \
             in its `given`, so it reads as true for a binding the matrix dispositions \
             supported and for which it is false"
        ));
    }
    assert!(
        unresolved.is_empty(),
        "{} passages state a cancellation signal that is wrong for a revision this \
         repository selected, without saying which revision they hold for:\n  {}",
        unresolved.len(),
        unresolved.join("\n  ")
    );
}

/// Under the interoperability revision a server **MAY** send JSON-RPC requests on the
/// stream, and `ping` is one the receiver **MUST** answer. The register names no outcome
/// for it, and section 7 routes every request arriving on a stream to
/// `mcp.outbound.framing-refused` — an outcome whose answer is to refuse it.
#[test]
fn a_request_the_interoperability_revision_makes_this_client_answer_has_an_outcome() {
    let interop_transport = archive("mcp-2025-11-25-basic-transports.mdx");
    assert!(
        interop_transport.contains(
            "The server **MAY** send JSON-RPC _requests_ and _notifications_ before sending \
             the JSON-RPC _response_."
        ),
        "the interoperability revision's archive no longer permits a server request on \
         the response stream, so this case's premise has drifted from the pin"
    );
    let ping = archive("mcp-2025-11-25-basic-utilities-ping.mdx");
    assert!(
        ping.contains("The receiver **MUST** respond promptly with an empty response"),
        "the interoperability revision's archive no longer requires an answer to `ping`, \
         so this case's premise has drifted from the pin"
    );
    let changelog = archive("mcp-2026-07-28-changelog.mdx");
    assert!(
        changelog.contains("Remove `ping`"),
        "the primary revision's changelog no longer records the removal of `ping`, so the \
         two revisions may no longer disagree here"
    );
    assert!(
        supported_outbound(&format!("`revision:{INTEROP}`")),
        "the selection matrix no longer dispositions `revision:{INTEROP}` supported \
         outbound, so no binding reaches the server request"
    );

    let document = semantics();
    let streaming = section(&document, "### `mcp.outbound.stream-opened`");
    assert!(
        flat(&streaming).contains("A JSON-RPC *request* arriving on the stream is refused"),
        "the outcome no longer carries the blanket refusal this case drives; re-read \
         section 7"
    );

    let mentions_ping = regex::Regex::new(r"(?i)\bping\b")
        .unwrap()
        .is_match(&document);
    assert!(
        mentions_ping || names_the_second_revision(&streaming),
        "semantics.md:313-315 refuses every JSON-RPC request arriving on a stream as \
         `upstream_protocol` \"because this revision defines no channel for it\", and \
         names no revision. For a binding configured for {INTEROP} — supported outbound \
         by the selection matrix — mcp-2025-11-25-basic-transports.mdx:121 permits the \
         server to send requests on that stream and \
         mcp-2025-11-25-basic-utilities-ping.mdx:31 makes answering `ping` a MUST. The \
         register carries no outcome for a server request this client must answer, and \
         the document names `ping` nowhere."
    );
}

/// The story says a reviewer can check that no sentence implies a durable binding record.
/// `determined_era` is a field of the `McpServerBinding` entity, and the document both
/// declares it unchanged by an observation and states that it is recorded from one.
#[test]
fn no_binding_field_is_both_unchanged_by_an_observation_and_written_from_one() {
    let fields = binding_fields();
    assert!(
        fields.iter().any(|(name, _)| name == "determined_era"),
        "the model no longer declares the field this case reads; it now declares {fields:?}"
    );
    let versioning = archive("mcp-2026-07-28-basic-versioning.mdx");
    assert!(
        versioning.contains(
            "attempt a modern request and inspect the body of a `400 Bad Request` before \
             falling back."
        ),
        "the archive no longer states the era probe for this transport, so the two \
         document sentences below may no longer be about one act"
    );

    let document = semantics();
    let unchanged = regex::Regex::new(r"(?i)unchanged|byte-identical").unwrap();
    let written = regex::Regex::new(r"(?i)\bis recorded\b|\bare recorded\b|\bis set\b").unwrap();

    let mut collisions = Vec::new();
    for (name, kind) in &fields {
        let mut declared_unchanged: Vec<usize> = Vec::new();
        let mut declared_written: Vec<usize> = Vec::new();
        for (line, text) in paragraphs(&document) {
            if !text.contains(name.as_str()) {
                continue;
            }
            if unchanged.is_match(&text) {
                declared_unchanged.push(line);
            }
            if written.is_match(&text) {
                declared_written.push(line);
            }
        }
        if !declared_unchanged.is_empty() && !declared_written.is_empty() {
            collisions.push(format!(
                "`{name}: {kind}` is declared unchanged by an observation at semantics.md:\
                 {declared_unchanged:?} and written from one at semantics.md:\
                 {declared_written:?}; it is a field of the `McpServerBinding` entity, \
                 whose identity is `binding_ref`, so the second sentence writes a \
                 wire-derived value onto the binding and the first says it cannot happen"
            ));
        }
    }
    assert!(
        collisions.is_empty(),
        "{} binding fields are both held constant and written by this document:\n  {}",
        collisions.len(),
        collisions.join("\n  ")
    );
}

/// Section 6 says the server answers a request with either a single JSON object or an SSE
/// stream and that the client supports both. Section 10 routes a dropped connection
/// carrying a request in flight to `mcp.outbound.stream-ended-without-an-answer`, whose
/// own scenario presupposes an open SSE stream. An answer that was never streamed and
/// never arrived therefore reaches no row of the register.
#[test]
fn an_answer_that_was_never_streamed_can_also_be_lost() {
    let document = semantics();
    assert!(
        flat(&document).contains(
            "The server answers a request with either a single JSON object or an SSE \
             stream and the client supports both."
        ),
        "section 6 no longer names two answer shapes, so this case's premise has drifted"
    );
    let dropped: Vec<(usize, String)> = paragraphs(&document)
        .into_iter()
        .filter(|(_, text)| text.contains("a dropped connection"))
        .collect();
    assert_eq!(
        dropped.len(),
        1,
        "section 10 no longer carries exactly one paragraph routing a dropped connection"
    );
    let (routing_line, routing) = &dropped[0];

    let outcome = regex::Regex::new(r"mcp\.outbound\.[a-z-]+").unwrap();
    let named: BTreeSet<String> = outcome
        .find_iter(routing)
        .map(|found| found.as_str().to_string())
        .collect();
    assert!(
        !named.is_empty(),
        "semantics.md:{routing_line} routes a dropped connection to no named outcome"
    );

    let streamed = regex::Regex::new(r"(?i)sse|stream").unwrap();
    let mut narrower = Vec::new();
    for name in &named {
        let file = scenario_of(name);
        let value = scenario_value(&file);
        let given = joined(&value, "given");
        let body = format!("{} {}", given, joined(&value, "then"));
        if streamed.is_match(&given) {
            narrower.push(format!(
                "{file} answers `{name}` only for a request already being streamed: \
                 given {given:?}"
            ));
        } else if body.contains("No request was framed") {
            narrower.push(format!(
                "{file} answers `{name}` only where nothing was framed: {body:?}"
            ));
        }
    }
    assert!(
        narrower.len() < named.len(),
        "semantics.md:{routing_line} routes every dropped connection to {} outcomes, and \
         all {} of them are scoped to a request that was either streamed or never \
         framed. A request answered with the single JSON object section 6 says the \
         client supports, whose connection dies before that object arrives, reaches no \
         row of the register:\n  {}",
        named.len(),
        narrower.len(),
        narrower.join("\n  ")
    );
}

/// The outbound transport option space has four families, not two, and the matrix
/// dispositions two of the other three `explicitly refused` under no blocker at all.
/// Section 9 calls the remainder "the other outbound transport family" and attributes it
/// to the stdio blocker.
///
/// The unit's own `outbound_stdio_is_held_by_its_blocker_and_specified_nowhere` cannot
/// see this: it exempts any line that names the blocker, so a line naming the blocker may
/// say anything at all about any transport.
#[test]
fn the_document_does_not_call_a_four_family_transport_space_a_pair() {
    let matrix = selection_matrix();
    let key = regex::Regex::new(r"\| `(transport:[a-z-]+)` \| (outbound|both) \|").unwrap();
    let outbound: BTreeSet<String> = key
        .captures_iter(&matrix)
        .map(|capture| capture[1].to_string())
        .collect();
    assert!(
        outbound.len() > 2,
        "the selection matrix names {} outbound transport families, so a document calling \
         the remainder \"the other\" one would be right: {outbound:?}",
        outbound.len()
    );

    let document = semantics();
    let pair = regex::Regex::new(r"(?i)the other (outbound )?transport family").unwrap();
    let mut found = Vec::new();
    for (index, line) in document.lines().enumerate() {
        if pair.is_match(line) {
            found.push(format!("semantics.md:{}: {}", index + 1, line.trim()));
        }
    }
    assert!(
        found.is_empty(),
        "the matrix dispositions {} outbound transport families — {outbound:?} — and two \
         of them are `explicitly refused` naming no blocker, so attributing \"the other\" \
         family to `decision-blocker:mcp-outbound-stdio-process-ownership` states a \
         disposition for transports this document does not own:\n  {}",
        outbound.len(),
        found.join("\n  ")
    );
}

/// Section 2 says a selection may name "the ordered pair of revisions" the binding may
/// speak; its own outcome says an observer sees "one revision the binding will declare on
/// every request"; and the model carries one scalar `selected_revision` and no second
/// carrier. An operator cannot be told which of the two they configured.
#[test]
fn a_selection_names_as_many_revisions_as_the_outcome_and_the_model_can_hold() {
    let fields = binding_fields();
    let revision_carriers: Vec<&(String, String)> = fields
        .iter()
        .filter(|(name, _)| name.contains("revision"))
        .collect();
    assert_eq!(
        revision_carriers.len(),
        1,
        "the model now declares {} revision carriers on `McpServerBinding`, so a pair may \
         have somewhere to go: {revision_carriers:?}",
        revision_carriers.len()
    );
    let (name, kind) = revision_carriers[0];
    assert!(
        !kind.contains("List<"),
        "`{name}` is now `{kind}`, which can hold more than one revision"
    );

    let document = semantics();
    let selection = section(&document, "## 2. Explicit selection of a server");
    let flat_selection = flat(&selection);
    let says_pair = flat_selection.contains("the ordered pair of revisions");
    let outcome = flat(&section(&document, "### `mcp.outbound.server-selected`"));
    let says_one = outcome.contains("one revision the binding will declare on every request");

    assert!(
        !(says_pair && says_one),
        "semantics.md:95-97 says a selection names \"the endpoint, the transport binding \
         and the revision — or the ordered pair of revisions — the binding may speak\", \
         while semantics.md:110-111 says an observer sees \"one revision the binding will \
         declare on every request\", and `{name}: {kind}` is the model's only carrier, \
         which holds one string. Whether a dual-era binding is configurable — the thing \
         sections 3 and 4 both rest on — is stated both ways and observable neither."
    );
}
