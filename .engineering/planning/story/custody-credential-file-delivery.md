---
format: aep.planning-md/3
id: story:custody-credential-file-delivery
kind: story
status: draft
title: One admitted delivery of a custody credential to an exact owner-only file
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

One admitted command delivers the credential a connection holds in custody to one exact
owner-only file, so a consumer that reads upstream credentials only from a file (for example a
model gateway's `credential_file`) can use a key that `key.create` put into custody, without the
secret passing through stdout, argv, a log or the caller's shell.

## Today

A key created through `key.create` (story:openrouter-management-provider) is held in custody as a
new connection, and no command writes it anywhere a consumer can read it. That story's outcome
says the one-time secret "never leaves Connectors", and the mounted-secret custody threat model
(docs/local-mounted-secret-custody.md, "Disclosure through Connectors") admits no value in any
output. This story adds the first path by which custody material leaves the process, so it widens
those accepted records: an architecture decision recording the new disclosure path, its admission
and its threat model is required before any implementing wave.

## Change (to be decided before implementation)

- A delivery command, admitted like a write: an approval the record can name, against one
  connection and its current custody version.
- The target is an exact absolute path; the file is created with mode 0600, owned by the running
  user, written atomically (temporary file in the same directory, fsync, rename).
- An existing file is refused unless the caller names the revision a previous delivery wrote, so a
  delivery never overwrites a file it did not write.
- Nothing secret on stdout, stderr, logs, the journal or errors; the receipt is the path and the
  delivered revision (a digest that identifies the version without revealing it).
- Read-only (mounted) custody and connections not ready refuse; a symlink or a directory owned by
  someone else at the target refuses.

## Spec first

The delivery command, its receipt and its refusals are modelled in `ess/domains/cli.yaml` (and the
credentials domain for the delivered-revision record) and validated before any code; the decision
record comes first.

## Acceptance

- A delivered file holds exactly the custody bytes of the named connection, with mode 0600, and
  the command's stdout holds only the path and revision.
- A second delivery to the same path without the matching revision is refused, and the file is
  unchanged; with it, the file is replaced atomically.
- Delivery from mounted custody, from a connection that is not ready, through a symlink, or into a
  directory other users can write is refused, with no file written.
- No test output, log or error of these runs contains the secret (a canary scan).

## Relation to queued work

Distinct from headless credential custody (a custody that reads a mounted secret file): that one
brings a credential into Connectors; this one takes a credential out.
