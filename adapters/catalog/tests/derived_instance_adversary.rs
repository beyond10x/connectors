//! Adversary pass on story:connector-instance-derived-fields: an unpinned
//! `[connectors.<c>.instances.<i>]` derived by the host from the catalog
//! provider installed in `CONNECTORS_INSTALLATION`, here the real provider
//! binary (or a stand-in script) copied into a private directory.
//!
//! `CONNECTORS_INSTALLATION` is process-wide, so every read holds `ENV`.
#![cfg(unix)]
use connectors_host::local::{
    Failure,
    config::{Config, Paths},
    filesystem,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

static ENV: Mutex<()> = Mutex::new(());

struct Installation {
    root: tempfile::TempDir,
}

impl Installation {
    fn with(write: impl FnOnce(&Path)) -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        let provider = bin.join("connectors-catalog-provider");
        write(&provider);
        fs::set_permissions(&provider, fs::Permissions::from_mode(0o700)).unwrap();
        Self { root }
    }

    /// The production catalog provider.
    fn real() -> Self {
        Self::with(|provider| {
            fs::copy(env!("CARGO_BIN_EXE_connectors-catalog-provider"), provider).unwrap();
        })
    }

    fn provider(&self) -> std::path::PathBuf {
        self.root.path().join("bin/connectors-catalog-provider")
    }

    fn read(&self, body: &str) -> Result<Config, Failure> {
        let _guard = ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        // SAFETY: every reader and writer of the environment in this test
        // binary holds `ENV`.
        unsafe { std::env::set_var("CONNECTORS_INSTALLATION", self.root.path().join("bin")) };
        let paths = Paths::resolve(
            Some(&self.root.path().join("config/config.toml")),
            Some(&self.root.path().join("state")),
        )
        .unwrap();
        if !paths.config.exists() {
            Config::initialize(&paths).unwrap();
        }
        fs::write(
            &paths.config,
            format!(
                "format='connectors-local/5'\nowner_uid={}\n{body}",
                filesystem::uid()
            ),
        )
        .unwrap();
        Config::load(&paths.config)
    }
}

/// Control: the setup below derives a Gmail instance from the real provider,
/// writes included, so a refusal elsewhere is about that connector.
#[test]
fn control_a_gmail_instance_named_alone_derives_from_the_real_provider() {
    let installation = Installation::real();
    let config = installation
        .read("[connectors.gmail]\nenabled=true\n[connectors.gmail.instances.private]\n")
        .expect("the real provider derives gmail.private");
    let entry = &config.adapters["gmail.private"];
    assert_eq!(entry.instance_id, "gmail.private");
    assert_eq!(
        entry.executable.sha256,
        hex::encode(Sha256::digest(fs::read(installation.provider()).unwrap()))
    );
    assert!(
        entry.permissions.operations.contains("users.drafts.send"),
        "{:?}",
        entry.permissions.operations
    );
}

/// story:connector-instance-derived-fields § Change: "`endpoint` absent uses
/// the connector's default endpoint; the auth profile is chosen at connect
/// time when the connector declares more than one." Zendesk declares two
/// (`zendesk.oauth`, `zendesk.basic`). The host starts the provider without
/// `--profile`, the provider refuses a two-profile connector without one
/// (`adapters/catalog/src/local.rs` `derived`), and the whole file is
/// refused, every other entry with it.
#[test]
fn a_zendesk_instance_named_alone_loads_and_leaves_its_profile_to_connect() {
    let installation = Installation::real();
    let config = installation
        .read(
            "[connectors.zendesk]\nenabled=true\n[connectors.zendesk.instances.support]\nendpoint='https://example.zendesk.com'\n",
        )
        .expect("a two-profile connector instance named alone does not refuse the file");
    assert!(!config.adapters.contains_key("zendesk.support"));
    let profiles = config
        .awaiting_profile("zendesk", "support")
        .expect("the instance waits for a profile");
    assert_eq!(
        profiles,
        &BTreeSet::from(["zendesk.basic".to_owned(), "zendesk.oauth".to_owned()])
    );
}

/// `Executable::print_bootstrap` promises "at most ten seconds". A provider
/// that leaves a descendant holding its stdout keeps the reader thread, and
/// so every configuration load, blocked until that descendant exits.
#[test]
fn printing_the_bootstrap_is_bounded_by_ten_seconds_even_when_a_descendant_keeps_stdout() {
    let installation = Installation::with(|provider| {
        fs::write(provider, "#!/bin/sh\nsleep 25 &\nexit 0\n").unwrap();
    });
    let started = Instant::now();
    let read = installation
        .read("[connectors.gmail]\nenabled=true\n[connectors.gmail.instances.private]\n");
    let elapsed = started.elapsed();
    assert_eq!(read.err(), Some(Failure::InvalidConfiguration));
    assert!(
        elapsed < Duration::from_secs(15),
        "a configuration load took {elapsed:?}"
    );
}
