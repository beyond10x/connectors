---
format: aep.planning-md/3
id: story:the-gates-cargo-calls-set-two-build-jobs-against-the-hosts-rule
kind: story
status: draft
title: The gate's cargo calls set two build jobs, against the host's rule for the shared pool
relations:
- informed_by: story:the-forges-gate-job-compiles-once-and-prints-its-time-per-step
revision: 2
---
## Defect

Found on 2026-11-25 while the needs of the build node were written down.

The gate's own cargo calls set the number of build jobs to 2. The host's rule for every build
is that no build sets `-j` or `CARGO_BUILD_JOBS`: the shared pool of build slots hands out the
jobs.

Three places in the code:

- `crates/connectors-build/src/gate.rs:43`, in the function `cargo` that every gate step uses;
- `crates/connectors-build/src/ignored.rs:1083`, the lane's `cargo test` call;
- `crates/connectors-build/src/docs.rs:215`.

The value came with two commits, of 2026-09-08 and of 2026-10-02, both older than the shared
pool's rule. 16 lines of 8 files under `docs/` show commands with `CARGO_BUILD_JOBS=2` as
well (`docs/development.md`, `docs/verification.md` and six `docs/local-*.md` pages).

Not measured: the time of a gate step with 2 jobs against the time with the jobs the pool or
the forge's runner would give it. Not read: whether the forge's runner needs a cap, for
memory or for another reason.

## Question

This story is a draft and has no decided change. Before one is written:

1. Read why the value was set (the two commits and their stories).
2. Measure one compiling gate step on the forge with the value and without it, 3 runs each,
   in a pull request that is not merged.
3. Decide between: no value anywhere; a value from the environment of the forge's workflow
   alone, so that the host's pool decides on the host; the value kept, with the reason written
   beside it and the host rule's owner asked for the exception.

## Acceptance

Not written yet: it follows the decision of question 3. The plan critics read this story
after that.

## Scope

- `crates/connectors-build/src/gate.rs`, `crates/connectors-build/src/ignored.rs`,
  `crates/connectors-build/src/docs.rs`: the three places above.
- The pages under `docs/` that show the value, if the decision changes what a reader types.

## Not in this story

- The compile cache and the debug info of test builds: the folder rules cover both, and the
  repository's `.cargo/config.toml` already holds `debug = false`.
