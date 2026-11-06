---
format: aep.planning-md/3
id: review-result:wave-20261106-u1-ci1-security
kind: review-result
status: active
title: Wave 2026-11-06 U1 CI follow-up 1 security review (700c77675)
relations:
- reviews: story:provider-programs-run-on-macos
revision: 1
---
unit: U1 security (700c77675)
verdict: nothing found on the question; 2 notes, both INFEASIBLE
cases: executed n/a→n/a, red 0 (I added no case: this code is macOS-only and none of it runs on Linux)
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 2 paths, listed below
needs-coordinator: no

`git --no-pager diff --stat`: empty. The tree is clean at 700c77675.

**Answer: the item's ACLs are the same as before.** No authorization gets wider or narrower: encrypt, delete, change-ACL, export and the decrypt prompt are all unchanged. I read Apple's Security sources at `apple-oss-distributions/Security` commit `db15acbe` (Security-61901.0.87.0.1), fetched from raw.githubusercontent.com. Nothing below comes from memory unless it is marked inferred.
- `SecAccessCreate` (SecAccess.cpp:258-275): a non-NULL list builds `Access(descriptor, trusted)` with one entry per element, so none here. A NULL list builds `Access(descriptor)` (Access.cpp:70-75), which adds `new TrustedApplication` for the caller. Both paths then run the same `makeStandard`.
- `makeStandard` (Access.cpp:83-117) uses the trusted list in one place only, `limited->applications() = trusted` (:115), on the decrypt/sign/mac/derive/export ACL.
  - The owner ACL (change-ACL) has no applications and is gated by a keychain prompt.
  - The encrypt ACL allows all applications.
  - Neither depends on the list.
- `SecAccessCopyMatchingACLList(decrypt)` finds only that limited ACL (`ACL::authorizes`, ACL.cpp:168-174). The loop's `SecACLSetContents(NULL)` switches it to allow-all (SecACL.cpp:196-204) without clearing its application list.
  - When the allow-all form is written out, the application list is never read (ACL.cpp:450-470). The result is "any application, or the prompt".
  - The description and prompt selector are copied through unchanged in both cases (descriptor and `defaultSelector`, ACL.cpp:45, 118-127).
  - So `setAccess` writes the same entries either way: encrypt and the decrypt/sign/mac/derive/export rights for any application, change-ACL behind the prompt, and no delete entry in either version.
- The integrity ACL is built from the item's digest (Item.cpp:478-508) and does not involve the trusted list. The partition ID is added by securityd from the caller's identity (inferred; I did not read securityd). The client sends identical entries either way, so this change cannot affect it.
- The inferred cause matches the source, but nobody has observed it at runtime. On the NULL path, `TrustedApplication()` (TrustedApplication.cpp:70-77) reaches `OSXVerifier` (osxverifier.cpp:43-65), which opens the calling executable by path (`makeLegacyHash`, :123). If that file has been deleted, this throws and `SecAccessCreate` returns an error. The empty array builds no `TrustedApplication`, and SecAccess.h:125-128 documents an empty array as "no applications".

**Failure points:** every one still answers `Unavailable` (keychain.rs:122, 134, 139, 156, 170). The only new call that can fail, `from_CFTypes(&[])` (:110), panics only if `CFArrayCreate` returns NULL (core-foundation 0.10.1, array.rs:88-94, lib.rs:115). `CFString::new` at :109 already fails the same way, so this is not a new kind of failure. `add` (:299) is the only caller.

**Rest of the diff:** comment changes, plus a workflow step that runs only when the job fails. That job has no secrets and `contents: read`; nothing changes when the job passes.

**Notes**
- U1-SEC-1, keychain.rs:108, pre-existing. No test reads back the created item's access to check that change-ACL stays behind the prompt and decrypt allows any application. The claim that the ACLs end as before is checked against the source, not by running anything. Such a test could only run on macOS.
- U1-SEC-2, macos-host.yml:120, introduced. `show-keychain-info` has no step timeout. If it raises a SecurityAgent prompt (inferred; `< /dev/null` does not answer a GUI prompt), the failed job waits out its 60-minute timeout. Adding `timeout-minutes: 2` to the step would bound that.

I did not re-run the darwin clippy build; the brief says it compiles.

Files written outside the worktree:
- ~/.cache/connectors-wave-20261106/u1/sec/apple-src/ (12 Apple source files, read-only reference)
- ~/.cache/connectors-wave-20261106/u1/sec/tree.txt

```findings
[{"file":"crates/connectors-custody/src/keychain.rs","line":108,"category":"mutant","severity":"note","verdict":"INFEASIBLE","origin":"pre-existing","message":"U1-SEC-1: no case reads back the created item's access to assert change-ACL stays prompt-gated and decrypt admits any application; the unchanged-ACL claim is source-traced only and a case could run only on macOS"},{"file":".github/workflows/macos-host.yml","line":120,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"U1-SEC-2: show-keychain-info in the failure-only step has no step timeout, so a SecurityAgent prompt (inferred) would hold the failed job until its 60-minute timeout"}]
```


(Coordinator: home-directory paths shortened to ~ for the repository's personal-path gate; nothing else changed.)
