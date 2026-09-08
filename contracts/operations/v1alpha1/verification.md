# Mutation outcome hardening — verification boundary

Owner: `story:contracts-mutation-outcomes`. Source findings: **F01, E01, E11** in `specification:contract-review-intake-20260908`. The source reviews remain immutable, anchored to `db1c329`; hardening starts after checkpoint `57be07c`.

This is a proposed semantic contract and ESS model. It does not implement mutations, a new wire codec, approval redemption, or a durable ledger. The existing read-only host is not described as an already-broken mutation implementation.

## What is checked now

- [Normative rules](semantics.md), especially §3's classification/cause separation, §4's dispatch fence and outcome table, §5's duplicate waiting, and §8's service/transport obligations.
- [ESS source](../../../ess/domains/mutations.yaml): six attempt states, five transitions with command causation, three approval modes, four effect classifications, one explicit instance reference, and an internal state view.
- [Nine authored scenarios](scenarios/): command/outcome/error/event/view/field/state references compile against the model. Wrong-state branches assert the existing state and absence of the forbidden event.
- Generated conformance obligations and the authored scenarios compile in the Rust [repository gate](../../../crates/connectors-build/src/gate.rs). They are not run against a mutation implementation.

The unreleased model addition retains system version `v1`; it changes no ESS format or existing wire identity. The wire compatibility decision remains with `story:contracts-wire-compatibility`.

## Finding and scenario audit

The classifications below are normative expectations, audited against the text. “Compiled trace” means ESS resolves the named ledger trace; it does **not** mean the physical fault in that row was injected or its response projection executed.

| Case | Source | Required observation | Current evidence / future binding test |
|---|---|---|---|
| Refusal before preparation, including credentials | E11 | `not_attempted`, preserving the specific cause | Text §3/§4; count zero business sends under every preflight refusal |
| Store fails before preparation, including lost prepare acknowledgement | F01, E11 | `Unavailable` cause, `not_attempted`; prepared state alone grants no send | Text §4; fault the prepare port before/after its durable write |
| Crash after approval spend, before dispatch gate | F01 | Recovery fences Prepared to Aborted, `not_attempted`; approval remains spent | Compiled `recovery-before-gate`; real spend/crash/CAS race still required |
| Crash after gate, before actual send | F01 | Indeterminate, `unknown`; the durable trace is indistinguishable from a sent request | Compiled `recovery-after-gate`; inject crash between gate acknowledgement and capability call |
| Crash after send, before durable terminal outcome | F01 | Indeterminate, `unknown` | Same compiled trace; inject crash after provider observes the request |
| Response loss with `approval: not_required` | F01 | Indeterminate, `unknown`; approval absence says nothing about dispatch | Compiled `response-loss-no-approval`; drop the provider answer |
| Response loss with `approval: event_claim` | F01 | Same classification as required/not_required | Compiled `event-claim-uncertainty`; verify claim remains spent |
| Service deadline before gate | E01 | Abort wins, `Timeout` cause, `not_attempted`; late dispatcher loses | Compiled `deadline-before-gate`; race deadline with gate CAS |
| Service deadline after gate | E01 | `outcome_unknown`, diagnostic `Timeout`; neither abort nor second dispatch allowed | Compiled `deadline-after-gate`; race deadline with definitive result recording |
| Definitive success | F01, E11 | Completed, `applied` according to the operation's success definition | Compiled `definitive-success`; verify provider evidence, including asynchronous acceptance semantics |
| Definitive provider refusal without business effect | F01, E11 | Failed, `refused`, specific safe cause | Compiled `definitive-refusal`; provider profile must establish the evidence |
| Ambiguous provider 5xx, malformed answer or partial effect | F01, E11 | Indeterminate, `unknown`; status alone does not prove refusal | Text §4; fault provider before/after effect with the same answer shape |
| Lost dispatch-gate acknowledgement or unreadable gate state | F01, E11 | `unknown`; ambiguous acknowledgement never grants this worker a send | Text §4; fault durable CAS acknowledgement and subsequent reads |
| Definitive provider answer, failed terminal write | F01, E11 | Live caller preserves known `applied`/`refused`; recovery lacking that answer remains `unknown` | Text §4; independently observe provider, live response, ledger and restarted host |
| Duplicate waiter deadline before observing an original terminal record | E01 | `unknown`, diagnostic `Timeout`; never `Capacity` implying original non-dispatch | Text §4/§5/§8; expire wait in Prepared and Dispatching, count zero additional sends and no original-state change |
| Duplicate waiter observes durable terminal record | E01 | Preserve original classification, including refused/aborted/unknown | Text §4/§5; race terminal observation against waiter deadline |
| HTTP deadline/closed connection | E01 | Coordinate proven pre-gate abort or preserve uncertainty; no generic post-gate Timeout | Text §8; test transport deadline earlier than service deadline and response loss after host completion |
| Cancellation | F01, E01 | Proven pre-gate abort or `received` acknowledgement only; waiter cancellation cannot abort original | Compiled abort/wrong-state traces; actual transport cancellation still required |
| Recovery of legacy attempted row without a fence | F01 | `unknown` unless separate durable non-dispatch proof also fences possible dispatchers | Text §4; any future migration must prove this before adopting a legacy row |
| Retry or late reconciliation after Indeterminate | F01 | No reopened gate or implicit terminal rewrite | Compiled `indeterminate-is-terminal`; retain late evidence separately without another send |

## Tool evidence

Pinned executable: `.local/toolchains/ess/0.20.0/bin/ess` (`ess 0.20.0`). Reproduce from the repository root:

```console
.local/toolchains/ess/0.20.0/bin/ess specify validate --path ess
.local/toolchains/ess/0.20.0/bin/ess specify compile --path ess --out .local/review/mutation-outcomes-ir.json
.local/toolchains/ess/0.20.0/bin/ess verify conform author --path ess --scenarios contracts/operations/v1alpha1/scenarios --out .local/review/mutation-outcomes-authored.json
.local/toolchains/ess/0.20.0/bin/ess verify conform synthesize --path ess --target ir --scenarios contracts/operations/v1alpha1/scenarios --out .local/review/mutation-outcomes-suite.json
cargo run -p connectors-build --locked --offline -- --ess .local/toolchains/ess/0.20.0/bin/ess gate
```

Initial model checks: `connectors v1 — 3 file(s), valid`; IR compilation exit 0; 9 authored scenarios, 0 refusals; synthesis without authored input produces 36 generated scenarios, 0 refusals. The combined gate population is 45 scenarios. Generated files are reproducible ignored evidence, not hand-maintained schemas or a second source of truth.

Negative control: in an isolated temporary copy, replace `Aborted` with undeclared `DispatchAborted` in the deadline-before-gate error/view assertions. Synthesis refuses all three references with `ESS-AUTHOR-017` and exits 1. Thus a stale scenario reference fails the same command the gate uses. This checks compiler refusal, not a simulated host's behavior.

During authoring, ESS refused unsupported command-level `summary` fields; descriptions were moved to comments. No ESS upgrade or compiler workaround was introduced.

Full gate: `cargo run -p connectors-build --locked --offline -- --ess .local/toolchains/ess/0.20.0/bin/ess gate` exited **0** on 2026-09-08. It passed formatting, descriptor drift checks, workspace build/tests (39 passed, 0 failed, 0 ignored), Clippy with warnings denied, provider/library and generic-CLI boundaries, ESS validation/compilation and AEP validation. Combined synthesis reported `45 scenario(s) (9 authored), 0 refusal(s)`. The local transcript is `.local/review/mutation-outcomes-gate.log`. The optional `--msrv` check was not requested or run for this change.

Two independent read-only reviewers approved, with no in-scope findings: [semantic review](../../../.engineering/planning/review-result/mutation-outcomes-semantics-20260908.md) and [model review](../../../.engineering/planning/review-result/mutation-outcomes-model-20260908.md). Their exact verdicts are preserved as immutable AEP records. AEP 0.54.0 validation succeeds while warning about the existing prose reports and approval records with empty findings lists; those records are not edited to suppress that diagnostic.

## Explicit gaps

The pinned CLI has no Connectors mutation target for `verify conform run`. No runtime conformance report is claimed. A later Rust binding must realize these commands and run the fault tests above, including durable compare-and-set and stale-writer fencing, one-shot capability calls, evidence provenance, timeouts and projection of classification plus cause. The typed `Observation` does not enforce that mapping by itself.

The ESS source marks unresolved operation qualification, Connection/ApprovalRedemption relations, idempotency ownership/retention, and tenant/principal authority as `UNMAPPED`. Their cardinalities and ownership are not guessed. Existing immutable declarations remain separate from a running connection's lifecycle. Approval and preflight conditions are trusted host obligations, not caller-controlled booleans passed to `OpenDispatch`.

Other source findings retain their original story owners, including F02 (idempotency scope), F03 (federated approval), E02 (versioned wire encoding) and E28 (persistence ownership). Their unresolved semantics are not closed by compiling this model. Later catalog documents are outside the reviewed 48-finding baseline.
