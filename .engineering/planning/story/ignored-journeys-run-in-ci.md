---
format: aep.planning-md/3
id: story:ignored-journeys-run-in-ci
kind: story
status: draft
title: The ignored CLI journeys run in CI and fail a pull request when red
revision: 1
---
## Observed
The ignored catalog CLI journeys (`adapters/catalog/tests/local_runtime/`) run in no CI job:
`connectors-build ignored` "is not automatically enrolled in CI" (docs/development.md, "Ignored
tests"). On `main` 74d8d857d (0.52.0), 2026-10-16, 17 of 26 local_runtime journeys passed:
- Both guarded_merge journeys panicked at `recorded_state.rs:395` since 6f3300fe8 (2026-10-08),
  which added `access` to the audit anchor; the test helper `audits()` never copied it. Fixed in
  the test only (aa1ee3437); nothing ran the journey for eight days.
- `lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority` (`lifecycle.rs:105`)
  fails every run: since 9cc4613c1 (ESS 0.52.0) the CLI refuses duplicate-key input itself with
  `cli_dynamic_input`, and the test expects `invalid_input`. Which side is right is undecided.
- Seven journeys time out at stage `admission` and one revoked-disclosure case fails, at host
  load 55-60 on 20 cores; load is the likely cause, not shown.
- Two failing journeys leave `__connectors-owner` and provider daemons running.

## Acceptance
- A CI job (Linux) runs the `disposable` family of `connectors-build ignored` on every pull
  request, and a red journey fails the pull request.
- The lifecycle duplicate-key case states the one refusal code the contract names, and the CLI
  and the test agree on it.
- Each of the eight load-sensitive journeys either passes on the CI runner or carries a named,
  measured cause.
- A journey that fails leaves no owner or provider process behind (the harness kills its own
  process group on drop).
