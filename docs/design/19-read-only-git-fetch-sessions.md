# 19. Read-only Git fetch sessions

Status: accepted for implementation on 2026-09-04.

## Decision

Connectors owns a short-lived, read-only Git byte plane for repositories admitted by a current
principal-owned Connection. A caller first authenticates to the ordinary hosted API and asks for a
fetch session naming the Connection, provider project, provider-selected branch, exact commit, and
bounded clone depth. A bounded idempotency key identifies the materialization attempt. Connectors
revalidates that the Connection belongs to the verified tenant and actor before returning a stable,
non-secret repository locator and a one-use source authority. Replaying the same key and coordinates
keeps the locator stable while rotating only the transient source authority and expiry; changing
coordinates under a retained key is a conflict.

The locator serves only Git smart-HTTP discovery and `git-upload-pack`. It never serves
`git-receive-pack`, accepts provider credentials, follows redirects, exposes an upstream URL, or
appears in the Connector catalogue, MCP surface, or public OpenAPI document. The upstream GitLab
OAuth or personal token remains in Connector custody and is attached only after the session,
Connection, actor, project, and request budget have all been admitted.

Fetch sessions expire after 15 minutes, allow at most 32 HTTP requests, and stop after one GiB of
aggregate response bytes. Creation admits depths from 1 through 50. A session is spent after its
pack exchange completes and is unusable after expiry, revocation, or budget exhaustion. Session
state contains only a digest of the returned source authority. At most eight live sessions belong
to one actor and at most 1,024 are live in one Connector process. Exhaustion refuses new creation;
it never evicts another actor's live or in-flight session. Responses and failures are non-cacheable
and never echo credential material.

## Routing and deployment

The control route is `POST /git-fetch-sessions` beneath the configured hosted API base. The byte
plane is rooted at `/internal/git-fetch/{session_ref}/{repository}.git`; it is deliberately outside
that public base. When fetch sessions are enabled, Connectors requires a dedicated
`server.git_fetch_tls` listener with absolute certificate and private-key paths. Only the internal
router is mounted on that native TLS listener; the ordinary public listener never contains the
internal route. A Kubernetes Service maps its private port to that listener, ingress never selects
the private port, and NetworkPolicy admits it only from Substrate. Thus TLS is terminated by
Connectors itself rather than being inferred from NetworkPolicy or a public ingress.

The hosted process accepts one atomic, secret-free deployment override for this placement. All four
variables must be absent or present together; a partial or invalid set prevents startup. They are
applied before the ordinary hosted configuration is validated:

- `CONNECTORS_GIT_FETCH_ORIGIN`
- `CONNECTORS_GIT_FETCH_TLS_LISTEN`
- `CONNECTORS_GIT_FETCH_TLS_CERTIFICATE_FILE`
- `CONNECTORS_GIT_FETCH_TLS_PRIVATE_KEY_FILE`

The TLS listener bounds concurrent established connections and handshake, read-idle, and
write-idle time. Request bodies are bounded and read only after Identity or source authority has
been admitted. Upstream response headers have a fixed deadline, response-body reads have an idle
deadline, and the session expiry remains a hard deadline while an upstream stream is stalled.

The source authority is presented in `X-B10X-Git-Source-Authorization`. It is not an Identity
bearer and cannot authorize any other Connector operation. The control response carries it once;
the client must not persist it. The repository locator contains no secret query parameters or
userinfo.

## Provider binding

GitLab project identity is the numeric project id already returned by the Connector datasource.
Session creation resolves the project through the selected Connection, verifies the requested
commit against that project, and requires the branch to still be the provider's default branch
rather than assuming a branch named `main`. Smart-HTTP requests are proxied to the exact repository
path returned for that numeric project by the configured GitLab instance, with redirects disabled;
that upstream path is re-read during every exchange so a provider rename cannot retain a stale
repository coordinate. Neither the path nor its credential crosses the Connector boundary.

This release explicitly refuses the `Git-Protocol` header and therefore protocol v2. Its legacy
discovery parser buffers a bounded advertisement and returns only the admitted default branch and
its exact commit (plus the matching `HEAD` row and capabilities). Other branch, tag, and object ids
are not disclosed.

**2026-09-05 amendment:** [design 20](20-bounded-git-protocol-v2.md) adds bounded protocol v2
negotiation while preserving this legacy path and the existing control contract.

Before proxying an upload-pack request, Connectors parses its packet-line negotiation. Every
`want` must name the admitted exact commit and the request must carry a positive `deepen` no larger
than the session depth. Alternate commits and unbounded, deepen-since, deepen-not, and want-ref
negotiations are refused.

## Failure posture

Invalid input is refused without contacting GitLab. Missing or foreign Connections and projects
are indistinguishable. Provider refusal, stale authority, and exhausted sessions fail closed. A
persisted legacy Connection row without the Grant that originally admitted it is refused rather
than rebound to current policy. A restart may invalidate all in-memory fetch sessions; callers
create a fresh session and Substrate reconciles an incomplete workspace materialization.

## Consequences

Workspace no longer reads and uploads every repository file. Substrate receives a normal shallow
Git repository while Connectors remains the only component that can see the user's provider token.
This is a capability seam, not a general-purpose credential or HTTP proxy.
