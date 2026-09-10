# GitLab pinned-head validation — 2026-09-10

`merge_request.validate` is usable through the existing local operation CLI. It
checks the requested exact MR source SHA and selected successful head pipeline,
with opened/non-draft/mergeable conditions, and reports ordered blockers when
those observations do not pass. It always reports `merge_performed:false`.
This is a read observation, not an approval or stable future-write receipt.
[Native semantics](../../../adapters/gitlab/contracts/merge-requests/v1alpha1/validation.md)
state the precise predicates, asynchronous-check limits and provenance boundary.
Dedicated GitLab sandbox evidence and the remainder of C14 are still required.

## Source and executables

Parent commit: `4aa3354788172616202b0d7dacea65bd5b9a35b0`; the commit containing
this receipt identifies the increment. [Source hashes](source-inputs.sha256)
cover 446 non-planning source inputs, excluding historical evidence and wave
receipts. All match both the current files and the
[package source capture](package-source.sha256.json.gz). The package's source
HEAD is the parent; its complete source digest includes the uncommitted increment.
Cargo.lock, shared host/SDK/core code, CLI/parser, credentials and migrations are
unchanged. The pinned GitLab OpenAPI is unchanged at SHA-256
`f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`.

The CLI tests use an optimized generic CLI with SHA-256
`65defbd8e8a4254e0914c06f5da71b102175b2906424d7d3a58bd5de51e4e198`, and the
test-profile native GitLab executable with SHA-256
`5390e8b4db13f34c595f9e1dea708fc056aef731ce60bf3377f355a5e3fe21e8`.
[Their hashes](executable-sha256.txt) and read-only copies under
`.local/tmp/gitlab-runtime-20260910/mr-validation/verified-bin` retain that exact
pair before packaging rebuilds the mutable Cargo output path.

The packaged development binary is separately identified by
[package evidence](package-build-evidence.json), with SHA-256
`8960a5baedde6709efd110475ae1470378cb0027ca113adb6097872671d0716c`.
Local image: `sha256:c0a8da5702c58739cd326b63882edfa57e241cbdf001c9abaeaefca4bd385b82`.
The [rootfs manifest](package-rootfs.sha256.json) and
[realization](package-realization.json) describe its exact local inputs.
No image was published, and packaging is not container/provider acceptance or
proof of reproducibility across build hosts.

[Tool hashes](tool-inputs.sha256) and [versions](versions.txt) preserve ESS,
Cargo/dependency, website, GNOME Keyring and D-Bus inputs. The same hashes still
match the previous approval-key evidence. Its eleven explicit key tests are not
repeated because their runtime, test, CLI and dependency inputs are unchanged;
ordinary host tests run again in this gate. Clock research added no dependency,
clock trust configuration or production time source.

## Verification

Commands use `TMPDIR=$PWD/.local/tmp/gitlab-runtime-20260910`,
`CARGO_TARGET_DIR=$TMPDIR/target` and `CARGO_BUILD_JOBS=2`. Builds, generation,
CLI fixture suites and packaging are serialized. The fixture credentials,
private HTTPS server, D-Bus and qualified keyring are disposable and synthetic.

| Command | Observed result |
|---|---|
| `cargo run --locked --offline -p connectors-spec -- --generate --specification adapters/gitlab/spec/adapter.json --output adapters/gitlab/generated` | [Generation passed](generate.log.gz). |
| `cargo test --locked --offline -p connectors-gitlab --test merge_requests` | [12 native tests passed](native-tests.log.gz); the gate repeats them successfully. |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | [Complete gate passed](repository-gate.log.gz): shared/native ESS, generation, structural/runtime conformance, workspace tests, Clippy, boundaries, Rust 1.88 and AEP. |
| `CONNECTORS_TEST_CLI=$CARGO_TARGET_DIR/release/connectors cargo test --locked --offline -p connectors-gitlab --test local_runtime -- --ignored --test-threads=1 --nocapture` | [Final six journeys passed](cli-all-final.log.gz) in 92.79 seconds, including the final HTTP-method assertion. |
| `cargo run --locked --offline -p connectors-build -- package --output .local/tmp/gitlab-runtime-20260910/mr-validation-package --image connectors-v2-gitlab:validation-20260910 --jobs 2` | [Local image and realization passed](package.log.gz). |
| `npm run typecheck` / `npm run build` in website | [Typecheck](website-typecheck.log.gz) and [build](website-build.log.gz) passed; 117 indexed pages, 477 audited public files. |
| `cargo run --locked --offline -p connectors-build -- docs --check` | [45 contract pages, 96 reference pages, no drift](docs-check.log.gz). |
| `cargo test --locked --offline --manifest-path website/examples/realization/Cargo.toml --target-dir website/.cache/demo/target` | [15 examples passed](website-examples.log.gz). |

Native ESS has 17 declarations; shared ESS remains at 415. The ordinary host
suite reports 77 passes and 16 ignored auxiliary/integration entries, separately
from explicitly executed CLI suites. Presentation/browser behavior is unchanged;
no new browser-interaction acceptance is claimed.

The six new native tests cover every negative predicate, exact selector/SHA/ID
binding, fork/target project IDs, SHA-256, nullable and malformed data, unknown
statuses, stable blocker order, no legacy-pipeline fallback, pre-HTTP schema/project
refusals and safe provider errors. The production CLI journey removes its entry
credential file, restarts the owner, observes a passing validation followed by a
changed-head blocker for the same input, then missing checks; schema/project and
removed-policy refusals perform no additional provider work. Every recorded
request is GET. The five earlier persistence, expiry, repair/stop, CI and MR-window
journeys also pass.

## Corrections and planning

[The first CLI run](cli-validation.log.gz) exposed a test-router mutex acquired
twice while selecting fixture responses, blocking native credential validation.
The exact test process was stopped after its failure while cleanup was stuck;
the router now holds one guard for both response selectors. Production locking
was unchanged. [The second run](cli-validation-fixed.log.gz) refused unsupported
because the new native operation lacked explicit Read classification in the
GitLab bootstrap; that exact classification was added without permitting unknown
effects. [The third run](cli-validation-final.log.gz) reached the expected results
but its request-count assertion did not allow the existing empty query delimiter.
The assertion now checks the route and absence of query parameters.
[The subsequent six-test run](cli-all.log.gz) passed in 99.32 seconds; the final
run above additionally verifies all recorded HTTP methods. No failed run is
counted as acceptance. A rejected patch application changed no files and was
resolved by rereading current source.

Eight immutable AEP critic records retain both rounds. The acceptance reviewer
identified two defects: independent cases joined in one acceptance and missing
positive preconditions. Both were fixed, each with its own review-outcome record;
all four final critics approve. Sonnet was unavailable, so the inherited model
was disclosed; the fourth reviewer followed the first three under the worker
limit. Root was the sole planning/source writer, directly on primary main.
[AEP validation](aep-reviewed.log.gz) is valid for 214 artifacts; it reports 127
historical/empty-array findings-block notices. Historical reviews were preserved.

The story stays active because dedicated sandbox evidence is missing. The parent
still owns actual approval issuance, authenticated subject policy, qualified time,
full mutation dispatch and native writes; the create/update SHA-guard decision is
unanswered. Complete GitLab remains before Kubernetes, PostgreSQL, MCP and remaining
providers. This receipt does not reduce the overall acceptance or reproducible
release requirements. A clean remote Atlas source at
`4eedd3636d499df1299d2a6d1b5415531b418b9d` supplies the unchanged private bot
wrapper; [authority hashes](atlas-authority.sha256) identify that read-only input.
