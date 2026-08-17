//! The one shipped [`BrowserEngine`]: a Chromium-family browser on a dedicated profile.
//!
//! This is the only module that names a browser product, and it names three because Brave, Chrome
//! and Chromium are one protocol implementation with three package names, not three drivers.
//!
//! Two facts about launching a browser here are load-bearing, and both fail *silently* when broken:
//!
//! 1. **A resolved executable path is never canonicalized.** `/usr/bin/brave` is a shell wrapper,
//!    and multi-call binaries elsewhere select their behavior from `argv[0]`. Executing the
//!    canonical target instead changes what the program does while still starting.
//! 2. **The launch flag is never trusted.** `/usr/bin/brave` `exec`s the real binary with `"$@"`
//!    followed by the operator's own `brave-flags.conf` entries — *user flags are appended after
//!    ours* — so a supplied `--remote-debugging-port` can be overridden. The port is requested as
//!    `0` and read back from `DevToolsActivePort`, which is immune to flag ordering.
//!
//! Both are pinned by tests here and in [`crate::cdp`].

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::{json, Value};

use crate::cdp::{first_page_target, read_active_port, CdpClient, CdpError};
use crate::page::{
    executable_at, truncate, BrowserAttestation, BrowserEngine, BrowserEngineError, PageAddress,
    PageStructure, SnapshotNode, NAME_BOUND,
};

/// The engine identity recorded in evidence.
pub const ENGINE_ID: &str = "chromium-devtools";

/// Browser executables considered, in order. Brave is preferred: it is the one that ships with
/// tracker and advertisement blocking on by default, so an agent-visited page carries less of a
/// third party's script than the same page in the alternatives.
pub const BROWSER_CANDIDATES: [&str; 7] = [
    "brave-browser",
    "brave",
    "brave-browser-stable",
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
];

/// How long a launched browser has to report a debugging port and expose a page.
const LAUNCH_BUDGET: Duration = Duration::from_secs(30);

/// How long one ordinary protocol command may take.
const CALL_BUDGET: Duration = Duration::from_secs(30);

/// The bound on one evaluated string, in characters.
const MAX_TEXT_CHARACTERS: usize = 96 * 1024;

/// The deployment-owned facts one engine was built from.
///
/// Nothing a caller supplied reaches this: the address travels separately and is admitted again on
/// the way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChromiumConfig {
    /// An explicit absolute browser executable. `None` walks [`BROWSER_CANDIDATES`] on `PATH`.
    pub executable: Option<PathBuf>,
    /// The dedicated profile directory. Never the operator's own.
    pub user_data_dir: PathBuf,
    /// The admitted per-snapshot node bound.
    pub maximum_nodes: usize,
    /// The admitted wall-clock bound on one navigation.
    pub maximum_navigation: Duration,
}

/// One browser session on a dedicated profile, driven over the `DevTools` protocol.
pub struct ChromiumBrowserEngine {
    config: ChromiumConfig,
    child: Option<Child>,
    client: Option<CdpClient>,
    attestation: Option<BrowserAttestation>,
}

impl ChromiumBrowserEngine {
    /// Build the engine from deployment-owned facts. Construction resolves no browser.
    #[must_use]
    pub const fn new(config: ChromiumConfig) -> Self {
        Self {
            config,
            child: None,
            client: None,
            attestation: None,
        }
    }

    /// The browser snapshot this engine resolved, once it has opened.
    #[must_use]
    pub const fn attestation(&self) -> Option<&BrowserAttestation> {
        self.attestation.as_ref()
    }

    /// Resolve the browser executable without launching it.
    ///
    /// # Errors
    ///
    /// Refuses when no candidate is an executable regular file.
    pub fn resolve_executable(&self) -> Result<PathBuf, BrowserEngineError> {
        if let Some(explicit) = &self.config.executable {
            return executable_at(explicit).ok_or_else(|| BrowserEngineError::BrowserUnavailable {
                reason: format!("`{}` is not an executable regular file", explicit.display()),
            });
        }
        BROWSER_CANDIDATES
            .iter()
            .find_map(|name| discover_executable(name))
            .ok_or_else(|| BrowserEngineError::BrowserUnavailable {
                reason: format!("none of {} is on PATH", BROWSER_CANDIDATES.join(", ")),
            })
    }

    fn attach(&self) -> Result<(CdpClient, u16, String), BrowserEngineError> {
        let (port, endpoint) = read_active_port(&self.config.user_data_dir, LAUNCH_BUDGET)?;
        let target = first_page_target(port, LAUNCH_BUDGET)?;
        Ok((CdpClient::connect(&target)?, port, endpoint))
    }

    fn terminate(&mut self) {
        self.client = None;
        self.attestation = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl BrowserEngine for ChromiumBrowserEngine {
    fn id(&self) -> &'static str {
        ENGINE_ID
    }

    fn is_open(&self) -> bool {
        self.child.is_some()
    }

    fn open(&mut self) -> Result<BrowserAttestation, BrowserEngineError> {
        if self.is_open() {
            return Err(BrowserEngineError::AlreadyOpen);
        }
        let executable = self.resolve_executable()?;
        prepare_directory(&self.config.user_data_dir)?;

        let child = Command::new(&executable)
            .arg(format!(
                "--user-data-dir={}",
                self.config.user_data_dir.display()
            ))
            // Requested as 0 and read back from DevToolsActivePort, because a distribution
            // launcher appends the operator's own flags after ours and could override this one.
            .arg("--remote-debugging-port=0")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| BrowserEngineError::LaunchFailed {
                reason: error.to_string(),
            })?;
        self.child = Some(child);

        let attached = self.attach();
        let (mut client, port, endpoint) = match attached {
            Ok(attached) => attached,
            Err(error) => {
                self.terminate();
                return Err(error);
            }
        };
        client.call("Page.enable", &json!({}), CALL_BUDGET)?;
        client.call("Runtime.enable", &json!({}), CALL_BUDGET)?;
        client.call("Accessibility.enable", &json!({}), CALL_BUDGET)?;
        self.client = Some(client);

        let attestation = BrowserAttestation {
            engine: ENGINE_ID.to_owned(),
            executable_path: executable.display().to_string(),
            user_data_dir: self.config.user_data_dir.display().to_string(),
            debugging_port: port,
            browser_endpoint: endpoint,
        };
        self.attestation = Some(attestation.clone());
        Ok(attestation)
    }

    fn goto(&mut self, address: &PageAddress) -> Result<(), BrowserEngineError> {
        let budget = self.config.maximum_navigation;
        let client = self.client.as_mut().ok_or(BrowserEngineError::NotOpen)?;
        client.call_awaiting_event(
            "Page.navigate",
            &json!({ "url": address.url() }),
            "Page.loadEventFired",
            budget,
        )?;
        Ok(())
    }

    fn snapshot(&mut self) -> Result<PageStructure, BrowserEngineError> {
        let maximum_nodes = self.config.maximum_nodes;
        let client = self.client.as_mut().ok_or(BrowserEngineError::NotOpen)?;
        let url = evaluate_string(client, "location.href").unwrap_or_default();
        let title = evaluate_string(client, "document.title").unwrap_or_default();
        let tree = client.call("Accessibility.getFullAXTree", &json!({}), CALL_BUDGET)?;
        let (nodes, total) = project_tree(&tree, maximum_nodes);
        Ok(PageStructure {
            url,
            title,
            nodes,
            total,
        })
    }

    fn screenshot(&mut self) -> Result<Vec<u8>, BrowserEngineError> {
        use base64::Engine as _;
        let client = self.client.as_mut().ok_or(BrowserEngineError::NotOpen)?;
        let result = client.call(
            "Page.captureScreenshot",
            &json!({"format": "png"}),
            CALL_BUDGET,
        )?;
        let encoded = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| CdpError::Protocol("screenshot carried no data".to_owned()))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| CdpError::Protocol(error.to_string()))?;
        Ok(bytes)
    }

    fn close(&mut self) {
        if let Some(client) = &mut self.client {
            let _ = client.call("Browser.close", &json!({}), Duration::from_secs(5));
        }
        self.terminate();
    }
}

impl Drop for ChromiumBrowserEngine {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Turn the raw accessibility tree into bounded, addressable nodes.
///
/// Ignored and nameless nodes are dropped: they are invisible to a person using the page and would
/// spend the result bound without telling a model anything actionable. The returned count is the
/// number that *survived that filter*, before the bound — so truncation is reported honestly rather
/// than as a page that happened to be exactly the maximum size.
fn project_tree(tree: &Value, maximum_nodes: usize) -> (Vec<SnapshotNode>, usize) {
    let mut nodes = Vec::new();
    let mut total = 0_usize;
    let empty = Vec::new();
    let raw = tree
        .get("nodes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    for node in raw {
        if node.get("ignored").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let role = node
            .get("role")
            .and_then(|role| role.get("value"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let name = node
            .get("name")
            .and_then(|name| name.get("value"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        if role.is_empty() || (name.is_empty() && role != "textbox") {
            continue;
        }
        let Some(handle) = node.get("backendDOMNodeId").and_then(Value::as_i64) else {
            continue;
        };
        total = total.saturating_add(1);
        if nodes.len() >= maximum_nodes {
            continue;
        }
        nodes.push(SnapshotNode {
            reference: format!("e{}", nodes.len() + 1),
            role: role.to_owned(),
            name: truncate(name, NAME_BOUND),
            value: node
                .get("value")
                .and_then(|value| value.get("value"))
                .and_then(Value::as_str)
                .map(|value| truncate(value, NAME_BOUND)),
            handle,
        });
    }
    (nodes, total)
}

fn evaluate_string(client: &mut CdpClient, expression: &str) -> Option<String> {
    let result = client
        .call(
            "Runtime.evaluate",
            &json!({"expression": expression, "returnByValue": true}),
            CALL_BUDGET,
        )
        .ok()?;
    let value = result
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_str)?;
    Some(truncate(value, MAX_TEXT_CHARACTERS))
}

fn prepare_directory(path: &Path) -> Result<(), BrowserEngineError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::create_dir_all(path).map_err(|error| BrowserEngineError::ProfileUnusable {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
        BrowserEngineError::ProfileUnusable {
            path: path.display().to_string(),
            reason: error.to_string(),
        }
    })
}

fn discover_executable(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|directory| directory.is_absolute())
        .find_map(|directory| executable_at(&directory.join(name)))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::*;

    fn ax_node(id: i64, role: &str, name: &str) -> Value {
        json!({
            "backendDOMNodeId": id,
            "ignored": false,
            "role": {"value": role},
            "name": {"value": name},
        })
    }

    fn engine(executable: Option<PathBuf>) -> ChromiumBrowserEngine {
        ChromiumBrowserEngine::new(ChromiumConfig {
            executable,
            user_data_dir: PathBuf::from("/nonexistent/b10x-browser-profile"),
            maximum_nodes: 400,
            maximum_navigation: Duration::from_secs(45),
        })
    }

    #[test]
    fn a_snapshot_addresses_nodes_and_reports_its_own_truncation() {
        let raw: Vec<Value> = (0..425)
            .map(|index| ax_node(index, "button", &format!("Button {index}")))
            .collect();
        let (nodes, total) = project_tree(&json!({ "nodes": raw }), 400);
        assert_eq!(nodes.len(), 400);
        assert_eq!(
            total, 425,
            "truncation must report the count before the bound"
        );
        assert_eq!(nodes[0].reference, "e1");
        assert_eq!(nodes[0].handle, 0);
        assert_eq!(nodes[399].reference, "e400");
    }

    #[test]
    fn ignored_and_nameless_nodes_do_not_spend_the_result_bound() {
        let raw = json!({"nodes": [
            {"backendDOMNodeId": 1, "ignored": true, "role": {"value": "button"},
             "name": {"value": "Hidden"}},
            {"backendDOMNodeId": 2, "ignored": false, "role": {"value": "generic"},
             "name": {"value": ""}},
            ax_node(3, "link", "Documentation"),
        ]});
        let (nodes, total) = project_tree(&raw, 400);
        assert_eq!(total, 1);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].role, "link");
        assert_eq!(nodes[0].name, "Documentation");
    }

    #[test]
    fn a_named_node_is_truncated_visibly_rather_than_silently() {
        let raw = json!({"nodes": [ax_node(1, "button", &"n".repeat(NAME_BOUND * 2))]});
        let (nodes, _total) = project_tree(&raw, 400);
        assert!(nodes[0].name.ends_with('…'));
        assert_eq!(nodes[0].name.chars().count(), NAME_BOUND + 1);
    }

    #[test]
    fn an_absent_browser_refuses_by_candidate_list_rather_than_hanging() {
        let error = engine(Some(PathBuf::from("/nonexistent/brave")))
            .resolve_executable()
            .expect_err("absent browser");
        assert_eq!(error.code(), "browser-unavailable");
    }

    #[test]
    fn brave_is_preferred_over_chrome_and_chromium() {
        assert_eq!(BROWSER_CANDIDATES[0], "brave-browser");
        let brave = BROWSER_CANDIDATES
            .iter()
            .position(|name| *name == "brave")
            .expect("brave");
        let chrome = BROWSER_CANDIDATES
            .iter()
            .position(|name| *name == "google-chrome")
            .expect("chrome");
        assert!(brave < chrome);
    }

    /// **A wrapper launcher must be executed under the name it was found as.**
    ///
    /// `/usr/bin/brave` is a shell wrapper that appends the operator's own flags after ours, and
    /// multi-call binaries select their behavior from `argv[0]`. Resolving the link to its target
    /// would silently change which program runs and which flags win.
    #[test]
    fn a_wrapper_launcher_is_never_canonicalized_to_its_target() {
        let directory = tempfile::tempdir().expect("temp dir");
        let target = directory.path().join("brave-real");
        fs::write(&target, "#!/bin/sh\nexit 0\n").expect("write target");
        fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("mode");
        let wrapper = directory.path().join("brave");
        std::os::unix::fs::symlink(&target, &wrapper).expect("symlink");

        let resolved = engine(Some(wrapper.clone()))
            .resolve_executable()
            .expect("the wrapper is executable");
        assert_eq!(resolved, wrapper, "the executed path was canonicalized");
        assert_ne!(resolved, target);
    }

    #[test]
    fn a_closed_session_refuses_every_page_operation() {
        let mut engine = engine(None);
        assert!(!engine.is_open());
        assert_eq!(
            engine.snapshot().expect_err("closed").code(),
            "browser-not-open"
        );
        assert_eq!(
            engine.screenshot().expect_err("closed").code(),
            "browser-not-open"
        );
        let address = PageAddress::new("http://example.test/").expect("address");
        assert_eq!(
            engine.goto(&address).expect_err("closed").code(),
            "browser-not-open"
        );
    }
}
