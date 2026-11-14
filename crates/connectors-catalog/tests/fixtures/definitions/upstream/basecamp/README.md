# basecamp upstream sources

Written by hand from [`beyond10x/connectors`](https://github.com/beyond10x/connectors)
`adapters/basecamp/upstream/`: `connectors-build definition import` cannot write this definition,
whose paging style says `link_next`. Each `.gz` file is a gzip copy of the unmodified file; SHA-256
and size are of the uncompressed bytes.

| File | Taken from | URL | Retrieved | SHA-256 | Size | License |
|---|---|---|---|---|---|---|
| `openapi.json.gz` | `adapters/basecamp/upstream/openapi.json` | https://raw.githubusercontent.com/basecamp/basecamp-sdk/5b2cd7d00fb8028840bba3fb46cb9d2ef132bf29/openapi.json | 2026-11-13 | `6f309fd546710c53747ff546e0459083fd565902df07932aed49d9ba1a2603bb` | 1,357,916 bytes | MIT, text in `LICENSE` |
| `LICENSE` | `adapters/basecamp/upstream/LICENSE` | https://raw.githubusercontent.com/basecamp/basecamp-sdk/867cdd87d0fbbefbad9fd99648002496c909c834/LICENSE | 2026-11-13 | `a5146eeb7e5f84a79d0f0595605b3d5c09f2f2b154ce3236f379ef9ed934f4c4` | 1,065 bytes | the license text |

`openapi.json` is git blob `477e1fb0215826ff1c1c70ad9f41cc03b9e0907b` of
`github.com/basecamp/basecamp-sdk`, OpenAPI 3.1.0, `info.version` `2026-11-05`, 300 operations
under 200 paths. It declares no `servers` and no `securitySchemes`: the base URL
`https://3.basecampapi.com/{accountId}` and the bearer token come from Basecamp's API reference,
<https://github.com/basecamp/bc3-api> (README, "Authentication" and "Identifying your
application").

Provenance, review and refresh notes: `adapters/basecamp/upstream/README.md` in `beyond10x/connectors`.
