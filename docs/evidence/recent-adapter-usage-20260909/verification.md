# Usage analysis verification

This checks the [report](../../recent-adapter-usage-20260909.md) and its evidence
inventory. It is not a provider call, ESS runtime conformance run or implementation
test of the proposed acceptance scenarios.

## Sources and method

The fixed interval and corpus/error counts are in [summary.json](summary.json).
The private local extraction retains a manifest with source record digests, exact
file/line tool and output indexes, earliest-occurrence deduplication identities,
and selected redacted excerpts. Some additionally hydrated excerpts are raw
private evidence. None of these private files belongs in a publication bundle.

The analysis used a temporary Rust crate under the ignored
`.local/tmp/session-intake-20260909/scanner/`, with cached dependencies and offline
builds. It read timestamped JSONL, decoded tool-use/output records, parsed submitted
Bash programs with tree-sitter, separately traced selected programmatic calls,
and exported only a restricted metadata schema. It never evaluated historical
programs. Source logs are not a complete process/network audit.

The private directory also retains the scanner source, source manifests, parse
audit, shell programs/sites and supplemental script candidates for local
reproduction. Re-running against changed/growing histories can change available
evidence even with the same fixed cutoff; compare the recorded source digests.
The manifest's SHA-256 feeds each nonempty raw JSONL line followed by LF; it
normalizes empty lines and a missing final newline. It is a record-stream digest,
not a byte-for-byte snapshot hash. It does not imply that every record in the
source file belongs to the window.

## Checks

- JSONL metadata keys, unique site coordinates, known in-window evidence IDs and
  per-client totals reconcile with the exported summary.
- All relative report/evidence links resolve. The report contains exactly 16
  workflow rows, 15 reviewed failure rows and 22 proposed acceptance rows.
- Programmatic Confluence and incremental collection calls are kept in a separate
  reviewed inventory, rather than added as guessed loop counts to shell statistics.
- Gzip integrity and whitespace checks pass. The archive is deterministic gzip
  without an embedded original filename or creation timestamp.
- Tracked evidence contains normalized operation names, timestamps and opaque IDs,
  not credential values, private source locations, command inputs, customer
  payloads or provider result bodies. Credential-pattern and private-identifier
  checks supplement manual review; a generic regex alone is not a privacy proof.
- AEP records are changed through the AEP CLI and the final store validation is
  retained in [aep-validate.log](aep-validate.log), with the local repository root
  normalized to `<repository>`. The verbatim output is retained privately and
  relayed to the operator.

The inventory-check.log file records the verifier's results.

The report's incompleteness disclosures are intentional: malformed source
records, dynamic scripts, truncated outputs, conditional sites and asynchronous
outcomes do not become fabricated successes. No success/failure rate is derived
from error-word matches or outer shell status.

## Scope of changes

Only analysis/evidence documents, a design-direction note and governed planning
records changed. No runtime, shared ESS domain, generated descriptor, website
publication manifest or provider configuration changed. The MCP epic records a
separate explicit operator requirement; it is not an implemented protocol binding.
