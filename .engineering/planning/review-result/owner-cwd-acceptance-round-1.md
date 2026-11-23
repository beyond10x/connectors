---
format: aep.planning-md/3
id: review-result:owner-cwd-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic, round 1, story:owner-and-providers-run-in-root-directory
relations:
- reviews: story:owner-and-providers-run-in-root-directory
revision: 1
---
verdict: needs-revision

```findings
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 60
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 1 cannot set the CLI's directory with Command::current_dir: Jail::command (crates/connectors-custody/src/mounted/fixture.rs:152) installs a pre_exec hook whose last steps are chroot and chdir(\"/\") (fixture.rs:279-280). std 1.99.1 runs chdir(cwd) (library/std/src/sys/process/unix/unix.rs:342) before the pre_exec closures (:388), so current_dir(D) is overridden and the CLI starts in the jail's /. The test would then pass on main and its 'fails on main with the path of D' clause could never be met. Fix: name the mechanism. Create D with container.jail.directory(\"/work/D\", 0o700), then add a second pre_exec after jail.command(...) that calls chdir on it (closures run in the order added), or run sh -c 'cd /work/D && exec ...' (/bin/sh is in the jail through the usr bind)."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 63
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'/proc/<pid>/cwd ... resolves to /' is not what the test reads. The test runs outside the chroot, so readlink gives the jail's host path. reap.rs:42-45 shows the same effect for /proc/<pid>/root, which it compares to container.path(\"/\"). Fix: say each cwd link equals container.path(\"/\"), or equals that pid's own /proc/<pid>/root link."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'every process whose parent is the owner' is true for the provider but is vacuous when the set is empty. Fix: require at least one child whose cmdline holds cold_owner_start_adapter_fixture and --connectors-private-fd, and tolerate pids that vanish between listing and readlink (a transient print child, see the config.rs finding). No helper lists children: container.owners() (support/mounted.rs:299, reap.rs:21) finds only the owner, and adapters() (owner_cancel_adversary.rs:682) is private to another binary and matches by cmdline and root, not parent. The story must say a new helper is added, either in cold_owner_start.rs or in support/mounted.rs. The ppid parse (text after the last ')' in /proc/<pid>/stat) exists at parent_death_adversary.rs:22 and owner_cancel_adversary.rs:954."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 52
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Design includes launch.rs:416, which is the consumer launch, not a provider. The CLI waits for it ('Start the consumer and wait for it', launch.rs:388). It inherits the caller's stdin, stdout and stderr (:434-436), and on Linux it gets SIGTERM when the launcher dies (:442). It does not outlive the call, so the Defect does not cover it. Putting a user's foreground tool in / is a behavior change that no acceptance line observes. The contract text on its directory is Windows-only (consumer-launch.md:239-240). Fix: drop launch.rs from Design and scope, or keep it with its own acceptance line and a stated decision."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 54
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The Design misses crates/connectors-host/src/local/config.rs:2224 (Executable::print). It spawns the provider with --print-local-bootstrap, --print-profiles, --version and --print-revision-inputs (callers config.rs:1320, 1404, 1472, 1596, 1774). The CLI waits up to 10 s, with no current_dir, so a relative executable.args entry still resolves against the caller's directory there, while the owner-started provider gets /. The changelog sentence 'it resolves against /' (acceptance 4) is then false for those calls. Executable.path must be absolute (config.rs:2177-2190), but args are unchecked. Fix: state whether print also gets current_dir(\"/\") (same Command pattern, add config.rs to scope) or word the changelog to match."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 66
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'same ... stdout' is not pinned. The second call finds the first call's owner already running (warm), unless owner-stop runs between (connected() does that at cold_owner_start.rs:52). I did not establish whether raw stdout carries per-call fields. Existing tests compare the parsed result: success() (support/mounted.rs:473) returns `result`, and cold_owner_start.rs:204 compares result.credential. Fix: compare the exit code and the parsed result, and say whether both calls are cold (run owner-stop between them)."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 3 puts the providers' directory only in owner.md, which has no Unix directory text (the Detaching bullet, owner.md:498, is Windows). The providers' directory contract lives in private-adapter.md:263-266, and its scope entry is missing. Acceptance 4 (line 69) names no CHANGELOG heading: v0.76.0 is tagged and CHANGELOG.md:3 has no Unreleased section. Fix: name the owner.md section and add private-adapter.md to scope if a provider sentence goes there, and name the changelog heading the Fixed entry goes under."
- file: .engineering/planning/story/owner-and-providers-run-in-root-directory.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Acceptance 5 states the 60 s per-test bound but not the 120 s step bound. Fix: add 'the command ends within 120 s'. .config/nextest.toml:5 already kills a test at 60 s (slow-timeout 30s, terminate-after 2)."
```

Facts the story can cite:
- Q4: the std docs for `CommandExt::pre_exec` say only "just before the `exec` function is invoked" and give no order against `current_dir`. In the 1.99.1 source `do_exec` runs chdir at unix.rs:342 and the closures at :388, and `posix_spawn` is skipped when closures exist (:470). The owner's hook (transport.rs:1698) and the provider's hook (process.rs:427) have no chdir. `/proc/self/fd/N` is absolute, so exec is unaffected by the order.
- Q2: long-lived spawns are the owner (transport.rs:1612, Unix) and the provider (process.rs:406, with macOS going through ExactChild::spawn on the same Command, macos.rs:424). The CLI waits for launch.rs:416 (consumer) and config.rs:2224 (print). oauth.rs:1315 and parent_death.rs:177 are inside `#[cfg(test)]` modules (oauth.rs:713, parent_death.rs:134), and tether.rs:324 is test-only too.
- Q3: no relative-path reliance found. `rg 'current_dir|set_current_dir|chdir'` over crates/connectors-host, crates/connectors-custody/src and apps/connectors/src finds only the jail's chdir("/") (fixture.rs:280). Every `args` literal in config.rs tests and docs is absolute (`/etc/connectors/...`). The tests/support fixtures use `/work`, `/srv` and `/bin-test` inside the jail, so none depends on the cwd.
- Q1: the provider is a direct `command.spawn()` of `/proc/self/fd/N` (process.rs:406-447), with no shell, double fork or tether process. `setsid` runs only for the owner (transport.rs:1699). So ppid == owner pid. The pool keeps an idle provider after a read (supervisor.rs idle_tests, line 2556).
- Q5: correct. `CreateProcessW` at transport/windows.rs:521 passes `std::ptr::null()` as 8th argument (lpCurrentDirectory). Windows providers already get the image copy's directory (artifact/windows.rs:554, 631), so only the owner inherits the caller's.
- Budget wording: no new test binary (cold_owner_start exists), no Python, and the new test's time is to be reported. Only the 120 s step bound is missing (last finding).
- Not established: how long the new test takes (no build allowed) and whether raw stdout of two reads is byte-equal.
- Out of my lane: scope and parallel-safety. The uncommitted CHANGELOG/docs edits of the Entity Runtime pin story may collide with this story's CHANGELOG entry.
