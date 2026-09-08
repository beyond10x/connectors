# Reproduce the three-provider acceptance run

Run from the repository root on Linux with Rust, Docker, `kubectl`, `jq`, and
OpenSSL installed. These commands create a disposable Kubernetes server and
PostgreSQL database. They use public GitLab reads over verified HTTPS. They never
select the user's default Kubernetes context. Kubernetes uses a private bearer
credential and its cluster CA; SQL uses a restricted reader role. GitLab private
token placement is covered separately by the HTTP fixture tests.

The example uses fixed local container names and service ports 17100–17103. Choose
unused names/ports if they already exist; do not remove another run's resources.
The database's Docker bridge address must be reachable from this Linux host. With
Docker Desktop or a remote Docker daemon, explicitly supply a reachable test
endpoint and publish it in the EndpointSlice instead.

## Build and create private test credentials

```sh
cargo build --workspace --locked
mkdir -p .local/e2e
chmod 700 .local/e2e
umask 077
openssl rand -hex 24 > .local/e2e/service.secret
openssl rand -hex 24 > .local/e2e/postgres.secret
openssl rand -hex 24 > .local/e2e/reader.secret
printf 'POSTGRES_DB=engineering\nPOSTGRES_PASSWORD=%s\n' \
  "$(cat .local/e2e/postgres.secret)" > .local/e2e/postgres.env
```

These are disposable test credentials. Use separate service tokens per trust
boundary in a real deployment. All relative credential/config paths resolve from
the service process's working directory.

## Start PostgreSQL and Kubernetes

```sh
docker run -d --name connectors-v2-postgres --label connectors-v2.e2e=true \
  --memory 512m --env-file .local/e2e/postgres.env \
  --publish 127.0.0.1::5432 postgres:17-alpine
docker run -d --name connectors-v2-k3s --label connectors-v2.e2e=true \
  --privileged --memory 2g --tmpfs /run --tmpfs /var/run \
  --publish 127.0.0.1::6443 rancher/k3s:v1.31.5-k3s1 \
  server --disable traefik,servicelb,metrics-server \
  --tls-san 127.0.0.1 --write-kubeconfig-mode 600
```

Wait for `docker exec connectors-v2-postgres pg_isready -U postgres -d engineering`
to succeed. Initialize the fresh database exactly once:

```sh
docker exec -i connectors-v2-postgres psql -U postgres -d engineering \
  -v ON_ERROR_STOP=1 < examples/live/postgres.sql
printf "ALTER ROLE connector_reader PASSWORD '%s';\n" \
  "$(cat .local/e2e/reader.secret)" | \
  docker exec -i connectors-v2-postgres psql -U postgres -d engineering -v ON_ERROR_STOP=1
```

Wait until the Kubernetes container has written its kubeconfig. Extract it without
printing its credentials, then point only that private config at the mapped port:

```sh
docker exec connectors-v2-k3s cat /etc/rancher/k3s/k3s.yaml > .local/e2e/kubeconfig
CONNECTORS_KUBE_ADDRESS=$(docker port connectors-v2-k3s 6443/tcp)
kubectl --kubeconfig .local/e2e/kubeconfig config set-cluster default \
  --server "https://$CONNECTORS_KUBE_ADDRESS"
kubectl --kubeconfig .local/e2e/kubeconfig wait --for=condition=Ready nodes --all --timeout=180s
kubectl --kubeconfig .local/e2e/kubeconfig apply -f examples/live/kubernetes.yaml
kubectl --kubeconfig .local/e2e/kubeconfig -n engineering \
  create token connector-reader --duration=1h > .local/e2e/kubernetes.secret
kubectl --kubeconfig .local/e2e/kubeconfig config view --raw \
  -o jsonpath='{.clusters[0].cluster.certificate-authority-data}' | \
  base64 -d > .local/e2e/kubernetes-ca.pem
CONNECTORS_PG_ADDRESS=$(docker inspect --format \
  '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' connectors-v2-postgres)
jq -n --arg address "$CONNECTORS_PG_ADDRESS" \
  '{apiVersion:"discovery.k8s.io/v1",kind:"EndpointSlice",
    metadata:{name:"postgres-external",namespace:"engineering",
      labels:{"kubernetes.io/service-name":"postgres",
              "endpointslice.kubernetes.io/managed-by":"connectors-v2-e2e"}},
    addressType:"IPv4",ports:[{name:"postgres",port:5432,protocol:"TCP"}],
    endpoints:[{addresses:[$address],conditions:{ready:true}}]}' | \
  kubectl --kubeconfig .local/e2e/kubeconfig apply -f -
```

The token expires. To rerun later, mint a replacement into a private regular file
and atomically rename it over the binding. A projected service-account symlink
cannot be passed to this strict file store; a host-owned credential implementation
or an explicit private copy/rotation process must handle that deployment.

## Configure and start the HTTP adapters

Generate non-secret configuration referencing the private files:

```sh
jq -n '{service:{instance:"gitlab-live",listen:"127.0.0.1:17101",
    service_credential:{kind:"file",path:".local/e2e/service.secret"}},
  http:{base_url:"https://gitlab.com/api/v4/",credential:null,
    credential_header:"private-token",bearer:false},
  adapter:{allowed_projects:["gitlab-org/gitlab"]}}' > .local/e2e/gitlab.json
jq -n --arg base "https://$CONNECTORS_KUBE_ADDRESS/" \
  '{service:{instance:"kubernetes-live",listen:"127.0.0.1:17102",
    service_credential:{kind:"file",path:".local/e2e/service.secret"}},
  http:{base_url:$base,credential:{kind:"file",path:".local/e2e/kubernetes.secret"},
    credential_header:"authorization",bearer:true,ca_file:".local/e2e/kubernetes-ca.pem"},
  adapter:{namespaces:["engineering"],
    resource_kinds:["pods","services","deployments","endpointslices"],
    discover_hosts:true}}' > .local/e2e/kubernetes.json
```

Start each service in its own terminal from the repository root:

```sh
target/debug/connectors-gitlab --config .local/e2e/gitlab.json
```

```sh
target/debug/connectors-kubernetes --config .local/e2e/kubernetes.json
```

## Discover an endpoint and explicitly bind SQL

```sh
printf '%s\n' '{"namespace":"engineering","limit":100}' > .local/e2e/discover.json
target/debug/connectors invoke --endpoint http://127.0.0.1:17102/ \
  --allow-plaintext --token-file .local/e2e/service.secret \
  --operation endpoints.discover --input .local/e2e/discover.json \
  > .local/e2e/discovered-endpoints.json
jq '.items[] | select(.service == "postgres")' .local/e2e/discovered-endpoints.json
```

Inspect the address, port, readiness, namespace and provenance. Selecting this
known disposable database is the explicit host/operator binding step. Discovery
itself has neither a password nor permission to connect. After inspecting it:

```sh
CONNECTORS_SQL_ADDRESS=$(jq -er \
  '.items[] | select(.service == "postgres" and .ready == true) | .address' \
  .local/e2e/discovered-endpoints.json)
jq -n --arg host "$CONNECTORS_SQL_ADDRESS" \
  '{service:{instance:"sql-live",listen:"127.0.0.1:17103",
    service_credential:{kind:"file",path:".local/e2e/service.secret"}},
  adapter:{host:$host,port:5432,database:"engineering",user:"connector_reader",
    allow_plaintext:true},password:{kind:"file",path:".local/e2e/reader.secret"}}' \
  > .local/e2e/sql.json
```

Start SQL in a third terminal:

```sh
target/debug/connectors-sql --config .local/e2e/sql.json
```

The plaintext PostgreSQL setting is specific to this disposable local database.
The adapter requires verified database TLS unless that option is explicitly set.

## Federate and verify

```sh
jq -n '{service:{instance:"engineering-live",listen:"127.0.0.1:17100",
    service_credential:{kind:"file",path:".local/e2e/service.secret"}},
  downstreams:([{name:"gitlab",port:17101},{name:"kubernetes",port:17102},
    {name:"sql",port:17103}] | map({name:.name,
    endpoint:("http://127.0.0.1:"+(.port|tostring)+"/"),
    credential:{kind:"file",path:".local/e2e/service.secret"},allow_plaintext:true}))}' \
  > .local/e2e/federation.json
```

Start the gateway in a fourth terminal after the three leaf services are listening:

```sh
target/debug/connectors serve --config .local/e2e/federation.json
```

Run the live Rust acceptance executable from another terminal:

```sh
target/debug/connectors-conformance --token-file .local/e2e/service.secret \
  --allow-plaintext > .local/e2e/live-acceptance.json
cat .local/e2e/live-acceptance.json
```

Exit 0 and `status: passed` mean nine scenario groups passed: service authentication
for each provider, then all supported operations and selected failure cases directly
and through federation. The SQL deadline checks intentionally take roughly ten
seconds each. Public GitLab availability and rate limits are external dependencies;
an unsuccessful run is not passing evidence. The runner performs no automatic retry.

## Shutdown

Send Ctrl-C or SIGTERM to each of the four service processes and wait for exit.
After verifying that the containers belong to this run, remove these exact fixtures:

```sh
docker inspect --format '{{.Name}} {{index .Config.Labels "connectors-v2.e2e"}}' \
  connectors-v2-postgres connectors-v2-k3s
docker stop connectors-v2-postgres connectors-v2-k3s
docker rm -v connectors-v2-postgres connectors-v2-k3s
```

Keep `.local/e2e` private; it is ignored by Git and contains ephemeral credentials.
No test creates an organization registration, remote repository, or Atlas record.
