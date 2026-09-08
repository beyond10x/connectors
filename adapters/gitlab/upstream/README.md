# Pinned GitLab source

`openapi_v3.yaml` is the unmodified official GitLab OpenAPI source at commit
`2ff8d865e5016b14b724d1c2ce745f8300696192`, retrieved on 2026-09-08:

- [Exact source](https://gitlab.com/gitlab-org/gitlab/-/raw/2ff8d865e5016b14b724d1c2ce745f8300696192/doc/api/openapi/openapi_v3.yaml)
- SHA-256: `f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530`
- Size: 3,764,107 bytes.
- [Official OpenAPI documentation](https://docs.gitlab.com/api/openapi/).
- [License at the same revision](https://gitlab.com/gitlab-org/gitlab/-/raw/2ff8d865e5016b14b724d1c2ce745f8300696192/LICENSE), retained as `LICENSE`.

Copyright GitLab Inc. The repository license explicitly places content under `doc/`
under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). That license
applies to this source and its selected/normalized derivative
`../generated/upstream.openapi.json`; it does not inherit the workspace's Rust
package license. The derivative selects three GET operations and their local
schema references, normalizes YAML into JSON, and preserves upstream descriptions.
GitLab does not endorse this adapter.

The selected operation IDs are `getApiV4ProjectsId`, `getApiV4ProjectsIdIssues`,
and `getApiV4ProjectsIdRepositoryFilesFilePath`. Connectors-authored mappings and
runtime obligations are in `../spec/adapter.json` and `../src/lib.rs`.

Source inspection found two response limitations: the issues response references
one issue instead of an array, and file retrieval declares no response body schema.
Handwritten response checks preserve the previously verified service contract.
`../generated/coverage.json` records source coverage; `ess-import.json` retains the
complete ESS 0.9.2 refusal of the original OpenAPI subset. Source refresh requires
updating this pin, its attribution and the reviewed mappings together.
