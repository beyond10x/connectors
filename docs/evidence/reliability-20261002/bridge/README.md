# Metadata timeout ownership verification

Source commit: 91511f4379579d0605af80420d5c34ea4874d162, authored and committed
by b10x-bot[bot]. The source retains the existing 30-second production bridge
bound, pool of verified authorities, corruption checks and reopen verification.

The implementor first ran four failing ownership cases, then made all four pass:
dispatched timeout, queued cancellation, recovery without another provider effect,
and legacy import before the handle installs its authority. The package grew
from 329 to 333 executed cases, with 26 ignored; Clippy and formatting passed.
Legacy import was found during coordinator review and fixed within the same scope.

The adversary added caller-unwind and pre-dispatch-rejection cases. Both passed
on their first focused runs; the full package ran 335 cases with 26 ignored and
no failures. The unwind case was then strengthened to require the exact injected
panic payload and rerun successfully. Full outputs are retained in the reports.

A refused authority transfers its lock to a cleanup thread. The lock is released
only after upstream ShutdownOutcome::Joined proves thread/runtime termination;
caller Drop performs no potentially unbounded shutdown. Spawn failure or cleanup
panic deliberately retains the holder until process exit. Those two rare branches
were inspected structurally, not fault-injected; the reports state that limit.

This change prevents a late metadata commit from racing a new owner. It does not
make an unknown result successful or solve the growing-store performance problem.
Final integrated gate and release evidence are recorded at the parent root.
