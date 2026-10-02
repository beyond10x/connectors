# Catalog CLI acceptance, 2026-10-02

This is partial evidence, not completed acceptance. The first run passed eight of
ten logical obligations and eleven of fifteen historical variants. The unchanged
settlement control passed; changing file permissions failed to prevent settlement.
Mode8 therefore failed correctly, and modes9–11 were not run. The next fixture
correction is still pending. Production semantics were not changed by this pass.

[Author report](partial-report.md) preserves the exact command/count distinctions,
earlier fixture corrections, source/binary identities and limitations. Public text
replaces the private home prefix with literal `$HOME`; original report SHA256 is
`445aca6ac928d61e9064fc54f59d077ae95f67528b35217dd378008069dcadb9`.

[Rejected chmod experiment](chmod-rejected.log) retains the deciding failed test.
Its original, before the same path redaction, has SHA256
`9269829524f6ba8ad61601aac25cbb7c5e3c77ec1371a0d3fb62ffebc44e046f`.
Passing pre-held assertions are retained in the test source; a complete serialized
pre-snapshot was not captured by that run.

[Seccomp feasibility](seccomp-feasibility.md) and its [single smoke log](seccomp-smoke.log)
establish only a kernel mechanism for failing exact already-open-file writes. That
probe did not execute Entity Runtime or the production CLI. Actual settlement
interception, all four missing variants and final independent review remain required.
The throwaway probe source/binary are private retained scaffolding, not release tooling.
