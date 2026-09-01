# AGENTS.md

Orientation for agents (and humans) working in this repository.
**github.com/beyond10x/connectors is the canonical home of Connectors.** The Zwirn product consumes
this repository as a pinned git dependency; the dependency never runs the other way.
Org-wide rules — the naming convention, the language rule (anything that runs is Rust, not Python),
the former-brand rule (atlas ADR 0001) and the rule that renaming anything another repo verifies is a
coordinated migration with an ADR — live in `atlas/AGENTS.md` and are not restated here.

## Serves

The objectives of the collection this repository moves, by id from `atlas/ROADMAP.md` — the only
cross-repository roadmap, and the page that says what each id means and which evidence closes it:

- **O1 — governed reach.** Reach into the outside world only through a catalogued operation, a bound credential and an explicit grant.
- **O5 — the generic agent platform.** The catalogue a platform tenant configures connectors from.

A change here that moves none of these is a question for the operator, not a task.
`atlas/scripts/check-map.sh` fails a repository whose `AGENTS.md` names no objective.

## Status

Pre-v1 implementation phase: the catalog, platform service, hosted integrations, and product
binaries are implemented alongside their design authority. Read current authority in order:
[docs/design/01-domain-model.md](docs/design/01-domain-model.md) →
[docs/design/02-architecture.md](docs/design/02-architecture.md) →
[docs/VISION.md](docs/VISION.md) (historical founding intent). The research corpus
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
`connectors` and `substrate` components — how everything connects — postponed until `substrate`'s
documents are finished. The procedure now exists as the `architecture-review` run-mode of the org's
main agent definition under `.agents/agents/`; trigger it when Timo says substrate is done.

## Commits

- This repository is public. Public-source privacy and release-delivery policy are governed from
  Atlas; component source must not carry organization credential machinery.
- Lowercase `area: summary` titles (`docs:`, `research:`, `scripts:`, `chore:`), body with
  bullet points. Write messages via `git commit -F -` with a quoted heredoc, never `-m` with
  backticks.
- Never commit key material. `.gitignore` blocks `*.pem`/`*.key`; delivery credentials and
  bot-authenticated remote operations are supplied by Atlas-owned tooling outside this repository.

## Adding a connector

The discipline comes from the predecessor pipeline (55 providers, 835 operations) and is
non-negotiable; the mechanics arrive with M1 (`catalog …`). Read VISION.md
principles 1–3 and the Provider/Operation sections of the domain model first.

1. **Pick the source mode.** Use an official machine-readable specification whenever one exists.
   When none exists, author the missing specification in this component from authoritative vendor
   documentation. OpenAPI ingest is for very large surfaces — and ingest **selects nothing by
   default**: a 398-operation document with no patch yields zero operations, deliberately.
2. **Specs are authoritative and truthfully owned.** Every connector begins in `specs/`. A spec is
   either the vendor's published artifact (`origin = "vendor"`) or a b10x-authored artifact
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
   from (kind, upstream, refresh script, pins, consumers). S-016 owns the future mechanical
   orphan and checksum checks; until it lands, review the index and refresh script together. The
   build is hermetic and offline; a spec refresh is
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
   a security bug. Every published operation also carries a non-empty, source-grounded one-line
   description, including operations not exposed to a model. When a bulk-selected vendor document
   omits that fact, use `[patch.descriptions.<service>]` keyed by exact `operationId`; do not turn
   the operations into exact selection blocks and reorder the catalog. The scaffolder writes
   `TODO` holes rather than guessing; filling them thoughtfully **is** the review.
7. **The expose discipline.** Catalogued ≠ projected. Everything selected is callable; only a
   curated handful get `expose = true` and reach a model. The largest predecessor provider
   catalogues 397 operations and exposes **nine** — "389 LLM tools is not a catalogue, it is a
   denial of service against a model's context." When in doubt, don't expose.
8. **Credentials: requirements, never values.** `[[auth]]` declares scheme, acquisition,
   placement, subject, hazard, full OAuth2 shape — and publishes which fields a deployment must
   supply, never a `client_id`, secret, or credential-shaped example anywhere.
9. **Declare the operation traits** — pagination, rate-limit header names, error envelope and
   verification probe. Pagination, rate limits and error envelopes are first-class operation
   fields; the retired `quirks` umbrella must not return.
10. **Build, verify, commit as one unit.**
   `catalog build` → `diff` must then report everything up to date → `check`
   (lock verifier, S-003). Unchanged inputs reproduce every artifact byte for byte — if `diff`
   moves without your input changing, stop and investigate; never re-commit drift. Current coverage
   tests prove selected operations exist; S-021 owns the missing reverse check that an allowlist
   entry cannot outlive the gap it explains.
   Commit the provider file **and** its generated artifacts together:
   `catalog: add <provider> connector`.

Checklist: identity chosen as permanent · spec origin + provenance explicit ·
selects split by damage class · risk/idempotency/effects by judgment, not guessed · no auth-flow
ops, no credential values · every description non-empty and source-grounded · expose is a
curation · operation traits declared · build + diff clean twice · one reviewable commit, by the
bot.

## Refreshing a source

`SOURCES.toml` is intended to become a **machine-processed manifest, not documentation**. S-016 owns
`catalog sources check`, checksum verification, orphan refusal, refresh, and upstream diff. Until
that story lands, use the registered refresh script and review the source bytes, provenance, pin,
and consumer list in the same change; do not claim the invariant suite checks them.

## Gate

The repository gate is authoritative. From the repository root run:

```text
bash scripts/gate.sh
```

It runs every Cargo workspace's `cargo test --workspace --locked`, the catalog lock verifier
(`catalog-cli -- check`), `scripts/check-links.py`, and `scripts/check-stories.py`. For a focused
iteration run the affected workspace's locked test/clippy/fmt commands directly; the full gate
remains required before treating a change as green. `catalog check` independently
rehashes provider declarations, vendored specs, lock rows, and generated artifacts.

The former-brand fence is **not** part of this gate and no longer lives in this repository. It is
org-wide, as `scripts/check-org-brand.sh` in atlas. The per-repo copies drifted apart and several
were silently broken — they reported "clean" and exited 0 against any input — so there is now one
fence, verified to fail on a planted string. Do not reintroduce a local copy.

`json-schemas.toml` is the closed inventory for JSON. Every component-owned JSON document names a
local schema, and the dedicated governance gate validates it with the pinned Draft 2020-12
implementation. Every JSON Schema declares and validates against that same meta-schema. Imported
vendor specs are listed by exact path as `vendored-source`: the gate syntax-checks their bytes but
does not pretend they conform to a b10x-owned schema. An unclassified JSON file, an unknown
schema reference, malformed imported JSON, or an invalid owned document fails the gate. The ignored
site projection is additionally validated against the embedded bytes of `web/catalog.schema.json`
inside the generator before the write plan can receive it.

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
   the organization delivery path.

### The gate does not fit on one machine

Eleven workspaces, none sharing a `target/` directory, need about **39 GB** between them —
`connectors-runtime` alone is 11 GB. A hosted CI runner starts with roughly 14 GB, so the gate is
sharded one workspace per runner. `scripts/gate.sh` takes three flags for that:

```text
bash scripts/gate.sh                      # everything, unchanged — what you run locally
bash scripts/gate.sh --list-workspaces    # the list, one per line, for a CI matrix
bash scripts/gate.sh --workspace <path>   # one workspace's locked tests
bash scripts/gate.sh --final              # catalog lock verifier + documentation checks
```

**CI reads the workspace list from this script and never keeps its own copy.** A second list in a
workflow file is a list that drifts from the one anybody actually runs, silently and in both
directions.

### An offline check still needs the registry populated

Three checks run offline on purpose: `engine_free` and `msrv_fence` shell out to
`cargo metadata --offline`, and `gate.sh --final` runs `catalog-cli -- check --offline`. The
no-network fence is the point of all three.

Offline does not mean the registry may be empty; it means nothing may be downloaded once the work
starts. On a cold machine, run `cargo fetch --locked` first. Without `--target` it populates the
registry for **every** target, which is exactly the graph those checks then walk — a Linux
`cargo test` never downloads a Windows-only crate, and `cargo metadata` wants the whole graph.
This is invisible on a developer machine whose registry was filled long ago.

### Read the gate's own exit status

Never pipe a gate through `tail` or `head` when you care whether it passed: the pipeline reports
the exit status of `tail`. Redirect to a file and read `$?`.

### After changing any dependency, refresh the nested lockfiles

The satellite workspaces carry their own `Cargo.lock`. Adding, removing or repointing a dependency
leaves them stale, and `--locked` then fails with *"cannot update the lock file … because --locked
was passed"*. Run `cargo metadata --offline` once in each workspace `--list-workspaces` names.

## Releases

**The version is an artifact identity, not a label.** `[workspace.package] version` is what
`catalog-build` writes into every catalog document's `generator` field, into every
`connectors.lock` row, and into the wire User-Agent. Cutting a version therefore rewrites all 67
catalog artifacts and the lockfile; that diff is the intended consequence, not churn. The bump
itself touches 184 pins across 27 manifests — every internal dependency is path-pinned to the exact
version — so bump them together and re-run `catalog build`.

Cutting a release is pushing a `v*` tag. `.github/workflows/release.yml` then runs, in order: the
tag against the committed version and the CHANGELOG against the same version; the sharded gate, the
history-wide secret scan and the `local-identity` refusal; a `--locked --release` build per target;
and a GitHub release carrying the archives, `SHA256SUMS`, and notes read from the CHANGELOG section
for that version. A tag that disagrees with the committed version is refused before anything builds.

- **The published targets are Unix only**: x86_64 and aarch64 Linux, x86_64 and aarch64 macOS.
  `connectors` does not compile for Windows and that is a design position rather than a gap —
  `connectors-config` opens files with `O_NOFOLLOW` and compares the effective uid to the owner,
  `connector-secrets` binds custody to Unix file modes, and the personal posture serves on a Unix
  socket. Supporting Windows means deciding what owner-bound credential custody is in terms of
  Windows ACLs. Do not add the target back without that decision.
- **A retired runner label queues forever rather than failing.** `macos-13` is retired; Intel macOS
  labels carry an `-intel` suffix and exist only for the two most recent versions. A release that
  silently never publishes is a worse failure than a red one, so check the label before adding a
  target.
- **No build that ships selects a Cargo feature.** `scripts/check-local-identity-refused.sh` holds
  both the Dockerfile and the release workflow to that, because both produce shipped binaries. It
  greps those two files, so a comment mentioning the flag name trips it — describe the flag instead
  of writing it.
- CI creates releases with `GITHUB_TOKEN` and `contents: write`. The bot App's private key stays
  out of CI: the rule above is about a workflow that creates or pushes a **commit**, and this one
  does neither.

## Rewriting history, and the secret baseline that depends on it

Both were done on 2026-08-25. Neither is routine; both have a failure mode that looks like success.

### Rewriting

Bundle everything first — `git bundle create <path> --all`, outside the repository — and keep the
bundle **off** the remote. Archiving the pre-rewrite lineage as a branch republishes exactly what
the rewrite removed.

Verify with the strongest check available: **if `HEAD` already contained none of what you are
removing, the rewritten `HEAD` tree must be byte-identical.** Compare `rev-parse main^{tree}` before
and after. Anything else means the rewrite changed live content, and a diff of files will not tell
you as clearly as one hash.

Then check the removal itself over every ref, not just `HEAD`: authorship, committer, commit
messages, paths ever used, and every blob (`rev-list --objects --all` piped through
`cat-file --batch-check`). A path that only ever existed in a deleted branch is still published.

`--force-with-lease` takes the **remote's** current SHA, not yours. Passing your local head rejects
the push with *"stale info"*, and if a tag push follows in the same script the remote is left
briefly inconsistent.

Force-pushing does not remove the old objects from GitHub: they stay fetchable by SHA until it
garbage-collects. Before a repository goes public, ask GitHub Support to run one, or push to a fresh
repository.

### The secret baseline

`.gitleaksignore` names exact `<commit>:<path>:<rule>:<line>` fingerprints, reviewed in
[`docs/security/secret-scan-baseline.md`](docs/security/secret-scan-baseline.md). Never widen it to
a rule or path allowlist: that survives a rewrite by also surviving a real secret landing in the
same file.

**Regenerate from a scan run with `.gitleaksignore` removed.** Scanning with it in place and asking
which entries still match is circular — findings it suppresses are absent from the report, so a
class that is merely hidden reads as resolved. That happened here and put a wrong baseline in a
commit.

A rewrite invalidates every fingerprint whose commit it *touched*, and only those. Rewriting all of
history invalidates all of them; editing one late commit leaves the early ones valid and breaks the
rest. Partial invalidation is the confusing case — expect it.

`check-secrets.sh` is not part of `scripts/gate.sh`. It runs in CI and it is the reason a red secret
gate went unnoticed for four commits. Run it before a release.

## Boundaries

- The catalog, platform family (`domain`, `protocol`, `service`, `server`), hosted runtime and
  integrations, and user-facing `connectors` binary have landed. `catalog` remains the
  component-maintenance CLI and is never a release artifact. Extend the existing named owner
  crates; do not recreate their responsibilities in a parallel package.
- **No `codewandler-flux-*` crate may enter this workspace**, in any dependency kind — not as a
  dependency, not as a dev-dependency, not behind an off-by-default feature.
  `crates/catalog-build/tests/main/engine_free.rs` asserts it three ways.
- **Argv is parsed with clap's derive API.** Hand-rolled argument parsing is banned, for every
  binary now and later.
- **A `custody_only` provider holds a credential and describes no request surface at all** — see
  [design 16](docs/design/16-subscription-credential-custody.md). Every key that could describe an
  outbound request is refused by name in `CUSTODY_ONLY_REFUSED_KEYS`; `[[channels]]` is on that list
  because a channel binding carries its own `auth` and `connector-resolve` places those credentials
  onto the composed URL and headers. Adding a declaration field that can reach a request means
  adding it there too, or the kind's one guarantee stops being true.
- **Ask the declared key, not the assembled value.** `#[serde(default)]` makes `base_url = ""`
  indistinguishable from no `base_url` once parsed, so a check written against the loaded struct
  accepts what an author plainly wrote. Where presence is what matters, read the TOML keys —
  `implicit_service_members` and `validate_custody_only` both do.
- **Provider testing is one consolidated file**, `crates/catalog-build/tests/main/catalog_invariants.rs`,
  which iterates the whole catalogue. Do not add a per-provider test file: a rule about connectors
  is stated once and parameterised, so the next connector is covered the moment it exists. The
  workspace fences (dependency, engine-free, no-network, MSRV) stay separate — each is an argument
  about the workspace, not about the catalogue.
- The predecessor repositories
  ([`codewandler/flux-connectors`](https://github.com/codewandler/flux-connectors) and
  [`codewandler/flux-exchange`](https://github.com/codewandler/flux-exchange)) are read-only
  references: mine them, copy from them per the architecture inventory, and never edit them from
  here.

`predecessor:docs/designs/...` citations in code are symbolic, nonnormative provenance markers, not
navigable authority. The surrounding comment must restate the rule the current implementation uses;
a private predecessor document can never be the only explanation for a b10x behavior.
