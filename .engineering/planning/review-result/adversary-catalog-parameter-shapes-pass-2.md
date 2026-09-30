---
format: aep.planning-md/3
id: review-result:adversary-catalog-parameter-shapes-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the engine lane
relations:
- reviews: story:catalog-repeated-query-parameters
- reviews: story:catalog-selection-required-parameters
- reviews: story:catalog-engine-provider-refusal-shapes
revision: 1
---
unit: engine lane `impl/catalog-query-parameter-shapes`, correction `2e14b1e18...86266164c` (base 81b509745), working tree `~/.local/state/worktree/trees/b10x/connectors/wave0930c-engine` at 86266164c
verdict: NEEDS-CHANGE
cases: executed 242→243, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths under `~/.cache/w0930en/adv2/` (listed in part 6)
needs-coordinator: none

**1. Diff of the tree.** `git --no-pager diff --stat` is empty. `git status --short` shows one new file, and it is a test file: `?? adapters/catalog/tests/engine_adversary_pass2.rs`. No implementation file was changed.

**2. Case added** in `adapters/catalog/tests/engine_adversary_pass2.rs`. It is red now.

`adversary2_an_instance_connected_before_the_rebuild_connects_again_after_it`:
- It prints the bootstrap for the guide's own GitLab site-form configuration twice: once over the base bundle rebuilt from the current one (its sha is checked against `da458f3b…`, the base index value), and once over the current bundles.
- It connects the instance under the old binding with `Registry::begin` and `consume`, then connects again under the new binding.
- It asserts the reconnect succeeds, because the guide names write approval policies as the only consequence of the rebuild.

The first run of the case alone was red:
```
panicked at adapters/catalog/tests/engine_adversary_pass2.rs:140:5:
instance `gitlab-sandbox` connected under configuration revision b7745324b4d2bcb5717d37823e27653e9bc9d9d44cc6fbd4fceae703d2cd649d is refused (Some(Conflict)) under f87354a4d17235768ef1745c5693296658c66e7fedd179fbe00aac2a58b5bb80 after the bundle rebuild; the guide names only write approval policies as what the rebuild requires
test result: FAILED. 0 passed; 1 failed
```
`rustfmt --check` on the file and `cargo fmt --package connectors-catalog-provider -- --check` both exit 0. `cargo clippy -p connectors-catalog-provider --test engine_adversary_pass2 -- -D warnings` also exits 0.

**3. Suite run.** `cargo test -p connectors-catalog -p connectors-catalog-provider --no-fail-fast` exited 101.
- The only failing target is `engine_adversary_pass2` (`0 passed; 1 failed`).
- Every other binary passed: `engine` 22, `engine_adversary` 4, `gateway_prefix` 7, `bundle_drift` 1.
- In total 243 cases ran; 242 of them are outside my file, which is where the "before" count comes from.

**4. Findings** (they cover tree 86266164c)

| # | file:line | measured | what reaches it | verdict / origin |
|---|---|---|---|---|
| 1 | `docs/local-catalog-provider.md:235-240` | The rewritten paragraph says the rebuild's only consequence is that write approval policies must be issued again. The actual consequence is larger: an instance id stays bound to its first configuration revision (`crates/connectors-host/src/local/registry/lifecycle.rs:288`). The connection binding comes from the bootstrap (`runtime.rs:308`). The rebuild moves the configuration revision from `b7745324…` to `f87354a4…`. The case above is red. Two more effects follow: `runtime/process.rs:129` refuses to start the adapter until `configuration_revision` is copied again, and a repair with the new binding returns `IdentityMismatch` (`lifecycle.rs:52`). For Jira and Confluence, which ship no writes, the guide names no consequence at all. Approval policies are invalidated by any binary upgrade anyway, because `executable_selection` includes the executable (`config.rs:131`). | Every existing GitLab, Jira or Confluence instance that follows the guide's upgrade. The registry mechanism is pre-existing ("configuration upgrades … need their own implementation", `docs/local-connection-registry.md:121`). The paragraph is not at base (`git show 81b509745:docs/local-catalog-provider.md`). | NEEDS-CHANGE / introduced |
| 2 | `adapters/catalog/src/lib.rs:804-811` | The new `integer_text` was inserted between `declared_type`'s doc comment and `declared_type` itself. `integer_text`'s rustdoc now opens with "The input schema of one parameter…", and `declared_type` has no documentation. | Readers of the source only | CONFIRMED / introduced |

The fix for 1 is a doc change: say that the configuration revision moves, the adapter entry's `configuration_revision` must be copied again, and an instance that is already connected must connect under a new `instance` id (or fresh state).

**5. Attacked and could not break.** I ran a temporary probe of 47 inputs × 4 element types × {optional, required}, comparing the declared schema's verdict with the engine's. The probe file was deleted afterwards; its log is `probe.log`.
- **Comma lists:** the schema and the engine agree on negatives, `-0`, leading zeros, whitespace, empty elements, a leading or trailing comma, `""`, `-`, `+1`, a trailing `\n`, Unicode digits, booleans, `True`, and `[" 1"]`.
- **Where they disagree:** only on JSON `2.0`, `[2.0]` and `18446744073709551616`. The schema accepts these and the engine refuses them. This is pre-existing and documented in `declared_type`'s comment.
- **Overflow beyond i64:** strings of any length (`"99999999999999999999999"`) are accepted and sent as given. That matches the non-repeated integer rule, which is unchanged since base.
- **String elements given JSON integers:** sent as their decimal text.
- **minItems:** `[]` is refused by both the schema and the engine for all four types when the parameter is required, and sends nothing when it is optional. `minItems` also applies to parameters made required by a selection (`lib.rs:312-319` marks them before `declare`).
- **Bounded repeated parameters:** the pattern still admits `"1,2"`, which the engine refuses. That falls under the existing "Advisory for callers" bound comment, and no shipped selection bounds a repeated parameter.
- **gateway_prefix re-pins:** my independent base reconstruction gives the old pinned revision `b7745324…` and the new `f87354a4…`. The correction moved only the bootstrap digest, not the revision, which is consistent with the schema-only change.
- **Doc claims:** the Confluence `id` and `space-id` elements are integers in the bundle (`"type":"integer","repeated":true`).

**6. Paths written outside the worktree:**
- `~/.cache/w0930en/adv2/probe.log`
- `~/.cache/w0930en/adv2/suite.log`
- `~/.cache/w0930en/adv2/pass2-final.rs` (a copy of the case file, already deleted)

The `tempfile` directories under that TMPDIR removed themselves. Build output went to `<worktree>/target`. My lease `wave0930c-engine-adv2` is released.

```findings
- file: docs/local-catalog-provider.md
  line: 239
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The guide names re-issued write approval policies as the rebuild's only consequence, but the moved configuration revision stops the adapter until it is re-copied and the registry refuses the instance id's reconnect as Conflict."
- file: adapters/catalog/src/lib.rs
  line: 804
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "integer_text was inserted under declared_type's doc comment, so it carries that prose and declared_type is undocumented."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
