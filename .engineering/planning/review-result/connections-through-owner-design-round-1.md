---
format: aep.planning-md/3
id: review-result:connections-through-owner-design-round-1
kind: review-result
status: active
title: Design critic, round 1, story:connections-list-answers-through-the-owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
verdict: needs-revision

Story file: `.engineering/planning/story/connections-list-answers-through-the-owner.md`. Paths below are relative to the worktree root.

1. **"The owner answers with the document the CLI prints today" (story:105) cannot be built as written.**
   - The document is built in `apps/connectors/src/local/connections.rs` (`summary` and `description` at :472-495, the list wrapper at :120-125, the acquisition value at :154-163). The host crate that serves the owner cannot call those.
   - The host already has a different builder, `connection_value` (`crates/connectors-host/src/local/owner.rs:1285-1303`), used for connect answers. It differs from the CLI's `description`: no `pending_reason` in the summary, `external_identity` is `null` when absent, and it adds `binding`.
   - Reusing it would break acceptance 2.
   - Change: the Design names the answer. Recommended: the owner returns the registry observation as typed data (connection fields, page fields, acquisition fields), and the CLI builds the document with the unchanged `summary`/`description`/acquisition code and `select::name`. State that `connection_value` is not reused and not altered.
   - Acceptance 2 adds a fixture with a `pending` connection (`pending_reason`), one without identity, and one that declares a binding, so a divergent builder fails it.

2. **Refusals will print the wrong stage unless the Design names how they are mapped (story:105-108).**
   - The owner's refusal passes through `From<registry::Failure> for Error` (`owner.rs:297-337`), which is lossy, and then through `owner_failure`. `owner_failure` prints `NotFound` as stage `admission` (`apps/connectors/src/local.rs:~420`). The direct read prints `not_found` at `observation` (`connections.rs:511`).
   - `Client::connections` flattens a refusal and a failed exchange into the same `Err`, via `simple` → `value` → `answer().unwrap_or_else(Err)` (`transport.rs:1127-1133`). "A refusal is not a reason to fall back" cannot be implemented on that path.
   - Change: the Design states that the `observe` client keeps the two results apart. A failed exchange falls back to the direct read. A refusal is printed through a mapping that reproduces the `registry_failure` rows (`connections.rs:496-559`) for every failure that `list`, `describe` and `acquisition_status` can return. `metadata_schema_newer` keeps its store version. Alternatively the refusal carries the registry failure's name.
   - Acceptance 3 adds `status --acquisition <unknown>` and `list --cursor <junk>`, each with and without an owner.

3. **"Describe without --connection" does not fit the selector while `select` is untouched for describe.**
   - `connections::select` runs before dispatch (`local.rs:233`). `connections-describe` is in `SELECTING` (`connections.rs:402-404`). `select` runs `Registry::held` in the CLI process (`connections.rs:434`) and writes `connection` into the input. So the CLI opens the store before any owner call. Acceptance 1's no-`--connection` case fails, and the selector's "none" is unreachable.
   - The refusals also differ. `select` gives `not_found`/`admission`/`create_connection` and `invalid_input`/`arguments`. A describe of an unknown connection gives `not_found`/`observation`. The same owner `Code::NotFound` would need two different stages.
   - The bullet at story:113 ("select ... is not changed") is true for revalidate, revoke, launch and token. It says nothing about describe.
   - Change: the Design removes `connections-describe` from `SELECTING`. The describe arm resolves the held connection itself, through the owner when one runs and directly otherwise, with the refusals of `select` kept. Both paths use one shared function. The mapping depends on whether the request named a connection.
   - Acceptance 3 adds `describe` without `--connection` on an instance with no connection and on one with two, with and without an owner.

4. **The Design is silent on the exchange deadline and on locking, and these change the cost it claims (story:88, 109).**
   - Deadline: `Client::connections` uses a 30 s span (`transport.rs:1066-1069`). The story names none for `observe`. A timeout is a "failed exchange" and falls back to the direct read. That read then queues behind `metadata.lock`, which the owner's still-running request holds, and repeats the full cost. On a large store this is slower than today. Change: name the span and say whether expiry falls back or refuses.
   - The CLI does touch the store: `Client::connect` first calls `Metadata::authority_at_until` (`transport.rs:278` → `metadata.rs:501-505`). That takes the lifecycle lock and opens SQLite read-only in the CLI. It runs even with no owner. It is not an ER open, so acceptance 1 stays green, but it blocks behind an owner `list`, which holds the lock across its open (`metadata.rs:1083`, `registry.rs:626-632`). After 10 s it gives up and the command falls back to the full direct read. Change: the Design states this, or uses the lock-free `reconnect_until` pattern (`transport.rs:229-243`).
   - In the owner, `list` and `status --acquisition` take `Hold::Locked`, and `describe` and `held` take `Hold::Relocks` (`metadata.rs:1080-1088`). `take_or_wait` does not wait for a handle that a relocking user holds, and waits only as long as the last fresh open took (`er.rs:571-607`). With a second observation or an admission in flight, the owner performs the full open the CLI was spared. That costs time and RSS inside the owner, under the lock.
   - Change: the Design says the no-open guarantee is for the CLI process only. Acceptance 6 adds a run with one read invoke or a second observation in flight, or the story names that run as out of scope.

5. **The ESS modelling adds the shared surface and has no consumer in the outcome (story:111, 139).**
   - Only 5 of about 14 request kinds are modelled in ESS: hello, build, cached, connections and reuse (`ess/domains/cli.yaml:232-339`). `Request` is hand-written serde (`owner.rs:1307-1445`). Nothing in code or tests reads the ESS type for it.
   - The cited precedent `c58fdb5f82f` was a separate commit after the request landed, and it did not touch metadata emissions.
   - Acceptance 1-6 and 9 do not depend on it. It is the only source of the `inputs.json` collision with `story:entity-runtime-0330-records-name-definition-by-digest` and of the clash with ten other drafts on `cli.yaml`. It also costs three `slot --timeout 30m` jobs, and the second story to merge repeats them.
   - A Rust-only request meets the outcome. What it gives up is only the convention that `owner.md` names a type for read-through requests, and `owner.md` already describes most kinds without one.
   - A smaller design that adds fields to `Connections` does not work. `Connections` is paged-all, drops revoked connections, judges custody in the owner and flattens refusals into fallback (`transport.rs:3120-3157`). Overloading it would need a union answer type and the same refusal rework.
   - Change: either split the ESS model and its regeneration into a follow-up story that `depends_on` this one (the c58fdb5 shape), or add a sentence to the story giving a reason it must ship with the request, as the story must do for the shared surface it creates.

6. **`owner.md` and the CHANGELOG sentence need corrections beyond "describes the request" (story:135-138).**
   - `contracts/cli/v1alpha1/owner.md:257-261` says a `cached` or `connections` exchange that fails after the greeting, "an owner's refusal ... included", is answered by the direct read. For `observe` the opposite is the design. Acceptance 7 must carve this out.
   - The idle interval restarts at `accept` (`transport.rs:1912-1914`), before the build is compared. After this change the CLI greets any running owner, including one of another build. The CHANGELOG line must say "any owner", not only a same-build one.

What I checked and found sound:
- Cursors are rows in the store (`observation.rs:262-297`), so cursor ownership cannot differ between the two paths.
- `with_system_clock` re-samples under the lock and ignores the caller's `now` (`registry.rs:537-542`, `:640-643`), so the clock floor cannot differ.
- `select::name` runs in the CLI after dispatch (`local.rs:243-245`) and reads only the config, so it is unaffected.
- `source` is a constant and `stale` comes from the page.
- Custody must be judged in the CLI. The owner starts with a cleared environment (`transport.rs:1964-1966` comment), so the owner's keyring view can differ.
- A new request kind needs no protocol version, because it is sent only after `same_build()`.
- The idle claim at story:93 is right. Each exchange runs on its own thread (`transport.rs:2216`), and `flock` is per file description, so threads inside the owner exclude each other.
- No `depends_on` edge touches the story and the graph has no cycle.

What I read: the story, `story:entity-runtime-0330-records-name-definition-by-digest` and `aep plan artifact relations`; the code files named in the brief, plus `metadata.rs`, `er.rs`, `registry.rs`, `lifecycle.rs`, `select.rs` and `definitions.rs`; and commit `c58fdb5f82f`. I did not run `graph` or `validate`.

What I could not establish: the owner's time on a grown store, which the story also lacks, and whether the other story's regeneration touches the same emission parts. The merge-order cost of `inputs.json` belongs to the parallel-safety critic.

```findings
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 105
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Design has the owner answer with the document the CLI prints, but that document is built in apps/connectors/src/local/connections.rs:120-125,154-163,472-495, out of the host crate's reach, and the host's connection_value (owner.rs:1285-1303) differs from it (no pending_reason, null identity, adds binding); the Design must say the owner returns the registry observation as typed data and the CLI builds the document, and acceptance 2 must cover pending_reason, absent identity and a binding"
- file: .engineering/planning/story/connections-list-answers-through-the-owner.md
  line: 105
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the refusal path is unspecified: Client::connections flattens refusal and failed exchange (transport.rs:1127-1133) and owner_failure prints NotFound at admission (local.rs) where the direct read prints observation (connections.rs:511); the Design must name the mapping that reproduces the registry_failure rows and keep refusal apart from fallback, and acceptance 3 must add acquisition and cursor refusals"
- file: apps/connectors/src/local/connections.rs
  line: 402
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "connections-describe is in SELECTING and select runs Registry::held in the CLI before dispatch (local.rs:233, connections.rs:434), so describe without --connection opens the store in the CLI and the selector's none is unreachable; the Design must remove describe from SELECTING, resolve the held connection in the describe arm (owner or direct, shared function, select's refusal rows kept) and test its refusals with and without an owner"
- file: crates/connectors-host/src/local/owner/transport.rs
  line: 278
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Design states no observe exchange deadline or locking effects: Client::connect takes the lifecycle lock in the CLI for the authority read and falls back to the full direct read after 10 s, a timed-out exchange falls back behind the owner's still-held lock, and a concurrent observation makes the owner do a fresh complete open (er.rs:603, metadata.rs:1083); state the span, the fallback on timeout and the owner-side concurrency, and measure or exclude a concurrent run in acceptance 6"
- file: ess/domains/cli.yaml
  line: 296
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the ESS model of the request is documentary (Request is hand-written serde, 9 of about 14 owner kinds are unmodelled) and is the only source of the inputs.json and cli.yaml shared surface and of three 30-minute generation jobs; either move it to a follow-up story that depends_on this one, as c58fdb5f82f did for connections, or state why it must ship with the request"
- file: contracts/cli/v1alpha1/owner.md
  line: 257
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "owner.md says a refusal of a cached or connections exchange is answered by the direct read, which is the opposite of observe, and the idle interval restarts at accept for an owner of any build; acceptance 7 must carve out observe's final refusal and the CHANGELOG entry must say any owner, not only a same-build one"
```
