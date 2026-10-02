# Integrated verification

Candidate: `7e6a33db463a581cb171c20215f98662b401bc1c`.
Tree: `e6cc76674fefa57507a1642d4ba83544dd1ad5fc`.
PR: https://github.com/beyond10x/connectors/pull/78.

`cargo run --locked -p connectors-build -- gate --msrv` exited 0 on 2026-10-02.
[Full gate output](gate.log) retains each step's own status: formatting, descriptor
and generated-output drift, workspace tests/Clippy, package boundaries, Rust 1.88
independent-library checks, Rust 1.91 workspace checks, specification synthesis,
metadata conformance and planning validation. The metadata authority ran 289
scenarios with 21 reported synthesis refusals; those refusals are retained, not
claimed as executed coverage. The gate concludes `gate: all checks passed`.

The first gate stopped at an existing source-citation assertion after the bridge
edit moved the cited cursor-removal branch. [Its failure](gate-first-citation-failure.log)
is retained. Updating only the ESS comment restored all five focused citation
cases; compiled ESS JSON before/after was byte-identical. No assertion was weakened.

`npm run build` exited 0 and audited 482 public files; its [output](website-build.log)
is retained. Earlier typecheck passed on unchanged TypeScript inputs. The final
citation correction changes no compiled model or rendered semantics. PR documentation
validation subsequently passed on the exact candidate.

[Planning validation](planning-validation.log) is complete, including historical
review warnings. Public logs have the same disclosed home-prefix-only redaction
as the unit reports. Raw logs remain private in the publication worktree until its
recovery archive is finalized. This receipt records local verification; source
release remains pending until the remaining PR gate, merge, exact tag and release
page are verified.

## Source release verified

[v0.25.0](https://github.com/beyond10x/connectors/releases/tag/v0.25.0) was published
at 2026-10-02T13:50:25Z by `b10x-bot[bot]` and verified as Latest. All four PR78
checks passed before the bot App merged it. Merge commit
`b1f432766057fb8b7db60736363926cc44fe0db8` has the exact candidate tree above.
Annotated tag object `16b1464c24f14021485dcf012dc81e9257657067` has the bot tagger,
peels to that merge commit and matches the remote. The
[tag source gate](https://github.com/beyond10x/connectors/actions/runs/37015553177)
passed. Both GitHub-generated source archives returned HTTP 200; this repository
requires no binary assets. Documentation publication remains pending.
