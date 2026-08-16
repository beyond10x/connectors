//! The neutral page port — the *browser* half of this driver, not the wire half.
//!
//! [`crate::cdp`] carries JSON frames; this module says what a page, an element reference and a
//! refusal are, without naming a browser product. [`crate::chromium`] is the one shipped
//! implementation.
//!
//! Keeping the two apart is what lets `cdp_v1` stay the driver word. The protocol axis answers
//! *which closed implementation speaks to the external system*, and the external system is a
//! browser reachable over the Chrome `DevTools` Protocol — which every Chromium-family browser
//! speaks, and which Brave, Chrome and Chromium are three instances of rather than three drivers.
//!
//! One implementation owns exactly one browser path. It never selects another browser as a
//! fallback, never retries a failed component through a different one, and never reaches the
//! operator's own profile.

use std::path::PathBuf;

use protocol::browser::{admit_address, AddressError, MAX_NAME_CHARACTERS};
use serde::{Deserialize, Serialize};

/// The exact components one engine resolved, retained as the operator-facing browser snapshot.
///
/// It is never surfaced to a model: the model-facing projection is `protocol::browser::PageView`,
/// which carries no path, executable, port or endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserAttestation {
    /// The engine identity recorded in evidence.
    pub engine: String,
    /// The exact executable the browser was found as — deliberately **not** canonicalized.
    ///
    /// Distribution browser launchers are wrapper scripts, and several browser and audio tools
    /// select their behavior from the name they were invoked as. Resolving the link would silently
    /// change what the program does.
    pub executable_path: String,
    /// The dedicated profile directory. Never the operator's own.
    pub user_data_dir: String,
    /// The debugging port the browser actually bound, read back from `DevToolsActivePort`.
    pub debugging_port: u16,
    /// The browser-level endpoint path the same file reported.
    pub browser_endpoint: String,
}

/// One address that has already passed every admitted bound.
///
/// It exists so an engine cannot be handed an unvalidated address: construction is the only way to
/// obtain one, and construction enforces the published grammar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageAddress {
    url: String,
}

impl PageAddress {
    /// Accept exactly one ordinary `http`/`https` web address.
    ///
    /// # Errors
    ///
    /// Refuses an empty, control-bearing or over-length address, and every scheme outside
    /// http/https — `file:`, `chrome:`, `devtools:`, `about:` and `javascript:` each turn a
    /// page-reading capability into something else entirely.
    pub fn new(url: &str) -> Result<Self, BrowserEngineError> {
        admit_address(url).map_err(|error| BrowserEngineError::AddressRefused {
            reason: address_reason(error).to_owned(),
        })?;
        Ok(Self {
            url: url.to_owned(),
        })
    }

    /// The exact admitted address.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}

const fn address_reason(error: AddressError) -> &'static str {
    match error {
        AddressError::Empty => "address is empty",
        AddressError::ControlCharacter => "address carries a control character",
        AddressError::TooLong => "address is above the admitted character bound",
        AddressError::SchemeRefused => "address is not an admitted http or https web address",
    }
}

/// One addressable element the engine observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotNode {
    /// The opaque handle a later call addresses. Valid only for the snapshot that produced it.
    pub reference: String,
    /// The element's accessibility role.
    pub role: String,
    /// The element's accessible name.
    pub name: String,
    /// The element's value, when it has one.
    pub value: Option<String>,
    /// The engine-internal node handle the reference stands for. Never leaves this crate.
    pub handle: i64,
}

/// What one page observation actually produced.
///
/// `total` is the count *before* the bound is applied, so an oversized page is reported with both
/// counts rather than silently cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageStructure {
    /// The address the page settled on.
    pub url: String,
    /// The document title.
    pub title: String,
    /// The bounded, addressable nodes.
    pub nodes: Vec<SnapshotNode>,
    /// How many addressable nodes the page had before the bound was applied.
    pub total: usize,
}

/// Every way the browser refuses.
///
/// Each variant names the exact component or bound at fault, because the operator's next action
/// differs for an absent browser, an unusable profile directory, and a page that never loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BrowserEngineError {
    /// No supported browser executable was found.
    #[error("no supported browser is available: {reason}")]
    BrowserUnavailable {
        /// Why none was admitted.
        reason: String,
    },
    /// The dedicated profile directory cannot be created or secured.
    #[error("the browser profile directory `{path}` is unusable: {reason}")]
    ProfileUnusable {
        /// The path that was tried.
        path: String,
        /// Why it was refused.
        reason: String,
    },
    /// The browser process could not be started.
    #[error("the browser could not be launched: {reason}")]
    LaunchFailed {
        /// Why the launch failed.
        reason: String,
    },
    /// No session is open, so there is nothing to observe or navigate.
    #[error("no browser session is open; open one first")]
    NotOpen,
    /// A session is already open on this Connection.
    #[error("a browser session is already open on this connection")]
    AlreadyOpen,
    /// The address failed the published grammar.
    #[error("browser address was refused: {reason}")]
    AddressRefused {
        /// Why it was refused.
        reason: String,
    },
    /// The reference is not from the most recent snapshot.
    #[error("element `{reference}` is not on the current page; snapshot again")]
    StaleReference {
        /// The reference that was offered.
        reference: String,
    },
    /// A screenshot could not be written for the operator.
    #[error("the screenshot could not be written to `{path}`: {reason}")]
    ArtifactFailed {
        /// The path that was tried.
        path: String,
        /// Why the write failed.
        reason: String,
    },
    /// The per-connection navigation budget is exhausted.
    #[error("the per-connection navigation budget of {maximum} is exhausted")]
    NavigationBudgetExhausted {
        /// The admitted budget.
        maximum: u32,
    },
    /// The per-connection screenshot budget is exhausted.
    #[error("the per-connection screenshot budget of {maximum} is exhausted")]
    ScreenshotBudgetExhausted {
        /// The admitted budget.
        maximum: u32,
    },
    /// The transport itself refused.
    #[error("browser protocol failed: {reason}")]
    Protocol {
        /// The transport's own message.
        reason: String,
    },
    /// Any other refusal from the driver itself.
    #[error("browser driver refused: {reason}")]
    Refused {
        /// Why it was refused.
        reason: String,
    },
}

impl BrowserEngineError {
    /// The stable machine-readable code carried into an operation refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::BrowserUnavailable { .. } => "browser-unavailable",
            Self::ProfileUnusable { .. } => "browser-profile-unusable",
            Self::LaunchFailed { .. } => "browser-launch-failed",
            Self::NotOpen => "browser-not-open",
            Self::AlreadyOpen => "browser-already-open",
            Self::AddressRefused { .. } => "browser-address-refused",
            Self::StaleReference { .. } => "browser-stale-reference",
            Self::ArtifactFailed { .. } => "browser-artifact-failed",
            Self::NavigationBudgetExhausted { .. } => "browser-navigation-budget-exhausted",
            Self::ScreenshotBudgetExhausted { .. } => "browser-screenshot-budget-exhausted",
            Self::Protocol { .. } => "browser-protocol-failed",
            Self::Refused { .. } => "browser-refused",
        }
    }

    /// The operator-facing action that would supply the missing component.
    ///
    /// It is returned to the operator, never to a model, and it therefore may name a package or a
    /// path.
    #[must_use]
    pub fn remediation(&self) -> Option<String> {
        match self {
            Self::BrowserUnavailable { .. } => Some(
                "install a Chromium-family browser (`brave-browser`, `google-chrome-stable` or \
                 `chromium`) or configure an absolute `executable` path on this Connection's \
                 browser route"
                    .to_owned(),
            ),
            Self::ProfileUnusable { .. } => Some(
                "configure an absolute, writable `user_data_dir` on this Connection's browser \
                 route. It must be a directory dedicated to this deployment, never the operator's \
                 own browser profile"
                    .to_owned(),
            ),
            Self::StaleReference { .. } => {
                Some("take a fresh snapshot and address the node it returns".to_owned())
            }
            _ => None,
        }
    }
}

impl From<crate::cdp::CdpError> for BrowserEngineError {
    fn from(error: crate::cdp::CdpError) -> Self {
        Self::Protocol {
            reason: error.to_string(),
        }
    }
}

/// One browser session held across calls.
///
/// The lifecycle is the point: `open` acquires the lease, `goto`/`snapshot`/`screenshot` act inside
/// it, and `close` releases it while **retaining** the profile directory.
pub trait BrowserEngine: Send {
    /// The exact engine identity retained in evidence.
    fn id(&self) -> &'static str;

    /// Whether a session is currently held.
    fn is_open(&self) -> bool;

    /// Launch the dedicated profile and attach to its first page.
    ///
    /// # Errors
    ///
    /// Refuses an absent browser, an unusable profile directory, a browser that never reports a
    /// debugging port, and an endpoint that exposes no page.
    fn open(&mut self) -> Result<BrowserAttestation, BrowserEngineError>;

    /// Navigate the attached page and wait for its load event.
    ///
    /// # Errors
    ///
    /// Refuses a closed session and a navigation that never loads. The address was already
    /// admitted; this cannot receive an unadmitted one.
    fn goto(&mut self, address: &PageAddress) -> Result<(), BrowserEngineError>;

    /// Read the current page as a bounded structure.
    ///
    /// # Errors
    ///
    /// Refuses a closed session or a protocol failure.
    fn snapshot(&mut self) -> Result<PageStructure, BrowserEngineError>;

    /// Capture the visible page as PNG bytes.
    ///
    /// The engine does not choose where the image lands: the artifact directory is a
    /// deployment-owned fact the driver holds.
    ///
    /// # Errors
    ///
    /// Refuses a closed session or a protocol failure.
    fn screenshot(&mut self) -> Result<Vec<u8>, BrowserEngineError>;

    /// Stop the browser, leaving the dedicated profile directory intact.
    ///
    /// The profile survives on purpose: a site the operator logged into once inside the dedicated
    /// profile stays logged in for the next session.
    fn close(&mut self);
}

/// Truncate visibly rather than silently, counting characters and not bytes.
pub(crate) fn truncate(value: &str, characters: usize) -> String {
    if value.chars().count() <= characters {
        return value.to_owned();
    }
    let kept: String = value.chars().take(characters).collect();
    format!("{kept}…")
}

/// One executable regular file at exactly the path given.
///
/// **The link is deliberately not resolved.** `/usr/bin/brave` is a shell wrapper, and multi-call
/// binaries elsewhere on this machine select their behavior from `argv[0]`; executing the canonical
/// target instead changes what the program does while still starting successfully.
pub(crate) fn executable_at(path: &std::path::Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    // `fs::metadata` follows symlinks for the existence and permission check, which is correct.
    // The *executed* path stays the one that was found.
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return None;
    }
    Some(path.to_path_buf())
}

/// The published per-node name bound, re-exported where the projection uses it.
pub(crate) const NAME_BOUND: usize = MAX_NAME_CHARACTERS;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unadmitted_address_cannot_be_constructed_at_all() {
        for refused in [
            "file:///etc/passwd",
            "chrome://settings",
            "devtools://devtools/bundled/x.html",
            "javascript:alert(1)",
            "about:blank",
            "ftp://example.test",
        ] {
            let error = PageAddress::new(refused).expect_err("admitted");
            assert_eq!(
                error.code(),
                "browser-address-refused",
                "admitted {refused}"
            );
        }
        assert_eq!(
            PageAddress::new("HTTPS://Example.test/")
                .expect("scheme case is not significant")
                .url(),
            "HTTPS://Example.test/"
        );
    }

    #[test]
    fn a_long_name_is_truncated_visibly_rather_than_silently() {
        let truncated = truncate(&"a".repeat(NAME_BOUND + 10), NAME_BOUND);
        assert!(truncated.ends_with('…'));
        assert_eq!(truncated.chars().count(), NAME_BOUND + 1);
    }

    #[test]
    fn a_missing_browser_and_an_unusable_profile_each_carry_their_own_remediation() {
        let absent = BrowserEngineError::BrowserUnavailable {
            reason: "none on PATH".to_owned(),
        };
        assert_eq!(absent.code(), "browser-unavailable");
        assert!(absent
            .remediation()
            .is_some_and(|text| text.contains("brave-browser")));

        let stale = BrowserEngineError::StaleReference {
            reference: "e1".to_owned(),
        };
        assert!(stale
            .remediation()
            .is_some_and(|text| text.contains("fresh snapshot")));

        assert!(BrowserEngineError::NotOpen.remediation().is_none());
    }

    #[test]
    fn a_directory_is_never_an_admitted_executable() {
        let directory = tempfile::tempdir().expect("temp dir");
        assert!(executable_at(directory.path()).is_none());
        assert!(executable_at(&directory.path().join("absent")).is_none());
    }
}
