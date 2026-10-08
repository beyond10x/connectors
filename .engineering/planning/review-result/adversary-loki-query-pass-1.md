---
format: aep.planning-md/3
id: review-result:adversary-loki-query-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Loki bearer connections and invoke
relations:
- reviews: story:parity-loki-query
revision: 1
---
## Report

unit: story:parity-loki-query, worktree conn-u-loki-e at a6733cfdf plus one untracked test file
verdict: NEEDS-CHANGE (red: 2 cases fail; the ESS model and the executable disagree)
cases: executed 27→33, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (scratch adv1/schemas, adv1/tmp)
needs-coordinator: none

Cases added in `adapters/loki/tests/adversary_pass1.rs`:

| Case | Result |
|---|---|
| `the_configuration_model_and_the_executable_agree_off_the_unit_list` | red |
| `the_entry_model_and_the_entry_parser_agree` | red |
| `a_window_of_exactly_twenty_four_hours_is_admitted_and_one_nanosecond_more_is_not` | green |
| `eleven_thousand_points_dispatch_and_eleven_thousand_and_one_do_not` | green |
| `a_server_outside_the_configured_ca_never_receives_the_token` | green |
| `a_redirect_is_never_followed_with_the_token` | green |

Red output, verbatim:

```
---- the_entry_model_and_the_entry_parser_agree stdout ----
panicked at adapters/loki/tests/adversary_pass1.rs:211:5:
[
    "{\"token\":\"fixture token\"}: model admits true, parser admits false",
    "{\"token\":\"fixture\\ttoken\"}: model admits true, parser admits false",
    "{\"token\":\"fïxture\"}: model admits true, parser admits false",
]
---- the_configuration_model_and_the_executable_agree_off_the_unit_list stdout ----
panicked at adapters/loki/tests/adversary_pass1.rs:189:5:
[
    "{...\"instance\":\"loki prod\"...}: model admits true, executable admits false",
    "{\"base_url\":\"HTTPS://loki.example/\",...}: model admits false, executable admits true",
    "{...\"ca_file\":null,...}: model admits false, executable admits true",
]
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
```

Attacked and not broken: redirects never followed (`http.rs:133`); CA mismatch sends no request or token; plain HTTP refused (`local.rs:64`); 24h window, 1,000-line and 11,000-point caps hold at their edges; no token or provider text in errors; `crates/connectors-host` untouched.

Judgement notes (not findings): the identity probe accepts any 200 body (decided design); unverified guess that an empty label answer without `data` would read as Unavailable (`lib.rs:620`).

```findings
- file: adapters/loki/spec/ess/domains/connection.yaml
  line: 31
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: LocalConfiguration and the executable disagree both ways (instance charset admitted by the model, uppercase https scheme and null ca_file admitted by the executable), outside ESS-LIMIT, falsifying ess_model.rs's "exactly" claim
- file: adapters/loki/spec/ess/domains/connection.yaml
  line: 43
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: BearerEntry bounds only length while §11.4 and auth.rs:45 require visible ASCII without spaces, so the model admits tokens the parser refuses
```
