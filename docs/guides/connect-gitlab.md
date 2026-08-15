# Connect GitLab

Choose the identity GitLab should see. This is independent of whether a person, Zwirn, or another
B10x agent invokes the Connection later.

## As myself

Use OAuth for the normal developer flow. Select **Add GitLab → As myself**, sign in to GitLab, and
approve the requested access. The resulting Connection is principal-owned: GitLab applies your
memberships and permissions, and actions such as opening an issue are attributed to you.

A personal access token is an explicit alternative for installations where OAuth is unavailable.
Paste it only into the protected Connect Session. `read_api` enables the selected reads; `api` also
enables writes. The harness, model, and client never receive the token.

## As automation

Use a non-human GitLab identity for B10x or babelforce automation. Select the narrowest kind:

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
B10x's Connection and operation Grants. The user and automation Connections remain separate,
have separate credential generations, and never fall back to one another.

The current repository ships this catalog and acquisition contract. The general Connect Session,
OAuth callback, capability-evidence, and invocation runtime remains the M3 platform implementation;
until that lands, this page describes the product flow rather than claiming the foundation CLI can
complete it today.

## GitLab references

- [REST API authentication](https://docs.gitlab.com/api/rest/authentication/)
- [OAuth 2.0 provider API](https://docs.gitlab.com/api/oauth2/)
- [Access-token scopes](https://docs.gitlab.com/security/tokens/access_token_scopes/)
- [Service accounts](https://docs.gitlab.com/user/profile/service_accounts/)
