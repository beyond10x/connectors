# Auth semantic hardening wave, 2026-09-08

F04 refresh coordination and F05 credential evidence are implemented at the semantic contract/model level. [The governed wave page](../../../.engineering/planning/specification/auth-hardening-wave-20260908.md) records approval, complete store selection, ownership, reviews and worktree handoff.

The [gate summary](integration/gate-summary.json) records 39 Rust tests passed, Rust 1.88 checks passed, and 169 ESS scenarios compiled (34 authored) with zero refusals. Auth runtime conformance remains unimplemented. [Full gate output](integration/full-gate.log) preserves every step result; [the first attempt](integration/full-gate-attempt-1.log) records the sccache startup socket failure resolved before the successful rerun.

Original briefs, implementor reports and adversarial reports/logs are retained in refresh/ and evidence/. They are historical records of their individual branches; final combined results live above. The extra reviewer scenarios remain in the contract scenario directories. Both reviews returned no findings; each added one structural boundary case.

[Disposable cleanup](integration/disposable-cleanup.json) records exact removed build/scratch/tool directories. The three wave source checkouts were subsequently removed through managed GC after the operator requested cleanup; [removal receipts](integration/managed-cleanup.json) record recovery through a persistent [local bare Git backup](integration/local-recovery.json). No external publication or release occurred.
