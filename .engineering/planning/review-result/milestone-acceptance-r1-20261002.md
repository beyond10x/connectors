---
format: aep.planning-md/3
id: review-result:milestone-acceptance-r1-20261002
kind: review-result
status: active
title: aep:plan-critic-acceptance, milestone plan round 1
relations:
- reviews: specification:milestone-acceleration-20261002
- reviews: story:bridge-drop-waits-for-dispatched-batch
- reviews: story:registry-clock-outside-shared-batches
- reviews: story:sql-fixture-accepts-stray-connections
- reviews: story:ignored-suites-have-a-runner
revision: 1
---
needs-revision

story:bridge-drop-waits-for-dispatched-batch — The acceptance combines ownership retention, duplicate-free recovery and uncertainty preservation as independent outcomes; name one aggregate pass condition for the three cases and retain their individual assertions in Verification. — .engineering/planning/story/bridge-drop-waits-for-dispatched-batch.md:27
story:registry-clock-outside-shared-batches — The acceptance combines an adoption decision with independent invariant outcomes; make the recorded decision's validity the single acceptance condition and require its evidence to include every numeric and invariant check. — .engineering/planning/story/registry-clock-outside-shared-batches.md:29
story:sql-fixture-accepts-stray-connections — The acceptance combines session accounting, extra-session rejection and repetition success as independent outcomes; name one aggregate regression result whose required evidence includes the positive repetitions and negative control. — .engineering/planning/story/sql-fixture-accepts-stray-connections.md:19
story:ignored-suites-have-a-runner — The acceptance combines inventory completeness, execution, prerequisite reporting, failure propagation and helper exclusion as independent outcomes; name one aggregate tooling-conformance result and retain those assertions as its mandatory cases. — .engineering/planning/story/ignored-suites-have-a-runner.md:24

Read all five assigned artifacts in full through `aep plan artifact show`: specification:milestone-acceleration-20261002 and the four stories above; also read their source acceptance lines, `kinds`, both applicable `lifecycle` outputs, the existing measurement probes and SQL regression, and ran `aep plan artifact validate` (valid, with historical review-record warnings).

Could not establish executable bindings for the proposed scenario names; their absence is explicitly acknowledged and is not a draft-plan finding. No implementation or external verification was performed.

```findings
- file: .engineering/planning/story/bridge-drop-waits-for-dispatched-batch.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: The acceptance combines ownership retention, duplicate-free recovery and uncertainty preservation as independent outcomes; name one aggregate pass condition for the three cases and retain their individual assertions in Verification.
- file: .engineering/planning/story/registry-clock-outside-shared-batches.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: The acceptance combines an adoption decision with independent invariant outcomes; make the recorded decision's validity the single acceptance condition and require its evidence to include every numeric and invariant check.
- file: .engineering/planning/story/sql-fixture-accepts-stray-connections.md
  line: 19
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: The acceptance combines session accounting, extra-session rejection and repetition success as independent outcomes; name one aggregate regression result whose required evidence includes the positive repetitions and negative control.
- file: .engineering/planning/story/ignored-suites-have-a-runner.md
  line: 24
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: The acceptance combines inventory completeness, execution, prerequisite reporting, failure propagation and helper exclusion as independent outcomes; name one aggregate tooling-conformance result and retain those assertions as its mandatory cases.
```
