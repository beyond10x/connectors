---
format: aep.planning-md/3
id: approval-record:connectors-0-66-0-harness-fix-go
kind: approval-record
status: approved
title: Fix the ignored inventory for harness=false targets before releasing 0.66.0
relations:
- decides: story:provider-ignored-tests-run-in-ci
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T02:59:04Z", actor: "human:operator", revision: 2}
---
## Fix go after the first CI run of pull request 239 (2026-11-05)

The first CI run of pull request 239 failed only in the new step "Run the ignored tests a built catalog
provider suffices for" (job 126990311895):

> Error: "unsupported inventory line in catalog_provider_failure: test result: ok. 0 passed; 0 failed;
> 0 ignored; 0 measured; 8 filtered out"

No test ran. The inventory listed every built test target with `--ignored --list --format terse`,
including the five `harness = false` stand-in targets of `apps/connectors`, whose own runner answers that
listing with a summary line. The defect predates the wave: the full `ignored` lane had refused since the
first stand-in target (2026-10-29) and had never run in full.

The directing session `b10x-chef-orchestrator` decided, under the operator's grant:

1. In `crates/connectors-build/src/ignored.rs`, skip `harness = false` test targets, read from each
   package's `[[test]]` entries in its manifest, with a unit test: a harness=false target is not
   inventoried and a libtest target still is.
2. Locally: `connectors-build` bin tests and clippy within unit limits, through the build slot.
3. One bot commit and push; one new CI run of pull request 239.
4. On all-green CI: the release go (approval-record:connectors-0-66-0-release-go) applies.
5. If that run fails again in the provider-only step, for any reason: revert the workflow line, keep the
   harness=false fix, move story:provider-ignored-tests-run-in-ci back to active with both findings, and
   ship 0.66.0 after one more green CI run.
