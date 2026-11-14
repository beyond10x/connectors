# Pinned Basecamp source

`openapi.json` is Basecamp's OpenAPI 3.1 document of the Basecamp API, `openapi.json` at the root
of `basecamp/basecamp-sdk`, git blob `477e1fb0215826ff1c1c70ad9f41cc03b9e0907b`, last changed by
commit `5b2cd7d00fb8028840bba3fb46cb9d2ef132bf29` and unchanged at `main`
`867cdd87d0fbbefbad9fd99648002496c909c834`, retrieved on 2026-11-13, unmodified:

- [Exact source](https://raw.githubusercontent.com/basecamp/basecamp-sdk/5b2cd7d00fb8028840bba3fb46cb9d2ef132bf29/openapi.json)
- SHA-256: `6f309fd546710c53747ff546e0459083fd565902df07932aed49d9ba1a2603bb`
- Size: 1,357,916 bytes.
- OpenAPI `3.1.0`; `info.title` `Basecamp`; `info.version` `2026-11-05`; 300 operations under
  200 paths, each path starting with `/{accountId}/`; no `servers` and no `securitySchemes`.
- `info.license` is MIT. The [license at `main`](https://raw.githubusercontent.com/basecamp/basecamp-sdk/867cdd87d0fbbefbad9fd99648002496c909c834/LICENSE)
  is retained as `LICENSE` (SHA-256
  `a5146eeb7e5f84a79d0f0595605b3d5c09f2f2b154ce3236f379ef9ed934f4c4`, 1,065 bytes).

What the document does not say comes from Basecamp's API reference,
[`basecamp/bc3-api`](https://github.com/basecamp/bc3-api) (README, read 2026-11-13): every URL
starts with `https://3.basecampapi.com/<account id>/`; a request carries
`Authorization: Bearer <token>` and a `User-Agent` naming the application and a contact, or is
answered `400`; a list names its next page in the `Link` header's `rel="next"`, and a blank
`Link` is the last page; `429` names the wait in `Retry-After`.

The reference writes a single resource's path with `.json` (`GET /projects/1.json`); the document
writes several without it (`GetProject` is `/{accountId}/projects/{projectId}`). The document is
pinned as published, with no amendment: it is the one Basecamp's own SDKs are generated from, and
their generated clients send these paths without `.json` (the TypeScript SDK's
`typescript/src/generated/services/projects.ts` at `main` sends `GET /projects/{projectId}`), which
the SDK's nightly live canary against production Basecamp reads for `GetProject` and `GetTodoset`
(`conformance/runner/typescript/live-dispatch.ts`). Basecamp's conformance case for
`GetTimesheetEntry` expects `/999/timesheet_entries/999` (`conformance/tests/paths.json`).

The catalog provider's Basecamp bundle and selection are in `adapters/catalog`
([guide](../../../docs/catalog-basecamp.md)); they are the lowering of the definition
`crates/connectors-catalog/tests/fixtures/definitions/connectors/basecamp.yaml`, the copy of
`connectors/basecamp.yaml` in `beyond10x/connectors-definitions`. Basecamp does not endorse this
connector. Refreshing the source means downloading the file at a new commit, replacing this file,
the digests and the gzip copy of the definitions repository, and lowering the definition again
(`adapters/catalog/tests/basecamp.rs`, `adapters/catalog/tests/bundle_drift.rs`).
