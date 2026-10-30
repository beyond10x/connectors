# github upstream sources

Taken from [`beyond10x/connectors`](https://github.com/beyond10x/connectors) at commit
`396a146f8`. The `.gz` file is a gzip copy of the file Connectors pins, which is GitHub's document
redacted by `connectors-build redact` under `upstream-redaction/4` (79 example credentials and 166
email addresses replaced); SHA-256 and size are of the uncompressed redacted bytes. The upstream
bytes before redaction are SHA-256 `4a5394a12ecd1dd0f386ab0eddc133befca3ab831f56f902d25767d64c41684c`,
8,380,323 bytes.

| File | Taken from | URL | Retrieved | SHA-256 | Size | License |
|---|---|---|---|---|---|---|
| `api.github.com.yaml.gz` | `adapters/github/upstream/api.github.com.yaml` | https://raw.githubusercontent.com/github/rest-api-description/v2.1.0/descriptions/api.github.com/api.github.com.yaml | 2026-10-17 | `5e9edc27c465626b020eb6d9b4d5892e93b4e96f8c7af53ff952ac59b48c0df9` | 8,372,769 bytes | MIT (`info.license`), text in `LICENSE.md` |
| `LICENSE.md` | `adapters/github/upstream/LICENSE.md` | none | none | `3243761cbac07e6d169a5a2f4e7c25cc544da85248e735df74c3672e055cc87b` | 1,063 bytes | the license text |

Provenance, review and refresh notes: `adapters/github/upstream/README.md` in `beyond10x/connectors`.
