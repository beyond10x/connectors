# Pinned OpenRouter source

`openrouter-openapi.json` is OpenRouter's OpenAPI document, retrieved on 2026-10-16
from <https://openrouter.ai/openapi.json>, with example credentials and personal
data redacted before it was committed:

- [Exact source](https://openrouter.ai/openapi.json)
- Upstream SHA-256: `8287890b719d375d64b5651ca4fa2bc34be3af84b24e8fceb8ab13ae9894fe12`,
  2,690,928 bytes. The upstream bytes are not committed.
- Redaction rule: `upstream-redaction/3`, applied by
  `cargo run --locked -p connectors-build -- redact --input <upstream file> --output openrouter-openapi.json`
  (`crates/connectors-build/src/upstream_redaction.rs`). Rule `/2` replaced every
  credential value under a key named like a token, secret or password, every email
  address outside the documentation domains and every phone number outside the
  fictional ranges. Rule `/3` adds the quoted values of the keys `key` and
  `key_hash` and every `sk-or-v<N>-…` OpenRouter key, in full or abbreviated: the
  document's examples carry publisher example keys that Gitleaks flags. On this
  document it replaced 41 credential values and 2 email addresses and no phone
  number. Gitleaks reports no finding in the result.
- Redacted SHA-256: `fe4f1ad414f5bd25e69b387598d23f3abd24fe74801cd157c633e30bbf766142`,
  2,690,724 bytes.
- OpenAPI `3.1.0`; `info.title` `OpenRouter API`; `info.version` `1.0.0`; the document declares no `servers`:
  the configured `api_base` is `https://openrouter.ai/api/v1` and the
  bundle's operation paths carry `/api/v1`.
- Licence: MIT (`info.license`), declared by the document; OpenRouter does not
  endorse this adapter.

The publisher serves this URL unversioned, so the upstream digest above, not the
URL, identifies the source. Reproducing the committed file means fetching the URL,
checking the upstream digest and running the redaction command.

`openrouter-source-hashes.json` records the URL, both digests and lengths, and the
rule. `connectors-build source-hashes` checks that the committed file has the
redacted digest and length and that the rule would replace nothing more in it.
`adapters/catalog/tests/openrouter.rs` asserts that this file, the record, this
README and the committed bundle index name one digest.

The catalog provider compiles this document into
`../../catalog/generated/bundles/openrouter.bundle.json`; the reviewed selection set
is `../../catalog/providers/openrouter/operations.json`, described in
[the OpenRouter guide](../../../docs/catalog-openrouter.md).
