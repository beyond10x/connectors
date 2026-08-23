# web — connector explorer source

The [VitePress](https://vitepress.dev) explorer migrated from the predecessor repository.
Publishing automation has not yet been restored; the repair is tracked in
[S-018](../docs/stories/S-018-the-explorer-works-against-the-new-site-json.md).

**The Node toolchain is contained here.** `package.json`, `package-lock.json` and `node_modules` all
live under `web/`; nothing about the Rust workspace at the component root knows or cares that this
directory exists.

## Build

Requires Node 22+.

```bash
cd web
npm ci          # or `npm install` on first setup, to create the lockfile
npm run build   # static site into web/.vitepress/dist
```

Other scripts:

```bash
npm run dev      # local dev server with hot reload
npm run preview  # serve the built output, base path included
npm test         # the explorer's contract with the catalogue — run after `npm run build`
```

`npm test` is Node's built-in runner over `test/*.test.mjs`; it adds no dependency. It reads
`public/catalog.json` and the built HTML in `.vitepress/dist`, so it must follow a build.

S-018 and S-020 own the future CI web job. Until both land, these commands are local diagnostics and
the component does not claim that a broken site is mechanically gated.

## Layout

| Path | What it is |
|---|---|
| `.vitepress/config.mts` | Site config — title, nav, sidebar, and the Pages **base path**. |
| `.vitepress/theme/` | The default theme plus the explorer's Vue components, registered globally. `index.mts` is the one file that knows VitePress; [`theme/components/README.md`](.vitepress/theme/components/README.md) records the three component tiers. |
| `index.md` | Landing page: what the project is, and what does not work yet. |
| `explorer.md` | The provider & operation explorer. |
| `operations/[operation].md` | One pre-rendered page per operation, enumerated from the catalogue. |
| `data/` | The catalogue's types, the questions the explorer asks of it, and the build-time loader. |
| `public/` | Served verbatim at the site root. Holds generated catalogue/specs and the Pages CNAME. |
| `test/` | The explorer's contract with the catalogue, over the built site. |

## Public content boundary

This site is for connector consumers. It explains available services and operations, their call
contracts, safety metadata, credentials, hosts, and current availability. Internal designs, roadmap
and story mechanics, crate architecture, and agent instructions belong in the component docs and
must not be linked or reproduced on the public pages.

The migrated theme references `public/brand/{icon,mark}.svg`, but neither those public copies nor a
canonical `assets/brand/` source exists yet. S-018 must either add and test one declared source or
remove the references; no byte-identity check is claimed today.

## Two things to keep right

**The base path.** `.vitepress/config.mts` sets `base: '/flux-connectors/'`, because that is where
GitHub actually serves the site.

`public/CNAME` names `flux.codewandler.org`, and the base was once set to `'/'` on the strength of
it. That shipped an unstyled site: a committed CNAME is a *request* for a custom domain, not evidence
one is serving, and GitHub never accepted it — the Pages API still reports `"cname": null`, so every
bundled asset resolved a level too high and 404'd.

Flip the base to `'/'` **only** once `gh api repos/beyond10x/connectors/pages --jq .cname`
reports the domain — not when the CNAME file lands, which is what went wrong.
`test/explorer.test.mjs` asserts the built HTML's own asset URLs sit under the deployed base, and was
verified to fail when the base is wrong.

**No hand-written catalogue data.** Everything the site says about providers and connector
operations must come from generated files, not from markdown or a `.vue` component. A
second, hand-maintained copy of the catalogue is the exact failure this component exists to correct.

`public/catalog.json` is a generated, gitignored work product written by
`cargo run -p catalog-cli -- build`; it is not a committed artifact today. S-018 owns the site input
and test contract. It must preserve the rule that explorer source never hand-maintains provider,
operation, credential, host, or issue-code data.
