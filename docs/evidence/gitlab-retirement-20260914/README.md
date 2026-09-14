# GitLab through the catalog provider after the native adapter was deleted

Date: 2026-09-14. Sandbox: the same disposable GitLab as
[gitlab-sandbox-20260913](../gitlab-sandbox-20260913/README.md), container
`connectors-gitlab-20260912`, project `root/connectors-sandbox`. The tree under
test has `adapters/gitlab/` reduced to `upstream/`, no `connectors-gitlab` crate
and no v3 write generator (`story:remove-native-gitlab-adapter`).

The provider serving GitLab is `connectors-catalog-provider` built from that
tree, over the committed bundle and the shipped selection set
`adapters/catalog/providers/gitlab/operations.json`. Its bootstrap
`configuration_revision` is `46f7e22844c2dfc3462aab35722c7b6a2be4dc0eda2f18819a0417a6d1547bfa`,
the same value the 2026-09-13 run printed: the bundle, the selection set, the
authentication profile and the API base are unchanged by the deletion. The
executable's SHA-256 is in [executables.sha256](executables.sha256).

[retire.sh](retire.sh) is the script; [retire.log](retire.log) its output;
[retire-config.toml](retire-config.toml) the host configuration with the custody
socket redacted; `retire-*.json` the CLI outputs. Fresh private configuration
and state (`cfg9`, `state9`), a fresh connection, fresh signing keys and policy.

## Reads

| selection | input | result |
|---|---|---|
| `project.get` | `{"id":"root/connectors-sandbox"}` | 200, object |
| `issues.list` | `per_page` 2 | 200, list of 1 |
| `file.get` | `.gitlab-ci.yml` at `main` | 200, object |
| `branch.get` | `main` | 200, object |
| `merge_requests.list` | `state` opened, `per_page` 3 | 200, list of 3 |
| `merge_request.get` | IID 10 | 200; `sha` `b268f4f4…`, head pipeline 19 `success`, `detailed_merge_status` `not_open` |
| `pipelines.list` | `per_page` 2 | 200, list of 2 |
| `pipeline.get` | 19 | 200, object |
| `pipeline.jobs` | 19 | 200, list of 1 (job 23) |
| `job.get` | 23 | 200, object |
| `job.trace` | 23 | 200, text |

Eleven reads, eleven 200s, the trace as a string. The source branch of merge
request 10 was removed at its merge on 2026-09-13, so `branch.get` reads `main`
here; the recorded head of the merged request is the `sha` field of
`merge_request.get`.

## Writes: three attempts, none dispatched

Merge request 10 is merged; merge request 999 does not exist. Every attempt ran
through `approvals prepare`, `approvals issue` and `operations invoke` with a
proof and an idempotency key, and the guard refused each one before the PUT.

| attempt | input | result | PUTs in GitLab's access log |
|---|---|---|---|
| `merge_request.merge` on 10 | pinned `body.sha` `b268f4f4…`, `pipeline_id` 19 | `forbidden`, stage `dispatch`, `not_attempted` | 0 before, 0 after |
| `merge_request.merge` on 999 | same pin | `forbidden`, `not_attempted` (preflight 404) | 0 |
| `merge_request.update` on 10 | pinned `sha` `b268f4f4…`, new title | `forbidden`, `not_attempted` | 0 before, 0 after |

The counts are `grep -c` over `/var/log/gitlab/nginx/gitlab_access.log` inside
the container for `PUT …/merge_requests/10/merge`, `…/999/merge` and
`…/merge_requests/10`. After the run GitLab still reports merge request 10 as
`merged` at `b268f4f4…` with the title set on 2026-09-13.

## What this run does and does not show

- The catalog provider built without the native adapter serves the whole shipped
  selection set against a live GitLab, and its guard refuses before dispatch on
  a merged request, a missing request and a merged request's update.
- The applied merge, the applied create and update, the 409 and the raced
  create are not repeated here; they are the 2026-09-13 record with the same
  engine code, which this increment did not change.
- The production CLI journeys for settlement, owner crash and background
  recovery have not been re-run with the catalog provider as the child;
  `story:catalog-cli-journeys` owns that.

## Repository gate

`cargo run --locked --offline -p connectors-build -- gate --msrv` on the tree
with the native adapter deleted, before the version bump: every command exit 0,
78 test targets, 0 failed, `gate: all checks passed`; the descriptor loop prints
`kubernetes` and `sql` only, the library-boundary loop `kubernetes`, `sql` and
`catalog-provider`. Log: [gate.log](gate.log). `cargo tree -p connectors-gitlab`
answers `package ID specification connectors-gitlab did not match any packages`.

The same gate on the release tree, after the version bump to 0.11.0 and the
planning-store moves: every command exit 0, 78 test targets, 0 failed,
`gate: all checks passed`; website typecheck and build exit 0. Log:
[gate-release.log](gate-release.log). This file and the planning record of the
run are the only edits after it.
