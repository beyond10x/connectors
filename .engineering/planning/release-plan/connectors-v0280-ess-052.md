---
format: aep.planning-md/3
id: release-plan:connectors-v0280-ess-052
kind: release-plan
status: active
title: 'Release 0.28.0: ESS 0.52.0 and the duplicate-key decoder refusal'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-05T12:17:36Z", actor: "human:timo", revision: 2, executor: "agent:claude"}
---
## Outcome and authorization

Prepare minor release 0.28.0, which ships the ESS 0.52.0 upgrade. Operator request of
2026-10-05: prepare the beyond10x/connectors release v0.28.0 from origin/main after the
history rewrite. Candidate base is remote main 12fceb8 (PR 92, ESS 0.52.0 from 0.45.0). The
tag namespace ends at v0.27.0, an ancestor of that base. Recheck both before tagging.

Minor, not patch: the ESS 0.52 generated CLI decoder refuses `operations invoke` business
input with a duplicate object key itself, as the `ess-cli/1` code `cli_dynamic_input` with
empty data, where 0.27.0 answered the owner's application `failure` with `data.code =
invalid_input`. Exit code 2, empty stdout and no dispatch are unchanged, but the error code
is part of the CLI contract (scenario C05, semantics §5) and a caller may match on it. Patch
releases here carry fixes, tests or specification changes with no caller-visible contract
change (0.13.1-0.13.3, 0.15.1, 0.21.1, 0.25.1). Existing configurations, stored metadata
and adapters are unchanged.

## Released scope (v0.27.0..12fceb8)

- ESS 0.52.0 (from 0.45.0): `crates/connectors-spec/toolchain.json`, the seven ESS crates in
  `Cargo.toml`, `Cargo.lock`, `docs/development.md`; the CLI contract and the metadata
  Entity Runtime definitions regenerated (PR 92).
- The local metadata conformance suite runs 290 scenarios (was 289): ESS synthesizes
  `ReviseLocalApprovalPolicy/outcome/exhausted` again (beyond10x/ess#251); 20 synthesis
  refusals (was 21).
- Duplicate-key business input is `cli_dynamic_input` (above); `cli_surface_minor` tests
  updated in PR 92.
- The Pages redirect façade is repinned at the current Website control (c57f3ecbf, CI only).
- Release preparation: workspace version 0.28.0 (14 workspace lock entries), CHANGELOG
  0.28.0 section moving Unreleased into it, README and website status page, CLI contract
  semantics §5 and scenario C05 naming the duplicate key as `cli_dynamic_input`, and the
  MCP selected-intent contract (`discovery-contract.json`/`.md`) and `docs/local-mcp-cli.md`
  naming the ESS 0.52.0 pin. AEP pin stays 0.65.0.

## Unfinished work and missing evidence

- No provider, operation or runtime surface is added. MCP delivery, Entity Runtime issue
  51, live Zendesk evidence, sustained-read acceptance and the remaining provider plan stay
  open. This release completes neither a provider batch nor its parent plan.
- `crates/connectors-build/tests/cli_spec_mapping_adversary.rs` still names ESS 0.45 in a
  doc comment; not changed here.

## Lifecycle

Active: release preparation in managed worktree connectors-rel028, uncommitted. Steps 5-7
(bot PR and merge, annotated tag, bot-authored release page) are the operator's and are not
done. The plan moves to implemented only after the tag, required checks and hosted release
page are verified.

## Release candidate evidence — 2026-10-05

Local candidate: 12fceb8 plus the uncommitted release preparation (Cargo.toml, Cargo.lock,
CHANGELOG.md, README.md, website/docs/introduction/status.md, contracts/cli/v1alpha1/
semantics.md and scenarios.md, adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json
and .md, docs/local-mcp-cli.md, this artifact). Toolchains: ESS 0.52.0, AEP 0.65.0, Rust
1.99.0 default with rustfmt and Clippy, MSRV +1.88.0 and +1.91.0.

- Repository gate `cargo run --locked -p connectors-build -- --aep <AEP 0.65.0> gate --msrv`
  with `CONNECTORS_ESS` selecting ESS 0.52.0 ran 2026-10-05T11:50:30Z-12:12:31Z and exited 0:
  1289 passed, 0 failed, 65 ignored. Log SHA-256
  50164f345d900f25266eb58ff3e2f866500182ba8be3da089e2ae45b62a17918. The CHANGELOG
  Verified lines were written after it; no gated input changed.
- Website: npm ci, typecheck, build (494 public files audited, no private paths),
  reference:check (45 contract and 100 reference pages, no drift) and test:examples
  (15 passed) all exited 0. Log SHA-256
  43a1391af9d4efb6140bffa762ffb546193b01e8693cef172112d7f8bd786703. npm ci reported
  41 audit advisories (1 low, 6 moderate, 34 high); no dependency changed.
- Not run: website test:browser and test:ui (presentation inputs unchanged since v0.27.0),
  ignored provider suites and any live provider call.

The candidate tree changes when the operator commits it; the gate and website results
apply to that tree only if the committed bytes equal these.
