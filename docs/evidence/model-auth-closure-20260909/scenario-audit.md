# Auth model closure scenario audit

Manual textual/model audit at base `8a5cf563fc717fd4b23e4dd7d470d0c967f39461`.
These rows inspect the selected contract and ESS lifecycle/command declarations;
they do not execute a host, callback, database, provider, clock or actual GC.

| Case | Required behavior and model disposition |
|---|---|
| AM01: instance identity | All new instance references use existing ServiceConfiguration.instance_id String. The isolated wrong-UUID fixture is refused with type_mismatch; existing eleven identities/lifecycles/field types compare unchanged. |
| AM02: two profile revisions | Same adapter/local profile with different declaration revisions produces distinct immutable profile_record_ref values. Logical profile_ref is preserved on Connection/capture. Compatible reviewed reference advancement applies current evidence/revision rules; a different logical profile needs a new binding. Exact qualification/equality is an owner predicate. |
| AM03: initial managed creation | Allocate a private, non-dispatchable Connection before Begin/custody/capture; Acquisition fixes target_connection_ref. CredentialGeneration can reference that existing private record without a public pending Connection or cyclic publication prerequisite. Completed result must equal the target. |
| AM04: static configuration | BeginAcquisition is refused for static_config. Independently admitted configuration activation may publish a configured capture; it invents neither callback correlation nor completed Acquisition. |
| AM05: duplicate completion | First acknowledged consumption moves Pending to Completing. A later consume has no transition from Completing/terminal states and cannot gain an exchange. Public observation is still pending. Removed consume causation is structurally refused by ESS. Actual durable exclusion and send counts require the binding. |
| AM06: unknown exchange | Consumed correlation remains consumed; owner records Failed/exchange_unknown, preserving repair target. A new observation of that Acquisition never grants another exchange. Unknown consume acknowledgement grants no original send. |
| AM07: completion versus revoke | CompleteAcquisition allow is selected only at the current shared binding fence with valid identity/baseline/custody. A revoked target refuses publication even if the Acquisition remains Completing. Live Connection alone is not readiness/permission. This cross-owner predicate is not executed by ESS. |
| AM08: completion acknowledgement lost | Resolve the same owner/attempt; never replace a possibly committed Completed outcome with Failed or perform another exchange. Unknown response is not proof of failed publication. Result ref is assigned exactly on acknowledged completion; schema command presence does not assign it. |
| AM09: expiry | At trustworthy now equal to original expires_at, no new exchange/publication; owner may move Pending/Completing to Expired. Both terminal and consumed safety facts remain non-reusable. Unknown time refuses progress. Public projection is failed/reason expired. |
| AM10: no-child-material modes | Anonymous/parent-authenticated child has absent active_generation, active custody and superseded custody, plus absent external identity. Missing configured bearer material cannot select either mode. Parent is independently admitted; parent generation evidence is optional only for explicitly credentialless direct parent. |
| AM11: configured capture without custody | An explicitly selected non-custody configuration may have a capture generation and no CustodyVersion. Managed publication requires the matching version and generation; it never silently adopts the configured alternative when custody fails. |
| AM12: scoped store-version collision | Same store_version in different scope_ref values yields distinct private version_ref identities. Tuple equality and material scope are checked at publication; String shape acceptance alone cannot establish them. |
| AM13: retired superseded material | Start 24-hour default retention at acknowledged supersession; retire only after trusted deadline and no valid use/recovery dependency. Host fences future publication/use before removing resolver links/physical deletion. Original capture and consumed-refresh safety records survive. |
| AM14: orphan with ambiguous publication | Unknown publication or possible stored-response recovery prevents definitive abandonment and cannot start orphan retirement. A confirmed never-published orphan starts retention at admitted abandonment; inactive private binding identity remains non-reusable. |
| AM15: unknown deletion and metadata capacity | Unknown physical deletion does not move Stored to Deleted or remove the retirement fence. Retain safe version/fence while erasing acknowledged-deleted material handle/bytes. Any full finite metadata/version store refuses new allocation, never deletes still-required material or resets safety identities. |
| AM16: route and declaration compatibility | Embedded mediated route comes from its discovery owner; no live observation foreign key pins history. AuthRequirement remains adapter-local profile/scopes value; adding an optional authored model field grants no support to current strict adapter/public codecs. |

The model deliberately permits shape-valid facts that a binding must reject:
wrong exact String qualification, mixed instance/profile/custody coordinates,
non-material mode with material fields, stale decision=allow, live-looking revoked
external authority, clock uncertainty, contradictory option presence and unassigned
result/retirement fields. These are specified owner predicates, not hidden claims
that the compiler enforces admission or persists command updates.
