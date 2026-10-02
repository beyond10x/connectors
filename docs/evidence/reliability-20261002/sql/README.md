# SQL cancellation fixture verification

Source commit: ef5a08f87f4c641325bbd46810b5aeb38666913b. Changed file SHA256:
094565444a701a196221cd902fc5ce50c447774ff31c1a6f1357b22e0ff5d23d.
Both commit author and committer are b10x-bot[bot].

The implementor reproduced valid PostgreSQL CancelRequest traffic being counted
as another authenticated session. Its package run grew from 13 to17 executed
cases; malformed cancellation, foreign startup and a second ordinary session
were independently demonstrated to fail when their expected-panic guards were
removed. The adversary added length/PID boundaries and repeated cancellation
followed by an actual read: package execution grew17 to19, no failures.
The reports retain their own exact commands and outputs.

The coordinator completed the remaining acceptance check after both reports:
50 of50 local_runtime runs passed, each10 passed,0 failed,1 ignored live-provider
case. Command per run, from the SQL managed tree:

```sh
TMPDIR="$PWD/.local/tmp" target/debug/deps/local_runtime-b43f87152464020b --test-threads=4
```

UTC interval: 2026-10-02T12:22:53Z–12:24:29Z. The separate runner tree's actual
`cargo test --workspace --no-fail-fast` began executing tests at12:22:19Z and
was still executing ordinary tests at12:25:06Z, with no intervening compilation.
Thus all50 runs overlapped actual workspace test execution. Per-run exit statuses
and timestamps are in repetitions-results.txt; complete test output is in
repetitions.log. The earlier50 runs during compilation are retained only in local
scratch and are not counted as this acceptance evidence.

This verifies the disposable wire fixture. The ignored real PostgreSQL journey
is a separate requirement and has not been credited by these results. The final
integration gate and release receipt are recorded at the parent evidence root.

The workspace load ended at 12:30:13Z with 1,187 passed, one failed and 45 ignored
cases (148 suite summaries), exit 101. The failure was the runner's new temporary
fixture directory permission assertion; its corrected package subsequently passed.
This load result is retained in workspace-load.log and is not claimed as a green
integration gate. SQL's own 50 runs all passed throughout that load.
