//! **Nothing this product prints or publishes teaches a path this release moved.**
//!
//! `story:cli-first-level-groups` moved every first-level word of `connectors` into one of eight
//! groups and left `MOVED` in `crates/connectors-cli/src/lib.rs` rewriting the old paths for one
//! release. A message, a `--help` line, a `next:` hint, a README line, a guide's transcript, an
//! image's `CMD` or an installer's parting sentence that still names one of them is advice with a
//! removal date on it, and the person reading it has no way to know that.
//!
//! The class is the point, not any one message. Four of these were found by running
//! `crates/connectors-cli/tests/first_level_groups.rs` — `connectors doctor` printed *"this
//! installation has a problem `connectors doctor` named above"* from its new path — six more by
//! reading the sources, and ten more by reading what ships beside them: the first version of this
//! file read `.rs` files under `crates/*/src` and nothing else, so `Dockerfile` started every hosted
//! container on `serve-hosted`, `Taskfile.yaml` sent every new installer to `connectors init &&
//! connectors doctor`, and `README.md` and two guides taught eight more. All of it is read now,
//! and so is `scripts/`, where the pass after that found a guard's header comment naming
//! `connectors serve-hosted`: a script is what a machine runs, its comments are what the person
//! running it reads first, and it is read whole, as `Taskfile.yaml` is.
//!
//! **Whether a written invocation names a path that moved is asked of the shim itself.**
//! `connectors_cli::moved` is the function `run_from` applies to argv, so a string it rewrites is
//! one the binary would rewrite, and a string it leaves alone is one the binary reads as its own —
//! `connectors serve local`, `connectors help serve`, `connectors -o json inspect doctor`. Nothing
//! here restates its rules, so nothing here can drift from them: the first version of this file did
//! restate them, and `connectors -o compact doctor` shipped until an adversary pass found it.
//!
//! **What is not read, and why.** `CHANGELOG.md` is the record of what each release said, and its
//! released entries name commands by the names they had. `docs/stories/S-*.md` are the dated records
//! the planning store was migrated from (`CHANGELOG.md`, 0.5.8), kept verbatim with a backlink
//! each. Editing either would falsify it. Everything else under `docs/` is read.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn visit(path: &Path, keep: &dyn Fn(&Path) -> bool, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("target") {
                visit(&path, keep, found);
            }
        } else if keep(&path) {
            found.push(path);
        }
    }
}

/// The `ENTRYPOINT` and `CMD` of a Dockerfile as the one line a container start runs, when the
/// entrypoint is this binary. `CMD ["serve-hosted", "--config", …]` does not itself say
/// `connectors`, which is how it escaped a search for that word. One output line per input line,
/// so a report names the `CMD` line.
fn dockerfile_invocations(source: &str) -> String {
    fn words(line: &str) -> Vec<&str> {
        line.split('"').skip(1).step_by(2).collect()
    }
    let mut entrypoint: Vec<&str> = Vec::new();
    let mut out = String::new();
    for line in source.lines() {
        if let Some(rest) = line.strip_prefix("ENTRYPOINT ") {
            entrypoint = words(rest);
        }
        if let Some(rest) = line.strip_prefix("CMD ") {
            let is_this_binary = entrypoint.first().is_some_and(|binary| {
                Path::new(binary)
                    .file_name()
                    .is_some_and(|name| name == "connectors")
            });
            if is_this_binary {
                let mut invocation = vec!["connectors"];
                invocation.extend(entrypoint.iter().skip(1));
                invocation.extend(words(rest));
                out.push_str(&invocation.join(" "));
            }
        }
        out.push('\n');
    }
    out
}

/// Every file a person reads or a machine runs, with its shipped text.
///
/// A `.rs` file under a crate's `src`, with its `#[cfg(test)]` module cut off — an assertion
/// message inside a test module tells nobody anything, and cutting at the marker is what
/// `crates/connectors-cli/tests/cli_surface.rs` does with the same file for the same reason.
/// `README.md`, `Taskfile.yaml`, every `.md` under `docs/` bar the story records, every file under
/// `scripts/`, and `Dockerfile` through [`dockerfile_invocations`].
fn shipped_text() -> Vec<(String, String)> {
    let root = repository_root();
    let mut files = Vec::new();
    visit(
        &root.join("crates"),
        &|path| {
            path.extension().and_then(|name| name.to_str()) == Some("rs")
                && path
                    .components()
                    .any(|component| component.as_os_str() == "src")
                && !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("_tests.rs"))
        },
        &mut files,
    );
    let sources = files.len();
    assert!(
        sources > 50,
        "only {sources} shipped sources were found under crates/; the layout moved"
    );
    let stories = root.join("docs/stories");
    visit(
        &root.join("docs"),
        &|path| {
            path.extension().and_then(|name| name.to_str()) == Some("md")
                && !path.starts_with(&stories)
        },
        &mut files,
    );
    assert!(
        files.len() > sources + 20,
        "only {} documents were found under docs/; the layout moved",
        files.len() - sources
    );
    let documents = files.len();
    visit(&root.join("scripts"), &|_| true, &mut files);
    assert!(
        files.len() > documents + 5,
        "only {} files were found under scripts/; the layout moved",
        files.len() - documents
    );
    for named in ["README.md", "Taskfile.yaml", "Dockerfile"] {
        let path = root.join(named);
        assert!(
            path.is_file(),
            "{named} is not at the repository root; it moved"
        );
        files.push(path);
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let source = read(&path);
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let text = if relative.ends_with(".rs") {
                source
                    .split("\n#[cfg(test)]")
                    .next()
                    .unwrap_or("")
                    .to_owned()
            } else if relative == "Dockerfile" {
                dockerfile_invocations(&source)
            } else {
                source
            };
            (relative, text)
        })
        .collect()
}

/// One written invocation as argv, `argv[0]` included.
///
/// A trailing run of punctuation comes off each word, because these are written inside backticks
/// and beside pipes far more often than not, and an emptied word — a `&&`, a `\`, a `>` — is
/// dropped. What the words mean is not decided here; the shim decides that.
fn argv(rest: &str) -> Vec<OsString> {
    std::iter::once("connectors")
        .chain(rest.split_whitespace().map(|token| {
            token.trim_end_matches(|character: char| {
                !(character.is_ascii_alphanumeric() || character == '-')
            })
        }))
        .filter(|word| !word.is_empty())
        .map(OsString::from)
        .collect()
}

/// **Every site that names a path this release moved.**
#[test]
fn nothing_this_product_prints_names_a_path_that_moved() {
    let mut teaching = Vec::new();
    let mut invocations = 0usize;

    for (file, text) in shipped_text() {
        let rust = file.ends_with(".rs");
        for (number, line) in text.lines().enumerate() {
            let head = line.trim_start();
            // A plain comment and a module header are read by whoever is editing the file; a `///`
            // doc comment on a Clap field is `--help`, so it is in.
            if rust
                && (head.starts_with("//!") || (head.starts_with("//") && !head.starts_with("///")))
            {
                continue;
            }
            for (at, _) in line.match_indices("connectors ") {
                invocations += 1;
                let mut written = argv(&line[at + "connectors ".len()..]);
                if let Some((old, new)) = connectors_cli::moved(&mut written) {
                    teaching.push(format!(
                        "{file}:{}: {}\n      `connectors {}` is `connectors {}` now",
                        number + 1,
                        line.trim(),
                        old.join(" "),
                        new.join(" ")
                    ));
                }
            }
        }
    }

    assert!(
        invocations > 100,
        "only {invocations} written invocations of `connectors` were found; the scan is reading \
         the wrong thing"
    );
    assert!(
        teaching.is_empty(),
        "these name a path `MOVED` rewrites, so they teach a path that is removed one release from \
         now:\n  {}\nName the path this release carries. `connectors_cli::moved` is what decided \
         each of these, so an entry added to the table brings its own sites with it.",
        teaching.join("\n  ")
    );
}
