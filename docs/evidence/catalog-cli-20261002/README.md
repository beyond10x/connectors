# Catalog CLI acceptance, 2026-10-02

All ten logical obligations and fifteen historical lifecycle/mutation variants
have passing evidence across retained runs. The final corrected candidate passed
334 ordinary package tests, both affected fault selections, two cleanup safety
controls, Clippy and formatting. The [correction follow-up](cleanup-followup-review.md)
found the prior blocker resolved and no new issue; it performed no new executions.
The [integrated repository gate](../provider-release-20261002/README.md) also passed,
including Rust 1.88/1.91 checks. Source release verification remains separate.

[Fault acceptance report](fault-acceptance-report.md) records the complete mapping,
real exact-file EIO injection, recovery branches, source/binary identities and
historical failures. Its final cleanup was subsequently found unsafe by the
[whole-candidate review](review-before-cleanup-fix.md): observation-only child
handles could authorize signals without sufficient ownership at acquisition.

[Cleanup correction](cleanup-fix-report.md) supersedes that candidate's cleanup
source and counts. A constructed control using only test-owned processes failed
against the old behavior and passed after correction. Weak child handles are now
polled only; strongly owned owner and notification-proven handles remain separate.
An exited notification-cache entry is removed before replacement capture. No real
PID-reuse race or unrelated-process signal was attempted.

Original effects use the production CLI, qualified disposable custody and the
shipped catalog selection. Some settled replay observations use a same-image
host-library helper. Lifecycle and modes0–7 retain earlier runs with an explicit
unchanged-relevant-source reuse argument; all fifteen variants were **not** rerun
on the final executable. The two fault selections were rerun sequentially after
the correction. Helpers, controls and repeated runs do not inflate journey counts.

Fault cases require Linux x86_64 seccomp user notifications. Modes8/9 observe
resumed production metadata writes; revoked modes10/11 use a separately labeled
read-only fixture fsync to prove restoration, not a production recovery write.
Serialized evidence contains selected typed terminal records after validation of
a complete capture; it is not an archived full capture or event history.

## Preserved earlier results

[First partial report](partial-report.md) records eight of ten obligations and
eleven of fifteen variants. The [rejected chmod experiment](chmod-rejected.log)
failed to prevent settlement; mode8 failed and modes9–11 were not run then.
Original report SHA256:
`445aca6ac928d61e9064fc54f59d077ae95f67528b35217dd378008069dcadb9`.
Original failed log SHA256:
`9269829524f6ba8ad61601aac25cbb7c5e3c77ec1371a0d3fb62ffebc44e046f`.

[Seccomp feasibility](seccomp-feasibility.md) and its [smoke log](seccomp-smoke.log)
proved only the kernel mechanism, before actual runtime interception. The
[bounded static review](static-subset-review.md) covered four frozen helper
modules without executing tests; it did not cover the whole fault candidate.
Later reports retain the unexplained recovery red, assertion refinements and one
disclosed dependent-run ordering deviation. Later passes do not erase those runs.

Public reports replace the private home prefix with literal `$HOME`. Raw logs and
manifests are retained in the task's private evidence; reports name their hashes.
No production catalog behavior, approval guard or operation deadline changed.
