//! Adversary, story:mcp-stdio-on-macos-and-windows. Acceptance: "The macOS and
//! Windows CI jobs run the MCP stdio tests that run on Linux today." Both jobs
//! run `cargo test -p connectors`, so a scenario file that serves a session
//! (it sends `tools/list` to `connectors server --transport stdio`) runs there
//! only if it is not compiled for Linux alone. The two launch tests moved to
//! `mcp_stdio_launch.rs` never reach the serve loop (an absent configuration
//! is refused at `load`, before `Console::open`), so they pass unchanged
//! against the base, which refused every launch off Linux.
//!
//! Revised after the fix. The ten older serving files bind `mounted_secret`
//! (Linux-only by compile) or drive the Linux owner harness, so they stay
//! Linux-only; the first form of this test, which required none of them to be,
//! could be satisfied only by deleting that coverage. The intent is kept: at
//! least one scenario that serves a session must compile and run off Linux,
//! and the macOS and Windows workflows must run it with an exact count.
use std::{fs, path::Path};

#[test]
fn adversary_mcp_stdio_scenarios_that_serve_a_session_run_off_linux() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut serving = Vec::new();
    let mut portable = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        if !(text.contains("tools/list") && text.contains("\"stdio\"")) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let gated = text.contains("#![cfg(target_os = \"linux\")]")
            || text.contains("#![cfg(all(target_os = \"linux\"");
        if !gated {
            portable.push(name.clone());
        }
        serving.push(name);
    }
    serving.sort();
    portable.sort();
    assert!(!serving.is_empty(), "no serving scenario found in {dir:?}");
    assert!(
        !portable.is_empty(),
        "none of {} MCP stdio scenario files that serve a session compiles off Linux, \
         so the macOS and Windows jobs run none of them: {serving:?}",
        serving.len()
    );
    let workflows = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows");
    for job in ["macos-host.yml", "windows-host.yml"] {
        let text = fs::read_to_string(workflows.join(job)).unwrap();
        for file in &portable {
            let test = file.trim_end_matches(".rs");
            assert!(
                text.contains(&format!("--test {test} "))
                    && text.contains("test result: ok. 1 passed"),
                "{job} does not run the portable serving scenario {test} with an exact count"
            );
        }
    }
}
