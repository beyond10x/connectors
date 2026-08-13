# 04 — The document carries the caller's contract

**Status:** landed with S-001 · **Date:** 2026-08-13 · **Story:** [S-001](../stories/S-001-the-document-carries-the-callers-contract.md)

The canonical document now stores, per operation, everything a caller or a model receives:
the caller-facing **symbol** beside every declared parameter, the model-facing **contract**
(`description` + lowered `input_schema`), the **`credential_requirement`** token, and — when a
provider declares one — the **`produces_credential`** minting join. This is the predecessor's
C-552, ported per decision 0026's named migration set, plus the two schema gaps M1 found in code
(`table.rs`'s two derivations). Five decisions the story left open are recorded here.

## 1. The port is an application, not a re-derivation

The M1 import pinned `flux-connectors` at `3650a136` — the direct parent of the predecessor's
reviewed C-552 commit — so the diff applied to the surviving crates verbatim:
`connector-resolve/src/document.rs` byte-identically, the document builder and schema with only
its two engine call sites replaced. The allocator (`names.rs`) is pure string code and moved
whole into `connector_spec::names`, which now **owns** the wire-name → symbol mapping; the one
genuine engine dependency — `OpSpec::lower`'s input-schema projection — is restated engine-free
in `catalog-build/src/contract.rs` as the closed narrowing it always was.

**The one-time S-001 differential** (the §7.6 pattern, run against the predecessor's C-552
regenerated documents rather than its pack): all **835 operations** and **1518 parameter
symbols** — `symbol`, `contract.description`, `contract.input_schema` — match the
engine-derived values exactly. Effects were excluded; they are S-002's.

## 2. `schema_version` stays 1

The story said "under a minor schema bump"; the field is a single `u32` with no minor component,
and the code's own rule (`document.rs::SCHEMA_VERSION`) is that **additive evolution does not
bump it** — a bump is what *refuses* older readers, which is exactly wrong for additions a
C-537-compatible reader must tolerate. The predecessor's C-552 made the same call. Every new
operation-level field is `required` in the schema — the validator describes this build's
artifacts, and schema + documents regenerate as one commit — while readers keep
`#[serde(default)]` fallbacks.

## 3. The extended description belongs to the artifact

The design edge C-552 recorded: the error-envelope-extended description states host-envelope
behaviour ("a non-2xx response is returned as data…"), so does it belong in the artifact or in a
projection layer? **In the artifact.** The platform's consumers (grant admission, the invocation
surface, any model-facing projection) must build the contract from document data alone —
architecture §5's rule — and a projection layer that composes prose would be a second author of
the contract, exactly the two-derivations disease S-001 exists to end. The document stores what
the declaration states; an operation with an empty summary and no envelope stores the empty
description it has.

## 4. `credential_requirement` is data, in C-206's tokens

The document's `auth` list is the *effective* requirement, which collapses "declared `auth = []`"
into "declared nothing anywhere". The distinction only exists at build time, where
`Operation::auth`'s `Option` still holds it — so the build publishes it:
`declared` / `no-credential-required` / `no-credential`, the predecessor's published status
tokens. `table.rs` now **reads** the token; the derivation from the connector default is gone,
and the pair of documents it could not tell apart is pinned as a fixture test.

## 5. The minting join is carried, not deferred

S-001 allowed deferral with a recorded reason; carrying the join was smaller than recording why
not. The operation-level `produces_credential { credential, secret }` mirrors the provider TOML
block, costs zero bytes on every shipped document (none declares one), and makes
`Acquisition::Minted` constructible from document data — C-136's property, *a caller can use a
credential it can never read*, now reaches the typed views. Conflicting provenance (OAuth2 and a
mint on one credential, two mints of one credential, a mint of an undeclared credential) is
refused by name at table build.

## What S-002 still owns

Per-operation host **effects** (the predecessor's fourth C-552 field) are deliberately absent:
S-002 rejects the predecessor's derived `[direction, "network"]` and demands declared effects
with a closed vocabulary and a grant-admission consumer — a curation pass over every provider,
not a port. The `semantic_effects` tier ships unchanged.
