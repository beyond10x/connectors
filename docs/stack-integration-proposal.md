# Proposal: extensions and adjustments for stack integration — 2026-09-08

**Local-stage update, 2026-09-09:** this dated proposal retains its hosted stack
dependencies. The selected [local CLI contract](../contracts/cli/v1alpha1/semantics.md)
uses local OS-keyring custody and private metadata without Identity, hosted Secrets
or federation. Its planning owners are `story:local-cli-binding-semantics` and
`story:local-cli-ess-surface`; the historical S8 proposal below does not block them.
MCP and Atlas enrollment remain deferred. Hosted admission and custody retain
their own contracts when that deployment binding is selected.

Baseline: local commit `57be07c` plus the uncommitted working tree of 2026-09-08. Source of the
findings: [preserved concept and stack integration review](../.engineering/planning/review-result/concept-stack-integration-20260908.md) (gaps G1–G10; originally `.local/review/2026-09-08-concept-and-stack-integration-review.md`).
Organization authority read from `../atlas` (README/ROADMAP dated 2026-09-04; ADRs 0001–0031).
This is a proposal. It creates no planning-store artifact, edits no existing contract, and does
not register anything in Atlas. Atlas integration remains deferred by the operator (`AGENTS.md`).

## 1. What is proposed, in one table

| # | Kind | Item | Closes | Where |
|---|---|---|---|---|
| P1 | new contract document | `service/v1alpha2` governed binding: verified context, admission profiles, policy/audit ports, `not_granted`/`stale_authority`, curation fields on every operation, delegated federation context | G1, G2, G3, G6, G7, G9 | [contracts/service/v1alpha2/semantics.md](../contracts/service/v1alpha2/semantics.md) |
| P2 | new user document | CLI migration 0.7.x → v2, with non-parity stated per boundary | operator request | [docs/cli-migration-v1-to-v2.md](cli-migration-v1-to-v2.md) |
| P3 | adjustments to existing documents | ADR citations, index rows, consumer table, seam rows | G5, G8 | § 3 below, applied under the stories that own those files |
| P4 | stories | eight stories, ESS-first where an entity appears | all | § 4, ready for `aep plan artifact create` |
| P5 | Atlas plan | records to create when integration is authorized; no crate rename required | G10 | § 5 |
| P6 | consumer order | which repository moves first and why | — | § 6 |

## 2. Principles the proposal keeps

- Authentication establishes trusted tenant/principal, optional realm/authority and any credential-bound executor before payload decoding. Policy then verifies a decoded executor assertion before constructing admitted context; the assertion never supplies authority itself ([service compatibility](../contracts/service/compatibility.md), ADR 0026, `docs/design.md` § 7).
- The host is the only place admission lives; adapters see a resolved context, not headers
  (`docs/design.md` § 3.2; `crates/connectors-host/src/server.rs:57-70` today).
- Metadata informs admission and UX; it authorizes nothing (`docs/design.md` § 7;
  `contracts/operations/v1alpha1/semantics.md`, "Authority boundary").
- One explicitly selected codec per interaction; a binary may support several. Selection precedes describe and refuses incompatibility without fallback or resend ([service compatibility](../contracts/service/compatibility.md)); the current binary supports only v1alpha1.
- A discovered observation never dials, authenticates or materializes
  (`contracts/service/v1alpha1/semantics.md`, Kubernetes paragraph).
- Connectors owns its audience and scope bytes; Identity stays relying-party agnostic (ADR 0021).
- Secrets is the shared custody service; connector-created values are non-revealable (ADR 0023).
- Generated services reach Connectors as ordinary external factories; Connectors has no
  dependency on ESS, service-sdk or a UI (ADR 0027, 0029).

## 3. Adjustments to existing documents

Every row names the file, the exact place, the change, and the constraint that decides when it
can be applied. Several files are cited by line number from draft stories (baseline `db1c329`),
so inserting lines shifts those citations. Those edits wait for the story that owns the file.

| File | Place | Change | Why | Constraint |
|---|---|---|---|---|
| `docs/design.md` | § 7, paragraph "For the Beyond10x service binding…" | add: "Specified by `contracts/service/v1alpha2/semantics.md`. Organization rules: ADR 0026 (realm), ADR 0031 (trusted access exchange), ADR 0021 (audience ownership)." | G1/G2; design currently cites 0 ADRs (`grep -c ADR docs/design.md` = 0) | cited by line from `story:contracts-host-composition` (`docs/design.md:158, 253, 798`) and others; apply when that story serializes, or append at end of § 7 only |
| `docs/design.md` | § 12.3 secret-store contract | add: "The hosted binding is the released Secrets client under ADR 0023; Vault is the temporary rollback source, not a v2 binding." | G4 | same |
| `docs/design.md` | § 16 federation, item 3 | add: "The `delegated` profile follows the complete F03 proposal in `contracts/service/delegation.md`: canonical preparation, exact receiver/request proofs, durable delivery nonce and leaf-only approval redemption. Advertisement awaits implemented conformance; a per-route signature sketch is insufficient." | G1 | same |
| `docs/design.md` | § 18.3 or § 4.1 | add a row: generated adapters from service-sdk lower a `ConnectorServiceFactoryDescriptor` into an adapter/v2 instance and a generated `Adapter`; Connectors takes no dependency on service-sdk (ADR 0027, 0029). | G5 | same |
| `docs/design.md` | § 19, last paragraph on MCP | replace "MCP can be a client-facing binding and/or an adapter" with the ADR 0028 split: inbound `/mcp` is a host transport binding onto already-governed operations; outbound MCP is an adapter over `EgressTransport` with a strict frozen tool snapshot. | G8 | same |
| `docs/design.md` | § 2.4 consumer table | add rows: service-sdk `service-connectors` at `235558c…` (`protocol`, `service`); org-brain intended, no pin. Add a note that Atlas `docs/catalog.md:595` records devcenter-http at `v0.5.3` while `../devcenter/Cargo.toml:37` pins `e80b7ae…` `=0.7.0`. | accuracy | same |
| `docs/design.md` | § 24 open decisions, row "Hosted read admission" | set the default: fail closed; policy `Unavailable` is `unavailable` with no dispatch. | G3 | same |
| `docs/design.md` | § 25 | add the Atlas registration path of § 5 below as operational context, still deferred. | G10 | same |
| `contracts/README.md` | index table | add row: `operations/v1alpha1` … `service/v1alpha2` binding — [service/v1alpha2](../contracts/service/v1alpha2/semantics.md) — proposed — verified context, admission profiles, policy/audit, `not_granted`/`stale_authority`, curation fields on every operation. | P1 | cited by line from `story:contracts-documentation-index` (`:29, :36`), `story:contracts-acquisition-profiles` (`:47`), `story:contracts-media-controls` (`:52`); apply under the documentation-index story |
| `contracts/README.md` | new section after "Which adapter needs which contract" | "Which consumer needs which contract" table (§ 6 below). | G5–G7 | same |
| `contracts/operations/v1alpha1/semantics.md` | § 7 compatibility | reference `service/v1alpha2` as the owner of the envelope that carries the mutation fields; keep the projection rule. | consistency | file is uncommitted and in flight under `story:contracts-mutation-outcomes`; hand to that story |
| `contracts/auth/connection/v1alpha1/semantics.md` | § 4 selection rule; § 9 UNMAPPED row | cite `service/v1alpha2` § 4 rule 4 as the scope rule; keep the entity relation UNMAPPED. | G1 | owned by `story:contracts-management-boundary` |
| `contracts/datasources/records/v1alpha1/semantics.md` | error section | state that `not_granted` and `stale_authority` from `service/v1alpha2` replace old `DatasourceErrorCode::{NotGranted, StaleAuthority}`. | G7 | owned by `story:contracts-wire-compatibility` |
| `spec-kinds/adapter/v2/semantics.md` | implementation binding | add realization kind `generated-service` whose `prepare`/`finish` are emitted by service-sdk; obligations remain visible in `coverage.json`. | G5 | no line citations found; can be applied with `story:descriptor-curation-fields` |
| `AGENTS.md` | Workspace operations | none now. When publication is authorized: add `b10x.docs.yaml` (if public), record the gate program for Atlas. | G10 | operator decision |

## 4. Stories

Not created. Each row is the input for `aep plan artifact create` under
`epic:contract-semantics-remediation` or a new epic `stack-integration`; the operator chooses.
`AGENTS.md` requires ESS modeling before decomposing implementation work that introduces an
entity, so S2 precedes S3–S7.

| Id | Title | Priority | Depends on | Scope (paths) | Acceptance |
|---|---|---|---|---|---|
| S1 `contracts-governed-binding` | Review and settle the governed service binding v1alpha2 | P1 | `contracts-wire-compatibility`, `contracts-mutation-outcomes`, `contracts-management-boundary` | `contracts/service/v1alpha2/semantics.md`, `contracts/README.md` | every proposed field, code and profile has a wire disposition in the compatibility matrix; § 6 scenarios have expected observations |
| S2 `ess-governed-values` | Model `AuditRecord`, the `VerifiedContext` value and the `GrantRecord` decision in ESS | P1 | S1 | `ess/domains/`, `ess/system.yaml` | `ess validate` passes; unresolved relations carry UNMAPPED, none guessed |
| S3 `host-admission-profiles` | Implement the `Admission` port with `static-bearer`, `identity-audience` (feature `identity`, pinned `identity-client`) and `delegated` | P1 | S1, S2 | `crates/connectors-host/src/server.rs`, new `admission.rs`, `crates/connectors-core/src/lib.rs` | the selected authentication, executor and refusal requirements in `service/v1alpha2` § 6 pass as fixtures (earlier numbered scenarios were proposal references); adapters build unchanged with `--no-default-features` |
| S4 `host-policy-audit-ports` | `Policy` and `Audit` ports, fail-closed, local bindings | P1 | S2, S3 | `crates/connectors-host/`, `crates/connectors-sdk/src/lib.rs` (`InvocationContext`) | named policy-outage, executor/realm admission, pre-dispatch audit failure, final-audit failure and safe-correlation requirements in `service/v1alpha2` § 6 pass (old scenario numbers are historical); audit output contains no credential bytes |
| S5 `descriptor-curation-fields` | `effects`, `semantic_effects`, `risk`, `idempotency`, `approval` on every operation; compiler refusals; agent-platform projection vectors | P1 | S1, `contracts-mutation-visibility` | `crates/connectors-core`, `crates/connectors-spec`, `spec-kinds/adapter/v2/schema.json`, `adapters/*/spec`, `adapters/*/generated` | three descriptors regenerate with the fields; a fixture compiles them into the old `OperationDescription` shape without loss |
| S6 `host-secrets-custody` | `SecretStore` over the released `secrets-client` behind feature `secrets`; non-revealable connector values | P2 | S2, `contracts-persistence-ownership` | `crates/connectors-host/src/credentials.rs`, new `secrets.rs` | file, memory and Secrets bindings pass the same port tests; no plaintext in any log |
| S7 `federation-delegated-context` | Sign, forward and verify the delegated context; return both audit references | P2 | S3, S4, `contracts-federated-approval` | `crates/connectors-host/src/federation.rs` | F03’s selected preparation/approval/delivery binding fixtures pass; replay, expiry, wrong receiver/body and unconfigured route refused; old scenario numbers are historical proposal references |
| S8 `cli-governed-surface` | Identity-token source, executor assertion, output modes; verbs decided and recorded in the CLI document | P2 | S3 | `apps/connectors/src/main.rs`, `docs/cli-migration-v1-to-v2.md` | the "not decided" list in the CLI document is empty or each item has a recorded decision |

Cross-repository work this repository does not own, recorded here so the arrows exist when Atlas
integration is authorized:

| Owner | Work | Depends on |
|---|---|---|
| service-sdk | emit a v2 `Adapter` and adapter/v2 spec instance from `ConnectorServiceFactoryDescriptor`; enforce the generated realm policy through the host's admission | S3, S5 |
| workspace | move from `HostedClient` + `DatasourceBinding` to the v2 client; map `not_granted`/`stale_authority` | S3, S4 |
| agent-platform | compile v1alpha2 descriptors; send the executor assertion; invoke under the exchanged Identity token | S3, S5 |
| devcenter | BFF over the v2 client; `devcenter-connectors` composition over v2 host wiring; Secrets binding | S3–S7 |
| zwirn | replace the `connectors-cli` embed (`../zwirn/Cargo.toml:25`, `1e0eb9f`) with the v2 client | S8 |

## 5. Atlas registration plan (when authorized)

Atlas component ids are repository-qualified (`connectors/connectors-client`,
`../atlas/docs/catalog.md:221`), so a second repository can carry a crate of the same name
without an id collision. Cargo consumers that pin both repositories during migration use
`package = "connectors-client"` renames. **No crate rename is required.** This retracts the
crate-rename suggestion in the review's § 6 item 4; the lineage record is the mechanism instead.

| Record | Definition | Content |
|---|---|---|
| `atlas.repository` | `catalog/definitions/repository.yaml` | id to be chosen (working name `connectors-v2`); `layer: platform`; `forms: [library, service, cli]`; `visibility: private` first (ADR 0027 precedent: both new repositories began private); `objectives: [O1, O5]` (matches `AGENTS.md` § Serves); `gate_program: cargo`, `gate_args: [run, --locked, -p, connectors-build, --, gate, --msrv]` with `TMPDIR` set as the README states |
| `atlas.release-unit` | `release-unit.yaml` | `<repo>/default`, tagged GitHub assets like `connectors/default` |
| `atlas.repository-lineage` | `repository-lineage.yaml` | now: `kind: extract`, sources `[connectors]`, targets `[<repo>]`, evidence `docs/design.md § 1`; at cutover: a second lineage `extract-and-retire` or `rename`, with `bind-github-redirect` if the GitHub name moves |
| `atlas.component` × 12 | `component.yaml` | 8 crates, 3 adapters, 1 app, each backed by its `Cargo.toml` |
| `atlas.repository-relation` + `atlas.component-dependency` | per consumer | one pair per pin as consumers move; `catalog refresh` refuses an undeclared pin (`../atlas/AGENTS.md`), so the relation is declared in the same change as the consumer's `Cargo.toml` |
| ROADMAP rows | `ROADMAP.md` § The arrows | `identity → <repo>` intended until S3 releases; `secrets → <repo>` intended until S6; `<repo> → org-brain` intended; consumer arrows move from `connectors` rows as each migrates |
| `b10x.docs.yaml` | `atlas docs reconcile` | only if the repository becomes public |

## 6. Which consumer needs which contract

| Consumer | Pin today | Uses | Needs from v2 |
|---|---|---|---|
| service-sdk `service-connectors` | `235558c…` (`../service-sdk/crates/service-connectors/Cargo.toml:25-26`) | `ConnectorBackend`, `ConnectorServiceFactory`, `PrincipalContext`, `ServiceDeployment` | `Adapter` target; `service/v1alpha2` admission with realm policy; curation fields |
| workspace | `b4f6d65…` (`../workspace/Cargo.toml:21`) | `HostedClient`, `OwnerContext`, `DatasourceBinding`, `NotGranted`, `StaleAuthority` | `service/v1alpha2`; `datasource.records`; `auth.connection` (`configured` is enough for repositories and branches) |
| agent-platform | tag `v0.3.1` (`../agent-platform/Cargo.toml:44`) | `OperationDescription` projection; `connectors-client` invocation | curation fields on every operation; executor assertion; Identity-audience admission |
| devcenter | `e80b7ae…` `=0.7.0` (`../devcenter/Cargo.toml:37-38`); `097b1c5…` for the composition crate | `connectors-client`, `protocol`, `connectors-runtime`, `service` | everything above plus Secrets custody and the hosted policy binding |
| org-brain | none; intended (`../org-brain/docs/design/org-brain-v0.1.md:39, 70`) | operations, datasources, `connectors serve` | `service/v1alpha2`; `datasource.records`; the CLI serve mode |
| zwirn | `1e0eb9f` (`../zwirn/Cargo.toml:25`) | `connectors-cli` as a library | v2 client library; no CLI embed |

Recommended order: service-sdk/Todo (no live traffic; factory is inert until a deployment
overlay exists, ADR 0029), workspace (reads only, 1 crate), agent-platform (projection then
invocation), devcenter (BFF and composition, needs S6), zwirn (last; ROADMAP marks its Identity
seam "unverified"). `docs/design.md` § 23.2: each consumer is evaluated against its actual pin;
`docs/design.md` § 23.3: no state is migrated by copying secrets; connections are re-established
through acquisition flows.

## 7. Decisions

| Fork | Default on silence | Alternative and its cost |
|---|---|---|
| D1 Separate Atlas repository with an `extract` lineage, migrating consumers one by one | separate repository | replace `connectors` main in place: all four pinned consumers break at once, against `docs/design.md` § 23.2 |
| D2 Hosted policy binding owned by Connectors (as `domain/grant.rs` was) | Connectors-owned | a separate grants service: none exists in the Atlas catalog; inventing one is out of scope |
| D3 v2 CLI stays uninstalled (`target/debug/connectors`) while 0.7.x is the installed `connectors` | uninstalled until S8 decides the name | install under another name now: a name chosen before S8 becomes a compatibility promise |
