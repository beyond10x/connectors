# sql implemented read profiles/v1alpha1

This adapter owns the native behavior of its existing local implementation.
Shared wire envelopes and admission remain in
[service v1alpha1](../../../../../contracts/service/v1alpha1/semantics.md).
This relocation changes no runtime or selected codec.

SQL: PostgreSQL schema discovery and parameterized, bounded single-statement reads
through a configured role/database. Database-enforced read-only transactions and
an adapter-owned execution deadline are mandatory. Each transaction installs
statement/lock timeouts as additional server-side guards. Caller-accessible
functions can change those session settings, including between portal executions;
they cannot extend the adapter's deadline. Schema scope is enforced by database grants,
not a string-prefix SQL filter. Preserve column type metadata, NULL, arrays/JSON
and numeric values without silently converting unsupported values into strings.
The first `postgresql-native-text` profile encodes non-NULL cells as PostgreSQL's
lossless text representation alongside each column's native type name; NULL is
JSON null. Numeric precision, arrays, timestamps and JSON are preserved as native
text rather than coerced to JavaScript numbers. Parameters are text or null with
server-side type resolution. This representation is explicit in the descriptor.
Report truncation explicitly; no invented resumable cursor for a fresh SQL query.
Each request uses a fresh connection and read-only transaction, initialized with
`search_path = public, pg_catalog`; qualify names in other schemas. Session settings
are not a substitute for the configured role's schema and function grants.
No connection pool is claimed in this local profile. A configured CA bundle
replaces public trust roots for SQL; empty bundles are refused.
Do not advertise writes, transactions across requests, or dialect portability.
Source facts: https://www.postgresql.org/docs/current/sql-set-transaction.html and
https://www.postgresql.org/docs/current/runtime-config-client.html.
Source access: 2026-09-08.

## Native deadlines and bounds

Within the shared 15 s provider budget (connect <=5 s), execution is at most 10 s
after connection, with up to 2 s reserved for cancellation/cleanup. Slow connection
reduces remaining execution time. Initial statement and lock timeouts are 10 s and
2 s. Native limits are 1,000 rows and 8 KiB query text; callers may request smaller
row limits, but configuration exposes no ceiling overrides.

An independent execution deadline triggers backend-keyed cancellation on failure,
deadline or dropped invocation, using the established endpoint and captured TLS
trust. The supervisor drives cleanup for at most 2 s if the caller disappears,
then closes the connection. Cancellation has no acknowledgement; an unreachable
database, process crash or shutdown may prevent backend termination. Finite local
wait does not guarantee zero query cost or remote termination.
