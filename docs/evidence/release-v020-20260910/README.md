# GitLab source release v0.2.0 — prepared 2026-09-11

Implementation base: `a5b399d4f790e993aa3ab76f6a61ac1ee25b6c7a`.
The containing release commit adds product versions, changelog, website guidance,
the operator-requested release procedure and its AEP record. Runtime Rust and
native/generated schemas remain those of the implementation base. No subsequent
clock research or provider implementation is included. Preparation began on
September 10; the changelog date follows the September 11 Europe/Berlin release
checkpoint. [Tool versions](versions.txt) retain both clocks.

## Verification

All commands ran in the isolated managed release checkout, using two Cargo jobs.
`CARGO_TARGET_DIR=$PWD/.local/tmp/release-v020/target` kept compiler output separate
from the primary checkout. `CONNECTORS_ESS` selected the exact qualified ESS 0.20.0
binary at source commit `6f7ef46163e758f3401945d1a946e0fc80ebc003` through its source
receipt; the ambient ESS 0.22.1 was not selected. The build TMPDIR was the release
checkout's `.local/tmp/release-v020`. Explicit custody/CLI fixtures used the shorter
task-owned primary path `.local/tmp/r020` so Unix socket paths fit Linux's limit.

| Command | Result |
|---|---|
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | [Passed](gate.log.gz), including shared/native ESS, generated descriptors, conformance, workspace tests, Clippy, boundaries, Rust 1.88 all-target checks and AEP. Ordinary host tests: 77 passed, 16 ignored; native MR tests: 12 passed. |
| `cargo build --release --locked --offline -p connectors -p connectors-gitlab` | [Passed](release-build.log.gz), with both executable packages at 0.2.0. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test --locked --offline -p connectors-host --lib local::approval_keys -- --include-ignored --skip key_crash_child --test-threads=1` | [11 passed](keys.log.gz) in 9.02 seconds, including controlled crashes and production key commands. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | [6 passed](cli-journeys.log.gz) in 99.07 seconds, including restart reuse, real expiry/revalidation, repair/stop, CI, MR collection and changed-head validation. |
| `npm run typecheck` | [Passed on final presentation sources](website-typecheck-final.log.gz). |
| `npm run build` | [Passed](website-build.log.gz): 117 indexed pages and 477 audited public files. |
| `cargo run --locked --offline -p connectors-build -- docs --check` | [Passed](reference-check.log.gz): 45 contract pages and 96 reference pages, no drift. |
| `cargo test --locked --offline --manifest-path website/examples/realization/Cargo.toml --target-dir website/.cache/demo/target` | [15 passed](examples.log.gz). |
| `CONNECTORS_WEBSITE_URL=http://127.0.0.1:3186 npm run test:ui` | [Passed](website-ui-final.log.gz), including navigation, adapter filters, search, themes, responsive layouts, diagrams and keyboard interaction. |
| `CONNECTORS_WEBSITE_URL=http://127.0.0.1:3186 npm run test:browser` | [Passed](website-browser.log.gz), including both walkthroughs, six failures, explicit read action, playback and advanced examples. |

The browser suites used an owned server for this checkout's production build;
that server was stopped afterward. Fixtures used synthetic credentials and
private HTTPS, D-Bus and qualified GNOME Keyring services. No user collection or
provider sandbox was used.

[Final AEP validation](aep-final.log.gz) reports 216 valid artifacts and the
127 existing review-findings notices. Its full output was relayed verbatim;
historical review records were preserved.

## Inputs and corrections

[Source hashes](source-inputs.sha256) cover 490 tracked non-planning/non-evidence
inputs. [Executable hashes](executables.sha256) distinguish the optimized CLI and
native adapter build from the test-profile native adapter used by the CLI suite.
[Tool hashes](tool-inputs.sha256), [Atlas authority hashes](atlas-authority.sha256)
and [website output hashes](website-output.sha256) retain exact identities.
Timestamped receipts are separate from generated payloads.

[Version checking](versions-check.log.gz) confirms twelve Connectors workspace
packages and the website at 0.2.0, with third-party lockfile entries unchanged.
Independent generated `gitlab-types` 1.0.0, CLI fixture 0.0.0 and example package
versions retain their own meaning. An initial ad-hoc version check incorrectly
counted the generated workspace member as a product package; the corrected check
uses the owning Connectors manifests. A wrong working-directory invocation of that
check wrote no file and was rerun from the root. Neither changed product source.

The [initial UI run](website-ui.log.gz) found a stale expectation of two GitLab
search pages. Inspection found the guide and four native contracts, including
completed CI/MR work. The test now checks those exact routes and their contract
filter, and the final run passes. Updated status/filter label expectations match
the release copy. No browser/product behavior was changed to satisfy the test.

## Publication and coordination

The operator authorized commit, annotated tag and push. The intended publication
repository URL was still missing at this preparation checkpoint; the only
configured remote is the local `ess-recovery` repository. A local recovery push
is not public distribution. `configuration-blocker:gitlab-v020-publication-target`
records the missing input, and `release-plan:gitlab-v020` retains the verified
scope. Actual remote branch/tag advertisements, retained outside this immutable
release commit, establish publication; this receipt does not claim it occurred.

A clean Atlas primary checkout at `4eedd3636d499df1299d2a6d1b5415531b418b9d`, freshly
verified against remote main, supplies the existing bot wrapper. No Atlas content
or registration is changed. Release worktree finish/GC uses exact reviewed IDs
only after recovery publication, with its receipt retained separately.

The parent thread resumed clock changes in the primary checkout during release
preparation. They remain untouched. Primary integration is a handoff to that
writer after its work is safe to merge; the release commit remains directly above
the requested implementation base. Planning-store reconciliation must use AEP.

Dedicated GitLab sandbox acceptance, approval issuance, qualified production
approval time, governed writes and the remaining provider plan stay open. The
[implementation receipt](../gitlab-mr-validation-20260910/README.md) preserves its
prior local packaging evidence; this source release does not claim a newly built
0.2.0 image, binary/registry publication, website/cloud deployment or cross-host
reproducibility.
