---
format: aep.planning-md/1
id: specification:public-documentation-website
kind: specification
status: draft
title: Human documentation and an interactive contract website
relations:
- derived_from: vision:independent-contract-adapters
- informed_by: specification:contract-driven-connectors-design
- informed_by: specification:core-model-closure-20260909
revision: 8
---
## Context

The operator approved a human-documentation and website-design plan on 2026-09-09, then explicitly requested a running Docusaurus development server to observe the website being built. That follow-up extends this pass into local website implementation. Integration builders are the primary audience; navigation is Home / Introduction, Contracts and Adapters. All documented adapters are visible with explicit status. Guided examples use executable Rust/WASM behavior.

## Deliverables

Create VISION.md, refine README.md for humans and AGENTS.md for agents, preserve detailed build/operating instructions in focused guides, and write [the website design](../../../docs/website-design.md). Implement the local Docusaurus site under website/ and leave its development server running for the operator. Curated authored pages and generated contract reference views share an explicit publication inventory. Implement and verify selected guided examples through a separate example composition and Rust realization; do not change production semantic guarantees to make demonstrations work.

## Ownership and scope

This specification owns root documentation, docs/development.md, docs/running-services.md, docs/website-design.md, docs/website-verification.md, website/, and the narrow Rust tooling/example surfaces required to generate its reference and execute demonstrations. The linked vision owns direction. The original design and reviewed model closure inform the technical boundaries. One agent writes this checkout and planning store. No runtime-story decomposition or parallel scheduling is being claimed.

No external publication, deployment, Atlas integration or consumer change is included. The development server is a local preview. Production adapter behavior and the reviewed contract baseline remain independent of example implementations.

## Acceptance

The repository has distinct human and agent entrypoints, accurate support claims, valid links, a documented website architecture, and a running local website with the three required navigation sections, curated adapter coverage, generated contract reference, and explicitly bounded interactive example behavior. Public content excludes local/private evidence by construction. Relevant generation, website, example and planning checks pass; unsupported semantics are visible rather than simulated as proven enforcement.

## Initial feasibility evidence

Pinned ESS 0.20.0 produced ess-docs/1 with 21 pages and a browser scaffold from the shared model. The catalog has 20 entities and 53 commands, with zero dispatchable commands because the shared root declares no component acceptors. The language-neutral synthesis plan reports 342 generated capabilities, 59 obligations and zero plan refusals; the browser target separately reports 53 refusals and six weakenings. This initial probe did not build WASM or execute behavior. The website must supply a separate example composition and explicit implementations before claiming executable examples.

## Status

Local implementation is complete. The development server runs at http://127.0.0.1:3100/ with hot reload. The website includes 40 selected canonical contract pages, 48 ESS model pages from six model roots, curated introductions and all documented adapter guides. VISION.md, README.md and AGENTS.md have distinct audiences, with detailed build and service instructions preserved in focused guides.

Three bounded Rust/WASM scenarios demonstrate discovery versus access, readiness versus operation eligibility, and uncertain mutation outcomes. Six canonical mutation commands use generated ports and typestate transitions. Discovery/readiness use generated value types with authored predicates and fictional host facts. The guided scenario interface is distinct from public service commands. No canonical contract, shared ESS model or production adapter behavior was changed.

[Delivery verification](../../../docs/website-verification.md) records reproducible commands and practical limits. The production build and TypeScript checks pass, reference generation has no drift, 10 example tests pass, browser checks pass, example Clippy/MSRV checks pass, and the full repository gate passes with 65 workspace tests. The gate synthesizes 315 conformance scenarios, including 34 authored inputs; that count is not a claim of production binding execution. The public-output audit checks 311 emitted files including WASM and finds no private path markers. Existing AEP validation warnings concern 73 historical reviews without structured findings blocks; 131 artifacts remain valid.

Public deployment, distributed persistence, cryptographic verification, retention cleanup, audit persistence and provider I/O remain outside this delivery. Generated outputs and local logs are ignored. Work remains local and uncommitted. The specification artifact remains in its initial draft lifecycle for subsequent review; that lifecycle is not a claim that the delivered website is still in progress.

No decomposition was made under this specification, so no multi-artifact planning critic panel was needed. This was an interactive operator session with one checkout writer.


## Practical walkthrough revision — 2026-09-09

The operator approved replacing the introductory labs with two guided GitLab request walkthroughs: configured federation available today and separately labeled specified per-user OAuth/delegation. Show laptop, remote gateway/adapter services and GitLab, credential locality, request/response travel and three failures per walkthrough. Keep the existing labs under advanced Contracts documentation. Add play/pause, next/back/restart, actual descriptor-shaped issue fixtures and readable minimum typography (16px explanation, 14px controls/meaningful labels, 12px secondary metadata). Reuse Rust/WASM for deterministic walkthrough state and React for presentation. No live provider calls, new production contracts, OAuth endpoints or guessed provider profiles. Keep the local dev server running. This approved revision is implemented. The verification above records the preceding delivery; the updated verification document records this revision separately.


### Revision completion and evidence

The introductory route now contains two practical GitLab journeys with moving request/response arrows, separate credential ownership, recognizable issue results, six failure variants and play/pause/next/back/restart controls. The user-authorization journey explicitly waits for a separate Read issues action after connection setup. Existing labs moved to /contracts/examples. Body text, controls and metadata meet the selected minimum sizes; diagrams adapt to mobile and reduced motion.

The checked-in fixture is validated against the real GitLab descriptor, including complete provenance. Canonical contracts, shared ESS and production adapters are unchanged. The illustration executes no live request or cryptographic/authentication flow; unsupported provider-specific OAuth details are not invented.

Verification: production build, TypeScript, public-output audit (314 files), reference drift (40 contract / 88 total reference pages), 15 Rust example tests, browser smoke covering both journeys and six failures, standalone Clippy/MSRV, and the full repository gate (66 workspace tests) pass. Detailed commands, test scope and limitations are in docs/website-verification.md; local logs are in .local/tmp/docs-walkthrough-20260909/. The development server remains running on 127.0.0.1:3100. The work remains local and uncommitted.

## b10x UI/UX overhaul — 2026-09-09

The operator approved the main ../website style, unified browsing and local full-text search. Reuse @beyond10x/docs-system pinned to 1ef1890272ce308eb6fec5446091f93afd14046f and Pagefind 1.5.2. Overhaul the homepage, authored guides, generated reference, adapter catalog and interactive examples together, retaining Home / Introduction, Contracts and Adapters and all existing routes/anchors. Share navigation and page context; organize references by family; add /search with section/type/owner filters, excerpts and URL state. Index only rendered public content, audit before/after indexing, and make refresh use isolated Docusaurus build directories so the port 3100 preview remains available. Preserve example state/semantics and canonical inputs. Use accessible light/dark tokens, 16px prose, 14px meaningful labels and controls, 12px secondary metadata, keyboard focus and 44px controls. Verify browser layouts, existing example scenarios, search, links, generated drift, types/build and public-output audit. One checkout writer; local only; no publication, Atlas integration or neighboring repository edits. No decomposition is introduced. Implementation is in progress; previous completion records describe preceding revisions.

### b10x overhaul completion and verification

The approved UI/UX overhaul is implemented locally. The site uses the main b10x Website's exact docs-system pin and Pagefind 1.5.2, with a navy/green homepage, shared navigation and page context across authored/generated pages, family-grouped references, a filterable 12-adapter catalog, readable light/dark layouts and restyled existing examples. Existing clean URLs and canonical text/anchors remain; reference source h1 headings render as h2 with a regression test. Search indexes 109 public pages, supports section/type/owner filters and URL state, and resolves clean links in dev and production. Rust audits readable HTML before indexing and all 452 final output files afterward. Isolated search refresh works alongside the hot-reloading preview.

Verification is recorded in docs/website-verification.md: TypeScript, production build/links/anchors/public audit, reference drift, isolated search refresh, 15 Rust example tests, the browser scenario suite in dev and production, the new navigation/search/responsive/theme/keyboard/zoom suite, and the full repository gate with 67 workspace tests pass. Four non-fatal CSS minifier optimization warnings remain; rendered font sizes and both themes were checked in the browser. No canonical contract, ESS or production adapter source changed. The temporary production test server was stopped; development remains on 127.0.0.1:3100. Work is local and uncommitted. No decomposition was introduced, so a multi-artifact critic panel was not required. The artifact remains draft for its existing review workflow; this completion records the delivered revision, not a lifecycle move.

## Lifecycle viewport correction — 2026-09-09

Corrected the operator-reported oversized lifecycle on the local Connectors model reference. The shared canvas forced a 107px diagram to at least 768px, clipping its start and end. Reference diagrams now use Docusaurus Mermaid with natural-size limits and responsive width/height fitting, retaining shared surface tokens and expandable source. Canonical inputs and example behavior are unchanged.

TypeScript, production build (109 indexed pages / 452 audited files), the UI suite with new one-state/branching lifecycle geometry checks across four widths and both themes, disclosure/source/live-resize checks, and diff whitespace checks pass. See docs/website-verification.md and .local/tmp/docs-lifecycle-viewport-20260909/. Port 3100 remains running with the fix; all work is local and uncommitted. No decomposition or lifecycle move is introduced.
