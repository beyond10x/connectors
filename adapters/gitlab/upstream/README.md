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
package license. The derivative selects ten GET operations and their local
schema references, normalizes YAML into JSON, and preserves upstream descriptions.
GitLab does not endorse this adapter.

The selected operation IDs are `getApiV4ProjectsId`, `getApiV4ProjectsIdIssues`,
`getApiV4ProjectsIdRepositoryFilesFilePath`, `getApiV4ProjectsIdPipelines`,
`getApiV4ProjectsIdPipelinesPipelineId`, `getApiV4ProjectsIdPipelinesPipelineIdJobs`,
`getApiV4ProjectsIdJobsJobId`, `getApiV4ProjectsIdJobsJobIdTrace`,
`getApiV4ProjectsIdMergeRequests` and
`getApiV4ProjectsIdMergeRequestsMergeRequestIid`.
Connectors-authored mappings and runtime obligations are in `../spec/adapter.json`,
`../src/lib.rs`, `../src/ci.rs` and `../src/merge_requests.rs`.

Source inspection found response limitations: the issue, MR, pipeline and pipeline-job
lists reference single entities instead of arrays; file retrieval declares no
response body schema; job trace declares JSON job data despite returning raw text.
Handwritten response checks preserve the service and native CI contracts. The trace
binding does not depend on the source's optional byte-offset/limit parameters.
`../generated/coverage.json` records source coverage; `ess-import.json` retains the
complete ESS 0.20.0 refusal of the original OpenAPI subset. Source refresh requires
updating this pin, its attribution and the reviewed mappings together.
