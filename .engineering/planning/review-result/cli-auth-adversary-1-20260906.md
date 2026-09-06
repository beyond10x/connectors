---
format: aep.planning-md/1
id: review-result:cli-auth-adversary-1-20260906
kind: review-result
status: active
title: First whole authentication-remediation adversarial review
relations:
- reviews: story:auth-as-tool-result
revision: 1
---
unit: auth-as-tool-result, formal pass 1; 8815dea456171dc28e3641668a3d7c50829b1b91 plus retained tests.patch
verdict: NEEDS-CHANGE
cases: executed 1008→1016, red 3
origin: introduced 0 / pre-existing 0 / undecided 3
wrote-outside-worktree: 210 retained reviewer scratch paths; assigned target/TMPDIR and tool-managed cache paths below
needs-coordinator: yes — record this immutable first pass, route findings with their measured reachability/origin limits, and own later integration/publication gates

````text
 crates/connectors-cli/tests/remediation.rs         |  59 ++++++
 crates/connectors-client/src/tests.rs              | 105 +++++++++++
 crates/connectors-console/tests/remediation.rs     |  97 ++++++++++
 crates/connectors-runtime/src/remediation_tests.rs | 101 +++++++++++
 crates/connectors-runtime/tests/personal_oauth.rs  | 201 +++++++++++++++++++++
 .../src/oauth_remediation_tests.rs                 |  75 ++++++++
 crates/protocol/tests/bundles.rs                   |  65 +++++++
 crates/server/src/hosted/tests/remediation.rs      |  78 ++++++++
 8 files changed, 781 insertions(+)
````

This first ordinary auth pass covers the frozen candidate, its whole delta from 95fad7c70556a7178cf94070fa5a43657ba9f72c, and earlier protocol 05c94ac457938d2f9a8059f7f905dd8a87ec4dca and service ae53a93092453266cd5b40a5c6f9cee484478310 slices. The unchanged reviewed OAuth prerequisite remains retained. All ten original test files are exact byte prefixes; eight files gained eight cases and no production/planning file changed. Maximum changed test length: 1,353 lines. This is an agent review with runner observations, not architecture acceptance or human approval.

The public copy replaces only the absolute local home-directory prefix with ~. Artifact references are plain/code paths; no live provider or integration action occurred.

1. Deciding cases: each selected alone before any full suite.

| Case | Assertion | First/final result |
| --- | --- | --- |
| auth-adversary1-protocol-first | Strict decoders driven from the unit vectors reject escaped duplicate identity/correlation keys and over-budget padded frames. | green, exit 0, exactly one test executed |
| auth-adversary1-purpose-first | Four canonically valid daemon DTO paths test matching, legacy absent, conflicting and split-pair credential purposes after exactly five completion exchanges. | red, exit 101, exactly one test executed |
| auth-adversary1-hosted-first | Actual hosted router, synthetic IdentityVerifier principal fixture and real GrantSet/MemoryState test selected response schemas and grant revocation before readiness/side effects. | red, exit 101, exactly one test executed |
| auth-adversary1-readiness-first | Actual personal OAuth owner classifies deleted access, expired/deleted refresh and revoked authority without refreshing or invoking. | green, exit 0, exactly one test executed |
| auth-adversary1-local-first | Actual local daemon, registry and OAuth custody keep two same-profile configured bindings separate through completion, wrong-owner status, wrong-target ack and discovery. | green, exit 0, exactly one test executed |
| auth-adversary1-registry-first | Actual registry refuses split/ambiguous owners and preserves claimed-owner Refused/Unavailable without dispatch. | green, exit 0, exactly one test executed |
| auth-adversary1-presenter-first | Actual presenter reserves before Start and clears its original private inode on error after path rename/replacement; synthetic servers are joined. | green, exit 0, exactly one test executed |
| auth-adversary1-cli-first | Real CLI with stdin held open refuses an absent socket; existing regular-file and symlink paths block until fixture timeout/kill/join; no private destination is opened. | red, exit 101, exactly one test executed |

Own placement precheck: the protocol addition first matched an earlier context. Before execution, the exact-prefix check failed; only the new addition was moved to EOF. Original placement patch and observation remain at ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/layout-precheck-tests.patch and ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/layout-precheck-observation.json. The first formatter path was absent (exit 127), recorded at ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/format-tool-observation.json. The available formatter later formatted only new suffixes, so the first purpose failure's line differs from the final formatted source. No old assertion changed. A report-construction tool syntax error occurred before tool execution and before report creation; it is preserved separately and is not a product or test result.

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.command.json
````json
{
  "label": "auth-adversary1-protocol-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "protocol",
    "--test",
    "bundles",
    "auth_adversary_original_vector_bytes_reject_escaped_duplicates_and_padded_overflow",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:11:01.024835+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.log
````text
   Compiling zerovec v0.11.7
   Compiling tinystr v0.8.4
   Compiling icu_locale_core v2.3.0
   Compiling zerotrie v0.2.5
   Compiling potential_utf v0.1.6
   Compiling icu_collections v2.3.0
   Compiling icu_provider v2.3.0
   Compiling icu_normalizer v2.3.0
   Compiling icu_properties v2.3.0
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling url v2.5.8
   Compiling clap_derive v4.6.4
   Compiling jsonschema v0.49.9
   Compiling clap v4.6.6
   Compiling protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
    Finished `test` profile [unoptimized] target(s) in 23.36s
     Running tests/bundles.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/bundles-5746c3cdd81e920e)

running 1 test
test auth_adversary_original_vector_bytes_reject_escaped_duplicates_and_padded_overflow ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out; finished in 0.10s

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.result.json
````json
{
  "label": "auth-adversary1-protocol-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 25453383680,
    "tmpfs_free_bytes": 16120930304,
    "mem_available_bytes": 35768135680
  },
  "maximum_target_bytes": 8499310592,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:11:25.932543+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.command.json
````json
{
  "label": "auth-adversary1-purpose-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connectors-client",
    "auth_adversary_fresh_operation_binding_rejects_conflicting_purpose",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:17:16.545198+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.log
````text
   Compiling futures-util v0.3.34
   Compiling tokio v1.53.1
   Compiling hyper v1.11.0
   Compiling hyper-util v0.1.20
   Compiling tower v0.5.3
   Compiling ahash v0.8.12
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling secret-service v5.2.0
   Compiling tower-http v0.6.11
   Compiling jsonschema-value v0.49.9
   Compiling referencing v0.49.9
   Compiling zbus-secret-service-keyring-store v1.0.1
   Compiling reqwest v0.12.28
   Compiling jsonschema v0.49.9
   Compiling identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
   Compiling keyring v4.2.0
   Compiling axum v0.8.9
   Compiling tempfile v3.27.0
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Finished `test` profile [unoptimized] target(s) in 38.49s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_client-df4e5f6918aa1036)

running 1 test
valid DTO purpose observations after exactly five exchanges each: [("matching", true), ("legacy-absent", true), ("conflicting", true), ("split-pair", true)]

thread 'tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose' (2337380) panicked at crates/connectors-client/src/tests.rs:939:5:
assertion `left == right` failed: an explicitly conflicting purpose must not be accepted as fresh matching binding
  left: [("matching", true), ("legacy-absent", true), ("conflicting", true), ("split-pair", true)]
 right: [("matching", true), ("legacy-absent", true), ("conflicting", false), ("split-pair", false)]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose ... FAILED

failures:

failures:
    tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 41 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p connectors-client --lib`
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.result.json
````json
{
  "label": "auth-adversary1-purpose-first",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 19909464064,
    "tmpfs_free_bytes": 15981367296,
    "mem_available_bytes": 36909510656
  },
  "maximum_target_bytes": 8628424704,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:17:56.309344+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.command.json
````json
{
  "label": "auth-adversary1-hosted-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "server",
    "auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:19:17.453479+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.log
````text
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Finished `test` profile [unoptimized] target(s) in 5.85s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-034f8d953ecabd82)

running 1 test
actual hosted selected response status/schema observations: [("V0Alpha1", 409, false), ("V0Alpha2", 409, false), ("V0Alpha3", 409, true)]

thread 'hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation' (2355178) panicked at crates/server/src/hosted/tests/remediation.rs:614:5:
served OpenAPI must admit the actual selected response at its actual HTTP status
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation ... FAILED

failures:

failures:
    hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 113 filtered out; finished in 0.87s

error: test failed, to rerun pass `-p server --lib`
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.result.json
````json
{
  "label": "auth-adversary1-hosted-first",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 17463406592,
    "tmpfs_free_bytes": 15959281664,
    "mem_available_bytes": 38681968640
  },
  "maximum_target_bytes": 8648458240,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:19:25.353396+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.command.json
````json
{
  "label": "auth-adversary1-readiness-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "integration-catalog",
    "auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:20:54.595894+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.log
````text
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 7.17s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_catalog-a3501834c45a2493)

running 1 test
actual personal owner readiness after custody/authority changes: [("access-missing", CredentialDegraded), ("refresh-missing", CredentialDegraded), ("authority-revoked", Unsupported)]
test oauth::tests::remediation::auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 100 filtered out; finished in 1.82s

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.result.json
````json
{
  "label": "auth-adversary1-readiness-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 15602323456,
    "tmpfs_free_bytes": 15982903296,
    "mem_available_bytes": 37007183872
  },
  "maximum_target_bytes": 8636674048,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:21:05.440515+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.command.json
````json
{
  "label": "auth-adversary1-local-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connectors-runtime",
    "--test",
    "personal_oauth",
    "auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:22:03.970965+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 3.18s
     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-c919cd09486b298f)

running 1 test
test auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 1.38s

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.result.json
````json
{
  "label": "auth-adversary1-local-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 18948722688,
    "tmpfs_free_bytes": 15968661504,
    "mem_available_bytes": 38449721344
  },
  "maximum_target_bytes": 8647872512,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:22:09.895109+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.command.json
````json
{
  "label": "auth-adversary1-registry-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "connectors-runtime",
    "auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:23:26.265599+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 5.56s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-684bebcc28bafc34)

running 1 test
actual registry owner/error observations: [("ordinary", "unsupported", 1, 0), ("split", "refused", 0, 0), ("ambiguous-ordinary", "unavailable", 0, 0), ("claimed-refused", "refused", 1, 0), ("claimed-unavailable", "unavailable", 1, 0)]
test registry::remediation_tests::auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 35 filtered out; finished in 0.00s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_catalog_writes-a9b24bcddccc6fae)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

     Running tests/local_gitlab_schedules.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_gitlab_schedules-b4e24eb63370b9ec)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s

     Running tests/one_shot_runtime.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_runtime-8657971482304daa)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.00s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-c919cd09486b298f)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s

     Running tests/rate_adversary_registry.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_registry-19aa25acf46dd869)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.result.json
````json
{
  "label": "auth-adversary1-registry-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17742618624,
    "tmpfs_free_bytes": 15982448640,
    "mem_available_bytes": 38355410944
  },
  "maximum_target_bytes": 8649842688,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:23:33.113319+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.command.json
````json
{
  "label": "auth-adversary1-presenter-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--test",
    "remediation",
    "auth_adversary_presenter_error_clears_original_inode_after_path_replacement",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:24:29.160758+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.log
````text
   Compiling syn v3.0.3
   Compiling serde_derive v1.0.229
   Compiling serde v1.0.229
   Compiling zerovec-derive v0.11.5
   Compiling zerovec v0.11.8
   Compiling displaydoc v0.2.7
   Compiling once_cell v1.21.4
   Compiling tinystr v0.8.4
   Compiling icu_locale_core v2.3.0
   Compiling zerotrie v0.2.5
   Compiling potential_utf v0.1.6
   Compiling rustix v1.1.4
   Compiling icu_collections v2.3.0
   Compiling icu_provider v2.3.0
   Compiling linux-raw-sys v0.12.1
   Compiling serde_core v1.0.229
   Compiling winnow v1.0.4
   Compiling parking v2.2.1
   Compiling icu_normalizer v2.3.0
   Compiling icu_properties v2.3.0
   Compiling thiserror-impl v2.0.20
   Compiling thiserror v2.0.20
   Compiling idna_adapter v1.2.2
   Compiling generic-array v0.14.7
   Compiling hashbrown v0.17.1
   Compiling crossbeam-utils v0.8.22
   Compiling indexmap v2.14.0
   Compiling idna v1.1.0
   Compiling toml_parser v1.1.3+spec-1.1.0
   Compiling async-trait v0.1.92
   Compiling toml_datetime v1.1.1+spec-1.1.0
   Compiling ref-cast v1.0.27
   Compiling toml_edit v0.25.13+spec-1.1.0
   Compiling crypto-common v0.1.7
   Compiling block-buffer v0.10.4
   Compiling enumflags2_derive v0.7.12
   Compiling ref-cast-impl v1.0.27
   Compiling tokio-macros v2.7.2
   Compiling futures-io v0.3.34
   Compiling tokio v1.53.1
   Compiling num v0.4.3
   Compiling digest v0.10.7
   Compiling concurrent-queue v2.5.0
   Compiling proc-macro-crate v3.5.0
   Compiling zvariant_utils v4.2.0
   Compiling event-listener v5.4.2
   Compiling futures-macro v0.3.34
   Compiling futures-util v0.3.34
   Compiling event-listener-strategy v0.5.4
   Compiling zvariant_derive v5.15.0
   Compiling futures-lite v2.6.1
   Compiling url v2.5.8
   Compiling ring v0.17.14
   Compiling tracing-core v0.1.36
   Compiling endi v1.1.1
   Compiling tracing v0.1.44
   Compiling sha2 v0.10.9
   Compiling async-io v2.6.0
   Compiling fluent-uri v0.4.1
   Compiling ahash v0.8.12
   Compiling parking_lot_core v0.9.12
   Compiling enumflags2 v0.7.12
   Compiling zcheapstr v1.1.0
   Compiling block-padding v0.4.2
   Compiling polling v3.11.0
   Compiling rustls-pki-types v1.15.1
   Compiling getrandom v0.2.17
   Compiling errno v0.3.14
   Compiling serde_derive_internals v0.30.0
   Compiling async-task v4.7.1
   Compiling fraction v0.15.4
   Compiling schemars_derive v1.2.2
   Compiling signal-hook-registry v1.4.8
   Compiling inout v0.2.2
   Compiling zvariant v5.15.0
   Compiling parking_lot v0.12.5
   Compiling async-channel v2.5.0
   Compiling connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-state)
   Compiling rustls v0.23.43
   Compiling jsonschema-value v0.49.9
   Compiling referencing v0.49.9
   Compiling schemars v1.2.2
   Compiling zbus_names v4.3.4
   Compiling cipher v0.5.2
   Compiling async-signal v0.2.14
   Compiling rustls-webpki v0.103.14
   Compiling async-lock v3.4.2
   Compiling piper v0.2.5
   Compiling connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-address)
   Compiling futures-channel v0.3.34
   Compiling email_address v0.2.9
   Compiling jsonschema v0.49.9
   Compiling connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-secrets)
   Compiling hyper v1.11.0
   Compiling blocking v1.7.0
   Compiling async-process v2.5.0
   Compiling zbus_macros v5.19.0
   Compiling domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/domain)
   Compiling async-executor v1.14.0
   Compiling async-broadcast v0.7.2
   Compiling ordered-stream v0.2.0
   Compiling uuid v1.26.0
   Compiling async-recursion v1.1.1
   Compiling serde_repr v0.1.21
   Compiling cpubits v0.1.1
   Compiling ipnet v2.12.1
   Compiling tower v0.5.3
   Compiling hyper-util v0.1.20
   Compiling catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog)
   Compiling aes v0.9.2
   Compiling zbus v5.19.0
   Compiling curve25519-dalek v4.1.3
   Compiling hkdf v0.13.0
   Compiling protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
   Compiling tokio-rustls v0.26.4
   Compiling cbc v0.2.1
   Compiling webpki-roots v1.0.9
   Compiling toml_datetime v0.6.11
   Compiling serde_spanned v0.6.9
   Compiling toml_edit v0.22.27
   Compiling hyper-rustls v0.27.9
   Compiling secret-service v5.2.0
   Compiling ed25519-dalek v2.2.0
   Compiling connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-resolve)
   Compiling tower-http v0.6.11
   Compiling serde_urlencoded v0.7.1
   Compiling keyring-core v1.0.0
   Compiling rusqlite v0.37.0
   Compiling reqwest v0.12.28
   Compiling zbus-secret-service-keyring-store v1.0.1
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
   Compiling toml v0.8.23
   Compiling connector-oauth v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-oauth)
   Compiling connect-session-transport v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport)
   Compiling connectors-config v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-config)
   Compiling keyring v4.2.0
   Compiling identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
   Compiling state-sqlite v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/state-sqlite)
   Compiling clap_derive v4.6.4
   Compiling rtoolbox v0.0.5
   Compiling unsafe-libyaml-norway v0.2.15
   Compiling serde_norway v0.9.42
   Compiling rpassword v7.5.4
   Compiling clap v4.6.6
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
   Compiling tempfile v3.27.0
    Finished `test` profile [unoptimized] target(s) in 1m 26s
     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-f1ff96957d29feb1)

running 1 test
test auth_adversary_presenter_error_clears_original_inode_after_path_replacement ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 1.10s

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.result.json
````json
{
  "label": "auth-adversary1-presenter-first",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 17143132160,
    "tmpfs_free_bytes": 15389691904,
    "mem_available_bytes": 37545672704
  },
  "maximum_target_bytes": 9237909504,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:25:58.081622+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.command.json
````json
{
  "label": "auth-adversary1-cli-first",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "--test",
    "remediation",
    "auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination",
    "--",
    "--nocapture"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:26:38.008908+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.log
````text
   Compiling syn v3.0.4
   Compiling serde_derive v1.0.229
   Compiling serde v1.0.229
   Compiling log v0.4.34
   Compiling mio v1.2.3
   Compiling tokio-macros v2.7.2
   Compiling tokio v1.53.1
   Compiling cc v1.4.4
   Compiling displaydoc v0.2.7
   Compiling futures-macro v0.3.34
   Compiling futures-util v0.3.34
   Compiling indexmap v2.14.1
   Compiling thiserror-impl v2.0.20
   Compiling thiserror v2.0.20
   Compiling zerovec-derive v0.11.6
   Compiling zerovec v0.11.8
   Compiling ipnet v2.12.1
   Compiling tracing v0.1.44
   Compiling serde_json v1.0.151
   Compiling tinystr v0.8.4
   Compiling async-trait v0.1.92
   Compiling icu_locale_core v2.3.0
   Compiling potential_utf v0.1.6
   Compiling zerotrie v0.2.5
   Compiling ring v0.17.14
   Compiling icu_collections v2.3.0
   Compiling icu_provider v2.3.1
   Compiling cmake v0.1.58
   Compiling aws-lc-sys v0.45.0
   Compiling icu_properties v2.3.0
   Compiling icu_normalizer v2.3.0
   Compiling idna_adapter v1.2.2
   Compiling idna v1.1.0
   Compiling aws-lc-rs v1.18.1
   Compiling url v2.5.8
   Compiling rustls v0.23.43
   Compiling rustls-webpki v0.103.15
   Compiling tokio-util v0.7.19
   Compiling tokio-rustls v0.26.4
   Compiling uuid v1.26.0
   Compiling hyper v1.11.1
   Compiling hyper-util v0.1.20
   Compiling tower v0.5.3
   Compiling ref-cast-impl v1.0.27
   Compiling ref-cast v1.0.27
   Compiling tower-http v0.6.11
   Compiling hyper-rustls v0.27.9
   Compiling ahash v0.8.12
   Compiling num v0.4.3
   Compiling serde_urlencoded v0.7.1
   Compiling connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-state)
   Compiling serde_derive_internals v0.30.0
   Compiling schemars_derive v1.2.2
   Compiling reqwest v0.12.28
   Compiling cpufeatures v0.3.1
   Compiling schemars v1.2.2
   Compiling fluent-uri v0.4.1
   Compiling fraction v0.15.4
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling uuid-simd v0.8.0
   Compiling domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/domain)
   Compiling connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-address)
   Compiling email_address v0.2.9
   Compiling referencing v0.49.9
   Compiling connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-secrets)
   Compiling jsonschema-value v0.49.9
   Compiling jsonschema v0.49.9
   Compiling catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog)
   Compiling protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
   Compiling connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-resolve)
   Compiling service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
   Compiling chacha20 v0.10.2
   Compiling rand v0.10.2
   Compiling webrtc-util v0.12.0
   Compiling futures-executor v0.3.34
   Compiling futures v0.3.34
   Compiling toml_datetime v0.6.11
   Compiling serde_spanned v0.6.9
   Compiling toml_edit v0.22.27
   Compiling either v1.18.0
   Compiling toml_edit v0.25.13+spec-1.1.0
   Compiling toml v0.8.23
   Compiling connectors-config v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-config)
   Compiling proc-macro-crate v3.5.0
   Compiling zvariant_utils v4.2.0
   Compiling concurrent-queue v2.5.0
   Compiling sha2 v0.11.0
   Compiling asn1-rs v0.6.2
   Compiling zvariant_derive v5.15.0
   Compiling connector-oauth v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-oauth)
   Compiling tungstenite v0.28.0
   Compiling prefix-trie v0.8.4
   Compiling hickory-proto v0.26.1
   Compiling oid-registry v0.7.1
   Compiling der-parser v9.0.0
   Compiling enumflags2 v0.7.12
   Compiling zcheapstr v1.1.0
   Compiling rtp v0.14.0
   Compiling sha1 v0.11.0
   Compiling quinn-proto v0.11.17
   Compiling moka v0.12.16
   Compiling async-io v2.6.0
   Compiling tungstenite v0.30.0
   Compiling zvariant v5.15.0
   Compiling x509-parser v0.16.0
   Compiling hickory-net v0.26.1
   Compiling quinn-udp v0.5.15
   Compiling async-channel v2.5.0
   Compiling stun v0.9.0
   Compiling sipx-sip v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling rtcp v0.14.0
   Compiling libsqlite3-sys v0.35.0
   Compiling hickory-resolver v0.26.1
   Compiling webrtc-mdns v0.10.0
   Compiling tokio-tungstenite v0.28.0
   Compiling webrtc-srtp v0.16.0
   Compiling rcgen v0.13.2
   Compiling turn v0.11.0
   Compiling quinn v0.11.11
   Compiling zbus_names v4.3.4
   Compiling tokio-tungstenite v0.30.0
   Compiling async-signal v0.2.14
   Compiling webrtc-sctp v0.13.0
   Compiling bincode v1.3.3
   Compiling postgres-protocol v0.6.12
   Compiling dtls v0.13.0
   Compiling blocking v1.7.0
   Compiling async-process v2.5.0
   Compiling webrtc-data v0.12.0
   Compiling sdp v0.10.0
   Compiling sipx-transport v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling zbus_macros v5.19.0
   Compiling webrtc-ice v0.14.0
   Compiling interceptor v0.15.0
   Compiling async-executor v1.14.0
   Compiling webrtc-media v0.11.0
   Compiling sipx-rtp v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling sipx-sdp v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling fluent-uri v0.3.2
   Compiling sipx-audio v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling smol_str v0.2.2
   Compiling serde_repr v0.1.21
   Compiling webrtc v0.14.0
   Compiling aes v0.9.2
   Compiling zbus v5.19.0
   Compiling sipx-ua v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling sipx-media v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling serde-value v0.7.0
   Compiling referencing v0.33.0
   Compiling k8s-openapi v0.28.0
   Compiling postgres-types v0.2.14
   Compiling connect-session-transport v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport)
   Compiling clap_derive v4.6.4
   Compiling rustls-platform-verifier v0.7.0
   Compiling serde-saphyr v0.0.27
   Compiling rusqlite v0.37.0
   Compiling tokio-postgres v0.7.18
   Compiling reqwest v0.13.4
   Compiling clap v4.6.6
   Compiling jsonschema v0.33.0
   Compiling secret-service v5.2.0
   Compiling kube-core v4.0.0
   Compiling process-wrap v9.1.0
   Compiling jsonpath-rust v1.0.10
   Compiling sipx-call v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
   Compiling rtvbp v0.1.0 (https://github.com/babelforce/rtvbp?rev=dc0a60f7425b4899885f372152028457791b1e72#dc0a60f7)
   Compiling driver-audio v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-audio)
   Compiling axum-core v0.5.6
   Compiling sse-stream v0.2.5
   Compiling hyper-timeout v0.5.2
   Compiling chrono v0.4.45
   Compiling tokio-stream v0.1.19
   Compiling keyring-core v1.0.0
   Compiling axum v0.8.9
   Compiling rmcp v3.2.0
   Compiling kube-client v4.0.0
   Compiling zbus-secret-service-keyring-store v1.0.1
   Compiling rtvbp-voice-endpoint v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/rtvbp-voice-endpoint)
   Compiling driver-sip v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-sip)
   Compiling postgres v0.19.14
   Compiling state-sqlite v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/state-sqlite)
   Compiling subscription-custody v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/subscription-custody)
   Compiling monitoring-model v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/monitoring-model)
   Compiling b10x-mcp-types v0.1.1 (https://github.com/beyond10x/mcp?rev=cb3b13a37dfef645ddfe916adc3cb40e82b7f620#cb3b13a3)
   Compiling b10x-mcp-client v0.1.1 (https://github.com/beyond10x/mcp?rev=cb3b13a37dfef645ddfe916adc3cb40e82b7f620#cb3b13a3)
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
   Compiling hosted-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/hosted-state)
   Compiling voice-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/voice-runtime)
   Compiling keyring v4.2.0
   Compiling kube v4.0.0
   Compiling driver-speech v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-speech)
   Compiling driver-cdp v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-cdp)
   Compiling hosted-vault v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/hosted-vault)
   Compiling voice-local-audio v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/voice-local-audio)
   Compiling identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
   Compiling rtoolbox v0.0.6
   Compiling serde_norway v0.9.42
   Compiling rpassword v7.5.4
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
   Compiling integration-sip v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-sip)
   Compiling hosted-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/hosted-secrets)
   Compiling integration-platform v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-platform)
   Compiling integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-kubernetes)
   Compiling identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
   Compiling integration-mcp v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-mcp)
   Compiling integration-monitoring v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-monitoring)
   Compiling integration-slack v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-slack)
   Compiling integration-jira v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-jira)
   Compiling integration-gitlab v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-gitlab)
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
   Compiling clap_complete v4.6.9
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 3m 39s
     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-9806fb21acc5f093)

running 1 test
real CLI with stdin kept open; timed-out fixture children were killed and joined: [("absent", false, Some(1)), ("regular", true, None), ("symlink", true, None)]

thread 'auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination' (2493897) panicked at tests/remediation.rs:590:5:
an existing non-socket or symlink must be refused before blocking on caller stdin
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination ... FAILED

failures:

failures:
    auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 6.02s

error: test failed, to rerun pass `--test remediation`
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.result.json
````json
{
  "label": "auth-adversary1-cli-first",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 26511577088,
    "tmpfs_free_bytes": 13922185216,
    "mem_available_bytes": 36287922176
  },
  "maximum_target_bytes": 10709254144,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:30:23.905644+00:00"
}
````

2. Full affected suites and strict checks, after all deciding cases.

Baseline comes from the implementor logs indexed in ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/before-counts.json. No pre-addition suite ran. Root retained packages: protocol 77, service 68, server 115, client 46 (306 total); runtime 455, console 107, CLI 140. Alternate-feature runtime repeats and isolated selections are excluded from the unique total. Two old PostgreSQL cases remain ignored in each runtime configuration. This is affected closure, not a complete twelve-workspace CI claim.

| Lane | Before → executed | Passed / failed / ignored | Exit |
| --- | --- | --- | --- |
| root-full | 306 → 309 | 307 / 2 / 0 | 101 |
| runtime-full | 455 → 458 | 458 / 0 / 2 | 0 |
| console-full | 107 → 108 | 108 / 0 / 0 | 0 |
| cli-full | 140 → 141 | 140 / 1 / 0 | 101 |
| runtime-no-default | 455 → 458 | 458 / 0 / 2 | 0 |

The console-fmt preflight refusal under the original 12 GiB disk floor is retained as its own command/result; the coordinator prospectively assigned an 8 GiB disk reserve solely for the remaining tmpfs-backed commands, with unchanged target/tmpfs/memory limits and a new 128 MiB TMPDIR cap sampled each second (publication runbook revision 37; resource-continuation-assignment.json); it launched no Cargo and invalidates no completed test. Any prospective resource supplement and later successful completion use separate retained command labels. No historical limit or failed output was rewritten.

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.command.json
````json
{
  "label": "auth-adversary1-root-full",
  "argv": [
    "cargo",
    "test",
    "--locked",
    "--offline",
    "-p",
    "protocol",
    "-p",
    "service",
    "-p",
    "server",
    "-p",
    "connectors-client",
    "--no-fail-fast"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:31:15.057959+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.log
````text
   Compiling server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
   Compiling connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
   Compiling protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
    Finished `test` profile [unoptimized] target(s) in 12.67s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_client-8b1fc807c3113e01)

running 42 tests
test identity::tests::keyring_account_contains_no_endpoint_or_principal ... ok
test identity::tests::mcp_invocation_uses_only_the_invoke_scope ... ok
test identity::tests::hosted_request_families_select_the_smallest_available_scope ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_destination_is_exact_numeric_loopback_and_capability_is_header_only ... ok
test personal_oauth::personal_oauth_tests::personal_instruction_parser_preserves_optional_device_uri_and_refuses_origin_changes ... ok
test personal_oauth::personal_oauth_tests::actual_private_instruction_request_keeps_capability_out_of_url_and_delivers_only_to_human_writer ... ok
test hosted_catalog::tests::posts_and_validates_a_catalog_frame ... ok
test personal_oauth::personal_oauth_tests::private_browser_authorization_is_never_a_provider_independent_redirect ... ok
test admin::tests::identity_pkce_exchange_returns_only_the_exact_access_credential ... ok
test admin::tests::named_resources_are_typed_and_the_value_is_not_exposed ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_session ... ok
test personal_oauth::personal_oauth_tests::expired_monotonic_instruction_budget_never_connects_even_with_future_wall_deadline ... ok
test personal_oauth::personal_oauth_tests::personal_create_refusal_cannot_echo_private_daemon_text ... ok
test personal_oauth::personal_oauth_tests::valid_browser_only_session_reaches_the_explicit_personal_handoff ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_integration_status ... ok
test git_fetch_client::tests::response_is_bound_to_the_request_and_source_authority_is_redacted ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_private_target_repeated_consistently ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_degraded_description ... ok
test personal_oauth::personal_oauth_tests::personal_success_retains_a_callable_correlated_description ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_describe_target ... ok
test personal_oauth::personal_oauth_tests::personal_poll_and_describe_refusals_close_inner_transport_and_daemon_text ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_another_credential_purpose ... ok
test personal_oauth::personal_oauth_tests::actual_instruction_redirect_and_cacheable_reply_are_closed_refusals ... ok
test tests::completion_endpoint_is_validated_before_secret_submission ... ok
test tests::hosted_client_requires_https_except_on_loopback_or_internal_cluster_dns ... ok
test tests::auth_stage2_hosted_409_is_typed_without_resend ... ok
test tests::hosted_client_posts_and_validates_a_datasource_frame ... ok
test tests::local_client_frames_and_correlates_an_operation ... ok
test tests::hosted_subscription_client_refuses_a_cacheable_credential_boundary ... ok
test tests::hosted_client_posts_the_same_typed_operation_frame ... ok
test tests::hosted_subscription_client_redacts_and_redeems_one_attempt_capability ... ok
test tests::hosted_subscription_client_starts_and_completes_pkce_without_retaining_the_code ... ok
test response::tests::rate_stage2_hosted_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_created_description ... ok
test personal_oauth::personal_oauth_tests::personal_success_refuses_a_changed_deadline ... ok
test tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose ... FAILED
test identity::tests::login_separates_the_session_and_refreshes_exact_scope_tokens ... ok
test identity::tests::auth_stage2_identity_409_does_not_renew_or_resend ... ok
test tests::auth_stage2_bound_completion_rechecks_target_schema_and_never_invokes ... ok
test personal_oauth::personal_oauth_tests::actual_connection_v1_polling_keeps_guarded_completion_private_and_never_repeats_create ... ok
test response::tests::rate_stage2_local_client_never_resends_after_any_received_refusal_or_invalid_reply ... ok
test tests::auth_stage2_local_versions_are_strict_without_resend ... ok

failures:

---- tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose stdout ----
valid DTO purpose observations after exactly five exchanges each: [("matching", true), ("legacy-absent", true), ("conflicting", true), ("split-pair", true)]

thread 'tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose' (2504811) panicked at crates/connectors-client/src/tests.rs:972:5:
assertion `left == right` failed: an explicitly conflicting purpose must not be accepted as fresh matching binding
  left: [("matching", true), ("legacy-absent", true), ("conflicting", true), ("split-pair", true)]
 right: [("matching", true), ("legacy-absent", true), ("conflicting", false), ("split-pair", false)]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    tests::auth_adversary_fresh_operation_binding_rejects_conflicting_purpose

test result: FAILED. 41 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s

error: test failed, to rerun pass `-p connectors-client --lib`
     Running tests/personal_oauth_adversary.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth_adversary-ac6fedc7b1c855b9)

running 2 tests
test oauth_pass1_public_instruction_refusals_do_not_redirect_poll_or_echo_private_bytes ... ok
test oauth_pass1_public_handoff_waits_for_bound_callable_success_without_replay ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.90s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/protocol-97038e98e9ec497b)

running 37 tests
test audio::tests::empty_control_bearing_and_over_length_text_refuse ... ok
test browser::tests::open_admits_an_absent_address_and_goto_does_not ... ok
test audio::tests::bounded_single_line_text_is_admitted_and_counted_in_characters ... ok
test browser::tests::only_ordinary_web_addresses_are_admitted ... ok
test approval::tests::approval_lifetime_is_bounded ... ok
test browser::tests::the_admitted_surface_carries_no_interaction_operation ... ok
test audio::tests::the_input_refuses_any_field_a_caller_invents ... ok
test browser::tests::the_input_refuses_any_field_a_caller_invents ... ok
test catalog::tests::a_setup_profile_cannot_exist_without_a_setup_form ... ok
test catalog::tests::catalog_membership_carries_no_callability_or_credential_value ... ok
test browser::tests::a_page_view_cannot_be_built_without_its_untrusted_content_label ... ok
test approval::tests::realm_is_not_an_approval_or_route_coordinate ... ok
test connection::tests::observations_are_value_free_and_lifecycle_consistent ... ok
test connection::tests::materialization_accepts_only_an_opaque_observation_reference ... ok
test connection::tests::pending_and_completed_sessions_cannot_mix_endpoint_and_connection ... ok
test datasource::tests::list_is_bounded_and_get_key_is_structured ... ok
test datasource::tests::response_refuses_ambiguous_success_and_failure ... ok
test connection::tests::browser_completion_url_is_an_exact_loopback_capability ... ok
test git_fetch::tests::request_refuses_unbounded_or_ambiguous_revisions ... ok
test git_fetch::tests::response_refuses_secret_or_non_tls_locators ... ok
test operation::legacy::tests::a_terminal_status_cannot_omit_its_observed_reason ... ok
test sip::tests::a_number_alone_is_admitted_because_the_trunk_supplies_the_destination ... ok
test operation::legacy::tests::effect_bearing_operations_require_approval ... ok
test sip::tests::a_number_that_could_escape_the_uri_user_part_is_refused ... ok
test sip::tests::a_number_is_bounded_rather_than_truncated ... ok
test sip::tests::aliases_admit_names_and_refuse_network_destinations ... ok
test sip::tests::an_absent_field_is_omitted_from_the_wire_rather_than_sent_as_null ... ok
test event::tests::cursor_and_wait_are_bounded ... ok
test operation::legacy::tests::invoke_requires_a_description_lease_and_bounded_structured_input ... ok
test operation::legacy::tests::owner_context_is_not_defaultable ... ok
test voice::tests::fixture_context_cannot_claim_trust ... ok
test operation::legacy::tests::connection_audiences_are_bounded_discovery_metadata ... ok
test voice::tests::owner_vectors_are_closed_and_unique ... ok
test connection::tests::mediated_route_is_value_free_closed_and_cannot_self_reference ... ok
test connection::tests::candidates_are_value_free_and_activation_selects_no_route ... ok
test operation::legacy::tests::response_envelope_round_trips_and_refuses_unknown_fields ... ok
test connection::tests::secret_shaped_unknown_fields_are_refused ... ok

test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/bundles.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/bundles-f0b59fa93166e973)

running 39 tests
test connector_catalog_bundle_is_immutable ... ok
test connector_connection_bundle_is_immutable ... ok
test connector_catalog_vectors_match_the_strict_reader ... ok
test connector_datasource_bundle_is_immutable ... ok
test connector_connection_vectors_match_the_strict_reader ... ok
test connector_event_vectors_match_the_strict_reader ... ok
test connector_operation_bundle_is_immutable ... ok
test connector_datasource_vectors_match_the_strict_reader ... ok
test connector_operation_vectors_match_the_strict_reader ... ok
test kubernetes_service_route_round_trips_through_the_connection_response ... ok
test connector_event_bundle_is_immutable ... ok
test operation_v2_advice_preserves_all_categories_and_checks_the_interval ... ok
test operation_v2_only_rate_limited_carries_retry_delay ... ok
test operation_v2_projects_throttling_to_v1_without_optional_extensions ... ok
test connector_operation_v2_bundle_is_immutable ... ok
test authentication_predecessor_artifacts_and_readers_stay_frozen ... ok
test operation_version_reader_rejects_unknown_versions_and_preserves_authority_fields ... ok
test owner_contract_bundle_is_immutable ... ok
test authentication_bundle_manifests_match_all_published_bytes ... ok
test rate_repair_source_uri_grammar_matches_wire_and_published_schema ... ok
test authentication_downgrade_is_exact_neutral_and_non_retriable ... ok
test rtvbp_binding_bundle_is_immutable ... ok
test operation_version_decoder_refuses_duplicate_fields_before_dispatch ... ok
test operation_v2_description_downgrade_loses_only_advice ... ok
test authentication_bound_requests_cannot_downgrade_to_unbound_creation ... ok
test operation_v2_vectors_cover_every_request_result_and_error_variant ... ok
test operation_v2_complete_request_and_response_vectors_match_rust ... ok
test operation_legacy_snapshot_preserves_deployed_schema_discrepancies ... ok
test authentication_vectors_cover_every_request_result_and_error_variant ... ok
test authentication_v3_inherits_every_v2_vector_and_schema_constraint ... ok
test authentication_version_adapters_preserve_ordinary_and_rate_frames ... ok
test authentication_new_payloads_refuse_every_predecessor_identity ... ok
test authentication_selected_decoders_preserve_original_duplicate_fields ... ok
test authentication_new_nested_dtos_refuse_original_duplicate_fields ... ok
test authentication_connection_frame_and_input_budgets_are_version_specific ... ok
test operation_v2_complete_request_and_response_vectors_match_schema ... ok
test authentication_operation_v3_vectors_have_independent_reader_and_schema_results ... ok
test authentication_connection_v2_vectors_have_independent_reader_and_schema_results ... ok
test auth_adversary_original_vector_bytes_reject_escaped_duplicates_and_padded_overflow ... ok

test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/rate_adversary.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary-735069fd71edcb71)

running 2 tests
test rate_adversary_published_schema_matches_source_url_reader ... ok
test rate_final_uri_composition_and_downgrade_validate_complete_frames ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/server-e255c22140985ca6)

running 114 tests
test egress::tests::ambiguous_retry_after_is_not_flattened_into_advice ... ok
test egress::tests::exact_origin_cannot_be_widened_by_path_host_or_userinfo ... ok
test egress::tests::egress_requires_a_nonempty_ascii_connection_or_session_reference ... ok
test egress::tests::ipv4_mapped_ipv6_cannot_bypass_address_classification ... ok
test egress::tests::cached_client_cannot_bypass_current_address_policy ... ok
test egress::tests::operator_network_may_admit_private_but_not_process_local_addresses ... ok
test egress::tests::malformed_retry_after_does_not_hide_the_definite_provider_response ... ok
test egress::tests::public_dns_refuses_private_local_reserved_and_mixed_answers ... ok
test egress::tests::retry_after_extraction_keeps_only_one_valid_decimal_and_admitted_headers ... ok
test egress::tests::suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port ... ok
test hosted::enforcement::tests::an_issued_record_round_trips_without_its_reference_in_any_key ... ok
test hosted::enforcement::tests::the_canonical_digest_ignores_member_order_and_nothing_else ... ok
test hosted::git_fetch::tests::rejected_control_identity_is_decided_before_the_request_body_is_polled ... ok
test hosted::principal::tests::lease_seeds_survive_the_verifier_token_rotation_composition ... ok
test hosted::git_fetch::tests::rejected_source_authority_is_decided_before_body_or_broker_exchange ... ok
test hosted::mcp::toolset::authentication_projection_closes_valid_private_reference_and_message_fields ... ok
test egress::tests::reused_connection_keeps_authorization_and_timeout_request_specific ... ok
test hosted::git_fetch::tests::ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body ... ok
test hosted::tests::admin_routes::operator_group_without_the_exact_scope_cannot_write ... ok
test hosted::tests::admin_routes::auth_metadata_is_public_and_selects_exact_authority ... ok
test egress::tests::rate_stage2_definite_http_429_survives_oversized_or_broken_error_bodies ... ok
test hosted::tests::admin_routes::operator_can_write_and_status_never_returns_the_secret ... ok
test hosted::git_fetch::tests::control_and_internal_routes_are_separate_and_non_cacheable ... ok
test hosted::tests::contract_validation::hosted_route_refuses_a_malformed_backend_contract ... ok
test egress::rate_adversary_tests::rate_final_chunked_429_keeps_definite_status_without_body_or_untrusted_advice ... ok
test hosted::tests::a_human_issues_one_exact_input_approval_which_is_spent_once ... ok
test egress::tests::pool_is_bounded_and_separates_current_addresses_authorities_origins_and_policy ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_boundary_selects_response_version_even_for_early_refusals ... ok
test egress::rate_adversary_tests::rate_adversary_header_cardinality_and_unfinished_body_are_separate ... ok
test hosted::tests::contract_validation::rate_stage2_hosted_http_projects_refusals_once_and_rejects_invalid_frames_before_backend ... ok
test hosted::tests::contract_validation::rate_final_hosted_describe_versions_reject_bad_advice_before_loss ... ok
test hosted::tests::contract_validation::rate_stage2_invalid_http_correlation_is_a_bounded_versioned_client_refusal ... ok
test hosted::tests::contract_validation::rate_adversary_hosted_grant_and_approval_refusals_keep_requested_version ... ok
test hosted::tests::enforcement::a_granted_effect_without_approval_demand_dispatches_on_the_grant_alone ... ok
test hosted::tests::enforcement::a_granted_mutation_demanding_approval_refuses_without_one ... ok
test hosted::tests::enforcement::a_granted_mutation_with_a_demanded_approval_dispatches_with_one ... ok
test hosted::tests::enforcement::a_mutation_with_no_admitting_grant_refuses ... ok
test hosted::tests::enforcement::a_second_presentation_of_the_same_approval_refuses_and_journals_replay ... ok
test hosted::tests::enforcement::an_unbound_grant_store_is_an_outage_for_effects_only ... ok
test hosted::tests::enforcement::every_enforcement_refusal_renders_the_same_bytes ... ok
test hosted::tests::enforcement::the_read_only_path_is_unchanged_for_callers_without_grants ... ok
test hosted::tests::hosted_completion_failures_are_non_cacheable_and_browser_hardened ... ok
test hosted::tests::hosted_completion_streams_fragments_into_a_redacted_bounded_submission ... ok
test hosted::tests::hosted_connection_route_uses_the_same_identity_boundary ... ok
test hosted::tests::hosted_datasource_route_passes_verified_groups_and_exact_tenant ... ok
test hosted::tests::docs::openapi_json_is_served_verbatim_with_a_content_hash_etag ... ok
test hosted::tests::docs::a_request_example_with_an_unknown_field_is_refused ... ok
test hosted::tests::hosted_route_requires_identity_and_exact_tenant_binding ... ok
test hosted::tests::docs::every_documented_refusal_example_names_a_real_error_code ... ok
test hosted::tests::docs::the_document_pins_the_exact_wire_contract_identities_and_audience ... ok
test hosted::tests::docs::auth_openapi_selects_each_supported_identity_explicitly ... ok
test hosted::tests::docs::auth_openapi_remediation_examples_keep_the_operation_unattempted ... ok
test hosted::tests::mcp::an_op_backed_invoke_describes_then_invokes_with_the_fresh_lease ... ok
test hosted::tests::mcp::an_invoke_without_the_invoke_scope_surfaces_not_granted ... ok
test hosted::tests::docs::every_documented_mcp_request_example_is_answered_by_the_live_transport ... ok
test hosted::tests::mcp::approval_demand_and_evidence_pass_through_the_admission_seam ... ok
test hosted::tests::mcp::a_stale_authority_refusal_is_retried_exactly_once_with_a_fresh_lease ... ok
test hosted::tests::mcp::a_busy_namespace_is_listed_whole_without_a_paging_surface ... ok
test hosted::tests::mcp::initialize_echoes_admitted_revisions_and_answers_ping ... ok
test hosted::tests::mcp::the_mcp_route_is_stateless_post_only ... ok
test hosted::tests::docs::every_documented_request_example_is_accepted_by_its_protocol_type ... ok
test hosted::tests::mcp::datasource_backed_tools_route_through_the_datasource_seam ... ok
test hosted::tests::mcp::tools_list_returns_exactly_the_three_meta_tools ... ok
test hosted::tests::docs::the_docs_page_is_served_unauthenticated_as_html ... ok
test hosted::tests::mcp::tool_search_projects_only_the_entries_the_callers_seam_results_support ... ok
test hosted::tests::docs::the_docs_page_links_the_contract_and_renders_its_version ... ok
test hosted::tests::mcp::transport_refusals_carry_the_designed_statuses_and_codes ... ok
test hosted::tests::mcp::rate_stage2_mcp_preserves_refusal_details_without_entering_stale_retry ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_routes_the_chosen_target_through_the_decided_seam ... ok
test hosted::tests::docs::the_docs_page_makes_zero_external_requests ... ok
test hosted::tests::mcp_monitoring::a_monitoring_invoke_refuses_dishonest_targets_before_any_dispatch ... ok
test hosted::tests::monitoring_transport_gate_admits_only_configured_groups_or_operator ... ok
test hosted::tests::pod_log_transport_gate_admits_only_kubernetes_read_groups_or_operator ... ok
test hosted::tests::mcp_monitoring::a_stale_monitoring_invoke_re_resolves_the_same_target_exactly_once ... ok
test hosted::tests::docs::every_docs_page_example_is_the_documents_example_after_json_normalization ... ok
test hosted::tests::docs::the_docs_page_refusal_table_carries_every_documented_code ... ok
test hosted::tests::mcp::tool_describe_projects_the_underlying_description_without_a_lease ... ok
test hosted::tests::production_router_publishes_client_discovery_without_authentication ... ok
test hosted::tests::mcp::rate_adversary_mcp_stale_then_rate_stops_without_losing_large_delay ... ok
test hosted::tests::self_event_scope_is_closed_to_slack_specific_requests ... ok
test hosted::tests::mcp_monitoring::the_acceptance_sequence_invokes_with_a_target_and_integer_epochs ... ok
test hosted::tests::remediation::auth_connection_v2_ordinary_requests_select_exact_identity_and_refuse_duplicates ... ok
test hosted::tests::signal::a_granted_session_signal_dispatches_behind_the_sessions_grant ... ok
test hosted::tests::signal::an_effect_bearing_session_signal_without_an_admitting_grant_refuses ... ok
test hosted::tests::signal::an_unbound_grant_store_is_an_outage_for_session_signals ... ok
test hosted::tests::signal::a_signal_refusal_matches_the_invoke_refusal_bytes ... ok
test hosted::tests::tenant_members_receive_only_read_only_module_invocation ... ok
test hosted::tests::subscription_oauth_start_is_identity_scoped_bounded_and_non_cacheable ... ok
test local::tests::a_broad_state_directory_refuses_without_repair ... ok
test hosted::tests::subscription_credential_stays_in_custody_and_only_an_exact_lease_redeems ... ok
test local::tests::a_second_daemon_cannot_unlink_the_live_daemons_socket ... ok
test hosted::tests::mcp_monitoring::tool_describe_enumerates_the_callers_configured_targets_without_a_lease ... ok
test local::tests::auth_one_shot_v3_refuses_before_backend_work_and_joins_shutdown ... ok
test local::tests::auth_one_shot_v3_serves_a_real_result_and_joins_shutdown ... ok
test local::tests::one_socket_dispatches_the_value_free_connection_and_event_contracts ... ok
test local::tests::owner_socket_serves_one_strict_bounded_operation_frame ... ok
test local::tests::rate_stage2_actual_socket_serves_both_versions_without_resending ... ok
test hosted::tests::mcp_monitoring::tool_search_lists_the_monitoring_tools_for_a_monitoring_read_principal ... ok
test hosted::tests::mcp_monitoring::monitoring_tool_schemas_state_the_documents_contract ... ok
test hosted::tests::mcp::a_pathological_namespace_is_cut_with_an_explicit_truncation_marker ... ok
test hosted::tests::docs::auth_openapi_schema_projection_preserves_protocol_vector_results ... ok
test hosted::tests::remediation::auth_v3_does_not_publish_a_structurally_valid_private_backend_reference ... ok
test hosted::tests::remediation::auth_v3_requires_a_real_grant_and_keeps_unknown_targets_opaque ... ok
test hosted::tests::remediation::auth_connection_v2_hosted_start_has_real_grants_and_no_production_acquisition ... ok
test catalog_projection::tests::a_deployment_publishes_only_the_setup_flows_it_can_complete ... ok
test catalog_projection::tests::search_is_whole_catalog_and_describe_is_descriptive_only ... ok
test catalog_projection::tests::platform_provider_satisfies_the_catalog_wire_contract ... ok
test local::tests::auth_one_shot_v3_need_precedes_dispatch_and_joins_shutdown ... ok
test hosted::tests::remediation::auth_v3_need_precedes_real_approval_redemption_and_dispatch ... ok
test catalog_projection::tests::every_shipped_provider_description_satisfies_the_catalog_wire_contract ... ok
test hosted::tests::hosted_liveness_and_identity_backed_readiness_are_distinct ... ok
test hosted::tests::hosted_liveness_stays_local_when_a_backend_dependency_is_unready ... ok
test hosted::tests::docs::every_documented_route_exists_in_the_real_router ... ok
test hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation ... FAILED

failures:

---- hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation stdout ----
actual hosted selected response status/schema observations: [("V0Alpha1", 409, false), ("V0Alpha2", 409, false), ("V0Alpha3", 409, true)]

thread 'hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation' (2505522) panicked at crates/server/src/hosted/tests/remediation.rs:614:5:
served OpenAPI must admit the actual selected response at its actual HTTP status
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    hosted::tests::remediation::auth_adversary_hosted_selected_refusals_match_actual_status_schema_and_revocation

test result: FAILED. 113 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s

error: test failed, to rerun pass `-p server --lib`
     Running tests/rate_adversary_local.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/rate_adversary_local-e255c2a977e11baf)

running 2 tests
test rate_final_local_describe_validates_before_version_loss ... ok
test rate_adversary_local_versions_validate_before_single_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/service-3731ef93ea80f9c6)

running 68 tests
test audio::tests::relative_paths_absent_bounds_and_bad_digests_never_reach_the_device ... ok
test browser::tests::a_route_for_another_connection_is_refused ... ok
test audio::tests::status_never_admits_an_utterance ... ok
test audio::tests::a_route_for_another_connection_is_refused ... ok
test audio::tests::a_speech_plan_and_its_deployment_route_admit_together ... ok
test audio::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test browser::tests::a_plan_for_another_driver_or_operation_is_refused ... ok
test browser::tests::a_unary_lifecycle_is_refused_because_a_browser_spans_calls ... ok
test audio::tests::the_caller_text_is_bounded_by_the_deployment_and_not_only_by_the_catalog ... ok
test audio::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test browser::tests::the_operators_own_browser_profile_is_never_an_admitted_route ... ok
test browser::tests::admitted_evidence_never_prints_as_a_serializable_route_secret ... ok
test browser::tests::relative_paths_nested_artifacts_and_absent_bounds_never_reach_the_browser ... ok
test browser::tests::every_admitted_browser_operation_and_its_route_admit_together ... ok
test browser::tests::a_non_web_address_is_refused_before_any_browser_is_touched ... ok
test browser::tests::only_open_may_omit_an_address_and_only_open_or_goto_may_carry_one ... ok
test dispatch::tests::composition_order_is_policy_redaction_audit_driver_audit ... ok
test authority::tests::proof_is_bound_to_exact_upgrade_uri ... ok
test authority::tests::debug_never_prints_authority_or_proof ... ok
test egress::tests::retry_delay_is_one_unsigned_decimal_with_only_http_whitespace ... ok
test connect_session::tests::a_completion_claim_blocks_expiry_shutdown_and_duplicate_completion ... ok
test connect_session::tests::preparing_and_uncertain_abort_cannot_publish_a_terminal_result ... ok
test connect_session::tests::the_claim_rechecks_the_original_target_and_inclusive_deadline_once ... ok
test connect_session::tests::clock_failure_refuses_authorization_but_does_not_prevent_confirmed_abort ... ok
test connect_session::tests::authority_and_claim_use_one_receiver_owned_instant ... ok
test connect_session::tests::terminal_transition_is_one_way_and_value_free ... ok
test connect_session::tests::capacity_counts_only_pending_sessions ... ok
test connect_session::tests::guarded_sessions_keep_capacity_until_a_confirmed_outcome ... ok
test runtime::tests::hosted_principals_do_not_fabricate_agent_revisions ... ok
test runtime::remediation_contract_tests::personal_remediation_factory_defaults_to_refusal_without_backend_work ... ok
test connect_session::tests::shutdown_fails_only_pending_sessions_and_returns_their_endpoints ... ok
test runtime::tests::local_principals_require_a_real_agent_revision ... ok
test runtime::tests::delegated_execution_must_match_the_authenticated_actor ... ok
test runtime::remediation_contract_tests::remediation_defaults_do_not_run_existing_backend_work ... ok
test runtime::remediation_contract_tests::remediation_default_bound_methods_refuse_without_authority_or_operation_work ... ok
test runtime::tests::hosted_completion_submission_never_reallocates_received_secret_material ... ok
test admin::tests::status_reports_presence_without_reading_the_value ... ok
test runtime::tests::the_stable_authority_seed_distinguishes_absent_and_literal_default_realms ... ok
test runtime::tests::the_stable_authority_seed_ignores_request_scoped_provenance ... ok
test admin::tests::write_requires_explicit_replacement_and_audits_no_secret ... ok
test authority::tests::session_lease_cannot_be_extended_by_authority_clock_skew ... ok
test authority::tests::audience_expiry_and_revocation_fail_before_redemption ... ok
test runtime::tests::the_stable_authority_seed_survives_token_scoped_snapshot_fields ... ok
test sip::tests::a_default_naming_an_absent_trunk_is_refused_when_the_table_is_built ... ok
test sip::tests::a_dial_with_no_alias_and_no_default_is_refused_rather_than_guessed ... ok
test sip::tests::a_partial_byte_prefix_masks_only_the_bits_it_declares ... ok
test sip::tests::a_dial_with_only_a_number_takes_the_declared_default_trunk ... ok
test sip::tests::a_prefix_aperture_admits_its_network_and_refuses_outside_it ... ok
test sip::tests::a_prefix_that_is_not_on_its_own_boundary_is_refused_rather_than_rounded ... ok
test sip::tests::a_named_trunk_is_resolved_before_admission_so_the_answer_is_aperture_checked ... ok
test sip::tests::a_trunk_that_does_not_admit_numbers_refuses_one ... ok
test authority::tests::serving_endpoint_redeems_once ... ok
test sip::tests::an_aperture_never_matches_across_address_families ... ok
test sip::tests::an_exact_aperture_still_admits_exactly_one_address ... ok
test sip::tests::missing_organization_in_grant_evidence_refuses_before_the_driver ... ok
test sip::tests::sip_dial_resolves_only_an_exact_connection_owned_alias ... ok
test sip::tests::the_dialled_host_comes_from_the_trunk_and_never_from_the_caller ... ok
test sip::tests::stable_network_and_aperture_widening_refuse_before_the_driver ... ok
test sip::tests::zero_or_excessive_dial_deadlines_refuse_before_the_driver ... ok
test sip::tests::exact_loopback_route_produces_non_serializable_driver_evidence ... ok
test voice::tests::invalid_endpoint_and_authority_windows_refuse_before_io ... ok
test voice::tests::one_proof_joins_exact_sip_and_application_routes ... ok
test runtime::remediation_contract_tests::remediation_internal_diagnostics_do_not_trust_printable_reference_fields ... ok
test planning::tests::missing_driver_refuses_before_dispatch ... ok
test planning::tests::bidirectional_connection_still_requires_the_operation_admission ... ok
test planning::tests::sip_plan_has_no_http_fields ... ok
test planning::tests::provider_only_connection_refuses_a_caller_initiated_operation ... ok
test planning::tests::mediated_http_plan_has_no_direct_origin_and_requires_the_closed_adapter ... ok

test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

   Doc-tests connectors_client

running 3 tests
test crates/connectors-client/src/model.rs - model::PendingPersonalOAuth (line 314) - compile fail ... ok
test crates/connectors-client/src/model.rs - model::PersonalOAuthInstructions (line 340) - compile fail ... ok
test crates/connectors-client/src/model.rs - model::PendingRemediation (line 375) - compile fail ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

   Doc-tests protocol

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests server

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests service

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 2 targets failed:
    `-p connectors-client --lib`
    `-p server --lib`
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.result.json
````json
{
  "label": "auth-adversary1-root-full",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 25608683520,
    "tmpfs_free_bytes": 13911085056,
    "mem_available_bytes": 36074434560
  },
  "maximum_target_bytes": 10722144256,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:31:34.936175+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.command.json
````json
{
  "label": "auth-adversary1-root-clippy",
  "argv": [
    "cargo",
    "clippy",
    "--locked",
    "--offline",
    "-p",
    "protocol",
    "-p",
    "service",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:32:25.377883+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.log
````text
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Checking protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
    Finished `dev` profile [unoptimized] target(s) in 4.28s
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.result.json
````json
{
  "label": "auth-adversary1-root-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 26001108992,
    "tmpfs_free_bytes": 13921710080,
    "mem_available_bytes": 36391387136
  },
  "maximum_target_bytes": 10709729280,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:32:31.328097+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.command.json
````json
{
  "label": "auth-adversary1-root-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:32:58.369914+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.log
````text

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.result.json
````json
{
  "label": "auth-adversary1-root-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 25948659712,
    "tmpfs_free_bytes": 13916119040,
    "mem_available_bytes": 36579971072
  },
  "maximum_target_bytes": 10709729280,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:33:00.114565+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.command.json
````json
{
  "label": "auth-adversary1-runtime-full",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:33:07.534738+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
   Compiling integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
    Finished `test` profile [unoptimized] target(s) in 7.14s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connect_session_transport-946dac6669d3f30d)

running 24 tests
test oauth::tests::callback_query_is_strict_bounded_and_distinguishes_unknown_state ... ok
test oauth::tests::fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths ... ok
test oauth::tests::device_deadline_is_capped_by_private_authorization_expiry ... ok
test oauth::tests::fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener ... ok
test oauth::tests::expiry_caps_a_stalled_read_and_future_drop_closes_the_port ... ok
test oauth::tests::already_expired_instruction_request_refuses_before_reading_or_writing ... ok
test oauth::tests::liveness_observer_drop_and_rebind_cannot_revive_old_receiver ... ok
test oauth::tests::already_accepted_replay_cannot_survive_the_callback_claim ... ok
test oauth::tests::liveness_observer_uses_original_receiver_deadline_without_polling_receive ... ok
test oauth::tests::request_parser_requires_exact_host_get_origin_form_and_bounded_headers ... ok
test oauth::tests::device_bridge_shows_only_human_instructions_and_has_no_callback ... ok
test oauth::tests::liveness_observer_is_retired_after_matching_callback ... ok
test oauth::tests::callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url ... ok
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test tests::unsafe_directory_refuses ... ok
test oauth::tests::oauth_and_raw_completion_endpoints_remain_independent ... ok
test oauth::tests::callback_origin_is_optional_but_exact_if_present_and_denial_retires_session ... ok
test oauth::tests::local_pkce_binding_inconsistency_is_a_safe_terminal_refusal ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test oauth::tests::request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok
test oauth::tests::protected_instructions_refuse_capability_and_origin_without_spending_state ... ok
test oauth::tests::accepted_connection_budget_retires_session_without_starting_a_second_flow ... ok
test oauth::tests::oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_config-8368461ac53c09bc)

running 24 tests
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_explicit_development_public_pkce ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
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

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_runtime-6f7ffe3605286651)

running 36 tests
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test registry::remediation_tests::remediation_registry_ambiguity_and_absence_do_not_probe_any_owner ... ok
test registry::remediation_tests::remediation_registry_uses_one_exact_owner_without_operation_dispatch ... ok
test registry::remediation_tests::auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok

test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_catalog_writes-dfc82ca0a1e557c9)

running 8 tests
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s

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
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test auth_one_shot_v3_refuses_persistent_control_before_configuration_or_state ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state ... ok
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test auth_one_shot_origin_precomposition_is_typed_and_preserves_legacy_envelopes ... ok
test final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test auth_one_shot_origin_comes_only_from_local_decisions_and_joins_shutdown ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-91fa2d33b9a20e1f)

running 8 tests
test one_shot_create_refuses_before_configuration_custody_and_listener ... ok
test unsupported_production_registration_refuses_before_readiness_or_oauth_store ... ok
test remediation_local_v2_routes_a_created_binding_and_joins_its_endpoint ... ok
test composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness ... ok
test oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target ... ok
test auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact ... ok
test mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation ... ok
test remediation_local_completion_dispatches_only_a_later_explicit_invocation ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s

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
test adapter::tests::readiness_requires_health_and_an_accepted_session_without_reading_a_credential ... ok
test prepared::tests::a_journal_in_the_shared_store_recovers_a_committed_transaction_after_a_restart ... ok
test prepared::tests::candidate_values_stay_in_the_secret_store_and_are_invisible_until_commit ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/identity_http-c9bce0206713f943)

running 3 tests
test adapter::tests::approval_issuance_scope_is_admitted ... ok
test adapter::tests::a_routable_plaintext_identity_origin_is_refused_in_every_build ... ok
test adapter::tests::hosted_verifier_requires_https_origin_and_closed_access_token_shape ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_catalog-c2b8b491b2143a2e)

running 101 tests
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test custody::tests::distinct_bindings_cannot_alias_the_same_reserved_credential_addresses ... ok
test custody::tests::a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit ... ok
test custody::tests::a_claimed_decision_write_can_finish_after_the_authorization_deadline ... ok
test custody::tests::a_reclaimed_publication_retains_its_original_authorization_timing ... ok
test custody::tests::a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks ... ok
test custody::tests::a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked ... ok
test custody::tests::a_full_decision_precedes_secret_commit_and_coherent_publication ... ok
test custody::tests::a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence ... ok
test custody::tests::proposal_bounds_digest_and_journal_serialization_keep_private_values_out ... ok
test custody::refresh_tests::binding_gate_preserves_the_new_generation_for_the_waiting_operation ... ok
test custody::refresh_tests::refresh_of_expired_access_uses_one_timely_claim_without_a_session ... ok
test custody::refresh_tests::a_stale_refresh_handle_cannot_prepare_after_another_valid_publication ... ok
test custody::refresh_tests::waiting_for_prepare_does_not_restart_the_refresh_window ... ok
test custody::refresh_tests::a_timely_refresh_claim_survives_a_held_full_decision_write ... ok
test custody::tests::generation_exhaustion_refuses_before_io_and_resolves_its_private_guard ... ok
test custody::tests::a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization ... ok
test custody::tests::mismatched_proposal_or_stale_prior_generation_never_prepares ... ok
test custody::tests::dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership ... ok
test custody::tests::one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation ... ok
test custody::refresh_tests::invalid_refresh_decision_timing_cannot_publish_on_reopen ... ok
test custody::tests::replacement_preserves_identity_and_publishes_only_same_generation_evidence ... ok
test custody::refresh_tests::cancelled_refresh_store_io_holds_the_binding_gate_until_recovery ... ok
test custody::tests::expiry_or_revocation_before_the_claim_aborts_without_a_decision ... ok
test custody::refresh_tests::uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session ... ok
test custody::tests::unknown_journal_or_secret_state_stays_unavailable_until_reconciled ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test custody::refresh_tests::refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock ... ok
test oauth::tests::admitted_response_reference_is_the_actual_configured_backend_identity ... ok
test oauth::tests::pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen ... ok
test oauth::tests::actual_authority_revocation_before_completion_claim_aborts_prepared_credentials ... ok
test oauth::tests::actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials ... ok
test oauth::tests::actual_timely_claim_survives_full_decision_write_after_session_deadline ... ok
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok
test oauth::tests::actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen ... ok
test oauth::tests::actual_refresh_request_is_cancelled_at_its_original_egress_budget ... ok
test oauth::tests::actual_unknown_decision_without_durable_decision_aborts_on_reopen ... ok
test oauth::tests::actual_secret_commit_before_metadata_publication_recovers_on_reopen ... ok
test oauth::tests::actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen ... ok
test oauth::tests::actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen ... ok
test oauth::tests::invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress ... ok
test oauth::tests::remediation::remediation_created_metadata_is_exact_and_does_not_publish_callable_discovery ... ok
test oauth::tests::remediation::remediation_personal_factory_binds_actual_policy_and_rechecks_current_authority ... ok
test oauth::tests::remediation::remediation_unknown_and_wrong_owner_refuse_without_credential_or_provider_work ... ok
test oauth::tests::refresh_does_not_reset_its_original_window_after_token_egress ... ok
test oauth::tests::remediation::remediation_bound_publication_rechecks_its_receiver_authority_before_custody_commit ... ok
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
test oauth::tests::remediation::remediation_bound_session_publishes_exact_target_and_acknowledges_once_without_dispatch ... ok
test tests::rate_adversary_catalog_checks_binding_and_lease_before_rate_disclosure ... ok
test oauth::tests::refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen ... ok
test tests::the_aperture_is_derived_from_the_same_declaration_the_request_is ... ok
test tests::the_ceiling_reads_declared_facts_rather_than_an_operation_list ... ok
test tests::the_default_ceiling_admits_reads_and_refuses_writes ... ok
test tests::the_instance_derivation_puts_the_provider_in_the_namespace ... ok
test tests::the_limit_drops_operations_rather_than_the_identities_that_serve_one ... ok
test tests::two_named_instances_of_one_provider_get_different_addresses ... ok
test tests::search_and_describe_require_approval_for_every_admitted_write ... ok
test tests::rate_stage2_catalog_refusal_keeps_only_trusted_optional_delay ... ok
test oauth::tests::unique_binding_and_owner_refusals_happen_before_listener_session_or_egress ... ok
test oauth::tests::remediation::remediation_expired_acknowledgement_refuses_without_rolling_back_published_credentials ... ok
test oauth::tests::remediation::remediation_fresh_and_safely_refreshable_credentials_are_ready_without_refresh ... ok
test oauth::tests::adversary::oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation ... ok
test oauth::tests::requested_scope_ceiling_never_substitutes_for_observed_operation_scopes ... ok
test oauth::tests::wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress ... ok
test oauth::tests::rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen ... ok
test oauth::tests::unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen ... ok
test oauth::tests::successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen ... ok
test custody::tests::every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen ... ok
test custody::tests::journal_capacity_is_reserved_for_publication_before_secret_prepare ... ok
test oauth::tests::remediation::auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority ... ok
test custody::tests::uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back ... ok
test custody::tests::malformed_or_inconsistent_decisions_never_recover_a_publication ... ok
test oauth::tests::device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry ... ok
test custody::refresh_tests::refresh_claim_rechecks_time_authority_generation_and_captured_evidence ... ok
test oauth::tests::adversary::oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.74s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_gitlab-0d98d4dd9b3844e3)

running 42 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::git_fetch::v2::tests::capabilities_accept_optional_upload_pack_preamble_across_chunk_boundaries ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::git_fetch::v2::tests::capabilities_refuse_malformed_or_repeated_service_preambles ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_jira-83ed1a5f25510a4f)

running 12 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_kubernetes-16470799659117e3)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s

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
test backend::tests::connect_session_uses_shared_transport_and_publishes_only_after_secret_custody ... ok
test backend::tests::hosted_federation_is_digest_bound_group_scoped_and_has_no_connect_session ... ok
test backend::tests::concurrent_completions_publish_exactly_one_parent_connection ... ok
test backend::tests::credential_custody_failure_is_distinguished_from_upstream_failures ... ok
test backend::tests::oversized_upstream_body_refuses_as_result_bound_not_unreachable ... ok
test backend::tests::mediated_alertmanager_dispatch_resolves_the_v2_api_path ... ok
test backend::tests::dashboards_list_dispatches_the_documents_required_only_input_over_http ... ok
test backend::tests::prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route ... ok
test backend::tests::discovery_materialization_and_query_stay_on_the_grafana_route ... ok
test backend::tests::dashboards_list_pages_upstream_with_a_bounded_limit_and_fetch_budget ... ok
test backend::tests::refused_dispatches_distinguish_upstream_status_class_from_transport ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.39s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_platform-2feb5147d7d1395a)

running 23 tests
test tests::ontology_nullable_fields_are_still_strict_after_catalog_lowering ... ok
test tests::browser_catalog_symbol_is_translated_into_the_closed_driver_input ... ok
test tests::a_mutating_post_dispatch_failure_is_not_declared_retriable ... ok
test tests::every_declared_write_requires_external_approval ... ok
test work_events::tests::cursors_events_and_replay_are_partitioned_by_tenant ... ok
test tests::every_projected_operation_has_a_response_schema ... ok
test tests::an_unknown_workspace_binding_is_named_rather_than_reported_as_stale ... ok
test tests::a_workspace_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test tests::work_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::planner_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::workspace_datasource_projects_only_the_logical_read_model ... ok
test tests::total_http_deadline_bounds_a_stalled_private_service ... ok
test tests::hosted_tenant_member_defaults_are_an_explicit_module_ceiling ... ok
test tests::module_global_ids_resolve_for_declarative_ui_requirements ... ok
test tests::search_projects_only_configured_capabilities ... ok
test tests::search_names_each_operation_once_and_never_by_its_second_name ... ok
test tests::every_name_of_an_operation_describes_one_operation ... ok
test tests::a_write_passes_no_local_approval_gate ... ok
test tests::planner_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::invalid_post_dispatch_output_is_audited_as_indeterminate ... ok
test tests::work_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::ontology_invocation_carries_request_bound_signed_authority ... ok
test tests::local_work_invocation_is_constrained_to_the_configured_unix_socket ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_sip-b2eb446efa1a6eea)

running 14 tests
test raw::tests::a_chosen_device_is_the_one_bound ... ok
test raw::tests::a_host_with_no_sound_stack_still_composes_a_launcher ... ok
test raw::tests::the_receipt_claims_no_application_channel ... ok
test raw::tests::readiness_contacts_nothing ... ok
test runtime::tests::stored_credential_readiness_is_value_free_and_reports_store_unavailability ... ok
test runtime::tests::missing_sip_credentials_fail_closed ... ok
test runtime::tests::authority_key_must_be_an_owner_only_real_file ... ok
test runtime::tests::stored_credentials_are_tenant_scoped_ordered_and_redacted ... ok
test backend::tests::a_binding_that_cannot_signal_refuses_rather_than_dropping_the_keypress ... ok
test backend::tests::readiness_delegates_to_the_mandatory_launcher_probe_without_launching ... ok
test backend::tests::an_unknown_session_is_not_found_and_a_refused_signal_is_reported ... ok
test backend::tests::a_signal_reaches_the_live_session_and_leaves_it_established ... ok
test backend::tests::catalog_projection_invocation_session_control_and_audit_share_one_path ... ok
test backend::tests::stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_slack-c1ea08c672ba4a9c)

running 30 tests
test backend::tests::a_local_companion_submission_is_one_bot_token_and_nothing_else ... ok
test backend::tests::a_declared_instance_name_fixes_its_identity_for_good ... ok
test backend::tests::hosted_companion_completion_requires_distinct_app_and_bot_credentials ... ok
test backend::tests::datasource_projection_excludes_unreviewed_slack_profile_fields ... ok
test backend::tests::hosted_completion_errors_separate_conflicts_from_store_outages ... ok
test backend::tests::hosted_setup_page_requires_capability_and_distinguishes_safe_failures ... ok
test backend::tests::message_loop_guards_and_closed_event_grants_are_applied_before_storage ... ok
test backend::tests::only_the_inner_admitted_event_is_projected ... ok
test backend::tests::slack_auth_test_refuses_only_explicit_invalid_credentials ... ok
test backend::tests::slack_auth_test_provider_and_transport_failures_are_unavailable ... ok
test backend::tests::socket_ticket_destination_is_closed_to_slack_tls_hosts ... ok
test backend::tests::a_credential_file_other_accounts_can_read_is_refused_rather_than_used ... ok
test tests::organization_credentials_do_not_claim_personal_oauth_is_configured ... ok
test backend::tests::event_is_durable_and_deduplicated_before_pull_and_replay ... ok
test backend::tests::a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted ... ok
test backend::tests::describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource ... ok
test backend::tests::ephemeral_open_never_starts_a_socket_mode_supervisor ... ok
test backend::tests::hosted_sessions_expire_and_release_pending_capacity_without_submission ... ok
test backend::tests::invalid_hosted_capability_cannot_consume_a_connect_session ... ok
test backend::tests::operation_audit_is_durable_bounded_and_value_free ... ok
test backend::tests::datasource_description_lease_ignores_request_scoped_provenance ... ok
test backend::tests::organization_bot_is_admitted_for_reads_without_an_event_channel ... ok
test backend::tests::slack_readiness_is_value_free_and_tracks_the_secret_store ... ok
test backend::tests::standalone_adapter_claims_only_its_connection_and_event_families ... ok
test backend::tests::read_ownership_matches_describe_ownership_for_every_slack_datasource ... ok
test backend::tests::stale_grant_metadata_cannot_reenter_any_connection_or_event_surface ... ok
test backend::tests::one_use_completion_publishes_only_value_free_connection_state ... ok
test backend::tests::rate_adversary_slack_zero_delay_settles_each_explicit_write_refusal ... ok
test backend::tests::rate_final_slack_definite_refusal_survives_terminal_audit_failure ... ok
test backend::tests::rate_stage2_slack_definite_write_refusal_is_not_an_uncertain_outcome ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/monitoring_model-ba8bcc738bad9da9)

running 4 tests
test tests::loki_timestamps_stay_strings_and_the_refusal_names_the_encoding ... ok
test tests::prometheus_timestamps_accept_integer_epoch_seconds_beside_strings ... ok
test tests::the_validator_still_refuses_outside_the_documents_contract ... ok
test tests::the_validator_admits_the_documents_required_only_input ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/state_sqlite-b1d5ce5e44bf1d75)

running 11 tests
test tests::full_open_refuses_unusable_paths ... ok
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::the_in_memory_backend_conforms ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::the_file_backend_conforms ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::existing_openers_keep_normal_synchronization ... ok
test tests::full_open_configures_wal_and_full_synchronization_on_every_open ... ok
test tests::full_commits_are_visible_before_close_and_survive_reopening ... ok
test tests::the_full_file_backend_preserves_state_and_grant_conformance ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s

     Running tests/approval_gate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/approval_gate-9774db699f1b2c77)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok
test a_replay_survives_reopening_the_database ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

   Doc-tests connect_session_transport

running 2 tests
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::BoundOAuthEndpoint (line 100) - compile fail ... ok
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::OAuthCallback (line 55) - compile fail ... ok

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

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.result.json
````json
{
  "label": "auth-adversary1-runtime-full",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 24527970304,
    "tmpfs_free_bytes": 13901619200,
    "mem_available_bytes": 34333876224
  },
  "maximum_target_bytes": 10713792512,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:33:46.392029+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.command.json
````json
{
  "label": "auth-adversary1-runtime-clippy",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:34:04.418021+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Checking integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
    Finished `dev` profile [unoptimized] target(s) in 3.59s
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.result.json
````json
{
  "label": "auth-adversary1-runtime-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 24149090304,
    "tmpfs_free_bytes": 13921845248,
    "mem_available_bytes": 35278950400
  },
  "maximum_target_bytes": 10713792512,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:34:09.322973+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.command.json
````json
{
  "label": "auth-adversary1-runtime-no-default",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:35:33.891874+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `test` profile [unoptimized] target(s) in 3.41s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connect_session_transport-946dac6669d3f30d)

running 24 tests
test oauth::tests::already_expired_instruction_request_refuses_before_reading_or_writing ... ok
test oauth::tests::callback_query_is_strict_bounded_and_distinguishes_unknown_state ... ok
test oauth::tests::fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths ... ok
test oauth::tests::fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener ... ok
test oauth::tests::device_bridge_shows_only_human_instructions_and_has_no_callback ... ok
test oauth::tests::request_parser_requires_exact_host_get_origin_form_and_bounded_headers ... ok
test oauth::tests::liveness_observer_drop_and_rebind_cannot_revive_old_receiver ... ok
test tests::browser_capability_comparison_rejects_prefixes_and_differences ... ok
test oauth::tests::request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline ... ok
test tests::unsafe_directory_refuses ... ok
test oauth::tests::expiry_caps_a_stalled_read_and_future_drop_closes_the_port ... ok
test oauth::tests::already_accepted_replay_cannot_survive_the_callback_claim ... ok
test oauth::tests::device_deadline_is_capped_by_private_authorization_expiry ... ok
test oauth::tests::liveness_observer_uses_original_receiver_deadline_without_polling_receive ... ok
test oauth::tests::callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url ... ok
test oauth::tests::liveness_observer_is_retired_after_matching_callback ... ok
test tests::endpoint_is_owner_only_one_use_and_removed_after_submission ... ok
test oauth::tests::oauth_and_raw_completion_endpoints_remain_independent ... ok
test oauth::tests::local_pkce_binding_inconsistency_is_a_safe_terminal_refusal ... ok
test tests::browser_page_submits_directly_to_the_one_use_endpoint ... ok
test oauth::tests::callback_origin_is_optional_but_exact_if_present_and_denial_retires_session ... ok
test oauth::tests::protected_instructions_refuse_capability_and_origin_without_spending_state ... ok
test oauth::tests::accepted_connection_budget_retires_session_without_starting_a_second_flow ... ok
test oauth::tests::oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_config-8368461ac53c09bc)

running 24 tests
test hosted::tests::claude_code_custody_is_explicit_and_requires_the_hosted_secret_store ... ok
test hosted::tests::an_existing_hosted_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_configuration_uses_same_handle_and_refuses_mutable_or_symlinked_files ... ok
test personal::tests::a_trunk_address_may_be_a_literal_or_a_name_and_nothing_else ... ok
test hosted::tests::hosted_integrations_are_explicit_and_fail_closed ... ok
test hosted::tests::hosted_grafana_requires_vault_exact_groups_and_digest_bound_targets ... ok
test hosted::tests::kubernetes_namespace_groups_are_exact_sorted_and_restart_is_a_read_subset ... ok
test hosted::tests::hosted_vault_requires_a_valid_distinct_sip_digest_pair ... ok
test hosted::tests::hosted_jira_separates_organization_and_user_authority ... ok
test hosted::tests::hosted_vault_is_all_or_nothing ... ok
test personal::tests::an_existing_personal_config_with_the_old_b10x_section_parses_unchanged ... ok
test hosted::tests::hosted_deployment_accepts_planner_integration ... ok
test hosted::tests::hosted_jira_service_api_token_excludes_a_service_oauth_client_id ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect ... ok
test hosted::tests::hosted_gitlab_requires_vault_and_a_same_origin_callback ... ok
test personal_oauth_tests::personal_oauth_configuration_admits_explicit_development_public_pkce ... ok
test personal::tests::grafana_configuration_names_origin_and_independent_target_grants_only ... ok
test personal::tests::kubernetes_configuration_is_policy_only ... ok
test personal::tests::the_named_default_trunk_example_parses_and_dials_by_number ... ok
test personal::tests::slack_only_configuration_contains_policy_but_no_secret_source ... ok
test personal::tests::documented_development_configuration_stays_strict_and_valid ... ok
test personal::tests::deployment_configuration_cannot_be_symlinked_or_group_writable ... ok
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

running 36 tests
test claims::tests::a_journal_this_build_cannot_parse_refuses_to_open ... ok
test composition::tests::an_empty_variable_is_no_store_at_all ... ok
test claims::tests::only_an_event_reference_is_claimable ... ok
test claims::tests::a_claim_survives_a_daemon_restart ... ok
test composition::tests::an_unopenable_sqlite_path_is_named_in_the_refusal ... ok
test composition::tests::naming_no_store_is_refused_rather_than_a_database_file_appearing_somewhere ... ok
test composition::tests::git_fetch_environment_override_is_atomic_and_secret_free ... ok
test composition::tests::the_refusal_names_both_stores_a_deployment_may_choose ... ok
test composition::tests::git_fetch_environment_override_refuses_an_invalid_listener ... ok
test composition::tests::working_tree_state_roots_are_refused ... ok
test composition::tests::naming_both_stores_is_refused_rather_than_one_of_them_quietly_winning ... ok
test registry::remediation_tests::auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors ... ok
test registry::remediation_tests::remediation_registry_ambiguity_and_absence_do_not_probe_any_owner ... ok
test registry::remediation_tests::remediation_registry_uses_one_exact_owner_without_operation_dispatch ... ok
test registry::tests::a_topical_datasource_query_that_matches_nothing_returns_the_admitted_set ... ok
test registry::tests::ambiguous_exclusive_claims_fail_with_typed_protocol_errors_before_dispatch ... ok
test registry::tests::direct_dispatch_selects_the_unique_claim_without_not_found_probing ... ok
test registry::tests::duplicate_channel_references_fail_search ... ok
test registry::tests::duplicate_connection_references_fail_search ... ok
test registry::tests::rate_stage2_advice_changes_refuse_merging_and_invalidate_the_registry_lease ... ok
test registry::tests::the_registry_lease_ignores_request_scoped_provenance ... ok
test service_bundle::tests::registration_is_inert_until_an_explicit_overlay_is_present ... ok
test service_bundle::tests::malformed_manifests_and_deployments_are_refused ... ok
test service_bundle::tests::catalog_dispatch_and_backend_ownership_mismatches_are_refused ... ok
test registry::tests::readiness_requires_every_configured_backend ... ok
test registry::tests::search_aggregates_compatible_operations_and_deduplicates_the_operation ... ok
test registry::tests::a_companion_reply_is_claimed_exactly_once_locally ... ok
test registry::tests::describe_merges_connections_and_invoke_receives_the_selected_local_lease ... ok
test service_bundle::tests::identity_and_operation_collisions_are_refused ... ok
test service_bundle::tests::bundle_order_and_policy_projection_are_deterministic ... ok
test registry::tests::an_undemanded_reference_is_not_spent_at_the_local_seam ... ok
test claims::tests::parallel_presentations_take_exactly_one_claim ... ok
test tls_listener::tests::established_connection_permit_is_lifetime_bound_and_reads_time_out ... ok
test tls_listener::tests::listener_serves_the_internal_application_over_tls ... ok
test composition::tests::empty_personal_runtime_binds_and_cleans_without_a_credential_store ... ok
test composition::tests::a_hosted_placement_keeps_its_state_in_a_file_when_no_database_is_offered ... ok

test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running tests/local_catalog_writes.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_catalog_writes-d5586ddea625db2a)

running 8 tests
test only_read_only_connections_hide_the_write_and_describe_its_missing_grant ... ok
test describing_a_write_directly_skips_the_first_read_only_connection ... ok
test adversary_a_read_description_cannot_authorize_a_different_write_operation ... ok
test adversary_an_approval_reference_cannot_raise_the_selected_read_only_grant ... ok
test adversary_http_200_application_refusal_survives_the_documented_socket_path ... ok
test stale_description_and_provider_refusal_have_distinct_actionable_results ... ok
test read_only_first_still_discovers_describes_and_posts_through_the_writer ... ok
test writable_first_still_discovers_describes_and_posts_through_the_writer ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.16s

     Running tests/local_gitlab_schedules.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/local_gitlab_schedules-ee9796a1607bd788)

running 6 tests
test configured_gitlab_connection_is_the_same_passive_reference_in_both_discovery_surfaces ... ok
test schedule_words_find_only_operations_admitted_by_the_existing_write_policy ... ok
test adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement ... ok
test read_only_connection_refuses_schedule_mutations_before_custody_or_egress ... ok
test describe_exposes_the_exact_generated_input_and_output_contracts ... ok
test schedule_requests_preserve_complete_json_values_and_optional_update_omission ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s

     Running tests/one_shot_runtime.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_runtime-e21658cd9cf3b194)

running 13 tests
test an_unsafe_socket_refuses_before_opening_the_reply_claim_journal ... ok
test auth_one_shot_v3_refuses_persistent_control_before_configuration_or_state ... ok
test unknown_backend_lifetime_is_refused_and_shutdown_releases_ownership ... ok
test adversary_lifetime_capability_never_admits_unknown_or_ambiguous_owners ... ok
test adversary_every_persistent_request_class_refuses_before_configuration_or_state ... ok
test auth_one_shot_origin_precomposition_is_typed_and_preserves_legacy_envelopes ... ok
test adversary_state_lock_outlives_async_shutdown_on_success_and_refusal ... ok
test final_adversary_invalid_envelopes_refuse_before_reading_config_or_creating_state ... ok
test an_existing_owner_refuses_before_opening_the_reply_claim_journal ... ok
test adversary_socket_publication_after_absence_probe_is_preserved_and_refused ... ok
test final_adversary_malformed_backend_reply_is_reduced_before_shutdown_and_lock_release ... ok
test auth_one_shot_origin_comes_only_from_local_decisions_and_joins_shutdown ... ok
test ephemeral_catalog_uses_the_described_credential_and_rejects_read_only_writes_before_egress ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-7e37c91150c1d49e)

running 8 tests
test one_shot_create_refuses_before_configuration_custody_and_listener ... ok
test unsupported_production_registration_refuses_before_readiness_or_oauth_store ... ok
test composition_opens_only_the_explicit_oauth_custody_and_reports_unsealed_readiness ... ok
test oauth_pass1_daemon_refuses_ambiguous_v1_profile_without_using_label_as_target ... ok
test remediation_local_v2_routes_a_created_binding_and_joins_its_endpoint ... ok
test mixed_raw_and_oauth_bindings_have_exactly_one_invoke_owner_and_keep_aggregation ... ok
test auth_adversary_local_same_profile_bindings_keep_completion_and_ack_exact ... ok
test remediation_local_completion_dispatches_only_a_later_explicit_invocation ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s

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

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/identity_http-c9bce0206713f943)

running 3 tests
test adapter::tests::approval_issuance_scope_is_admitted ... ok
test adapter::tests::a_routable_plaintext_identity_origin_is_refused_in_every_build ... ok
test adapter::tests::hosted_verifier_requires_https_origin_and_closed_access_token_shape ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_catalog-c2b8b491b2143a2e)

running 101 tests
test argocd::tests::read_only_acquisition_carries_no_sync_policy ... ok
test argocd::tests::a_project_or_role_name_that_could_change_the_path_is_refused ... ok
test argocd::tests::an_unreachable_origin_names_the_aperture_as_a_possibility ... ok
test argocd::tests::every_generated_policy_is_one_argo_cd_will_accept ... ok
test argocd::tests::a_missing_project_is_not_reported_as_a_permission_problem ... ok
test argocd::tests::a_zero_lifetime_is_refused_rather_than_meaning_forever ... ok
test argocd::tests::a_rejected_sign_in_says_so_rather_than_reporting_a_missing_project ... ok
test argocd::tests::a_login_without_projects_update_is_told_which_grant_it_lacks ... ok
test argocd::tests::an_existing_role_is_reused_and_its_policies_are_left_alone ... ok
test argocd::tests::the_four_calls_happen_in_order_and_the_token_comes_back ... ok
test argocd::tests::the_project_is_written_back_whole ... ok
test config::tests::an_unsupplied_variable_is_absent_rather_than_empty ... ok
test config::tests::an_endpoint_value_reaches_the_resolver_under_its_declared_name ... ok
test config::tests::a_value_without_operator_approval_says_so_rather_than_claiming_it ... ok
test argocd::tests::the_password_appears_once_and_the_session_token_never_persists ... ok
test custody::tests::distinct_bindings_cannot_alias_the_same_reserved_credential_addresses ... ok
test custody::tests::a_claimed_decision_write_can_finish_after_the_authorization_deadline ... ok
test custody::tests::a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit ... ok
test custody::tests::a_reclaimed_publication_retains_its_original_authorization_timing ... ok
test custody::tests::a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks ... ok
test custody::tests::a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked ... ok
test custody::tests::a_full_decision_precedes_secret_commit_and_coherent_publication ... ok
test custody::tests::a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence ... ok
test custody::tests::proposal_bounds_digest_and_journal_serialization_keep_private_values_out ... ok
test custody::refresh_tests::binding_gate_preserves_the_new_generation_for_the_waiting_operation ... ok
test custody::refresh_tests::a_stale_refresh_handle_cannot_prepare_after_another_valid_publication ... ok
test custody::refresh_tests::refresh_of_expired_access_uses_one_timely_claim_without_a_session ... ok
test custody::refresh_tests::waiting_for_prepare_does_not_restart_the_refresh_window ... ok
test custody::refresh_tests::a_timely_refresh_claim_survives_a_held_full_decision_write ... ok
test custody::tests::generation_exhaustion_refuses_before_io_and_resolves_its_private_guard ... ok
test custody::tests::a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization ... ok
test custody::tests::mismatched_proposal_or_stale_prior_generation_never_prepares ... ok
test custody::tests::dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership ... ok
test custody::tests::one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation ... ok
test custody::refresh_tests::invalid_refresh_decision_timing_cannot_publish_on_reopen ... ok
test custody::tests::replacement_preserves_identity_and_publishes_only_same_generation_evidence ... ok
test custody::refresh_tests::cancelled_refresh_store_io_holds_the_binding_gate_until_recovery ... ok
test custody::tests::expiry_or_revocation_before_the_claim_aborts_without_a_decision ... ok
test custody::refresh_tests::uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session ... ok
test custody::tests::unknown_journal_or_secret_state_stays_unavailable_until_reconciled ... ok
test hosted::tests::completion_does_not_overwrite_a_terminal_session_after_verification_awaits ... ok
test hosted::tests::two_people_receive_isolated_connections_and_credential_addresses ... ok
test custody::refresh_tests::refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock ... ok
test oauth::tests::admitted_response_reference_is_the_actual_configured_backend_identity ... ok
test oauth::tests::pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen ... ok
test oauth::tests::adversary::oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts ... ok
test oauth::tests::actual_authority_revocation_before_completion_claim_aborts_prepared_credentials ... ok
test oauth::tests::actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials ... ok
test oauth::tests::actual_refresh_request_is_cancelled_at_its_original_egress_budget ... ok
test oauth::tests::actual_timely_claim_survives_full_decision_write_after_session_deadline ... ok
test oauth::tests::actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen ... ok
test oauth::tests::actual_unknown_decision_without_durable_decision_aborts_on_reopen ... ok
test oauth::tests::actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen ... ok
test oauth::tests::actual_secret_commit_before_metadata_publication_recovers_on_reopen ... ok
test oauth::tests::actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen ... ok
test oauth::tests::invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress ... ok
test oauth::tests::remediation::remediation_created_metadata_is_exact_and_does_not_publish_callable_discovery ... ok
test oauth::tests::remediation::remediation_unknown_and_wrong_owner_refuse_without_credential_or_provider_work ... ok
test oauth::tests::remediation::remediation_personal_factory_binds_actual_policy_and_rechecks_current_authority ... ok
test oauth::tests::remediation::remediation_bound_publication_rechecks_its_receiver_authority_before_custody_commit ... ok
test oauth::tests::refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen ... ok
test oauth::tests::refresh_does_not_reset_its_original_window_after_token_egress ... ok
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
test oauth::tests::remediation::remediation_expired_status_never_swallows_an_opaque_receiver_rejection ... ok
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
test oauth::tests::unique_binding_and_owner_refusals_happen_before_listener_session_or_egress ... ok
test oauth::tests::remediation::remediation_fresh_and_safely_refreshable_credentials_are_ready_without_refresh ... ok
test oauth::tests::adversary::oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation ... ok
test oauth::tests::requested_scope_ceiling_never_substitutes_for_observed_operation_scopes ... ok
test oauth::tests::wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress ... ok
test oauth::tests::unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen ... ok
test oauth::tests::rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen ... ok
test custody::tests::journal_capacity_is_reserved_for_publication_before_secret_prepare ... ok
test oauth::tests::successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen ... ok
test custody::tests::every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen ... ok
test oauth::tests::remediation::auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority ... ok
test custody::tests::uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back ... ok
test custody::tests::malformed_or_inconsistent_decisions_never_recover_a_publication ... ok
test oauth::tests::device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry ... ok
test custody::refresh_tests::refresh_claim_rechecks_time_authority_generation_and_captured_evidence ... ok
test oauth::tests::adversary::oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline ... ok

test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.79s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_gitlab-0d98d4dd9b3844e3)

running 42 tests
test backend::git_fetch::tests::source_authority_digest_comparison_checks_every_byte ... ok
test backend::git_fetch::tests::upload_pack_request_is_bound_to_exact_commit_and_depth ... ok
test backend::git_fetch::tests::upstream_repository_is_derived_without_forwarding_provider_metadata ... ok
test backend::git_fetch::tests::unsupported_protocol_is_refused_before_provider_egress ... ok
test backend::git_fetch::tests::removed_principal_connection_revokes_a_live_session ... ok
test backend::tests::a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification ... ok
test backend::git_fetch::tests::current_grant_and_provider_default_tip_are_revalidated ... ok
test backend::git_fetch::v2::tests::request_framing_refuses_truncation_ambiguity_and_oversize ... ok
test backend::git_fetch::v2::tests::commands_are_closed_and_prefixes_cannot_expand_upstream_discovery ... ok
test backend::tests::datasource_cursors_are_bound_to_connection_and_project ... ok
test backend::git_fetch::tests::discovery_advertises_only_the_exact_default_branch_snapshot ... ok
test backend::git_fetch::tests::upload_pack_stream_is_bounded_and_spends_the_session ... ok
test backend::tests::datasource_projection_drops_sensitive_and_unknown_fields ... ok
test backend::git_fetch::tests::project_and_branch_authority_reads_overlap_on_creation_and_each_exchange ... ok
test backend::git_fetch::tests::idempotent_replay_keeps_locator_and_rotates_transient_authority ... ok
test backend::tests::pat_shape_rejects_whitespace_and_oversize_values ... ok
test backend::tests::profiles_are_closed_and_self_service ... ok
test backend::tests::recorded_scopes_are_the_retained_subset_sorted_and_deduped ... ok
test backend::tests::repository_file_paths_cannot_traverse_or_change_root ... ok
test backend::tests::the_adapter_carries_every_field_gitlab_sends ... ok
test backend::tests::origins_are_exact_https_only ... ok
test backend::tests::the_refresh_policy_ignores_the_two_fields_that_path_recomputes ... ok
test backend::tests::the_refresh_policy_still_requires_bearer_and_a_refresh_token ... ok
test transport::tests::oauth_forms_encode_secret_delimiters_without_logging_values ... ok
test backend::tests::malformed_state_and_empty_grant_policy_still_fail_closed ... ok
test transport::tests::page_decoding_reads_only_the_selected_cursor_header ... ok
test backend::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::tests::legacy_connection_starts_unusable_and_preserves_pending_custody ... ok
test backend::git_fetch::tests::v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it ... ok
test backend::tests::project_admission_refuses_a_non_advancing_continuation ... ok
test backend::git_fetch::v2::tests::pack_is_streamed_but_final_framing_and_sections_are_enforced ... ok
test backend::git_fetch::tests::v2_drop_budget_revocation_and_foreign_authority_fail_closed ... ok
test backend::tests::project_admission_reaches_a_repository_after_the_first_hundred ... ok
test backend::git_fetch::tests::per_principal_capacity_is_atomic_and_never_evicts_a_live_session ... ok
test backend::git_fetch::tests::global_capacity_refuses_without_evicting_an_inflight_other_principal ... ok
test backend::git_fetch::v2::tests::refs_filter_prefix_collisions_and_verify_full_response_before_emission ... ok
test backend::git_fetch::v2::tests::capabilities_accept_optional_upload_pack_preamble_across_chunk_boundaries ... ok
test backend::git_fetch::v2::tests::capabilities_require_v2_shallow_sha1_and_strip_expanding_features ... ok
test backend::git_fetch::v2::tests::capabilities_refuse_malformed_or_repeated_service_preambles ... ok
test backend::git_fetch::tests::stream_expiry_revokes_a_stalled_upload ... ok
test backend::git_fetch::http_tests::real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes ... ok
test backend::tests::repository_file_paths_are_encoded_as_one_gitlab_segment ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_jira-83ed1a5f25510a4f)

running 12 tests
test backend::auth::tests::oauth_scopes_are_canonical_and_exact ... ok
test backend::auth::tests::the_refresh_policy_allows_a_response_that_rotates_only_the_access_token ... ok
test backend::tests::organization_and_user_profiles_are_distinct ... ok
test backend::tests::delegated_connection_is_withdrawn_when_its_grant_changes ... ok
test backend::operations::tests::operation_inputs_are_closed_and_bounded_before_request_assembly ... ok
test backend::datasource::tests::projection_drops_sensitive_and_unknown_provider_fields ... ok
test backend::auth::tests::the_service_policy_needs_only_the_read_scope_and_no_refresh_token ... ok
test backend::auth::tests::the_exchange_policy_refuses_everything_the_inline_condition_refused ... ok
test backend::operations::tests::all_writes_require_approval ... ok
test backend::operations::tests::issue_keys_bind_an_exact_project ... ok
test backend::datasource::tests::schemas_are_closed_and_projection_is_stable ... ok
test backend::operations::tests::operation_outputs_are_closed_safe_projections ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_kubernetes-16470799659117e3)

running 63 tests
test hosted::database_tests::a_404_from_a_served_group_is_an_error_not_an_empty_inventory ... ok
test hosted::database_tests::a_database_read_on_a_non_admitted_namespace_is_not_granted ... ok
test hosted::database_tests::database_endpoint_bindings_appear_per_admitted_namespace_only ... ok
test hosted::database_tests::search_lists_the_databases_datasource_under_its_terms ... ok
test hosted::tests::a_status_invocation_survives_the_scope_change_between_describe_and_invoke ... ok
test hosted::tests::deployment_projection_requires_observed_available_replicas ... ok
test hosted::tests::an_upstream_log_refusal_surfaces_as_not_granted ... ok
test hosted::database_tests::a_cluster_without_crossplane_discovers_nothing ... ok
test hosted::tests::a_missing_namespace_grant_names_the_namespace_and_the_group_that_carries_it ... ok
test hosted::tests::restart_requires_sre_group_and_exact_resource_authority_without_local_approval ... ok
test hosted::tests::search_lists_pod_logs_for_read_group_principals_and_hides_it_otherwise ... ok
test hosted::tests::an_unknown_binding_is_named_rather_than_reported_as_an_ungranted_one ... ok
test hosted::database_tests::database_datasource_description_names_the_projection_for_read_principals_only ... ok
test local::inventory_tests::adversary_inventory_rejects_null_cursor_as_its_published_schema_requires ... ok
test hosted::database_tests::database_endpoint_list_derives_descriptors_from_both_engines ... ok
test hosted::database_tests::no_secret_value_ever_appears_in_database_endpoint_output ... ok
test local::inventory_tests::adversary_final_overlapping_cursor_replays_dispatch_only_once ... ok
test local::inventory_tests::inventory_discovery_publishes_both_reads_for_every_activated_connection ... ok
test local::inventory_tests::adversary_inventory_walks_empty_pages_and_preserves_all_regular_container_images ... ok
test local::inventory_tests::inventory_namespaces_are_configured_admission_without_cluster_enumeration ... ok
test hosted::tests::a_workload_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test local::inventory_tests::adversary_final_invalid_inputs_do_not_spend_a_live_inventory_cursor ... ok
test hosted::database_tests::database_endpoint_get_returns_one_descriptor_by_name ... ok
test local::inventory_tests::inventory_fresh_cursor_cannot_cross_a_connection_or_namespace_boundary ... ok
test local::inventory_tests::inventory_inputs_require_the_published_object_shape ... ok
test local::inventory_tests::inventory_preserves_owner_and_description_lease_admission ... ok
test local::inventory_tests::inventory_preserves_rbac_refusal ... ok
test local::inventory_tests::adversary_final_empty_fetch_budget_retains_continuation_and_malformed_pages_refuse ... ok
test local::inventory_tests::inventory_optional_output_cursor_is_omitted_or_a_string_never_null ... ok
test local::tests::a_half_attached_cluster_is_not_published ... ok
test local::tests::a_renamed_argocd_release_is_recognized_by_its_identity_label ... ok
test hosted::tests::datasource_projects_only_safe_workload_fields_for_granted_namespaces ... ok
test local::tests::argocd_recognition_takes_the_api_service_and_none_of_its_siblings ... ok
test local::inventory_tests::inventory_shared_reader_preserves_the_existing_compact_datasource_shape ... ok
test local::inventory_tests::inventory_optional_limit_accepts_every_in_range_integer_number_representation ... ok
test local::tests::every_activated_cluster_is_published_in_a_stable_order ... ok
test local::inventory_tests::inventory_reads_each_selected_cluster_and_retains_scaled_to_zero_template_images ... ok
test local::tests::monitoring_service_recognition_is_curated ... ok
test local::tests::providers_that_need_a_credential_are_not_materializable_here ... ok
test local::inventory_tests::inventory_refuses_nonactivated_connections_unadmitted_namespaces_and_bad_inputs_before_io ... ok
test local::inventory_tests::inventory_optional_inputs_distinguish_absence_from_explicit_values ... ok
test local::tests::service_observation_pins_uid_and_one_closed_tcp_port ... ok
test local_workloads::tests::a_binding_ref_that_names_no_configured_namespace_is_the_callers_mistake ... ok
test local::tests::the_argocd_observation_pins_the_api_port_rather_than_the_redirect ... ok
test local_workloads::tests::a_name_that_is_not_a_dns_label_never_reaches_a_request_path ... ok
test local_workloads::tests::a_cluster_listing_becomes_the_same_compact_record_the_deployment_returns ... ok
test local_workloads::tests::query_values_are_encoded_rather_than_interpolated ... ok
test local::tests::insecure_api_server_contexts_are_not_candidates ... ok
test local::inventory_tests::inventory_refuses_upstream_page_overrun_and_wrong_namespace ... ok
test hosted::database_tests::database_endpoint_listing_pages_across_both_engines ... ok
test local::tests::passive_candidates_expose_only_context_label_and_opaque_evidence ... ok
test local_workloads::tests::the_local_placement_publishes_the_deployments_projection_verbatim ... ok
test local::inventory_tests::inventory_pagination_is_bounded_and_cursors_bind_connection_and_namespace ... ok
test hosted::tests::describing_without_a_read_grant_names_the_grant_rather_than_a_missing_datasource ... ok
test hosted::tests::hosted_connection_projection_is_value_free_and_tenant_bound ... ok
test hosted::tests::pod_log_invoke_refuses_a_non_admitted_namespace_and_a_stale_lease ... ok
test hosted::tests::pod_log_description_carries_schemas_and_a_lease_for_read_principals_only ... ok
test hosted::tests::read_only_status_is_description_bound_and_namespace_scoped ... ok
test hosted::tests::pod_log_invoke_passes_the_input_through_and_defaults_tail_lines ... ok
test hosted::tests::pod_log_invoke_enforces_the_input_caps_before_the_reader ... ok
test local::inventory_tests::inventory_refuses_oversized_provider_and_projected_values ... ok
test hosted::tests::an_oversized_log_body_is_front_trimmed_inside_a_validating_envelope ... ok
test hosted::paging_tests::a_busy_namespace_lists_in_full_despite_the_upstream_response_bound ... ok

test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_mcp-dea562dbcc14fb87)

running 2 tests
test tests::changed_live_snapshot_is_refused_before_a_factory_exists ... ok
test tests::frozen_reviewed_tools_cross_connector_custody_and_egress ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_monitoring-65d9e7d55512deb4)

running 16 tests
test backend::tests::refusal_log_record_names_operation_route_and_exact_upstream_status ... ok
test backend::tests::safe_projections_drop_provider_secrets_and_redact_free_text ... ok
test backend::tests::readiness_checks_only_the_mandatory_credential_store ... ok
test backend::tests::standalone_adapter_refuses_unowned_requests_without_fallthrough ... ok
test backend::tests::failed_credential_custody_rolls_back_discovery_and_parent_state ... ok
test backend::tests::hosted_federation_is_digest_bound_group_scoped_and_has_no_connect_session ... ok
test backend::tests::connect_session_uses_shared_transport_and_publishes_only_after_secret_custody ... ok
test backend::tests::credential_custody_failure_is_distinguished_from_upstream_failures ... ok
test backend::tests::oversized_upstream_body_refuses_as_result_bound_not_unreachable ... ok
test backend::tests::discovery_materialization_and_query_stay_on_the_grafana_route ... ok
test backend::tests::mediated_alertmanager_dispatch_resolves_the_v2_api_path ... ok
test backend::tests::concurrent_completions_publish_exactly_one_parent_connection ... ok
test backend::tests::prometheus_range_accepts_integer_epoch_seconds_on_the_mediated_route ... ok
test backend::tests::dashboards_list_dispatches_the_documents_required_only_input_over_http ... ok
test backend::tests::dashboards_list_pages_upstream_with_a_bounded_limit_and_fetch_budget ... ok
test backend::tests::refused_dispatches_distinguish_upstream_status_class_from_transport ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_platform-2feb5147d7d1395a)

running 23 tests
test tests::ontology_nullable_fields_are_still_strict_after_catalog_lowering ... ok
test tests::every_declared_write_requires_external_approval ... ok
test tests::browser_catalog_symbol_is_translated_into_the_closed_driver_input ... ok
test tests::a_mutating_post_dispatch_failure_is_not_declared_retriable ... ok
test work_events::tests::cursors_events_and_replay_are_partitioned_by_tenant ... ok
test tests::every_projected_operation_has_a_response_schema ... ok
test tests::an_unknown_workspace_binding_is_named_rather_than_reported_as_stale ... ok
test tests::a_workspace_read_survives_the_access_token_rotation_between_describe_and_read ... ok
test tests::planner_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::workspace_datasource_projects_only_the_logical_read_model ... ok
test tests::work_owner_events_are_checkpointed_into_connector_sequence_space ... ok
test tests::total_http_deadline_bounds_a_stalled_private_service ... ok
test tests::hosted_tenant_member_defaults_are_an_explicit_module_ceiling ... ok
test tests::every_name_of_an_operation_describes_one_operation ... ok
test tests::search_names_each_operation_once_and_never_by_its_second_name ... ok
test tests::module_global_ids_resolve_for_declarative_ui_requirements ... ok
test tests::search_projects_only_configured_capabilities ... ok
test tests::a_write_passes_no_local_approval_gate ... ok
test tests::work_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok
test tests::local_work_invocation_is_constrained_to_the_configured_unix_socket ... ok
test tests::invalid_post_dispatch_output_is_audited_as_indeterminate ... ok
test tests::ontology_invocation_carries_request_bound_signed_authority ... ok
test tests::planner_invocation_crosses_the_private_http_boundary_with_signed_authority ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.73s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_sip-b2eb446efa1a6eea)

running 14 tests
test raw::tests::a_chosen_device_is_the_one_bound ... ok
test raw::tests::the_receipt_claims_no_application_channel ... ok
test raw::tests::readiness_contacts_nothing ... ok
test raw::tests::a_host_with_no_sound_stack_still_composes_a_launcher ... ok
test runtime::tests::missing_sip_credentials_fail_closed ... ok
test runtime::tests::stored_credential_readiness_is_value_free_and_reports_store_unavailability ... ok
test runtime::tests::stored_credentials_are_tenant_scoped_ordered_and_redacted ... ok
test runtime::tests::authority_key_must_be_an_owner_only_real_file ... ok
test backend::tests::a_binding_that_cannot_signal_refuses_rather_than_dropping_the_keypress ... ok
test backend::tests::an_unknown_session_is_not_found_and_a_refused_signal_is_reported ... ok
test backend::tests::readiness_delegates_to_the_mandatory_launcher_probe_without_launching ... ok
test backend::tests::a_signal_reaches_the_live_session_and_leaves_it_established ... ok
test backend::tests::catalog_projection_invocation_session_control_and_audit_share_one_path ... ok
test backend::tests::stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/integration_slack-c1ea08c672ba4a9c)

running 30 tests
test backend::tests::a_local_companion_submission_is_one_bot_token_and_nothing_else ... ok
test backend::tests::a_declared_instance_name_fixes_its_identity_for_good ... ok
test backend::tests::datasource_projection_excludes_unreviewed_slack_profile_fields ... ok
test backend::tests::a_credential_file_other_accounts_can_read_is_refused_rather_than_used ... ok
test backend::tests::hosted_companion_completion_requires_distinct_app_and_bot_credentials ... ok
test backend::tests::hosted_completion_errors_separate_conflicts_from_store_outages ... ok
test backend::tests::hosted_setup_page_requires_capability_and_distinguishes_safe_failures ... ok
test backend::tests::message_loop_guards_and_closed_event_grants_are_applied_before_storage ... ok
test backend::tests::only_the_inner_admitted_event_is_projected ... ok
test backend::tests::slack_auth_test_refuses_only_explicit_invalid_credentials ... ok
test backend::tests::slack_auth_test_provider_and_transport_failures_are_unavailable ... ok
test backend::tests::socket_ticket_destination_is_closed_to_slack_tls_hosts ... ok
test backend::tests::a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted ... ok
test backend::tests::invalid_hosted_capability_cannot_consume_a_connect_session ... ok
test tests::organization_credentials_do_not_claim_personal_oauth_is_configured ... ok
test backend::tests::event_is_durable_and_deduplicated_before_pull_and_replay ... ok
test backend::tests::operation_audit_is_durable_bounded_and_value_free ... ok
test backend::tests::describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource ... ok
test backend::tests::hosted_sessions_expire_and_release_pending_capacity_without_submission ... ok
test backend::tests::datasource_description_lease_ignores_request_scoped_provenance ... ok
test backend::tests::ephemeral_open_never_starts_a_socket_mode_supervisor ... ok
test backend::tests::read_ownership_matches_describe_ownership_for_every_slack_datasource ... ok
test backend::tests::standalone_adapter_claims_only_its_connection_and_event_families ... ok
test backend::tests::slack_readiness_is_value_free_and_tracks_the_secret_store ... ok
test backend::tests::organization_bot_is_admitted_for_reads_without_an_event_channel ... ok
test backend::tests::stale_grant_metadata_cannot_reenter_any_connection_or_event_surface ... ok
test backend::tests::one_use_completion_publishes_only_value_free_connection_state ... ok
test backend::tests::rate_adversary_slack_zero_delay_settles_each_explicit_write_refusal ... ok
test backend::tests::rate_final_slack_definite_refusal_survives_terminal_audit_failure ... ok
test backend::tests::rate_stage2_slack_definite_write_refusal_is_not_an_uncertain_outcome ... ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.46s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/monitoring_model-ba8bcc738bad9da9)

running 4 tests
test tests::loki_timestamps_stay_strings_and_the_refusal_names_the_encoding ... ok
test tests::prometheus_timestamps_accept_integer_epoch_seconds_beside_strings ... ok
test tests::the_validator_still_refuses_outside_the_documents_contract ... ok
test tests::the_validator_admits_the_documents_required_only_input ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s

     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/state_sqlite-b1d5ce5e44bf1d75)

running 11 tests
test tests::concatenation_would_have_corrupted_binary_and_the_transaction_does_not ... ok
test tests::full_open_refuses_unusable_paths ... ok
test tests::the_in_memory_backend_serves_grant_evaluation ... ok
test tests::the_in_memory_backend_conforms ... ok
test tests::the_file_backend_serves_grant_evaluation ... ok
test tests::a_cell_survives_reopening_the_file ... ok
test tests::the_file_backend_conforms ... ok
test tests::existing_openers_keep_normal_synchronization ... ok
test tests::full_open_configures_wal_and_full_synchronization_on_every_open ... ok
test tests::full_commits_are_visible_before_close_and_survive_reopening ... ok
test tests::the_full_file_backend_preserves_state_and_grant_conformance ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s

     Running tests/approval_gate.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/approval_gate-9774db699f1b2c77)

running 3 tests
test sixteen_concurrent_identical_presentations_redeem_exactly_once ... ok
test a_replay_survives_reopening_the_database ... ok
test a_crash_between_redemption_and_terminal_write_leaves_a_recoverable_attempted_row ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

   Doc-tests connect_session_transport

running 2 tests
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::BoundOAuthEndpoint (line 100) - compile fail ... ok
test ~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport/src/oauth.rs - oauth::OAuthCallback (line 55) - compile fail ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

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

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.result.json
````json
{
  "label": "auth-adversary1-runtime-no-default",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 24299585536,
    "tmpfs_free_bytes": 13905002496,
    "mem_available_bytes": 35689385984
  },
  "maximum_target_bytes": 10725044224,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:36:07.417574+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.command.json
````json
{
  "label": "auth-adversary1-runtime-no-default-clippy",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:36:40.937726+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Finished `dev` profile [unoptimized] target(s) in 1.26s
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.result.json
````json
{
  "label": "auth-adversary1-runtime-no-default-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 24293740544,
    "tmpfs_free_bytes": 13915312128,
    "mem_available_bytes": 35963215872
  },
  "maximum_target_bytes": 10714603520,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:36:43.717200+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.command.json
````json
{
  "label": "auth-adversary1-runtime-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:37:29.556588+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.log
````text

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.result.json
````json
{
  "label": "auth-adversary1-runtime-fmt",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 23986655232,
    "tmpfs_free_bytes": 13921034240,
    "mem_available_bytes": 35854606336
  },
  "maximum_target_bytes": 10714603520,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:37:32.347907+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.command.json
````json
{
  "label": "auth-adversary1-console-full",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:37:38.341150+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.log
````text
   Compiling connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Finished `test` profile [unoptimized] target(s) in 3.20s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_console-c3fd591076a9af80)

running 76 tests
test auth::tests::a_basic_credential_row_reports_whether_its_user_half_is_configured_and_never_the_value ... ok
test auth::tests::nothing_in_the_result_can_carry_a_secret ... ok
test admin::tests::explicit_secret_file_must_be_owner_only ... ok
test auth::tests::the_store_preference_matches_what_the_runtime_composes ... ok
test connect::tests::a_provider_outside_the_guided_set_is_refused_by_name ... ok
test connect::tests::the_error_for_an_unknown_provider_names_it ... ok
test doctor::tests::a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it ... ok
test doctor::tests::a_report_is_unhealthy_only_when_something_cannot_work ... ok
test doctor::tests::a_short_state_root_passes_both_budgets ... ok
test connect::personal_oauth_tests::instruction_file_refuses_shared_parent_symlink_and_existing_content ... ok
test doctor::tests::the_budget_is_measured_against_the_deepest_path_the_daemon_binds ... ok
test enrol::tests::a_provider_outside_the_catalogue_is_named_rather_than_guessed_at ... ok
test connect::tests::a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path ... ok
test doctor::tests::the_report_renders_every_check_as_data ... ok
test doctor::tests::every_state_a_check_can_report_reaches_the_reader_as_its_own_marker ... ok
test doctor::tests::doctor_names_the_default_local_target_and_its_socket ... ok
test envelope::tests::a_refusal_becomes_an_error_rather_than_a_result ... ok
test envelope::personal_oauth_tests::ordinary_connection_result_payload_has_no_private_instruction_endpoint ... ok
test envelope::tests::a_result_loses_its_envelope_and_its_discriminant ... ok
test admin::tests::command_shape_accepts_secret_stdin_without_a_secret_argument ... ok
test envelope::tests::an_envelope_carrying_neither_is_a_named_failure_not_an_empty_success ... ok
test init::tests::admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained ... ok
test init::tests::an_agent_id_is_stable_across_calls ... ok
test input::tests::input_accepts_only_the_stdin_marker ... ok
test input::tests::an_inline_object_is_parsed ... ok
test input::tests::no_source_names_all_three_rather_than_defaulting_to_empty ... ok
test init::tests::the_separator_keeps_a_concatenation_from_colliding ... ok
test init::tests::the_snapshot_digest_is_stable_and_moves_with_the_admitted_set ... ok
test input::tests::a_file_is_read_from_its_path ... ok
test output::tests::a_payload_carrying_its_own_value_field_is_left_alone ... ok
test init::tests::an_existing_configuration_is_never_replaced_silently ... ok
test output::tests::a_field_a_record_does_not_carry_reads_as_absent_rather_than_blank ... ok
test output::tests::a_record_that_is_not_an_object_keeps_the_name_the_report_gave_it ... ok
test output::tests::a_structured_format_carries_its_failure_on_stdout ... ok
test output::tests::a_row_shows_its_severity_before_anybody_reads_it ... ok
test output::tests::a_table_reads_left_to_right_with_the_column_that_runs_long_last ... ok
test output::tests::a_wide_character_cell_keeps_the_column_after_it_aligned ... ok
test output::tests::a_word_the_renderer_cannot_rank_is_marked_unknown_rather_than_good ... ok
test output::tests::an_object_with_two_arrays_is_not_unwrapped ... ok
test output::tests::an_empty_listing_is_an_empty_stream_rather_than_a_line_shaped_like_a_record ... ok
test output::tests::an_unranked_table_still_keeps_the_marker_column ... ok
test output::tests::columns_of_equal_width_keep_the_order_the_record_carries ... ok
test output::tests::compact_leaves_a_single_record_as_one_line ... ok
test output::tests::compact_keeps_a_field_a_record_carries_below_its_top_level ... ok
test output::tests::compact_keeps_the_scalar_a_list_response_carries_beside_its_records ... ok
test output::tests::every_protocol_state_this_renderer_can_be_handed_has_a_rank ... ok
test output::tests::compact_unwraps_the_one_array_a_list_response_carries ... ok
test output::tests::no_cell_is_ever_empty_so_no_row_can_end_in_whitespace ... ok
test output::tests::severity_survives_a_pipe_because_it_is_not_carried_by_colour ... ok
test output::tests::text_says_none_rather_than_printing_an_empty_bracket ... ok
test output::tests::text_does_not_quote_a_string_a_person_is_reading ... ok
test output::tests::text_keeps_every_field_a_record_carries_including_a_nested_list ... ok
test output::tests::text_spends_one_aligned_row_on_each_record ... ok
test output::tests::the_result_discriminant_is_stripped_so_compact_can_see_the_records ... ok
test output::tests::the_structured_formats_render_the_bytes_they_rendered_before ... ok
test output::tests::yaml_renders_through_the_maintained_crate ... ok
test output::tests::the_widest_column_moves_last_even_when_the_record_puts_it_first ... ok
test output::tests::a_cell_never_carries_a_character_that_breaks_the_row ... ok
test output::tests::every_status_word_this_package_emits_is_one_the_renderer_can_rank ... ok
test connect::personal_oauth_tests::instruction_file_is_exclusive_owner_only_and_cleared_on_drop ... ok
test init::tests::a_configuration_the_daemon_would_refuse_is_not_left_on_disk ... ok
test init::tests::what_init_writes_is_what_the_daemon_can_read ... ok
test connect::personal_oauth_tests::instruction_cleanup_never_removes_a_replacement_inode ... ok
test auth::tests::the_catalogue_is_what_says_a_credential_has_a_user_half ... ok
test enrol::tests::gitlab_asks_for_nothing_when_its_default_origin_is_wanted ... ok
test providers::tests::a_provider_without_a_probe_is_not_ready_and_says_why_by_omission ... ok
test enrol::tests::a_self_hosted_origin_is_the_case_operator_approval_exists_for ... ok
test enrol::tests::slack_declares_a_bot_and_a_user_credential_which_one_identity_may_both_hold ... ok
test enrol::tests::most_of_the_catalogue_asks_no_configuration_question_at_all ... ok
test providers::tests::an_unmatched_query_is_an_empty_listing_rather_than_the_whole_catalogue ... ok
test providers::tests::a_query_narrows_to_one_provider_and_its_summary_follows ... ok
test providers::tests::the_shipped_catalogue_is_reported_rather_than_asserted ... ok
test output::tests::a_table_too_wide_for_a_terminal_starts_its_last_column_inside_the_budget ... ok
test output::tests::two_providers_that_differ_in_their_id_differ_on_screen ... ok
test output::tests::the_budget_is_documented_as_what_it_is_and_a_real_row_is_wider_than_it ... ok
test output::tests::a_cell_the_budget_cut_says_so_and_the_column_names_are_cut_last ... ok

test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/adversary_budget_prose.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_budget_prose-4e62c7b456388460)

running 3 tests
test pass3_render_helper_child ... ok
test the_quoted_module_header_sentence_is_at_the_line_the_pass_two_suite_cites ... ok
test the_widths_the_documents_state_are_the_widths_the_renderer_prints ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s

     Running tests/adversary_readability.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_readability-9377cf56efde0b31)

running 7 tests
test render_helper_child ... ok
test an_unranked_table_lets_a_cell_sit_where_the_severity_marker_sits ... ok
test a_wide_character_cell_leaves_the_column_after_it_ragged ... ok
test a_record_whose_cells_are_all_empty_is_rendered_as_a_blank_line ... ok
test doctor_spreads_one_check_over_several_unmarked_lines_when_the_configuration_is_malformed ... ok
test compact_no_longer_puts_one_record_on_every_line ... ok
test providers_starts_its_last_column_past_the_width_of_any_terminal ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s

     Running tests/adversary_readability_pass2.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_readability_pass2-c18999e78fd39843)

running 5 tests
test pass2_render_helper_child ... ok
test compact_drops_the_name_of_the_array_a_report_carries ... ok
test a_column_the_budget_squeezes_to_nothing_pushes_every_later_column_out_of_line ... ok
test compact_answers_an_empty_listing_with_a_line_that_is_not_a_record ... ok
test the_last_column_of_providers_begins_one_column_past_the_terminal_it_is_laid_out_for ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s

     Running tests/personal_oauth.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/personal_oauth-04eff4149fab18d6)

running 11 tests
test ambiguous_profile_refuses_before_output_file_or_daemon_connection ... ok
test doctor_retains_ordinary_credential_store_diagnostic_for_an_owner_only_config ... ok
test headless_oauth_requires_private_file_before_session_creation ... ok
test doctor_reports_exact_redirect_and_unsealed_custody_without_client_material ... ok
test unsafe_private_destination_refuses_before_any_daemon_connection ... ok
test successful_private_daemon_label_cannot_reach_the_public_summary ... ok
test explicit_private_file_is_reserved_before_create_and_erased_before_public_success ... ok
test oauth_pass1_cancellation_erases_already_written_private_inode_before_return ... ok
test private_daemon_refusal_is_closed_on_stdout_and_stderr_in_all_formats ... ok
test oauth_pass1_real_controlling_pty_handoff_keeps_redirected_outputs_private ... ok
test oauth_pass2_private_file_expires_while_completion_grace_stays_bounded ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.05s

     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-f1ff96957d29feb1)

running 6 tests
test auth_stage2_hostile_output_child ... ok
test auth_stage2_bound_input_is_bounded_and_errors_do_not_echo_values ... ok
test auth_stage2_valid_hostile_daemon_output_is_private_in_every_real_format ... ok
test auth_stage2_bound_presenter_refuses_before_session_or_output ... ok
test auth_adversary_presenter_error_clears_original_inode_after_path_replacement ... ok
test auth_stage2_bound_presenter_clears_written_inode_on_success_expiry_and_drop ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s

   Doc-tests connectors_console

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.result.json
````json
{
  "label": "auth-adversary1-console-full",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 23139602432,
    "tmpfs_free_bytes": 13749981184,
    "mem_available_bytes": 34429091840
  },
  "maximum_target_bytes": 10884079616,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:38:09.084812+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.command.json
````json
{
  "label": "auth-adversary1-console-clippy",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:38:31.118336+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.log
````text
    Checking libc v0.2.189
    Checking serde_core v1.0.229
    Checking serde v1.0.229
    Checking zerovec v0.11.8
    Checking once_cell v1.21.4
    Checking memchr v2.8.3
    Checking tinystr v0.8.4
    Checking icu_locale_core v2.3.0
    Checking zerotrie v0.2.5
    Checking potential_utf v0.1.6
    Checking zeroize v1.9.0
    Checking icu_collections v2.3.0
    Checking icu_provider v2.3.0
    Checking linux-raw-sys v0.12.1
    Checking rustix v1.1.4
    Checking parking v2.2.1
    Checking serde_json v1.0.151
    Checking icu_normalizer v2.3.0
    Checking icu_properties v2.3.0
    Checking thiserror v2.0.20
    Checking idna_adapter v1.2.2
    Checking generic-array v0.14.7
    Checking idna v1.1.0
    Checking num-bigint v0.4.8
    Checking num-rational v0.4.2
    Checking crossbeam-utils v0.8.22
    Checking crypto-common v0.1.7
    Checking block-buffer v0.10.4
    Checking mio v1.2.2
    Checking socket2 v0.6.5
    Checking futures-io v0.3.34
    Checking tokio v1.53.1
    Checking ref-cast v1.0.27
    Checking num v0.4.3
    Checking digest v0.10.7
    Checking concurrent-queue v2.5.0
    Checking getrandom v0.3.4
    Checking event-listener v5.4.2
    Checking subtle v2.6.1
    Checking unicode-ident v1.0.24
    Checking futures-util v0.3.34
    Checking proc-macro2 v1.0.107
    Checking event-listener-strategy v0.5.4
    Checking futures-lite v2.6.1
    Checking url v2.5.8
    Checking tracing-core v0.1.36
    Checking tracing v0.1.44
    Checking quote v1.0.47
    Checking sha2 v0.10.9
    Checking aho-corasick v1.1.5
    Checking borrow-or-share v0.2.4
    Checking fluent-uri v0.4.1
    Checking regex-automata v0.4.18
    Checking syn v3.0.3
    Checking ahash v0.8.12
    Checking parking_lot_core v0.9.12
    Checking block-padding v0.4.2
    Checking polling v3.11.0
    Checking rustls-pki-types v1.15.1
    Checking getrandom v0.2.17
    Checking errno v0.3.14
    Checking async-task v4.7.1
    Checking winnow v1.0.4
    Checking zvariant_utils v4.2.0
    Checking ring v0.17.14
    Checking fraction v0.15.4
    Checking signal-hook-registry v1.4.8
    Checking async-io v2.6.0
    Checking inout v0.2.2
    Checking parking_lot v0.12.5
    Checking async-channel v2.5.0
    Checking enumflags2 v0.7.12
    Checking connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-state)
    Checking zcheapstr v1.1.0
    Checking endi v1.1.1
    Checking jsonschema-value v0.49.9
    Checking uuid-simd v0.8.0
    Checking schemars v1.2.2
    Checking zvariant v5.15.0
    Checking referencing v0.49.9
    Checking cipher v0.5.2
    Checking async-signal v0.2.14
    Checking fancy-regex v0.19.0
    Checking rustls-webpki v0.103.14
    Checking regex v1.13.1
    Checking async-lock v3.4.2
    Checking piper v0.2.5
    Checking connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-address)
    Checking futures-channel v0.3.34
    Checking email_address v0.2.9
    Checking fs2 v0.4.3
    Checking jsonschema v0.49.9
    Checking connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-secrets)
    Checking hyper v1.11.0
    Checking blocking v1.7.0
    Checking async-process v2.5.0
    Checking rustls v0.23.43
    Checking getrandom v0.4.3
    Checking zbus_names v4.3.4
    Checking domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/domain)
    Checking async-executor v1.14.0
    Checking async-broadcast v0.7.2
    Checking indexmap v2.14.0
    Checking ordered-stream v0.2.0
    Checking uuid v1.26.0
    Checking cpubits v0.1.1
    Checking signature v2.2.0
    Checking ipnet v2.12.1
    Checking tower v0.5.3
    Checking hyper-util v0.1.20
    Checking ed25519 v2.2.3
    Checking aes v0.9.2
    Checking catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog)
    Checking zbus v5.19.0
    Checking curve25519-dalek v4.1.3
    Checking hkdf v0.13.0
    Checking protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
    Checking tokio-rustls v0.26.4
    Checking cbc v0.2.1
    Checking webpki-roots v1.0.9
    Checking serde_spanned v0.6.9
    Checking toml_datetime v0.6.11
    Checking toml_edit v0.22.27
    Checking hyper-rustls v0.27.9
    Checking secret-service v5.2.0
    Checking libsqlite3-sys v0.35.0
    Checking ed25519-dalek v2.2.0
    Checking connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-resolve)
    Checking tower-http v0.6.11
    Checking serde_urlencoded v0.7.1
    Checking keyring-core v1.0.0
    Checking rusqlite v0.37.0
    Checking reqwest v0.12.28
    Checking zbus-secret-service-keyring-store v1.0.1
    Checking service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
    Checking toml v0.8.23
    Checking connector-oauth v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-oauth)
    Checking connect-session-transport v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport)
    Checking connectors-config v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-config)
    Checking keyring v4.2.0
    Checking identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
    Checking state-sqlite v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/state-sqlite)
    Checking rtoolbox v0.0.5
    Checking unsafe-libyaml-norway v0.2.15
    Checking serde_norway v0.9.42
    Checking rpassword v7.5.4
    Checking clap v4.6.6
    Checking integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Checking tempfile v3.27.0
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Finished `dev` profile [unoptimized] target(s) in 1m 00s
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.result.json
````json
{
  "label": "auth-adversary1-console-clippy",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 22507962368,
    "tmpfs_free_bytes": 13641510912,
    "mem_available_bytes": 34911526912
  },
  "maximum_target_bytes": 10988535808,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:39:33.393676+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.command.json
````json
{
  "label": "auth-adversary1-console-fmt",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 12884901888,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:42:07.395009+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.log
````text
Not launched: a prospective resource guard was crossed.
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.result.json
````json
{
  "label": "auth-adversary1-console-fmt",
  "exit": 125,
  "minimum": {
    "disk_free_bytes": 11004416000,
    "tmpfs_free_bytes": 13647101952,
    "mem_available_bytes": 35450433536
  },
  "maximum_target_bytes": 10988535808,
  "interruptions": [
    {
      "time": "2026-09-06T22:42:08.158494+00:00",
      "disk_free_bytes": 11004416000,
      "tmpfs_free_bytes": 13647101952,
      "mem_available_bytes": 35450433536,
      "target_bytes": 10988535808,
      "crossed": [
        "disk_free_bytes"
      ]
    }
  ],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:42:08.223943+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.command.json
````json
{
  "label": "auth-adversary1-console-fmt-continuation",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 8589934592,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:47:26.640598+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.log
````text

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.result.json
````json
{
  "label": "auth-adversary1-console-fmt-continuation",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 21445505024,
    "tmpfs_free_bytes": 13647101952,
    "mem_available_bytes": 35656110080
  },
  "maximum_target_bytes": 10988535808,
  "maximum_tmpdir_bytes": 4096,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:47:29.493936+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.command.json
````json
{
  "label": "auth-adversary1-cli-full-continuation",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 8589934592,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:47:38.871505+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.log
````text
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
   Compiling connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `test` profile [unoptimized] target(s) in 11.40s
     Running unittests src/lib.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/connectors_cli-c13928f076fe4de5)

running 5 tests
test tests::grafana_connect_uses_the_same_guided_surface ... ok
test tests::kubernetes_connect_accepts_an_exact_context_selection ... ok
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
test the_wire_name_rule_citation_in_the_design_document_points_at_the_rule ... ok
test the_wire_name_rule_citation_in_the_specification_points_at_the_rule ... ok
test the_copies_this_probe_carries_are_still_copies ... ok
test the_typeable_words_are_the_words_the_design_document_names ... ok
test a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries ... ok
test every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

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

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.67s

     Running tests/adversary_shim_pass4.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/adversary_shim_pass4-fdd3b6d12af54d38)

running 3 tests
test the_table_this_suite_copies_by_hand_is_the_table_the_binary_ships ... ok
test the_auth_group_still_answers_the_help_subcommand_it_advertised ... ok
test a_two_word_path_that_moved_works_with_the_global_flag_between_its_words ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

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
test every_declared_group_help_line_is_the_summary_the_specification_declares ... ok
test every_declared_group_is_a_group_of_the_parser ... ok
test no_path_is_both_declared_and_excepted ... ok
test every_path_of_the_parser_is_declared_or_a_named_exception ... ok
test no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare ... ok
test every_declaration_the_adversary_probe_copies_is_still_a_copy ... ok
test every_citation_this_unit_wrote_resolves ... ok
test personal_oauth_setup_requires_explicit_profile_and_private_instruction_option ... ok
test the_committed_generated_tree_is_the_specification_word_for_word ... ok
test the_old_login_selected_target_guard_is_absent ... ok
test every_citation_that_names_a_symbol_lands_on_its_declaration ... ok
test a_read_stops_being_an_exception_once_the_specification_declares_a_view ... ok
test a_command_absorbed_into_the_exception_list_alone_is_refused ... ok
test the_parser_accepts_target_before_and_after_each_dual_target_leaf ... ok
test target_conflict_does_not_wait_for_open_stdin ... ok
test the_read_verb_enumeration_partitions_the_protocols_it_names ... ok
test the_specification_names_the_binary_the_parser_builds ... ok
test the_target_countdown_is_exactly_what_the_parser_still_owes ... ok
test the_regeneration_command_the_documents_name_is_the_one_the_gate_runs ... ok
test targeted_errors_keep_the_target_in_yaml_and_text ... ok
test target_conflict_precedes_invoke_payload_loading ... ok
test the_exception_list_is_the_set_the_specification_enumerates ... ok
test the_kinds_the_tree_derives_are_the_kinds_the_list_carries ... ok
test an_explicit_hosted_target_requires_a_login_by_name_for_every_group ... ok
test target_conflict_precedes_missing_or_malformed_inline_input ... ok
test hosted_refuses_each_local_only_option_for_every_group ... ok
test an_exception_whose_kind_the_tree_contradicts_is_refused ... ok
test selected_target_preserves_provider_owned_target_fields_in_every_renderer ... ok
test local_success_and_protocol_refusals_report_the_selected_target ... ok
test broken_explicit_hosted_selection_never_falls_back_to_a_local_listener ... ok
test all_target_conflicts_precede_local_and_hosted_state_access ... ok
test every_local_leaf_ignores_broken_login_metadata_and_preserves_its_request ... ok
test an_omitted_target_ignores_a_saved_login_for_every_dual_target_group ... ok
test oauth_pass1_cli_private_setup_refusal_closes_all_output_formats_and_clears_file ... ok

test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.58s

     Running tests/cli_surface_drift.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/cli_surface_drift-6d0250b71da89e03)

running 10 tests
test the_thin_frontend_citation_points_at_the_thin_frontend_test ... ok
test the_copied_declarations_are_still_copies ... ok
test cutting_the_admin_group_over_to_the_generated_tree_is_refused ... ok
test a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused ... ok
test a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused ... ok
test a_command_added_under_a_declared_group_is_refused ... ok
test the_restated_contract_is_green_against_the_unchanged_tree ... ok
test a_command_added_under_the_wrong_declared_group_is_refused ... ok
test a_target_flag_removed_from_a_group_is_refused_by_the_countdown ... ok
test cargo_can_read_the_committed_emitted_manifest ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/cli_surface_pass_two.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/cli_surface_pass_two-cfe8de2fffcacee5)

running 6 tests
test the_design_document_describes_the_countdown_assertion_the_contract_makes ... ok
test the_design_document_names_only_constants_that_exist ... ok
test the_design_document_states_the_shape_of_the_exception_list ... ok
test the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers ... ok
test the_drift_suites_copies_are_checked_rather_than_cited ... ok
test the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/closed_pipe.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/closed_pipe-faf5d0d83388d04c)

running 20 tests
test completion_scripts_keep_other_output_write_failures_unsuccessful ... ok
test completion_scripts_still_accept_a_closed_reader ... ok
test admin_authentication_failures_remain_unsuccessful_with_a_closed_reader ... ok
test stdin_read_errors_stay_unsuccessful_with_a_closed_output_reader ... ok
test a_closed_transport_stays_unsuccessful ... ok
test an_unhealthy_doctor_stays_unsuccessful_when_its_report_reader_closes ... ok
test a_healthy_doctor_accepts_a_closed_report_reader ... ok
test successful_admin_results_accept_a_closed_reader_in_every_format ... ok
test setup_init_commits_its_result_before_a_reader_close_but_keeps_repeat_refusal ... ok
test successful_admin_credential_write_accepts_a_closed_reader ... ok
test text_consumer_closes_early ... ok
test every_admin_leaf_preserves_non_broken_pipe_output_failures ... ok
test json_consumer_closes_early ... ok
test compact_consumer_closes_early ... ok
test yaml_consumer_closes_early ... ok
test each_unhealthy_report_class_keeps_its_failure_when_output_closes ... ok
test every_format_really_emits_more_than_a_64_kib_pipe_buffer ... ok
test protocol_refusals_keep_their_failure_when_a_result_reader_closes ... ok
test each_protocol_search_distinguishes_closed_readers_from_other_write_failures ... ok
test a_real_non_broken_pipe_output_failure_stays_unsuccessful ... ok

test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.57s

     Running tests/first_level_groups.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/first_level_groups-dafc97dfb2159b1d)

running 5 tests
test the_first_level_is_eight_words ... ok
test doctor_reports_the_same_installation_at_both_paths ... ok
test the_serve_group_answers_bare_and_with_help_like_the_other_groups ... ok
test a_path_of_the_new_tree_is_left_alone ... ok
test every_moved_path_still_works_and_names_where_it_went ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/moved_paths_are_not_taught.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/moved_paths_are_not_taught-521e266820c15e14)

running 1 test
test nothing_this_product_prints_names_a_path_that_moved ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/one_shot_operations.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/one_shot_operations-dd9b3dc280dfc31c)

running 23 tests
test doctor_enumerates_bounded_and_persistent_verbs ... ok
test a_running_daemon_is_used_without_constructing_a_local_runtime ... ok
test a_transport_that_drops_the_request_is_never_retried_locally ... ok
test connection_mutations_require_daemon_before_creating_continuation_state ... ok
test adversary_hosted_refusal_and_target_conflict_never_construct_the_local_runtime ... ok
test events_and_session_signals_name_the_persistent_daemon_requirement ... ok
test existing_or_unsafe_socket_objects_never_trigger_ephemeral_fallback ... ok
test adversary_json_source_and_size_refusals_precede_one_shot_state_creation ... ok
test invalid_bounds_and_unsafe_state_refuse_before_runtime_state_is_opened ... ok
test rate_final_cli_describe_spelling_and_invalid_advice_never_resend ... ok
test rate_adversary_cli_keeps_integer_extremes_and_never_resends_before_exit ... ok
test final_adversary_kubernetes_candidates_never_publish_a_dead_connection_or_run_auth_exec ... ok
test rate_stage2_cli_json_and_yaml_preserve_delay_and_never_resend_an_invoke ... ok
test rate_stage2_one_shot_refusals_preserve_retriable_without_inventing_delay ... ok
test ordinary_search_and_connection_list_use_default_paths_without_a_daemon ... ok
test separate_describe_and_invoke_processes_reuse_the_same_authority_without_a_daemon ... ok
test concurrent_commands_and_daemon_start_cannot_take_the_in_flight_invocation_state ... ok
test final_adversary_provider_cursor_survives_two_distinct_one_shot_processes ... ok
test adversary_uncertain_invoke_never_resends_after_the_control_socket_disappears ... ok
test final_adversary_invalid_provider_output_is_not_resent_and_releases_the_state_root ... ok
test a_changed_authority_or_selected_connection_never_reaches_fixture_egress ... ok
test adversary_caller_input_cannot_rebind_routes_or_revoked_grants ... ok
test browser_session_operations_are_refused_under_canonical_and_published_aliases ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.47s

     Running tests/remediation.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/remediation-9806fb21acc5f093)

running 8 tests
test auth_stage2_bound_grammar_pairs_targets_and_preserves_existing_provider_mode ... ok
test auth_stage2_bound_setup_requires_daemon_before_input_or_private_file ... ok
test auth_stage2_unknown_operation_version_refuses_before_input_or_socket ... ok
test auth_stage2_operation_versions_select_the_real_exchange_without_resend ... ok
test auth_stage2_real_cli_auth_refusals_keep_every_format_private_without_resend ... ok
test auth_stage2_v3_refusal_keeps_failure_and_privacy_with_open_or_closed_output ... ok
test auth_stage2_bound_overrides_refuse_before_waiting_for_stdin ... ok
test auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination ... FAILED

failures:

---- auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination stdout ----
real CLI with stdin kept open; timed-out fixture children were killed and joined: [("absent", false, Some(1)), ("regular", true, None), ("symlink", true, None)]

thread 'auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination' (2742351) panicked at tests/remediation.rs:590:5:
an existing non-socket or symlink must be refused before blocking on caller stdin
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    auth_adversary_cli_unsafe_daemon_objects_refuse_before_open_stdin_or_private_destination

test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.03s

error: test failed, to rerun pass `--test remediation`
     Running tests/search_bounds.rs (/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target/debug/deps/search_bounds-03ef24390f00ba96)

running 4 tests
test every_search_help_names_its_protocol_range_and_existing_default ... ok
test every_search_parser_refuses_zero_and_values_above_the_protocol_maximum ... ok
test every_search_preserves_its_default_and_accepts_both_protocol_edges ... ok
test invalid_search_limits_exit_before_target_configuration_or_transport ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s

   Doc-tests connectors_cli

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 1 target failed:
    `--test remediation`
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.result.json
````json
{
  "label": "auth-adversary1-cli-full-continuation",
  "exit": 101,
  "minimum": {
    "disk_free_bytes": 21255217152,
    "tmpfs_free_bytes": 13494546432,
    "mem_available_bytes": 35187417088
  },
  "maximum_target_bytes": 11135500288,
  "maximum_tmpdir_bytes": 2080768,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:48:30.775022+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.command.json
````json
{
  "label": "auth-adversary1-cli-clippy-continuation",
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
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 8589934592,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:49:15.468582+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.log
````text
    Checking serde v1.0.229
    Checking log v0.4.34
    Checking mio v1.2.3
    Checking tokio v1.53.1
    Checking futures-util v0.3.34
    Checking indexmap v2.14.1
    Checking thiserror v2.0.20
    Checking zerovec v0.11.8
    Checking ipnet v2.12.1
    Checking tracing v0.1.44
    Checking serde_json v1.0.151
    Checking tinystr v0.8.4
    Checking icu_locale_core v2.3.0
    Checking potential_utf v0.1.6
    Checking zerotrie v0.2.5
    Checking icu_collections v2.3.0
    Checking icu_provider v2.3.1
    Checking ring v0.17.14
    Checking icu_properties v2.3.0
    Checking icu_normalizer v2.3.0
    Checking idna_adapter v1.2.2
    Checking idna v1.1.0
    Checking aws-lc-sys v0.45.0
    Checking aws-lc-rs v1.18.1
    Checking url v2.5.8
    Checking rustls-webpki v0.103.15
    Checking rustls v0.23.43
    Checking tokio-util v0.7.19
    Checking tokio-rustls v0.26.4
    Checking uuid v1.26.0
    Checking hyper v1.11.1
    Checking hyper-util v0.1.20
    Checking tower v0.5.3
    Checking ref-cast v1.0.27
    Checking tower-http v0.6.11
    Checking hyper-rustls v0.27.9
    Checking ahash v0.8.12
    Checking num v0.4.3
    Checking serde_urlencoded v0.7.1
    Checking connector-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-state)
    Checking reqwest v0.12.28
    Checking cpufeatures v0.3.1
    Checking schemars v1.2.2
    Checking fluent-uri v0.4.1
    Checking fraction v0.15.4
warning: connector-secrets@0.6.5: no live Vault was offered (CONNECTOR_SECRETS_VAULT_ADDR and CONNECTOR_SECRETS_VAULT_TOKEN are unset), so the reqwest HttpTransport is UNEXERCISED by this build: tests/vault_live.rs is compiled #[ignore]d
    Checking uuid-simd v0.8.0
    Checking domain v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/domain)
    Checking connector-address v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-address)
    Checking email_address v0.2.9
    Checking referencing v0.49.9
    Checking connector-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-secrets)
    Checking jsonschema-value v0.49.9
    Checking jsonschema v0.49.9
    Checking catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/catalog)
    Checking protocol v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/protocol)
    Checking connector-resolve v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-resolve)
    Checking service v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/service)
    Checking chacha20 v0.10.2
    Checking rand v0.10.2
    Checking webrtc-util v0.12.0
    Checking futures-executor v0.3.34
    Checking futures v0.3.34
    Checking serde_spanned v0.6.9
    Checking toml_datetime v0.6.11
    Checking toml_edit v0.22.27
    Checking either v1.18.0
    Checking toml v0.8.23
    Checking connectors-config v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-config)
    Checking concurrent-queue v2.5.0
    Checking sha2 v0.11.0
    Checking asn1-rs v0.6.2
    Checking connector-oauth v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connector-oauth)
    Checking tungstenite v0.28.0
    Checking prefix-trie v0.8.4
    Checking hickory-proto v0.26.1
    Checking syn v3.0.4
    Checking oid-registry v0.7.1
    Checking der-parser v9.0.0
    Checking rtp v0.14.0
    Checking sha1 v0.11.0
    Checking quinn-proto v0.11.17
    Checking moka v0.12.16
    Checking zvariant_utils v4.2.0
    Checking async-io v2.6.0
    Checking tungstenite v0.30.0
    Checking x509-parser v0.16.0
    Checking hickory-net v0.26.1
    Checking quinn-udp v0.5.15
    Checking async-channel v2.5.0
    Checking stun v0.9.0
    Checking enumflags2 v0.7.12
    Checking sipx-sip v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking rtcp v0.14.0
    Checking zcheapstr v1.1.0
    Checking hickory-resolver v0.26.1
    Checking zvariant v5.15.0
    Checking webrtc-mdns v0.10.0
    Checking tokio-tungstenite v0.28.0
    Checking webrtc-srtp v0.16.0
    Checking rcgen v0.13.2
    Checking turn v0.11.0
    Checking quinn v0.11.11
    Checking tokio-tungstenite v0.30.0
    Checking async-signal v0.2.14
    Checking webrtc-sctp v0.13.0
    Checking bincode v1.3.3
    Checking postgres-protocol v0.6.12
    Checking dtls v0.13.0
    Checking blocking v1.7.0
    Checking async-process v2.5.0
    Checking webrtc-data v0.12.0
    Checking sdp v0.10.0
    Checking sipx-transport v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking webrtc-ice v0.14.0
    Checking interceptor v0.15.0
    Checking zbus_names v4.3.4
    Checking async-executor v1.14.0
    Checking webrtc-media v0.11.0
    Checking sipx-rtp v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking sipx-sdp v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking fluent-uri v0.3.2
    Checking sipx-audio v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking smol_str v0.2.2
    Checking webrtc v0.14.0
    Checking aes v0.9.2
    Checking zbus v5.19.0
    Checking sipx-ua v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking sipx-media v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking serde-value v0.7.0
    Checking referencing v0.33.0
    Checking libsqlite3-sys v0.35.0
    Checking k8s-openapi v0.28.0
    Checking postgres-types v0.2.14
    Checking connect-session-transport v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connect-session-transport)
    Checking rustls-platform-verifier v0.7.0
    Checking serde-saphyr v0.0.27
    Checking rusqlite v0.37.0
    Checking tokio-postgres v0.7.18
    Checking reqwest v0.13.4
    Checking clap v4.6.6
    Checking jsonschema v0.33.0
    Checking secret-service v5.2.0
    Checking kube-core v4.0.0
    Checking process-wrap v9.1.0
    Checking jsonpath-rust v1.0.10
    Checking sipx-call v1.0.0-rc.23 (https://github.com/codewandler/sipx?rev=004ac534b8b222060ad2d2308763efe6e1dedc10#004ac534)
    Checking rtvbp v0.1.0 (https://github.com/babelforce/rtvbp?rev=dc0a60f7425b4899885f372152028457791b1e72#dc0a60f7)
    Checking driver-audio v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-audio)
    Checking axum-core v0.5.6
    Checking sse-stream v0.2.5
    Checking hyper-timeout v0.5.2
    Checking chrono v0.4.45
    Checking tokio-stream v0.1.19
    Checking keyring-core v1.0.0
    Checking axum v0.8.9
    Checking rmcp v3.2.0
    Checking kube-client v4.0.0
    Checking zbus-secret-service-keyring-store v1.0.1
    Checking rtvbp-voice-endpoint v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/rtvbp-voice-endpoint)
    Checking driver-sip v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-sip)
    Checking postgres v0.19.14
    Checking state-sqlite v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/state-sqlite)
    Checking subscription-custody v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/subscription-custody)
    Checking monitoring-model v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/monitoring-model)
    Checking b10x-mcp-types v0.1.1 (https://github.com/beyond10x/mcp?rev=cb3b13a37dfef645ddfe916adc3cb40e82b7f620#cb3b13a3)
    Checking b10x-mcp-client v0.1.1 (https://github.com/beyond10x/mcp?rev=cb3b13a37dfef645ddfe916adc3cb40e82b7f620#cb3b13a3)
    Checking server v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/server)
    Checking integration-catalog v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-catalog)
    Checking hosted-state v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/hosted-state)
    Checking voice-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/voice-runtime)
    Checking keyring v4.2.0
    Checking kube v4.0.0
    Checking driver-speech v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-speech)
    Checking driver-cdp v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/driver-cdp)
    Checking hosted-vault v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/hosted-vault)
    Checking voice-local-audio v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/voice-local-audio)
    Checking identity-client v0.5.6 (https://github.com/beyond10x/identity.git?tag=0.5.6#e3231bc3)
    Checking rtoolbox v0.0.6
    Checking serde_norway v0.9.42
    Checking rpassword v7.5.4
    Checking connectors-client v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-client)
    Checking integration-sip v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-sip)
    Checking hosted-secrets v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/hosted-secrets)
    Checking integration-platform v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-platform)
    Checking integration-kubernetes v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-kubernetes)
    Checking identity-http v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/identity-http)
    Checking integration-mcp v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-mcp)
    Checking integration-monitoring v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-monitoring)
    Checking integration-jira v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-jira)
    Checking integration-slack v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-slack)
    Checking integration-gitlab v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/integration-gitlab)
    Checking connectors-runtime v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-runtime)
    Checking connectors-console v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-console)
    Checking clap_complete v4.6.9
    Checking connectors-cli v0.6.5 (~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli)
    Finished `dev` profile [unoptimized] target(s) in 2m 08s
````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.result.json
````json
{
  "label": "auth-adversary1-cli-clippy-continuation",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19298996224,
    "tmpfs_free_bytes": 13297266688,
    "mem_available_bytes": 33479348224
  },
  "maximum_target_bytes": 11337584640,
  "maximum_tmpdir_bytes": 1396736,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:51:24.755880+00:00"
}
````

Command/environment/guard: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.command.json
````json
{
  "label": "auth-adversary1-cli-fmt-continuation",
  "argv": [
    "cargo",
    "fmt",
    "--all",
    "--check"
  ],
  "cwd": "~/.local/state/worktree/trees/b10x/connectors/wt-78f189927231/crates/connectors-cli",
  "env_overrides": {
    "TMPDIR": "~/.cache/cw6/av1",
    "CARGO_TARGET_DIR": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "0",
    "CARGO_PROFILE_TEST_DEBUG": "0",
    "CARGO_BUILD_JOBS": "1",
    "PATH": "~/.cargo/bin:~/.local/bin:~/.deno/bin:/home/linuxbrew/.linuxbrew/Caskroom/codex/0.153.4/codex-path:~/.codex/tmp/arg0/codex-arg05Bvydg:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.opencode/bin:~/.fly/bin:~/.cargo/bin:~:~/anaconda/bin:/usr/bin:~/.rbenv:/usr/local/go/bin:~/go/bin:~/go:~/.local/share/gem/ruby/3.0.0/bin:~/.deno/bin:~/.yarn/bin:~/.pulumi/bin:/opt/rocm/bin:~/.bun/bin:/home/linuxbrew/.linuxbrew/bin:/home/linuxbrew/.linuxbrew/sbin:~/.sdkman/candidates/scala/current/bin:~/.sdkman/candidates/maven/current/bin:~/.sdkman/candidates/java/current/bin:~/.sdkman/candidates/groovy/current/bin:~/.sdkman/candidates/grails/current/bin:~/.sdkman/candidates/gradle/current/bin:~/.local/bin:/usr/local/bin:/bin:/usr/local/sbin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems:/usr/lib/jvm/default/bin:/usr/bin/site_perl:/usr/bin/vendor_perl:/usr/bin/core_perl:/usr/lib/rustup/bin:/var/lib/snapd/snap/bin:/opt/cuda/bin:/opt/cuda/integration/nsight-compute:/opt/cuda/integration/nsight-systems"
  },
  "env_unset": [
    "RUSTC_WRAPPER"
  ],
  "limits": {
    "disk_free_bytes": 8589934592,
    "tmpfs_free_bytes": 8589934592,
    "mem_available_bytes": 17179869184,
    "target_max_bytes": 12884901888,
    "tmpdir_max_bytes": 134217728
  },
  "target": "/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target",
  "tmpfs": "/dev/shm",
  "sample_interval_seconds": 1,
  "started": "2026-09-06T22:51:48.689840+00:00"
}
````

Complete raw output: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.log
````text

````

Actual result: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.result.json
````json
{
  "label": "auth-adversary1-cli-fmt-continuation",
  "exit": 0,
  "minimum": {
    "disk_free_bytes": 19551580160,
    "tmpfs_free_bytes": 13291675648,
    "mem_available_bytes": 34140917760
  },
  "maximum_target_bytes": 11337584640,
  "maximum_tmpdir_bytes": 1396736,
  "interruptions": [],
  "remaining_process_group": [],
  "ended": "2026-09-06T22:51:51.616641+00:00"
}
````

3. Findings: measured assertion and actual caller/configuration.

| Source | Verdict / origin | Finding | Measured | Reaches it |
| --- | --- | --- | --- | --- |
| crates/connectors-client/src/remediation.rs:430 | NEEDS-CHANGE / undecided | The bound completion client accepts a fresh Operation binding with an explicitly conflicting credential purpose and reports readiness after acknowledgement. | First exit 101; matching=true, absent=true, conflicting=true, split-pair=true. The last two violate the assertion; root full repeats the failure. | LocalClient::finish_remediation is called by connectors-console::remediation::run and the bound CLI branch. The actual client accepts hostile but canonically valid daemon DTOs, an explicit assignment threat model. No Invoke occurs. The production OAuth owner is not claimed to naturally emit that conflicting DTO. |
| crates/server/src/hosted/docs/openapi.json:719 | INFEASIBLE / undecided | With the explicit synthetic hosted readiness backend, selected v1/v2 neutral credential refusals return HTTP 409 but fail the served v3-only 409 response schema; no current production hosted acquisition producer was found. | First exit 101; actual selected v1/v2 return HTTP 409 and schema=false, while v3 returns 409/schema=true. Root full repeats the failure. | The actual hosted route executes admission with a synthetic verifier/principal and readiness backend, plus a real MemoryState GrantSet. Later revocation gives neutral 403 for all three versions before any further readiness. Dispatch, session work and approval audit stay zero. No current production hosted acquisition producer for this state was found: it remains Unsupported. This is INFEASIBLE for that production posture. |
| crates/connectors-cli/src/lib.rs:879 | CONFIRMED / undecided | Bound CLI setup with --input - blocks on stdin when connectors.sock is a regular file or symlink because the socket safety check happens after input acquisition. | First exit 101; absent socket exits 1 immediately; regular and symlink fixtures remain blocked for three seconds, then are killed and joined. CLI full repeats the failure. | The real bound CLI branch reads a valid fixture configuration and enters read_input after local_socket_absent sees only an existing directory entry. require_daemon checks socket type/safety later. Thus an ordinary unsafe state-root socket object reaches blocking stdin before refusal, independently of any provider work. |

All three origins remain undecided under the charter: no base execution tree or scratch compiler output was assigned for these cases. Reading the base/delta is not an old-loader execution. Current caller anchors are pinned in ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/finding-callers.json, source inventories and the retained whole-unit/protocol/service patches. Nothing here claims credential publication or operation-dispatch bypass from a false client readiness result.

4. Attacks that stayed green.

- Strict decoders driven from the unit vectors reject escaped duplicate identity/correlation keys and over-budget padded frames.
- Actual personal OAuth owner classifies deleted access, expired/deleted refresh and revoked authority without refreshing or invoking.
- Actual local daemon, registry and OAuth custody keep two same-profile configured bindings separate through completion, wrong-owner status, wrong-target ack and discovery.
- Actual registry refuses split/ambiguous owners and preserves claimed-owner Refused/Unavailable without dispatch.
- Actual presenter reserves before Start and clears its original private inode on error after path rename/replacement; synthetic servers are joined.
- Retained full cases cover v1/v2/v3 selection, no resend, real 401 renewal versus typed 409, nonconsuming hosted approval, canonical hostile CLI/console/MCP reduction, owner authority/publication/status/ack rechecks, concurrent/expired acknowledgement, external-schema retrieval refusal and receiver-only daemon guidance.
- Full root tests retain served-doc vector and inherited-schema checks. ESS/domain/generated deltas were inspected as source: hosted acquisition remains Unsupported, setup connect has no generated forwarding, and no durable remediation entity or combined hosted Identity scope is introduced. No live adoption or fresh ESS generation is claimed.

5. Resources, retained scope and every outside path.

````json
{
  "minimum": {
    "disk_free_bytes": 11004416000,
    "tmpfs_free_bytes": 13291675648,
    "mem_available_bytes": 33479348224
  },
  "maximum_target_bytes": 11337584640,
  "maximum_measured_tmpdir_bytes": 2080768,
  "guard_observations": [
    {
      "label": "auth-adversary1-console-fmt",
      "exit": 125,
      "observations": [
        {
          "time": "2026-09-06T22:42:08.158494+00:00",
          "disk_free_bytes": 11004416000,
          "tmpfs_free_bytes": 13647101952,
          "mem_available_bytes": 35450433536,
          "target_bytes": 10988535808,
          "crossed": [
            "disk_free_bytes"
          ]
        }
      ]
    }
  ],
  "remaining_process_groups": [],
  "commands": 23
}
````

All owned command process groups are empty. Per-command assignments retain jobs=1, incremental=0, dev/test debug=0 and RUSTC_WRAPPER unset; source/target ownership never changed. Guard refusals and prospective assignments above are preserved. No target cleanup, Git/AEP mutation, source fix, installed-binary change or operator configuration change occurred. Compiler release: ~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/compile-slot-release.json.

~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/source-preservation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/command-index.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/runner-counts.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-executables.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-tmp-inventory.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/prior-evidence-verification.json

Assigned roots and tool-managed paths:

````text
/dev/shm/b10x-cli-wave-wt-78f189927231-urhrmq5y/runtime-target
~/.cache/cw6/av1
~/.cargo/.global-cache
~/.cargo/.package-cache
~/.cargo/.package-cache-mutate
````

Every retained reviewer scratch path, including this report pair and manifest:

````text
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/affected-gates-plan.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-clippy-continuation.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-fmt-continuation.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-cli-full-continuation.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt-continuation.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-console-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-hosted-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-local-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-presenter-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-protocol-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-purpose-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-readiness-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-registry-first.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-root-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-fmt.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-full.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default-clippy.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.command.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.exit
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.log
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.process.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.resources.jsonl
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.result.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/auth-adversary1-runtime-no-default.warm-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/before-counts.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/brief.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/candidate-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/command-index.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/compile-slot-release.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/deciding-results.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/diffstat.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/evidence.sha256
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-executables.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-target.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/final-tmp-inventory.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/finding-callers.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/findings.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/format-tool-observation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/layout-precheck-observation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/layout-precheck-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-run-lane.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/connectors-cli/tests/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/connectors-client/src/tests.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/connectors-console/tests/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/connectors-runtime/src/remediation_tests.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/connectors-runtime/tests/one_shot_runtime.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/connectors-runtime/tests/personal_oauth.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/integration-catalog/src/oauth_remediation_tests.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/protocol/tests/bundles.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/server/src/hosted/tests/docs.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/original-test-files/crates/server/src/hosted/tests/remediation.rs
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/outside-paths.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/owned-test-paths.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/pre-full-preservation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/pre-full-source.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/pre-full-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/prior-evidence-verification.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/protocol-first-tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/protocol-source.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/raw-report.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/report-construction-observation.txt
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/report.md
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/resource-continuation-assignment.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/resource-summary.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/run-lane-continuation.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/run-lane.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/runner-counts.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/seal-pass.py
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/service-source.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/source-preservation.json
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/tests.patch
~/.cache/connectors-cli-wave-20260906/auth-as-tool-result/whole-unit-adversary-1/whole-unit-source.patch
````

Target and retained temp descendants are enumerated in the exact inventories above; normal fixture teardown is distinct from cleanup commands. The manifest seals every retained scratch member except itself.

```findings
- file: "crates/connectors-client/src/remediation.rs"
  line: 430
  category: "acceptance"
  severity: "blocker"
  verdict: "NEEDS-CHANGE"
  origin: "undecided"
  message: "The bound completion client accepts a fresh Operation binding with an explicitly conflicting credential purpose and reports readiness after acknowledgement."
- file: "crates/server/src/hosted/docs/openapi.json"
  line: 719
  category: "contract-drift"
  severity: "warning"
  verdict: "INFEASIBLE"
  origin: "undecided"
  message: "With the explicit synthetic hosted readiness backend, selected v1/v2 neutral credential refusals return HTTP 409 but fail the served v3-only 409 response schema; no current production hosted acquisition producer was found."
- file: "crates/connectors-cli/src/lib.rs"
  line: 879
  category: "acceptance"
  severity: "warning"
  verdict: "CONFIRMED"
  origin: "undecided"
  message: "Bound CLI setup with --input - blocks on stdin when connectors.sock is a regular file or symlink because the socket safety check happens after input acquisition."
```
