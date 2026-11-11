---
format: aep.planning-md/3
id: review-result:wave-b-config-change-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the configuration change error code
relations:
- reviews: story:configuration-change-error
revision: 1
---
## Adversary pass 1 on story:configuration-change-error

Reviewed commit 79d638116f on impl/configuration-change-error (base 2e839b426). Model: opus.

```
verdict: green
cases: executed 39→39, red 0
origin: introduced 4, pre-existing 2, undecided 2
```

No test cases added: every finding is a documentation gap or a scope question. Runs:
`connectors --bin connectors -- local::tests` 13 passed, `vocabulary_text` 4 passed,
`connectors-host --lib -- registry::upgrade_tests owner::tests` 22 passed.

Held: both mappers (`owner::Error::from` and the CLI `registry_failure`) send only
`AuthenticationChanged` to the new code, and `BindingChanged` stays `lifecycle_conflict` /
`create_connection`. Connect, reconnect, revalidate and invoke answer it at stage `admission` with
exit 1. All 37 `FailureCode` enums in `binding.json` carry it. No other exhaustive code list or
string classifier can receive it. The vocabulary count of 68 is correct.

```findings
[
{"file":"docs/local-connection-registry.md","line":110,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The change table still answers a provider-authority or profile-authentication change as lifecycle_conflict, which the unit changed to configuration_changed."},
{"file":"contracts/cli/v1alpha1/semantics.md","line":1671,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The new row lists consumer launch, but admit_launch binds the recorded authority and profile, so launch answers lifecycle_conflict/revalidate_connection and never configuration_changed."},
{"file":"apps/connectors/src/local/endpoint.rs","line":424,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"undecided","message":"A short-form connect whose new endpoint moves a connected instance's authority, the same only-a-new-instance case, still answers lifecycle_conflict naming endpoint."},
{"file":"apps/connectors/src/local.rs","line":377,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"The next action stays the generic check_configuration and no envelope member names a new instance id; only the docs do, against Acceptance line 1 read literally."},
{"file":"ess/domains/cli.yaml","line":1020,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"PendingReason configuration_changed (any revision change, revalidate usually helps) now shares its name with the failure code meaning only a new instance id helps."},
{"file":"crates/connectors-host/src/local/registry.rs","line":101,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The AuthenticationChanged doc comment, and definition.rs:1060, still name lifecycle_conflict as its public code."},
{"file":"apps/connectors/src/mcp/projection.rs","line":46,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"MCP projects configuration_changed to unavailable, which invites the retry the new code exists to prevent."},
{"file":"contracts/cli/v1alpha1/scenarios.md","line":30,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"Approval key and policy commands under an authentication change still name create_connection, a connect that then answers configuration_changed."}
]
```
