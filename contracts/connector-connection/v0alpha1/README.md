# ConnectorConnection v0alpha1

`b10x.connector-connection.v0alpha1` is the value-free control contract for durable
Connections and short-lived Connect Sessions. It deliberately has no field capable of carrying a
vendor credential, credential address, provider URL, or transport ticket.

`connect_session_create` returns a pending session with a short-lived `completion_endpoint`. That
endpoint belongs to Connectors, accepts one credential-acquisition completion, and disappears. It
is not the durable Connection and it is not the harness's reusable Endpoint. The creator polls
`connect_session_status`; only a completed terminal status names `connection_ref`, and it names
nothing else from acquisition.

`search` and `describe` return non-secret Connection lifecycle, initiation, route, and Channel summaries.
Initiation answers which side may start; it does not replace a Connector Grant or an Agent endpoint
grant. Local transport identity and the request's exact owner context remain independent gates.

`candidate_search` is the pre-Connection discovery phase for trusted local configuration. It is
passive: no provider request, credential helper, or login may run. Results expose an opaque
`candidate_ref`, Integration, human label, lifecycle, evidence digest, and optional activated
Connection only. `candidate_activate` is the explicit boundary that permits the Connector to
resolve the candidate's private credential source, verify the provider identity and authority, and
create a direct Connection. The caller cannot submit a kubeconfig path, server URL, user binding,
credential helper, token, or resulting Connection identity.

A route is either `direct` or `via_connection`. The mediated form names only the parent Connection
and one closed route-adapter identity. The Connector-owned discovered-resource binding, provider
URL, Grafana data-source UID, proxy path, and credentials never cross this contract. The child is a
mediated Connection governed by the target Provider contract and requires its own Connector Grant;
neither semantics nor authority are inherited from the parent. “Target Provider” describes the API
contract (for example Prometheus or Loki), not a direct transport or another credential. Its route
may remain entirely through Grafana.

`observation_search` reads bounded, already-reconciled observations for one source Connection. It
does no provider I/O and returns only normalized type/title, lifecycle, evidence generation and
digest, an optional target Provider contract, and an optional materialized child Connection. It never
returns the Connector-owned resource binding. An active refresh remains invocation of the
Provider-declared discovery read under normal operation admission.

`materialize` is the explicit control-plane transition from one recognized observation to one
mediated Connection. Its request carries only `observation_ref`. The Connector resolves the stored
target Provider, resource binding, parent Connection and closed route adapter; callers cannot
select or override any of them. Repeating the request for an already-materialized observation is
idempotent and returns the same Connection.

The personal-local binding multiplexes this contract with Operation and Event frames on the
owner-only Connector Unix socket. The completion endpoint is a separate single-use owner-only
socket and is intentionally outside this credential-free JSON schema.
