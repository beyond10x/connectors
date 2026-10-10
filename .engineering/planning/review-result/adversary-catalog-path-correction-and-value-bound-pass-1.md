---
format: aep.planning-md/3
id: review-result:adversary-catalog-path-correction-and-value-bound-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: catalog-path-correction-and-value-bound'
relations:
- reviews: story:catalog-path-correction-and-value-bound
revision: 1
---
unit: story:catalog-path-correction-and-value-bound at e223d8ff9 (wave/20261010c)
verdict: 2 findings, both unreachable from the committed amendment and selections; both fixed
cases: executed 969→971, red 2 (then green after the fixes)
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: none

## Findings

1. warning, contract drift, `crates/connectors-catalog/src/amendment.rs` (`resolves`): a path
   correction could remove a parenthesised group holding path parameters. On the pinned GitLab
   NuGet route `.../Packages(Id='{package_name}',Version='{package_version}')` a correction to
   `.../nuget/v2/Packages` was admitted, dropping two path parameters and retargeting the
   operation, against the model's promise (`amendment.yaml`) that a correction keeps every path
   parameter. Test: `crates/connectors-catalog/tests/path_correction_adversary.rs`,
   `adversary_a_path_correction_may_not_drop_a_group_holding_path_parameters`. Fix: a group
   holding a path parameter may only be kept, never removed; model comment, guide and refusal
   text say so.
2. note, boundary, `adapters/catalog/src/lib.rs` (`values_enum`): the declared `enum` added the
   integer form of an allowed value only for `i64` text, so a JSON integer above `i64::MAX` whose
   text is allowed was refused by the schema though the bound check admits it. Test:
   `adapters/catalog/tests/selection_values_bound_adversary.rs`,
   `adversary_an_allowed_integer_text_above_i64_is_admitted_as_its_json_integer`. Fix: also
   parse as `u64`.

## Attacked without a finding

- Values outside the set (`07`, `7.0`, `+7`, `" 7"`, case variants, arrays, `"blobs,issues"`, a
  repeated parameter with one bad element): refused; the bound compares the exact sent text after
  schema validation.
- Overriding `scope`: code search takes no body or header parameter, the input schema is closed,
  the withheld `type`, `state`, `confidential` and `fields` are refused, and the transport
  percent-encodes path segments and query pairs, so no second `scope` key can be added.
- Path corrections: nested or unbalanced parentheses, a stale `from`, the wrong operation,
  `to == from`, both changes in one amendment and two corrections of one operation are refused;
  the method cannot change; the bundle drift test applies the GitLab amendment file.
- Re-pinned hashes are not weakened: the engine adversary files still assert the base bundle
  hash; the commit-reads adversary asserts its rebuilt bundle equals the index hash before the
  correction; only the gateway-prefix documentation example pair moved, with the bundle hash.

## Gates after the fixes

`cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo test -p <crate>` on
connectors-catalog and connectors-catalog-provider: 736 passed, 0 failed; `cargo fmt --check`
clean.
