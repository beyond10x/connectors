# Connectors documentation website

A local Docusaurus site with the shared b10x visual system, authored guides,
selected canonical contract views, ESS model reference, local full-text search
and bounded Rust/WASM examples.

## Start the preview

Use Node 22 or later, Rust 1.88 or later, and the repository's exact ESS pin. Follow
[toolchain setup](../docs/gitlab-generation.md#generate-and-check) if the pin is not
already available. The examples also need Rust's browser target:

```sh
rustup target add wasm32-unknown-unknown
cd website
npm ci --allow-git=root
npm start
```

Run the target-install command only when the target is missing. `npm start` generates
reference data, validates the example composition, builds its WASM and prepares search before
starting Docusaurus at **http://127.0.0.1:3100/**. It binds loopback and opens no
browser automatically. No production deployment is configured.

The example build uses Cargo's offline mode. On a fresh machine, after the first
reference/example generation has created the declared path dependencies, populate
missing registry dependencies explicitly with:

```sh
cargo fetch --locked --manifest-path examples/realization/Cargo.toml
npm start
```

Ordinary website edits hot reload. After changing a contract, ESS model or Rust
example, run `npm run reference` or `npm run examples` as appropriate. Refresh an
open example to load rebuilt WASM. Reset restarts the current in-memory exercise.

Search reflects the latest completed build. After content changes, refresh it
without stopping the development server:

```sh
npm run search:refresh
```

For changed contract/model sources, run `npm run reference` first. The refresh uses
isolated build directories under `.cache/`; ordinary `npm run build` also isolates
its Docusaurus generation so it can run alongside the preview. Both audit public
HTML, run Pagefind, audit the complete output and update the development search
cache. A fresh `npm start` prepares this index automatically.

The Git install flag permits only dependencies declared by this package. The
shared `@beyond10x/docs-system` dependency uses the exact commit pinned by the main
b10x website; it is distributed through Git rather than npm. This repository does
not link to or mutate the neighboring checkout.

## Build and verify

```sh
npm run typecheck
npm run build
npm run reference:check
npm run test:examples
npm run test:browser
npm run test:ui
```

`test:browser` expects the preview to be running; `CONNECTORS_WEBSITE_URL` selects
another local server. Install the matching Playwright Chromium with
`npx playwright install chromium` if it is unavailable. The production build
checks routes and anchors, then audits all emitted files, including WASM, for local
and private path markers. Browser tests exercise both practical walkthroughs,
their six failure variants, deliberate invocation after connection setup, playback,
navigation, generated references, a lifecycle diagram, all three advanced exercises,
replay refusal, mobile layout and reduced motion.
The UI checks cover shared navigation and active state, adapter filtering, search
results/filters/deep links and missing indexes, mobile menus, responsive layouts,
light/dark themes, keyboard focus and zoom. They also check that one-state and
branching lifecycles fit their initial viewport without enlarging small diagrams
or requiring nested scrolling.

The Rust repository gate remains a separate check; the website does not add Node
to ordinary Cargo builds. See [development](../docs/development.md).

## Edit the owning source

- `docs/` contains authored explanations and adapter guides. MDX may embed reviewed
  presentation components.
- `publication.json` explicitly selects canonical shared/native contract sources
  and model roots. The Rust `connectors-build docs` command validates ownership,
  renders imported Markdown safely and invokes pinned ESS for `ess-docs/1`.
- `plugins/reference.ts` creates routes from that output; `src/components/Reference.tsx`
  renders ESS blocks without reconstructing semantics.
- `src/components/DocumentationFrame.tsx` and the authored-doc theme wrappers share
  navigation and page context. `documentation.ts` holds curated adapter summaries
  and navigation presentation; it does not define runtime capabilities.
- `plugins/search.ts` indexes only built public HTML through pinned Pagefind.
  `src/pages/search.tsx` supplies search presentation and route normalization.
- `src/css/` separates shell tokens, documentation layout, homepage and exercises.
  Import shared tokens before local mappings; do not introduce another theme palette.
- `examples/components.yaml` selects canonical mutation commands for an example
  component. The Rust realization uses generated ports and typestate transitions.
  Discovery/readiness exercises evaluate generated value types against authored,
  cited predicates and fictional host facts.
- `examples/realization/src/walkthrough.rs` supplies deterministic illustrated
  request journeys independently of the advanced labs. `walkthrough-fixtures.json`
  contains the issue input/result; the Rust build tool validates these against the
  checked-in GitLab descriptor before building WASM. It does not import provider
  implementation into the example engine.
- `.cache/`, `static/examples/`, `.docusaurus/` and `build/` are generated. Do not
  edit or commit them. Canonical contract and ESS inputs stay in their existing
  owners; the example model is assembled in disposable storage.

The three examples use a fixed namespace/principal, one mutation key, deterministic
injected time bounded to the first fictional hour, fictional observations and
in-memory behavior. They demonstrate
selected rules. Audit persistence, distributed atomicity, real cryptographic
verification, retention cleanup and provider I/O are not implemented by the lab.
It is not a production binding or a general-purpose model interpreter.

The introductory route `/introduction/examples` follows one issue read through a
laptop, federation gateway, GitLab adapter and provider. Its second walkthrough
illustrates specified user authorization, with a deliberate pause between connection
setup and the read. `/contracts/examples` retains the original advanced exercises.
The walkthroughs perform no authentication, OAuth, cryptographic verification or
provider requests. Credential symbols explain ownership; they carry no token values.

The [website design](../docs/website-design.md) records audience, ownership,
publication decisions, implementation boundaries and future ideas.
