---
format: aep.planning-md/3
id: review-result:windows-bounded-read-scope-round-1
kind: review-result
status: active
title: Scope critic, round 1, story:windows-mcp-tool-call-answers-timeout
relations:
- reviews: story:windows-mcp-tool-call-answers-timeout
revision: 1
---
Recorded by the session that ran the panel: the brief asked for the members severity, where, finding and fix; the store reads file, category, severity, verdict, origin and message. Each entry below keeps its words in `message`; file, category, verdict and origin were set at recording.

verdict: needs-revision

```findings
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: story:windows-mcp-tool-call-answers-timeout, frontmatter `scope:` entry `website/docs/reference/contracts` finding: The entry is a directory, not the file the Design changes. The generated mirror of private-bounded-read.md is `website/docs/reference/contracts/cli.md` (it holds \"CLOCK_MONOTONIC nanoseconds\"). Acceptance 7's `generate --check` fails if it is not regenerated. fix: Replace the entry with `website/docs/reference/contracts/cli.md`, confidence `cited`."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: scope entry `apps/connectors/src/mcp/mod.rs` (inferred), acceptance 2 finding: mod.rs gates `mod tests;` at line 97 with `#[cfg(all(test, unix))]`. The Design never says mod.rs changes, and the required not-unix-gated test cannot go into tests.rs. It belongs in a `#[cfg(test)]` module in projection.rs, which is already in scope. fix: Drop mod.rs from scope. Or state in the Design that the test lives in projection.rs, and name the mod.rs change if there is one."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Design, last bullet; scope entry `ess/domains/transport.yaml` finding: The change is the comment at `transport.yaml:18` (\"this Linux boot's CLOCK_MONOTONIC\"). The story says \"no declaration changes\" but does not say whether a comment edit changes the ESS spec digest or a generated file. Acceptance 7 checks only `connectors-docs generate --check`. fix: Add to acceptance 7: \"the ESS digest check, if the repository has one, exits 0 after the comment edit\". Or state that none exists."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Context paragraph \"Recorded before:\", and relations finding: The reversal names D1, the test and the contract sentence. It does not name the reason D1 gave (the budget is \"this boot's CLOCK_MONOTONIC nanoseconds\", which another platform does not share) or say why that reason no longer holds. It also does not say the record itself stays unamended. A reader of `approval-record:wave-20261022` finds no pointer forward, and the store has no edge from this story to it. fix: Add: \"Reverses D1 of approval-record:wave-20261022 (2026-10-23): D1 held the bounded read Unix-only because a budget is boot-local CLOCK_MONOTONIC ticks; Windows has an equivalent unbiased interrupt clock (contracts/service/clock.md:138-143), so D1's reason is gone. The record is not edited.\" Pending operator confirmation, add a relation to the record if the CLI has one for it. The only other store artifact with the old premise is `.engineering/planning/verification-report/connectors-v0760-release-scope.md:38` (\"built for Unix only\"). It is a dated finding, so it needs no change."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Acceptance 7 and `docs/local-mcp-stdio.md:10-20`, `website/docs/connectors/mcp.md:61-68`, `website/docs/status.md:50`, `docs/local-mcp-cli.md:3` finding: Acceptance 7 covers only sentences saying a tool call \"does not work on Windows\". No such sentence exists. The sentences that go stale if step 10 turns green are \"serving on macOS and Windows is not yet shown\", in these four files. Only the first two are in scope, and the story does not say who changes the other two. The blocked story's acceptance drops only the Linux-only statement. fix: State that the \"not yet shown\" sentences stay with story:mcp-stdio-on-macos-and-windows, or add status.md and docs/local-mcp-cli.md to scope and acceptance 7. Also reword acceptance 7's \"says the two files hold none\", which is already known to be true."
- file: .engineering/planning/story/windows-mcp-tool-call-answers-timeout.md
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "where: Acceptance 8 finding: CHANGELOG.md has no `## Unreleased` heading: line 3 is `## 0.76.0 — 2026-11-23`. The entry has no stated place. fix: \"Add `## Unreleased` above `## 0.76.0` if absent, with `### Fixed`, coordinating with story:settled-read-uses-leave-the-store.\""
```

Coverage against the source: the Windows MCP step going green by a fix and not by skipping is covered (acceptance 5, no `continue-on-error`). Nothing drafted is outside the source or story:mcp-stdio-on-macos-and-windows. The out-of-scope list matches the blocked story's Linux-only tests and the release-binary story. The macOS check in acceptance 5 is a no-regression check; it is fine as written. `.github/workflows/windows-host.yml` needs no change: its grep expects `1 passed`, and the story keeps that count.

Out of my lane: whether acceptance 3 is checkable, and whether the cancel redesign (the `try_clone` pipe) holds together.
