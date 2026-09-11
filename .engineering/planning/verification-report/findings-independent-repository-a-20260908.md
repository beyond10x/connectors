---
format: aep.planning-md/1
id: verification-report:findings-independent-repository-a-20260908
kind: verification-report
status: draft
title: Structured transcription of review-result:independent-repository-a-20260908
tags:
- review-findings-supplement
refs:
- provider: review-body-sha256
  reference: 4d68f85486f3938d9edd37f5453018bfa57b48553ee3fa8e9679233148b9d27f
relations:
- verifies: review-result:independent-repository-a-20260908
- derived_from: specification:review-findings-toolchain-upgrade-20260911
revision: 1
---
# Structured transcription of review-result:independent-repository-a-20260908

This supplements [the immutable original](../review-result/independent-repository-a-20260908.md).
It records no new critic run, approval, review outcome or current implementation
finding. Source binding uses the SHA-256 of the exact original body returned by AEP.
The original review's scope, verdict, source snapshot and historical date remain authoritative.

## Original conclusion

Severity: blocker 0, major 0, minor 8, nit 4.

## Transcription method

12 findings remain in the scope of this report's final stated conclusion.
Closed items in a same-pass disposition and verification/command tables are excluded.
Each nonempty message reproduces a source section or table row verbatim, including
its original identifier and citations. P0/P1, major and blocking map to blocker;
P2, minor and should-fix map to warning; P3 and nit map to note. Ungraded observations
remain unspecified. No per-finding verdict or introduced/pre-existing classification
is invented. The file is the first explicit source citation; when only shorthand
citations are present, legacy-source-excerpt identifies the original report itself.
The full citation context remains in the message and original. This transcription
makes no claim that paraphrased findings across different historical rounds have
identical comparison signatures.

## Findings

```findings
[
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M1 | minor | verified | docs/design.md:1247 | §28 still says GitLab \"lowers its selected local request types into ESS 0.9.2\"; the pin is 0.20.0 | `crates/connectors-spec/toolchain.json` = `{\"ess\":\"0.20.0\"}`; `adapters/gitlab/generated/manifest.json` `\"ess\":\"ess 0.20.0\"` |",
    "line": 1247
  },
  {
    "file": "adapters/gitlab/upstream/README.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M2 | minor | verified | adapters/gitlab/upstream/README.md:28 | says `ess-import.json` \"retains the complete ESS 0.9.2 refusal\"; the committed file is the 0.20.0 refusal | `adapters/gitlab/generated/ess-import.json`: `\"refusals\": [\"/openapi: only OpenAPI 3.1 is supported, found 3.0.0\"]`; docs/gitlab-generation.md:293-294 describes this as the 0.20.0 change |",
    "line": 28
  },
  {
    "file": "docs/design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M3 | minor | verified | docs/design.md:1283-1284; contracts/README.md:31 | design says \"14 proposed … 4 families deferred\"; the index has 16 `proposed` rows and lists 3 deferred families; `configuration` (design.md:180) is in neither list; `contracts/service/v1alpha2` (proposed) is absent from the index | `grep -c \"| proposed\" contracts/README.md` = 16; README.md:31 \"Deferred families …: `execution`, `events`, `resources`\"; `grep -n v1alpha2 contracts/README.md` = no index row. E13/E22 are owned by draft `story:contracts-documentation-index`, but the v1alpha2 row and the 14-vs-16 count post-date that intake (db1c329) |",
    "line": 1283
  },
  {
    "file": ".engineering/planning/story/contracts-mutation-outcomes.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M4 | minor | verified | .engineering/planning/story/contracts-mutation-outcomes.md:108; …/contracts-idempotency-scope.md:104 | both `implemented` stories state their changes \"remain local and uncommitted\"; both are committed | `git log`: 34f298a \"docs: harden mutation outcomes…\", 8903166 \"docs: harden idempotency scope…\"; `git show --stat` of each commit includes the respective story file |",
    "line": 108
  },
  {
    "file": ".engineering/planning/specification/contract-driven-connectors-design.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M5 | minor | verified | .engineering/planning/specification/contract-driven-connectors-design.md:38 (also :12) | \"No implementation stories are decomposed, no lifecycle approval is asserted\" / \"does not claim that any adapter has been implemented\" while 5 stories `derived_from` this artifact are `implemented` | status grep: three-adapters-e2e, gitlab-spec-service, full-review-remediation, ess-executable-pin, ess-pin-upgrade = `status: implemented`; artifact is `draft`, revision 2 |",
    "line": 38
  },
  {
    "file": "docs/stack-integration-proposal.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M6 | minor | verified | docs/stack-integration-proposal.md:3-4; contracts/service/v1alpha2/semantics.md:6 | the source of gaps G1–G10 is `.local/review/2026-09-08-concept-and-stack-integration-review.md`, a gitignored path; no tracked artifact preserves those findings (the two other reviews were preserved as review-result artifacts for exactly this reason) | `.gitignore:2` = `/.local/`; `git ls-files \\| grep -i concept` = none; intake spec :19 \"preserved … so the plan does not depend on ignored workspace files\" |",
    "line": 3
  },
  {
    "file": "crates/connectors-build/src/gate.rs",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M7 | minor | verified | crates/connectors-build/src/gate.rs:456; README.md:24 | `--msrv` forces `CARGO_TARGET_DIR=<root>/target/msrv`, ignoring the caller's `CARGO_TARGET_DIR`; README says the gate \"uses a task-owned temporary directory under `.local/tmp`\" | `find target/msrv -newer logs/cargo-build.log \\| wc -l` = 13 after a gate run with `CARGO_TARGET_DIR` set elsewhere; gate.rs:456 `.env(\"CARGO_TARGET_DIR\", root.join(\"target/msrv\"))` |",
    "line": 456
  },
  {
    "file": "docs/cli-migration-v1-to-v2.md",
    "category": "legacy-review",
    "severity": "warning",
    "message": "| M8 | minor | verified | docs/cli-migration-v1-to-v2.md:124 | refers to `story:cli-governed-surface` as the owner of 7 open decisions; no such planning artifact exists | `git grep cli-governed-surface` hits only the two docs; docs/stack-integration-proposal.md:63 \"Not created\" |",
    "line": 124
  },
  {
    "file": "crates/connectors-host/src/credentials.rs",
    "category": "legacy-review",
    "severity": "note",
    "message": "| N1 | nit | verified | crates/connectors-host/src/credentials.rs:51; server.rs:137-161; federation.rs:254-266,328-335; adapters/kubernetes/src/lib.rs:60-62,288-293; adapters/sql/src/lib.rs:79-99 | untested paths: `CredentialRef::Environment` resolution (no test constructs it; `git grep \"Environment {\" -- '*.rs'` hits only the definition), host `Capacity` (semaphore, result size) and the 20 s deadline, federation connect refusals (duplicate/`__`/cycle), `hosts.discover` disabled, SQL TLS handshake (only `database_roots` unit test at sql lib.rs:330-355), `schema.list` success (live runner only, conformance main.rs:494-507) | grep results above; test list in cargo-test.log |",
    "line": 51
  },
  {
    "file": "crates/connectors-host/src/federation.rs",
    "category": "legacy-review",
    "severity": "note",
    "message": "| N2 | nit | inferred | crates/connectors-host/src/federation.rs:17-20; crates/connectors-client/src/lib.rs:39-45 | `DownstreamConfig` has no `ca_file`, and `Endpoint::new` builds a reqwest client with built-in roots only, so a downstream served over HTTPS with a private CA cannot be federated; provider HTTP has `ca_file` (http.rs:24) | code read; README.md:117 only says \"use loopback locally or a trusted TLS-terminating ingress\" |",
    "line": 17
  },
  {
    "file": "contracts/operations/v1alpha1/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| N3 | nit | verified | contracts/operations/v1alpha1/semantics.md:61 | the Invocation example carries `\"version\": \"v1alpha1\"` plus `idempotency_key`/`approval`, which the v1alpha1 strict decoder refuses; §7 (:207) and §10 (:241) take the `v1alpha2` wire | core lib.rs:91 `#[serde(deny_unknown_fields)]` on `Invocation` |",
    "line": 61
  },
  {
    "file": "contracts/service/v1alpha2/semantics.md",
    "category": "legacy-review",
    "severity": "note",
    "message": "| N4 | nit | verified | contracts/service/v1alpha2/semantics.md:15 | \"Unchanged: … error envelope shape\" while §3.4 adds `audit_ref` to every response and §3.5 adds two codes; both are refused by v1alpha1 readers (core lib.rs:13-27 closed `ErrorCode`, :120 `deny_unknown_fields` on `Response`). §4 rule 10 makes the behaviour consistent (v1alpha1 clients refused); the summary row is imprecise | file lines cited |",
    "line": 15
  }
]
```

