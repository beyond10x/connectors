# Public documentation and executable contract examples

Status: implemented locally on 2026-09-09. Public hosting,
release publication and Atlas integration remain deferred. The operator requested
a development server while the site is built. See the
[delivery verification](website-verification.md) for the implemented surface,
observed checks and remaining limits.

## Audience and navigation

Integration builders are the primary readers: people consuming adapters, writing
one, or combining several. Explain purpose and practical use before introducing
model internals. Operators can follow configuration and troubleshooting links;
contract designers can inspect exact semantics and models from those explanations.

Top-level navigation is **Home / Introduction | Contracts | Adapters**. Guided
examples belong alongside the concepts they explain rather than becoming a
separate product. All documented adapters appear, with runtime status stated
separately from specification coverage.

The September 9 UI/UX overhaul follows the organization-wide `beyond10x/website`
shell: a navy homepage hero, green actions, blue/cyan accents, sans-serif headings
and monospace metadata. Light mode is the default; system preference and an
explicit theme choice are respected. `@beyond10x/docs-system` is pinned to commit
`1ef1890272ce308eb6fec5446091f93afd14046f`, matching that website's dependency.
Its tokens load before the local shell mappings, with shared cards, headers,
callouts, search controls and code examples reused directly. ESS reference diagrams
use Docusaurus Mermaid with the same surface tokens: preserve the natural size of
small diagrams and fit larger diagrams to the column and available screen height.
Do not apply the shared panning canvas's minimum width to these reference diagrams.

Authored and generated pages share a documentation frame, a navigation tree,
breadcrumbs and contents navigation. References are grouped by family; the adapter
catalog supports provider search and runtime-availability filters. Native guide
status describes the adapter runtime separately from a reference's specification
status. Public URLs and source heading anchors remain stable. Canonical h1 titles
render as h2 under the reference page's primary heading, retaining their text and ids.

### Local full-text search

`/search` uses Pagefind 1.5.2 over built public HTML, with section, document-type
and owner filters, excerpts and URL-preserved query state. The index includes the
curated guides and selected contract/model references; it never reads arbitrary
repository Markdown. Navigation, provenance machinery and interactive controls
are excluded. Result links normalize generated `.html` filenames to the site's
existing clean routes. Query headings are weighted, and results load in batches.
Search code and index fragments load only on the search page.

The Docusaurus search plugin invokes the pinned Pagefind CLI after the Rust public
audit checks readable output; a second audit checks the complete indexed artifact.
The normal build uses an isolated generated directory, and `npm run search:refresh`
uses separate generation/output directories under `.cache/`. Both update a
development-only static search cache. Old hashed fragments remain available for
tabs using an earlier index; the new entry manifest is copied last. This cache is
not an input to production builds. Search reflects the latest completed build;
authored page changes still hot reload immediately. A missing index shows an
explicit unavailable state and links back to browsing.

| Section | Initial content | Public route |
|---|---|---|
| Home | Purpose, architecture, reading paths and current milestone | `/` |
| Introduction | Concepts, getting started, composition, status and guided examples | `/introduction` |
| Contracts | Family explanations, canonical semantics, model reference and examples | `/contracts` |
| Adapters | Overview, practical guides for GitLab/Kubernetes/SQL, documented native profiles for other owners | `/adapters` |

The [vision](../VISION.md) owns direction. The [README](../README.md) is the human
repository entrypoint. [AGENTS.md](../AGENTS.md) gives working instructions. The
website is a curated reader experience; repository history and agent workflow do
not become its public navigation.

## Content ownership and rendering

Docusaurus 3 with TypeScript/React supplies the static site and presentation. The
initial local stack pins Docusaurus 3.10.2, uses Node 22 or later and an npm lockfile.
Use authored Markdown/MDX for introductions and guides. A local Docusaurus plugin
loads generated reference data and creates public routes using its supported
[content and route APIs](https://docusaurus.io/docs/advanced/plugins).

Keep authored sources under `website/docs/`, presentation under `website/src/`, and
an explicit [publication inventory](../website/publication.json) beside the site configuration. Generated data
and build output are disposable. Do not hand-edit a generated page or establish a
second editable definition of contract semantics.

The public reference combines three inputs:

1. Selected canonical Markdown from root `contracts/` and adapter-owned native
   contracts. Render it as Markdown data, not executable MDX. Preserve normative
   wording, headings and source provenance.
2. The pinned ESS `ess-docs/1` projection for typed declarations, relationships,
   lifecycle diagrams and command reference. ESS owns this format; consume its
   documented blocks and reject unknown variants rather than silently omitting them.
3. Reviewed publication metadata: stable route, title, source, owner, associated
   model page/declaration ids and support status. Metadata selects presentation; it
   cannot override a contract or infer that a described capability is implemented.

Use the existing Rust build-tool boundary for source selection, ESS invocation,
provenance and reference checks. TypeScript owns Docusaurus integration and UI.
Generate only through the pinned ESS command; do not write a parallel ESS parser
or schema projector. The Rust generator renders canonical Markdown with
`pulldown-cmark`; React presents the resulting safe HTML and ESS blocks.

Shared protocols stay in this repository. Native descriptions and fixtures stay
under their adapter owner. Website assembly references them, so moving an adapter
can later replace a local source with a pinned documentation artifact without
moving its native semantic authority into the website. External artifact intake is
not required by this initial local site.

### Publication boundary

The inventory is an allowlist, with stable route identities independent of file
layout. Reject duplicate routes, missing files/declarations, traversal outside the
repository and symlinks resolving outside an allowed owner. Build from selected
inputs, never a repository-wide Markdown glob.

Resolve local links to selected public pages and anchors. A link into historical or
private evidence gets an explicit non-linked source note in the public view. The
historical **Old evidence and disposition** section is omitted; normative text
outside that evidence section is retained. A missing public reference is a build error. Never
publish a guessed remote repository URL while no public source repository exists.
Expose source paths relative to the repository, a content digest and the model
version/digest where applicable. Raw-source downloads are a future option, subject
to the same explicit selection; there is no repository file endpoint.

Planning stores, AGENTS.md, `.local/`, historical reviews, evidence archives,
absolute workstation paths, private configuration and credentials do not enter
site content or its generated data. A Rust audit checks every production output
file, including WASM, for private path markers. Compiler source paths are remapped
and Docusaurus route modules use portable aliases. Some existing contract documents cite internal evidence; do not
copy their surrounding repository automatically to satisfy those links.

Render code as text, disable executable HTML in imported Markdown, and allow only
safe link schemes. Authored MDX belongs to the reviewed website source. Generated
Mermaid diagrams use strict rendering. Load contract/model detail by route so a
reader opening the home page does not download the entire model.

### Status and versions

Display distinct facts: specification status/version, example implementation and
runtime support. A broader adapter specification never implies that all its
operations are supported by the first-slice runtime. The v0.1.0 milestone is a
reviewed specification baseline, not a new implementation of all its profiles.

Start with a single current documentation set. Display exact semantic family and
wire versions on references; do not introduce Docusaurus version snapshots for
unreleased drafts. Public versioned site releases are a later distribution task.

## ESS feasibility and demo execution

The following was observed using the repository's pinned ESS **0.20.0** against
its shared model at the v0.1.0 baseline:

| Probe | Observation |
|---|---|
| `ess generate --path ess --kind docs-ir --out <temporary-directory>` | `ess-docs/1`, 21 pages; suitable input for a custom presentation layer |
| `ess generate synthesize --path ess --target web --out <temporary-directory>` | Browser catalog and Rust/WASM bridge source generated |
| Browser catalog | 20 entities, 53 commands, zero dispatchable commands |
| Language-neutral synthesis plan | 342 generated capabilities, 59 obligations, zero plan refusals |
| Browser target report | Explicit target refusals, including commands with no accepting component; these are separate from plan refusals |

Generation alone did not build a WASM module or execute a command. The shared
model has no component acceptors. A useful browser example therefore needs an
explicit example composition and behavior implementation.

Create an isolated example composition from unchanged canonical domain sources,
adding only the example's component acceptors and observable views as needed.
Validate that composition before generating both Rust and web targets. Their
relative layout must satisfy the web emitter's references to the Rust target.
Keep the example's component selection and any example-only views outside the
normative shared root. Do not rename public commands, simplify canonical guards or
invent persistent product entities for UI state.

The implemented composition accepts six canonical mutation commands. Its Rust
realization uses generated ports and typestate transitions. Discovery and readiness
exercises use generated value types with authored, cited predicates and injected
host facts; they do not implement all canonical discovery or authentication commands.

The generated browser bridge is linked into the WASM module. The guided UI calls a
separate bounded scenario interface in that same module, whose coordinator invokes
the generated mutation ports. Native and browser tests execute the same Rust
behavior. Scenario actions are teaching controls, not new public service commands.
TypeScript renders returned states/outcomes; it does not maintain a second
implementation of the contract. Reset creates a fresh example instance. Malformed
requests, unknown actions and module-load failures are explicit results.

Where a contract rule is only textual, cite it beside the implementing example
predicate and cover it independently with scenarios. Do not turn trusted host facts
into caller authority: UI switches inject fictional environment facts into the
example, not fields an actual public invocation accepts. Use small fictional
values compatible with the browser target's numeric limits. Show target limitations
in the example's technical notes.

Every demo is labeled **executable example, in-memory behavior**. It demonstrates
selected rules; it does not prove durable storage, audit persistence, retention
cleanup, distributed atomicity, real cryptographic verification or live-provider enforcement. No provider credentials,
external API requests or persistent user input are involved.

## Guided experiences

The introductory experience at `/introduction/examples` is **Follow a request to
GitLab**. It replaces the rule-focused labs as the first example. A fictional
`issues.list` request selects `acme/website` and a limit of 20; input and returned
page fixtures must match the checked-in GitLab descriptor. There is no invented
state filter. The return path ends in a three-issue table with source attribution.

The diagram groups laptop, remote gateway/adapter services and GitLab into explicit
deployment areas. The identity service is a separate boundary, never drawn inside
the laptop. Highlight request/response travel and explain the current check beside
the diagram. Credential symbols identify separate client-to-gateway,
gateway-to-adapter and adapter-to-provider access; they contain no token values.

Two walkthroughs have separate support labels:

- **Configured access:** implemented federation behavior, illustrated with fictional
  requests. Authenticated describe and invoke, revision and route selection,
  downstream admission, project restrictions, provider authentication and the
  attributed result. Variants refuse gateway access, an unreachable adapter or a
  rejected provider credential without retrying or fabricating an empty issue page.
- **User authorization:** specified architecture, runtime pending. Separate
  application identity from trusted browser consent, protected callback, provider
  exchange, custody and safe connection publication. Pause after connection setup;
  a distinct **Read issues** action begins the delegated business request. Show
  independent receiver verification and current connection policy/readiness.
  Variants cover declined consent, denied connection access and unavailable custody.
  Provider-specific OAuth endpoints/scopes are not invented where no profile exists.

The Rust/WASM walkthrough engine provides deterministic step and outcome state;
React handles playback, diagrams and presentation. Play/pause, next/back and restart
operate on that state. No live services or auth verifier are involved. The same
module also exposes the advanced labs through a separate session interface.

Body explanations are at least 16px, meaningful diagram labels and controls at least
14px, and secondary metadata at least 12px. Keep reduced motion, keyboard access,
mobile layouts and explicit module failures. The examples use the shared b10x
visual system and the shell's navbar height for sticky playback controls.

### Advanced contract exercises

The original three exercises remain at `/contracts/examples`, linked from contract
reference pages and the introductory walkthrough. They are for readers examining
particular semantic rules, not the introductory explanation of Connectors.

Each example has a short purpose statement, a small set of commands or environment
controls, a state/relationship view, a command/result timeline and links to exact
contract rules. Provide step and reset controls, readable outcomes, keyboard access
and reduced-motion behavior. The surrounding explanation remains readable if
JavaScript or WASM is unavailable; it must not display fabricated execution output.

### 1. Discovery does not grant access

Use a fictional discovery observation and an SQL target to connect discovery,
connection publication and request eligibility. Begin with an observed endpoint and
no admitted SQL connection. Show that an attempted read remains ineligible. Admit
selection, configure a matching binding and fresh evidence, then show the permitted
example read. Provider output is an explicitly injected fixture.

Include stale/withdrawn observation behavior and a changed destination. Distinguish
eligibility for a new binding from the semantics of an already retained fixed
binding; withdrawal alone must not silently retarget or erase its historical
coordinates. Follow the [discovery](../contracts/discovery/resources/v1alpha1/semantics.md),
[connection](../contracts/auth/connection/v1alpha1/semantics.md) and
[composition](../contracts/discovery/composition.md) owners.

### 2. Ready for which operation?

Show global connection viability next to operation eligibility. Change enabled or
revoked state, custody availability, credential validity, baseline freshness,
required scopes and exact-target permission evidence. A generally ready connection
can still have an ineligible operation. Expired or unknown evidence must not become
an authorization success. Use the connection owner's reduction and the
[evidence contract](../contracts/auth/evidence/v1alpha1/semantics.md).

### 3. The response disappeared

Follow one mutation through approval, durable-attempt preparation, dispatch and a
lost response. Display the distinction between not dispatched, known outcome and
unknown effect. A second request cannot resend an uncertain attempt. Include a
conflicting idempotency input and known-result replay under current admission.
Approval and storage are explicit in-memory test bindings; there is no claim that
browser memory supplies durable redemption. Follow the
[mutation](../contracts/operations/v1alpha1/semantics.md),
[idempotency](../contracts/operations/v1alpha1/semantics.md#51-key-namespace-fingerprint-and-replay-admission) and
[audit](../contracts/service/audit.md) owners.

Later candidates are rotating-credential races, direct versus federated invocation,
contract revision comparisons and an adapter-to-contract dependency explorer.
Keep these ideas out of the initial completion claim.

## Delivery order and verification

1. Finish human/agent entrypoints, focused guides and the governed design record.
2. Bring up the local Docusaurus shell and share its address; keep hot reload running.
3. Add curated introduction/adapter pages and generated contract/model references.
4. Prove one complete example composition, generated interface, Rust realization
   and browser interaction before extending the engine to the remaining examples.
5. Complete publication checks, repeatable builds and example/browser verification.

Check authored links, headings and current support claims. Verify reference
generation is deterministic, route/source identities resolve and changing an
unpublished file cannot change public output. Test malformed input, unsafe links,
missing sources and unknown ESS rendering blocks. Verify normative text is
preserved through rendering and exclusions do not silently remove rules.

Run website type checking and a production build with broken-link failures enabled.
Use browser tests over the actual demo bridge for successful/refused commands,
reset, stale evidence, changed target, lost response, replay and conflicting input.
Check mobile navigation, light/dark themes, keyboard operation and visible module
errors. The repository's existing semantic and dependency boundaries continue to
apply; executable tooling changes receive their relevant Rust checks and full gate.

Completion is a locally reproducible site with the three navigation sections,
curated adapter coverage, traceable reference views and the explicitly implemented
example set. Record implemented and deferred portions truthfully in the planning
owner. Keep the local preview running for the operator. No deployment, DNS,
registry publication, remote repository creation or Atlas delivery is part of this
work.
