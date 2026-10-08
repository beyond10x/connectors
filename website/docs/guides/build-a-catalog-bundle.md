---
title: Build a catalog bundle
sidebar_position: 3
description: Compile a pinned OpenAPI document into a catalog bundle and confirm it reproduces the committed bundle byte for byte.
lede: A catalog bundle is a deterministic function of its pinned source; building it twice gives the same bytes, and nothing reaches the network.
source: crates/connectors-catalog/src/pipeline.rs, adapters/catalog/tests/bundle_drift.rs, docs/local-catalog-provider.md
---

# Build a catalog bundle

The catalog provider serves operations from a bundle: a pinned API document compiled into an
inventory of operations with the source's SHA-256. `connectors-build catalog` writes the bundle
and an `index.json` beside it. This guide rebuilds the GitLab bundle into a scratch directory
and compares it with the committed one. Run it from the repository root after
`cargo build --locked -p connectors-build`.

## Compile the pinned source

```sh
mkdir -p target/try/bundles
target/debug/connectors-build catalog --provider gitlab \
  --source adapters/gitlab/upstream/openapi_v3.yaml \
  --directory target/try/bundles --auth-profile gitlab.pat \
  | jq '{provider, source_sha256, openapi, info_version, inventoried, unsupported}'
```

```json
{
  "provider": "gitlab",
  "source_sha256": "f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530",
  "openapi": "3.0.0",
  "info_version": "19.4",
  "inventoried": 1847,
  "unsupported": 67
}
```

The command prints a coverage report; `jq` keeps its summary. All 1,847 operations of the pinned
GitLab document are inventoried. The 67 it names as unsupported are array query parameters the
source serialises with `explode: false`; each is sent as the one value a caller gives. The full
report lists every reason with the operations it affects.

## Confirm it is the committed bundle

```sh
cmp target/try/bundles/gitlab.bundle.json adapters/catalog/generated/bundles/gitlab.bundle.json && echo identical
```

```text
identical
```

```sh
jq '.entries[] | {provider, operations, unsupported, bundle_sha256}' target/try/bundles/index.json
```

```json
{
  "provider": "gitlab",
  "operations": 1847,
  "unsupported": 67,
  "bundle_sha256": "89090cc0a41c921afe9fc2d93480a493a33731e8017a1232fac32aa5478a3788"
}
```

The repository's own test does the same for every committed bundle and fails when one differs,
so a bundle can only change with its source.

## Sources that are not OpenAPI

- A Google Discovery document is first projected to OpenAPI 3.0.3 with
  `connectors-build discovery`; the bundle is then built with `--derived-from` naming the
  Discovery document, and the pipeline recomputes the projection and refuses a source that
  differs from it.
- A Swagger 2.0 document is projected to OpenAPI 3.1.0 with `connectors-build swagger` and
  ingested the same way (since 0.37.0). A construct the projection has no exact row for is
  refused by its JSON pointer.
- `--amendments` adds cited optional query parameters a pinned source leaves out, and the bundle
  records the amendment file's SHA-256.

A bundle alone exposes nothing. A reviewed selection set under
`adapters/catalog/providers/<provider>/operations.json` names which operations a configured
provider serves, with each one's effect; see [The catalog provider](../concepts/catalog-provider.md).
