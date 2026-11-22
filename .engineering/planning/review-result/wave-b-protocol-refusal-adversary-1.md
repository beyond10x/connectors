---
format: aep.planning-md/3
id: review-result:wave-b-protocol-refusal-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on adapter-private-protocol-refusal-named (single defect wave)
relations:
- reviews: story:adapter-private-protocol-refusal-named
revision: 1
---
```
unit: story:adapter-private-protocol-refusal-named
verdict: red
cases: executed 31→32, red 1
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/units/adapter-private-protocol-refusal-named/adv/{case.rs,red.log,suite.log}
needs-coordinator: yes
```

Against commit 624f6d22b plus one uncommitted test in `apps/connectors/tests/local_cli.rs` (+59).

1. Case `adversary_status_after_reconfiguring_names_the_protocol_actually_refused`: an entry selects `connectors-private/2`, the program speaks only /1, status refuses; the operator follows `check_configuration`, changes the entry to `connectors-private/1` and runs `instances status` again; if status still refuses it must name the protocol the program was asked for (/2). Red alone, exit 101: `"private_protocol":"connectors-private/1", ... "supported_private_protocols":["connectors-private/1"]`, `left: String("connectors-private/1") right: "connectors-private/2"`, `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 32 filtered out`.
2. Suite `cargo test --locked -p connectors --test local_cli`, exit 101: `FAILED. 31 passed; 1 failed; 1 ignored` (only the case above).
3. Findings:
   - Status names a protocol nobody asked for (`crates/connectors-host/src/local/owner/lifecycle.rs:103`): `observe` takes `private_protocol` from the entry as it reads now, not from the refused start; contradicts `semantics.md` ("the protocol asked for"). Reached by the documented workflow (`next_action=check_configuration`, then `instances status`; the owner reloads configuration per request, `transport.rs:2478`). Fix: record the protocol asked in `lifecycle::Start` (`supervisor.rs:1830`). The Acceptance holds without it; the contract text does not.
   - The catalog provider binary probably no longer compiles (`adapters/catalog/src/served.rs:738`): `fn error` matches every `runtime::Failure` variant with no wildcard and no arm for `UnsupportedPrivateProtocol` (E0004). `served` is compiled only into that binary (`main.rs:7`). Not proven by a build.
   - The README check can pass while the README is wrong (`crates/connectors-docs/src/onboarding.rs:373`): it counts a protocol as supported when `fn bootstrap_v2(` / `fn bootstrap_v3(` appears anywhere in the adapter's sources; Kubernetes speaks /2 only when `pod_exec` is set (`adapters/kubernetes/src/local.rs:328`, `:484`), so the README's unconditional `connectors-private/1, 2` is wrong for a default instance; a comment or a test mock also counts.
4. Attacked, not broken: oversized or hostile `refused` frames (zero document limit, no secret, unknown versions and duplicates dropped); a `refused` frame naming another version is a mismatch; a `refused` frame after the handshake is a protocol error and the child is terminated; new host with an old adapter still `exited_before_handshake`; old host with a new adapter reads a broken handshake; `next_action` kept on connect, invoke and status; `instances status` is per instance; MCP maps the new code to `unavailable`.

```findings
[
{"file":"crates/connectors-host/src/local/owner/lifecycle.rs","line":103,"category":"contract-drift","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"status of a refused instance names the entry's current private_protocol, not the one the program was asked and refused, so after the user switches the entry to connectors-private/1 it reports a refusal of /1 beside supported [/1]"},
{"file":"adapters/catalog/src/served.rs","line":738,"category":"contract-drift","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the exhaustive match over runtime::Failure has no arm for the new UnsupportedPrivateProtocol, so the connectors-catalog-provider binary fails to compile (E0004); not yet confirmed by a build"},
{"file":"crates/connectors-docs/src/onboarding.rs","line":373,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the README protocol check is a text match on fn bootstrap_v2(/v3(, so it accepts Kubernetes as speaking connectors-private/2 although a default instance (no pod_exec) refuses /2"}
]
```
