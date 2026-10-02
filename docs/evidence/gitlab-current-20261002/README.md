# Current GitLab read replay — 2026-10-02

All eighteen shipped catalog reads returned HTTP 200 through the current
production CLI and catalog child against the dedicated GitLab sandbox. The
[observations](read-observations.json) retain each operation, exact input,
response kind/count and provider provenance. Original response bodies remain
private; their [hashes](private-response-hashes.sha256) identify the raw evidence.
This is read-only provider evidence, not a fresh mutation/recovery run.

The eleven historical reads are project.get, issues.list, file.get, branch.get,
merge_requests.list, merge_request.get, pipelines.list, pipeline.get, pipeline.jobs,
job.get and job.trace. The seven additional shipped reads are projects.list,
tags.list, releases.list, project.events, commits.list, repository.compare and
deployments.list. Tags, releases and deployments returned empty lists; those
results do not prove nonempty payload semantics. The job trace returned text,
2,383 UTF-8 bytes. List results are individual bounded pages, not complete exports.

Fixture: existing `connectors-gitlab-20260912`, API `https://localhost:8929/api/v4`,
project `root/connectors-sandbox` / id 1, delegated sandbox user id 2. Exact current
inputs came from the preceding reads: merge request IID 11, pipeline 23, job 27,
main branch and its actual commit/parent. No GitLab API write, configuration change,
token rotation or runner change occurred. The protected credential was read from
its existing owner-only file by the CLI; no secret entered argv or a published log.

Fresh private custody passed setup qualification. Initial admission succeeded.
The first explicit revalidation returned [unavailable at dispatch](revalidate-1.stderr)
at approximately 16:26:31 UTC, and four subsequent reads refused `not_granted`
while the connection was pending. These are retained failures, not passing rows.
The cause of that single revalidation failure is not established. A later explicit
[revalidation](revalidate-2.json) succeeded, followed by all eighteen reads; another
[revalidation](revalidate-3.json) also succeeded. No deadline or evidence lifetime
was changed to make this replay pass.

The production owner was restarted after exact child stop. Its sealed executable
digest, configuration argument and process start identity were checked before
terminating that task-owned process. The new owner reused the saved credential:
[connection and revision stayed equal](restart-connection-check.json), the
[host incarnation changed](restart-host-check.json), explicit
[revalidation succeeded](revalidate-after-restart.json), and project.get again
returned HTTP 200. This establishes an owner process restart, not an orderly
shutdown protocol test or a keyring restart test.

The [cleanup record](cleanup.log) records exact child stop, disappearance of the
final detached owner's executable and waited exit 0 for both custody daemons.
A numeric child wait status is unavailable for the detached owner. The private
state/evidence is retained; the shared GitLab container and runner remain running.

[Binary identities](binaries.sha256) identify copies of the reviewed v0.25.0
production CLI/catalog inputs at source base
`f3fb222b7edc7fc29520dd30effdb58bfdec5274`; the relevant production source diff
against that base was empty. The source base also contains planning/dev-dependency
work, which does not change these executable inputs. This observation does not
claim a new release, the deterministic catalog mutation matrix, all GitLab
workflows, sustained store performance or MCP delivery.

Commands used `connectors --config <private-config> --state-dir <private-state>
--output json`: setup init/check; connections connect with a protected credential
file; operations describe followed by operations invoke with the returned schema
and revision and an input file; explicit connections revalidate with the original
revision; and adapters status/stop with the exact observed owner/child coordinates.
Home prefixes in published records are replaced by literal `$HOME`. Raw evidence
and command outputs remain in the task's private replay directory.

The [read review](review.md) matched all eighteen observations to their raw inputs
and results and checked the retained response hashes without new provider calls.
All retained operation descriptions were marked cached/stale; this replay does
not claim fresh vendor discovery. The initial revalidation failure remains
unexplained, and process termination ordering is coordinator-recorded evidence.
