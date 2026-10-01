---
format: aep.planning-md/3
id: review-result:adversary-cli-spec-mapping-fixes-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: cli-spec-mapping-fixes'
relations:
- reviews: story:cli-spec-mapping-fixes
revision: 1
---
unit: story:cli-spec-mapping-fixes, worktree wave1001b-spec at 6aa797a37 plus one untracked test file
verdict: NEEDS-CHANGE
cases: executed 121→126, red 4
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths (p2/, p2/cliref, p2/m1, p2/m2, p2/suite.log under wave-20261001b/spec/scratch)
needs-coordinator: whether the owner frames should model the `kind` tag on all three frames or on none

Cases (crates/connectors-build/tests/cli_spec_mapping_adversary_pass2.rs), run alone: `test result: FAILED. 1 passed; 4 failed`.
- the_owner_hello_types_its_uuid_fields: red — connectors.cli.LocalOwnerHello.challenge is `String`, but the owner refuses any value that is not a UUID
- the_owner_greeting_types_its_uuid_fields: red — connectors.cli.LocalOwnerGreeting.challenge is `String`, but the host produces and checks a UUID
- the_owner_frame_tag_is_modelled_the_same_way_on_every_frame: red — only ["connectors.cli.LocalOwnerBuildRequest"] declare the `kind` tag; the other owner frames carry it on the wire too
- the_build_digest_comment_cites_the_file_that_encodes_it: red — cli.yaml says owner.rs writes the build digest with hex::encode; it does not (own_build in local/owner/transport.rs does)
- the_build_digest_admits_exactly_lowercase_64_hex: green (control)

Suite: `cargo test --locked -p connectors-build --no-fail-fast` EXIT=101, 122 passed, 4 failed. cli --check exit 0; metadata-entities --check exit 0.

Not broken: LocalOwnerHello has exactly Request::Hello's five fields; LocalOwnerBuildRequest matches {"kind":"build"} (transport.rs:261); BuildDigest refuses uppercase, 63, 65, empty, non-hex; invariant kept in the resolved model; CLI contract byte-identical; no non-cli domain references connectors.cli (PR #69 closure on f905839ad).

```findings
- file: ess/domains/cli.yaml
  line: 159
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: LocalOwnerHello.challenge and .authority are any String although the owner refuses a non-UUID challenge and compares a rendered Uuid authority, and the pinned ESS accepts Uuid on them
- file: ess/domains/cli.yaml
  line: 167
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: LocalOwnerGreeting.challenge, host_incarnation and authority are String although the CLI parses host_incarnation as a UUID and the reply echoes UUID challenge and authority
- file: ess/domains/cli.yaml
  line: 178
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: only LocalOwnerBuildRequest models the serde kind tag, so the spec says the hello frames carry no kind while the build frame does, though all owner frames are kind-tagged
- file: ess/domains/cli.yaml
  line: 146
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the BuildDigest comment cites owner.rs as the hex::encode writer, but the digest is produced by own_build in local/owner/transport.rs:33-39
```
