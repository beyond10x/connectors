# Connectors v0.8.0 — release gate

The first gate run on the **merged** state of `main`. Both constituent waves gated
on their integration branches, below the merge commits, so until this run nothing
had checked what `main` actually holds.

| | |
|---|---|
| command | `cargo run --locked -p connectors-build -- gate --msrv` |
| branch | `main` at `1faf4cc`, the commit `v0.8.0` tags |
| result | **35 gate steps, 85 test targets, every step exit 0** |
| log | [gate.log](gate.log), ending `GATE-EXIT:0` |

`CARGO_BUILD_JOBS=1`, `RUSTC_WRAPPER=""`, `TMPDIR` task-owned under `.local/tmp`.

## Scope

74 commits after this line's v0.2.0. Numbered 0.8.0 because the destination repository's published tags already reach v0.7.2. `CHANGELOG.md` § 0.3.0 has the user-visible list;
`release-plan:connectors-v080` holds the scope and its limits.

## What this run establishes

Every gate step on the merged tree: shared ESS vocabulary, seven adapter ESS models
compiled independently, 84 archived upstream source digests re-derived across six
manifests, formatting across sixteen packages, three descriptor-drift checks, a
workspace build and test, Clippy with `-D warnings`, four library-boundary builds,
`+1.88.0 check --workspace --all-targets`, scenario synthesis over 34 authored
scenarios, and `plan artifact validate`.

## What it does not establish

No live provider answered anything in this run. The GitLab write controls are
verified by disposable CLI journeys against a private HTTPS fixture; Helm release
reads are fixture-verified; MCP is contracts with no connection made. Kubernetes
and PostgreSQL were verified against real servers on 2026-09-11 and that evidence
is separate, at `docs/evidence/provider-sandboxes-20260911/`.

## Publication

Not published. `configuration-blocker:gitlab-v020-publication-target` is open —
this checkout has no publication remote, and the only configured remote is a local
recovery repository. `AGENTS.md` § Cutting a release is explicit that a local
commit or local tag alone is not a cut release, so **v0.8.0 is cut locally and
incomplete** until a destination exists.
