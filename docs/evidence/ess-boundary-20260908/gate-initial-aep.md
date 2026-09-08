needs-revision

Independent read-only review of the initial ESS boundary gate, its CLI/repository-gate wiring, terminology policy and adapter-model ownership documentation. No tracked files or planning artifacts were changed. I used isolated fixtures with the existing connectors-build binary and pinned ESS 0.20.0; I did not run the full repository gate.

ESS-GATE-01 — major (P1), story:shared-ess-provider-boundary — crates/connectors-build/src/ess_boundary.rs:50; contracts/ess-boundary.json:4.

Known provider names in ordinary CamelCase bypass the central check. The policy tokenizes `gitlab`, `github`, `openai` and `logql` as one word, while the source tokenizer splits `GitLabTarget`, `GitHubTarget`, `OpenAITarget` and `LogQLTarget` into multiple words. `matching_term` only compares equal-length word sequences, so it cannot recognize these spellings. This is an enforcement defect for already-known vocabulary, rather than the documented limit concerning unfamiliar terminology. Raw text, paths and decoded YAML all use the same matcher and inherit the defect.

Reproduction: `.local/ess-boundary-20260908/reviewer-gate-fixture/` initially copied the shared ESS and four authored adapter models, declared a new `connectors.boundary_fixture` domain containing all four provider-named enums above, and supplied a minimal adapter specification. Running `target/debug/connectors-build --root .local/ess-boundary-20260908/reviewer-gate-fixture --ess /home/timo/beyond10x/connectors_v2/.local/toolchains/ess/0.20.0/bin/ess ess-boundary` accepted the lexical/structural check and validated and compiled every model. The retained initial output is `.local/ess-boundary-20260908/reviewer-gate-camelcase.log`; the shared model compiled 226 declarations. The fixture was subsequently changed for separate structural checks, so the retained log describes its initial state.

Normalize known aliases and adjacent complete source words consistently, preserving word boundaries so `cargo` does not match `argo`. Add negative cases for conventional provider casing and acronyms, including dynamically discovered adapter identifiers, and retain positive cases for legitimate shared protocols and unrelated words. The root was notified before this report and is preparing a correction; this report records the independently observed initial defect and does not approve an unreviewed correction.

Other checks: the CLI has a dedicated boundary command and the full gate invokes the same implementation before the Rust checks. Inspection found explicit rejection of symlinked model entries, nested/unlisted YAML, duplicate/mismatched domain declarations and foreign domain namespaces. An isolated `.yml` domain was both inventoried and compiled successfully. A separate adapter fixture referencing a shared-root type was correctly refused by ESS as `undeclared_reference`; output is `.local/ess-boundary-20260908/reviewer-gate-cross-root.log`. I found no additional actionable defect in the reviewed structural enforcement or documented limitations. This review does not validate provider behavior, public codecs or the broader datasource drafts.

```findings
- file: crates/connectors-build/src/ess_boundary.rs
  line: 50
  category: correctness
  severity: major
  verdict: needs-revision
  origin: new
  message: "ESS-GATE-01 (P1): Known aliases gitlab, github, openai and logql do not match conventional CamelCase names GitLabTarget, GitHubTarget, OpenAITarget and LogQLTarget. The actual boundary command accepts and compiles all four shared types. Normalize complete word sequences consistently and add regression cases for known and dynamically discovered names without substring false positives."
```
