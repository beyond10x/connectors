---
format: aep.planning-md/3
id: review-result:er-0330-pin-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2, story:entity-runtime-0330-records-name-definition-by-digest
relations:
- reviews: story:entity-runtime-0330-records-name-definition-by-digest
revision: 1
---
verdict: needs-revision

Earlier findings 1 to 4, 7 (the RSS acceptance, now dropped) and 9 hold. Their cited lines read right: `metadata_conformance.rs:966`, `registry/tests.rs:1839-1849`, and the existence of `setup_checkpoints_enable.rs`. Four findings still block.

1. Acceptance 5, "with the `owner_uid` rewrite … if the store needs it": the test cannot decide this ahead of time.
   - Every store records its writer's uid in `local_authority.owner_uid` (`metadata.rs:27`). `Metadata` refuses another uid's store (`metadata_store_previous_pin.rs:98-102`).
   - A fixture written here with uid 1000 fails on a CI runner with another uid, and `load` (`store_cost_tests.rs:224-251`) does not rewrite the row.
   - Accept: "…loads it with `load` after rewriting `local_authority.owner_uid` to the uid running the test, as `metadata_store_previous_pin.rs:103-111` does, unconditionally".

2. Acceptance 6, the source of the store: "a state directory in which the new build's CLI recorded a read (taken from one of the jailed CLI tests' containers)" names no test and no step. A second person cannot reproduce the input.
   - Accept: name the test and the path or command that leaves the state directory, or use the fixed input of the fallback: "a store saved by `read_invoke_cost_by_store_size` at the pin commit, written into by one run of the four `measured_invoke` calls".

3. Acceptance 7 and the fallback of acceptance 6 disagree.
   - Line 7 makes the "do not open a store" sentence conditional on "that exit code is non-zero". The fallback is a `load` error with no exit code, so the condition is undefined.
   - The "One-way" section says the answer is "confirmed by acceptance 5", but that is acceptance 6.
   - Accept: "…the sentence …, only if the run of line 6 exits non-zero, or, when the fallback was used, `load` returns `Err`; the Breaking entry says which of the two was run."

4. Acceptance 8 does not fit the 120 s budget as written.
   - `cargo nextest run -p connectors-host -p connectors` runs both whole packages, not "the tests of these files". `.config/nextest.toml` exists, but no filterset is named.
   - It names no `slot` prefix and no `--timeout`.
   - Accept: "`slot cargo nextest run -p connectors-host -p connectors -E '<filterset naming the nine files' tests/binaries>'` exits 0 in at most 120 s with no test over 60 s", with the filterset written in the line.

Minor, not a blocker: the acceptance 4 command takes 100 invokes. It runs outside the gate, but it should say `slot --timeout`, since a job over 10 minutes is killed.

Read: 1 artifact, `story:entity-runtime-0330-records-name-definition-by-digest` (revision 20, the whole body). I also checked its cited lines in the tree at b9b56f7bd19.
Not established: whether `connections list` makes a 0.74.2 build replay the store, and so refuse it, rather than only open it; whether 50 events leave at least one read invoke. I ran nothing. The `0.75.0` baseline does not exist as a tag yet; this is the planned PR #283 and is outside my lane.

```findings
- file: crates/connectors-host/tests/metadata_store_previous_pin.rs
  line: 103
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 5 makes the owner_uid rewrite conditional ('if the store needs it'), so the test passes only for the uid that wrote the fixture; it must be unconditional"
- file: story:entity-runtime-0330-records-name-definition-by-digest#Acceptance-6
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 names no test or command that produces the state directory with a recorded read ('one of the jailed CLI tests' containers')"
- file: story:entity-runtime-0330-records-name-definition-by-digest#Acceptance-7
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 conditions the sentence on an exit code the acceptance 6 fallback does not produce, and the One-way section cites acceptance 5 for what acceptance 6 confirms"
- file: story:entity-runtime-0330-records-name-definition-by-digest#Acceptance-8
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 8 runs both whole packages with no filterset, no slot prefix and no 120 s bound on the step"
```
