# Review warnings and pinned tools — 2026-09-11

The 139 AEP missing-findings warnings are removed without changing any original
review-result file. Of these, 88 already contained an explicit empty findings
block and exposed an AEP validator bug. The other 51 immutable reports now have
source-bound verification-report transcriptions: 227 historical findings, with
18 ungraded observations preserved as `unspecified`. Nineteen reports explicitly
state zero residual findings within their reviewed scope. A transcription adds
no critic run, approval, outcome record or current implementation claim.

The [planning owner](../../../.engineering/planning/specification/review-findings-toolchain-upgrade-20260911.md)
relates all 51 supplements. AEP checks their original body SHA-256, target kind,
unique binding and canonical findings block. Missing, stale, ambiguous or malformed
supplements do not satisfy validation. Original source excerpts and verdicts remain
available, including same-pass corrections and boundaries on partial approvals.

## Tool identities

| Tool | Version | Exact source | Local executable SHA-256 |
|---|---|---|---|
| ESS | 0.22.2 | `6b666e58f2e87dd8798d27f935e9a012203296a3` | `8096c280fc4e4cab040d64adc92735dee1e9bea66cefb20dd973f7705e4c03dd` |
| AEP | 0.55.0 plus local patch | `4eb999e0ae3cc77d1c387152e23a85ad4eae86dc` | `665e0d01965315b2e72c6a10d89e8cc88051565ea479dde09ac884d5e661a27e` |

Both builds used Rust and Cargo 1.98.1. Receipts bind the source, version, build
arguments, Cargo.lock and executable digest; AEP also binds patch SHA-256
`73d78e50369adf0853e457a226002bcc84e2b43a50903f28ed3e6afdd046e68f`.
See the owning [ESS pin](../../../crates/connectors-spec/toolchain.json),
[AEP pin](../../../crates/connectors-build/aep-toolchain.json), and
[build instructions](../../development.md#pinned-tools). Receipts attest a local
build observation, not an upstream signature or cross-machine reproducibility.

Upstream tags were rechecked at handoff: ESS 0.22.2 and AEP 0.55.0 remain the latest
releases. The AEP release tag peels to `28abe09bb6e5b0a6b4db839f6bf5693957d39324`;
the selected source includes the subsequent public main changes. The findings fix
is local candidate `6fc586429db303a06f5ca643b95caf2ffa34d953`, authored and committed
as `b10x-bot[bot]`. It is not an upstream released fix. Gates refused its local
recovery push because that destination differs from the enrolled AEP repository;
the clean managed AEP worktree is retained for integration. The consumer patch
allows builds from the already public source commit without publishing that candidate.

## Verification

Commands ran with task-owned temporary directories and bounded Cargo jobs.
Raw logs are retained in the handoff worktrees under `.local/tmp/review-warnings/`.

| Command / observation | Result |
|---|---|
| Empty-findings CLI regression, before correction | Failed with the false missing-findings diagnostic |
| AEP focused findings tests and full `task check` | Pass; full gate includes Clippy, workspace tests, docs/schema checks, Rust 1.85 and website build |
| `cargo run --locked -p connectors-build -- toolchain --source <ESS source>` | Exact ESS source built and receipt checked |
| `cargo run --locked -p connectors-build -- aep-toolchain --source <AEP repository>` | Exact public source plus pinned patch built and receipt checked |
| GitLab bundle regeneration | Only manifest ESS provenance changes; generated code, descriptor, coverage and refusal bytes are unchanged |
| ESS CLI reference generation, exact output adoption, and `connectors-build cli` | All 10 generated files byte-identical; 81 structural cases, 2 cached expectations, 5 acquisition and 3 page consistency cases pass |
| `cargo run --locked -p connectors-build -- gate --msrv` with the final pins | Exit 0; all required checks pass, including Rust 1.88 |
| Pinned AEP `plan artifact validate --strict` | Exit 0; 283 artifacts valid, no warnings |
| `git diff --exit-code -- .engineering/planning/review-result` | Exit 0; original reviews unchanged |
| Website `npm ci --allow-git=root`, `npm run build`, `npm run reference:check` | Exit 0; 99 reference pages, 120 indexed pages, 487 public files audited; no private path markers or reference drift |

The AEP full-gate run uses a task-owned TMPDIR outside any project because an
existing project-discovery test requires that location. An initial in-project
TMPDIR run exposed that assumption; no product change was made for it. AEP's optional
live PostgreSQL check skipped because `ENTITY_POSTGRES_URL` was unset.

ESS conformance synthesis still records 315 scenarios (34 authored) and 22 explicit
refusals for existing uninstantiable approval-issuer states. Those obligations were
preserved; synthesis and structural compilation do not prove runtime conformance.
GitLab's original OpenAPI 3.0 input still meets the documented ESS OpenAPI 3.1 refusal.
This upgrade refreshes no vendor input and adds no provider operation.

## Retained log digests

| Log | SHA-256 |
|---|---|
| `connectors-gate-final.log` | `0f422516c5024d7caa163bfdee55734f55ad8a2f66b182517e02397bdc8b60c1` |
| `ess-toolchain-build.log` | `7e6dbf73988570d9391d48ea49fb6faa2a61579f8360983aa8ffa8527c06e513` |
| `aep-toolchain-build.log` | `b7da49eeed785d5977e00088d9aa5ae1c054c45fda4d0732a3d5364c54d30f55` |
| `website-build.log` | `1fcff8ddb358c8f21b1498dec9a0af2afa06c632c583d4a34d170ceacd56e63a` |
| `website-reference-check.log` | `9f74a377769e825853235cbe38984ae51ec014f03d15155c179601921fd2d6dd` |
| `final-planning-validation.log` | `2d7187521876223bff41d80ca0ca37c88ee162eb3e63f718b987f794e8c59637` |

Source integration and primary-store replay remain with the existing single
writer. The concurrent GitLab implementation checkout is not modified by this handoff.
