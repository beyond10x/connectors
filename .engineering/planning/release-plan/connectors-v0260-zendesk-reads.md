---
format: aep.planning-md/3
id: release-plan:connectors-v0260-zendesk-reads
kind: release-plan
status: active
title: 'Release 0.26.0: Zendesk Support reads through the catalog provider'
relations:
- delivers: story:catalog-zendesk-reads
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-05T03:10:40Z", actor: "human:timo", revision: 2}
---
## Outcome and authorization

Prepare minor release 0.26.0, which ships Zendesk Support reads through the catalog
provider. Operator request of 2026-10-05: "new connectors release including zendesk
released". Candidate base is remote main da3a04726 (PR 86, Zendesk reads). The tag
namespace ends at v0.25.1. Recheck both before tagging. Minor, not patch: the catalog
provider gains a new provider (`zendesk`), a new auth profile (`zendesk.basic`) and seven
new read operations, and the inventory follows parameter `$ref` and expands `deepObject`
query parameters. Existing configurations and bundles are unchanged.

## Released scope (v0.25.1..da3a04726)

- Zendesk Support through the catalog provider: `tickets.incremental`, `users.incremental`,
  `organizations.incremental`, `ticket.show`, `user.show`, `organization.show` and
  `ticket.comments`, read-only, from the pinned and redacted Support API document
  (story:catalog-zendesk-reads, PR 86).
- Catalog inventory: local parameter `$ref` and exploded `deepObject` scalar expansion. Only
  the Zendesk bundle and the bundle index changed among committed bundles.
- `connectors-build redact` (rule `upstream-redaction/2`) and its gate check.
- MCP contracts with document cases (PRs 80, 82, 83): outbound invocation results, inbound
  capability projection, outbound auth lifecycle, inbound mutation replay, composition
  provenance and selected local CLI intent. Specification only; no MCP runtime.
- Release preparation: workspace version 0.26.0 (13 workspace lock entries), CHANGELOG
  0.26.0 section, README and website status page.

Excluded: the paused ESS 0.52.0 upgrade (worktree connectors-ess052) ships in a later
release. ESS pin stays 0.45.0, AEP pin 0.65.0.

## Unfinished work and missing evidence

- Zendesk has not been run against a live account. No sandbox evidence exists: every read
  was verified only against a local HTTPS fixture with synthetic records. Unconfirmed live:
  the API-token basic header, organization export page overlap at `end_time`, and which
  ticket changes move a ticket into the export.
- The provider does not walk pages, bound `per_page`/`page[size]` or retry on 429. 35 Zendesk
  operations with `deepObject` parameters stay unsupported; none is shipped.
- story:catalog-zendesk-reads stays in its current status; this release does not close it.
- MCP delivery, Entity Runtime issue 51, sustained-read acceptance and the remaining provider
  plan stay open. This release completes neither a provider batch nor its parent plan.

## Lifecycle

Active: release preparation in managed worktree connectors-release-0260, uncommitted. Steps
5-7 (bot PR and merge, annotated tag, bot-authored release page) are the operator's and are
not done. The plan moves to implemented only after the tag, required checks and hosted
release page are verified.

## Release candidate evidence — 2026-10-05

Local candidate: da3a04726 plus the uncommitted release preparation (Cargo.toml, Cargo.lock,
CHANGELOG.md, README.md, website/docs/introduction/status.md, this artifact). Toolchains:
ESS 0.45.0, AEP 0.65.0, Rust 1.98.1 default with rustfmt and Clippy, MSRV +1.88.0 and +1.91.0.

- Repository gate `cargo run --locked -p connectors-build -- gate --msrv` exited 0 at
  2026-10-05T03:30:26Z: 1275 passed, 0 failed, 65 ignored. Log SHA-256
  3cb5996e10ce655ac08bef910ea3bef895ebac6b6f053fdce8a1719ffbfbb459.
- Website: npm ci, typecheck, build (482 public files audited, no private paths),
  reference:check (43 contract and 97 reference pages, no drift) and test:examples
  (15 passed) all exited 0. Log SHA-256
  168972d335b644b570443bf86e26abce342f44cdfba6e412493cd25989f76910. A first
  reference:check run before build exited 1 because the fresh tree had no generated
  `website/.cache/reference.json` (log SHA-256
  e94937243b95208fb6402cfeb47d01344594f79e17261160d67eacea5570c4cf). npm ci reported
  41 audit advisories (1 low, 6 moderate, 34 high); no dependency changed.
- Not run: website test:browser and test:ui (presentation inputs unchanged since v0.25.1,
  which passed them), ignored provider suites, and any live Zendesk call. No Zendesk
  sandbox evidence exists.

The candidate tree changes when the operator commits it; the gate and website results
apply to that tree only if the committed bytes equal these.
