//! The release workflow checks `connectors-catalog-provider --version` against the tag's
//! version before it packages the binary, so the binary answers it without a configuration.
use std::process::Command;

#[test]
fn the_provider_names_its_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--version")
        .output()
        .expect("run the provider");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        format!(
            "connectors-catalog-provider {}\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}
