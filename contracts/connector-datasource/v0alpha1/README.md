# ConnectorDatasource v0alpha1

`b10x.connector-datasource.v0alpha1` is the credential-free contract for bounded discovery
and read access to Connector-owned datasources. Search grants nothing. A read is tied to an exact
definition, opaque binding, current description lease, owner authority snapshot, and closed
`list` or `get` verb.

Datasource owners publish JSON Schemas plus the SHA-256 identity of a
`b10x.value-projection.v1` declaration. Results retain binding, projection, and Connector
audit provenance after provider envelopes are removed. Cursors are opaque, bounded, expiring, and
scope-bound.

This contract has no credential, arbitrary query, raw-object, or write field. The first consumer,
`kubernetes.workloads`, uses it to exclude Secrets, environment values, labels, annotations, raw
Kubernetes objects, and event messages before data reaches Agent or Zwirn.
