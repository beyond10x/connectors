# b10x/connectors

A unified integration platform for agent automation: a text-declared connector catalog compiled
to a canonical data artifact, plus a deployable service owning identity, connections,
credentials, grants, invocation, and event delivery. Clients authenticate once, hold one token,
and do everything their grants admit. Three deployment postures — personal, org, saas — with an
identical feature set.

**Status: pre-v1, foundation phase.** The catalog family builds; the deployable platform remains
design work.

- [docs/VISION.md](docs/VISION.md) — what this is, why, principles, non-goals.
- [docs/design/01-domain-model.md](docs/design/01-domain-model.md) — the nouns and their
  invariants.
- [docs/research/](docs/research/) — the platform-category survey and mined catalog-as-text
  precedents (with vendored primary sources under `docs/research/vendor/`).

This repository consolidates and succeeds `flux-connectors` and `flux-exchange`. The catalog and
ingestion pipeline migrate here largely as-is; the platform is a fresh design informed by what
those codebases proved and what they got wrong.
