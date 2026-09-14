---
format: aep.planning-md/1
id: epic:retire-native-gitlab-adapter
kind: epic
status: active
title: Serve GitLab from the catalog only and retire the native adapter
relations:
- derived_from: architecture-decision-record:declarative-http-provider-runtime
- decomposes: initiative:complete-local-connectors
- serves: vision:independent-contract-adapters
revision: 3
---
## Outcome

GitLab is served by one runtime: the catalog provider, from the pinned OpenAPI
source at `adapters/gitlab/upstream/openapi_v3.yaml` and a reviewed selection set
committed in the repository. The hand-authored adapter specification, its generated
Rust and its native handlers under `adapters/gitlab/` are retired once every
operation they carry runs through the catalog with equal or stricter guarantees.

`architecture-decision-record:declarative-http-provider-runtime` decided that
ordinary HTTP providers run declarative templates and must not require a
handwritten Rust handler per endpoint. After v0.10.0 the repository carries two
runtimes for one provider: `adapters/gitlab/src/` (1,788 lines, 11 reads, `merge`
and `update` native) and `adapters/catalog/` (1,046 lines, generic, 0 GitLab
mentions in source) selecting `get`, `branch.get`, `create` and `update` from the
same document. `merge_request.get` and `merge_request.update` exist in both.

## Scope

Two increments, in order.

1. `story:gitlab-through-catalog-complete`: additive. Every native read, `merge`
   and `update` are exposed by a committed GitLab selection set that the catalog
   provider loads; the declarative guard gains literal expectations and multiple
   checks so the merge precondition (`sha` in the PUT body, `state`,
   `detailed_merge_status`) is data; text responses such as the job trace are
   declared by the bundle's response media types. Nothing native is removed.
2. Removal, decomposed after increment 1 is proven in the sandbox: delete
   `adapters/gitlab/{src,generated,spec/adapter.json,tests,contracts}` and the
   `connectors-gitlab` crate; re-point `crates/connectors-conformance` (GitLab
   slice), `crates/connectors-build/src/docs.rs` (walkthrough descriptor),
   `crates/connectors-spec/tests/*` (generator fixture, 5 references to
   `adapters/gitlab/spec/adapter.json`), the gate's adapter list, the website
   GitLab page and `docs/*gitlab*.md`. Keep `adapters/gitlab/upstream/` (the pinned
   source), `adapters/gitlab/spec/ess/` only if the ESS boundary gate still needs
   it, and every `docs/evidence/gitlab-*` directory as history.

Open in increment 2 and left for the operator: whether the v3 write generator in
`crates/connectors-spec` keeps a consumer once GitLab writes are catalog-only.

## Acceptance

- One `connectors` configuration exposes every operation the native adapter
  exposed today, through `adapter_id = "catalog"`, with the sandbox evidence
  recorded per operation.
- `cargo tree -p connectors-gitlab` fails: the crate no longer exists.
- The repository gate passes with the adapter list `["kubernetes","sql"]` plus the
  catalog provider.
