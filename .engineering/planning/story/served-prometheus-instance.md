---
format: aep.planning-md/3
id: story:served-prometheus-instance
kind: story
status: draft
title: connectors-prometheus serves query reads over HTTP as its own Deployment
summary: 'A --config served mode beside --local-config: static caller token, series.query and query_range only, mounted bearer for an in-cluster Prometheus'
relations:
- serves: vision:independent-contract-adapters
- depends_on: story:served-kubernetes-authenticates-in-cluster
- informed_by: story:served-kubernetes-admits-service-account-tokens
revision: 2
---
Requested by a downstream consumer, 2026-11-11. Why: the cluster in question has no
Grafana, so the Grafana data-source proxy route (story:catalog-grafana-datasource-proxy-reads)
cannot reach its data.

## Outcome

`connectors-prometheus` gains a served mode, the same two modes `connectors-kubernetes` has. Beside
`--local-config` it takes `--config` with `{service, http, adapter}`
(`adapters/kubernetes/src/main.rs:30-36`). Today `adapters/prometheus/src/main.rs` has only
`--local-config` and the private descriptor exchange (`main.rs:10-22`).

The target deployment is a Connectors Prometheus instance running as its own Deployment in a
Kubernetes cluster:

- an HTTP listener that admits callers by a static caller token;
- only `series.query` and `series.query_range` are admitted;
- `rules.list` is absent unless the instance enables it (`adapters/prometheus/src/lib.rs:14`);
- `base_url` is a Prometheus Service in the same cluster.

The instance holds a bearer token from a mounted file, for example a projected ServiceAccount
token. No credential-free profile is asked for. `prometheus.bearer` is proved as today by
`GET api/v1/status/buildinfo` (`adapters/prometheus/src/auth.rs:7,22,76`).

## Acceptance

- `connectors-prometheus --config <file>` with `{service, http, adapter}` starts an HTTP listener.
  A caller presenting the static caller token gets `series.query` and `series.query_range`
  answered against a recorded Prometheus fixture. A caller without the token, or with another
  token, is refused before any upstream request.
- The descriptor lists `rules.list` only when the instance enables it. A test pins both
  descriptors.
- The existing bounds hold in served mode: a range window of at most seven days
  (`lib.rs:19,439`) and at most 2,000 points per series (`SAMPLES`, `lib.rs:23`). A test exceeds
  each one in served mode and is refused as in local mode.
- The upstream bearer is proved once at start by `GET api/v1/status/buildinfo`. A 401 or 403
  refuses the start by a named reason. The token never appears in a log, an error or a describe
  answer.
- `--local-config` and the private exchange are unchanged. The existing local tests stay green.

## Out of scope

- A credential-free upstream profile.
- Caller admission by TokenReview (see Ambiguities).
- Serving the Loki or Grafana adapters.

## Ambiguities

- **Would the served host refuse the projected token file?** Yes, as the code stands.
  - The upstream credential is read by `read_secret_file` → `read_file(.., Readers::Owner)`
    (`crates/connectors-host/src/credentials.rs:136-166`). It opens the path with `O_NOFOLLOW`
    and admits only a regular file of at most 8192 bytes, with no group or other bits, owned by
    the effective user (`admitted`, `credentials.rs:174-193`).
  - A projected ServiceAccount token at its standard path is a symlink into the rotating `..data`
    directory, so `O_NOFOLLOW` refuses it. Its file mode is usually 0644, or 0640 under
    `fsGroup`, and its owner is root. The owner-only rule refuses both unless the pod runs as
    that owner with mode 0600 or 0400.
  - The host already reads one projected token: the TokenReview instance token
    (`ReviewTokenFile`, `credentials.rs:104-115`) is admitted at exactly 0640 or 0440 under a
    group the process holds. That rule is not applied to upstream credentials.
  - So this story needs the projected-token file reading that
    story:served-kubernetes-authenticates-in-cluster introduces: a symlinked, rotating token,
    re-read before expiry. It depends on that story, or on the shared part split out of it.
- **Caller admission by TokenReview:** whether `ServiceConfigV3` admission (as in
  story:served-kubernetes-admits-service-account-tokens) belongs in this story's scope instead of
  the static caller token is an open question for the operator.

