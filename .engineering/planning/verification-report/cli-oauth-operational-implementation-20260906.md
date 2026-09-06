---
format: aep.planning-md/1
id: verification-report:cli-oauth-operational-implementation-20260906
kind: verification-report
status: draft
title: Frozen personal OAuth operational implementation
relations:
- verifies: story:connect-session-oauth-custody-in-personal-posture
revision: 1
---
Coordinator checkpoint, 2026-09-06. The complete implementor report follows unchanged. Root verified all 608 frozen evidence hashes and all 1,188 original source hashes; the evidence manifest SHA256 is efea8eb039d5b92572550e19bf0b1e16143d9df4c195e735735921a7659ce52e. Report SHA256 is 47d608273c36207f38c6afd43b574190a3b5f1bde94670e2a07e4debc0bf9cd1; its separately preserved portable counterpart has SHA256 3ffbcce389e1bfcc271addbf4efc9530c1a1a74a340413bc549888fb901e3006.

A planning-only fast-forward from df2855b9 to 789525f3 preserved every one of the 1,017 nonplanning source hashes. The exact 127 scoped source paths were committed locally as 6fae9df000986f39d009a2e8503bdff03db41762, tree dbb13f2312cf785fa066548c2a72112c77ce1fc0, with both bot identities verified and all 1,017 hashes rechecked. Source is clean. The original report retains its historical uncommitted observation. Root proof files are oauth-operational-root-freeze-verification.json, oauth-operational-fast-forward-proof.json and oauth-operational-source-commit-proof.json in the assigned coordinator scratch.

This records affected-package implementor execution, not a whole-unit adversary verdict, a full integration gate, public delivery or installed runtime. The story remains active. The preserved complete-base review brief has SHA256 4b4b29bbbf958d88166c0ef5291dfe8f493eaffbfa675d15b5b6b835aa4b8fab and names the exact 0c694509 published base, all prior OAuth source stages and test-only ownership.

---

# OAuth stage 2C operational implementation handoff

The scoped operational implementation is ready for coordinator inspection and whole-unit independent review. The retained affected-package executions cover **1,496 passing tests and one preexisting ignored reader measurement** without adding repeated runs to the total. Strict affected Clippy and formatting results are recorded below. This is an implementor handoff, not a whole-story review, full twelve-workspace gate or publication claim.

The source is uncommitted in `/home/timo/.local/state/worktree/trees/b10x/connectors/wt-4f1de73d0685`, HEAD `df2855b91e71e3349c44345eb06e9c9447438e41`, with the original stage2C source baseline `87149e7d5181ce5dff1ccdeceefd1e0bd34e9e30`. The coordinator's later machine scope, including OAuth story71/runbook43 at `36d04fef238ec18581d7b2136d9fcc9e9db71caf`, remains authoritative. No Git/AEP/ESS/model, operator, release or live-provider action was performed by this implementor. The reserved independent-review test owner is untouched.

The full-unit source union is recorded in `stage2c-whole-unit-source-bases.json` and four retained binary-capable commit patches: helper `fc26525e` over `66dbede1`; FULL `81fac96b` over `fc26525e`; completion kernel `3b6c72d3` over `eef5ccb6`; private refresh `9d1cef37` over `67e60406`; then this uncommitted stage2C snapshot. Every source commit is an ancestor of the current tree. This explicit union avoids treating intervening rate/planning merges as OAuth implementation. The coordinator-selected published review base is `0c69450921ab1794c81dadec915b717a61bf0983`; each of the four source commits is absent from that base and present in the current lineage. Unrelated Design22/model-only preparation is excluded from this OAuth source union. Earlier stage reports remain separate and their counts are not added to the stage2C total.

## Resulting behavior

Schema 4 declares the public personal OAuth flows and observed token-info requirements while retaining exact frozen schema 2/3 artifacts. The current reader admits version 4 and refuses the old/future versions at its documented boundary. All 65 canonical documents preserve their operation schemas, rates and ordinary auth facts after removing only the recorded top-level version identities and GitLab's personal-flows/device-grant addition. The earlier deterministic build/diff/check evidence remains immutable: 65 providers, 70 planned artifacts, a second build with zero writes. Official GitLab documentation grounding is retained in `stage2c-provider-source-recheck.md`; synthetic runtime fixtures are not live interoperability evidence.

Config admits explicit GitLab public development registrations with an allowed-scope ceiling and a dedicated **unsealed** FileStore. Production custody refuses this development-only posture. Owner, integration and admitted profile/default must identify exactly one configured stable binding before session, listener or egress. The runtime composes one OAuth owner over one FileStore and the same FULL SQLite Arc through acquisition, coherent read, refresh and dispatch. Exact Connection ownership separates raw and OAuth Invoke dispatch; Search/Describe aggregation remains unchanged, and ownership is not a grant.

Actual PKCE loopback and device acquisition validate observed client, subject and scopes. Completion claims live authority/generation and one receiver-owned time strictly before the original deadline, then writes the FULL decision, commits credentials and publishes coherent metadata. I/O may finish after a timely claim. Uncertain writes remain unavailable until guarded recovery; they never report false Completed or Expired. Real SQLite/FileStore close/reopen fixtures exercise decision absence, confirmed decision, committed secrets before publication, and publication before receipt reclamation. The temporary unused-module expectation is removed.

Refresh runs under that same binding gate after the unchanged raw lease/grant/input validation. Its receiver-owned 30-second window is captured before egress and never reset. Source inspection found no usable outer deadline in the existing lease. Token and token-info egress consume the original remaining budget. A FULL-confirmed, binding/generation-specific refresh-attempt marker precedes egress; uncertain marker writes send zero requests. A surviving marker blocks old-token reuse after rotation followed by token-info/prepare failure and actual reopen. There is no claim of remote rollback, power-loss simulation or secret-store digest readback. The existing custody Image/version and transaction compatibility remain unchanged.

Transport liveness is an opaque read-only observation using the admitted receiver clock. Retirement/drop flips it before receiver reply completion; status observes it at its serialized decision point, without promising the URL cannot retire immediately afterward. Daemon shutdown retires and joins owned tasks. Stopping Connection v1 polling does not itself send cancellation; expiry/shutdown owns bounded cleanup. Committing/RecoveryRequired remain endpoint-free and unavailable under unchanged v1.

The trusted client fetches private instructions only through the exact numeric loopback capability endpoint, without proxy/redirect exposure, and checks its bounded no-store reply and configured authorization origin. It polls the original session deadline without repeating Create. New OAuth errors are projected into existing closed value-free variants; generic/raw client contracts remain unchanged. Success requires the original session, integration, deadline, expected Connection, credential purpose and Callable description. The expected reference is a private local response check derived through the backend's existing admitted identity helper, not a new wire selector, binding authority or entity. The returned daemon ConnectionDescription remains unchanged.

Console selects the configured profile and derives its public label from trusted local input/config. Private instructions go to the controlling terminal or a newly reserved explicit private file; headless use without that destination refuses before Create. New private files require owner-only parents and exclusive 0600 creation, are pinned against replacement and erased before public success or on drop/expiry. Ordinary typed Connection output removes completion/browser endpoints. The new CLI options are `--auth-profile` and `--instruction-file`; unrelated defaults/help, raw behavior and one-shot assertions remain intact. The exact scoped example is `crates/connectors-config/examples/gitlab-personal-oauth.example.toml`.

## Executed checks

Every command has its exact cwd/manifest/argv, raw log, actual exit and resource observations in scratch. `stage2c-operational-commands.json` indexes them. The total below uses each package's final applicable complete execution once; repeated deciding or verification runs are not added.

| Packages | Passing cases | Final applicable evidence |
| --- | ---: | --- |
| connector-spec, connector-resolve, catalog, catalog-reader | 645, plus 1 old ignored measurement | Sealed root evidence: 544 + 59 + 25 + 17; final hash audit confirms those four package sources/artifacts unchanged. |
| catalog-build | 148 | `stage2c-root-client-regression-tests-2`: full current schema-4 rate/URI vectors included. |
| connectors-client | 39 | `stage2c-client-full-tests-final-1`: 37 unit + 2 compile-fail doctests after the final formatting-borrow correction. |
| protocol, server, service | 231 | `stage2c-root-client-regression-tests-2`: 64 + 103 + 64. That command totals 418 including build/client above. |
| connectors-config, connect-session-transport, integration-catalog, connectors-runtime | 202 | `stage2c-runtime-affected-tests-final-2`: 26 + 25 + 89 + 62. |
| connectors-console | 99 | `stage2c-console-full-tests-final-1`: ordinary workspace suite, including successful-daemon-label privacy. |
| connectors-cli | 132 | `stage2c-cli-full-tests-3`: ordinary default-feature workspace suite including SIP and all original one-shot tests. |

Strict all-target Clippy final applicable results are the sealed root five-package checks, `stage2c-root-client-clippy-final-2`, `stage2c-runtime-clippy-final-1`, `stage2c-console-clippy-final-1` and `stage2c-cli-clippy-final-2`. Final owning-workspace formatting checks are `stage2c-{root,runtime,console,cli}-fmt-final-2`. Expected compile-fail doctests are passing privacy checks, not compiler failures in this report.

## Deciding failures and corrections retained

Earlier root/runtime reports retain every first observation: spec 2 pass/2 fail; reader 0/1 against unchanged production; first root full run 782/9 plus the old ignore; config 1/2; runtime composition 2/1; transport liveness 1/2. Two extracted-module compiler failures and two strict-lint failures are explicitly separate from product-red counts. Those immutable reports contain the exact corrections and full runtime recovery fixtures.

New client/console/CLI witnesses are retained under these prefixes:

| Prefix after `stage2c-` | Actual deciding result |
| --- | --- |
| `client-handoff-red-1` | 0 pass, 1 fail: explicit trusted handoff absent at the added closed seam. |
| `client-private-error-red-2` | 0 pass, 1 fail: arbitrary daemon text escaped the new trusted boundary. |
| `console-private-flow-red-1` | 0 pass, 1 fail: explicit private instruction flow absent at the added closed seam. |
| `console-endpoint-red-1` | 0 pass, 1 fail: ordinary successful envelope leaked the completion endpoint. |
| `console-doctor-red-1` | 1 pass, 1 fail: ordinary diagnostic preserved, required OAuth custody/redirect facts absent. |
| `cli-profile-red-1` | 0 pass, 1 fail against unchanged parser production. |
| `client-success-red-1` | 1 pass, 8 fail: well-formed correlated Created/Degraded, wrong session/integration/deadline/Describe target/purpose and consistently repeated private-target responses were accepted. |
| `console-success-label-red-1` | 0 pass, 1 fail: a private marker in a successful daemon label reached the public summary. |

The success correction was prompted by coordinator implementation inspection, not a formal independent adversary pass. Actual deciding fixtures preceded it. The two successful console fixtures now derive their synthetic daemon reference through the same owning helper, preserving their old privacy/success assertions and adding the trusted local label assertion. Preimages and exact failures remain in `stage2c-success-red-source/` and the raw logs.

Additional non-product-red evidence is preserved. `client-private-error-red-1` refused to start at the disk floor (exit96); no compiler ran. `root-client-regression-tests-1` retained 404 passes/5 failures caused by the existing service fixture's schema3 literal. The coordinator assigned exactly that cfg(test) helper's 3-to-4 correction; production and assertions are byte-exact. `root-client-clippy-final-1` reported two redundant formatting borrows in the new human writer; their removal added no suppression, and fresh strict/full client checks pass.

`console-full-tests-1` retained two new private-file fixture failures and a manually interrupted synthetic server wait (actual exit -2, not a resource interruption). The deciding mode observation measured the fixture directory as 0755 rather than 0700. Only new fixture directories now set 0700 explicitly; production refusal is unchanged. The synthetic server uses a bounded response acknowledgement before join. The first source/log/interruption record remains intact. The first strict CLI check reported only the new options pushing SetupCommand over the enum-size threshold. Only those two new fields moved into a boxed, flattened clap::Args structure; all existing option fields and tests remain in place, with no suppression. The exact source is retained in `stage2c-cli-before-args-box.rs`; fresh complete CLI and strict checks follow that correction. Console formatting first exposed a new client module's edition formatting; the final source is checked by all four owning workspaces.

A cached example candidate briefly created the new example under the wrong root-relative directory. Scope inspection immediately corrected it to the assigned config-crate examples path and removed the new empty directory. No existing file was overwritten. `stage2c-example-path-correction.json` preserves that observation; the final source audit proves the wrong path absent.

## Preservation and dependency closure

`stage2c-operational-preservation.log` freshly verifies 106 sealed root evidence hashes, 167 runtime hashes, the 14-entry target-handoff seal and all 7 success-red preimages. Original custody/session-refresh cases and ten rate adversary files are byte-exact. The two remaining rate-owned fixture files retain the separately assigned narrow schema identity/active-schema exceptions, with original historical bytes and equivalent current-v4 assertions retained. The integration-catalog entry helper has only `oauth: None`; the existing transport test prefix is exact. Connection v1 and Operation v1/v2 contracts and frozen catalog schemas 2/3 are unchanged. All sources stay within their recorded owners; no independent-review test file is edited.

All twelve `--locked --offline` metadata graphs pass: root350, runtime541, CLI600, console343, audio44, speech162, CDP203, SIP371, SQL187, RTVBP298, local-audio37 and voice-runtime372 package rows. Final audit confirms every manifest and recorded lock hash remains unchanged since those commands. Only runtime/CLI/console locks differ from tree HEAD: integration-catalog adds the existing local `connector-oauth` and `connect-session-transport` edges, and console gains those two local package rows. Existing package versions/checksums are unchanged; nine other locks are byte-exact. The already integrated GitLab resolver/jsonschema graph is inherited, not introduced by this OAuth stage. Exact metadata rows and outputs are retained.

## Resources, freeze and limits

The original root lane used its ordinary target and sccache, then sealed/cleaned that exact target. The prospectively assigned private tmpfs target was used serially for runtime, root/client and console with the wrapper unset. The separate prospective CLI assignment used only this tree's ordinary CLI target and `/usr/bin/sccache`, with CARGO_TARGET_DIR unset. Every compilation used jobs1, incremental0/debug0 and `/home/timo/.cache/cw6/a` for TMPDIR. Guards continuously checked disk above12GiB, tmpfs free at least8GiB, MemAvailable at least16GiB and each assigned target at most8GiB; no running command was interrupted by a resource guard. The one no-start refusal and one explicit fixture-wait interruption remain distinct.

The final source inventory contains 1,188 files (116 tracked changes and 11 new files), and the command index contains 69 exact command records. Final target inventory: private 8,403,660,800 allocated bytes, 16,981 files/1183 ELF hashes; CLI 2,782,216,192 bytes, 7,697 files/545 ELF hashes. Every serial command records its warm target inventory and exact workspace. Final target inventories/ELF hashes are historical evidence; both targets remain in place for coordinator disposition. Per-path allocated-byte inventories conservatively count aliases separately and are not silently replaced with `du` figures. Ordinary root/runtime targets remain absent. No other tree/cache was cleaned. Outside writes are only assigned persistent scratch/candidates, original TMPDIR transients, the exact authorized targets and ordinary tool-managed Cargo/sccache bookkeeping. Repository product code remains Rust; scratch orchestration is not shipped code.

`stage2c-operational-source.sha256`, the binary-capable patch, copied untracked files, source status/HEAD and final target inventories identify this handoff. `stage2c-operational-evidence.sha256` seals reports, source snapshots, command/exit/resource records, preimages and old immutable evidence; mutable proposed candidates and this drafting file are excluded. Earlier reports are not rewritten to imply they tested later changes.

The remaining work belongs to the coordinator: inspect/commit the source, run the whole-unit independent review and full combined gate, and complete publication ordering. Schema3 source/CI/documentation publication precedes schema4 writer publication. No future Design22 Operation v3/Connection v2 behavior, live GitLab registration or production sealed-custody support is claimed here.

The portable copy replaces only the original absolute home-directory prefix with a tilde. Raw and portable reports have separate hashes; source, logs and test results are unchanged.
