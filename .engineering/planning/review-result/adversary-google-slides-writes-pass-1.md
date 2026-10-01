---
format: aep.planning-md/3
id: review-result:adversary-google-slides-writes-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Google Slides writes
relations:
- reviews: story:catalog-google-slides-writes
revision: 1
---
unit: story:catalog-google-slides-writes, impl/catalog-google-slides-writes at 03268683f plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 108→110, red 2
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: none

**1. Diff proof.** `git --no-pager diff --stat` is empty because I changed no tracked file. `git status --short` shows one new path, and it is a test file:
```
?? adapters/catalog/tests/google_slides_writes_adversary.rs
```

**2. Cases added** (in `adapters/catalog/tests/google_slides_writes_adversary.rs`; rustfmt and `clippy -D warnings` pass). Each ran alone first and was red:

- `a_batch_update_narrowed_with_fields_that_google_applied_is_applied` (red). It sends `batchUpdate` with the pinned revision and `fields: "replies"`, and asserts that a 200 from Google is reported as applied. The preflight and query checks inside the case pass first. The case then fails:
  ```
  panicked at adapters/catalog/tests/google_slides_writes_adversary.rs:152:43:
  a batchUpdate Google answered 200 for was not reported applied: UpstreamProtocol write acknowledged with a value at `/presentationId` other than the pinned one; the effect is possible
  ```
- `the_write_configuration_keeps_the_binding_connections_repair_requires` (red). It builds the guide's read configuration and the guide's write configuration through `--print-local-bootstrap`, starts a connection under the read one in a real registry, then starts one under the write one. It asserts that the binding is unchanged and that the second start is accepted:
  ```
  panicked at adapters/catalog/tests/google_slides_writes_adversary.rs:239:5:
  the write configuration changes the connection binding in ["configuration_revision", "profile"], which `connections repair` refuses as an identity mismatch, and a new acquisition on the same instance is refused as Some(Conflict)
  ```

**3. Suite run** (after the cases existed): `cargo test -p connectors-catalog-provider --no-fail-fast`, exit 101. Every other binary passes; `google_slides` passes 12, `local_runtime` passes 30 with 6 ignored. My binary: `test result: FAILED. 0 passed; 2 failed`. That makes 110 executed; 108 is the same run without my binary.

**4. Findings**

| # | file:line | what was measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| F1 | docs/catalog-google-slides.md:157-166 | The guide says `connections repair` adds the write scope to an existing connection. The write configuration it prescribes changes `configuration_revision` (the provider hashes `auth` at `adapters/catalog/src/local.rs:703`) and `profile.minimum_scopes`. `begin_repair` refuses any change to the binding as `IdentityMismatch` (`crates/connectors-host/src/local/registry/lifecycle.rs:52`; this is from reading the code, since repair needs Secret Service). A fresh connection on the same instance is refused as `Conflict` (lifecycle.rs:288-291; measured). | An agent following the guide's own write-setup paragraph. What works is a new instance id with `connections connect`. | NEEDS-CHANGE / introduced |
| F2 | adapters/catalog/providers/google-slides/operations.json:19 | The postflight `/presentationId` check treats a missing field as a mismatch. With `fields` narrowed, Google's 200 comes back as `Unknown` (`adapters/catalog/src/lib.rs:577`). | `docs/catalog-google-slides.md:86` says `fields` is accepted on every operation, and the bundle declares it on `batchUpdate`. No documented workflow passes it. Fix: warn in the guide that `fields` on `batchUpdate` must keep `presentationId`. | CONFIRMED / introduced |
| F3 | adapters/catalog/tests/google_slides.rs:1138 | `a_write_approved_for_a_different_body_is_refused` never sends a proof through the owner. The write at :1164 carries no approval, and the subject at :1096 is a hand copy of `approval_issuance.rs:281`. The test would stay green if the owner hashed less than the whole input. The real binding does hold: issuance hashes the whole input at approval_issuance.rs:281, and execution.rs:166 re-verifies it. | The Acceptance line "a write with an approval for a different body is refused". | CONFIRMED / introduced |
| F4 | adapters/catalog/src/lib.rs:496 | When the preflight answer has no `revisionId` (per the pinned document, a user without edit access), the refusal code is `upstream_protocol`. That reads as a provider fault, not a permission problem. | Any view-only connection used for `batchUpdate`. `lib.rs` is byte-identical at the base. | CONFIRMED / pre-existing |

**5. Attacked and not broken**
- A missing `writeControl` or a null `requiredRevisionId` is refused before any request (lib.rs:457-460, the unit's test at google_slides.rs). An empty string `""` reaches the preflight and is refused as different. Objects and arrays are refused because they are not scalars.
- Type coercion: `scalar()` turns numbers and booleans into strings on purpose (the GitLab `pipeline_id` guard relies on it). No bypass exists, because the pinned document says revision ids are "a nebulous string", and Google checks `requiredRevisionId` again when it applies the batch.
- Splitting the reference path on `.`: `Value::get` looks up one key at a time, so a body key named `"writeControl.requiredRevisionId"` does not match. A top-level `body.*` key is blocked by `additionalProperties:false`.
- Postflight on another deck: the preflight and the POST fill the same `presentationId` into the same kind of template, so the check cannot be satisfied by a different deck. A `/` in the id is escaped in both.
- `create` cannot copy or import from another file: the body is only checked to be an object, and the pinned document says every field other than `title` and `presentationId` is ignored.
- A preflight answer over 4 MiB fails with `capacity` at `connectors-client/src/lib.rs:147-162` and is recorded `NotAttempted` (execution.rs:275). The guide's claim that it is refused before dispatch is accurate.
- Scope claims: the write scope covers both writes and all three reads, and `presentations.readonly` covers neither write. Insufficient scope maps to `not_granted` with `repair_connection` (`apps/connectors/src/local/connections.rs:151`).

**6. Paths written outside the worktree**
- `~/.cache/w0930ws/adv1/` (scratch directory, mode 700)
- `~/.cache/w0930ws/adv1/suite.log`

**7. Findings block**
```findings
- file: docs/catalog-google-slides.md
  line: 164
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The documented connections repair route cannot add the write scope, because the prescribed write configuration changes the connection binding, which begin_repair refuses as an identity mismatch, and a new acquisition on the same instance is refused as Conflict."
- file: adapters/catalog/providers/google-slides/operations.json
  line: 19
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "A batchUpdate with a fields value that omits presentationId is reported Unknown even when Google answered 200, because the postflight treats the missing field as a mismatch, and the guide does not warn about it."
- file: adapters/catalog/tests/google_slides.rs
  line: 1138
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "The approval-binding test checks the approvals library against a hand-copied subject and writes with no proof, so it cannot fail if the owner stops hashing the whole input."
- file: adapters/catalog/src/lib.rs
  line: 496
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "A preflight answer with no revisionId, which the pinned document says means the user lacks edit access, is refused as upstream_protocol, which reads as a provider fault rather than a permission refusal."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
