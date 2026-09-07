# What's new in Connectors

This page explains what the latest release means for people using Connectors and applications
built on it. The [changelog](https://github.com/beyond10x/connectors/blob/main/CHANGELOG.md)
keeps the detailed history and compatibility notes.

## v0.7.1 — 7 September 2026

Applications can now collect changed Jira issues and comments, Confluence pages, and GitLab
project activity and delivery records through bounded reads with explicit pagination. The
operation descriptions explain how to keep a time window consistent and continue to its end.
Comment results retain restriction metadata so applications can enforce their audience policy;
provider identity objects are omitted from these read results.

This release also fixes generated catalog calls that were incorrectly stopped by optional
authentication-remediation checks. It retains v0.7.0's features and operation protocols.

When upgrading a source-built installation, verify every operation your application uses against
the candidate, including its input and output contract. Replace the CLI and local daemon together;
a successful sign-in or account probe does not test a complete collection workflow.

## v0.7.0 — 7 September 2026

This release makes everyday command-line use easier and adds more useful GitLab and Kubernetes
operations. These highlights cover changes since the last downloadable release, v0.6.0.

### More things you can do

- **Manage GitLab pipeline schedules.** Use a personal GitLab connection to list, create, update,
  or delete schedules. Changes still require permission on the connection you select.
- **See what is running in Kubernetes.** List available namespaces and Deployments, including
  their container images and desired and ready replica counts.
- **Fetch GitLab source through a hosted deployment.** Applications can request a short-lived,
  read-only fetch for an allowed project's default branch and exact commit. The provider
  credential stays inside Connectors.

### Less friction in everyday use

- **Simple commands can work without a running daemon.** Search for operations, inspect them,
  make a single permitted call, or read connection details when the local daemon is absent.
  Events, ongoing sessions, and connection activation still need the local service.
- **Clearer help when access is missing.** Authentication failures can identify the connection
  and operation that need setup. After setup succeeds, you explicitly run the action again;
  Connectors does not silently repeat it.
- **Write discovery finds the connections you can use.** A read-only connection no longer hides
  another connection that permits the write. Search and describe also show when approval is needed.
- **Rate limits are easier to handle.** Applications receive a structured refusal and, when the
  provider supplies trustworthy advice, a suggested wait before trying again. Connectors does
  not retry the action automatically.
- **Piping output behaves naturally.** A command can finish successfully when the next program
  in a pipe has read enough and closes its input.

Provider requests can also reuse network connections. Hosted connection setup now keeps an
expired session expired, even if credential verification finishes afterward.

### Before you upgrade

- **Local is the default.** A saved hosted login no longer changes where ordinary operation,
  connection, or event commands run. Choose `--target hosted` when you intend to use a hosted service.
- **Personal GitLab OAuth is an early development feature.** It needs explicit application
  configuration, and its initial credential store is unsealed. See the
  [GitLab guide](docs/guides/connect-gitlab.md) before using it. Older GitLab connections may need
  a verified reconnect before they become active.
- **Multiple credentials per provider are still outside this release.**
- **Custom client authors should check the compatibility notes.** Authentication and rate-limit
  failures have new structured responses, and the canonical catalog format has advanced.

For setup and command details, start with the [documentation overview](README.md) and
[commands and interfaces](docs/architecture/interfaces.md).
