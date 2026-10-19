---
format: aep.planning-md/3
id: story:docs-first-run-path
kind: story
status: draft
title: The first-run path from README to a first read has no wrong command or maintainer history
summary: 'docs review run 3: seven wrong facts on the first-run and concept pages; README 38.8 KB'
relations:
- serves: vision:independent-contract-adapters
- informed_by: specification:public-documentation-website
revision: 1
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
Size: README.md is 38,807 B and puts 147 lines of status before "Get started" (README.md:185-331);
install and configuration text is duplicated between README and the site.

## Acceptance

- The seven wrong facts above are fixed.
- README keeps the support table's first column (`onboarding.rs` reads it), the vocabulary table
  (gate rules 2 and 5) and links; its status history, install commands and configuration block move to
  the site or go. README.md is at most 15,000 B.
- `set-up-the-local-cli.md:163-230` moves to a new `guides/upgrade-the-cli.md`.
- `concepts/*.md` lose the line ranges listed in the review (catalog-provider 11-31, 45-57, 73-74;
  governed-invocation 30-46; local-runtime 21-26, 60-67, 124-132, clock block 87-121 to `docs/`;
  adapters 11-41, 54-62).
- Files: README.md, `website/docs/{index.md,getting-started.md,examples/follow-a-request.mdx}`,
  `website/docs/guides/{set-up-the-local-cli,invoke-a-definition-file,author-a-definition,build-a-catalog-bundle}.md`,
  new `guides/upgrade-the-cli.md`, `website/docs/concepts/*.md`.
