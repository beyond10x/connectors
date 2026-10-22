---
format: aep.planning-md/3
id: story:docs-first-run-path
kind: story
status: active
title: The first-run path from README to a first read has no wrong command or maintainer history
summary: 'docs review run 3: seven wrong facts on the first-run and concept pages; README 38.8 KB'
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:public-documentation-website
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/connectors-docs/src/vocabulary.rs
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: cited
  path: docs/local-clock.md
- confidence: cited
  path: website/docs/concepts
- confidence: cited
  path: website/docs/examples/contract-exercises.mdx
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/guides/author-a-definition.md
- confidence: cited
  path: website/docs/guides/build-a-catalog-bundle.md
- confidence: cited
  path: website/docs/guides/invoke-a-definition-file.md
- confidence: cited
  path: website/docs/guides/set-up-the-local-cli.md
- confidence: cited
  path: website/docs/guides/upgrade-the-cli.md
- confidence: cited
  path: website/docs/index.md
revision: 35
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T09:48:11Z", actor: "human:timo", revision: 34, decided_on: {"recorded":{"review_outcome":5}}, executor: "agent:connectors-session", correlation: "wave-20261022"}
- {from: "proposed", to: "active", at: "2026-10-22T09:48:11Z", actor: "human:timo", revision: 35, decided_on: {"recorded":{"review_outcome":5}}, executor: "agent:connectors-session", correlation: "wave-20261022"}
---
## Outcome

A new user goes from the README to a first read without a wrong command, a missing value or a page
of maintainer history in the way; the concept pages keep only what a user needs to act.

## Why now

Post-0.54.0 docs review (run 3, 2026-10-19, lanes D1 and D2, verified) found these wrong at
83aee7c96:
- `website/docs/guides/set-up-the-local-cli.md:234-235`: the "Next" `connections connect` omits
  `--profile`, which `apps/connectors/spec/cli.yaml:449-450` requires with no default.
- `set-up-the-local-cli.md:22-32`: the default paths are described, not given (code default:
  `$XDG_CONFIG_HOME/connectors/config.toml`, `$XDG_STATE_HOME/connectors`, else `~/.config/…`,
  `~/.local/state/connectors`).
- `website/docs/index.md:44-49` omits OpenRouter and Prometheus.
- `concepts/catalog-provider.md:21-23` says every catalog connector but Slack ships a
  `connector.json`; OpenRouter has none either.
- `guides/build-a-catalog-bundle.md:62` shows `bundle_sha256` `89090cc0…3788`; the committed bundle is
  `79ce6c959045d173c45d206eaf0bfd8dcdf9ba87215df4654ff207733865025c`.
- `guides/author-a-definition.md:257-269` lacks six refusals (`auth_profile_missing`, `auth_library`,
  `auth_lifetime`, `source_kind`, `source_too_large`, `body_enum`;
  `crates/connectors-catalog/src/definition.rs:893-915`).
- README.md:421-425 and `examples/follow-a-request.mdx:13` call a walkthrough on fictional data
  "practical".
Size: README.md is 38,807 B and puts 147 lines of status before "Get started" (README.md:185-338);
install and configuration text is duplicated between README and the site.

## Acceptance

Each wrong fact of "Why now", with its after-state:
- `set-up-the-local-cli.md`'s "Next" `connections connect` line carries `--profile`.
- `set-up-the-local-cli.md` gives the default paths literally: `$XDG_CONFIG_HOME/connectors/config.toml`,
  `$XDG_STATE_HOME/connectors`, `~/.config/connectors/config.toml`, `~/.local/state/connectors`.
- `website/docs/index.md` names OpenRouter and Prometheus.
- No page says every catalog connector but Slack ships a `connector.json` (`catalog-provider.md:21-23`
  goes with the 11-31 cut below).
- `build-a-catalog-bundle.md` shows `bundle_sha256`
  `79ce6c959045d173c45d206eaf0bfd8dcdf9ba87215df4654ff207733865025c`.
- `author-a-definition.md`'s refusal list (`:259-271` at `fc9d455e1`) names `auth_profile_missing`,
  `auth_library`, `auth_lifetime`, `source_kind`, `source_too_large` and `body_enum`.
- The word "practical" is gone from the README walkthrough line (`README.md:422`) and from
  `website/docs/examples/contract-exercises.mdx:14`.

README and pages:
- README keeps the vocabulary table (gate rules 2 and 5), links, and the support table with its
  `Connector` column unchanged (`onboarding.rs` reads only that column) and its `Binding` column. The
  `Implemented operations` cells (3,699 B of the table's 14,992 B, `README.md:284-304`) become one
  short cell per row: the count of implemented operations and a link to that connector's onboarding
  page, which lists them. The status block and the install section go as "Where the status block
  goes" says. `wc -c README.md` is at most 27,000 (38,807 at `fc9d455e1`; the kept parts measure about 25.7 KB: lines 1-184 9,040 B, the table with short operations cells about 12.7 KB, the status lines about 1.3 KB, lines 384-431 2,611 B). The `Binding` cells (9,307 B) stay; no code reads them, and shortening them is later work.
- `set-up-the-local-cli.md:163-230` moves to a new `guides/upgrade-the-cli.md`.
- `concepts/*.md` lose the line ranges listed in the review (catalog-provider 11-31, 45-57, 73-74;
  governed-invocation 30-46; local-runtime 21-26, 60-67, 124-132; adapters 11-41, 54-62). After the
  change `local-runtime.md` no longer holds the clock text from "Issuance needs a bounded clock" (`:87`, mid-line; the `key-*` sentence before it stays) to `:121` and links
  `docs/local-clock.md`, which already exists; any fact of that block `docs/local-clock.md` lacks is
  added to it.
- `crates/connectors-docs/src/vocabulary.rs::the_readme_names_every_custody_binding` (new, in
  `cargo test -p connectors-docs`): fails on the README at this story's base, which names one Linux
  binding; passes when the README names `mounted_secret` and links
  `docs/local-mounted-secret-custody.md`, and no file in the vocabulary set says "one binding per
  platform".
- Files: README.md, `website/docs/{index.md,getting-started.md,examples/contract-exercises.mdx}`,
  `website/docs/guides/{set-up-the-local-cli,invoke-a-definition-file,author-a-definition,build-a-catalog-bundle}.md`,
  new `guides/upgrade-the-cli.md`, `website/docs/concepts/*.md`, `docs/local-clock.md`,
  `crates/connectors-docs/src/vocabulary.rs`, `CHANGELOG.md`.

## Where the status block goes

`README.md:185-338` ("Where the project stands") is replaced by the support table (`README.md:284`,
its operations cells shortened as the Acceptance says) and at most 12 lines that link `CHANGELOG.md` and the site's status page
(`website/docs/status.md`). Its release history is already in `CHANGELOG.md`; nothing moves into
the generated status page, so this story does not edit `crates/connectors-docs/src/status.rs`,
`website/docs/status.md` or `website/data/status.json`.

The 12 lines name credential custody per platform, which fixes `README.md:199-203` ("Credential
custody has one binding per platform, fixed when the binary is built"):
- Linux: the Secret Service, or the read-only `mounted_secret` binding a `[custody]` table selects
  (`contracts/auth/custody/v1alpha1/semantics.md` §5.2), which refuses OAuth refresh and approval
  signing (`docs/local-mounted-secret-custody.md:124-126`) and is how Connectors runs in a
  container with no keyring; it links `docs/local-mounted-secret-custody.md`.
- macOS: the login keychain. Windows: the Credential Manager.

`README.md:339-383` ("Install a release binary") moves to `website/docs/getting-started.md`
§ "Or install a release binary" (`:32`), whose link to `README#install-a-release-binary` (`:44`)
it replaces. story:claude-getting-started-tutorial rewrites that section after this story.

This absorbs story:readme-names-mounted-custody (archived), asked by the Loom Kubernetes agent
design (question Q3, citing `README.md:198-200`).
