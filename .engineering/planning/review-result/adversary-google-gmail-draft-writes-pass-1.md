---
format: aep.planning-md/3
id: review-result:adversary-google-gmail-draft-writes-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Gmail draft writes
relations:
- reviews: story:catalog-google-gmail-draft-writes
revision: 1
---
unit: story:catalog-google-gmail-draft-writes at 3dfb03159 on impl/catalog-google-gmail-draft-writes (base 307c82dd0), plus my one untracked test file
verdict: NEEDS-CHANGE
cases: executed 188→190, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (~/.cache/w0930wg/adv1, ~/.cache/w0930wg/adv1/suite.log)
needs-coordinator: the fix for findings 1 and 2 needs a change to `adapters/catalog/src/lib.rs`, which the story rules out and the 20260930b session is editing

## 1. Diff

`git --no-pager diff --stat` is empty because my only file is untracked. `git status --short`:
```
?? .agents/
?? adapters/catalog/tests/google_gmail_writes_adversary.rs
```
`.agents/` was already there and is not mine. I changed no tracked file and no implementation file.

## 2. Cases added (in `adapters/catalog/tests/google_gmail_writes_adversary.rs`)

| case | what it asserts | now |
|---|---|---|
| `a_send_whose_body_replaces_the_draft_message_sends_nothing` | A `users.drafts.send` input `{"messageId": <the stored draft's message.id>, "body": {"id": DRAFT, "message": {"raw": <other message>}}}` never reaches a POST | red |
| `the_send_declaration_refuses_a_body_message` | The send's declared input schema accepts `body: {"id"}` and refuses a `body.message` | red |

Red output (`cargo test --locked -p connectors-catalog-provider --test google_gmail_writes_adversary`):
```
a send replacing the draft's message passed the preflight (applied: true); preflight reads [(["gmail", "v1", "users", "me", "drafts", "fixture-draft-1"], [])], POSTs [(Post, ["gmail", "v1", "users", "me", "drafts", "send"], Object {"id": String("fixture-draft-1"), "message": Object {"raw": String("VG86IG90aGVyQGV4YW1wbGUudGVzdA0KDQpPdGhlcg0K")}})]
the send declaration admits body.message; declared body schema: {"type":"object"}
test result: FAILED. 0 passed; 2 failed
```

## 3. Suite

`cargo test --locked -p connectors-catalog-provider --no-fail-fast`, exit 101. The only failures are in `google_gmail_writes_adversary`: 0 passed, 2 failed. The other 22 test binaries passed 188 cases, 6 ignored, including `google_gmail` 31 and `local_runtime` 30.

## 4. Findings

**1. A send can replace the approved draft and still pass the guard** (`adapters/catalog/providers/google-gmail/operations.json:33`, NEEDS-CHANGE, introduced, blocker). `prepare` runs the `drafts.get` preflight; the stored draft is unchanged, so its `message.id` matches `messageId` and the check passes. `lib.rs:539` then forwards the whole `body`, so the POST to `/drafts/send` carries a `message` that nothing compared. The only defence is a rule for the approver (`docs/catalog-google-gmail.md:193`); the approval presentation shows only instance, operation, connection and revision. Fix: let a selection declare a closed body (body allowing only `id`), enforced in `declare` and `prepare` in `lib.rs`. Assumption not checked live: that drafts.send accepts a `body.message` replacement (from the guide at `docs:193-195`).

**2. The declared schema contradicts the send's description** (`adapters/catalog/src/lib.rs:860`, NEEDS-CHANGE, introduced, warning). The send's declared schema gives `body` as `{"type":"object"}`; `validate_write_value` accepts `body.message` although the description says "Send body as {"id": draft id} only".

**3. A scope sentence in the guide reads wrong** (`docs/catalog-google-gmail.md:224`, CONFIRMED, introduced, note). It reads as if `drafts.get` rejects `gmail.readonly`; the pinned Discovery lists `gmail.readonly` for `gmail.users.drafts.get`.

## 5. Attacked and could not break
Changing `messageId`, or leaving out `messageId` or `body.id`, sends nothing; a missing draft is refused on the preflight's 404. `read_json` refuses duplicate keys. Create reaches only `/drafts`; upload paths are excluded. Every scope claim matches the pinned Discovery. The 256 KiB cap matches `TARGET_LIMIT`. `threadId` and In-Reply-To only thread the draft. Not testable offline: standard-base64 `raw`, Bcc in the draft's raw form, whether editing a draft changes its `message.id`.

## 6. Paths written outside the worktree
`~/.cache/w0930wg/adv1` (mode 700), `~/.cache/w0930wg/adv1/suite.log`. Lease `wave0930c-gmail-writes-adv1` released.

## 7. Findings block
```findings
- file: adapters/catalog/providers/google-gmail/operations.json
  line: 33
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "A users.drafts.send whose body carries a replacement message passes the messageId preflight on the unchanged stored draft and POSTs content no guard compared, so what is sent is not the draft the operator read."
- file: adapters/catalog/src/lib.rs
  line: 860
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The send's declared input schema types body as any object, so operations describe and the owner's validate_write_value admit body.message although the send's description forbids anything but id."
- file: docs/catalog-google-gmail.md
  line: 224
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "The guide's scope sentence reads as if users.drafts.get rejects gmail.readonly, but the pinned Discovery lists gmail.readonly for gmail.users.drafts.get."
```


(Absolute paths under the home directory are written as `~` in this record; the report was otherwise recorded as returned.)
