---
format: aep.planning-md/3
id: review-result:wave4-drafts-parallel-safety-round-5
kind: review-result
status: active
title: Parallel-safety critic, round 5, two drafts of wave 4
relations:
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
## Verdict

Read at d7124123d. Nothing was built or run. Both approvals are for the text at that commit.

- Sixteen ess_model tests: approve. No finding.
- Provider budget that ends in an unanswered TLS handshake: approve. No finding.

## Do the changes since round 4 move a file or a surface

No.

- The ess_model draft still writes `adapters/sql/tests/ess_model.rs` alone.
  - `CONNECTORS_TEST_ESS_MODEL_SCHEMAS_PARENT` is named in this draft alone; the child test
    reads it and the test sets it on its 4 children.
  - The working directory of `ess generate` moves to `adapters/sql/spec` with `--path ess`;
    the output stays under `CARGO_TARGET_TMPDIR`. Not confirmed by a run: that `ess` writes
    nothing beside its output. The 16 calls of today carry the same risk.
  - The new test copies the 4 files to a directory of its own under `target/tmp` and changes
    the copy; `adapters/sql/spec/ess` is not written.
  - `--exact` and `--nocapture` change how the children are started, nothing else.
- The provider budget changed in prose alone. Its four files are the same. The three contract
  texts are named and not written.

## Can the three run at once

Yes: the two drafts and the story on the two MCP tests, in three worktrees, merged in any
order.

- The three write sets share no path.
- `aep plan artifact waves` prints no collision line between two of the three.
- `target/tmp` is written by the ess_model draft alone, and each worktree has its own target
  directory.
- No fixed port, and no new environment variable beside the child's.
- 3 items placed, all by a cited scope. 0 reads of an acceptance go to a producer in the set.
