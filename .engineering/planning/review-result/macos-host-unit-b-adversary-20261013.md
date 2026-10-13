---
format: aep.planning-md/3
id: review-result:macos-host-unit-b-adversary-20261013
kind: review-result
status: active
title: Adversary review of macOS capture, launch and owner build
relations:
- reviews: story:host-runs-on-macos
revision: 1
---
## Adversary review of macOS capture, launch and owner build

Reviewed on 2026-10-13: unit B of `story:host-runs-on-macos` (capture and control of child processes on macOS, 4da5abb0d). The cases are in commit 6c27862dd (`owner/transport.rs` and `runtime/artifact/macos.rs`, macOS only). They first ran on macOS in pull request https://github.com/beyond10x/connectors/pull/163.

| # | case | outcome | fixed by |
|---|---|---|---|
| 1 | An owner executed from its private copy, whose launch is dropped as soon as spawn returns, must still measure its build from its running executable's path. | needs change: the CLI removed the copy before the owner read it | d7d0c72d9: the CLI keeps the copy until the owner has answered the greeting |
| 2 | The `sun_path` bound at its edge: 103 bytes admitted, 104 refused as `invalid_configuration`. | test defect: the case built 104 bytes for its 103-byte path | bca3666b1, a063d3190 |
| 3 | A capture dropped without a launch must not leave its copy and directory behind. | needs change: the capture was a plain file and left its copy | c81ccd45e: an unlaunched capture removes its copy and directory |
| 4 | `launch_path` must refuse, and never remove, a file merely named like a capture. | needs change: a same-user file with a capture's name was admitted | c81ccd45e: only copies this process captured or received are launched |

All four pass on `macos-latest` in run 38386593205 on f326e69a5.

Found while the cases were made to run, recorded here and not in a new issue: on macOS a stream accepted from a non-blocking listener is itself non-blocking (bca3666b1), a peer that has closed refuses `LOCAL_PEERPID` and every socket option (d7d0c72d9), and the kernel sets `PENDIN` in restored terminal flags (69521ad5b).
