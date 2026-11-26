---
format: aep.planning-md/3
id: review-result:wave4-drafts-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdicts: the MCP draft `approve`, the ess_model draft `needs-revision`, the provider-budget
draft `needs-revision`. Read at commit 7ac4c2086. No build or test was run.

## Round-1 findings

All 7 are resolved: the three drafts carry `depends_on` edges to the stories that write their
files, the MCP draft adds no release test and no unit test, its two variables carry the forwarded
prefix, and the ess_model draft names its parent directory by crate and binary and its temporary
directory by process id.

## Pairs

- The three drafts are independent of each other: no shared file; the provider-budget draft alone
  writes `Cargo.lock`. A change of `http.rs` rebuilds the tests of the other two, which is no
  collision.
- `Cargo.lock` and `crates/connectors-host/Cargo.toml` against stories of later waves: recorded by
  scope entries; no change needed.
- `.config/nextest.toml`: none of the three writes it.
- The inventory of ignored tests: no draft adds an ignored test.

## New findings

ess_model draft:

1. warning: the two new tests name no unique parent directory, so two runs at once in one tree can
   empty each other's directory and share `generations.log`. Change:
   `tempfile::tempdir_in(CARGO_TARGET_TMPDIR)` as the parent of both tests and of the children.
2. warning: the lock is described as released when the process ends. Under `cargo test` the
   threads of one process open the same lock file, and `flock` on separate opens conflicts.
   Change: say the file is dropped when the helper returns.

Provider-budget draft:

3. warning: "Other stories on these files" omits story:test-binary-per-package-rest, which would
   move the tests of `crates/connectors-host` under one binary and change the binary names of
   acceptance 5. Change: the same "not planned beside" line the other two drafts carry.
