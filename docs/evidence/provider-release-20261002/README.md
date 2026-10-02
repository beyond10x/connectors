# Provider handoff integration checks, 2026-10-02

The local Rust gate passed on the integrated 0.25.1 candidate based on
`2617ee34cea358b8ed033dda31131ae1f76708a1`, with the release version, ignored-runner
classification and documentation changes present. Provider source commits are
PostgreSQL `c3e20e1df82ccd4bc33b25c2943d3059ae0b51e1`, Kubernetes
`8ce90cb6d0f765a78f4273ca0ccade1ed7253a35`, and catalog
`d7d2d841e801ce670f6d587a8885854a9ae3db5f`.
[Selected source inputs](source-inputs.sha256) retain exact version, classifier
and affected test/source hashes. Subsequent AEP/evidence writes record the result;
they are not new runtime behavior. Publication and final remote checks remain
separate requirements.

Command: `cargo run --locked -p connectors-build -- gate --msrv`, with
`CONNECTORS_ESS` selecting 0.45.0, `CONNECTORS_AEP` selecting 0.65.0,
`CARGO_BUILD_JOBS=2`, and `RUSTC_WRAPPER=/usr/bin/sccache`. Exit 0.

- Workspace test result summaries: 1,207 passed, 0 failed, 65 ignored.
- Formatting, generated CLI/descriptor drift, archived source digests, adapter
  boundaries, independent shared/native ESS validation, workspace Clippy with
  warnings denied, and production builds passed.
- Independent library and selected provider-library checks passed on Rust 1.88;
  the full workspace/all-targets check passed on Rust 1.91.
- Contract synthesis produced 498 scenarios, including 43 authored scenarios,
  and retained 21 synthesis refusals. This is synthesis, not 498 executions.
- Local metadata conformance reported 289 total/passed, 0 failed/error/unsupported/
  skipped. Its overall status remains **Inconclusive: coverage undeclared**.
  This does not establish complete contract conformance or erase the 21 refusals.
- AEP validation returned `valid`, retaining historical missing-findings and
  missing-review-outcome warnings. No historical record was rewritten to hide them.

Original full gate log SHA256:
`f12f0f751223733ffdf3f610fa5c6bb11dcbfd69da26ad7c1fc6b5b769c978e7`.
It remains in task-owned private evidence with the exact commands and output.
The gate was paused during compilation to preserve an 8 GiB disk reserve and
resumed after free space recovered. No test had begun in the paused command.

## Ignored-test inventory

`cargo run --locked -p connectors-build -- ignored --inventory --report <report>`
exited 0. It found 65 ignored cases across 134 test binaries: 41 selected by the
default family, 0 executed, 65 skipped, 38 missing prerequisites, 0 failures and
**0 unknown classifications**. Missing prerequisites in this unconfigured inventory
are not passing acceptance evidence. The selected real/disposable provider runs
are recorded separately in the PostgreSQL, Kubernetes, GitLab and catalog reports.
The earlier browser-count and missing adversary-binary observations remain open.

## Website checks and publication boundary

Website dependency installation, typecheck, build, reference drift check,
15 example tests, browser smoke and UI checks all passed. The build/public audit
checked 482 files with no private path markers. Browser checks used the built site
on a task-owned loopback server and the installed matching Playwright Chromium.
No website/cloud deployment was performed. Publication and exact tag/release
verification remain pending at this record. npm reported 29 dependency audit advisories
(1 low, 25 moderate, 3 high); this patch changes no website dependencies.

## Follow-up to the first PR run

The first PR 79 repository gate exposed an intermittent, pre-existing Google
export fixture truncation. The [scoped correction and refreshed integration
checks](../google-export-ci-20261002/README.md) supersede the original local gate
for the corrected candidate: 1,208 tests pass, with the same 65 ignored cases and
unchanged conformance limitations. The original failed CI and red/green regression
are retained. The fixture correction changes no production limits or behavior.
