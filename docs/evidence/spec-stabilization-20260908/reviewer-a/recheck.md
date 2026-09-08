# Independent review A: final specification recheck

Session F06/E20 verdict: **approve**. Residual findings: **0 blockers, 0 majors, 0 minors, 0 nits**.

E30 bounded documentation review verdict: **approve**. Findings: **0 blockers, 0 majors, 0 minors, 0 nits**.

This recheck preserves the original `review.md` and `reviewed-sources.json`. The original report hash remains `b8a098ed280935a545dd064e69593536e2e7b71c6361b56873eb9c97e6f8a683`. The new `recheck-reviewed-sources.json` records the 25 files read for this final scope, including all 13 session scenarios. It was taken after the final distinction between live-data-lease expiry and unredeemed establishment-token expiry was added to the normative prose and model comments. Later evidence-only additions to the verification record are outside these exact reviewed bytes.

## Session findings disposition

**R1 fixed.** The accepted close decisions in `first-terminal-not-replaced`, `media-overload-shared-reason`, and `revocation-refuses-data-and-renewal` now cap cutoff at the existing `12:00:04Z` live-lease expiry. The fourth cited example, `revocation-beats-renewal`, now closes through the originating trusted denial; its later `BeginClose` is a refused observation and cannot publish the supplied later cutoff. The separate rule that earlier lease/drain expiry wins over the ordinary terminal ceiling remains normative.

**R2 fixed.** `RenewDataLease` observing trusted revocation at `12:00:03Z` now takes the declared `deny_renewal` transition directly from Ready to Closing. Later `BeginClose` is wrong-state, carries the original acceptance time, emits no new ClosingBegun fact, and cannot reset deadlines. The successful local teardown is at `12:00:08Z`. `contracts/sessions/v1alpha1/semantics.md:112` and the corresponding model comments make atomic denial and first-observation time explicit for readiness, data admission and renewal.

I also checked the added counterexamples and related model changes. Expired data admission enters Closing and cannot be followed by a successful stale renewal. Actual continuity loss during Closing can enter Lost; later apparent resource release cannot fabricate Closed. Ready establishment requires completed stream/application negotiation rather than circularly requiring the Ready state before entering it. Live-data-lease expiry maps to `lease_expired`; unredeemed establishment-token expiry follows `BeginClose(reason=expired)` before readiness. These changes preserve first-terminal retention and the distinction between absent peer acknowledgement and actual owner/continuity loss.

The remaining executable obligations are represented honestly: trusted authority derivation, immutable field assignment, clocks and uncertainty, gate fan-out, queue/device cutoff and real resource accounting remain future binding work. ESS author/synthesize compilation is now explicitly distinguished from sequential trace execution as well as from timed runtime execution. The new authored directory still requires its explicit checks in addition to the current repository gate. No session implementation or runtime-conformance claim is introduced.

## Independent recheck evidence

Using pinned ESS 0.20.0, independently run for this recheck:

- `specify validate --path ess`: exit 0, 8 files valid.
- `verify conform author --path ess --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/reviewer-a/recheck-authored.json`: exit 0, 13 authored scenarios, 0 refusals.
- `verify conform synthesize --path ess --target ir --scenarios contracts/sessions/v1alpha1/scenarios --out .local/spec-stabilization-20260908/reviewer-a/recheck-synthesized.json`: exit 0, 201 scenarios including 13 authored, 0 refusals.

I independently audited all 13 final traces against the declaration's guarded outcomes, legal transition source states, wrong-state expectations, final states, successfully issued lease lengths, accepted close cutoff timestamps and first-terminal teardown ceilings. All 13 passed. The audit output is `recheck-trace-audit.json` in this directory. This ephemeral audit interprets supplied specification data; it does not execute Connectors or establish the trustworthiness of authority inputs or clocks. I also read the parent's separate `docs/evidence/spec-stabilization-20260908/session-trace-audit.json`; the independent results agree.

The final prose/comment-only expiry qualification does not change model declarations or scenario bytes. The parent reported its final full gate exited 0; I did not rerun the Rust gate independently. No tracked Rust diff was present during this recheck. No timed media/session runtime tests were executed by this reviewer.

## Separate bounded E30 documentation review

Reviewed `contracts/datasources/logs/v1alpha1/semantics.md` section 4.1, `docs/adapters/grafana.md` section 6 and adjacent authentication text, and `docs/evidence/spec-stabilization-20260908/tenant-header.md`. I checked the current `HttpConfig` declaration as the local implementation reference.

The two normative descriptions consistently select receiver-owned `http.extra_headers` for the proposed direct tenant binding and reject interpreting `tenant_header` as a working alias. They correctly state that the current host does not implement this extension. Caller input and adapter-built headers cannot override the admitted tenant; case-duplicate and credential/transport-header conflicts are refused. Provider tenancy remains distinct from SaaS application authority. A new admitted revision isolates cached results and continuations after a configuration change. For mediation, the parent route/datasource owns the downstream tenant and an unsupported child override is refused without direct fallback.

The evidence record describes textual obligations and makes no claim of an executed HTTP fixture, configuration schema, or vendor implementation. The change names an existing proposed configuration value and does not invent entity ownership or a new ESS relation. This approval covers E30's representation and authority-boundary consistency; it does not claim that all Grafana, log-continuation, provider-admission or configuration contracts are stabilized.
