---
format: aep.planning-md/3
id: review-result:standing-grant-local-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:local-invoke-admits-a-keyed-write-under-a-standing-grant
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
revision: 1
---
verdict: needs-revision

I read the code the Design names. Where a line below is my inference, it says so. Nothing was built or run.

```findings
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  line: 372
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Read: execution.rs re-verifies the proof at :375, :424-432 (spend), :440 and :502 (final_check) up to the commit, but the Design replaces only :167-172. A proof lives 295 s, so those rechecks are cheap; a grant lives long, and revocation, expiry or key rotation during the preflight (up to 120 s) would be admitted. The Design should name each site and re-read the file, the revoked set in the uses store, the key and expiry at :373, :440 and :500, and say that open_dispatch is the commit point and a revoke after it does not stop that write."
- file: crates/connectors-host/src/local/owner/transport.rs
  line: 2640
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Read: an empty proof frame becomes None here, and protected.rs:286-308 file_until accepts an empty --approval-file, so an empty file plus a key on an instance with standing_grants would take the grant path, contradicting 'with --approval-file every byte as today'. Read: the CLI gate at invoke.rs:1155 is client-side only and Design step 3 never requires a key at the owner. The Design should say the owner takes the grant path only with key Some and proof None, the CLI refuses an empty approval file as today, and a present proof never falls back to a grant."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "grant-standing and revoke-standing are read-modify-write then rename with no lock, so two concurrent commands lose an update and a revocation can be undone by a grant-standing that read the older file. The Design should state an exclusive lock (the Lease pattern of approval_keys.rs), the order 'write the revoked id to the uses store first, then publish the file', publish_replace (filesystem.rs:516), the API Uses gains (revoked table, checked inside the take transaction), and a rule for the 256-id revoked cap (standing.rs:135). Replay from the CLI (operations.rs:223, before any owner) must read grant_of read-only and never create the store."
- file: crates/connectors-host/src/local/mutations.rs
  line: 300
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Q2 is unanswered. Under 'prepare in place of prepare_approved' the attempt records approval_mode='not_required' (candidate() defaults to it, mutation.rs:382). The CHECK at mutations.sql:21 bars an approval_ref with it, and the dispatch gate at :300 admits any not_required attempt with no live-approval check, unlike :552. The grant id then lives only in a second sqlite file, with no atomicity with prepare. The Design should choose and state: keep not_required (parity with the served host, and say the ledger cannot tell) or add a 'standing' mode with approval_ref = grant id and a live check at dispatch. I recommend the third mode. It needs a schema migration and ESS variant, and a ledger actor beside ServiceHost (standing_approvals.yaml:151-156)."
- file: crates/connectors-host/src/local/approval_keys.rs
  line: 641
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Read: with_signer opens the key from custody for any process of the operator's UID, and approval-issue runs in the CLI (local.rs:293-301). Inferred: a caller of invoke with the same UID can run grant-standing itself, with no ceremony and with not_after and max_per_hour up to 10,000 (standing.rs:47) where a proof is 295 s and single-use. The Design should state that boundary (see cli semantics.md:117 'same-user boundary'), require the prepare-then-approved step that approval-issue has, bound the not_after horizon, and give the operator a way to list grants."
- file: crates/connectors-host/src/local/approvals/proof.rs
  line: 104
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Read: proof Claims and Header are deny_unknown_fields and TYPE is checked (:22-55), so a grant cannot verify as a proof, and a proof cannot verify as a grant if the new decoder checks its own typ. Gaps: the Design must reuse the key window checks of public_key (:104-122) rather than say 'active key'. The local path must never accept a bare unsigned standing::read document (downgrade). The grant binds neither connection nor configuration_revision (the subject does, resolve :274-285), so a re-authorised connection keeps admitting. Rotation ends grants as a silent approval_required, so say how the operator learns why."
- file: crates/connectors-host/src/local/standing.rs
  line: 473
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "take(now_ms) is clock-agnostic, but the Design does not say which end of the approval clock's interval it passes. Say: upper for not_after and for admitted_at, as proofs refuse when upper >= exp (proof.rs:113). The ESS clock paragraph (standing_approvals.yaml:176-185) says system clock, so the local case needs its own text. A taken use must be released on every non-bound exit, not only step 3 refusals: the arms at execution.rs:311, :345 and :389 (a failed Slack auth.test at :277 would otherwise burn the cap). Use one drop guard."
- file: crates/connectors-host/src/local/standing.rs
  line: 207
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "input_keys gaps. Read: read_json rejects duplicates (lib.rs:300), and with arbitrary_precision on (core Cargo.toml:12) Value equality of numbers is textual (100 vs 1e2 fails closed); compare targets by canonical digest. An input_keys pointer that is a prefix of a target pointer (/body) or of another listed pointer voids the closure, so reject it in grant-standing. 'Listed' must say whole subtree, and that a listed array admits any elements. The walk must run on the same request.input bytes that execution.rs:282 sends and that unchanged (mutation.rs:386) pins by input_sha256."
- file: crates/connectors-host/src/local/owner/mutation.rs
  line: 385
  category: design
  severity: note
  verdict: approve
  origin: introduced
  message: "Keeping standing_grants out of selection() is safe only because each call re-reads and verifies the signature, as unchanged (:385-393) would not notice a changed path. Say so. Also say that the path must be absolute (config refuses relative paths, cli semantics.md:405-410), that a symlinked grant file is unavailable (open_delivered follows only ..data), and that the grant's directory must satisfy admitted() (standing.rs:265)."
- file: .engineering/planning/story/local-invoke-admits-a-keyed-write-under-a-standing-grant.md
  category: design
  severity: note
  verdict: approve
  origin: introduced
  message: "Coupling. Grant is deny_unknown_fields (standing.rs:96), so a served host older than input_keys refuses the whole new file: fail-closed, but compatibility.md should say so. Adding standing to MutationObservation (mutation.rs:156) means project() must read Uses on replay; say what a replay answers when the store is missing. Neither the ADR decisions 1-7 nor the dependent draft story conflict with this."
```
