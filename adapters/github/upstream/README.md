# Pinned GitHub REST source

`api.github.com.yaml` is GitHub's OpenAPI description of the `api.github.com`
REST API, published in
[github/rest-api-description](https://github.com/github/rest-api-description)
at release `v2.1.0` (tag commit `c8142ac6c70bf74a638137deeecdae4b7fc8eb61`,
2022-10-25, the repository's latest release on 2026-10-17), with example
credentials and email addresses redacted before it was committed:

- [Exact source](https://raw.githubusercontent.com/github/rest-api-description/v2.1.0/descriptions/api.github.com/api.github.com.yaml)
- Upstream SHA-256: `4a5394a12ecd1dd0f386ab0eddc133befca3ab831f56f902d25767d64c41684c`,
  8,380,323 bytes. The upstream bytes are not committed.
- Redaction rule: `upstream-redaction/4`, applied by
  `cargo run --locked -p connectors-build -- redact --input <upstream file> --output api.github.com.yaml`
  (`crates/connectors-build/src/upstream_redaction.rs`, which states the rule).
  On this document it replaced 79 example credentials (3 of them PEM private
  key blocks and 10 YAML `key:` values) and 166 email addresses (to
  `user@example.com`); it found no phone number.
- Redacted SHA-256: `5e9edc27c465626b020eb6d9b4d5892e93b4e96f8c7af53ff952ac59b48c0df9`, 8,372,769 bytes.
- OpenAPI `3.0.3`; `info.title` `GitHub v3 REST API`; `info.version` `1.1.4`;
  server `https://api.github.com`.
- Licence: MIT, `info.license` and the repository's
  [`LICENSE.md`](LICENSE.md), vendored here as published at the tag
  (SHA-256 `3243761cbac07e6d169a5a2f4e7c25cc544da85248e735df74c3672e055cc87b`,
  "Copyright (c) 2020 GitHub").

The tag, not the URL alone, identifies the source; the upstream digest above
pins the bytes. Reproducing the committed file means fetching the URL,
checking the upstream digest and running the redaction command.

`github-source-hashes.json` records the URL, both digests and lengths, and the
rule. The repository gate (`connectors-build gate`, and `connectors-build
source-hashes`) checks that the committed file has the redacted digest and
length and that the rule would replace nothing more in it.
`adapters/catalog/tests/github.rs` asserts that this file, the manifest
record, this README, the licence and the committed bundle index name one
digest.

The catalog provider compiles this document into
`../../catalog/generated/bundles/github.bundle.json`; the reviewed read-only
selection set is `../../catalog/providers/github/operations.json`, described
in [the GitHub guide](../../../docs/catalog-github.md). GitHub does not
endorse this adapter. Refreshing the source means replacing this file, its
record and the bundle together.
