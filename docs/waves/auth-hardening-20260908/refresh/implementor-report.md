unit: story:contracts-refresh-coordination
verdict: green
cases: executed 0→0, red 0
origin: n/a
wrote-outside-worktree: none
needs-coordinator: yes

Runtime execution is unchanged because this approved unit is semantic only: ESS compiled 12 authored scenarios and 120 generated scenarios; it executed no runtime conformance target. The coordinator still owes adversarial review, combined F04/F05 checks, the full integration gate, planning updates and local commit/merge. No shared-file patch is needed.

1. Unit and acceptance

Specify refresh exclusion and recovery after owner loss: the failure matrix permits no second rotating-token exchange while a previous exchange may have consumed the token.

2. Observed diff

```text
 contracts/auth/acquisition/v1alpha1/semantics.md |  63 ++++-
 contracts/auth/custody/v1alpha1/semantics.md     |  20 +-
 docs/adapters/atlassian.md                       |   7 +-
 ess/domains/refresh.yaml                         | 280 ++++++++++++++++++++++-
 4 files changed, 352 insertions(+), 18 deletions(-)
```

The stat excludes the 13 new unstaged files (twelve authored YAML scenarios plus verification.md); all are inside the assigned acquisition directory. Owned hunk headers, including new files:

```text
diff --git a/contracts/auth/acquisition/v1alpha1/semantics.md b/contracts/auth/acquisition/v1alpha1/semantics.md
@@ -75 +75 @@ Flow (specializes `docs/design.md:543-549`):
@@ -77,5 +77,42 @@ Refresh:
@@ -103 +140,4 @@ Client credentials (`oauth2_client_credentials`): no browser; `begin` performs t
@@ -116 +156 @@ Client credentials (`oauth2_client_credentials`): no browser; `begin` performs t
@@ -126 +166,4 @@ Client credentials (`oauth2_client_credentials`): no browser; `begin` performs t
@@ -128,0 +172,2 @@ Client credentials (`oauth2_client_credentials`): no browser; `begin` performs t
diff --git a/contracts/auth/custody/v1alpha1/semantics.md b/contracts/auth/custody/v1alpha1/semantics.md
@@ -12 +12 @@
@@ -23 +23 @@ Design responsibility three of four: persist sensitive material through an injec
@@ -44,0 +45,2 @@ publish_active(connection, expected_revision, new_version) -> Published(revision
@@ -56,2 +58,4 @@ Outcomes are distinct: `Missing` (never written or deleted), `Unavailable` (back
@@ -79 +83,2 @@ Outcomes are distinct: `Missing` (never written or deleted), `Unavailable` (back
@@ -94 +99 @@ Outcomes are distinct: `Missing` (never written or deleted), `Unavailable` (back
@@ -103 +108,2 @@ Outcomes are distinct: `Missing` (never written or deleted), `Unavailable` (back
diff --git a/docs/adapters/atlassian.md b/docs/adapters/atlassian.md
@@ -56 +56 @@ Auth (`providers/jira.toml:140-297`, `confluence.toml` auth block): `jira.api_to
@@ -60,0 +61,4 @@ Auth (`providers/jira.toml:140-297`, `confluence.toml` auth block): `jira.api_to
@@ -130,0 +135 @@ None.
diff --git a/ess/domains/refresh.yaml b/ess/domains/refresh.yaml
@@ -2 +2,279 @@ domain: connectors.refresh
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/authorization-wins-fence-race.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/authorization-wins-fence-race.yaml
@@ -0,0 +1,77 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
@@ -0,0 +1,65 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/binding-replacement-before-publication.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/binding-replacement-before-publication.yaml
@@ -0,0 +1,78 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/durable-response-wins-owner-loss-race.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/durable-response-wins-owner-loss-race.yaml
@@ -0,0 +1,88 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/expired-candidate-before-publication.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/expired-candidate-before-publication.yaml
@@ -0,0 +1,78 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/late-response-after-quarantine.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/late-response-after-quarantine.yaml
@@ -0,0 +1,81 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/published-reply-loss-no-reexchange.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/published-reply-loss-no-reexchange.yaml
@@ -0,0 +1,96 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/recovery-owner-loss-never-reopens-source.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/recovery-owner-loss-never-reopens-source.yaml
@@ -0,0 +1,87 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/reserved-owner-fenced-before-successor.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/reserved-owner-fenced-before-successor.yaml
@@ -0,0 +1,82 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-publication.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/revocation-before-publication.yaml
@@ -0,0 +1,93 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/stored-response-recovery-rejects-stale-publisher.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/stored-response-recovery-rejects-stale-publisher.yaml
@@ -0,0 +1,108 @@
diff --git a/contracts/auth/acquisition/v1alpha1/scenarios/two-replicas-one-authorization.yaml b/contracts/auth/acquisition/v1alpha1/scenarios/two-replicas-one-authorization.yaml
@@ -0,0 +1,72 @@
diff --git a/contracts/auth/acquisition/v1alpha1/verification.md b/contracts/auth/acquisition/v1alpha1/verification.md
@@ -0,0 +1,117 @@
```

3. First acceptance scenario before model declarations

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/red-authored.json
refusal[ESS-AUTHOR-005]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.RefreshAttempt` is not an entity this specification declares
  help: name an entity the specification declares; an authored scenario acts on the model's own instances and invents none
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.ReserveRefresh` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.AuthorizeExchange` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.QuarantineRefresh` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.AuthorizeExchange` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-006]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.ReserveRefresh` is not a command this specification declares
  help: name a command the specification declares; a scenario that invokes anything else is checking a system this model does not describe
refusal[ESS-AUTHOR-012]: `connectors.refresh/authored/authorized-owner-loss-no-second-exchange` in contracts/auth/acquisition/v1alpha1/scenarios/authorized-owner-loss-no-second-exchange.yaml
  `connectors.refresh.RefreshStates` is not a view this specification declares
  help: name a view the specification declares
0 authored scenario(s) from 1 file(s), 7 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/red-authored.json
exit: 1
```

The first scenario existed before refresh declarations. The final typed decision input is a declaration-shape correction; its uncertainty, no-second-authorization, source-unavailable, no-event and terminal-state assertions remain. Subsequent model authoring exposed multiple unconditional outcomes, fixed by typed trusted transaction decisions. ESS-AUTHOR-021 rejected a captured error-field reference; the private ownership error now asserts concrete reason stale_owner rather than an unsupported captured id.

4. Semantic unit gate, verbatim

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path .local/waves/auth-hardening-20260908/refresh/baseline-ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/baseline-operations.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/baseline-operations.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
connectors v1 — 7 file(s), valid
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/waves/auth-hardening-20260908/refresh/ir.json
connectors v1 — 7 file(s), 76 declaration(s), compiled to .local/waves/auth-hardening-20260908/refresh/ir.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/authored.json
12 authored scenario(s) from 12 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/authored.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/auth/acquisition/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/synthesized.json
132 scenario(s) (12 authored), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/synthesized.json
exit: 0
```

```text
$ .local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/waves/auth-hardening-20260908/refresh/operations-authored.json
15 authored scenario(s) from 15 file(s), 0 refusal(s), written to .local/waves/auth-hardening-20260908/refresh/operations-authored.json
exit: 0
```

```text
$ git diff --check
exit: 0
```

Refresh author lane: compiled 0 → 12, refusals 7 → 0, exit 1 → 0. Synthesis lane: 132 compiled (120 generated + 12 authored), exit 0. Operation regression lane: compiled 15 → 15, exit 0 on both opening and changed models. Runtime lane: executed 0 → 0 (not run; no Connectors target exists). Relative markdown link audit: 0 broken. No Rust changed, so the unit appropriately ran no Rust build/test/clippy lane; the coordinator owns the integration gate.

5. Scope audit, decisions and obligations

All cited surfaces were verified and revised: acquisition assigns the coordinator and atomic protocol; custody distinguishes baseline CAS from rotating-refresh metadata transactions; Atlassian references the same strict policy. All inferred surfaces were checked: refresh.yaml was an empty registered private domain; private scenarios/verification were absent. None was a mistaken path. No file outside the six assigned surfaces was edited. Shared ESS registration, credential-generation seam, Rust gate, planning store, F05 files and versioning/migration proposals were preserved.

Decision matrix:
- Two replicas: one reservation and one irreversible authorization; contender observes/refuses.
- Reserved owner loss: fence wins before a successor may reserve.
- Authorized owner loss, including before send: quarantine unless response was durably registered; never authorize another source exchange.
- Response-storage/recovery race: CAS chooses committed candidate recovery or terminal uncertainty; late responses cannot resurrect uncertainty.
- Stored-response takeover: change publication ownership/fence only; old publisher fails; no new exchange. A second lost recovery owner causes discard/repair in this first profile.
- Publication: atomically compare source, binding revision, current owner/fence, exact candidate, F05 validity and revocation; replace active generation and invalidate old admissions together.
- Authorization also invalidates old-generation dispatch admissions while refresh is unresolved. Pins never override invalidation. Already-opened transport is not undone.
- Revocation first: publication fails; publication first: revocation applies to new active generation. Changed binding and invalid/expired evidence likewise refuse.
- Committed publication reply loss: read ledger; never replay exchange.
- Aliases of the same rotating material must not obtain independent refresh authority; UUID uniqueness alone is insufficient.

Explicit UNMAPPED runtime obligations: actual transaction-decision derivation, unique reservation/consumed-material association, fence/clock checks, non-replayable send authorization, provider-client retry suppression, crash durability, field assignment/current recovery owner, immutable material capture, candidate identity/scope/F05 validity, cross-entity publication/revocation/dispatch cutoff, retention and transport call counts. ESS validates types, optional candidate/source relations and lifecycle causation; it does not prove these mechanisms. No persistent evidence entity or bespoke interpreter was invented. Detailed matrix and exact evidence live in contracts/auth/acquisition/v1alpha1/verification.md.

6. Storage and handoff

No artifact was written outside the worktree. All scratch evidence is in .local/waves/auth-hardening-20260908/refresh/: red-author.log/json, baseline-ess/, baseline-ir.json, baseline-operations.json/log, ir.json, authored.json, synthesized.json, operations-authored.json, named command logs, green-logs.json and this implementor-report.md. The twelve scenario files and verification.md are wanted unstaged changes. No build target was created. No commit, staging, cleanup or planning mutation was performed. The coordinator retains its lease and owns review/integration/cleanup; my codex-auth-refresh-impl-20260908 lease is released on handback.
