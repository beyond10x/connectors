# AGENTS.md

Orientation for agents (and humans) working in this repository.

## Status

Pre-v1, design phase: **the documents are the product**. Read, in order:
[docs/VISION.md](docs/VISION.md) → [docs/design/01-domain-model.md](docs/design/01-domain-model.md)
→ [docs/design/02-architecture.md](docs/design/02-architecture.md). The research corpus
([docs/research/](docs/research/)) grounds both; vendored third-party artifacts live under
`docs/research/vendor/` with provenance and are never edited.

Design documents are a numbered series (`docs/design/NN-title.md`). New design work gets the next
number; earlier documents are amended with dated notes, not silently rewritten. Nouns come from
the domain model and are not renamed casually (vision principle 10).

## Commits

- Lowercase `area: summary` titles (`docs:`, `research:`, `scripts:`, `chore:`), body with
  bullet points. Write messages via `git commit -F -` with a quoted heredoc, never `-m` with
  backticks.
- Never commit key material. `.gitignore` blocks `*.pem`/`*.key`; the bot's private key lives
  **outside** every working tree (`~/selfdirect/*.pem`, mode 0600).

## Automation identity: selfdirect-bot

**Anything not typed by Timo commits and pushes as the bot — including agent sessions (Claude,
flux, CI).** Only Timo at the keyboard pushes as `timofriedlberlin`. The bot is the org-owned
GitHub App **selfdirect-bot** (App ID `4575767`; permissions: contents, pull_requests, metadata,
deployments — nothing more). No PATs, no machine accounts, no long-lived credentials.

For agents that means: work normally, but commit via `scripts/as-bot.sh commit …` and push via
`scripts/as-bot.sh push …` — never plain `git commit`/`git push`.

The scripts work with zero configuration (defaults: App ID baked in, key found at
`~/selfdirect/selfdirect-bot.*.private-key.pem`; `SELFDIRECT_BOT_*` env vars override).

Three paved paths, in order of everyday-ness:

1. **Git as the bot** — any git command, authored as `selfdirect-bot[bot]`, authenticated with a
   fresh 1-hour installation token (ssh remotes are rewritten to https for the push; the token
   travels via env + credential helper, never argv):

   ```bash
   scripts/as-bot.sh commit -m "chore: bump catalog lock"
   scripts/as-bot.sh push origin main
   ```

2. **gh CLI as the bot** — per-invocation override; the human keyring login stays untouched:

   ```bash
   GH_TOKEN=$(scripts/bot-token.sh) gh pr create ...
   GH_TOKEN=$(scripts/bot-token.sh) gh api ...
   ```

3. **API-created commits** (GraphQL `createCommitOnBranch`) — only when the GitHub "Verified"
   badge matters: GitHub signs commits it creates itself. Routine automation does not need this.

`scripts/bot-token.sh` is the primitive under all three: app JWT → installation lookup → 1-hour
token on stdout. Diagnostics go to stderr; the token is the only stdout line.

Inside GitHub Actions, prefer the built-in `GITHUB_TOKEN` (`github-actions[bot]`) over the app
unless cross-repo access is needed.

## Boundaries

- Nothing here builds yet; do not scaffold crates ahead of the build order in
  [02-architecture.md §9](docs/design/02-architecture.md).
- The predecessor repositories (`~/projects/flux-connectors`, `~/projects/flux-exchange`) are
  read-only reference: mine them, copy from them per the architecture's inventory, never edit
  them from here. Cross-product decisions live in `~/projects/flux-roadmap/decisions/`
  (0026 is the consolidation record).
