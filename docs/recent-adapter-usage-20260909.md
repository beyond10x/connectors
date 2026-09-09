# Recent agent usage: the local Connectors acceptance baseline

Recorded 2026-09-09. Design evidence for
`specification:recent-agent-adapter-usage-20260909`; analysis, not a runtime
conformance result or approval to operate any provider.

**The product baseline is a local CLI that remembers local connections and
credentials and completes the operator's engineering workflows.** A hosted
Connectors service, cloud identity authority, and federation are not prerequisites.
Remote GitLab, Jira, Slack, database, or monitoring APIs are still ordinary
integration targets. This direction was explicitly confirmed by the operator
during this analysis.

The current Kubernetes/GitLab/PostgreSQL slice and reviewed shared contracts are
useful foundations. They do **not** establish replacement parity with recent
usage. The largest gaps are local credential management as a complete CLI journey,
MySQL/Aurora, GitLab CI and merge requests, Jira JQL, Slack conversations and writes,
monitoring discovery and instant/derived metrics, and Kubernetes/Docker execution.
Programmatic CLI calls also establish Confluence document search and incremental
multi-source collection as required workflows.
GitHub and AWS also appear in actual tool use and must not disappear from the
coverage inventory simply because they were outside the three-adapter milestone.

## 1. Evidence and limits

The fixed rolling five-day window is **2026-09-04 10:37:24 UTC through
2026-09-09 10:37:24 UTC**: 4 September 12:37 through 9 September 12:37 CEST.
Event timestamps, at second resolution, select evidence; file creation dates do
not. A query made during this window can legitimately investigate an older event.

The scan enumerated the normal Codex sessions and Claude projects stores, checked
the Codex archive location, and included additional session stores found under
the local cache/state directories. The additional automation harness, fixtures,
and probe stores contained no in-window events. Older records were used for
enumeration and earliest tool-call identity deduplication, not as recent usage.
No historical command was replayed and no provider API was called by this study.

| Measure | Result |
|---|---:|
| JSONL source files scanned | 6,268 |
| Source bytes scanned | 14,737,200,000 |
| Files with valid in-window events | 486: 317 Codex, 169 Claude |
| Unique in-window tool-call identities | 86,673: 68,761 Codex, 17,912 Claude |
| Broad keyword/continuation candidates retained privately | 43,881 |
| Recognized integration CLI command sites | 7,723 |
| Tool calls containing those sites | 4,928: 4,094 Codex, 834 Claude |
| Distinct session identifiers containing those calls | 103 |
| Additional selected helper command sites | 39 |

An actual tool call is distinguished from a **command site** inside its submitted
shell program. One tool call may contain several commands, loops, conditionals,
or parallel executions. A site can run zero, one, or several times. Counts below
include help, discovery, implementation checks and attempted calls; they are
neither HTTP request counts nor successful business-operation counts. Session
identifiers include agent work and do not count conversations authored solely by
the operator. Counts across clients overlap at tool-call/session level.

| Client | Command sites | Tool calls | Session IDs |
|---|---:|---:|---:|
| Connectors | 1,777 | 1,142 | 71 |
| fluxplane-plugin | 600 | 481 | 11 |
| kubectl | 899 | 574 | 20 |
| Docker | 956 | 555 | 32 |
| Helm | 132 | 114 | 11 |
| glab | 429 | 326 | 11 |
| gh | 2,639 | 1,819 | 48 |
| AWS CLI | 14 | 12 | 3 |
| psql | 9 | 8 | 3 |
| Podman | 2 | 2 | 2 |
| curl, including local service checks | 266 | 205 | 39 |

No direct standalone `fluxplane` executable invocation was resolved. Fluxplane
references, plugin operations, authored helpers, and implementation work remain
in the broader private extraction. An executable/package name appearing in an
argument or document was not counted as an invocation.

Extraction decoded Claude tool_use/tool_result and Codex function/custom calls,
joined outputs by tool-call ID, and deduplicated copied histories using the earliest
valid occurrence across the corpus. Bash syntax parsing excluded quoted heredoc
bodies and function definitions from direct command counts. Executable lookups
such as `command -v` were counted separately. Wrapper parsing was corrected after
finding repository arguments falsely recognized as executable names.

Coverage has practical limits:

- There are 476 malformed JSON records across the full historical corpus. In
  files also containing valid in-window events, 52 records are malformed: 27
  have an in-window timestamp, 14 are older, and 11 have no recoverable complete
  timestamp in their prefix. These were not silently reconstructed.
- 141 recognized command sites occur in shell programs with parse errors;
  1,237 are under conditional/loop/list syntax. They remain candidates in the
  machine inventory, visibly flagged.
- 607 dynamic command sites and 16,493 non-shell or unresolved tool envelopes
  are not fully interpretable by the static shell pass. Many are ordinary file,
  collaboration or continuation tools. Arbitrary JavaScript, Python subprocesses,
  aliases and script contents are not executed to resolve them.
- Selected SQL, Grafana and Slack helpers were traced separately: 33 SQL helper
  sites in 13 calls, five Grafana helper sites in five calls, and one Slack helper
  call. Loop expansion is not added to provider counts. The SQL helper delegates
  to the SQL plugin; Grafana helpers perform describe/invoke around query files.
- Source pointers survive output truncation, but a retained transcript cannot
  restore provider output that the original harness never recorded. Relevant
  asynchronous continuations can be followed through their session/cell IDs;
  the inventory does not claim all background executions terminated in-window.
- Business data can contain words such as `ERROR`, `Forbidden` or `timeout`.
  Those are not automatically tool failures. A successful history query returning
  a failed deployment is successful evidence collection.

The result is an auditable extraction of recoverable usage, with representative
outcomes reviewed for each major workflow. It is not a claim to have observed
every underlying process or network effect.

### Artifacts and private traceability

- [Action inventory](evidence/recent-adapter-usage-20260909/actions.csv): normalized
  command/operation groups, counts and example evidence IDs. Action labels are
  coarse; unresolved/dynamic values do not become invented operation names.
- [Command-site inventory](evidence/recent-adapter-usage-20260909/command-sites.jsonl.gz):
  every recognized site plus the selected helpers, with timestamp, client,
  opaque evidence ID and interpretation flags. It contains no command arguments,
  result bodies, session IDs, source paths or user prompts.
- [Scan summary](evidence/recent-adapter-usage-20260909/summary.json).
- [Reviewed programmatic invocations](evidence/recent-adapter-usage-20260909/programmatic-invocations.csv):
  additional operations resolved inside submitted scripts, with separately traced
  continuation outcomes. These are examples, not inferred loop execution totals.
- [Verification and reproduction notes](evidence/recent-adapter-usage-20260909/verification.md).

`E-…` references below resolve locally through the ignored
`.local/tmp/session-intake-20260909/` extraction. Its source manifest records
per-file JSONL record digests, and tool/output indexes record exact source file/line and original call
identity. That directory is private evidence, not public documentation. Raw
credentials, customer information, private hosts and project/channel identifiers
are excluded from the tracked report. The temporary extraction tools are Rust;
no permanent project helper is introduced.

## 2. Observed workflows and specification coverage

“Specified” means selected textual/model semantics, not implemented behavior.
“Gap” includes deliberately deferred behavior that recent usage now needs. Old
command spelling is evidence, not a requirement to preserve every old flag.

| ID | Observed use case and required features | Current coverage and gap | Representative evidence |
|---|---|---|---|
| U01 | Start a local service, discover configured providers, inspect saved credentials/connections, select a target and invoke again in later sessions. Repair readiness without dumping secret values. | [Custody](../contracts/auth/custody/v1alpha1/semantics.md), [connection](../contracts/auth/connection/v1alpha1/semantics.md) and [management](../contracts/auth/management.md) specify foundations. Current env/file reads do not implement a complete persistent connect/repair/revoke CLI. Exact local management codecs, protected entry and restart behavior need selection. | E-bc181dae01bfa2de, E-ce852a934f907245, E-7fc1e03ff640dc53 |
| U02 | Enumerate kubeconfig contexts and saved endpoints; discover services and their reachability; use the intended cluster and database/monitoring connection. Diagnose missing endpoints and credential-exec failures. | [Kubernetes reads/discovery](../adapters/kubernetes/contracts/reads/v1alpha1/semantics.md) provides the small implemented slice; [discovery](../contracts/discovery/resources/v1alpha1/semantics.md) and [mediation](../contracts/discovery/mediated_route/v1alpha1/semantics.md) describe richer boundaries. CLI persistence/activation and a usable laptop-to-private-service route remain incomplete. Discovery cannot manufacture credentials. | E-5bcd5eca43849113, E-7632b58f48f4b016, E-2f6c1bd877ed554c; E-e780f751d134adf4 is a fixture |
| U03 | Diagnose Kubernetes workloads: list resources, images and conditions; inspect events; read selected pod/container logs, including previous-container and all-container requests; check per-verb permission. | The implemented four-kind inventory is partial. [Finite pod logs](../adapters/kubernetes/contracts/logs/v1alpha1/semantics.md) are specified, but previous-container and fan-out are explicitly excluded. Events, ingress/stateful/storage and other observed resources need selected native projections; a resource snapshot does not require durable event ingestion. | E-d1f3fd09b02d7ce4, E-bd592e6041a10e54, E-a10c05558431e61c, E-120eff4352f7e3ca, E-79d98e506685d215 |
| U04 | Apply/create/delete/patch/scale workloads; restart and wait for a rollout; execute in pods; copy files; hold and close a port forward; inspect Helm release history and request rollback. | [Kubernetes mutations](../adapters/kubernetes/contracts/mutations/v1alpha1/semantics.md) select restart behavior, not arbitrary apply/exec/tunnels. Execution and deployment/resource-management profiles remain deferred. Helm rendering/linting are local tool work; release operations need their own explicit semantics, not an implied Kubernetes list capability. | E-521c462ef24b4597, E-45d0cd6bcfc1dc02, E-5bf10f7267ca6836, E-17b7f1d61c293ac7, E-e79ce1a4cd8e61ba, E-bebba3b14998c83e |
| U05 | Build and inspect images, compare tags/digests/platforms, authenticate to a registry, pull/push, run disposable containers, exec/copy/log, inspect storage/cache use and clean exact resources. | [Docker design](../adapters/docker/design.md), [logs](../adapters/docker/contracts/logs/v1alpha1/semantics.md) and [mutations](../adapters/docker/contracts/mutations/v1alpha1/semantics.md) cover proposed inventory/log/lifecycle subsets. BuildKit/buildx, exec/copy, registry transfers and safe build/cache cleanup are gaps. A build tool or artifact profile may own some of these; they need not all become Engine API operations. | E-d5397b60dff753a8, E-1960f009168e7898, E-0495153b97525ccd, E-b96940406c1513d9, E-1d3b9207db28b3aa, E-043947f525239eff |
| U06 | Reconstruct incidents using read-only SQL against several MySQL/Aurora databases: joins, counts, conditional aggregation, schema knowledge and exact UTC/date ranges. Iterate query files safely and correlate rows with logs. PostgreSQL is also used for local fixtures. | [SQL reads](../adapters/sql/contracts/reads/v1alpha1/semantics.md) implement PostgreSQL, bounded read-only execution and lossless typed values. They do not establish MySQL dialect, parameters, column metadata, timezone or cancellation semantics. A MySQL native binding is required for the observed incident work. | E-1598e4e1debb7d61, E-b6e104a327d275c3, E-c624466e8167f05f, E-0352bb9d842183cb; E-a095dd7be747527c is a v2 fixture |
| U07 | Find Loki endpoints and labels/label values; narrow namespace/app; retrieve historical log lines; filter/parse LogQL; calculate counts and other metrics from logs. | [Loki log range](../adapters/loki/contracts/logs/v1alpha1/semantics.md) preserves log queries but excludes sample/metric expressions. Label discovery and a LogQL metric profile are missing. Large windows need honest truncation and a usable bounded exploration path, not silent evidence loss. | E-6b244a84e1475691, E-920c79d4b1136c51, E-4963eb3f96de7159, E-6f4b711d43317675 |
| U08 | Discover Grafana datasource UIDs; run Prometheus instant and range queries; inspect metric inventory, deployment image history and rates; batch correlated Loki/Prometheus queries with per-query identifiers. | [Prometheus range](../adapters/prometheus/contracts/series/v1alpha1/semantics.md) is specified. Instant/vector results and the used Grafana batch surface need coverage. [Grafana design](../adapters/grafana/design.md) explicitly leaves POST /api/ds/query unselected. Native child queries could satisfy part of the same use case, but only with equivalent time/result/partial-failure semantics. Grafana mediation is provider access, not mandatory Connectors federation. | E-57b5c6a7ff9f61dc, E-7dff8d336a75622f, E-c81867aa07195afa, E-02825f21c4ab24d6 |
| U09 | Find GitLab projects/groups/branches/files/tree and search code; inspect a merge request; locate a pipeline for the intended ref/SHA; poll its status; enumerate jobs and retrieve failed-job traces. | [GitLab reads](../adapters/gitlab/contracts/reads/v1alpha1/semantics.md) cover project/issues/file subsets. CI pipelines, jobs, traces, MR views, tree/search and richer discovery need selected profiles. A project-read operation cannot prove “this commit passed CI.” | E-8eb5c84a162d9136, E-49a9d4608aff7280, E-abf61dfe3a2d7e9c, E-78376247cc51bda0, E-a0b794d8feca722e |
| U10 | Create/update/merge GitLab MRs; bind merging to a SHA; inspect approval/mergeability; trigger CI and create releases. Validate an input separately from actually dispatching it. | [Mutation semantics](../contracts/operations/v1alpha1/semantics.md) supply shared effect/unknown/replay rules. Native MR, CI and release mutation contracts and reconciliation rules are not supplied by that generic document. | E-08e333f91e59e6b9, E-bc2e8fcee8cc2254, E-352b259a96de5bb1, E-b82d36090cc171e4, E-d086cb08ab384532, E-d64fa62e129599a7 |
| U11 | Search Jira using JQL across project/type/status/label/text with sorting and field selection; read issues/comments/create metadata; create and edit issues, add comments/links and run transitions. | [Atlassian design](../adapters/atlassian/design.md) proposes these mutation classes, but explicitly defers free JQL. The constrained changed-since document profile is not a substitute for the searches used here. Exact native write inputs, rich-text representation and read-back checks need coverage. | E-29228ba83dfe64b5, E-1f9613ac54c23ed5, E-1f218f3d58ab541e, E-c1d05a9314671e92, E-4b27eb6ca6637983, E-aa8129c6f00f21b6, E-7a9f1a6337d377d7 |
| U12 | Find a Slack channel, distinguish membership/private/archive state, page channel/user listings, read history and complete threads, resolve users, post a message or thread reply after investigation. | No Slack-owned adapter specification directory exists in this baseline. Generic catalog/auth/mutation contracts are foundations, not a thread/message contract. Preserve string timestamps/thread identity, pagination, bot/user/app credential differences, grant checks and actual message outcome. | E-b030180014b3b281, E-218e348ad289afc6, E-2b7c1472805fd354, E-7eba6f3e2f62b699, E-dc505b4fcc874bfb, E-04b0e34fe28fe511 |
| U13 | GitHub repository/PR review and merge, Actions runs/jobs/checks/logs/artifact downloads, workflow dispatch/rerun/cancel, releases, branch/ruleset inspection and some repository/configuration changes. | Actual native CLI use, outside the current three-adapter milestone. A GitHub-owned profile or explicit retained client boundary is needed before claiming full recent-use replacement. Release artifacts are not ordinary JSON records; automation identity must remain explicit. | E-0187f7cb3ac54d06, E-03b4961bed788917, E-00491756fe6292f6, E-01b79a976e79ec6b, E-9f73a83b97213ade, E-a672c443cfb1defa |
| U14 | Select AWS profiles, inspect caller identity, renew an expired SSO session, inspect ECR image digests/manifests and an EC2 volume. Kubernetes credential exec can depend on the same local AWS setup. | No selected AWS adapter profile. Generic HTTP-header credentials do not implement AWS signing/session acquisition. Separate use of AWS as a provider from AWS-backed Kubernetes authentication. A fresh browser login and a valid usable profile are distinct facts. | E-107daa8c1d7958bd, E-85825dcf2af501c5, E-d7894a55ce2425ea, E-38df729d3bcfc2a5, E-8b0b14f8999a25d1, E-7632b58f48f4b016 |
| U15 | Ask the CLI what it supports, inspect exact input schemas, discover configured versus callable integrations, and understand a capability gap before falling back to another client. | [Catalog](../contracts/catalog/v1alpha1/semantics.md) and service metadata address parts. The local CLI must expose installed/declared/configured/authorized/reachable separately, with a stable machine-readable discovery path and useful human help. Describing an operation is not proof of a completed workflow. | E-51ed557d4b7c36a0, E-709700332467ef6b, E-4662a718aae7521c, E-4996fd0f48a94d56, E-32ad1679571b9ad9 |
| U16 | Enumerate Confluence spaces and search page bodies using CQL, modification dates and multiple spaces; collect changing Jira issues/comments and GitLab projects/issues/MRs/pipelines/deployments/commits. Preserve required operations across a CLI upgrade. | [Confluence documents/CQL](../adapters/atlassian/contracts/documents/v1alpha1/semantics.md) already specify much of scoped body search; prove the actual query shapes, space discovery and paging. GitLab incremental profiles and native date-filter constraints remain gaps. Consumer progress/checkpoint ownership is separate from a read cursor; preserving a protocol version alone does not guarantee the required operation set survives an upgrade. | E-4079606d4f59eab4 → E-6ea947707959e271; E-c75be4cc6b48330b → E-d5df47cd68220570; E-91966d319a5fedaa → E-b32c3fa959c19b35; E-3eb55d88d9de2383, E-6369eaa6256ff5ec |

Confluence was absent from direct shell invoke sites but present in successful
programmatic calls. This is why the shell inventory alone is not the coverage
oracle. Alertmanager, SIP/media and many other catalog names appear in
documentation, searches or development context; this study does not turn those
mentions into successful provider use. The observed `event search` attempt failed
at hosted authentication (E-7fc1e03ff640dc53); it is a discovery/readiness use case,
not evidence of successful event ingestion. Likewise, Docker Compose, continuous
log follow and Kubernetes watch were not established as direct executions by this
pass. They should not be added to the minimum on keyword evidence alone.

The cross-adapter use case is especially important: establish a time window from
a conversation or ticket, inspect logs and metrics, corroborate with SQL and code,
check the relevant deployment/CI revision, then write the finding back to Jira or
Slack. Each step must retain target and evidence identity. This composition can
run in the local CLI or an agent workflow; it does not require a federating host.

## 3. Reviewed failure modes and their implications

| ID | Observed failure or friction | Required behavior |
|---|---|---|
| F01 | Ordinary search/list/event commands selected a hosted path and failed because Identity could not issue access (E-7fc1e03ff640dc53). | Explicit local mode works without hosted identity. Report the selected mode and configuration origin safely. A saved hosted login must not redirect local work. |
| F02 | A saved endpoint reference did not exist; Kubernetes selected an unreachable loopback endpoint (E-5bcd5eca43849113, E-048beb6657d467d6). | Distinguish missing registration, wrong target, unreachable transport and provider refusal. List available admitted references without guessing a replacement. |
| F03 | AWS SSO expiry prevented identity use; Kubernetes credential exec failed (E-85825dcf2af501c5, E-7632b58f48f4b016). | Identify the failing credential provider/profile and safe repair action. Persist successful repair locally; re-check the intended identity. Discovery/help must not unexpectedly run interactive credential exec. |
| F04 | Slack admin search had no credential satisfying its declared mechanisms; another connection admitted reads only (E-5a615d6999455051, E-4996fd0f48a94d56). | A stored credential is not permission for every operation. Distinguish provider mechanism/scope from local read/write grant; preserve both checks. |
| F05 | Guessed operation names failed; invocation omitted connection/description selectors or used the wrong JSON input flag (E-51ed557d4b7c36a0, E-4662a718aae7521c, E-ae5eb26066ecf3e2). | Discover exact operations and schemas; make the complete invocation journey discoverable. Accept well-defined file/stdin input. Preserve admission checks while avoiding repeated manual plumbing. |
| F06 | Pipeline input rejected `per_page`; project ID required a string while pipeline ID required a number. Correcting each type produced results (E-da3a10b07179c07e, E-df88a04a257d228a, E-8eb5c84a162d9136, E-8a242d2a0aec86a3, E-49a9d4608aff7280). | Public schema, decoder and examples agree exactly. Use typed native identifiers and field-specific errors; do not silently coerce identifiers or drop unknown fields. |
| F07 | SQL contained URL-encoded quote text and MySQL returned syntax error 1064 (E-1598e4e1debb7d61). | Transport JSON/query bytes once; use native parameter binding where supported. Report safe dialect/field errors without echoing customer query contents. |
| F08 | A helper assumed an object-shaped result, but received a list and crashed on `.get` (E-8afcd6cea0a9768b). | Stable outer result/error envelope with explicit inner shape; discovery must describe result variants. Consumers need not guess between raw lists, `.result`, `.output`, and provider `ok`. |
| F09 | Channel enumeration and pod logs overwhelmed the tool output budget; callers used head/grep/cut or ad hoc query wrappers (E-b030180014b3b281, E-bd592e6041a10e54). | Bounded pages, safe projections and explicit completeness/truncation. Machine output must remain valid at the CLI boundary. Local output files/artifacts must not silently weaken provider limits or disclosure. |
| F10 | A shell tool reported success while its JSON contained a validation or permission error; piping could hide exit status (E-da3a10b07179c07e, E-4662a718aae7521c). | The CLI returns a nonzero status for refusal/failure and a typed machine error. Wrappers inspect both layers. Empty filtered output is never proof that a read succeeded. |
| F11 | Rollout restart acknowledged a change, then continued waiting; Helm history showed failed/timed-out revisions (E-521c462ef24b4597, E-e8f4cb2bf97d163e). | Separate acceptance, progress, completion and timeout/unknown. Observe the same target/revision after interruption; do not resend a mutation because a wait timed out. |
| F12 | GitLab merge validation returned `valid: true` while a subsequent execution was still asynchronous (E-bc2e8fcee8cc2254). | Validation/dry run grants no success claim. Preserve the exact revision and actual dispatch/result identity through polling and read-back. |
| F13 | Grafana batches and Loki queries mix log streams, vectors and matrices; logs themselves contain service errors (E-02825f21c4ab24d6, E-6f4b711d43317675). | Explicit result variants and per-query outcomes. Query failure, partial response, empty data, and a successfully retrieved application error are different results. |
| F14 | An upgraded CLI candidate lacked required Jira comment and Confluence search operations; the session explicitly rolled it back (E-6369eaa6256ff5ec). | Validate the consumer's required operation/schema/connection contract before replacing its binary or adapter bundle. A newer version is not evidence of compatible capability coverage. |
| F15 | In a multi-source read batch, GitLab deployment collection returned HTTP 400 while adjacent issue/MR/pipeline/commit reads succeeded (E-d5df47cd68220570). | Model native filter/ordering constraints and preserve independent outcomes. A failed source page cannot advance collection progress or erase successfully collected sources. |

These are reviewed examples, not failure-frequency estimates. In particular,
there is no defensible global success rate from shell exit status or error-word
counts in this corpus. Historical fallbacks document missing capability or
unusable interfaces; they do not authorize automatic fallback in the replacement.

## 4. Minimum invariants for the local product

These are proposed acceptance requirements derived from the evidence, except the
local CLI/persistent local credentials/no mandatory federation direction, which
is explicit operator intent. Existing shared semantics remain authoritative until
a scoped profile change is reviewed.

1. **Local independence.** Bootstrap, inspect, select and invoke local adapters
   without a hosted Connectors login. Provider authentication can still contact
   the provider. Management metadata remains inspectable when provider access fails.
2. **Persistence with a truthful acknowledgement.** Once connect/repair reports
   success, a fresh CLI process and restarted local owner recover the same saved
   connection and usable credential association. A failed store write cannot
   report success. Volatile test custody is never advertised as persistent.
3. **Replaceable custody.** Local metadata and sensitive material have separate
   responsibilities. Select and specify a local backend and its locking,
   durability, permissions and locked/unavailable behavior. No silent plaintext,
   environment or alternate-account fallback. Provider code does not depend on
   the backend's paths or layout.
4. **Stable target and identity.** A default context, endpoint label, datasource
   UID or repaired token cannot silently move work to a different account,
   cluster, database or principal. Explicit selection and safe provenance survive
   discovery, paging, retries and workflow composition.
5. **Credentials stay in the protected boundary.** Ordinary arguments/results,
   discovery, logs and model-visible auth status contain no secret values or
   actionable completion authority. Specify a practical local terminal/browser
   entry path, consistent with the existing management boundary. Local execution
   does not require sending credentials through a remote coordinator.
6. **Readiness is layered.** Installed, declared, configured, credential present,
   credential usable, locally permitted, provider permitted and reachable are
   distinct. A failure has an actionable safe reason; an empty result has evidence
   of successful execution under the selected scope.
7. **Schema agreement.** A discovered schema, accepted input, decoded native
   request and result shape agree. Preserve string IDs and timestamp precision;
   no inferred coercion or query rewriting changes meaning.
8. **Machine output is dependable.** One documented result/error envelope, valid
   machine output, useful nonzero exit codes and separate progress/diagnostic
   behavior. Shell composition never has to infer success from silence.
9. **Bounded evidence remains honest.** Pagination, provider saturation, local
   clipping, byte ceilings and retained cursors are visible. “No matches,” “no
   permission,” “unavailable,” and “not searched completely” remain distinct.
10. **Time semantics are explicit.** Declare instant versus range, start/end
    inclusivity, UTC conversion, timestamp precision, step, relative-time capture
    and database timezone. A query execution date does not restrict the age of
    data it may investigate.
11. **Composition retains provenance.** Each result identifies the selected
    connection/operation and safe observation coordinates. Correlating SQL, logs,
    metrics and CI is a client workflow, not a shared customer-specific domain.
12. **Writes remain intentional.** Read credentials/grants do not admit writes.
    Bind destructive changes and merges to stable targets/revisions; report
    provider conflicts. Local policy can authorize work without a cloud approval
    service, but its exact profile must be specified rather than bypassed.
13. **Uncertainty never becomes a duplicate effect.** Validation is not dispatch;
    dispatch acknowledgement is not completion. Lost replies, timeout and process
    restart do not authorize replay of a write. Observe/reconcile exact attempts.
14. **Execution has a lifecycle.** For selected exec/build/tunnel profiles, specify
    argv/stdin, output streams or artifacts, exit status, deadline/cancel behavior,
    target identity and ownership/cleanup. Do not stretch a 15-second unary read
    into an unbounded process or claim resumability without retained state.
15. **Provider ownership survives extraction.** MySQL dialect, JQL, LogQL,
    Kubernetes resource rules, Slack thread semantics and registry protocols live
    with their adapters. Shared contracts describe reusable data/effect/lifecycle
    guarantees. No provider names or customer semantics leak into shared ESS domains.
16. **Upgrades preserve selected consumer contracts.** Pin the actual artifact and
    required operation/profile/schema set, check compatibility before activation,
    and keep a recoverable previous binding. A version string or successful help
    command is insufficient; never silently remove an operation a consumer needs.
17. **Incremental collection is truthful.** Declare date-filter/ordering and
    continuation semantics, including live-versus-snapshot behavior. A source
    failure or incomplete page supplies no evidence that its change window was
    consumed. Durable consumer checkpoints stay with the consumer unless a
    separate contract explicitly assigns their ownership.

A later remote binding can reuse these interfaces and workflows. It still needs
its own authenticated transport, delegation and credential-locality conformance;
“another plug” is the architectural boundary, not proof that remote authority
semantics arise automatically. None of that future delivery is required to finish
the local baseline.

## 5. Recommended specification order

The previous milestone closed a bounded shared model for three adapters. This
evidence changes the acceptance target; it does not retroactively make that
milestone a promise of complete CLI parity. The following is proposed order,
not newly created implementation stories or an implementation wave.

| Priority | Specification work | Completion evidence |
|---|---|---|
| P0 | Select the local CLI binding: configuration/default selection, adapter launch/readiness, persisted connection management, local credential backend and protected entry, failure envelope and local read/write admission. Reuse existing custody/connection owners. | C01–C05 below, with restart/store-failure/identity-switch cases and no cloud identity dependency. |
| P1 | Close observed read workflows: MySQL, GitLab CI/jobs/traces/MR and incremental reads, Jira JQL, Confluence CQL/documents, Slack history/threads/discovery, Loki labels and metrics, Prometheus instant and Grafana batch equivalence, richer Kubernetes diagnosis. | C06–C12 and C21–C22; every required native operation has a selected input/result/permission/limit contract and fixtures. |
| P2 | Close observed writes and execution: Slack/Jira/GitLab writes, Kubernetes resource changes/exec/copy/tunnels, Docker build/registry/runtime/cache work; place Helm and artifact behavior with an explicit owner. | C13–C17, including unknown outcomes, progress, interruption and exact-target cleanup. These are required for full parity, not optional because sequenced later. |
| P2 | Add an explicit disposition for the observed GitHub and AWS workflows. Specify selected adapter profiles or document an intentionally retained client boundary; do not claim replacement while silently excluding them. | C18–C19 and a visible coverage decision for every observed family. |
| P3 | Revisit discovery-only/mentioned capabilities and future remote bindings after the local target is usable. | Positive usage or explicit product requirements before adding scope; no invented evidence for media or event ingestion. |

For each selected gap: refine the owning textual contract first; model changed
typed values, entities and unresolved relations through ESS; add valid/invalid
examples and executable lifecycle scenarios where ESS supports them; then review
and decompose runtime work through AEP. Do not add a new persistent entity merely
because a command needs a name. Pure query variants may need only typed values
and a native profile. Storage atomicity, credential behavior, actual network
effects and output correctness also need implementation conformance fixtures;
schema validation and simulated state transitions alone cannot prove them.

## 6. Concrete acceptance scenarios

Use synthetic provider/customer values and controlled fixtures. These scenarios
are **specified next work**, not tests run by this analysis.

| ID | Scenario and required observable result |
|---|---|
| C01 | Fresh local setup → connect a provider through protected input → invoke a read → exit CLI and restart its local owner → invoke with the same saved connection. No hosted identity request; no re-entry of a still-valid secret. |
| C02 | A hosted login exists, then the operator selects local mode. Local discovery/invocation remains local and reports its selected configuration; hosted auth outage has no effect. |
| C03 | Locked/unavailable custody, failed durable write and crash between write/publication. Status identifies the cause; no false connected result or unintended credential fallback; previous valid binding is preserved where the contract permits. |
| C04 | Repair expires/replaces credentials. Same-identity renewal preserves connection semantics; a different principal/account/target requires the declared replacement action. Revocation survives restart. |
| C05 | Use discovered schemas and stdin/file JSON to invoke. Wrong field/type/operation, missing permission and unreachable service produce distinct typed errors and nonzero status; progress does not corrupt machine output. |
| C06 | MySQL incident query uses parameters, joins, grouping and UTC bounds against an explicitly selected read-only database. Verify native types/nulls/precision, quote handling, row limits, cancellation and blocked writes. Repeat against PostgreSQL without pretending their dialects are identical. |
| C07 | Discover Loki labels/values, select a scope, fetch a log range, then derive a count series from those logs. Preserve LogQL semantics, interval/precision, duplicates and honest saturation; metric results cannot masquerade as log lines. |
| C08 | Discover Grafana datasources, perform Prometheus instant and range queries, and batch a log query with metric queries. Preserve query IDs, independent failures and exact timestamps; compare any replacement child-query composition against equivalent results. |
| C09 | Find the pipeline for an exact GitLab commit, wait through pending/running, enumerate jobs and read a failed trace. A successful pipeline for another SHA cannot satisfy the check. Truncated traces remain marked. |
| C10 | Jira search combines project/status/type/text/labels and sort/field selection; page through results and fetch issue/comments. A constrained changed-since search cannot silently replace the requested query. |
| C11 | Slack channel lookup traverses pages, distinguishes membership/archive state, then reads a thread with multiple reply pages and resolves users. String message/thread timestamps remain exact. |
| C12 | Kubernetes diagnosis reads additional selected resource kinds, conditions/events and requested current/previous/container logs. Permission refusal is not empty success; multi-container selection is explicit and bounded. |
| C13 | Post a Slack thread reply and create/edit/comment/link/transition a Jira issue under local write admission. Bind the exact target/input and verify the observed outcome; a lost reply cannot create duplicate messages/issues automatically. |
| C14 | Validate then create/update/merge a GitLab MR for a pinned SHA; refuse a changed head or missing required checks. Validation alone reports no merge; a lost merge response requires exact-target observation. |
| C15 | Apply a selected Kubernetes change, observe rollout status, time out or interrupt, and inspect the same revision afterward. Keep change acceptance separate from readiness; inspect Helm history and safely target a selected rollback. |
| C16 | Execute in a selected pod/container with stdin and separate output/exit status; copy a bounded artifact; open a loopback port forward and terminate the owning session. No orphan tunnel, arbitrary retargeting or false resume after owner loss. |
| C17 | Build an image for a selected platform, inspect its immutable digest, push/pull through saved registry auth, run and inspect it, and clean only admitted exact resources. Build/progress/output and cache-capacity failures remain truthful. |
| C18 | Inspect GitHub PR/checks and exact Actions run/jobs/logs, retrieve a bounded artifact, dispatch a selected workflow, and perform an admitted merge/release action with explicit automation identity. |
| C19 | Select an AWS profile, detect expired SSO, repair through protected interaction, verify caller identity, and inspect ECR/EC2 resources. Kubernetes exec credentials must use the intended profile and report repair failure without revealing token material. |
| C20 | Run the complete incident workflow locally: Slack/Jira context → logs and metrics → MySQL evidence → code/CI revision → an explicitly requested ticket update/thread reply. Retain safe provenance at every step; unrelated provider failure does not erase successful evidence or authorize a fallback account. |
| C21 | Enumerate admitted Confluence spaces, issue a multi-space modification-date CQL search, retrieve versioned storage-format bodies and follow a result cursor. Collect changed Jira/GitLab records separately; a rejected deployment filter leaves that source incomplete and its consumer checkpoint unchanged. |
| C22 | Upgrade a local CLI/adapter against a consumer requiring Jira comment reads and Confluence page search. A candidate missing either capability or changing a required schema is refused before activation; the existing usable binding remains available. |

The local parity goal is reached when each selected scenario has reviewed textual
semantics and applicable ESS declarations/scenarios, a concrete adapter/binding
owner, and a passing runtime fixture for the advertised behavior. Required used
workflows cannot be counted as complete by marking them deferred or unsupported.
An explicit operator scope reduction can define a smaller milestone; until then,
the three-adapter slice and full recent-use replacement remain distinct targets.

## 7. Additional explicit requirement: MCP

During this analysis the operator separately requested full MCP contract coverage:
outbound authenticated MCP connections from the CLI, and inbound MCP exposed by
`$BIN server` to local clients or from a cloud deployment. This is tracked as
[epic:mcp-contracts](../.engineering/planning/epic/mcp-contracts.md), informed by
this study and the existing design review. It is explicit product direction,
not a claim that the five-day command inventory proves either binding works.
The epic requires selected protocol/transport/auth/capability mappings, applicable
ESS modeling and conformance before implementation; cloud serving remains optional
for the local credential-persistence baseline.
