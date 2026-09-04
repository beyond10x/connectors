---
format: aep.planning-md/1
id: story:cli-surface-fences-assert-what-they-claim
kind: story
status: implemented
title: The CLI surface fences assert what they claim to
relations:
- derived_from: epic:cli-surface
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/catalog-build/tests/main/dependency_fence.rs
- confidence: cited
  path: crates/catalog-build/tests/main/ess_citation_fence.rs
- confidence: cited
  path: crates/catalog-build/tests/main/ess_claim_fence.rs
- confidence: cited
  path: crates/connectors-cli/tests/adversary_fence_probe.rs
- confidence: cited
  path: crates/connectors-cli/tests/adversary_fence_probe_pass2.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface.rs
- confidence: cited
  path: crates/connectors-cli/tests/cli_surface_drift.rs
- confidence: cited
  path: crates/connectors-console/src/output.rs
- confidence: cited
  path: crates/connectors-console/tests/adversary_budget_prose.rs
- confidence: cited
  path: docs/design/19-the-cli-surface.md
- confidence: cited
  path: ess/system/components.yaml
revision: 17
---
# Story: the CLI surface fences assert what they claim to

## Defect

An independent review of 2026-09-04's whole change set found the fences guarding the CLI surface
are green for the wrong reasons. Verified directly:

```
crates/catalog-build/tests/main/ess_claim_fence.rs:228  !declared.contains(claim) || sites.len() == 1,
crates/catalog-build/tests/main/ess_claim_fence.rs:261  !declared.contains(claim) || sites.len() == 1,
crates/catalog-build/tests/main/ess_claim_fence.rs:321  !declared.contains(claim) || !writers.is_empty(),
crates/catalog-build/tests/main/ess_claim_fence.rs:367  !declared.contains(claim) || guards.is_empty(),
```

Zero of those four sentences remain in `ess/system`. **Four of the five tests pass by their guard
clause.** The reviewer deleted `MaterializeObservation`'s three refusal outcomes — the exact defect
the fence exists for — and it stayed green. Only reinstating the literal old sentence makes it fail.

A test that cannot fail is worse than no test: the gate reports it green.

## The four blockers

| # | where | what was measured |
|---|---|---|
| 1 | `ess_claim_fence.rs:228,261,321,367` | four of five assertions are `!declared.contains(claim) \|\| …`, and no claim string survives in the specification |
| 2 | `cli_surface.rs:98`, `:78`, `:434-435` | `connection prune` added to the parser is absorbed by three edits — one `Unmodelled` entry with an arbitrary reason, its copy in the drift suite, and two numbers in the design document. 25/25 green, `ess/system` and `ess/generated` byte-identical. **The failure message instructs exactly this.** No check ties a `Forwarded` or `Read` reason to anything |
| 3 | `crates/connectors-cli/src/lib.rs:71,164`, `scripts/gate.sh:69-71` | editing `Admin`'s doc comment alone leaves 25/25 green — no fence reads `get_about`. `#[command(alias = "conn")]` on `Connection` is green, `connectors conn --help` works, and the fish completion lists it zero times. The gate comment claims the parser's `--help` lines are compared; only the generated tree's are |
| 4 | `components.yaml:39-60` | 20 `publishes: events`; 16 carry neither a citation nor an `UNMAPPED:` marker, and no file under `crates/` mentions any of 13 sampled names. `ess_citation_fence.rs`'s `entries_of_block` is only ever called for `errors`. ESS-COMMAND-007 forces an emit or error per outcome, so they are schema-forced and unmarked |

## Also found

- `cli_surface_drift.rs:287` still carries the literal-filter `deployment_protocol_modules` the
  contract replaced, while its header claims a faithful restatement of every assertion.
- `output.rs:28` and `CHANGELOG.md:29-31` claim a row fits a terminal at a 120-column budget. 66 of
  71 `connectors providers` lines exceed 120 columns; the longest is 237. The code is as designed —
  the claim is false as worded.
- `release.yml:105-108` pins `ess` at `--tag 0.16.0`; 0.18.0 is released and installed.
- `release.yml:32-33` says eleven workspaces and 39 GB; `gate.sh:8-9` says twelve and 41 GB. Twelve
  is right.
- `cli_surface_drift.rs:726` says the emitted manifest is in neither `members` nor `exclude`;
  `Cargo.toml:76` excludes it, and without that line `cargo metadata` exits 101.
- The generated files say to regenerate with `cargo xtask synth` and `ess synthesize`. Neither is
  the command `scripts/gate.sh` runs.

## Shape

A guard clause that makes an assertion conditional on a string still being present is the failure
mode, not an instance. Every fence assertion is rewritten so it holds against the specification as
it is, and a claim that disappears is a **failure**, not a pass. Where a fence needs a sentence to
exist, it asserts the sentence exists.

The exception list stops being a list a failure message invites you to extend: an entry needs
something checkable behind it, and the message stops naming the escape hatch first.

## Acceptance

- Each of the reviewer's four constructions is reproduced as a test and each goes red before the
  fix: deleting `MaterializeObservation`'s refusal outcomes, adding `connection prune`, editing a
  doc comment, adding `alias = "conn"`.
- No assertion in any fence is conditional on a claim string being present.
- `bash scripts/gate.sh` exits 0, every step read as its own status.

## Provenance

Independent review, `fable` model, over `e162cbf..wave/cli-contract` — 37 commits, 366 files,
+46531 −4087. Its verdict was *not mergeable as a gate*. The merge to `main` (`7b2d680`) had already
happened when the review returned; nothing is pushed.
