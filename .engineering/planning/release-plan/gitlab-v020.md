---
format: aep.planning-md/1
id: release-plan:gitlab-v020
kind: release-plan
status: active
title: Release v0.2.0 from the verified GitLab CLI increment
relations:
- informed_by: initiative:complete-local-connectors
- informed_by: story:gitlab-mr-validation
revision: 4
---
## Scope and authority

Cut source release v0.2.0 directly above a5b399d4f790e993aa3ab76f6a61ac1ee25b6c7a, the completed GitLab MR-validation increment, as explicitly requested by the operator in the side conversation. Include the operator-requested release definition in AGENTS.md, owning package-version inputs and lockfiles, CHANGELOG.md, README support claims, and website documentation. Do not include later clock research or additional provider implementation.

This authorizes committing, tagging and pushing the release source; it supersedes the initiative's earlier local-only publication exclusion for this release. Cloud/website deployment, registry or binary distribution, hosted release-page creation and Atlas registration remain outside this request. The only currently configured remote is the local ess-recovery repository; the publication URL has been requested and must be resolved before publication is claimed.

## Released behavior and limits

The released scope is the Linux x86_64 persistent GitLab CLI, eleven native GitLab reads including exact-head MR validation, protected signing-key management, and tested private mutation/audit/approval foundations. It preserves describe/invoke/serve and the existing Kubernetes/PostgreSQL standalone reads. Public approval issuance, GitLab writes, a qualified production approval clock, Kubernetes/PostgreSQL persistent lifecycle binding, MCP and remaining providers are unfinished. Dedicated GitLab sandbox acceptance and the MR create/update SHA-guard decision remain open. This release does not close those artifacts or the overall initiative.

The previous v0.1.0 tag is a specification milestone. v0.2.0 records the added usable runtime surface; contract-family versions and independent generated fixture/example package versions are not product version numbers. Existing installed configuration and credentials are not automatically migrated. Minimum Rust remains 1.88.

## Verification and delivery

Use an isolated managed worktree based on the exact implementation commit, one implementation/planning writer, task-owned TMPDIR and two Cargo jobs. Check version/lockfile consistency, run the required gate with MSRV and affected website checks, and rerun production GitLab CLI journeys against release-version executables. Keep prior fixture evidence with its exact source identity; do not reinterpret it as sandbox acceptance or cross-host reproducibility.

After verification, commit through a clean Atlas authority checkout freshly verified against remote main; verify bot author and committer, and create an annotated v0.2.0 tag with bot tagger identity at that exact release commit. Publish main and the exact tag without rewriting existing tags or bypassing hooks. Verify advertised remote main and peeled tag resolve to the intended commit, retain the publication receipt, integrate primary main without overwriting concurrent changes, and finish/GC only exact reviewed task worktree IDs.

The release plan records readiness and verification before publication. Git's verified remote branch/tag advertisements are the authoritative publication evidence; preparing this plan is not publication.

## Coordination

The parent thread has resumed clock implementation on primary main, including uncommitted CLI, host, model and AEP changes. Do not overwrite, stash, reset or merge into that active checkout. The release remains based on a5b399d4f790e993aa3ab76f6a61ac1ee25b6c7a in its isolated worktree and excludes those changes. Publish the reviewed release commit and tag from this checkout; hand the exact commit to the primary writer for subsequent integration using AEP for planning-store reconciliation. Primary main cleanliness is deferred while that independent work is active, not falsely reported as release cleanup. The release worktree itself still requires verified recovery and exact-ID cleanup.

## Verification outcome

The release-version full repository gate with Rust 1.88, optimized build, eleven explicit key tests, six GitLab production CLI journeys, website typecheck/build/reference check, fifteen example tests and both browser suites pass. docs/evidence/release-v020-20260910/README.md retains exact commands, logs, inputs, artifact hashes and the corrected stale browser-test expectation. Runtime Rust, native/generated operation schemas and shared contracts/ESS remain unchanged from the implementation base. The changelog release checkpoint is September 11 in Europe/Berlin.

configuration-blocker:gitlab-v020-publication-target remains open because the publication URL has not been supplied. Prepare the local commit and annotated tag and preserve them in the configured recovery remote; do not claim intended source publication from that recovery push. All implementation, sandbox and parent-goal statuses remain unchanged. The active primary checkout is preserved for its own writer.
