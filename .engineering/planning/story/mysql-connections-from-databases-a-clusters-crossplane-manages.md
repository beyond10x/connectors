---
format: aep.planning-md/3
id: story:mysql-connections-from-databases-a-clusters-crossplane-manages
kind: story
status: draft
title: A mysql connection is made from a database a cluster's Crossplane manages, its password read from the cluster
summary: The kubernetes connector discovers Crossplane-managed databases, users and grants; a mysql connection takes its password from the cluster Secret and defaults to the read replica
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
revision: 1
---
## Need

The operator's request of 2026-11-26: a MySQL read connection to a managed database that a
Kubernetes cluster provisions through Crossplane should be set up by pointing connectors at the
cluster. The kubernetes connector finds the databases and their users there, and the mysql
connection takes its password from the cluster's Secret itself, with the database's read
replica as the default endpoint.

How it is done today, on 0.78.x, for one such connection (the same day):

- The kubernetes connector does not help. Its custom collections can list Crossplane's own
  resources (a `group` with a dot, `docs/local-kubernetes-cli.md:470-480`), but no setting adds
  a Secret read (`docs/local-kubernetes-cli.md:62`, :477), and the password of a Crossplane SQL
  user is in a Secret of type `connection.crossplane.io/v1alpha1` with the members `endpoint`,
  `port`, `username` and `password`.
- So the person setting it up read the users and their grants (`grants.mysql.sql.crossplane.io`)
  and the cluster's reader endpoint (the `DBCluster` status) with `kubectl`, copied the password
  from the Secret into an owner-only file, ran `connections connect --credential-file`, and
  removed the file.
- The Secret's `endpoint` is the writer endpoint. The reader endpoint is only in the status of
  the cluster resource, so a connection built from the Secret alone reaches the writer.
- Which user may only read is read from the grant resources (database and privileges), not
  from the user's name.
- The release archive holds no `connectors-sql`
  (story:released-install-reads-mysql-and-loki-per-environment, finding 1), so the server was
  built from source.

## What the operator asked for

1. Discovery: an operation of the kubernetes connector that lists the databases Crossplane
   manages in a namespace, each with its engine, its writer and reader endpoints, and its
   users with their grants (database and privileges). No password and no Secret value is in the
   answer.
2. A mysql connection whose credential is taken from the cluster Secret by connectors, with no
   file step and without the value passing through the caller. Re-reading the Secret is how a
   rotated password is picked up.
3. The read replica is the default endpoint. The writer endpoint is not used unless a separate,
   explicit choice says so.

## Open questions (to settle before this story is proposed)

- The new nouns (a Crossplane-managed database, its users and grants, a connection whose
  credential source is a cluster Secret) have no ESS document yet. Under the planning rules the
  domain is drafted first; this draft names no entity.
- Where the Secret read lives: in the kubernetes connector (a narrow read of one Secret type,
  named by the discovered user, never a general Secret read), or in connectors' custody as a new
  credential source of the mysql connection. Either changes a stated boundary of the kubernetes
  guide.
- Whether a connection may be made for a user whose grants are not `SELECT` only, and who
  decides that: the guide already says the user's grants are the boundary for routines and
  databases (`docs/local-mysql-cli.md`, "Query bounds and the read-only session").
- The cluster credential: the kubernetes connector reads with a kubeconfig user; a cloud
  provider's sign-in behind it (an SSO session) expires, and the connection's own password
  outlives it.
- PostgreSQL databases provisioned the same way (the same Crossplane provider serves both).

## Not in this draft

- A design or an acceptance. Nothing is built.
- The release archive's missing `connectors-sql`, which is
  story:released-install-reads-mysql-and-loki-per-environment.
