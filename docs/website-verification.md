# Local website verification — 2026-09-09

This checks the documentation and website implementation based on local commit
`249276b287e31f8570a58255c35691d7bcfdeb4e`. The changes were uncommitted when checked.
The [website design](website-design.md) owns scope; [website setup](../website/README.md)
documents how to reproduce the checks. No external publication was performed.

## Delivered surface

The website combines authored introductions and adapter guides with 40 selected
canonical contract pages and 48 ESS model pages from six explicit model roots.
Navigation is Home / Introduction, Contracts and Adapters. Source ownership,
digests and implementation status remain visible. Historical evidence sections
are excluded from the public contract view; unpublished source links become notes.

Three guided Rust/WASM exercises demonstrate discovery versus access, global
readiness versus operation eligibility, and uncertain mutation outcomes. Six
canonical mutation commands use generated ports and typestate transitions.
Discovery/readiness predicates use generated value types and fictional host facts.
The scenario interface is a teaching interface, not another product contract.

VISION.md, README.md and AGENTS.md now have distinct audiences. Detailed build and
service instructions live in focused development and operating guides.

## Observed checks

| Command or inspection | Result |
|---|---|
| `npm run typecheck` in `website/` | Passed |
| `npm run build` in `website/` | Passed; reference generation, validated example composition, WASM compilation, strict links/anchors and public-output audit |
| `npm run reference:check` | 40 contract pages, 88 total reference pages; no drift |
| `npm run test:examples` | 10 Rust tests passed, including invalid requests, admission, freshness, changed target, replay, conflict and uncertain effects |
| `npm run test:browser` against the live preview | Passed navigation, canonical text/anchors, ESS lifecycle SVG, three guided paths, replay/refusal, keyboard, themes, mobile layout and unavailable-module behavior |
| Example workspace Clippy, all targets, `-D warnings` | Passed |
| Example workspace `cargo +1.88.0 check --locked --offline --all-targets` | Passed |
| `cargo run --locked --offline -p connectors-build -- gate --msrv` | All checks passed: 65 workspace tests, formatting, Clippy, descriptor/model boundaries, Rust 1.88 checks, conformance synthesis and AEP validation |
| ESS conformance synthesis within the gate | 315 scenarios, including 34 authored inputs; zero synthesis refusals |
| Public-output byte audit | 311 files checked, including WASM; no private path markers |
| Entrypoint, guide and new planning-record local links | 64 links checked; no missing files |
| Desktop and mobile screenshots | Inspected; readable layout, no horizontal overflow or compiler overlay |
| `git diff --check` | Passed |

The reference tooling's seven tests cover owner/traversal boundaries, binary path
disclosure, unsafe Markdown, normative text preservation, stable anchors, selected
link resolution and unknown ESS projection variants. They are included in the 65
workspace tests. The root gate does not silently substitute for the separate
website and example-workspace checks.

The gate's AEP validation found 131 valid artifacts and the same 73 existing
warnings about historical review records without structured findings blocks.
This task did not rewrite those review records.

## Practical limits

The preview binds `127.0.0.1:3100`. It needs the local process to remain running;
hosting and public release distribution are separate work.

These examples use bounded in-memory state, a fixed namespace/principal and
request choices, and injected time within one fictional hour. They do not implement
audit persistence, distributed atomicity, cryptographic verification, retention
cleanup or provider I/O. Their tests establish selected example behavior, not
production runtime conformance. The conformance synthesis count above likewise
does not mean every generated scenario was executed against a production binding.

Source selection, safe rendering, portable route aliases, compiler path remapping
and the byte audit enforce the declared publication boundary. A path-marker audit
is not a general secret detector. Future published sources still require review.

Local command logs and screenshots are retained under
`.local/tmp/docs-website-20260909/`; they are excluded from publication and Git.

## Practical walkthrough revision — 2026-09-09

The operator requested a recognizable federation/authentication story in place of
the introductory rule exercises. `/introduction/examples` now follows a fictional
GitLab issue request from a laptop through a remote gateway and adapter to GitLab,
with the result traveling back. The two independently labeled journeys show
configured access available today and specified user authorization. The latter
pauses after connection setup and requires a separate **Read issues** action.

Each has three failure variants. Provider credentials stay at the adapter boundary;
application identity, service credentials, delegation and provider consent are
distinct. The identity service is depicted outside the laptop. A Rust walkthrough
session drives the illustration; it is not a service implementation, OAuth flow or
cryptographic verifier. The checked-in issue input/result match the actual GitLab
descriptor; an unsupported state filter or incomplete provenance is refused by
the fixture validation test. The three original labs remain under
`/contracts/examples` as advanced exercises.

The revision increases smaller custom text to at least 12px for metadata, 14px for
controls and meaningful diagram labels, and 16px for explanations. Diagrams and
explanations share a desktop view; mobile layouts stack them. Playback can be
paused and the diagram respects reduced motion.

Rechecked results:

| Check | Result |
|---|---|
| Production build and TypeScript | Passed; strict public links/anchors |
| Public-output audit | 314 files checked, including the rebuilt WASM; no private path markers |
| Reference drift check | 40 contract pages, 88 total reference pages; no drift |
| Rust example tests | 15 passed: the original 10 plus successful journeys, all six failures, credential locality, explicit post-setup invocation and deterministic navigation/input refusal |
| Browser smoke | Both journeys, all six failures, returned issue table, setup pause, play/pause/back/restart, original labs, references, keyboard, themes, reduced motion, mobile layout and missing-module behavior passed |
| Example Clippy and Rust 1.88 checks | Passed |
| Full repository gate with MSRV | Passed; 66 workspace tests, including descriptor-fixture validation, and unchanged 315-scenario conformance synthesis |
| Desktop/mobile screenshots | Inspected for readable deployment boundaries, request arrows and text |

Canonical contracts, shared ESS and production adapters are unchanged. The work
is local and uncommitted, and the existing preview process remains running.
Revision logs/screenshots are under `.local/tmp/docs-walkthrough-20260909/`.

## b10x UI/UX overhaul — 2026-09-09

The operator selected the main b10x Website style and unified browsing plus local
full-text search. This revision reuses `@beyond10x/docs-system` at the main site's
exact commit `1ef1890272ce308eb6fec5446091f93afd14046f`, with Pagefind 1.5.2. The
homepage, authored guides, generated references, adapter catalog and examples now
share the b10x typography, semantic colors and navigation conventions. The existing
URLs, canonical text and anchors remain. Canonical source h1 headings render as h2
beneath the reference page title; a Rust regression test checks text/id preservation.

The shared documentation frame supplies navigation, active state, breadcrumbs,
mobile browsing and contents controls. The adapter catalog shows all 12 documented
owners, with three working first slices distinguished from broader specification
coverage. Search covers 109 public pages with section, document-type and owner
filters, excerpts, pagination and URL state. Result URLs work in development and
production. The Rust audit runs before Pagefind compresses content and after the
complete search bundle is written. Search refresh uses separate build directories
and updates a development-only cache without interrupting hot reload.

| Verification | Result |
|---|---|
| `npm run typecheck` | Pass |
| `npm run build` | Pass; strict links/anchors; 109 indexed pages; 452 emitted files audited |
| `npm run reference:check` | Pass; 40 canonical contracts and 88 total reference pages, no drift |
| `npm run search:refresh` with dev server running | Pass; isolated output and generation; preview stays available |
| `npm run test:examples` | Pass; 15 Rust example tests |
| `npm run test:browser` on the dev server | Pass; both walkthroughs, six failures, advanced labs and unavailable WASM |
| `npm run test:ui` on the dev server | Pass; reference family/active navigation, adapter filters, search filters/results/URL restoration/pagination/clean links, mobile menus, themes, zoom, keyboard focus and unavailable search |
| `CONNECTORS_WEBSITE_URL=http://127.0.0.1:3101 npm run test:browser` | Pass against the built production artifact |
| Production search check | Filtered GitLab results open the clean `/adapters/gitlab` route |
| `connectors-build gate --msrv` | Pass; 67 workspace tests, formatting, Clippy, MSRV, generation, boundaries and planning |
| Targeted canonical-source status | No changes in `contracts/`, `ess/`, `adapters/` or `spec-kinds/` |
| `git diff --check` | Pass |

Responsive checks cover 1440, 1024, 768 and 390px widths in both themes, plus zoom
and mobile keyboard navigation. Representative homepage, adapter, contract,
walkthrough and search screenshots were inspected. Visible text on those pages
has a 12px minimum; body text is at least 16px. The themes now keep Docusaurus and
shared-component canvas colors identical. The final build emits four non-fatal
CSS minifier “Missing font size” optimization warnings; browser inspection verified
the resulting font sizes. This is not a claim of an exhaustive accessibility audit.

Local logs and screenshots are under `.local/tmp/docs-overhaul-20260909/`.
`build-final.log`, `gate.log`, `ui-tests.log`, `browser-smoke.log`,
`production-browser.log`, `example-tests.log`, `reference-check.log` and
`search-refresh.log` record the checks. The temporary production-test server was
stopped; the development server remains at http://127.0.0.1:3100/ with its log in
that directory. Search reflects the latest completed build; authored edits still
hot reload immediately. No external publication, runtime expansion or commit was
performed. Previous delivery records above retain their historical counts.

## Lifecycle viewport correction — 2026-09-09

The shared panning canvas imposed a 48rem minimum width on every SVG. A 107px
single-state lifecycle therefore became 768px wide and 1322px tall, with its start
and end outside the visible frame. ESS reference diagrams now use Docusaurus
Mermaid directly, preserving its intrinsic size limit while fitting larger graphs
to the column and at most 60vh / 32rem in height. The existing theme tokens and
expandable source remain. No semantic or generated input changed.

`npm run typecheck`, `npm run build` (109 indexed pages and 452 audited public
files), `npm run test:ui` and `git diff --check` pass. The UI suite now checks
single-state and branching lifecycles at 1440, 1024, 768 and 390px in both themes:
visible bounds, no enlargement, bounded height and no nested scrolling. A focused
browser check also verifies collapse/reopen, source access and live resize.
Screenshots at 820px and 390px confirm the reported lifecycle fits at natural size.

Evidence is retained in `.local/tmp/docs-lifecycle-viewport-20260909/`. The
development server remains on port 3100 with the correction loaded. Work remains
local and uncommitted; unrelated runtime and example behavior tests were not rerun.
