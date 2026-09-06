# Connect GitLab

Choose the identity GitLab should see. This is independent of whether a person, Zwirn, or another
platform agent invokes the Connection later.

## Use an existing personal catalog Connection

For a GitLab placement already configured in a personal `[[catalog]]` block, run the local daemon
with its configuration and private state root:

```sh
connectors serve local --config /absolute/path/personal.toml \
  --state-root /absolute/path/private-state
```

In another terminal, use those same paths to find the Connection and its admitted schedule operations:

```sh
connectors connection list --target local --query gitlab -o json \
  --config /absolute/path/personal.toml --state-root /absolute/path/private-state
connectors operation search --target local --query 'pipeline schedule' --limit 25 -o json \
  --config /absolute/path/personal.toml --state-root /absolute/path/private-state
connectors operation describe --target local --operation gitlab-pipeline-schedule-list -o json \
  --config /absolute/path/personal.toml --state-root /absolute/path/private-state
```

Connection list and operation discovery use the same opaque `connection_ref`. The passive catalog
listing reports `created`: the placement is configured, but this listing does not read credentials
or contact GitLab and has no persisted verification result. A healthy configured placement can
therefore appear as `created`; that state alone does not prove it can authenticate. Actor, ownership
and authentication-profile fields stay absent when this configuration does not establish them.

Use the Connection reference and the fresh `description_ref` from describe to list schedules:

```sh
connectors operation invoke --target local --operation gitlab-pipeline-schedule-list \
  --connection CONNECTION_REF --description-ref DESCRIPTION_REF \
  --input-json '{"id":"group/project"}' -o json \
  --config /absolute/path/personal.toml --state-root /absolute/path/private-state
```

The project `id` accepts a numeric ID or a logical project path such as `group/project`; pass the
logical value without pre-encoding it. Operation search accepts up to 25 results, and Connection
list accepts up to 64. Read-only placements expose schedule listing. Creating, updating and deleting
schedules require write permission on the selected placement and sufficient GitLab access.

To authorize an existing placement for writes, review its exact `[[catalog]]` block and set
`allow_writes = true`, then restart its daemon with the same paths. Other placements can remain
read-only. Follow the [daemon update procedure](../architecture/deployment.md#update-a-running-local-daemon)
when installing a build containing this flow. Local write permission does not expand the token's
GitLab scopes or project role.

Describe `gitlab-pipeline-schedule-create` after enabling writes. Its required input is an `id` plus
one JSON `body` containing `description`, `ref` and `cron`:

```sh
connectors operation describe --target local --operation gitlab-pipeline-schedule-create -o json \
  --config /absolute/path/personal.toml --state-root /absolute/path/private-state
connectors operation invoke --target local --operation gitlab-pipeline-schedule-create \
  --connection CONNECTION_REF --description-ref DESCRIPTION_REF \
  --input-json '{"id":"group/project","body":{"description":"Nightly build","ref":"main","cron":"0 2 * * *"}}' -o json \
  --config /absolute/path/personal.toml --state-root /absolute/path/private-state
```

Use the new description's references for that invocation. The complete body schema, including
schedule `inputs`, is shown by describe. Omitted fields stay omitted and explicit `null` is checked
against that schema. For `gitlab-pipeline-schedule-update`, `pipeline_schedule_id` is also required,
while `body` is optional: leaving out `body` sends no body; supplying `"body": {}` sends an empty JSON
object. `gitlab-pipeline-schedule-delete` takes `id` and `pipeline_schedule_id` and sends no body.

Schedule mutations carry `approval: required`. In the personal catalog path, the local owner and
the selected Connection's write permission enforce admission. Do not invent an
`--approval-evidence-ref` to authorize it. Selecting a read-only Connection remains refused before
credential access or provider egress, even when another Connection admits the operation.

## As myself

Use OAuth for the normal developer flow. Select **Add GitLab → As myself**, sign in to GitLab, and
approve the requested access. The resulting Connection is principal-owned: GitLab applies your
memberships and permissions, and actions such as opening an issue are attributed to you.

A personal access token is an explicit alternative for installations where OAuth is unavailable.
Paste it only into the protected Connect Session. `read_api` enables the selected reads; `api` also
enables writes. The harness, model, and client never receive the token.

For a hosted deployment, an operator first creates one GitLab OAuth application whose callback is
the configured `/oauth/gitlab/callback` URL. Its application ID is non-secret hosted configuration;
its secret is supplied once to the running Connector:

```shell-session
connectors admin credentials set gitlab oauth_client_secret --endpoint URL --secret-stdin
```

Every engineer then authenticates that same application as themselves. GitLab issues authority for
the person completing consent; the shared application secret does not cause users to operate under
the operator who registered the application.

## As automation

Use a non-human GitLab identity for platform or babelforce automation. Select the narrowest kind:

- **Project access token** for one project.
- **Group access token** for one group and its projects.
- **Service account token** when one durable automation identity needs memberships across several
  groups or projects.

Create the token in GitLab with `read_api` for read-only automation or `api` for the selected writes,
then enter it once in the protected Connect Session. These token kinds all commonly begin with
`glpat-`; the prefix cannot prove which authority they carry, so the choice is explicit and is
stored as the Connection's credential purpose. Before the Connection becomes callable, the
Connector checks the effective user is a GitLab bot and reads the token's actual classic scopes and
expiry from GitLab. A human PAT submitted as automation is refused.

## What “on behalf of” means

```text
Zwirn ── Grant ──▶ your GitLab Connection ──▶ GitLab attributes the call to you
      └─ Grant ──▶ automation Connection ───▶ GitLab attributes it to the bot/service account
```

Zwirn is the caller in both cases. It does not exchange an automation token for your authority.
When you authorize a user Connection, you delegate permission to use that specific Connection under
the platform's Connection and operation Grants. The user and automation Connections remain separate,
have separate credential generations, and never fall back to one another.

## GitLab references

- [REST API authentication](https://docs.gitlab.com/api/rest/authentication/)
- [OAuth 2.0 provider API](https://docs.gitlab.com/api/oauth2/)
- [Access-token scopes](https://docs.gitlab.com/security/tokens/access_token_scopes/)
- [Service accounts](https://docs.gitlab.com/user/profile/service_accounts/)
