---
format: aep.planning-md/3
id: story:docs-generated-reference-readable
kind: story
status: draft
title: The generated CLI reference names what a user types and says what each flag does
summary: 'docs review run 3: 172 ESS-id placeholders, 147 of 181 flags without a meaning'
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:public-documentation-website
revision: 1
---
## Outcome

The generated CLI reference names what a user types and says what every flag does, and generated
pages drop boilerplate a user cannot act on.

## Why now

Post-0.54.0 docs review (run 3, 2026-10-19, lane D5, verified): `website/docs/reference/cli.md` shows
172 placeholders such as `--connector <field:connector>` (value names fall back to ESS ids,
`crates/connectors-docs/src/cli.rs:53-56`, `apps/connectors/src/help.rs` sets no `value_name`), and 147
of 181 flag rows have an empty Meaning cell. Five sections only say "Deprecated name of …" (3,061 B).
106 generated reference pages repeat one of two info banners (31,861 B,
`crates/connectors-docs/src/reference.rs:336-341`, `:416-418`). Rendered contract pages carry internal
tokens (`story:<id>`, `(F<nn>)`, `Design §N`; 10 hits on 8 pages). `connectors/mcp.md` repeats "Not
available" under eight headings because the gate requires them of planned pages too.

## Acceptance

- `help.rs` sets value names (`FILE`, `DIR`, `NAME`, `PROFILE`, `REVISION`, …) and a one-line help for
  every visible flag; a test in `crates/connectors-docs/src/cli.rs` fails on a visible flag without help.
- `cli.rs` skips deprecated leaf sections (rows stay in the Commands table) and leaves the Values cell
  empty for free text.
- `reference.rs` drops the two banners and the owner suffix; the contract index page states it once.
- `markdown.rs` strips `story:<id>`, `(F<nn>)` and `Design §N` from rendered pages.
- `onboarding.rs` exempts `planned` pages from the eight sections; `mcp.md` shrinks.
- `cargo run --locked -p connectors-docs -- generate` output is committed and the drift gate passes.
- Files: `crates/connectors-docs/src/{cli,onboarding,status,reference,markdown}.rs`,
  `apps/connectors/src/help.rs` and its tests, `website/docs/connectors/mcp.md`, regenerated pages.
