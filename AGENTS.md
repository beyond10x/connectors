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

## Dispatching work

**Implementation over planning, dispatched wide.** Progress is judged by working code landing, not
by how well work is specified — a wave in which half the merged stories are `docs/design/*.md` reads
as stalling. Front-load design into as few stories as possible and fan the code stories out in
parallel, up to five at once. When a wave is narrow, say plainly what caps it: if the dependency
graph rather than caution, name the blocked stories and what unblocks each.

**Deferred by Timo on 2026-08-13:** a family-level architecture review across the `architecture`,
`connectors` and `substrate` repositories — how everything connects — postponed until `substrate`'s
documents are finished. Pick it up when Timo says substrate is done.

## Commits

- This repository is private. Its visibility must not change without Timo's explicit approval.
- Lowercase `area: summary` titles (`docs:`, `research:`, `scripts:`, `chore:`), body with
  bullet points. Write messages via `git commit -F -` with a quoted heredoc, never `-m` with
  backticks.
- Never commit key material. `.gitignore` blocks `*.pem`/`*.key`; the bot's private key and App
  configuration live in the user's XDG config directory, outside every working tree, mode 0600.

## Automation identity: b10x-bot

**Anything not typed by Timo commits and pushes as the bot — including agent sessions and CI.**
Only Timo at the keyboard pushes as a human. The bot is the org-owned GitHub App
**b10x-bot** (permissions: contents, workflows, metadata — nothing more). No PATs, machine
accounts, or long-lived repository credentials.

For agents that means: work normally, but commit via `scripts/as-bot.sh commit …` and push via
`scripts/as-bot.sh push …` — never plain `git commit`/`git push`.

The scripts read `b10x-bot.json` and `b10x-bot.private-key.pem` from the external
B10x XDG config directory; `B10X_BOT_*` environment variables override those defaults.

Three paved paths, in order of everyday-ness:

1. **Git as the bot** — any git command, authored as `b10x-bot[bot]`, authenticated with a
   fresh 1-hour installation token (ssh remotes are rewritten to https for the push; the token
   travels via env + credential helper, never argv):

   ```bash
   scripts/as-bot.sh commit -m "chore: bump catalog lock"
   scripts/as-bot.sh push origin main
   ```

2. **gh CLI as the bot** — per-invocation override; the human keyring login stays untouched:

   ```bash
   scripts/bot-gh.sh pr create ...
   scripts/bot-gh.sh api ...
   ```

3. **API-created commits** (GraphQL `createCommitOnBranch`) — only when the GitHub "Verified"
   badge matters: GitHub signs commits it creates itself. Routine automation does not need this.

`scripts/bot-token.sh` is the primitive under all three: app JWT → installation lookup → 1-hour
token on stdout. Diagnostics go to stderr; the token is the only stdout line.

GitHub Actions that only read or test may use `GITHUB_TOKEN`. A workflow that creates or pushes a
commit must authenticate and author it as `b10x-bot[bot]`.

## Adding a connector

The discipline comes from the predecessor pipeline (55 providers, 835 operations) and is
non-negotiable; the mechanics arrive with M1 (`catalog …`). Read VISION.md
principles 1–3 and the Provider/Operation sections of the domain model first.

1. **Pick the source mode.** Use an official machine-readable specification whenever one exists.
   When none exists, author the missing specification in this repository from authoritative vendor
   documentation. OpenAPI ingest is for very large surfaces — and ingest **selects nothing by
   default**: a 398-operation document with no patch yields zero operations, deliberately.
2. **Specs are authoritative and truthfully owned.** Every connector begins in `specs/`. A spec is
   either the vendor's published artifact (`origin = "vendor"`) or a B10x-authored artifact
   (`origin = "repository-authored"`). The latter must say that it is ours, cite the exact official
   references from which every endpoint, parameter and auth fact was derived, state its coverage,
   and pin its bytes. Never present an authored document as vendor-published. For an agent
   specifically: writing endpoint paths, parameters or auth details from memory is fabrication;
   every operation declared in a spec must be grounded in a source that was fetched and can be
   cited. When hunting a spec location, mine the vendored competitor
   catalogs first (`docs/research/vendor/` — Nango's providers.yaml alone carries docs URLs and
   auth endpoints for ~950 vendors); `catalog sources mint <vendor>` (S-017) automates the
   lookup with per-field citations and printed disagreements.
3. **Identity is irreversible.** In `providers/<id>.toml`: `id` (lowercase, stable, public),
   `authority` (reverse-DNS, e.g. `com.gitlab.api` — it leads every credential path and is
   never repointed once published), and operation ids (one namespace per service, public API
   forever). Choose like crate names.
4. **Own the spec properly.** A vendor spec uses `scripts/vendor-<vendor>-spec.*` to pull bytes
   into `specs/<vendor>/`, apply a **declared scrub** (credential-shaped examples, emails, phone
   numbers removed, allowlist-shaped, test-enforced), and record source, date, `sha256` and
   `upstream_sha256` in `specs/<vendor>.provenance.toml`. A repository-authored spec records its
   authoritative references, authorship, coverage and `sha256` in the same provenance layer; it is
   reviewed against those references rather than pretending to have an upstream byte stream.
   Either mode then feeds a deterministic projection, with reviewed overlays for judgments the
   source cannot supply, into the canonical connector document. The same spec + overlays must
   reproduce our own format byte for byte.
   **Register the source in `SOURCES.toml`** — the single index of everything this repo derives
   from (kind, upstream, refresh script, pins, consumers); the index is test-enforced, so a
   vendored file without an entry fails. The build is hermetic and offline; a spec refresh is
   its own reviewable commit (see "Refreshing a source" below).
5. **Declare the surface as rules, not lists.** `[patch.naming]` is one rule (pins only to
   hold shipped ids still); one `[[patch.select]]` per (document, method class) — never reads
   and deletes in one statement, `risk` is a damage claim and they are not one claim;
   `[[patch.operations]]` for the exceptions only. Auth-flow endpoints (`/oauth/token`,
   `/oauth/authorize`, revocation) are **withheld by rule** — the platform performs
   authentication; they never become operations.
6. **The judgment calls no spec can make.** `risk` (ordered; when unsure, claim higher),
   `idempotency` (a create that dedupes on an external key is `conditional`), `direction`,
   `effects` (declared, never derived) — the grant system consumes these, so a wrong claim is
   a security bug. The scaffolder writes `TODO` holes rather than guessing; filling them
   thoughtfully **is** the review.
7. **The expose discipline.** Catalogued ≠ projected. Everything selected is callable; only a
   curated handful get `expose = true` and reach a model. The largest predecessor provider
   catalogues 397 operations and exposes **nine** — "389 LLM tools is not a catalogue, it is a
   denial of service against a model's context." When in doubt, don't expose.
8. **Credentials: requirements, never values.** `[[auth]]` declares scheme, acquisition,
   placement, subject, hazard, full OAuth2 shape — and publishes which fields a deployment must
   supply, never a `client_id`, secret, or credential-shaped example anywhere.
9. **Declare the runtime traits** — pagination, rate-limit header names, error envelope,
   verification probe. (They live under `quirks` until S-015 renames them; declare them
   regardless.)
10. **Build, verify, commit as one unit.**
   `catalog build` → `diff` must then report everything up to date → `check`
   (lock verifier, S-003). Unchanged inputs reproduce every artifact byte for byte — if `diff`
   moves without your input changing, stop and investigate; never re-commit drift. Coverage
   tests hold in both directions (an allowlist entry may not outlive the gap it explains).
   Commit the provider file **and** its generated artifacts together:
   `catalog: add <provider> connector`, via `as-bot.sh` if you're an agent.

Checklist: identity chosen as permanent · spec origin + provenance explicit ·
selects split by damage class · risk/idempotency/effects by judgment, not guessed · no auth-flow
ops, no credential values · expose is a curation · runtime traits declared · build + diff clean
twice · one reviewable commit, by the bot.

## Refreshing a source

`SOURCES.toml` is a **machine-processed manifest, not documentation**. Code owns it (S-016):
`catalog sources check` validates the index, verifies every checksum against the bytes on
disk and refuses orphans in both directions — it runs in the invariant suite, so a drifted pin
fails the build; you will never discover drift by reading TOML. `catalog sources refresh`
executes the fetch + declared scrub + re-pin; `catalog sources diff` probes upstream without
mutating anything. Nobody fetches or compares by hand, at any scale.

The agent instruction is therefore short:

1. `catalog sources refresh <id>` for vendor-published specs (until S-016 lands: run the entry's
   `scripts/vendor-*.*`). For a repository-authored spec, use the source tool's authored-review
   path: re-check it against every cited official reference, preserve the authorship marker and
   coverage statement, and re-pin the reviewed bytes. Never silently change origin class.
2. `catalog build`, review the **canonical-document diff** — the reviewable object.
   Empty catalog diff → say so and commit the pin bump alone.
3. Newly selected operations are a **curation decision, never an auto-select** (the runbook's
   judgment calls apply); vanished operations are acknowledged, not silently dropped — coverage
   holds in both directions.
4. One commit: `catalog: refresh <vendor> spec`, body summarizing the catalog diff, via
   `as-bot.sh`.

## Boundaries

- **M1 has landed: the catalog builds here.** `cargo build --workspace` and
  `cargo test --workspace` are green, and `catalog build` compiles `providers/` plus the vendored
  spec cache into `catalog/`, the pack, `connectors.lock` and the site projection. Do not scaffold
  crates ahead of the remaining build order in
  [02-architecture.md §9](docs/design/02-architecture.md) — the platform family (`domain`,
  `protocol`, `service`, `server`) is M2's, and the user-facing `connectors` binary arrives with it.
  `catalog` is the repository-maintenance CLI and is never a release artifact.
- **No `codewandler-flux-*` crate may enter this workspace**, in any dependency kind — not as a
  dependency, not as a dev-dependency, not behind an off-by-default feature.
  `crates/catalog-build/tests/main/engine_free.rs` asserts it three ways.
- **Argv is parsed with clap's derive API.** Hand-rolled argument parsing is banned, for every
  binary now and later.
- **Provider testing is one consolidated file**, `crates/catalog-build/tests/main/catalog_invariants.rs`,
  which iterates the whole catalogue. Do not add a per-provider test file: a rule about connectors
  is stated once and parameterised, so the next connector is covered the moment it exists. The
  workspace fences (dependency, engine-free, no-network, MSRV) stay separate — each is an argument
  about the workspace, not about the catalogue.
- The predecessor repositories
  ([`codewandler/flux-connectors`](https://github.com/codewandler/flux-connectors) and
  [`codewandler/flux-exchange`](https://github.com/codewandler/flux-exchange)) are read-only
  references: mine them, copy from them per the architecture inventory, and never edit them from
  here. The durable housing decision is
  [`b10x/architecture` ADR 0006](https://github.com/b10x/architecture/blob/main/adr/0006-b10x-supersedes-selfdirect-housing.md).
