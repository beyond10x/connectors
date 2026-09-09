---
format: aep.planning-md/1
id: review-result:cli-semantic-adversary-r2-20260909
kind: review-result
status: active
title: CLI semantic adversary final pass 2026-09-09
relations:
- reviews: story:local-cli-binding-semantics
revision: 1
---
unit: story:local-cli-binding-semantics — ATTACK2 final semantic pass of uncommitted work/cli-semantics-20260909 atop 88c036562f6ae011bd5cb0ce8e4d4671146b4922
verdict: nothing found
cases: executed 79→79, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: exact-current-source ESS regeneration and required integrated gate remain pending

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

No source, implementation, existing case, generated schema, AEP store, index or
commit was changed by this pass. The untracked source paths were present on entry;
only the assigned ignored scratch receives new report/log files. This is ATTACK2,
the final allowed semantic attack, and does not grant approval.

Read all corrective diffs against `semantic-fixes-originals/`, the complete
`semantic-fixes-report.md`, affected authored values and examples, and owning
acquisition semantics. The before count is the reported baseline across five
lanes: 64 structural + 2 named cached + 5 recursive cached + 5 acquisition +
3 connection-list checks = 79. The same selected suite counts are compared after
this pass. The five isolated regression reruns below are repeated diagnostic
executions, separate from that suite total.

2. Cases and isolated regressions before the suite

No new concrete hypothesis survived source/owner comparison, so no new case was
invented. The five R1 assertions already existed, with recorded red executions
before implementation. This pass reran each unchanged and in isolation before the
complete suite. All five are green now. The five original JSON files, including
the historical combined fixture, remain byte-for-byte unchanged.

Original red outputs, retained in the unchanged R1 logs and report:

```text
case adversary-public-status-rejects-internal-completing: FAILED
value result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1

case adversary-public-status-rejects-internal-expired: FAILED
value result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1

case adversary-cached-connection-list-can-carry-stale-label: FAILED
value result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1

case adversary-cached-operation-list-fixture-is-stale: FAILED (source=cached, stale=false, required stale=true)
cached fixture result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1

case adversary-cached-operation-describe-fixture-is-stale: FAILED (source=cached, stale=false, required stale=true)
cached fixture result: FAILED. 0 passed; 1 failed; 1 executed
exit: 1
```

These are historical red outputs from ATTACK1, not failures in ATTACK2. The exact
original commands remain in `adversary-semantic-r1.md`; the commands and outputs
actually executed in ATTACK2 follow.

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/semantic-fixes-schema/schema/types .local/tmp/cli-wave/adversary-r1-completing.json 2>&1 | tee .local/tmp/cli-wave/adversary-r2-completing.log
case adversary-public-status-rejects-internal-completing: ok
value result: ok. 1 passed; 0 failed; 1 executed
exit: 0
```

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/semantic-fixes-schema/schema/types .local/tmp/cli-wave/adversary-r1-expired.json 2>&1 | tee .local/tmp/cli-wave/adversary-r2-expired.log
case adversary-public-status-rejects-internal-expired: ok
value result: ok. 1 passed; 0 failed; 1 executed
exit: 0
```

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/semantic-fixes-schema/schema/types .local/tmp/cli-wave/adversary-r1-cached-list.json 2>&1 | tee .local/tmp/cli-wave/adversary-r2-cached-list.log
case adversary-cached-connection-list-can-carry-stale-label: ok
value result: ok. 1 passed; 0 failed; 1 executed
exit: 0
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json --slurpfile expected .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json --arg id adversary-cached-operation-list-fixture-is-stale '[$expected[0].cases[] | select($id == "all" or .id == $id) | . as $case | [$actual[0].cases[] | select(.id == $case.fixture_id)] as $matches | {id: $case.id, ok: (($matches | length) == 1 and $matches[0].value.source == $case.expected_source and $matches[0].value.stale == $case.expected_stale), source: $matches[0].value.source, stale: $matches[0].value.stale}] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end) (source=\(.source), stale=\(.stale), required stale=true)"), "cached fixture result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r2-cached-operation-list.log
case adversary-cached-operation-list-fixture-is-stale: ok (source=cached, stale=true, required stale=true)
cached fixture result: ok. 1 passed; 0 failed; 1 executed
exit: 0
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json --slurpfile expected .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json --arg id adversary-cached-operation-describe-fixture-is-stale '[$expected[0].cases[] | select($id == "all" or .id == $id) | . as $case | [$actual[0].cases[] | select(.id == $case.fixture_id)] as $matches | {id: $case.id, ok: (($matches | length) == 1 and $matches[0].value.source == $case.expected_source and $matches[0].value.stale == $case.expected_stale), source: $matches[0].value.source, stale: $matches[0].value.stale}] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end) (source=\(.source), stale=\(.stale), required stale=true)"), "cached fixture result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r2-cached-operation-describe.log
case adversary-cached-operation-describe-fixture-is-stale: ok (source=cached, stale=true, required stale=true)
cached fixture result: ok. 1 passed; 0 failed; 1 executed
exit: 0
```

An initial tool-orchestration lookup for the prior turn's in-memory jq string
returned `Error: Expected preserved cached-jq program` before invoking any shell
or test. The unchanged expression was then restored from the R1 report and the
five runs above completed. That orchestration error selected zero tests and is
not reported as a semantic failure.

3. Complete suite, after the isolated regressions

The author's existing compiled Rust value verifier is reused with its corrected
provisional ESS 0.20.0 projection at
`.local/tmp/cli-wave/semantic-fixes-schema/schema/types`. No compilation,
regeneration or ambient ESS selection occurred in this pass. Source/projection
shape checks and a byte comparison with the copied authored domain confirm the
reviewed enum/reason/list declarations; exact-current-source generation remains
a coordinator obligation.

The acquisition assertions check valid acquisition objects for public state
membership, failed iff safe reason, completed iff yielded connection, and reason
membership in the existing owner enum. The list assertions check admitted source,
Boolean stale, cached-implies-stale and unknown/already-expired-at-observation
deadline-implies-stale. These are example consistency checks; they do not measure
runtime current time, earliest-member reduction, private fencing or dispatch.

```text
$ TMPDIR="$PWD/.local/tmp/cli-wave" .local/tmp/cli-wave/value-verifier/target/debug/cli-value-verifier .local/tmp/cli-wave/semantic-fixes-schema/schema/types contracts/cli/v1alpha1/fixtures/values.json 2>&1 | tee .local/tmp/cli-wave/adversary-r2-suite-values.log
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
case adversary-public-status-rejects-internal-completing: ok
case adversary-public-status-rejects-internal-expired: ok
case adversary-cached-connection-list-can-carry-stale-label: ok
case public-acquisition-pending-projection: ok
case public-acquisition-failed-reason-rejected: ok
case public-acquisition-failed-reason-exchange_unknown: ok
case public-acquisition-failed-reason-expired: ok
case public-acquisition-rejects-unknown-reason: ok
case public-acquisition-rejects-protected-reason: ok
case connection-list-requires-source: ok
case connection-list-requires-stale: ok
case connection-list-rejects-string-validity-deadline: ok
case cached-connection-list-with-known-validity-deadline: ok
value result: ok. 64 passed; 0 failed; 64 executed
exit: 0
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json --slurpfile expected contracts/cli/v1alpha1/fixtures/cached-expectations.json --arg id all '[$expected[0].cases[] | select($id == "all" or .id == $id) | . as $case | [$actual[0].cases[] | select(.id == $case.fixture_id)] as $matches | {id: $case.id, ok: (($matches | length) == 1 and $matches[0].value.source == $case.expected_source and $matches[0].value.stale == $case.expected_stale), source: $matches[0].value.source, stale: $matches[0].value.stale}] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end) (source=\(.source), stale=\(.stale), required stale=true)"), "cached fixture result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r2-suite-cached-fixtures.log
case adversary-cached-operation-list-fixture-is-stale: ok (source=cached, stale=true, required stale=true)
case adversary-cached-operation-describe-fixture-is-stale: ok (source=cached, stale=true, required stale=true)
cached fixture result: ok. 2 passed; 0 failed; 2 executed
exit: 0
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json '[ $actual[0].cases[] | select(.valid == true) | . as $case | .value | .. | objects | select(.source? == "cached") | {id: $case.id, ok: (.stale == true)} ] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end)"), "cached class result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r2-suite-cached-class.log
case valid-AdapterDescribeResult-6: ok
case valid-OperationListResult-24: ok
case valid-OperationDescribeResult-26: ok
case adversary-cached-connection-list-can-carry-stale-label: ok
case cached-connection-list-with-known-validity-deadline: ok
cached class result: ok. 5 passed; 0 failed; 5 executed
exit: 0
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json '[ $actual[0].cases[] | select(.valid == true) | . as $case | .value | .. | objects | select(has("acquisition") and (.acquisition | type) == "string" and has("state") and has("expires_at_ms")) | . as $observation | {id: $case.id, ok: ((["pending","completed","failed"] | index($observation.state)) != null and (($observation.state == "failed") == ($observation.reason != null)) and (($observation.state == "completed") == ($observation.connection != null)) and ($observation.reason == null or (["rejected","exchange_unknown","expired"] | index($observation.reason)) != null))} ] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end)"), "acquisition example result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r2-suite-acquisition-examples.log
case valid-ConnectionStatusResult-37: ok
case public-acquisition-pending-projection: ok
case public-acquisition-failed-reason-rejected: ok
case public-acquisition-failed-reason-exchange_unknown: ok
case public-acquisition-failed-reason-expired: ok
acquisition example result: ok. 5 passed; 0 failed; 5 executed
exit: 0
```

```text
$ jq -ner --slurpfile actual contracts/cli/v1alpha1/fixtures/values.json '[ $actual[0].cases[] | select(.valid == true and .type == "connectors.cli.ConnectionListResult") | . as $case | .value as $page | {id: $case.id, ok: ((["authority","cached"] | index($page.source)) != null and ($page.stale | type) == "boolean" and ($page.source != "cached" or $page.stale == true) and (($page.valid_until_ms != null and $page.valid_until_ms > $page.observed_at_ms) or $page.stale == true))} ] as $results | if ($results | length) == 0 then error("empty case selection") else ([$results[] | select(.ok == false)] | length) as $failed | ($results[] | "case \(.id): \(if .ok then "ok" else "FAILED" end)"), "connection-list example result: \(if $failed == 0 then "ok" else "FAILED" end). \(($results | length) - $failed) passed; \($failed) failed; \($results | length) executed", (if $failed > 0 then "" | halt_error(1) else empty end) end' 2>&1 | tee .local/tmp/cli-wave/adversary-r2-suite-list-examples.log
case valid-ConnectionListResult-12: ok
case adversary-cached-connection-list-can-carry-stale-label: ok
case cached-connection-list-with-known-validity-deadline: ok
connection-list example result: ok. 3 passed; 0 failed; 3 executed
exit: 0
```

| Selected suite lane | Before reported | Executed in ATTACK2 | Red |
|---|---:|---:|---:|
| Structural values | 64 | 64 | 0 |
| Named cached expectations | 2 | 2 | 0 |
| Recursive cached examples | 5 | 5 | 0 |
| Acquisition examples | 5 | 5 | 0 |
| Connection-list examples | 3 | 3 | 0 |
| Total | 79 | 79 | 0 |

The unchanged count is deliberate: this pass reruns the corrected unit's complete
selected suite and adds no case. Five isolated regression executions preceded
the suite and are not counted again as suite additions. Each selection was
nonempty. Final metadata checks confirmed that the 64 fixture ids are unique.

The historical `adversary-r1-combined-values.json` is retained unchanged and was
not selected as the current suite. Its old authoritative-list sample predates the
new required source/stale fields. Current durable `values.json` contains all
three original structural adversarial cases and is the selected corrected suite;
no historical record was rewritten to manufacture a green run.

4. Findings

Nothing found. No remaining ATTACK1 finding or new semantic finding is returned.
This statement covers only the frozen uncommitted semantic source identified by
the hashes below, atop `88c036562f6ae011bd5cb0ce8e4d4671146b4922`.
It neither approves the unit nor claims exact-toolchain or runtime conformance.

5. Attacked without a reproducible remaining finding

- Public acquisition projection: `ess/domains/cli.yaml:46,131` now uses pending/completed/failed and the existing safe failure enum; the original completing/expired cases are rejected, public failed reasons validate, and all five valid observations satisfy their explicit state/reason/yielded-reference rules.
- Cached connection pages: `ess/domains/cli.yaml:228` now requires source/stale and allows an honest unknown validity deadline; the untouched stale-label regression and all three valid page examples pass.
- Cached descriptor examples: the two corrected operation examples now use stale:true, and recursive selection checks all five current cached objects.
- Fix class coverage: adapter inventory is explicitly configuration-only, adapter/acquisition status has no cached fallback, connection description/status retain typed freshness labels, and every surfaced cache is covered by the stated cached-is-stale rule.
- Scope and ownership: no new persistent entity, production handler, private acquisition lifecycle exposure or changed repair/revocation authority was introduced; public cross-field and page-reduction obligations remain explicitly separate from structural schema validity.

6. Write audit, source identity and handoff

Every path written outside the managed worktree: **none**. The worktree CLI owns
its external lifecycle bookkeeping. No integration, AEP command, source edit,
commit, index change, cleanup or build was performed.

The five original adversarial JSON hashes are preserved:

```text
c9cd8c69151e96597fef004748cd9b98dc08a024711089d44e34429d19ca8852  .local/tmp/cli-wave/adversary-r1-completing.json
d3365c764f952e79175c0a384678c667e4d2e2780fa2dab8cf80dba7d7e83a8b  .local/tmp/cli-wave/adversary-r1-expired.json
35b5504878b0d03a572ac188bf8660bb62191fd2c6b0fcff9fea6fdf0b392440  .local/tmp/cli-wave/adversary-r1-cached-list.json
b12a9a57cb38076893603da8aba1224768f64dd6182a9ae468206cf9f0f3f319  .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json
763377cb9ea29a3f98add5a1f50622584ade2bcff73c65a330b416c1d345ed4c  .local/tmp/cli-wave/adversary-r1-combined-values.json
```

Frozen source SHA-256 values:

```text
addaaae89ce81e29c77baddc341a8e7d9d0aa3819226c30bf2ccf7a276120cfd  contracts/cli/v1alpha1/semantics.md
1e93ca756cc54e347ca2c3631e2e5b7d8bdc6352baa3592dc7215e75c8244ca2  contracts/cli/v1alpha1/scenarios.md
dc32d3e3feccf30f9242713acd55acacc9bf6aad4aa0e4c3b0c09d7bbfb27a10  contracts/cli/v1alpha1/fixtures/values.json
b12a9a57cb38076893603da8aba1224768f64dd6182a9ae468206cf9f0f3f319  contracts/cli/v1alpha1/fixtures/cached-expectations.json
6f257e9752d9c94b1739f5f5b0355a1bbebff7ca438e1976a0c368783b698116  contracts/cli/v1alpha1/fixtures/config-valid.toml
df768f6337e848be0c8383c7c3b07055b1b6adc8cca11e7003cb448f17b32983  contracts/cli/v1alpha1/fixtures/config-invalid.toml
5692b1be3b344bf20e6e0420f39928efc89f81b7c13e38af363fe062b019bacf  ess/domains/cli.yaml
```

Final audit was fail-fast; every command below completed successfully:

```text
$ set -e
sha256sum --check .local/tmp/cli-wave/semantic-fixes-adversary-before.sha256
sha256sum --check .local/tmp/cli-wave/semantic-fixes-final-source.sha256
cmp ess/domains/cli.yaml .local/tmp/cli-wave/semantic-fixes-model/domains/cli.yaml
cmp contracts/cli/v1alpha1/fixtures/cached-expectations.json .local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json
jq -e '(.cases | length) > 0 and (([.cases[].id] | length) == ([.cases[].id] | unique | length))' contracts/cli/v1alpha1/fixtures/values.json
git diff --check
git --no-pager diff --stat
.local/tmp/cli-wave/adversary-r1-cached-fixture-expectations.json: OK
.local/tmp/cli-wave/adversary-r1-cached-list.json: OK
.local/tmp/cli-wave/adversary-r1-combined-values.json: OK
.local/tmp/cli-wave/adversary-r1-completing.json: OK
.local/tmp/cli-wave/adversary-r1-expired.json: OK
contracts/cli/v1alpha1/semantics.md: OK
contracts/cli/v1alpha1/scenarios.md: OK
contracts/cli/v1alpha1/fixtures/values.json: OK
contracts/cli/v1alpha1/fixtures/cached-expectations.json: OK
ess/domains/cli.yaml: OK
true
exit: 0
```

Retained new files, all under
`/home/timo/.local/state/worktree/trees/b10x/connectors_v2/cli-semantics-20260909/.local/tmp/cli-wave/`:

- `adversary-r2-completing.log`
- `adversary-r2-expired.log`
- `adversary-r2-cached-list.log`
- `adversary-r2-cached-operation-list.log`
- `adversary-r2-cached-operation-describe.log`
- `adversary-r2-suite-values.log`
- `adversary-r2-suite-cached-fixtures.log`
- `adversary-r2-suite-cached-class.log`
- `adversary-r2-suite-acquisition-examples.log`
- `adversary-r2-suite-list-examples.log`
- `adversary-semantic-r2.md`

Only reviewer lease `cli-semantic-review-r2-20260909` is released on handoff.
Tree id `cli-semantics-20260909` remains active and coordinator-owned, at the
path above. No cleanup or publication was attempted. The coordinator owns the
next action: preserve this final pass verbatim, integrate the frozen source,
regenerate and validate with the selected exact current ESS source, and execute
the required repository gate. Linux protected-source/keyring/process/persistence
conformance remains deferred production work.

7. Machine-readable findings

```findings
[]
```

