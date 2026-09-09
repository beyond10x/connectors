---
format: aep.planning-md/1
id: review-result:cli-semantic-adversary-r1-20260909
kind: review-result
status: active
title: Local CLI semantic adversary, round 1
relations:
- reviews: story:local-cli-binding-semantics
revision: 1
---
unit: story:local-cli-binding-semantics — ATTACK1 of uncommitted work/cli-semantics-20260909 atop 88c036562f6ae011bd5cb0ce8e4d4671146b4922
verdict: NEEDS-CHANGE
cases: executed 51→56, red 5
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: route findings to semantic author; exact-source ESS regeneration and integrated gate remain pending

1. `git --no-pager diff --stat`

```text
$ git --no-pager diff --stat
(no output)
exit: 0

$ git status --short
?? contracts/cli/
?? ess/domains/cli.yaml
exit: 0
```

No implementation/source file was changed by this attack. The two untracked
source paths above were already present on entry; all added cases, logs and this
report are under the assigned ignored `.local/tmp/cli-wave/` directory. An empty
tracked diff cannot inventory those untracked authored inputs, so their hashes
are recorded below. No charter violation was observed. No suite was run before
the added cases existed and had individually failed. The before count, 51, comes
from the author's declared green run in `semantic-report.md`, section 4.

This resumes the interrupted first attack; it is not a second attack or recheck.
The previous reviewer left no report or executed adversarial cases in the assigned
scratch. This report covers the inputs independently read and probed here.

2. Added cases, isolated first executions

All five cases were written before the first execution. The existing compiled
Rust `cli-value-verifier` was reused, with no compilation or Cargo slot.
Its source checks a nonempty fixture selection, validates each value against the
selected ESS JSON Schema, compares against `valid`, and exits 1 for mismatches.
The cached-fixture checks use a direct jq assertion over the author's fixture
documents, with an exact-one fixture match and explicit nonempty selection.
No test alters an existing case or a generated schema.

The projection was the author's existing `schema-final/schema/types`, generated
provisionally by ESS 0.20.0. `cmp ess/domains/cli.yaml
.local/tmp/cli-wave/model/domains/cli.yaml` returned 0. The suspect enum and closed
list fields in that projection match the current authored YAML. These probes do
not replace exact current-source generation or runtime conformance.

- `adversary-r1-completing.json`: public `ConnectionStatusResult` must reject
  internal `completing`; the auth owner requires public `pending` at
  `contracts/auth/acquisition/v1alpha1/semantics.md:155–158`. Red now.
- `adversary-r1-expired.json`: public status must reject internal `expired`;
  the auth owner requires `failed` with reason `expired` at the same owner
  document, lines 165–170. Red now.
- `adversary-r1-cached-list.json`: a connection list with a cached ready
  observation must be able to carry `source:cached, stale:true`, consistent with
  CLI semantics lines 74–77 and scenario L01. It adds these labels to the
  author's otherwise valid list sample. Red now because the closed list result
  and its summaries provide no such fields, not because the ready sample or
  references are malformed.
- `adversary-r1-cached-fixture-expectations.json`: two separately selected
  assertions require the authored operation-list and operation-describe
  `source:cached` examples to have `stale:true`. Both are red now.

Verbatim isolated executions, before either full suite lane:

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/schema-final/schema/types .local/tmp/cli-wave/adversary-r1-completing.json
case adversary-public-status-rejects-internal-completing: FAILED
value result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1
```

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/schema-final/schema/types .local/tmp/cli-wave/adversary-r1-expired.json
case adversary-public-status-rejects-internal-expired: FAILED
value result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1
```

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/schema-final/schema/types .local/tmp/cli-wave/adversary-r1-cached-list.json
case adversary-cached-connection-list-can-carry-stale-label: FAILED
value result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json --slurpfile expected .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json --arg id adversary-cached-operation-list-fixture-is-stale '[$expected[0].cases[] | select($id == "all" or .id == $id) | . as $case | [$actual[0].cases[] | select(.id == $case.fixture_id)] as $matches | {id: $case.id, ok: (($matches | length) == 1 and $matches[0].value.source == $case.expected_source and $matches[0].value.stale == $case.expected_stale), source: $matches[0].value.source, stale: $matches[0].value.stale}] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end) (source=\(.source), stale=\(.stale), required stale=true)"), "cached fixture result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r1-cached-operation-list.log
case adversary-cached-operation-list-fixture-is-stale: FAILED (source=cached, stale=false, required stale=true)
cached fixture result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json --slurpfile expected .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json --arg id adversary-cached-operation-describe-fixture-is-stale '[$expected[0].cases[] | select($id == "all" or .id == $id) | . as $case | [$actual[0].cases[] | select(.id == $case.fixture_id)] as $matches | {id: $case.id, ok: (($matches | length) == 1 and $matches[0].value.source == $case.expected_source and $matches[0].value.stale == $case.expected_stale), source: $matches[0].value.source, stale: $matches[0].value.stale}] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end) (source=\(.source), stale=\(.stale), required stale=true)"), "cached fixture result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r1-cached-operation-describe.log
case adversary-cached-operation-describe-fixture-is-stale: FAILED (source=cached, stale=false, required stale=true)
cached fixture result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1
```

3. Suite run, after all isolated cases existed and failed

The combined structural fixture is an additional scratch file; the author's
51-case fixture remains unchanged:

```text
$ jq -s '{format: "connectors-cli-values/1", cases: [.[].cases[]]}' contracts/cli/v1alpha1/fixtures/values.json .local/tmp/cli-wave/adversary-r1-completing.json .local/tmp/cli-wave/adversary-r1-expired.json .local/tmp/cli-wave/adversary-r1-cached-list.json > .local/tmp/cli-wave/adversary-r1-combined-values.json
exit: 0
```

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/schema-final/schema/types .local/tmp/cli-wave/adversary-r1-combined-values.json 2>&1 | tee .local/tmp/cli-wave/adversary-r1-suite-values.log
case valid-SetupInitResult-1: ok
case valid-SetupCheckResult-2: ok
case valid-AdapterListInput-3: ok
case valid-AdapterListResult-4: ok
case valid-AdapterDescribeInput-5: ok
case valid-AdapterDescribeResult-6: ok
case valid-AdapterStatusInput-7: ok
case valid-AdapterStatusResult-8: ok
case valid-AdapterStopInput-9: ok
case valid-AdapterStopResult-10: ok
case valid-ConnectionListInput-11: ok
case valid-ConnectionListResult-12: ok
case valid-ConnectionDescribeInput-13: ok
case valid-ConnectionDescribeResult-14: ok
case valid-ConnectionConnectInput-15: ok
case valid-ConnectionConnectResult-16: ok
case valid-ConnectionRepairInput-17: ok
case valid-ConnectionRepairResult-18: ok
case valid-ConnectionStatusInput-19: ok
case valid-ConnectionStatusResult-20: ok
case valid-ConnectionRevokeInput-21: ok
case valid-ConnectionRevokeResult-22: ok
case valid-OperationListInput-23: ok
case valid-OperationListResult-24: ok
case valid-OperationDescribeInput-25: ok
case valid-OperationDescribeResult-26: ok
case valid-OperationInvokeInput-27: ok
case valid-OperationInvokeResult-28: ok
case valid-LegacyDescribeInput-29: ok
case valid-LegacyDescribeResult-30: ok
case valid-LegacyInvokeInput-31: ok
case valid-LegacyInvokeResult-32: ok
case valid-ConnectionCreateRequest-33: ok
case valid-ConnectionRepairRequest-34: ok
case valid-Failure-35: ok
case valid-ConnectionStatusInput-36: ok
case valid-ConnectionStatusResult-37: ok
case valid-AdapterObservation-38: ok
case C03-no-secret-in-management-request: ok
case C03-no-custody-reference-in-result: ok
case C03-no-secret-in-failure: ok
case stop-requires-exact-child-incarnation: ok
case stop-rejects-caller-pid: ok
case startup-rejects-unknown-mode: ok
case restart-rejects-background-retry: ok
case repair-requires-revision: ok
case C05-carrier-is-text-not-wire-object: ok
case C05-revision-required: ok
case list-rejects-string-limit: ok
case globals-stay-in-context: ok
case revoke-does-not-claim-provider-success: ok
case adversary-public-status-rejects-internal-completing: FAILED
case adversary-public-status-rejects-internal-expired: FAILED
case adversary-cached-connection-list-can-carry-stale-label: FAILED
value result: FAILED. 51 passed; 3 failed; 54 executed
exit: 1
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json --slurpfile expected .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json --arg id all '[$expected[0].cases[] | select($id == "all" or .id == $id) | . as $case | [$actual[0].cases[] | select(.id == $case.fixture_id)] as $matches | {id: $case.id, ok: (($matches | length) == 1 and $matches[0].value.source == $case.expected_source and $matches[0].value.stale == $case.expected_stale), source: $matches[0].value.source, stale: $matches[0].value.stale}] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end) (source=\(.source), stale=\(.stale), required stale=true)"), "cached fixture result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r1-suite-cached-fixtures.log
case adversary-cached-operation-list-fixture-is-stale: FAILED (source=cached, stale=false, required stale=true)
case adversary-cached-operation-describe-fixture-is-stale: FAILED (source=cached, stale=false, required stale=true)
cached fixture result: FAILED. 0 passed; 2 failed; 2 executed
exit: 1
```

Total after count is 54 structural cases plus 2 fixture-semantic assertions:
56 executed, 51 passed, 5 red. The original 51 structural cases all passed during
the augmented run. The two supplemental assertions do not claim that JSON Schema
must enforce cross-field semantics; they check whether the authored examples
agree with their own prose. No full repository gate, provider test, TOML parser,
real process lifecycle or keyring operation was executed by this review.

4. Findings and reachability

These findings cover the uncommitted semantic unit atop
`88c036562f6ae011bd5cb0ce8e4d4671146b4922`, with the source hashes below.
The selected acceptance is the reviewed contract and typed value model, not a
production handler. Reachability below therefore identifies the documented command
and fixture path that consumes the modeled public value; it does not assert that
a production runtime has emitted these values.

| File:line | Verdict | Origin | Severity | Finding |
|---|---|---|---|---|
| `ess/domains/cli.yaml:46` | NEEDS-CHANGE | introduced | blocker | The public acquisition status enum admits completing and expired although the owning acquisition contract requires pending and failed-with-expired-reason projections. |
| `ess/domains/cli.yaml:220` | NEEDS-CHANGE | introduced | blocker | ConnectionListResult exposes connection state but cannot represent the stale label required for the documented cached connection-list workflow. |
| `contracts/cli/v1alpha1/fixtures/values.json:368` | CONFIRMED | introduced | warning | The cached operation-list and operation-describe examples set stale to false while the local CLI contract requires cached facts to be labeled stale. |

F1 — what was measured: the two one-case status fixtures were accepted by the
current projected public `ConnectionStatusResult` shape although marked invalid,
so each verifier run exited 1. The offending enum is declared at
`ess/domains/cli.yaml:44–46`, consumed by `AcquisitionObservation.state:126`
and `ConnectionStatusResult.acquisition:285`.
What reaches it: `connections status --acquisition` is explicitly selected in
`contracts/cli/v1alpha1/semantics.md:68,88–90`; after one-use consumption or
known expiry the owning acquisition state machine reaches Completing/Expired.
The inherited owner requires Completing→public pending at
`contracts/auth/acquisition/v1alpha1/semantics.md:155–158` and
Expired→public failed with reason expired at `:165–170`; `:226` explicitly
repeats that Completing is internal. The CLI contract delegates to this owner at
`contracts/cli/v1alpha1/semantics.md:15–22`. A correct public binding needs the
owner's public state projection and a safe expiry failure reason, rather than
exposing the internal lifecycle enum. The current observation also has no failure
reason field. This is a contract/model conflict; no runtime information leak was
claimed or measured.

F2 — what was measured: adding the explicit cached/stale labels to the author's
otherwise valid ready-list sample causes the one-case verifier to fail with
exit 1. `ConnectionListResult` has only connections, next_cursor and
observed_at_ms at `ess/domains/cli.yaml:220–225`; its closed
`ConnectionSummary` has no label or validity field either (`:103–111`).
The projected schema rejects additional properties in both objects.
What reaches it: `connections list` maps directly to this result at
`contracts/cli/v1alpha1/semantics.md:64`, while `:74–77` permits cached facts
and requires them to be marked stale; L01 at
`contracts/cli/v1alpha1/scenarios.md:35` exercises cached inspection with the
host absent. The author's valid list example at
`contracts/cli/v1alpha1/fixtures/values.json:164–181` includes `state:ready`.
An observation timestamp alone has no source or expiry policy and cannot encode
the required stale label. The neighboring ConnectionDescription deliberately
carries source, stale and valid_until_ms (`ess/domains/cli.yaml:112–120`).
The author should select a consistent safe list representation or explicitly
tighten the owning workflow; the test's field placement is one minimal form,
not a claim that the contract freezes that exact layout. The issue is that no
admitted field in the current result can carry the required distinction.

F3 — what was measured: exact-one fixture assertions report
`source=cached, stale=false, required stale=true` and each exit 1.
The two locations are `contracts/cli/v1alpha1/fixtures/values.json:367–368`
and `:396–397`.
What reaches it: the authored operation-list/describe successful examples are
consumed by the structural suite and correspond to the cached descriptor commands
at `contracts/cli/v1alpha1/semantics.md:70–71`. The blanket rule at `:74–77`
and L01 require stale marking. Correct these examples or explicitly select and
describe a bounded fresh-cache exception; no such exception is currently selected.
This is a fixture/prose discrepancy, not proof of stale provider dispatch.

All three origins are introduced: `git ls-tree -r
88c036562f6ae011bd5cb0ce8e4d4671146b4922 contracts/cli ess/domains/cli.yaml`
returned no entries, exit 0; these are entirely new source surfaces. A read of
`git show <base>:contracts/auth/acquisition/v1alpha1/semantics.md` confirmed the
owner's pending/failed projection predates this unit. The tree was never switched
or reset to the base.

Covered source SHA-256 values, unchanged between first inspection and report:

```text
103b219e4def9b1905a722a60682848660fd87302a528465e06f794cf6abc1d7  contracts/cli/v1alpha1/semantics.md
eff147704f58a1b78e3d5e15f5d1aee3af9356b23ff4a9a37d0bafaf38dd3366  contracts/cli/v1alpha1/scenarios.md
e8bc9335585785138fdbbf4fe34acaa9dc592ac0a1d7ccb1225a311cc55f0ed7  contracts/cli/v1alpha1/fixtures/values.json
6f257e9752d9c94b1739f5f5b0355a1bbebff7ca438e1976a0c368783b698116  contracts/cli/v1alpha1/fixtures/config-valid.toml
df768f6337e848be0c8383c7c3b07055b1b6adc8cca11e7003cb448f17b32983  contracts/cli/v1alpha1/fixtures/config-invalid.toml
f7eb48408bf3a493bd749568545144fb26b1389851d5bea9c94b1a157b3882a9  ess/domains/cli.yaml
```

5. Attacked without another reproducible finding

- Same-identity repair: semantics, C04, model comments and repair fixtures preserve the public semantic revision while advancing private generation/publication fencing.
- Lifecycle scope: per-entry on-demand default, automatic startup at host launch, no launch during listing, coalescing, exact owned stop and durable suppression are explicitly specified.
- Custody boundaries: protected source separation, keyring failure, durable publication ordering, restart reuse and terminal local revoke are explicitly stated; executable Linux/backend conformance remains deferred.
- Structural rejection: all 51 original cases still passed, including secret-field rejection, exact stop coordinates, startup/restart enums, required repair revision, business carrier typing and provider-revoke outcome.
- Scope discipline: no production handler, persistent CLI entity, hosted orchestration, MCP binding or provider-runtime claim was introduced in the reviewed surface.

6. Write inventory and handoff

Every path written outside the managed worktree: **none**. Worktree lifecycle
bookkeeping is owned by the `worktree` CLI; no credential machinery or external
integration was invoked. No source, generated artifact, existing test, AEP store,
Git index or commit was mutated by this review.

Retained paths inside the assigned worktree scratch, all relative to
`/home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-semantics-20260909/`:

- `.local/tmp/cli-wave/adversary-r1-completing.json`
- `.local/tmp/cli-wave/adversary-r1-expired.json`
- `.local/tmp/cli-wave/adversary-r1-cached-list.json`
- `.local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json`
- `.local/tmp/cli-wave/adversary-r1-combined-values.json`
- `.local/tmp/cli-wave/adversary-r1-completing.log`
- `.local/tmp/cli-wave/adversary-r1-expired.log`
- `.local/tmp/cli-wave/adversary-r1-cached-list.log`
- `.local/tmp/cli-wave/adversary-r1-cached-operation-list.log`
- `.local/tmp/cli-wave/adversary-r1-cached-operation-describe.log`
- `.local/tmp/cli-wave/adversary-r1-suite-values.log`
- `.local/tmp/cli-wave/adversary-r1-suite-cached-fixtures.log`
- `.local/tmp/cli-wave/adversary-semantic-r1.md`

The existing verifier target and schema directories were read, not rebuilt or
modified. Final `git diff --check` and `git --no-pager diff --stat` both
returned 0 with no output. Tree `cli-semantics-20260909` remains active under
coordinator ownership; no cleanup/finish/commit/publication was attempted.
The reviewer releases only session lease `cli-semantic-review-finish-20260909`
on handoff. The coordinator is the next owner: record this first attack verbatim,
route the two blocking model findings and fixture correction, then select a
recheck and exact-source/integrated validation. This report grants no approval.

7. Machine-readable findings

```findings
- file: ess/domains/cli.yaml
  line: 46
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The public acquisition status enum admits completing and expired although the owning acquisition contract requires pending and failed-with-expired-reason projections.
- file: ess/domains/cli.yaml
  line: 220
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ConnectionListResult exposes connection state but cannot represent the stale label required for the documented cached connection-list workflow.
- file: contracts/cli/v1alpha1/fixtures/values.json
  line: 368
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The cached operation-list and operation-describe examples set stale to false while the local CLI contract requires cached facts to be labeled stale.
```

