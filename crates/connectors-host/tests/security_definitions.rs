//! story:invoke-definition-file, security review: an invoked definition's
//! entry, held in `<state>/definitions/instances.toml`, is admitted only in the
//! shape `operations invoke --definition` records it. That command pins the
//! installed catalog provider (`Installation::pinned`) on the native
//! configuration of the entry's own cache directory
//! (`<state>/definitions/<digest>/native.json`), and a definition never names
//! a binary or a command (architecture-decision-record:
//! provider-definitions-and-runtimes D12 point 4). Integrity conformance of
//! the stored file against that contract.
#![cfg(unix)]
use connectors_host::local::{
    config::{Adapter, Config, Executable, Paths, Permissions, Selector},
    definitions::{self, Entry},
    runtime,
};
use std::collections::BTreeSet;

fn paths() -> (tempfile::TempDir, Paths) {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths {
        config: root.path().join("config/config.toml"),
        state: root.path().join("state"),
    };
    Config::initialize(&paths).unwrap();
    (root, paths)
}

fn entry(executable: Executable) -> Entry {
    Entry {
        digest: "b".repeat(64),
        adapter: Adapter {
            credential_revision: Some("credential-1".into()),
            instance_id: definitions::alias("forge"),
            adapter_id: "catalog".into(),
            configuration_revision: "cfg-1".into(),
            protocol: "v1alpha1".into(),
            private_protocol: Some(runtime::PrivateProtocol::V2),
            startup: Default::default(),
            restart: Default::default(),
            executable,
            permissions: Permissions {
                profiles: BTreeSet::from(["forge.pat".to_owned()]),
                operations: BTreeSet::from(["project.get".to_owned()]),
                custody: Default::default(),
            },
        },
    }
}

/// An entry whose executable is not the installed catalog provider, started
/// on a command line that is not `--local-config` of its own cache entry, is
/// not recorded and not held as the connector's instance `definition`.
#[test]
fn security_an_entry_naming_another_executable_or_argv_is_not_held() {
    let (_root, paths) = paths();
    let foreign = entry(Executable {
        path: "/usr/bin/env".into(),
        sha256: "c".repeat(64),
        args: vec!["sh".into(), "-c".into(), "exit 0".into()],
    });
    let recorded = definitions::record(&paths, "forge", &foreign);
    let config = Config::read_at(&paths).unwrap();
    let held = config
        .select(Selector::Instance {
            connector: "forge",
            instance: definitions::INSTANCE,
        })
        .map(|selected| selected.adapter.executable.clone());
    assert!(
        recorded.is_err() && held.is_none(),
        "recorded: {recorded:?}; held executable: {held:?}"
    );
}
