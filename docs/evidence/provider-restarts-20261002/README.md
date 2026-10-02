# Disposable provider restart acceptance, 2026-10-02

Both exact live-provider cases passed against task-owned local containers, using
source commit `0f9d56f994009b19490f609ced3239f85a0c7953` with release metadata
prepared for 0.25.0. The built executable identities are in [binaries.sha256](binaries.sha256).
These are runtime journey results, not evidence that every provider workflow is complete.

| Provider | Exact libtest case | Result | Provider |
| --- | --- | --- | --- |
| PostgreSQL | `cli_journey::a_real_postgres_session_persists_across_cli_and_owner_restart` | 1 passed, 0 failed; 30.81s | PostgreSQL 17.6 Alpine 3.22 |
| Kubernetes | `cli_journey::a_real_cluster_session_persists_across_cli_and_owner_restart` | 1 passed, 0 failed; 21.23s | K3s v1.31.5-k3s1 |

Raw test outputs: [PostgreSQL](postgres.log), [Kubernetes](kubernetes.log).
Each compiled test binary ran with `--ignored --exact <case> --nocapture
--test-threads=1`, the current `CONNECTORS_TEST_CLI`, its explicit sandbox endpoint,
and a physical private temporary directory. Kubernetes used an explicit CA file
and a namespaced service-account token; credentials are retained privately only.
No ambient Kubernetes context was selected. PostgreSQL used an isolated incidents
database and a read-only reader role. Both owner/keyring restart journeys used the
qualified GNOME Secret Service executable, SHA256
`b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b`.

Container image identities:

- PostgreSQL: `sha256:d741b376874687de90374fd34f55c6b2760e8f7bd7e4ae5cd47f50757fc08cf8`.
- K3s: `sha256:efe65d76faac869ca7329373cb5a5c5676cb3dbfc0a48d0574c6f4d7fa285d2a`.

The Kubernetes fixture pod existed but remained Pending under node disk pressure.
This evidence therefore establishes API inventory and connection reuse, not a
running workload, logs, exec, mutation, tunnel, or Helm acceptance. The test's
invalid-token refusal does not establish the full per-operation authorization
coverage contract. No provider milestone is closed by this receipt alone.
