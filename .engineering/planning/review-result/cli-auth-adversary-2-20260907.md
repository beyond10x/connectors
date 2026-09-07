---
format: aep.planning-md/1
id: review-result:cli-auth-adversary-2-20260907
kind: review-result
status: active
title: Authentication recovery adversarial review, second and final pass
relations:
- reviews: story:auth-as-tool-result
revision: 1
---
unit: auth-as-tool-result, second/final ordinary pass; 77a5857cb3cd37ebb6fd17886926029240338a01 plus retained tests.patch
verdict: INFEASIBLE
cases: executed 871→877, red 1
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: 237 retained scratch paths; assigned target/TMPDIR and existing Cargo bookkeeping below
needs-coordinator: yes — record this immutable final pass and route the remaining served-document finding with its production-reachability limit; no third ordinary pass

````text
 crates/connectors-cli/tests/remediation.rs         |  61 ++++++++++
 crates/connectors-client/src/tests.rs              | 124 +++++++++++++++++++++
 crates/connectors-console/tests/remediation.rs     |  96 ++++++++++++++++
 crates/connectors-runtime/src/remediation_tests.rs | 109 ++++++++++++++++++
 crates/server/src/hosted/tests/remediation.rs      |  88 +++++++++++++++
 5 files changed, 478 insertions(+)
````

The final attack adds six cases in five existing test owners. All ten frozen original files, including every first-review and correction assertion, remain exact byte prefixes. No production, manifest, generated contract, planning, Git reference/index, configuration, installed binary or live provider changed. The complete auth unit includes its prior protocol/service slices and the unchanged separately reviewed OAuth prerequisite; several-credentials remains held. This report is a runner-backed agent review, not architecture acceptance, human approval or live adoption.

The public copy replaces only the absolute local home-directory prefix with ~. All artifact references are plain/code paths.

1. Deciding cases, before full suites.

| Case | Actual boundary | First actual selection and final disposition |
| --- | --- | --- |
| Hosted status/schema matrix | Real router, synthetic verifier/remediation backend, real grants; selected missing/degraded/outage/stale responses, no dispatch/session/approval | 1 failed, exit101; same document red in full suite |
| Fresh schema composition | Public LocalClient; canonical hostile DTOs, internal reference and conditional schema applied to captured input | 1 passed, exit0 |
| Deadline after Ready | Public LocalClient; stalled Ack or final description bounded by original deadline; one Ack, no Invoke/resend | 1 passed, exit0 |
| Session dispatch ownership | Actual registry; exact owner errors, ambiguous/unknown sessions; Status and Ack never fall through | Initial filter selected0, exit0; corrected exact module filter selected1 passed, exit0 |
| Private instruction fetch cancellation | Actual console presenter; cancellation before headers or during partial body, original inode cleared, no Status/Ack/Invoke | 1 passed, exit0 |
| CLI permission ordering | Real CLI; four output formats and private-socket/public-root permission variants with stdin held open, no connection or destination | First fixture oracle failed1, exit101; corrected new oracle passed1, exit0 |

Every new case existed before the first Cargo command. The zero-count registry selection omitted its enclosing registry module; it supplies no deciding result. The first CLI oracle expected configuration for all permission refusals. Its public-root row correctly reached the earlier state-root guard, returning runtime before stdin. Only the new expected-code expression was corrected; all no-block/no-connect/no-file/privacy assertions and every frozen original assertion remained. The original 473-line complete patch and CLI source/log remain separately retained. No product repair is attributed to that fixture correction.

auth-adversary2-hosted-first

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.command.json
````json
{
  "label": "auth-adversary2-hosted-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "server",
    "--lib",
    "hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:51:21.234664+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.log
````text
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `test` profile [unoptimized] target(s) in 12.03s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-034f8d953ecabd82)

running 1 test
actual selected status/schema matrix, no dispatch: [(V0Alpha1, "missing", 503, true), (V0Alpha1, "degraded", 503, true), (V0Alpha1, "outage", 503, true), (V0Alpha1, "stale", 409, false), (V0Alpha2, "missing", 503, true), (V0Alpha2, "degraded", 503, true), (V0Alpha2, "outage", 503, true), (V0Alpha2, "stale", 409, false), (V0Alpha3, "missing", 409, true), (V0Alpha3, "degraded", 409, true), (V0Alpha3, "outage", 503, true), (V0Alpha3, "stale", 409, true)]

thread 'hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts' (3445702) panicked at crates/server/src/hosted/tests/remediation.rs:705:5:
every selected response must satisfy the schema served for its actual HTTP status, including non-authentication conflicts
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts ... FAILED

failures:

failures:
    hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 115 filtered out; finished in 1.70s

error: test failed, to rerun pass `-p server --lib`
````

Collected result: 
````json
{
  "label": "auth-adversary2-hosted-first",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 14567911424,
    "tmpfs_free_bytes": 13233709056,
    "mem_available_bytes": 32281399296
  },
  "maximum_target_bytes": 11350683648,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:51:37.617329+00:00"
}
````

auth-adversary2-client-schema-first

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.command.json
````json
{
  "label": "auth-adversary2-client-schema-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connectors-client",
    "--lib",
    "tests::auth_adversary2_fresh_internal_schema_composition_validates_captured_input",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:52:02.407311+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.log
````text
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Finished `test` profile [unoptimized] target(s) in 4.76s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_client-df4e5f6918aa1036)

running 1 test
fresh schema internal-reference: accepted=true, methods=["remediation_start", "remediation_status", "remediation_acknowledge", "describe", "describe"]
fresh schema changed-composition: accepted=false, methods=["remediation_start", "remediation_status", "remediation_acknowledge", "describe", "describe"]
test tests::auth_adversary2_fresh_internal_schema_composition_validates_captured_input ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.01s

````

Collected result: 
````json
{
  "label": "auth-adversary2-client-schema-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 16154382336,
    "tmpfs_free_bytes": 13247815680,
    "mem_available_bytes": 33705041920
  },
  "maximum_target_bytes": 11349147648,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:52:08.730817+00:00"
}
````

auth-adversary2-client-deadline-first

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.command.json
````json
{
  "label": "auth-adversary2-client-deadline-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connectors-client",
    "--lib",
    "tests::auth_adversary2_deadline_after_ready_bounds_ack_and_fresh_description_without_resend",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:52:30.195262+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.log
````text
    Finished `test` profile [unoptimized] target(s) in 0.26s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_client-df4e5f6918aa1036)

running 1 test
captured deadline late-ack: accepted=false, methods=["remediation_start", "remediation_status", "remediation_acknowledge"]
captured deadline late-description: accepted=false, methods=["remediation_start", "remediation_status", "remediation_acknowledge", "describe", "describe"]
test tests::auth_adversary2_deadline_after_ready_bounds_ack_and_fresh_description_without_resend ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out; finished in 4.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-client-deadline-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 15537807360,
    "tmpfs_free_bytes": 13265768448,
    "mem_available_bytes": 35824832512
  },
  "maximum_target_bytes": 11316527104,
  "maximum_tmpdir_bytes": 8192,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:52:35.325102+00:00"
}
````

auth-adversary2-registry-first

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.command.json
````json
{
  "label": "auth-adversary2-registry-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--lib",
    "remediation_tests::auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:53:08.795800+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
   Compiling identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 6.52s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-684bebcc28bafc34)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-registry-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 18231508992,
    "tmpfs_free_bytes": 13268840448,
    "mem_available_bytes": 35254755328
  },
  "maximum_target_bytes": 11329933312,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:53:17.099281+00:00"
}
````

auth-adversary2-registry-selected

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.command.json
````json
{
  "label": "auth-adversary2-registry-selected",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--lib",
    "registry::remediation_tests::auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:54:00.054765+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.23s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-684bebcc28bafc34)

running 1 test
session route single-refused, acknowledge=false: Refused, calls=1
session route single-refused, acknowledge=true: Refused, calls=1
session route single-unavailable, acknowledge=false: Unavailable, calls=1
session route single-unavailable, acknowledge=true: Unavailable, calls=1
session route single-unsupported, acknowledge=false: Unsupported, calls=1
session route single-unsupported, acknowledge=true: Unsupported, calls=1
session route ambiguous, acknowledge=false: Unavailable, calls=0
session route ambiguous, acknowledge=true: Unavailable, calls=0
session route unknown, acknowledge=false: Refused, calls=0
session route unknown, acknowledge=true: Refused, calls=0
test registry::remediation_tests::auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 36 filtered out; finished in 0.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-registry-selected",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 20620685312,
    "tmpfs_free_bytes": 13270511616,
    "mem_available_bytes": 36455092224
  },
  "maximum_target_bytes": 11318513664,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:54:01.864785+00:00"
}
````

auth-adversary2-console-first

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.command.json
````json
{
  "label": "auth-adversary2-console-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--test",
    "remediation",
    "auth_adversary2_instruction_fetch_cancellation_clears_reserved_destination_before_poll",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:54:27.948812+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.log
````text
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 3.17s
     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-f1ff96957d29feb1)

running 1 test
instruction fetch cancelled partial_body=false: reserved inode cleared, no poll/ack/invoke
instruction fetch cancelled partial_body=true: reserved inode cleared, no poll/ack/invoke
test auth_adversary2_instruction_fetch_cancellation_clears_reserved_destination_before_poll ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 1.23s

````

Collected result: 
````json
{
  "label": "auth-adversary2-console-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19616632832,
    "tmpfs_free_bytes": 13267521536,
    "mem_available_bytes": 35054825472
  },
  "maximum_target_bytes": 11319828480,
  "maximum_tmpdir_bytes": 8192,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:54:34.066402+00:00"
}
````

auth-adversary2-cli-first

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.command.json
````json
{
  "label": "auth-adversary2-cli-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--test",
    "remediation",
    "auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:54:58.165206+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
   Compiling identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 10.54s
     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-9806fb21acc5f093)

running 1 test
CLI permissions json/700/640: blocked=false, exit=Some(1)
CLI permissions json/700/606: blocked=false, exit=Some(1)
CLI permissions json/750/600: blocked=false, exit=Some(1)

thread 'auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format' (3473033) panicked at tests/remediation.rs:641:13:
actual refusal: {
  "error": {
    "code": "runtime",
    "message": "the state root must be absolute and outside the current working tree"
  }
}

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format ... FAILED

failures:

failures:
    auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.03s

error: test failed, to rerun pass `--test remediation`
````

Collected result: 
````json
{
  "label": "auth-adversary2-cli-first",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 19615903744,
    "tmpfs_free_bytes": 13171638272,
    "mem_available_bytes": 34692063232
  },
  "maximum_target_bytes": 11526176768,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:55:09.606972+00:00"
}
````

auth-adversary2-cli-corrected

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.command.json
````json
{
  "label": "auth-adversary2-cli-corrected",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--test",
    "remediation",
    "auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format",
    "--",
    "--exact",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:55:53.218142+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 1.14s
     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-9806fb21acc5f093)

running 1 test
CLI permissions json/700/640: blocked=false, exit=Some(1)
CLI permissions json/700/606: blocked=false, exit=Some(1)
CLI permissions json/750/600: blocked=false, exit=Some(1)
CLI permissions yaml/700/640: blocked=false, exit=Some(1)
CLI permissions yaml/700/606: blocked=false, exit=Some(1)
CLI permissions yaml/750/600: blocked=false, exit=Some(1)
CLI permissions text/700/640: blocked=false, exit=Some(1)
CLI permissions text/700/606: blocked=false, exit=Some(1)
CLI permissions text/750/600: blocked=false, exit=Some(1)
CLI permissions compact/700/640: blocked=false, exit=Some(1)
CLI permissions compact/700/606: blocked=false, exit=Some(1)
CLI permissions compact/750/600: blocked=false, exit=Some(1)
test auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.13s

````

Collected result: 
````json
{
  "label": "auth-adversary2-cli-corrected",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19608956928,
    "tmpfs_free_bytes": 13266219008,
    "mem_available_bytes": 34992959488
  },
  "maximum_target_bytes": 11322093568,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:55:56.142340+00:00"
}
````

2. Affected full suites, strict all-target Clippy and fmt, after the deciding cases.

No pre-addition baseline suite ran. The completed affected cohort is server117 + client47 + runtime458 + console108 + CLI141 =871 before this pass. Retained logs correct the brief's protocol/service labels: protocol37 unit +39 bundle +2 rate cases =78, and service68. Their combined146 was correctly included in the earlier1,017 cohort; those unchanged packages are not rerun here. Thus the complete retained cohort would be1,017→1,023, while this report's executed header honestly counts only the current871→877 owning-package/workspace commands. Alternate runtime features and isolated selections are not additional unique cases. Exact baseline provenance is in attack-plan-and-count-provenance.json and prior-seal-verification.json.

| Lane | Before → executed | Passed / failed / ignored | Exit |
| --- | --- | --- | --- |
| root-full | 164 → 167 | 166 / 1 / 0 | 101 |
| runtime-full | 458 → 459 | 459 / 0 / 2 | 0 |
| runtime-no-default-continued | 458 → 459 | 459 / 0 / 2 | 0 |
| console-full | 108 → 109 | 109 / 0 / 0 | 0 |
| cli-full | 141 → 142 | 142 / 0 / 0 | 0 |

The single full default root command reproduces the hosted schema mismatch. All completed other full suites and all strict/fmt commands pass. Two pre-existing PostgreSQL cases remain ignored in each runtime configuration. No complete twelve-workspace CI gate is claimed.

The first no-default monitor failed when du encountered a fixture WAL file removed during sampling. The driver exited1 without collecting Cargo's exit. Immediate inspection found its exact PGID3490203 already empty, so no signal was sent. Its complete Cargo log, driver stderr, partial resource timeline and source proof are preserved; any visible passing summaries there are observations only, excluded from gate/count claims. The coordinator confirmed continuation of this same lane. A new scratch runner counts existing fixture entries while tolerating normal disappearance and kills only its own group on other active sampling exceptions. Caps, reserves and the original runner remain unchanged. The distinctly named continued no-default run is the completed result below; this is not a new attack or a baseline rerun.

auth-adversary2-root-full

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.command.json
````json
{
  "label": "auth-adversary2-root-full",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "server",
    "-p",
    "connectors-client",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:57:19.324874+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.log
````text
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `test` profile [unoptimized] target(s) in 12.87s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_client-8b1fc807c3113e01)

running 44 tests
test identity::tests::hosted_request_families_select_the_smallest_available_scope ... ok
test identity::tests::keyring_account_contains_no_endpoint_or_principal ... ok
test identity::tests::mcp_invocation_uses_only_the_invoke_scope ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_destination_is_exact_numeric_loopback_and_capability_is_header_only ... ok
test hosted_catalog::tests::posts_and_validates_a_catalog_frame ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_parser_preserves_optional_device_uri_and_refuses_origin_changes ... ok
test personal_oauth::personal_oauth_tests::actual_private_instruction_request_keeps_capability_out_of_url_and_delivers_only_to_human_writer ... ok
test admin::tests::named_resources_are_typed_and_the_value_is_not_exposed ... ok
test personal_oauth::personal_oauth_tests::personal_create_refusal_cannot_echo_private_daemon_text ... ok
test personal_oauth::personal_oauth_tests::private_browser_authorization_is_never_a_provider_independent_redirect ... ok
test admin::tests::identity_pkce_exchange_returns_only_the_exact_access_credential ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_private_target_repeated_consistently ... ok
test tests::auth_adversary2_fresh_internal_schema_composition_validates_captured_input ... ok
test tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_changed_deadline ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_integration_status ... ok
test git_fetch_client::tests::response_is_bound_to_the_request_and_source_authority_is_redacted ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_session ... ok
test personal_oauth::personal_oauth_tests::valid_browser_only_session_reaches_the_explicit_personal_handoff ... ok
test personal_oauth::personal_oauth_tests::actual_instruction_redirect_and_cacheable_reply_are_closed_refusals ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_describe_target ... ok
test tests::hosted_client_requires_https_except_on_loopback_or_internal_cluster_dns ... ok
test personal_oauth::personal_oauth_tests::personal_success_retains_a_callable_correlated_description ... ok
test tests::completion_endpoint_is_validated_before_secret_submission ... ok
test tests::hosted_client_posts_and_validates_a_datasource_frame ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_credential_purpose ... ok
test tests::auth_stage2_hosted_409_is_typed_without_resend ... ok
test tests::hosted_client_posts_the_same_typed_operation_frame ... ok
test personal_oauth::personal_oauth_tests::expired_monotonic_instruction_budget_never_connects_even_with_future_wall_deadline ... ok
test tests::hosted_subscription_client_redacts_and_redeems_one_attempt_capability ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_created_description ... ok
test personal_oauth::personal_oauth_tests::personal_poll_and_describe_refusals_close_inner_transport_and_daemon_text ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_degraded_description ... ok
test tests::local_client_frames_and_correlates_an_operation ... ok
test tests::hosted_subscription_client_refuses_a_cacheable_credential_boundary ... ok
test response::tests::rate_stage2_hosted_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok
test tests::hosted_subscription_client_starts_and_completes_pkce_without_retaining_the_code ... ok
test identity::tests::login_separates_the_session_and_refreshes_exact_scope_tokens ... ok
test identity::tests::auth_stage2_identity_409_does_not_renew_or_resend ... ok
test tests::auth_stage2_bound_completion_rechecks_target_schema_and_never_invokes ... ok
test personal_oauth::personal_oauth_tests::actual_connection_v1_polling_keeps_guarded_completion_private_and_never_repeats_create ... ok
test response::tests::rate_stage2_local_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok
test tests::auth_stage2_local_versions_are_strict_without_resend ... ok
test tests::auth_adversary2_deadline_after_ready_bounds_ack_and_fresh_description_without_resend ... ok

test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.03s

     Running tests/personal_oauth_adversary.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth_adversary-ac6fedc7b1c855b9)

running 2 tests
test oauth_pass1_public_instruction_refusals_do_not_redirect_poll_or_echo_private_bytes ... ok
test oauth_pass1_public_handoff_waits_for_bound_callable_success_without_replay ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.52s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-e255c22140985ca6)

running 116 tests
test egress::tests::ambiguous_retry_after_is_not_flattened_into_advice ... ok
test egress::tests::egress_requires_a_nonempty_ascii_connection_or_session_reference ... ok
test egress::tests::cached_client_cannot_bypass_current_address_policy ... ok
test egress::tests::exact_origin_cannot_be_widened_by_path_host_or_userinfo ... ok
test egress::tests::operator_network_may_admit_private_but_not_process_local_addresses ... ok
test egress::tests::ipv4_mapped_ipv6_cannot_bypass_address_classification ... ok
test egress::tests::public_dns_refuses_private_local_reserved_and_mixed_answers ... ok
test egress::tests::malformed_retry_after_does_not_hide_the_definite_provider_response ... ok
test egress::tests::retry_after_extraction_keeps_only_one_valid_decimal_and_admitted_headers ... ok
test egress::tests::suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port ... ok
test hosted::enforcement::tests::the_canonical_digest_ignores_member_order_and_nothing_else ... ok
test hosted::enforcement::tests::an_issued_record_round_trips_without_its_reference_in_any_key ... ok
test hosted::principal::tests::lease_seeds_survive_the_verifier_token_rotation_composition ... ok
test hosted::git_fetch::tests::rejected_control_identity_is_decided_before_the_request_body_is_polled ... ok
test hosted::git_fetch::tests::rejected_source_authority_is_decided_before_body_or_broker_exchange ... ok
test egress::tests::reused_connection_keeps_authorization_and_timeout_request_specific ... ok
test hosted::mcp::toolset::authentication_projection_closes_valid_private_reference_and_message_fields ... ok
test hosted::git_fetch::tests::control_and_internal_routes_are_separate_and_non_cacheable ... ok
test hosted::git_fetch::tests::ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body ... ok
test hosted::tests::admin_routes::auth_metadata_is_public_and_selects_exact_authority ... ok
test hosted::tests::admin_routes::operator_group_without_the_exact_scope_cannot_write ... ok
test egress::tests::rate_stage2_definite_http_429_survives_oversized_or_broken_error_bodies ... ok
test hosted::tests::admin_routes::operator_can_write_and_status_never_returns_the_secret ... ok
test hosted::tests::contract_validation::hosted_route_refuses_a_malformed_backend_contract ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_boundary_selects_response_version_even_for_early_refusals ... ok
test egress::rate_adversary_tests::rate_final_chunked_429_keeps_definite_status_without_body_or_untrusted_advice ... ok
test hosted::tests::a_human_issues_one_exact_input_approval_which_is_spent_once ... ok
test egress::tests::pool_is_bounded_and_separates_current_addresses_authorities_origins_and_policy ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_http_projects_refusals_once_and_rejects_invalid_frames_before_backend ... ok
test hosted::tests::contract_validation::rate_stage2_invalid_http_correlation_is_a_bounded_versioned_client_refusal ... ok
test hosted::tests::contract_validation::rate_final_hosted_describe_versions_reject_bad_advice_before_loss ... ok
test egress::rate_adversary_tests::rate_adversary_header_cardinality_and_unfinished_body_are_separate ... ok
test hosted::tests::contract_validation::rate_adversary_hosted_grant_and_approval_refusals_keep_requested_version ... ok
test hosted::tests::docs::a_request_example_with_an_unknown_field_is_refused ... ok
test hosted::tests::docs::openapi_json_is_served_verbatim_with_a_content_hash_etag ... ok
test hosted::tests::enforcement::a_granted_mutation_demanding_approval_refuses_without_one ... ok
test hosted::tests::enforcement::a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone ... ok
test hosted::tests::enforcement::a_granted_mutation_with_a_demanded_approval_dispatches_with_one ... ok
test hosted::tests::enforcement::a_mutation_with_no_admitting_grant_refuses ... ok
test hosted::tests::enforcement::an_unbound_grant_store_is_an_outage_for_effects_only ... ok
test hosted::tests::enforcement::a_second_presentation_of_the_same_approval_refuses_and_journals_replay ... ok
test hosted::tests::enforcement::the_read_only_path_is_unchanged_for_callers_without_grants ... ok
test hosted::tests::hosted_completion_failures_are_non_cacheable_and_browser_hardened ... ok
test hosted::tests::hosted_completion_streams_fragments_into_a_redacted_bounded_submission ... ok
test hosted::tests::hosted_connection_route_uses_the_same_identity_boundary ... ok
test hosted::tests::docs::every_documented_refusal_example_names_a_real_error_code ... ok
test hosted::tests::docs::auth_openapi_selects_each_supported_identity_explicitly ... ok
test hosted::tests::hosted_datasource_route_passes_verified_groups_and_exact_tenant ... ok
test hosted::tests::docs::auth_openapi_remediation_examples_keep_the_operation_unattempted ... ok
test hosted::tests::docs::the_document_pins_the_exact_wire_contract_identities_and_audience ... ok
test hosted::tests::docs::every_documented_request_example_is_accepted_by_its_protocol_type ... ok
test hosted::tests::hosted_route_requires_identity_and_exact_tenant_binding ... ok
test hosted::tests::enforcement::every_enforcement_refusal_renders_the_same_bytes ... ok
test hosted::tests::docs::the_docs_page_is_served_unauthenticated_as_html ... ok
test hosted::tests::docs::every_documented_mcp_request_example_is_answered_by_the_live_transport ... ok
test hosted::tests::mcp::an_invoke_without_the_invoke_scope_surfaces_not_granted ... ok
test hosted::tests::mcp::an_op_backed_invoke_describes_then_invokes_with_the_fresh_lease ... ok
test hosted::tests::mcp::approval_demand_and_evidence_pass_through_the_admission_seam ... ok
test hosted::tests::docs::the_docs_page_links_the_contract_and_renders_its_version ... ok
test hosted::tests::docs::the_docs_page_makes_zero_external_requests ... ok
test hosted::tests::docs::the_docs_page_refusal_table_carries_every_documented_code ... ok
test hosted::tests::mcp::initialize_echoes_admitted_revisions_and_answers_ping ... ok
test hosted::tests::mcp::a_stale_authority_refusal_is_retried_exactly_once_with_a_fresh_lease ... ok
test hosted::tests::docs::every_docs_page_example_is_the_documents_example_after_json_normalization ... ok
test hosted::tests::mcp::the_mcp_route_is_stateless_post_only ... ok
test hosted::tests::mcp::tools_list_returns_exactly_the_three_meta_tools ... ok
test hosted::tests::mcp::datasource_backed_tools_route_through_the_datasource_seam ... ok
test hosted::tests::mcp::rate_stage2_mcp_preserves_refusal_details_without_entering_stale_retry ... ok
test hosted::tests::mcp_monitoring::a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once ... ok
test hosted::tests::mcp::a_busy_namespace_is_listed_whole_without_a_paging_surface ... ok
test hosted::tests::mcp::tool_search_projects_only_the_entries_the_callers_seam_results_support ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam ... ok
test hosted::tests::monitoring_transport_gate_admits_only_configured_groups_or_operator ... ok
test hosted::tests::pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator ... ok
test hosted::tests::mcp::transport_refusals_carry_the_designed_statuses_and_codes ... ok
test hosted::tests::mcp::tool_describe_projects_the_underlying_description_without_a_lease ... ok
test hosted::tests::production_router_publishes_client_discovery_without_authentication ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch ... ok
test hosted::tests::mcp_monitoring::the_acceptance_sequence_invokes_with_a_target_and_integer_epochs ... ok
test hosted::tests::remediation::auth_connection_v2_ordinary_requests_select_exact_identity_and_refuse_duplicates ... ok
test hosted::tests::mcp::rate_adversary_mcp_stale_then_rate_stops_without_losing_large_delay ... ok
test hosted::tests::self_event_scope_is_closed_to_slack_specific_requests ... ok
test hosted::tests::signal::a_granted_session_signal_dispatches_behind_the_sessions_grant ... ok
test hosted::tests::mcp_monitoring::tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal ... ok
test hosted::tests::mcp_monitoring::tool_describe_enumerates_the_callers_configured_targets_without_a_lease ... ok
test hosted::tests::signal::an_unbound_grant_store_is_an_outage_for_session_signals ... ok
test hosted::tests::signal::an_effect_bearing_session_signal_without_an_admitting_grant_refuses ... ok
test hosted::tests::subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable ... ok
test hosted::tests::tenant_members_receive_only_read_only_module_invocation ... ok
test hosted::tests::subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems ... ok
test local::tests::a_broad_state_directory_refuses_without_repair ... ok
test local::tests::a_second_daemon_cannot_unlink_the_live_daemons_socket ... ok
test local::tests::auth_one_shot_v3_refuses_before_backend_work_and_joins_shutdown ... ok
test hosted::tests::signal::a_signal_refusal_matches_the_invoke_refusal_bytes ... ok
test local::tests::auth_one_shot_v3_serves_a_real_result_and_joins_shutdown ... ok
test local::tests::one_socket_dispatches_the_value_free_connection_and_event_contracts ... ok
test local::tests::owner_socket_serves_one_strict_bounded_operation_frame ... ok
test local::tests::rate_stage2_actual_socket_serves_both_versions_without_resending ... ok
test hosted::tests::mcp_monitoring::monitoring_tool_schemas_state_the_documents_contract ... ok
test hosted::tests::docs::auth_openapi_503_schemas_admit_all_supported_unavailable_versions ... ok
test hosted::tests::mcp::a_pathological_namespace_is_cut_with_an_explicit_truncation_marker ... ok
test hosted::tests::docs::auth_openapi_schema_projection_preserves_protocol_vector_results ... ok
test hosted::tests::remediation::auth_v3_does_not_publish_a_structurally_valid_private_backend_reference ... ok
test hosted::tests::remediation::auth_v3_requires_a_real_grant_and_keeps_unknown_targets_opaque ... ok
test hosted::tests::remediation::auth_connection_v2_hosted_start_has_real_grants_and_no_production_acquisition ... ok
test catalog_projection::tests::a_deployment_publishes_only_the_setup_flows_it_can_complete ... ok
test catalog_projection::tests::search_is_whole_catalog_and_describe_is_descriptive_only ... ok
test local::tests::auth_one_shot_v3_need_precedes_dispatch_and_joins_shutdown ... ok
test catalog_projection::tests::platform_provider_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::remediation::auth_v3_need_precedes_real_approval_redemption_and_dispatch ... ok
test catalog_projection::tests::every_shipped_provider_description_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::hosted_liveness_and_identity_backed_readiness_are_distinct ... ok
test hosted::tests::hosted_liveness_stays_local_when_a_backend_dependency_is_unready ... ok
test hosted::tests::docs::every_documented_route_exists_in_the_real_router ... ok
test hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation ... ok
test hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts ... FAILED

failures:

---- hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts stdout ----
actual selected status/schema matrix, no dispatch: [(V0Alpha1, "missing", 503, true), (V0Alpha1, "degraded", 503, true), (V0Alpha1, "outage", 503, true), (V0Alpha1, "stale", 409, false), (V0Alpha2, "missing", 503, true), (V0Alpha2, "degraded", 503, true), (V0Alpha2, "outage", 503, true), (V0Alpha2, "stale", 409, false), (V0Alpha3, "missing", 409, true), (V0Alpha3, "degraded", 409, true), (V0Alpha3, "outage", 503, true), (V0Alpha3, "stale", 409, true)]

thread 'hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts' (3480769) panicked at crates/server/src/hosted/tests/remediation.rs:705:5:
every selected response must satisfy the schema served for its actual HTTP status, including non-authentication conflicts
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    hosted::tests::remediation::auth_adversary2_hosted_non_auth_conflict_and_outage_keep_selected_status_contracts

test result: FAILED. 115 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s

error: test failed, to rerun pass `-p server --lib`
     Running tests/rate_adversary_local.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_local-e255c2a977e11baf)

running 2 tests
test rate_final_local_describe_validates_before_version_loss ... ok
test rate_adversary_local_versions_validate_before_single_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_client

running 3 tests
test crates/connectors-client/src/model.rs - model::PersonalOAuthInstructions (line 340) - compile fail ... ok
test crates/connectors-client/src/model.rs - model::PendingPersonalOAuth (line 314) - compile fail ... ok
test crates/connectors-client/src/model.rs - model::PendingRemediation (line 375) - compile fail ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

   Doc-tests server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `-p server --lib`
````

Collected result: 
````json
{
  "label": "auth-adversary2-root-full",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 19596152832,
    "tmpfs_free_bytes": 13216509952,
    "mem_available_bytes": 34441232384
  },
  "maximum_target_bytes": 11325239296,
  "maximum_tmpdir_bytes": 36864,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:57:41.480253+00:00"
}
````

auth-adversary2-root-clippy

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.command.json
````json
{
  "label": "auth-adversary2-root-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "-p",
    "server",
    "-p",
    "connectors-client",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:57:41.895769+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.log
````text
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `dev` profile [unoptimized] target(s) in 5.41s
````

Collected result: 
````json
{
  "label": "auth-adversary2-root-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19590459392,
    "tmpfs_free_bytes": 13231939584,
    "mem_available_bytes": 35053535232
  },
  "maximum_target_bytes": 11322671104,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:57:49.082566+00:00"
}
````

auth-adversary2-root-fmt

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.command.json
````json
{
  "label": "auth-adversary2-root-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:57:49.488340+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.log
````text

````

Collected result: 
````json
{
  "label": "auth-adversary2-root-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19586740224,
    "tmpfs_free_bytes": 13231939584,
    "mem_available_bytes": 35411484672
  },
  "maximum_target_bytes": 11322671104,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:57:51.324508+00:00"
}
````

auth-adversary2-runtime-full

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.command.json
````json
{
  "label": "auth-adversary2-runtime-full",
  "argv": [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--offline",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:57:51.728986+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
   Compiling identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
   Compiling integration-gitlab v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-gitlab)
    Finished `test` profile [unoptimized] target(s) in 16.12s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connect_session_transport-946dac6669d3f30d)

running 24 tests
test oauth::tests::callback_query_is_strict_bounded_and_distinguishes_unknown_state ... ok
test oauth::tests::already_expired_instruction_request_refuses_before_reading_or_writing ... ok
test oauth::tests::fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths ... ok
test oauth::tests::request_parser_requires_exact_host_get_origin_form_and_bounded_headers ... ok
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test tests::unsafe_directory_refuses ... ok
test oauth::tests::fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener ... ok
test oauth::tests::expiry_caps_a_stalled_read_and_future_drop_closes_the_port ... ok
test oauth::tests::liveness_observer_uses_original_receiver_deadline_without_polling_receive ... ok
test oauth::tests::device_bridge_shows_only_human_instructions_and_has_no_callback ... ok
test oauth::tests::liveness_observer_drop_and_rebind_cannot_revive_old_receiver ... ok
test oauth::tests::liveness_observer_is_retired_after_matching_callback ... ok
test oauth::tests::device_deadline_is_capped_by_private_authorization_expiry ... ok
test oauth::tests::already_accepted_replay_cannot_survive_the_callback_claim ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test oauth::tests::local_pkce_binding_inconsistency_is_a_safe_terminal_refusal ... ok
test oauth::tests::callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url ... ok
test oauth::tests::oauth_and_raw_completion_endpoints_remain_independent ... ok
test oauth::tests::request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok
test oauth::tests::callback_origin_is_optional_but_exact_if_present_and_denial_retires_session ... ok
test oauth::tests::protected_instructions_refuse_capability_and_origin_without_spending_state ... ok
test oauth::tests::accepted_connection_budget_retires_session_without_starting_a_second_flow ... ok
test oauth::tests::oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_config-8368461ac53c09bc)

running 24 tests
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_explicit_development_public_pkce ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test personal_oauth_tests::oauth_pass1_scope_ceiling_and_ttl_boundaries_survive_real_config_loading ... ok
test personal_oauth_tests::personal_oauth_configuration_refuses_nonlocal_redirects_and_implicit_custody ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/catalog_usernames.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/catalog_usernames-ec5ea063c345b894)

running 3 tests
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-6f7ffe3605286651)

running 37 tests
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::remediation_tests::auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors ... ok
test registry::remediation_tests::remediation_registry_ambiguity_and_absence_do_not_probe_any_owner ... ok
test registry::remediation_tests::auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim ... ok
test registry::remediation_tests::remediation_registry_uses_one_exact_owner_without_operation_dispatch ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_catalog_writes-dfc82ca0a1e557c9)

running 8 tests
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s

     Running tests/local_gitlab_schedules.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_gitlab_schedules-abe7b432dd56fe2f)

running 6 tests
test configured_gitlab_connection_is_the_same_passive_reference_in_both_discovery_surfaces ... ok
test schedule_words_find_only_operations_admitted_by_the_existing_write_policy ... ok
test read_only_connection_refuses_schedule_mutations_before_custody_or_egress ... ok
test adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement ... ok
test describe_exposes_the_exact_generated_input_and_output_contracts ... ok
test schedule_requests_preserve_complete_json_values_and_optional_update_omission ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s

     Running tests/one_shot_runtime.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_runtime-84c81d325d556c5c)

running 13 tests
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test auth_one_shot_v3_refuses_persistent_control_before_configuration_or_state ... ok
test final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release ... ok
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test auth_one_shot_origin_precomposition_is_typed_and_preserves_legacy_envelopes ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state ... ok
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test auth_one_shot_origin_comes_only_from_local_decisions_and_joins_shutdown ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.18s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-91fa2d33b9a20e1f)

running 8 tests
test one_shot_create_refuses_before_configuration_custody_and_listener ... ok
test unsupported_production_registration_refuses_before_readiness_or_oauth_store ... ok
test remediation_local_v2_routes_a_created_binding_and_joins_its_endpoint ... ok
test oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target ... ok
test composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness ... ok
test mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation ... ok
test auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact ... ok
test remediation_local_completion_dispatches_only_a_later_explicit_invocation ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.45s

     Running tests/rate_adversary_registry.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_registry-001066b7c1d51548)

running 1 test
test rate_adversary_registry_checks_advice_through_describe_and_invoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_secrets-e10bab4d0f5c6302)

running 1 test
test tests::wire_reference_round_trips ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/bin/migrate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_secrets_migrate-ebc16d9e9a7348de)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_state-a89faf9ef68e5455)

running 4 tests
test port_tests::the_postgres_backend_conforms ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test port_tests::the_postgres_backend_serves_grant_evaluation ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test tests::live_postgres_round_trip_is_bounded_and_atomic ... ok
test tests::state_keys_are_closed_and_bounded ... ok

test result: ok. 2 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_vault-8ff9a7fbac54efdb)

running 6 tests
test adapter::tests::only_healthy_vault_status_is_ready ... ok
test adapter::tests::vault_origin_and_role_are_closed ... ok
test prepared::tests::clean_initialize_does_not_create_an_empty_journal ... ok
test prepared::tests::a_journal_in_the_shared_store_recovers_a_committed_transaction_after_a_restart ... ok
test adapter::tests::readiness_requires_health_and_an_accepted_session_without_reading_a_credential ... ok
test prepared::tests::candidate_values_stay_in_the_secret_store_and_are_invisible_until_commit ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/identity_http-c9bce0206713f943)

running 3 tests
test adapter::tests::approval_issuance_scope_is_admitted ... ok
test adapter::tests::a_routable_plaintext_identity_origin_is_refused_in_every_build ... ok
test adapter::tests::hosted_verifier_requires_https_origin_and_closed_access_token_shape ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_catalog-c2b8b491b2143a2e)

running 101 tests
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test custody::tests::distinct_bindings_cannot_alias_the_same_reserved_credential_addresses ... ok
test custody::tests::a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks ... ok
test custody::tests::a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit ... ok
test custody::tests::a_reclaimed_publication_retains_its_original_authorization_timing ... ok
test custody::tests::a_full_decision_precedes_secret_commit_and_coherent_publication ... ok
test custody::tests::a_claimed_decision_write_can_finish_after_the_authorization_deadline ... ok
test custody::tests::a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked ... ok
test custody::tests::a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence ... ok
test custody::tests::proposal_bounds_digest_and_journal_serialization_keep_private_values_out ... ok
test custody::refresh_tests::a_timely_refresh_claim_survives_a_held_full_decision_write ... ok
test custody::refresh_tests::a_stale_refresh_handle_cannot_prepare_after_another_valid_publication ... ok
test custody::refresh_tests::waiting_for_prepare_does_not_restart_the_refresh_window ... ok
test custody::refresh_tests::refresh_of_expired_access_uses_one_timely_claim_without_a_session ... ok
test custody::refresh_tests::binding_gate_preserves_the_new_generation_for_the_waiting_operation ... ok
test custody::tests::a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization ... ok
test custody::tests::generation_exhaustion_refuses_before_io_and_resolves_its_private_guard ... ok
test custody::tests::mismatched_proposal_or_stale_prior_generation_never_prepares ... ok
test custody::tests::dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership ... ok
test custody::tests::one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation ... ok
test custody::refresh_tests::invalid_refresh_decision_timing_cannot_publish_on_reopen ... ok
test custody::refresh_tests::cancelled_refresh_store_io_holds_the_binding_gate_until_recovery ... ok
test custody::tests::replacement_preserves_identity_and_publishes_only_same_generation_evidence ... ok
test custody::tests::expiry_or_revocation_before_the_claim_aborts_without_a_decision ... ok
test custody::refresh_tests::uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session ... ok
test custody::tests::unknown_journal_or_secret_state_stays_unavailable_until_reconciled ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test oauth::tests::admitted_response_reference_is_the_actual_configured_backend_identity ... ok
test custody::refresh_tests::refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock ... ok
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok
test oauth::tests::actual_authority_revocation_before_completion_claim_aborts_prepared_credentials ... ok
test oauth::tests::actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials ... ok
test oauth::tests::actual_timely_claim_survives_full_decision_write_after_session_deadline ... ok
test oauth::tests::actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen ... ok
test oauth::tests::actual_refresh_request_is_cancelled_at_its_original_egress_budget ... ok
test oauth::tests::pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen ... ok
test oauth::tests::actual_unknown_decision_without_durable_decision_aborts_on_reopen ... ok
test oauth::tests::actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen ... ok
test oauth::tests::actual_secret_commit_before_metadata_publication_recovers_on_reopen ... ok
test oauth::tests::actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen ... ok
test oauth::tests::remediation::remediation_created_metadata_is_exact_and_does_not_publish_callable_discovery ... ok
test oauth::tests::invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress ... ok
test oauth::tests::remediation::remediation_personal_factory_binds_actual_policy_and_rechecks_current_authority ... ok
test oauth::tests::remediation::remediation_unknown_and_wrong_owner_refuse_without_credential_or_provider_work ... ok
test oauth::tests::remediation::remediation_bound_publication_rechecks_its_receiver_authority_before_custody_commit ... ok
test oauth::tests::refresh_does_not_reset_its_original_window_after_token_egress ... ok
test oauth::tests::remediation::remediation_bound_session_publishes_exact_target_and_acknowledges_once_without_dispatch ... ok
test oauth::tests::remediation::remediation_expired_status_never_swallows_an_opaque_receiver_rejection ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test oauth::tests::refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::rate_adversary_catalog_checks_binding_and_lease_before_rate_disclosure ... ok
test tests::rate_stage2_catalog_refusal_keeps_only_trusted_optional_delay ... ok
test oauth::tests::unique_binding_and_owner_refusals_happen_before_listener_session_or_egress ... ok
test oauth::tests::remediation::remediation_expired_acknowledgement_refuses_without_rolling_back_published_credentials ... ok
test oauth::tests::remediation::remediation_fresh_and_safely_refreshable_credentials_are_ready_without_refresh ... ok
test oauth::tests::adversary::oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation ... ok
test oauth::tests::wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress ... ok
test oauth::tests::requested_scope_ceiling_never_substitutes_for_observed_operation_scopes ... ok
test custody::tests::journal_capacity_is_reserved_for_publication_before_secret_prepare ... ok
test oauth::tests::rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen ... ok
test oauth::tests::unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen ... ok
test oauth::tests::successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen ... ok
test custody::tests::every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen ... ok
test oauth::tests::remediation::auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority ... ok
test custody::tests::malformed_or_inconsistent_decisions_never_recover_a_publication ... ok
test custody::tests::uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back ... ok
test oauth::tests::device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry ... ok
test custody::refresh_tests::refresh_claim_rechecks_time_authority_generation_and_captured_evidence ... ok
test oauth::tests::adversary::oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.12s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_gitlab-0d98d4dd9b3844e3)

running 42 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::git_fetch::v2::tests::capabilities_accept_optional_upload_pack_preamble_across_chunk_boundaries ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::git_fetch::v2::tests::capabilities_refuse_malformed_or_repeated_service_preambles ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_jira-83ed1a5f25510a4f)

running 12 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_kubernetes-16470799659117e3)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_mcp-dea562dbcc14fb87)

running 2 tests
test tests::changed_live_snapshot_is_refused_before_a_factory_exists ... ok
test tests::frozen_reviewed_tools_cross_connector_custody_and_egress ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_monitoring-65d9e7d55512deb4)

running 16 tests
test backend::tests::refusal_log_record_names_operation_route_and_exact_upstream_status ... ok
test backend::tests::safe_projections_drop_provider_secrets_and_redact_free_text ... ok
test backend::tests::standalone_adapter_refuses_unowned_requests_without_fallthrough ... ok
test backend::tests::readiness_checks_only_the_mandatory_credential_store ... ok
test backend::tests::hosted_federation_is_digest_bound_group_scoped_and_has_no_connect_session ... ok
test backend::tests::connect_session_uses_shared_transport_and_publishes_only_after_secret_custody ... ok
test backend::tests::failed_credential_custody_rolls_back_discovery_and_parent_state ... ok
test backend::tests::credential_custody_failure_is_distinguished_from_upstream_failures ... ok
test backend::tests::oversized_upstream_body_refuses_as_result_bound_not_unreachable ... ok
test backend::tests::concurrent_completions_publish_exactly_one_parent_connection ... ok
test backend::tests::dashboards_list_dispatches_the_documents_required_only_input_over_http ... ok
test backend::tests::mediated_alertmanager_dispatch_resolves_the_v2_api_path ... ok
test backend::tests::prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route ... ok
test backend::tests::discovery_materialization_and_query_stay_on_the_grafana_route ... ok
test backend::tests::dashboards_list_pages_upstream_with_a_bounded_limit_and_fetch_budget ... ok
test backend::tests::refused_dispatches_distinguish_upstream_status_class_from_transport ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.24s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_platform-2feb5147d7d1395a)

running 23 tests
test tests::ontology_nullable_fields_are_still_strict_after_catalog_lowering ... ok
test tests::every_declared_write_requires_external_approval ... ok
test tests::a_mutating_post_dispatch_failure_is_not_declared_retriable ... ok
test tests::browser_catalog_symbol_is_translated_into_the_closed_driver_input ... ok
test work_events::tests::cursors_events_and_replay_are_partitioned_by_tenant ... ok
test tests::every_projected_operation_has_a_response_schema ... ok
test tests::planner_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::an_unknown_workspace_binding_is_named_rather_than_reported_as_stale ... ok
test tests::work_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::a_workspace_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test tests::workspace_datasource_projects_only_the_logical_read_model ... ok
test tests::total_http_deadline_bounds_a_stalled_private_service ... ok
test tests::hosted_tenant_member_defaults_are_an_explicit_module_ceiling ... ok
test tests::search_projects_only_configured_capabilities ... ok
test tests::module_global_ids_resolve_for_declarative_ui_requirements ... ok
test tests::search_names_each_operation_once_and_never_by_its_second_name ... ok
test tests::a_write_passes_no_local_approval_gate ... ok
test tests::every_name_of_an_operation_describes_one_operation ... ok
test tests::work_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::local_work_invocation_is_constrained_to_the_configured_unix_socket ... ok
test tests::invalid_post_dispatch_output_is_audited_as_indeterminate ... ok
test tests::ontology_invocation_carries_request_bound_signed_authority ... ok
test tests::planner_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.39s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_sip-b2eb446efa1a6eea)

running 14 tests
test raw::tests::a_chosen_device_is_the_one_bound ... ok
test raw::tests::readiness_contacts_nothing ... ok
test raw::tests::a_host_with_no_sound_stack_still_composes_a_launcher ... ok
test raw::tests::the_receipt_claims_no_application_channel ... ok
test runtime::tests::stored_credential_readiness_is_value_free_and_reports_store_unavailability ... ok
test runtime::tests::missing_sip_credentials_fail_closed ... ok
test runtime::tests::stored_credentials_are_tenant_scoped_ordered_and_redacted ... ok
test runtime::tests::authority_key_must_be_an_owner_only_real_file ... ok
test backend::tests::an_unknown_session_is_not_found_and_a_refused_signal_is_reported ... ok
test backend::tests::readiness_delegates_to_the_mandatory_launcher_probe_without_launching ... ok
test backend::tests::a_binding_that_cannot_signal_refuses_rather_than_dropping_the_keypress ... ok
test backend::tests::a_signal_reaches_the_live_session_and_leaves_it_established ... ok
test backend::tests::catalog_projection_invocation_session_control_and_audit_share_one_path ... ok
test backend::tests::stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.37s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_slack-c1ea08c672ba4a9c)

running 30 tests
test backend::tests::a_local_companion_submission_is_one_bot_token_and_nothing_else ... ok
test backend::tests::a_declared_instance_name_fixes_its_identity_for_good ... ok
test backend::tests::a_credential_file_other_accounts_can_read_is_refused_rather_than_used ... ok
test backend::tests::hosted_completion_errors_separate_conflicts_from_store_outages ... ok
test backend::tests::datasource_projection_excludes_unreviewed_slack_profile_fields ... ok
test backend::tests::hosted_companion_completion_requires_distinct_app_and_bot_credentials ... ok
test backend::tests::only_the_inner_admitted_event_is_projected ... ok
test backend::tests::message_loop_guards_and_closed_event_grants_are_applied_before_storage ... ok
test backend::tests::slack_auth_test_refuses_only_explicit_invalid_credentials ... ok
test backend::tests::hosted_setup_page_requires_capability_and_distinguishes_safe_failures ... ok
test backend::tests::slack_auth_test_provider_and_transport_failures_are_unavailable ... ok
test backend::tests::socket_ticket_destination_is_closed_to_slack_tls_hosts ... ok
test tests::organization_credentials_do_not_claim_personal_oauth_is_configured ... ok
test backend::tests::operation_audit_is_durable_bounded_and_value_free ... ok
test backend::tests::describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource ... ok
test backend::tests::a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted ... ok
test backend::tests::ephemeral_open_never_starts_a_socket_mode_supervisor ... ok
test backend::tests::read_ownership_matches_describe_ownership_for_every_slack_datasource ... ok
test backend::tests::organization_bot_is_admitted_for_reads_without_an_event_channel ... ok
test backend::tests::invalid_hosted_capability_cannot_consume_a_connect_session ... ok
test backend::tests::event_is_durable_and_deduplicated_before_pull_and_replay ... ok
test backend::tests::datasource_description_lease_ignores_request_scoped_provenance ... ok
test backend::tests::slack_readiness_is_value_free_and_tracks_the_secret_store ... ok
test backend::tests::standalone_adapter_claims_only_its_connection_and_event_families ... ok
test backend::tests::hosted_sessions_expire_and_release_pending_capacity_without_submission ... ok
test backend::tests::stale_grant_metadata_cannot_reenter_any_connection_or_event_surface ... ok
test backend::tests::one_use_completion_publishes_only_value_free_connection_state ... ok
test backend::tests::rate_adversary_slack_zero_delay_settles_each_explicit_write_refusal ... ok
test backend::tests::rate_final_slack_definite_refusal_survives_terminal_audit_failure ... ok
test backend::tests::rate_stage2_slack_definite_write_refusal_is_not_an_uncertain_outcome ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/monitoring_model-ba8bcc738bad9da9)

running 4 tests
test tests::the_validator_admits_the_documents_required_only_input ... ok
test tests::the_validator_still_refuses_outside_the_documents_contract ... ok
test tests::loki_timestamps_stay_strings_and_the_refusal_names_the_encoding ... ok
test tests::prometheus_timestamps_accept_integer_epoch_seconds_beside_strings ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/state_sqlite-b1d5ce5e44bf1d75)

running 11 tests
test tests::the_in_memory_backend_conforms ... ok
test tests::full_open_refuses_unusable_paths ... ok
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::the_file_backend_conforms ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::existing_openers_keep_normal_synchronization ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::full_open_configures_wal_and_full_synchronization_on_every_open ... ok
test tests::full_commits_are_visible_before_close_and_survive_reopening ... ok
test tests::the_full_file_backend_preserves_state_and_grant_conformance ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s

     Running tests/approval_gate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/approval_gate-9774db699f1b2c77)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_replay_survives_reopening_the_database ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

   Doc-tests connect_session_transport

running 2 tests
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::OAuthCallback (line 55) - compile fail ... ok
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::BoundOAuthEndpoint (line 100) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests connectors_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_secrets

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_state

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_vault

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests identity_http

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_gitlab

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_jira

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_kubernetes

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_mcp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_monitoring

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_platform

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_sip

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_slack

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests monitoring_model

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests state_sqlite

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-runtime-full",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19582709760,
    "tmpfs_free_bytes": 13224034304,
    "mem_available_bytes": 34419838976
  },
  "maximum_target_bytes": 11344154624,
  "maximum_tmpdir_bytes": 2895872,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:58:39.676737+00:00"
}
````

auth-adversary2-runtime-clippy

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.command.json
````json
{
  "label": "auth-adversary2-runtime-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--workspace",
    "--locked",
    "--offline",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:58:40.105313+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Checking identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Checking integration-gitlab v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-gitlab)
    Finished `dev` profile [unoptimized] target(s) in 5.53s
````

Collected result: 
````json
{
  "label": "auth-adversary2-runtime-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 24257630208,
    "tmpfs_free_bytes": 13249945600,
    "mem_available_bytes": 35563753472
  },
  "maximum_target_bytes": 11330117632,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T23:58:47.574795+00:00"
}
````

auth-adversary2-runtime-no-default

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.command.json
````json
{
  "label": "auth-adversary2-runtime-no-default",
  "argv": [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--offline",
    "--no-default-features",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T23:58:47.976453+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 13.21s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connect_session_transport-946dac6669d3f30d)

running 24 tests
test oauth::tests::already_accepted_replay_cannot_survive_the_callback_claim ... ok
test oauth::tests::fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths ... ok
test oauth::tests::fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener ... ok
test oauth::tests::callback_query_is_strict_bounded_and_distinguishes_unknown_state ... ok
test oauth::tests::already_expired_instruction_request_refuses_before_reading_or_writing ... ok
test oauth::tests::device_deadline_is_capped_by_private_authorization_expiry ... ok
test oauth::tests::callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url ... ok
test oauth::tests::liveness_observer_is_retired_after_matching_callback ... ok
test oauth::tests::expiry_caps_a_stalled_read_and_future_drop_closes_the_port ... ok
test oauth::tests::liveness_observer_uses_original_receiver_deadline_without_polling_receive ... ok
test oauth::tests::callback_origin_is_optional_but_exact_if_present_and_denial_retires_session ... ok
test oauth::tests::device_bridge_shows_only_human_instructions_and_has_no_callback ... ok
test oauth::tests::liveness_observer_drop_and_rebind_cannot_revive_old_receiver ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test oauth::tests::local_pkce_binding_inconsistency_is_a_safe_terminal_refusal ... ok
test oauth::tests::oauth_and_raw_completion_endpoints_remain_independent ... ok
test oauth::tests::request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline ... ok
test oauth::tests::protected_instructions_refuse_capability_and_origin_without_spending_state ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok
test oauth::tests::request_parser_requires_exact_host_get_origin_form_and_bounded_headers ... ok
test tests::unsafe_directory_refuses ... ok
test oauth::tests::accepted_connection_budget_retires_session_without_starting_a_second_flow ... ok
test oauth::tests::oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_config-8368461ac53c09bc)

running 24 tests
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_explicit_development_public_pkce ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test personal_oauth_tests::oauth_pass1_scope_ceiling_and_ttl_boundaries_survive_real_config_loading ... ok
test personal_oauth_tests::personal_oauth_configuration_refuses_nonlocal_redirects_and_implicit_custody ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running tests/catalog_usernames.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/catalog_usernames-ec5ea063c345b894)

running 3 tests
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-b267738104b2e904)

running 37 tests
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test registry::remediation_tests::auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::remediation_tests::remediation_registry_uses_one_exact_owner_without_operation_dispatch ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::remediation_tests::remediation_registry_ambiguity_and_absence_do_not_probe_any_owner ... ok
test registry::remediation_tests::auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test registry::tests::rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_catalog_writes-d5586ddea625db2a)

running 8 tests
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s

     Running tests/local_gitlab_schedules.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_gitlab_schedules-ee9796a1607bd788)

running 6 tests
test configured_gitlab_connection_is_the_same_passive_reference_in_both_discovery_surfaces ... ok
test schedule_words_find_only_operations_admitted_by_the_existing_write_policy ... ok
test adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement ... ok
test describe_exposes_the_exact_generated_input_and_output_contracts ... ok
test read_only_connection_refuses_schedule_mutations_before_custody_or_egress ... ok
test schedule_requests_preserve_complete_json_values_and_optional_update_omission ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s

     Running tests/one_shot_runtime.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_runtime-e21658cd9cf3b194)

running 13 tests
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test auth_one_shot_origin_precomposition_is_typed_and_preserves_legacy_envelopes ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test auth_one_shot_v3_refuses_persistent_control_before_configuration_or_state ... ok
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state ... ok
test auth_one_shot_origin_comes_only_from_local_decisions_and_joins_shutdown ... ok
test final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.47s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-7e37c91150c1d49e)

running 8 tests
test one_shot_create_refuses_before_configuration_custody_and_listener ... ok
test unsupported_production_registration_refuses_before_readiness_or_oauth_store ... ok
test composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness ... ok
test remediation_local_v2_routes_a_created_binding_and_joins_its_endpoint ... ok
test oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target ... ok
test mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation ... ok
test auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact ... ok
test remediation_local_completion_dispatches_only_a_later_explicit_invocation ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s

     Running tests/rate_adversary_registry.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_registry-bb4e11169753d84f)

running 1 test
test rate_adversary_registry_checks_advice_through_describe_and_invoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_secrets-e10bab4d0f5c6302)

running 1 test
test tests::wire_reference_round_trips ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/bin/migrate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_secrets_migrate-ebc16d9e9a7348de)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_state-a89faf9ef68e5455)

running 4 tests
test port_tests::the_postgres_backend_conforms ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test port_tests::the_postgres_backend_serves_grant_evaluation ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test tests::live_postgres_round_trip_is_bounded_and_atomic ... ok
test tests::state_keys_are_closed_and_bounded ... ok

test result: ok. 2 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_vault-8ff9a7fbac54efdb)

running 6 tests
test adapter::tests::only_healthy_vault_status_is_ready ... ok
test adapter::tests::vault_origin_and_role_are_closed ... ok
test prepared::tests::clean_initialize_does_not_create_an_empty_journal ... ok
test adapter::tests::readiness_requires_health_and_an_accepted_session_without_reading_a_credential ... ok
test prepared::tests::a_journal_in_the_shared_store_recovers_a_committed_transaction_after_a_restart ... ok
test prepared::tests::candidate_values_stay_in_the_secret_store_and_are_invisible_until_commit ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/identity_http-c9bce0206713f943)

running 3 tests
test adapter::tests::approval_issuance_scope_is_admitted ... ok
test adapter::tests::a_routable_plaintext_identity_origin_is_refused_in_every_build ... ok
test adapter::tests::hosted_verifier_requires_https_origin_and_closed_access_token_shape ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_catalog-c2b8b491b2143a2e)

running 101 tests
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test custody::tests::distinct_bindings_cannot_alias_the_same_reserved_credential_addresses ... ok
test custody::tests::a_reclaimed_publication_retains_its_original_authorization_timing ... ok
test custody::tests::a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked ... ok
test custody::tests::a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks ... ok
test custody::tests::a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit ... ok
test custody::tests::a_claimed_decision_write_can_finish_after_the_authorization_deadline ... ok
test custody::tests::a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence ... ok
test custody::tests::a_full_decision_precedes_secret_commit_and_coherent_publication ... ok
test custody::tests::proposal_bounds_digest_and_journal_serialization_keep_private_values_out ... ok
test custody::refresh_tests::a_stale_refresh_handle_cannot_prepare_after_another_valid_publication ... ok
test custody::refresh_tests::a_timely_refresh_claim_survives_a_held_full_decision_write ... ok
test custody::refresh_tests::waiting_for_prepare_does_not_restart_the_refresh_window ... ok
test custody::refresh_tests::binding_gate_preserves_the_new_generation_for_the_waiting_operation ... ok
test custody::refresh_tests::refresh_of_expired_access_uses_one_timely_claim_without_a_session ... ok
test custody::tests::a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization ... ok
test custody::tests::generation_exhaustion_refuses_before_io_and_resolves_its_private_guard ... ok
test custody::tests::mismatched_proposal_or_stale_prior_generation_never_prepares ... ok
test custody::tests::one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation ... ok
test custody::tests::dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership ... ok
test custody::refresh_tests::invalid_refresh_decision_timing_cannot_publish_on_reopen ... ok
test custody::tests::replacement_preserves_identity_and_publishes_only_same_generation_evidence ... ok
test custody::refresh_tests::cancelled_refresh_store_io_holds_the_binding_gate_until_recovery ... ok
test custody::tests::expiry_or_revocation_before_the_claim_aborts_without_a_decision ... ok
test custody::refresh_tests::uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session ... ok
test custody::tests::unknown_journal_or_secret_state_stays_unavailable_until_reconciled ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test oauth::tests::admitted_response_reference_is_the_actual_configured_backend_identity ... ok
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok
test oauth::tests::actual_authority_revocation_before_completion_claim_aborts_prepared_credentials ... ok
test oauth::tests::actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials ... ok
test oauth::tests::actual_timely_claim_survives_full_decision_write_after_session_deadline ... ok
test oauth::tests::actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen ... ok
test oauth::tests::actual_unknown_decision_without_durable_decision_aborts_on_reopen ... ok
test oauth::tests::actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen ... ok
test oauth::tests::actual_refresh_request_is_cancelled_at_its_original_egress_budget ... ok
test oauth::tests::actual_secret_commit_before_metadata_publication_recovers_on_reopen ... ok
test oauth::tests::actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen ... ok
test custody::refresh_tests::refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock ... ok
test oauth::tests::pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen ... ok
test oauth::tests::remediation::remediation_created_metadata_is_exact_and_does_not_publish_callable_discovery ... ok
test oauth::tests::invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress ... ok
test oauth::tests::remediation::remediation_personal_factory_binds_actual_policy_and_rechecks_current_authority ... ok
test oauth::tests::remediation::remediation_unknown_and_wrong_owner_refuse_without_credential_or_provider_work ... ok
test oauth::tests::remediation::remediation_bound_publication_rechecks_its_receiver_authority_before_custody_commit ... ok
test oauth::tests::refresh_does_not_reset_its_original_window_after_token_egress ... ok
test oauth::tests::remediation::remediation_expired_status_never_swallows_an_opaque_receiver_rejection ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test oauth::tests::refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen ... ok
test tests::rate_adversary_catalog_checks_binding_and_lease_before_rate_disclosure ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::rate_stage2_catalog_refusal_keeps_only_trusted_optional_delay ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test oauth::tests::remediation::remediation_bound_session_publishes_exact_target_and_acknowledges_once_without_dispatch ... ok
test oauth::tests::remediation::remediation_expired_acknowledgement_refuses_without_rolling_back_published_credentials ... ok
test oauth::tests::remediation::remediation_fresh_and_safely_refreshable_credentials_are_ready_without_refresh ... ok
test oauth::tests::unique_binding_and_owner_refusals_happen_before_listener_session_or_egress ... ok
test custody::tests::journal_capacity_is_reserved_for_publication_before_secret_prepare ... ok
test oauth::tests::adversary::oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation ... ok
test oauth::tests::wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress ... ok
test oauth::tests::requested_scope_ceiling_never_substitutes_for_observed_operation_scopes ... ok
test oauth::tests::unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen ... ok
test oauth::tests::rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen ... ok
test oauth::tests::successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen ... ok
test oauth::tests::remediation::auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority ... ok
test custody::tests::every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen ... ok
test oauth::tests::device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry ... ok
test custody::tests::malformed_or_inconsistent_decisions_never_recover_a_publication ... ok
test custody::tests::uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back ... ok
test custody::refresh_tests::refresh_claim_rechecks_time_authority_generation_and_captured_evidence ... ok
test oauth::tests::adversary::oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.32s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_gitlab-0d98d4dd9b3844e3)

running 42 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::git_fetch::v2::tests::capabilities_accept_optional_upload_pack_preamble_across_chunk_boundaries ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test backend::git_fetch::v2::tests::capabilities_refuse_malformed_or_repeated_service_preambles ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_jira-83ed1a5f25510a4f)

running 12 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_kubernetes-16470799659117e3)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_mcp-dea562dbcc14fb87)

running 2 tests
test tests::changed_live_snapshot_is_refused_before_a_factory_exists ... ok
test tests::frozen_reviewed_tools_cross_connector_custody_and_egress ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_monitoring-65d9e7d55512deb4)

running 16 tests
test backend::tests::safe_projections_drop_provider_secrets_and_redact_free_text ... ok
test backend::tests::refusal_log_record_names_operation_route_and_exact_upstream_status ... ok
test backend::tests::standalone_adapter_refuses_unowned_requests_without_fallthrough ... ok
test backend::tests::readiness_checks_only_the_mandatory_credential_store ... ok
test backend::tests::failed_credential_custody_rolls_back_discovery_and_parent_state ... ok
test backend::tests::hosted_federation_is_digest_bound_group_scoped_and_has_no_connect_session ... ok
test backend::tests::connect_session_uses_shared_transport_and_publishes_only_after_secret_custody ... ok
test backend::tests::concurrent_completions_publish_exactly_one_parent_connection ... ok
test backend::tests::oversized_upstream_body_refuses_as_result_bound_not_unreachable ... ok
test backend::tests::credential_custody_failure_is_distinguished_from_upstream_failures ... ok
test backend::tests::discovery_materialization_and_query_stay_on_the_grafana_route ... ok
test backend::tests::mediated_alertmanager_dispatch_resolves_the_v2_api_path ... ok
test backend::tests::dashboards_list_dispatches_the_documents_required_only_input_over_http ... ok
test backend::tests::prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route ... ok
test backend::tests::dashboards_list_pages_upstream_with_a_bounded_limit_and_fetch_budget ... ok
test backend::tests::refused_dispatches_distinguish_upstream_status_class_from_transport ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_platform-2feb5147d7d1395a)

running 23 tests
test tests::ontology_nullable_fields_are_still_strict_after_catalog_lowering ... ok
test tests::a_mutating_post_dispatch_failure_is_not_declared_retriable ... ok
test tests::browser_catalog_symbol_is_translated_into_the_closed_driver_input ... ok
test tests::every_declared_write_requires_external_approval ... ok
test work_events::tests::cursors_events_and_replay_are_partitioned_by_tenant ... ok
test tests::every_projected_operation_has_a_response_schema ... ok
test tests::an_unknown_workspace_binding_is_named_rather_than_reported_as_stale ... ok
test tests::workspace_datasource_projects_only_the_logical_read_model ... ok
test tests::work_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::planner_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::a_workspace_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test tests::total_http_deadline_bounds_a_stalled_private_service ... ok
test tests::hosted_tenant_member_defaults_are_an_explicit_module_ceiling ... ok
test tests::a_write_passes_no_local_approval_gate ... ok
test tests::every_name_of_an_operation_describes_one_operation ... ok
test tests::module_global_ids_resolve_for_declarative_ui_requirements ... ok
test tests::search_projects_only_configured_capabilities ... ok
test tests::search_names_each_operation_once_and_never_by_its_second_name ... ok
test tests::invalid_post_dispatch_output_is_audited_as_indeterminate ... ok
test tests::planner_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::ontology_invocation_carries_request_bound_signed_authority ... ok
test tests::work_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::local_work_invocation_is_constrained_to_the_configured_unix_socket ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.08s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_sip-b2eb446efa1a6eea)

running 14 tests
test raw::tests::readiness_contacts_nothing ... ok
test raw::tests::a_chosen_device_is_the_one_bound ... ok
test runtime::tests::stored_credential_readiness_is_value_free_and_reports_store_unavailability ... ok
test raw::tests::the_receipt_claims_no_application_channel ... ok
test runtime::tests::missing_sip_credentials_fail_closed ... ok
test runtime::tests::authority_key_must_be_an_owner_only_real_file ... ok
test raw::tests::a_host_with_no_sound_stack_still_composes_a_launcher ... ok
test runtime::tests::stored_credentials_are_tenant_scoped_ordered_and_redacted ... ok
test backend::tests::a_binding_that_cannot_signal_refuses_rather_than_dropping_the_keypress ... ok
test backend::tests::readiness_delegates_to_the_mandatory_launcher_probe_without_launching ... ok
test backend::tests::an_unknown_session_is_not_found_and_a_refused_signal_is_reported ... ok
test backend::tests::a_signal_reaches_the_live_session_and_leaves_it_established ... ok
test backend::tests::catalog_projection_invocation_session_control_and_audit_share_one_path ... ok
test backend::tests::stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.41s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_slack-c1ea08c672ba4a9c)

running 30 tests
test backend::tests::hosted_companion_completion_requires_distinct_app_and_bot_credentials ... ok
test backend::tests::only_the_inner_admitted_event_is_projected ... ok
test backend::tests::a_declared_instance_name_fixes_its_identity_for_good ... ok
test backend::tests::message_loop_guards_and_closed_event_grants_are_applied_before_storage ... ok
test backend::tests::a_local_companion_submission_is_one_bot_token_and_nothing_else ... ok
test backend::tests::slack_auth_test_refuses_only_explicit_invalid_credentials ... ok
test backend::tests::hosted_completion_errors_separate_conflicts_from_store_outages ... ok
test backend::tests::hosted_setup_page_requires_capability_and_distinguishes_safe_failures ... ok
test backend::tests::datasource_projection_excludes_unreviewed_slack_profile_fields ... ok
test backend::tests::socket_ticket_destination_is_closed_to_slack_tls_hosts ... ok
test backend::tests::slack_auth_test_provider_and_transport_failures_are_unavailable ... ok
test backend::tests::a_credential_file_other_accounts_can_read_is_refused_rather_than_used ... ok
test tests::organization_credentials_do_not_claim_personal_oauth_is_configured ... ok
test backend::tests::event_is_durable_and_deduplicated_before_pull_and_replay ... ok
test backend::tests::operation_audit_is_durable_bounded_and_value_free ... ok
test backend::tests::read_ownership_matches_describe_ownership_for_every_slack_datasource ... ok
test backend::tests::a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted ... ok
test backend::tests::ephemeral_open_never_starts_a_socket_mode_supervisor ... ok
test backend::tests::hosted_sessions_expire_and_release_pending_capacity_without_submission ... ok
test backend::tests::invalid_hosted_capability_cannot_consume_a_connect_session ... ok
test backend::tests::describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource ... ok
test backend::tests::slack_readiness_is_value_free_and_tracks_the_secret_store ... ok
test backend::tests::datasource_description_lease_ignores_request_scoped_provenance ... ok
test backend::tests::organization_bot_is_admitted_for_reads_without_an_event_channel ... ok
test backend::tests::standalone_adapter_claims_only_its_connection_and_event_families ... ok
test backend::tests::stale_grant_metadata_cannot_reenter_any_connection_or_event_surface ... ok
test backend::tests::one_use_completion_publishes_only_value_free_connection_state ... ok
test backend::tests::rate_adversary_slack_zero_delay_settles_each_explicit_write_refusal ... ok
test backend::tests::rate_final_slack_definite_refusal_survives_terminal_audit_failure ... ok
test backend::tests::rate_stage2_slack_definite_write_refusal_is_not_an_uncertain_outcome ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.41s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/monitoring_model-ba8bcc738bad9da9)

running 4 tests
test tests::loki_timestamps_stay_strings_and_the_refusal_names_the_encoding ... ok
test tests::the_validator_admits_the_documents_required_only_input ... ok
test tests::prometheus_timestamps_accept_integer_epoch_seconds_beside_strings ... ok
test tests::the_validator_still_refuses_outside_the_documents_contract ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/state_sqlite-b1d5ce5e44bf1d75)

running 11 tests
test tests::full_open_refuses_unusable_paths ... ok
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::the_in_memory_backend_conforms ... ok
test tests::existing_openers_keep_normal_synchronization ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::the_file_backend_conforms ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::full_open_configures_wal_and_full_synchronization_on_every_open ... ok
test tests::full_commits_are_visible_before_close_and_survive_reopening ... ok
test tests::the_full_file_backend_preserves_state_and_grant_conformance ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s

     Running tests/approval_gate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/approval_gate-9774db699f1b2c77)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_replay_survives_reopening_the_database ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.32s

   Doc-tests connect_session_transport

running 2 tests
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::OAuthCallback (line 55) - compile fail ... ok
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::BoundOAuthEndpoint (line 100) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

   Doc-tests connectors_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_secrets

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests hosted_state

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_vault

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests identity_http

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_gitlab

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_jira

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_kubernetes

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_mcp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_monitoring

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_platform

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_sip

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_slack

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests monitoring_model

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests state_sqlite

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

No Cargo exit was collected; separate monitor observation: 
````json
{
  "exit": null,
  "runner_exit": 1,
  "cargo_exit": "uncollected",
  "remaining_process_group": [],
  "observation": {
    "at": "2026-09-06T23:59:48.980798+00:00",
    "runner_failure": "du/check_output race on removed fixture journal.db-wal; runner exited1 without collecting Cargo exit",
    "pgid": 3490203,
    "before": [],
    "signals": [],
    "after": [],
    "product_result": "uncollected exit; retained output is not a completed gate claim"
  }
}
````

Sampler stderr transcribed from the tool-delivered driver output (original Cargo log above remains unchanged): 
````text
du: cannot access '~/.cache/cw6/av2/.tmpH2V8nR/journal.db-wal': No such file or directory
Traceback (most recent call last):
  File "~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-lane.py", line 118, in <module>
    value = sample()
  File "~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-lane.py", line 93, in sample
    'tmpdir_bytes': max(int(subprocess.check_output(['du', '-s', '-B1', *flags, env['TMPDIR']]).split()[0]) for flags in ([], ['--apparent-size']))}
                    ~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-lane.py", line 93, in <genexpr>
    'tmpdir_bytes': max(int(subprocess.check_output(['du', '-s', '-B1', *flags, env['TMPDIR']]).split()[0]) for flags in ([], ['--apparent-size']))}
                            ~~~~~~~~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  File "/usr/lib/python3.14/subprocess.py", line 473, in check_output
    return run(*popenargs, stdout=PIPE, timeout=timeout, check=True,
           ~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
               **kwargs).stdout
               ^^^^^^^^^
  File "/usr/lib/python3.14/subprocess.py", line 578, in run
    raise CalledProcessError(retcode, process.args,
                             output=stdout, stderr=stderr)
subprocess.CalledProcessError: Command '['du', '-s', '-B1', '~/.cache/cw6/av2']' returned non-zero exit status 1.
Stopped after unexpected result: auth-adversary2-runtime-no-default
````

auth-adversary2-runtime-no-default-continued

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.command.json
````json
{
  "label": "auth-adversary2-runtime-no-default-continued",
  "argv": [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--offline",
    "--no-default-features",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:01:10.841500+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Finished `test` profile [unoptimized] target(s) in 0.27s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connect_session_transport-946dac6669d3f30d)

running 24 tests
test oauth::tests::fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths ... ok
test oauth::tests::already_expired_instruction_request_refuses_before_reading_or_writing ... ok
test oauth::tests::request_parser_requires_exact_host_get_origin_form_and_bounded_headers ... ok
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test oauth::tests::callback_query_is_strict_bounded_and_distinguishes_unknown_state ... ok
test tests::unsafe_directory_refuses ... ok
test oauth::tests::expiry_caps_a_stalled_read_and_future_drop_closes_the_port ... ok
test oauth::tests::fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener ... ok
test oauth::tests::liveness_observer_is_retired_after_matching_callback ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test oauth::tests::already_accepted_replay_cannot_survive_the_callback_claim ... ok
test oauth::tests::liveness_observer_uses_original_receiver_deadline_without_polling_receive ... ok
test oauth::tests::oauth_and_raw_completion_endpoints_remain_independent ... ok
test oauth::tests::callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok
test oauth::tests::local_pkce_binding_inconsistency_is_a_safe_terminal_refusal ... ok
test oauth::tests::request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline ... ok
test oauth::tests::liveness_observer_drop_and_rebind_cannot_revive_old_receiver ... ok
test oauth::tests::callback_origin_is_optional_but_exact_if_present_and_denial_retires_session ... ok
test oauth::tests::device_deadline_is_capped_by_private_authorization_expiry ... ok
test oauth::tests::device_bridge_shows_only_human_instructions_and_has_no_callback ... ok
test oauth::tests::protected_instructions_refuse_capability_and_origin_without_spending_state ... ok
test oauth::tests::accepted_connection_budget_retires_session_without_starting_a_second_flow ... ok
test oauth::tests::oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_config-8368461ac53c09bc)

running 24 tests
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_explicit_development_public_pkce ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal_oauth_tests::oauth_pass1_scope_ceiling_and_ttl_boundaries_survive_real_config_loading ... ok
test personal_oauth_tests::personal_oauth_configuration_refuses_nonlocal_redirects_and_implicit_custody ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/catalog_usernames.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/catalog_usernames-ec5ea063c345b894)

running 3 tests
test an_entry_with_no_user_half_still_reads_and_reports_none ... ok
test a_basic_credential_carries_its_user_half_beside_its_endpoints ... ok
test a_user_half_is_refused_when_it_is_empty_or_could_not_travel ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-b267738104b2e904)

running 37 tests
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::remediation_tests::remediation_registry_ambiguity_and_absence_do_not_probe_any_owner ... ok
test registry::remediation_tests::auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors ... ok
test registry::remediation_tests::auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim ... ok
test registry::remediation_tests::remediation_registry_uses_one_exact_owner_without_operation_dispatch ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test registry::tests::rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_catalog_writes-d5586ddea625db2a)

running 8 tests
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s

     Running tests/local_gitlab_schedules.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_gitlab_schedules-ee9796a1607bd788)

running 6 tests
test configured_gitlab_connection_is_the_same_passive_reference_in_both_discovery_surfaces ... ok
test schedule_words_find_only_operations_admitted_by_the_existing_write_policy ... ok
test adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement ... ok
test read_only_connection_refuses_schedule_mutations_before_custody_or_egress ... ok
test describe_exposes_the_exact_generated_input_and_output_contracts ... ok
test schedule_requests_preserve_complete_json_values_and_optional_update_omission ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.23s

     Running tests/one_shot_runtime.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_runtime-e21658cd9cf3b194)

running 13 tests
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test auth_one_shot_v3_refuses_persistent_control_before_configuration_or_state ... ok
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release ... ok
test auth_one_shot_origin_precomposition_is_typed_and_preserves_legacy_envelopes ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test auth_one_shot_origin_comes_only_from_local_decisions_and_joins_shutdown ... ok
test final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-7e37c91150c1d49e)

running 8 tests
test one_shot_create_refuses_before_configuration_custody_and_listener ... ok
test unsupported_production_registration_refuses_before_readiness_or_oauth_store ... ok
test oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target ... ok
test composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness ... ok
test remediation_local_v2_routes_a_created_binding_and_joins_its_endpoint ... ok
test mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation ... ok
test auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact ... ok
test remediation_local_completion_dispatches_only_a_later_explicit_invocation ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s

     Running tests/rate_adversary_registry.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_registry-bb4e11169753d84f)

running 1 test
test rate_adversary_registry_checks_advice_through_describe_and_invoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_secrets-e10bab4d0f5c6302)

running 1 test
test tests::wire_reference_round_trips ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/bin/migrate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_secrets_migrate-ebc16d9e9a7348de)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_state-a89faf9ef68e5455)

running 4 tests
test port_tests::the_postgres_backend_conforms ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test port_tests::the_postgres_backend_serves_grant_evaluation ... ignored, requires a PostgreSQL named by CONNECTORS_DATABASE_URL
test tests::state_keys_are_closed_and_bounded ... ok
test tests::live_postgres_round_trip_is_bounded_and_atomic ... ok

test result: ok. 2 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/hosted_vault-8ff9a7fbac54efdb)

running 6 tests
test adapter::tests::only_healthy_vault_status_is_ready ... ok
test prepared::tests::clean_initialize_does_not_create_an_empty_journal ... ok
test adapter::tests::vault_origin_and_role_are_closed ... ok
test prepared::tests::a_journal_in_the_shared_store_recovers_a_committed_transaction_after_a_restart ... ok
test adapter::tests::readiness_requires_health_and_an_accepted_session_without_reading_a_credential ... ok
test prepared::tests::candidate_values_stay_in_the_secret_store_and_are_invisible_until_commit ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/identity_http-c9bce0206713f943)

running 3 tests
test adapter::tests::a_routable_plaintext_identity_origin_is_refused_in_every_build ... ok
test adapter::tests::hosted_verifier_requires_https_origin_and_closed_access_token_shape ... ok
test adapter::tests::approval_issuance_scope_is_admitted ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_catalog-c2b8b491b2143a2e)

running 101 tests
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test custody::tests::distinct_bindings_cannot_alias_the_same_reserved_credential_addresses ... ok
test custody::tests::a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit ... ok
test custody::tests::a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked ... ok
test custody::tests::a_reclaimed_publication_retains_its_original_authorization_timing ... ok
test custody::tests::a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks ... ok
test custody::tests::a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence ... ok
test custody::tests::a_full_decision_precedes_secret_commit_and_coherent_publication ... ok
test custody::tests::a_claimed_decision_write_can_finish_after_the_authorization_deadline ... ok
test custody::tests::proposal_bounds_digest_and_journal_serialization_keep_private_values_out ... ok
test custody::refresh_tests::binding_gate_preserves_the_new_generation_for_the_waiting_operation ... ok
test custody::refresh_tests::waiting_for_prepare_does_not_restart_the_refresh_window ... ok
test custody::refresh_tests::refresh_of_expired_access_uses_one_timely_claim_without_a_session ... ok
test custody::refresh_tests::a_timely_refresh_claim_survives_a_held_full_decision_write ... ok
test custody::refresh_tests::a_stale_refresh_handle_cannot_prepare_after_another_valid_publication ... ok
test custody::tests::generation_exhaustion_refuses_before_io_and_resolves_its_private_guard ... ok
test custody::tests::a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization ... ok
test custody::tests::mismatched_proposal_or_stale_prior_generation_never_prepares ... ok
test custody::tests::dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership ... ok
test custody::tests::one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation ... ok
test custody::refresh_tests::invalid_refresh_decision_timing_cannot_publish_on_reopen ... ok
test custody::refresh_tests::cancelled_refresh_store_io_holds_the_binding_gate_until_recovery ... ok
test custody::tests::replacement_preserves_identity_and_publishes_only_same_generation_evidence ... ok
test custody::tests::expiry_or_revocation_before_the_claim_aborts_without_a_decision ... ok
test custody::refresh_tests::uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session ... ok
test custody::tests::unknown_journal_or_secret_state_stays_unavailable_until_reconciled ... ok
test custody::refresh_tests::refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test oauth::tests::admitted_response_reference_is_the_actual_configured_backend_identity ... ok
test oauth::tests::pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen ... ok
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok
test oauth::tests::actual_timely_claim_survives_full_decision_write_after_session_deadline ... ok
test oauth::tests::actual_authority_revocation_before_completion_claim_aborts_prepared_credentials ... ok
test oauth::tests::actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen ... ok
test oauth::tests::actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials ... ok
test oauth::tests::actual_refresh_request_is_cancelled_at_its_original_egress_budget ... ok
test oauth::tests::actual_unknown_decision_without_durable_decision_aborts_on_reopen ... ok
test oauth::tests::actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen ... ok
test oauth::tests::actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen ... ok
test oauth::tests::actual_secret_commit_before_metadata_publication_recovers_on_reopen ... ok
test oauth::tests::invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress ... ok
test oauth::tests::remediation::remediation_created_metadata_is_exact_and_does_not_publish_callable_discovery ... ok
test oauth::tests::remediation::remediation_personal_factory_binds_actual_policy_and_rechecks_current_authority ... ok
test oauth::tests::remediation::remediation_unknown_and_wrong_owner_refuse_without_credential_or_provider_work ... ok
test oauth::tests::refresh_does_not_reset_its_original_window_after_token_egress ... ok
test oauth::tests::remediation::remediation_expired_status_never_swallows_an_opaque_receiver_rejection ... ok
test oauth::tests::remediation::remediation_bound_publication_rechecks_its_receiver_authority_before_custody_commit ... ok
test tests::a_basic_credentials_user_half_reaches_the_resolver_from_the_entry ... ok
test tests::a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be ... ok
test tests::a_provider_with_a_fixed_base_url_needs_no_configuration ... ok
test tests::a_read_only_first_connection_does_not_hide_an_admitted_write ... ok
test tests::a_request_template_exists_for_every_operation_this_backend_would_offer ... ok
test tests::an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string ... ok
test tests::an_unnamed_entry_keeps_the_address_it_had_before_instances_existed ... ok
test tests::an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder ... ok
test tests::effect_class_comes_from_the_declaration_not_the_method ... ok
test tests::every_catalogued_provider_can_address_a_credential ... ok
test tests::every_declared_user_half_field_names_a_credential_that_actually_wants_one ... ok
test tests::rate_adversary_catalog_checks_binding_and_lease_before_rate_disclosure ... ok
test oauth::tests::remediation::remediation_expired_acknowledgement_refuses_without_rolling_back_published_credentials ... ok
test tests::rate_stage2_catalog_refusal_keeps_only_trusted_optional_delay ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test oauth::tests::refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen ... ok
test oauth::tests::unique_binding_and_owner_refusals_happen_before_listener_session_or_egress ... ok
test oauth::tests::remediation::remediation_bound_session_publishes_exact_target_and_acknowledges_once_without_dispatch ... ok
test oauth::tests::remediation::remediation_fresh_and_safely_refreshable_credentials_are_ready_without_refresh ... ok
test oauth::tests::requested_scope_ceiling_never_substitutes_for_observed_operation_scopes ... ok
test custody::tests::every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen ... ok
test oauth::tests::adversary::oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation ... ok
test oauth::tests::wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress ... ok
test oauth::tests::rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen ... ok
test oauth::tests::unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen ... ok
test oauth::tests::successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen ... ok
test custody::tests::uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back ... ok
test custody::tests::malformed_or_inconsistent_decisions_never_recover_a_publication ... ok
test oauth::tests::remediation::auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority ... ok
test custody::tests::journal_capacity_is_reserved_for_publication_before_secret_prepare ... ok
test oauth::tests::device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry ... ok
test custody::refresh_tests::refresh_claim_rechecks_time_authority_generation_and_captured_evidence ... ok
test oauth::tests::adversary::oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.84s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_gitlab-0d98d4dd9b3844e3)

running 42 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::git_fetch::v2::tests::capabilities_accept_optional_upload_pack_preamble_across_chunk_boundaries ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::v2::tests::capabilities_refuse_malformed_or_repeated_service_preambles ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_jira-83ed1a5f25510a4f)

running 12 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_kubernetes-16470799659117e3)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_mcp-dea562dbcc14fb87)

running 2 tests
test tests::changed_live_snapshot_is_refused_before_a_factory_exists ... ok
test tests::frozen_reviewed_tools_cross_connector_custody_and_egress ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_monitoring-65d9e7d55512deb4)

running 16 tests
test backend::tests::refusal_log_record_names_operation_route_and_exact_upstream_status ... ok
test backend::tests::safe_projections_drop_provider_secrets_and_redact_free_text ... ok
test backend::tests::readiness_checks_only_the_mandatory_credential_store ... ok
test backend::tests::standalone_adapter_refuses_unowned_requests_without_fallthrough ... ok
test backend::tests::failed_credential_custody_rolls_back_discovery_and_parent_state ... ok
test backend::tests::hosted_federation_is_digest_bound_group_scoped_and_has_no_connect_session ... ok
test backend::tests::connect_session_uses_shared_transport_and_publishes_only_after_secret_custody ... ok
test backend::tests::concurrent_completions_publish_exactly_one_parent_connection ... ok
test backend::tests::credential_custody_failure_is_distinguished_from_upstream_failures ... ok
test backend::tests::oversized_upstream_body_refuses_as_result_bound_not_unreachable ... ok
test backend::tests::dashboards_list_dispatches_the_documents_required_only_input_over_http ... ok
test backend::tests::prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route ... ok
test backend::tests::mediated_alertmanager_dispatch_resolves_the_v2_api_path ... ok
test backend::tests::discovery_materialization_and_query_stay_on_the_grafana_route ... ok
test backend::tests::dashboards_list_pages_upstream_with_a_bounded_limit_and_fetch_budget ... ok
test backend::tests::refused_dispatches_distinguish_upstream_status_class_from_transport ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.46s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_platform-2feb5147d7d1395a)

running 23 tests
test tests::ontology_nullable_fields_are_still_strict_after_catalog_lowering ... ok
test tests::a_mutating_post_dispatch_failure_is_not_declared_retriable ... ok
test tests::browser_catalog_symbol_is_translated_into_the_closed_driver_input ... ok
test tests::every_declared_write_requires_external_approval ... ok
test work_events::tests::cursors_events_and_replay_are_partitioned_by_tenant ... ok
test tests::every_projected_operation_has_a_response_schema ... ok
test tests::an_unknown_workspace_binding_is_named_rather_than_reported_as_stale ... ok
test tests::planner_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::a_workspace_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test tests::total_http_deadline_bounds_a_stalled_private_service ... ok
test tests::hosted_tenant_member_defaults_are_an_explicit_module_ceiling ... ok
test tests::work_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::workspace_datasource_projects_only_the_logical_read_model ... ok
test tests::search_names_each_operation_once_and_never_by_its_second_name ... ok
test tests::every_name_of_an_operation_describes_one_operation ... ok
test tests::search_projects_only_configured_capabilities ... ok
test tests::module_global_ids_resolve_for_declarative_ui_requirements ... ok
test tests::a_write_passes_no_local_approval_gate ... ok
test tests::local_work_invocation_is_constrained_to_the_configured_unix_socket ... ok
test tests::invalid_post_dispatch_output_is_audited_as_indeterminate ... ok
test tests::work_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::planner_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::ontology_invocation_carries_request_bound_signed_authority ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.86s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_sip-b2eb446efa1a6eea)

running 14 tests
test raw::tests::a_chosen_device_is_the_one_bound ... ok
test raw::tests::a_host_with_no_sound_stack_still_composes_a_launcher ... ok
test raw::tests::the_receipt_claims_no_application_channel ... ok
test raw::tests::readiness_contacts_nothing ... ok
test runtime::tests::stored_credential_readiness_is_value_free_and_reports_store_unavailability ... ok
test runtime::tests::missing_sip_credentials_fail_closed ... ok
test runtime::tests::stored_credentials_are_tenant_scoped_ordered_and_redacted ... ok
test runtime::tests::authority_key_must_be_an_owner_only_real_file ... ok
test backend::tests::a_binding_that_cannot_signal_refuses_rather_than_dropping_the_keypress ... ok
test backend::tests::readiness_delegates_to_the_mandatory_launcher_probe_without_launching ... ok
test backend::tests::an_unknown_session_is_not_found_and_a_refused_signal_is_reported ... ok
test backend::tests::a_signal_reaches_the_live_session_and_leaves_it_established ... ok
test backend::tests::catalog_projection_invocation_session_control_and_audit_share_one_path ... ok
test backend::tests::stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_slack-c1ea08c672ba4a9c)

running 30 tests
test backend::tests::a_local_companion_submission_is_one_bot_token_and_nothing_else ... ok
test backend::tests::a_declared_instance_name_fixes_its_identity_for_good ... ok
test backend::tests::datasource_projection_excludes_unreviewed_slack_profile_fields ... ok
test backend::tests::hosted_companion_completion_requires_distinct_app_and_bot_credentials ... ok
test backend::tests::hosted_completion_errors_separate_conflicts_from_store_outages ... ok
test backend::tests::hosted_setup_page_requires_capability_and_distinguishes_safe_failures ... ok
test backend::tests::message_loop_guards_and_closed_event_grants_are_applied_before_storage ... ok
test backend::tests::only_the_inner_admitted_event_is_projected ... ok
test backend::tests::a_credential_file_other_accounts_can_read_is_refused_rather_than_used ... ok
test backend::tests::slack_auth_test_provider_and_transport_failures_are_unavailable ... ok
test backend::tests::slack_auth_test_refuses_only_explicit_invalid_credentials ... ok
test backend::tests::socket_ticket_destination_is_closed_to_slack_tls_hosts ... ok
test tests::organization_credentials_do_not_claim_personal_oauth_is_configured ... ok
test backend::tests::event_is_durable_and_deduplicated_before_pull_and_replay ... ok
test backend::tests::describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource ... ok
test backend::tests::a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted ... ok
test backend::tests::ephemeral_open_never_starts_a_socket_mode_supervisor ... ok
test backend::tests::invalid_hosted_capability_cannot_consume_a_connect_session ... ok
test backend::tests::datasource_description_lease_ignores_request_scoped_provenance ... ok
test backend::tests::operation_audit_is_durable_bounded_and_value_free ... ok
test backend::tests::hosted_sessions_expire_and_release_pending_capacity_without_submission ... ok
test backend::tests::organization_bot_is_admitted_for_reads_without_an_event_channel ... ok
test backend::tests::slack_readiness_is_value_free_and_tracks_the_secret_store ... ok
test backend::tests::read_ownership_matches_describe_ownership_for_every_slack_datasource ... ok
test backend::tests::standalone_adapter_claims_only_its_connection_and_event_families ... ok
test backend::tests::stale_grant_metadata_cannot_reenter_any_connection_or_event_surface ... ok
test backend::tests::one_use_completion_publishes_only_value_free_connection_state ... ok
test backend::tests::rate_adversary_slack_zero_delay_settles_each_explicit_write_refusal ... ok
test backend::tests::rate_final_slack_definite_refusal_survives_terminal_audit_failure ... ok
test backend::tests::rate_stage2_slack_definite_write_refusal_is_not_an_uncertain_outcome ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.71s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/monitoring_model-ba8bcc738bad9da9)

running 4 tests
test tests::prometheus_timestamps_accept_integer_epoch_seconds_beside_strings ... ok
test tests::loki_timestamps_stay_strings_and_the_refusal_names_the_encoding ... ok
test tests::the_validator_still_refuses_outside_the_documents_contract ... ok
test tests::the_validator_admits_the_documents_required_only_input ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/state_sqlite-b1d5ce5e44bf1d75)

running 11 tests
test tests::full_open_refuses_unusable_paths ... ok
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::the_in_memory_backend_conforms ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::the_file_backend_conforms ... ok
test tests::existing_openers_keep_normal_synchronization ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::full_open_configures_wal_and_full_synchronization_on_every_open ... ok
test tests::full_commits_are_visible_before_close_and_survive_reopening ... ok
test tests::the_full_file_backend_preserves_state_and_grant_conformance ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s

     Running tests/approval_gate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/approval_gate-9774db699f1b2c77)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok
test a_replay_survives_reopening_the_database ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

   Doc-tests connect_session_transport

running 2 tests
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::OAuthCallback (line 55) - compile fail ... ok
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::BoundOAuthEndpoint (line 100) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests connectors_config

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests connectors_runtime

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_secrets

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_state

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hosted_vault

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests identity_http

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_catalog

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_gitlab

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_jira

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_kubernetes

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_mcp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_monitoring

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_platform

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_sip

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests integration_slack

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests monitoring_model

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests state_sqlite

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-runtime-no-default-continued",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17474670592,
    "tmpfs_free_bytes": 13248524288,
    "mem_available_bytes": 32680869888
  },
  "maximum_target_bytes": 11333328896,
  "maximum_tmpdir_bytes": 17776640,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:01:48.390624+00:00"
}
````

auth-adversary2-runtime-no-default-clippy

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.command.json
````json
{
  "label": "auth-adversary2-runtime-no-default-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--workspace",
    "--locked",
    "--offline",
    "--no-default-features",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:01:48.796792+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `dev` profile [unoptimized] target(s) in 2.61s
````

Collected result: 
````json
{
  "label": "auth-adversary2-runtime-no-default-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 20168364032,
    "tmpfs_free_bytes": 13249937408,
    "mem_available_bytes": 34543947776
  },
  "maximum_target_bytes": 11333328896,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:01:52.813287+00:00"
}
````

auth-adversary2-runtime-fmt

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.command.json
````json
{
  "label": "auth-adversary2-runtime-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:01:53.242808+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.log
````text

````

Collected result: 
````json
{
  "label": "auth-adversary2-runtime-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19844825088,
    "tmpfs_free_bytes": 13249937408,
    "mem_available_bytes": 34778394624
  },
  "maximum_target_bytes": 11333328896,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:01:56.111695+00:00"
}
````

auth-adversary2-console-full

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.command.json
````json
{
  "label": "auth-adversary2-console-full",
  "argv": [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--offline",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:01:56.520009+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.log
````text
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 4.69s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_console-c3fd591076a9af80)

running 76 tests
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test connect::personal_oauth_tests::instruction_file_refuses_shared_parent_symlink_and_existing_content ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test envelope::personal_oauth_tests::ordinary_connection_result_payload_has_no_private_instruction_endpoint ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test input::tests::an_inline_object_is_parsed ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test connect::personal_oauth_tests::instruction_file_is_exclusive_owner_only_and_cleared_on_drop ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test connect::personal_oauth_tests::instruction_cleanup_never_removes_a_replacement_inode ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok

test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s

     Running tests/adversary_budget_prose.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_budget_prose-4e62c7b456388460)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s

     Running tests/adversary_readability.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_readability-9377cf56efde0b31)

running 7 tests
test render_helper_child ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s

     Running tests/adversary_readability_pass2.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_readability_pass2-c18999e78fd39843)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-04eff4149fab18d6)

running 11 tests
test ambiguous_profile_refuses_before_output_file_or_daemon_connection ... ok
test doctor_retains_ordinary_credential_store_diagnostic_for_an_owner_only_config ... ok
test doctor_reports_exact_redirect_and_unsealed_custody_without_client_material ... ok
test unsafe_private_destination_refuses_before_any_daemon_connection ... ok
test headless_oauth_requires_private_file_before_session_creation ... ok
test explicit_private_file_is_reserved_before_create_and_erased_before_public_success ... ok
test successful_private_daemon_label_cannot_reach_the_public_summary ... ok
test oauth_pass1_cancellation_erases_already_written_private_inode_before_return ... ok
test private_daemon_refusal_is_closed_on_stdout_and_stderr_in_all_formats ... ok
test oauth_pass1_real_controlling_pty_handoff_keeps_redirected_outputs_private ... ok
test oauth_pass2_private_file_expires_while_completion_grace_stays_bounded ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.20s

     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-f1ff96957d29feb1)

running 7 tests
test auth_stage2_hostile_output_child ... ok
test auth_stage2_bound_input_is_bounded_and_errors_do_not_echo_values ... ok
test auth_stage2_valid_hostile_daemon_output_is_private_in_every_real_format ... ok
test auth_stage2_bound_presenter_refuses_before_session_or_output ... ok
test auth_adversary_presenter_error_clears_original_inode_after_path_replacement ... ok
test auth_adversary2_instruction_fetch_cancellation_clears_reserved_destination_before_poll ... ok
test auth_stage2_bound_presenter_clears_written_inode_on_success_expiry_and_drop ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-console-full",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17648795648,
    "tmpfs_free_bytes": 13249286144,
    "mem_available_bytes": 33437597696
  },
  "maximum_target_bytes": 11333980160,
  "maximum_tmpdir_bytes": 49152,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:02:29.341321+00:00"
}
````

auth-adversary2-console-clippy

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.command.json
````json
{
  "label": "auth-adversary2-console-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--workspace",
    "--locked",
    "--offline",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:02:29.761019+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.log
````text
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Finished `dev` profile [unoptimized] target(s) in 2.91s
````

Collected result: 
````json
{
  "label": "auth-adversary2-console-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17637445632,
    "tmpfs_free_bytes": 13253484544,
    "mem_available_bytes": 33583460352
  },
  "maximum_target_bytes": 11333980160,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:02:33.708704+00:00"
}
````

auth-adversary2-console-fmt

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.command.json
````json
{
  "label": "auth-adversary2-console-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:02:34.123334+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.log
````text

````

Collected result: 
````json
{
  "label": "auth-adversary2-console-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17633681408,
    "tmpfs_free_bytes": 13253484544,
    "mem_available_bytes": 34044063744
  },
  "maximum_target_bytes": 11333980160,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:02:37.090944+00:00"
}
````

auth-adversary2-cli-full

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.command.json
````json
{
  "label": "auth-adversary2-cli-full",
  "argv": [
    "cargo",
    "test",
    "--workspace",
    "--locked",
    "--offline",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:02:37.680291+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 10.94s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_cli-c13928f076fe4de5)

running 5 tests
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::slack_connect_needs_no_internal_reference_or_path_argument ... ok
test tests::normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing ... ok
test tests::every_supported_shell_gets_a_script_naming_the_whole_surface ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/main.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors-d810b0b842aea7e2)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_cli_cap_pass3.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_cli_cap_pass3-e05a39da68cbbb63)

running 1 test
test the_cap_the_design_page_says_is_measured_is_declared_and_asserted ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_fence_probe.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_fence_probe-2958f05cacfaeb2e)

running 6 tests
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

     Running tests/adversary_fence_probe_pass2.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_fence_probe_pass2-97569d95a35575d8)

running 3 tests
test every_file_the_committed_contract_opens_is_a_file_a_clone_has ... ok
test the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence ... ok
test the_paths_the_design_document_says_send_no_protocol_request_send_none ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/adversary_shim_pass3.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_shim_pass3-6019c2ab0de31545)

running 6 tests
test the_serve_group_advertises_a_help_subcommand ... ok
test a_help_path_of_the_new_tree_under_serve_is_left_alone ... ok
test connectors_help_still_answers_for_a_path_that_moved ... ok
test a_moved_path_typed_with_the_global_output_flag_still_works ... ok
test the_group_word_whose_only_command_moved_still_points_somewhere ... ok
test nothing_this_product_prints_names_a_moved_path_behind_a_global_flag ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.82s

     Running tests/adversary_shim_pass4.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_shim_pass4-fdd3b6d12af54d38)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running tests/adversary_shim_pass5.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_shim_pass5-1d13e928db1131f9)

running 4 tests
test a_positional_value_spelled_help_is_a_value_not_a_help_request ... ok
test an_argument_neither_the_group_nor_the_leaf_declares_is_not_the_old_leaf ... ok
test the_double_dash_escape_is_not_a_word_that_moved ... ok
test serve_with_only_global_options_is_the_group_in_every_spelling_and_position ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli_surface.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/cli_surface-5b3ef5afe4176a12)

running 36 tests
test every_kind_of_exception_is_used_and_every_entry_gives_a_reason ... ok
test every_named_exception_is_still_a_path_of_the_parser ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test personal_oauth_setup_requires_explicit_profile_and_private_instruction_option ... ok
test no_path_is_both_declared_and_excepted ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test every_citation_this_unit_wrote_resolves ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok
test oauth_pass1_cli_private_setup_refusal_closes_all_output_formats_and_clears_file ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok

test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.08s

     Running tests/cli_surface_drift.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/cli_surface_drift-6d0250b71da89e03)

running 10 tests
test the_copied_declarations_are_still_copies ... ok
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/cli_surface_pass_two-cfe8de2fffcacee5)

running 6 tests
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/closed_pipe-faf5d0d83388d04c)

running 20 tests
test completion_scripts_still_accept_a_closed_reader ... ok
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test admin_authentication_failures_remain_unsuccessful_with_a_closed_reader ... ok
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok
test a_closed_transport_stays_unsuccessful ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... ok
test a_healthy_doctor_accepts_a_closed_report_reader ... ok
test compact_consumer_closes_early ... ok
test every_admin_leaf_preserves_non_broken_pipe_output_failures ... ok
test setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal ... ok
test json_consumer_closes_early ... ok
test successful_admin_credential_write_accepts_a_closed_reader ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... ok
test text_consumer_closes_early ... ok
test yaml_consumer_closes_early ... ok
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test each_unhealthy_report_class_keeps_its_failure_when_output_closes ... ok
test protocol_refusals_keep_their_failure_when_a_result_reader_closes ... ok
test each_protocol_search_distinguishes_closed_readers_from_other_write_failures ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.71s

     Running tests/first_level_groups.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/first_level_groups-dafc97dfb2159b1d)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s

     Running tests/moved_paths_are_not_taught.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/moved_paths_are_not_taught-521e266820c15e14)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

     Running tests/one_shot_operations.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_operations-dd9b3dc280dfc31c)

running 23 tests
test a_transport_that_drops_the_request_is_never_retried_locally ... ok
test a_running_daemon_is_used_without_constructing_a_local_runtime ... ok
test doctor_enumerates_bounded_and_persistent_verbs ... ok
test connection_mutations_require_daemon_before_creating_continuation_state ... ok
test adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime ... ok
test existing_or_unsafe_socket_objects_never_trigger_ephemeral_fallback ... ok
test events_and_session_signals_name_the_persistent_daemon_requirement ... ok
test adversary_json_source_and_size_refusals_precede_one_shot_state_creation ... ok
test rate_final_cli_describe_spelling_and_invalid_advice_never_resend ... ok
test invalid_bounds_and_unsafe_state_refuse_before_runtime_state_is_opened ... ok
test rate_adversary_cli_keeps_integer_extremes_and_never_resends_before_exit ... ok
test rate_stage2_cli_json_and_yaml_preserve_delay_and_never_resend_an_invoke ... ok
test final_adversary_kubernetes_candidates_never_publish_a_dead_connection_or_run_auth_exec ... ok
test rate_stage2_one_shot_refusals_preserve_retriable_without_inventing_delay ... ok
test ordinary_search_and_connection_list_use_default_paths_without_a_daemon ... ok
test separate_describe_and_invoke_processes_reuse_the_same_authority_without_a_daemon ... ok
test final_adversary_invalid_provider_output_is_not_resent_and_releases_the_state_root ... ok
test adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears ... ok
test concurrent_commands_and_daemon_start_cannot_take_the_in_flight_invocation_state ... ok
test a_changed_authority_or_selected_connection_never_reaches_fixture_egress ... ok
test final_adversary_provider_cursor_survives_two_distinct_one_shot_processes ... ok
test adversary_caller_input_cannot_rebind_routes_or_revoked_grants ... ok
test browser_session_operations_are_refused_under_canonical_and_published_aliases ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.93s

     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-9806fb21acc5f093)

running 9 tests
test auth_stage2_bound_setup_requires_daemon_before_input_or_private_file ... ok
test auth_stage2_bound_grammar_pairs_targets_and_preserves_existing_provider_mode ... ok
test auth_stage2_operation_versions_select_the_real_exchange_without_resend ... ok
test auth_stage2_unknown_operation_version_refuses_before_input_or_socket ... ok
test auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination ... ok
test auth_stage2_real_cli_auth_refusals_keep_every_format_private_without_resend ... ok
test auth_stage2_v3_refusal_keeps_failure_and_privacy_with_open_or_closed_output ... ok
test auth_stage2_bound_overrides_refuse_before_waiting_for_stdin ... ok
test auth_adversary2_cli_socket_permissions_refuse_before_open_stdin_in_every_format ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s

     Running tests/search_bounds.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/search_bounds-03ef24390f00ba96)

running 4 tests
test every_search_help_names_its_protocol_range_and_existing_default ... ok
test every_search_preserves_its_default_and_accepts_both_protocol_edges ... ok
test every_search_parser_refuses_zero_and_values_above_the_protocol_maximum ... ok
test invalid_search_limits_exit_before_target_configuration_or_transport ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Collected result: 
````json
{
  "label": "auth-adversary2-cli-full",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17518350336,
    "tmpfs_free_bytes": 13248512000,
    "mem_available_bytes": 32710074368
  },
  "maximum_target_bytes": 11338428416,
  "maximum_tmpdir_bytes": 2080768,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:03:32.485225+00:00"
}
````

auth-adversary2-cli-clippy

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.command.json
````json
{
  "label": "auth-adversary2-cli-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--workspace",
    "--locked",
    "--offline",
    "--all-targets",
    "--",
    "-D",
    "warnings"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:03:33.319230+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Checking identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Checking connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `dev` profile [unoptimized] target(s) in 17.84s
````

Collected result: 
````json
{
  "label": "auth-adversary2-cli-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17211097088,
    "tmpfs_free_bytes": 13252448256,
    "mem_available_bytes": 31773130752
  },
  "maximum_target_bytes": 11334361088,
  "maximum_tmpdir_bytes": 1396736,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:03:53.578821+00:00"
}
````

auth-adversary2-cli-fmt

Exact command/environment: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.command.json
````json
{
  "label": "auth-adversary2-cli-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av2",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg0Bq2TOf:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-07T00:03:54.397722+00:00"
}
````

Complete original Cargo/stdout/stderr log: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.log
````text

````

Collected result: 
````json
{
  "label": "auth-adversary2-cli-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17848692736,
    "tmpfs_free_bytes": 13252448256,
    "mem_available_bytes": 33677651968
  },
  "maximum_target_bytes": 11334361088,
  "maximum_tmpdir_bytes": 1396736,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-07T00:04:00.482479+00:00"
}
````

3. Finding and actual reachability.

| File:line | Verdict / origin | Measured | What reaches it |
| --- | --- | --- | --- |
| crates/server/src/hosted/docs/openapi.json:730 | INFEASIBLE / undecided | The served Operation409schema rejects selected v1/v2 stale_authority envelopes. New assertion at tests/remediation.rs:705 exits101 alone and in the full root suite. | Real POST /operations, real grant store, synthetic verifier and remediation metadata backend; caller supplies a retired description_ref. Current built-in hosted backends do not produce that remediation metadata, so no current production hosted path was found. |

The served Operation 409 schema rejects the actual selected v1/v2 stale_authority envelopes from a grant-admitted stale description.

The test first proves current grant admission, then reaches the actual fresh-description comparison before readiness. Narrow status projection correctly keeps this non-authentication conflict409. Strict decoders prove exact selected identities and stale_authority/non-retriable/no-authentication bodies. The served409schema names only the v3 response root. All v1/v2 missing/degraded/outage503 controls and all v3 controls satisfy their actual schemas. Matrix fields printed by the runner and fields proven by assertions are distinguished in hosted-status-body-schema-matrix.json; raw HTTP frames were not separately printed.

Production reachability is limited explicitly: service defaults return Unsupported, the registry preserves the ordinary owner's default, and the only implemented PersonalOAuthBackend metadata owner is installed in personal composition. Current hosted composition builds the ordinary hosted backends; a custom embedding can supply the synthetic port used here. No live hosted acquisition, credential publication, authority bypass or operation dispatch is demonstrated. The source can inform coordinator routing, but origin remains undecided because no base execution was assigned or run. Detailed caller anchors are in finding-reachability.json/current-callers.txt. No original first-review disposition was rewritten.

4. Attacks that stayed green.

- Fresh schema references/composition validate captured input after exact same binding/profile acknowledgement; hostile description strings stay inside the trusted workflow.
- Stalled acknowledgement and description responses stop at the captured deadline; no implicit Invoke or resend occurs.
- Session ambiguity/absence and selected-owner failures never fall through to another backend or ordinary dispatch.
- Cancellation during private instruction fetch clears the reserved inode before any polling, even with a partial private body.
- Actual CLI permission refusal precedes open stdin, daemon connection and private destination creation in all four formats.
- Retained full suites keep strict vector/version/downgrade readers, real grant/approval ordering, one-use acknowledgement/current authority, bound-owner isolation, private CLI/console/MCP projection and explicit-only resumption assertions. Hosted acquisition remains Unsupported and OAuth is not reopened.

5. Resources, scope and all outside paths.

````json
{
  "minimum": {
    "disk_free_bytes": 14567911424,
    "tmpfs_free_bytes": 13171638272,
    "mem_available_bytes": 31773130752
  },
  "maximum_target_bytes": 11526176768,
  "maximum_tmpdir_bytes": 17776640,
  "guard_crossings": [],
  "commands_with_collected_exits": 22,
  "monitor_failure_attempts": 1,
  "remaining_process_groups": [],
  "limits": "disk12GiB/tmpfs8GiB/MemAvailable16GiB, targetcap12GiB/TMPcap128MiB, one-second samples; no first-pass reserve exception carried forward",
  "limitation": "No continuous measurement or collected Cargo exit is claimed after the original no-default sampler exception; that attempt is separately retained and excluded from completed gates."
}
````

Compiler release preceded sealing: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/compile-slot-release.json. All owned process groups are empty; source and target were held stable. The first pass's limited8GiB disk exception did not carry forward. No target cleanup or shared-cache cleanup occurred.

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/source-preservation.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/tests.patch

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-source.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/command-index.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/runner-counts.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/prior-seal-verification.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-target.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-executables.json

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-tmp-inventory.json

Assigned roots and existing tool-managed bookkeeping paths:
````text
/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target
~/.cache/cw6/av2
~/.cargo/.global-cache
~/.cargo/.package-cache
~/.cargo/.package-cache-mutate
````

Every retained scratch path, including this immutable report pair and seal:
````text
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/attack-plan-and-count-provenance.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-corrected.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-cli-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-deadline-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-client-schema-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-console-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-hosted-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-registry-selected.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-root-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default-continued.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.source-proof.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/auth-adversary2-runtime-no-default.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/brief.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/candidate-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/cli-first-complete.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/cli-first-source.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/cli-placement-observation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/cli-suffix-formatted.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/cli-suffix.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/client-suffix-formatted.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/client-suffix.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/command-index.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/compile-slot-release.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/console-suffix-formatted.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/console-suffix.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/corrected-pre-full-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/corrected-pre-full-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/correction-and-publication.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/current-callers.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/deciding-case-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/diffstat.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/evidence.sha256
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-executables.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/final-tmp-inventory.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/finding-reachability.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/findings.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/first-selection-source.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/hosted-status-body-schema-matrix.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/outside-paths.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/owned-test-paths.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/pre-full-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/pre-full-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preflight.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/connectors-cli/tests/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/connectors-client/src/tests.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/connectors-console/tests/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/connectors-runtime/src/remediation_tests.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/connectors-runtime/tests/one_shot_runtime.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/connectors-runtime/tests/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/integration-catalog/src/oauth_remediation_tests.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/protocol/tests/bundles.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/server/src/hosted/tests/docs.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/preimage/crates/server/src/hosted/tests/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/prior-seal-verification.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/raw-report.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/report.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/resource-summary.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-full-checks.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-lane-continuation.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-lane.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/run-remaining-checks.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/runner-counts.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/runtime-no-default-monitor-failure.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/runtime-no-default-runner-stderr.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/runtime-suffix-formatted.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/runtime-suffix.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/seal-pass2.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/server-suffix-formatted.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/server-suffix.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/source-preservation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-2/whole-auth.patch
````

Target/temp descendants are enumerated exactly above; normal fixture teardown may retire transient descendants. The manifest seals every retained scratch file except itself.

```findings
- file: "crates/server/src/hosted/docs/openapi.json"
  line: 730
  category: "contract-drift"
  severity: "warning"
  verdict: "INFEASIBLE"
  origin: "undecided"
  message: "The served Operation 409 schema rejects the actual selected v1/v2 stale_authority envelopes from a grant-admitted stale description."
```
