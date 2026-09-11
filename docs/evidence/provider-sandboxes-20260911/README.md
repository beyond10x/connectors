# Kubernetes and PostgreSQL against real servers

The two provider journeys had only fixture evidence. Both now run against a real
server, and the Kubernetes run found a defect no fixture had.

## Servers

| provider | server | scope |
|---|---|---|
| Kubernetes | k3s `v1.31.5+k3s1` through k3d, API on loopback `:56443` | ServiceAccount `fixture:reader`, a Role granting `get,list` on pods, services and endpointslices in one namespace |
| PostgreSQL | `postgres:17`, server `17.11`, loopback `:55432` | role `reader` with `SELECT` on one table, no other grant |

Both are disposable and were created for this run. Credentials are fictional and
belong to the sandboxes only. Recorded in [versions.txt](versions.txt).

## The defect a real cluster found

`endpoints.discover` failed the **whole page** with `upstream_protocol` whenever
the namespace held a Service with no ready backends. Kubernetes serialises such
an EndpointSlice with `"endpoints": null` and `"ports": null` rather than empty
arrays, and the adapter called `as_array()` on that and treated it as malformed.
One backend-less Service therefore broke discovery for every other Service
beside it.

Null now reads as no observations from that slice, which is what Kubernetes
means. Any other non-array value remains a protocol violation. The regression
case feeds the exact shape the cluster produced and was mutation-checked: it
fails when the null arm is removed and passes when it is restored.

No fixture had this shape, because every fixture slice was written with a
backend in it.

## What the live journeys establish

Both run the production CLI against a task-owned dbus and GNOME Keyring, with
the credential file deleted before the post-restart read.

| step | Kubernetes | PostgreSQL |
|---|---|---|
| rejected credential | `service_failure`, `service_code: unauthorized` | same, from PostgreSQL `28P01` |
| connect | `ready`, identity from a real SelfSubjectReview | `ready`, identity `reader@incidents` |
| read | `resources.list` returns real pods, `complete: true` | parameterized `WHERE severity = $1` returns exactly the 2 matching rows |
| refusal | namespace outside configured scope is `forbidden` before any request | `INSERT` refused by the read-only transaction, not a keyword filter |
| restart | CLI, owner and keyring restarted, credential file deleted, read repeats on the saved version at the same connection revision | same |
| second operation | `endpoints.discover` on the saved credential | `schema.list` on the saved credential |

`owner::Code` carries no `invalid_credential`, so a rejected credential is
`service_failure` with the provider's own code preserved beside it. That is the
same envelope for both providers, and an earlier draft of these tests asserted
the wrong one.

## Gate

ESS 0.22.2 at `6b666e58f2e8`, AEP 0.55.0 at `4eb999e0` with patch `73d78e50`.

| step | exit |
|---|---:|
| 2 `ess-boundary` | 0 |
| 3 `cli --check` | 0 |
| 4 `fmt --check` | 0 |
| 5 descriptor drift ×3 | 0 |
| 6 `build --workspace` | 0 |
| 7 `test --workspace` — 423 passed, 0 failed, 35 ignored | 0 |
| 8 `clippy -D warnings` | 0 |
| 9 adapter boundary ×3 | 0 |
| 10 generic CLI boundary | 0 |
| 11 `+1.88.0 check` | 0 |
| 12 `ess verify conform synthesize` — 315 scenarios, 22 refusals | 0 |
| 13 `aep plan artifact validate` | 0 |

Live runs: Kubernetes 11 passed, 0 failed; PostgreSQL 5 passed, 0 failed.

## What is still not established

TLS to PostgreSQL. The sandbox runs with `allow_plaintext` on loopback, so the
CA path and TLS negotiation are exercised for Kubernetes only.

Helm remains unimplemented and `decision-blocker:helm-execution-family` is open.
Neither sandbox is a production system, and neither run says anything about
scale, concurrency or a provider under load.
