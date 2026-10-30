//! story:connectors-do-short-invocation, decision D10, its edges: what
//! `connections connect <connector>/<instance> --endpoint <url>` writes into
//! the operator's configuration file, and what it refuses to touch. The file
//! keeps every byte but the endpoint, stays owner-only, is replaced atomically
//! or not at all, and is never written for a refused command. See
//! `connect_endpoint.rs` for the stand-in provider this binary also is.
#[cfg(unix)]
#[path = "support/standin.rs"]
mod standin;

#[cfg(unix)]
fn main() {
    tests::main();
}

#[cfg(not(unix))]
fn main() {}

#[cfg(unix)]
mod tests {
    use super::standin::*;
    use serde_json::json;
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    };

    const URL: &str = "https://gitlab.example.test";
    const GITLAB: &[&str] = &[
        "connections",
        "connect",
        "gitlab/ref-1",
        "--profile",
        "gitlab.pat",
        "--endpoint",
        URL,
    ];

    pub fn main() {
        super::standin::main(&[
            (
                "comments_and_spacing_keep_every_other_byte",
                comments_and_spacing_keep_every_other_byte,
            ),
            (
                "an_inline_connector_table_takes_the_instance",
                an_inline_connector_table_takes_the_instance,
            ),
            (
                "the_same_endpoint_twice_writes_nothing",
                the_same_endpoint_twice_writes_nothing,
            ),
            (
                "an_owner_read_only_file_keeps_its_mode",
                an_owner_read_only_file_keeps_its_mode,
            ),
            (
                "a_symlinked_or_group_readable_configuration_is_not_written",
                a_symlinked_or_group_readable_configuration_is_not_written,
            ),
            (
                "an_endpoint_with_surrounding_whitespace_is_refused",
                an_endpoint_with_surrounding_whitespace_is_refused,
            ),
            (
                "a_disabled_connector_table_is_refused",
                a_disabled_connector_table_is_refused,
            ),
            (
                "an_older_format_is_refused_naming_it",
                an_older_format_is_refused_naming_it,
            ),
            (
                "a_pinned_instance_takes_no_endpoint",
                a_pinned_instance_takes_no_endpoint,
            ),
            (
                "an_endpoint_needs_a_named_instance",
                an_endpoint_needs_a_named_instance,
            ),
        ]);
    }

    /// The lines of `after` without its `endpoint = "<URL>"` lines.
    fn without_endpoint(after: &str) -> String {
        after
            .split_inclusive('\n')
            .filter(|line| line.trim() != format!("endpoint = \"{URL}\""))
            .collect()
    }

    /// story:connectors-do-short-invocation: comments, blank lines, odd
    /// spacing and quoting styles of the file survive the write byte for byte;
    /// the endpoint is the only line added to an existing instance.
    pub fn comments_and_spacing_keep_every_other_byte() {
        let installation = Installation::new();
        let before = installation.configure(
            "# operator notes\n\n[connectors.gitlab]   # the forge\nenabled    =   true\n\n[connectors.gitlab.instances.ref-1]\n  description = \"team forge\"   # trailing\n\n# a comment after the table\n[connectors.jira]\nenabled=false\n",
        );
        let (_, data) = refusal(&installation.run(GITLAB));
        assert_eq!(data["stage"], "protected_entry", "{data}");
        let after = installation.text();
        assert_ne!(after, before);
        assert_eq!(without_endpoint(&after), before, "{after}");
        assert_eq!(
            parsed(&after)["connectors"]["gitlab"]["instances"]["ref-1"]["endpoint"],
            URL.into()
        );
        assert_eq!(installation.mode(), 0o600);
    }

    /// story:connectors-do-short-invocation: a connector written as an inline
    /// table takes the new instance inside it.
    pub fn an_inline_connector_table_takes_the_instance() {
        let installation = Installation::new();
        installation.configure("[connectors]\ngitlab = { enabled = true }\n");
        let (_, data) = refusal(&installation.run(GITLAB));
        assert_eq!(data["stage"], "protected_entry", "{data}");
        let after = parsed(&installation.text());
        assert_eq!(
            after["connectors"]["gitlab"]["instances"]["ref-1"]["endpoint"],
            URL.into(),
            "{after}"
        );
        assert_eq!(after["connectors"]["gitlab"]["enabled"], true.into());
    }

    /// story:connectors-do-short-invocation: the same endpoint again writes
    /// nothing: the file is the same inode with the same bytes.
    pub fn the_same_endpoint_twice_writes_nothing() {
        let installation = Installation::new();
        installation.configure("");
        refusal(&installation.run(GITLAB));
        let (text, inode) = (
            installation.text(),
            fs::metadata(installation.config()).unwrap().ino(),
        );
        let (_, data) = refusal(&installation.run(GITLAB));
        assert_eq!(data["stage"], "protected_entry", "{data}");
        assert_eq!(installation.text(), text);
        assert_eq!(fs::metadata(installation.config()).unwrap().ino(), inode);
    }

    /// story:connectors-do-short-invocation: the replacement keeps the
    /// owner-only mode the file had, and leaves no temporary file beside it.
    pub fn an_owner_read_only_file_keeps_its_mode() {
        let installation = Installation::new();
        installation.configure("");
        fs::set_permissions(installation.config(), fs::Permissions::from_mode(0o400)).unwrap();
        let (_, data) = refusal(&installation.run(GITLAB));
        assert_eq!(data["stage"], "protected_entry", "{data}");
        assert_eq!(installation.mode(), 0o400);
        let names: Vec<_> = fs::read_dir(installation.config().parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, ["config.toml"], "{names:?}");
    }

    /// story:connectors-do-short-invocation: a configuration that is a
    /// symbolic link, or that its group can read, is the configuration's
    /// refusal, and neither it nor what a link names is written.
    pub fn a_symlinked_or_group_readable_configuration_is_not_written() {
        let installation = Installation::new();
        let before = installation.configure("");
        let real = installation.root.path().join("real.toml");
        fs::rename(installation.config(), &real).unwrap();
        symlink(&real, installation.config()).unwrap();
        let (code, data) = refusal(&installation.run(GITLAB));
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(data["code"], "invalid_configuration", "{data}");
        assert_eq!(fs::read_to_string(&real).unwrap(), before);
        assert!(
            fs::symlink_metadata(installation.config())
                .unwrap()
                .file_type()
                .is_symlink()
        );
        fs::remove_file(installation.config()).unwrap();
        fs::rename(&real, installation.config()).unwrap();
        fs::set_permissions(installation.config(), fs::Permissions::from_mode(0o640)).unwrap();
        let (code, data) = refusal(&installation.run(GITLAB));
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(
            (&data["code"], &data["configuration_rule"]),
            (
                &json!("invalid_configuration"),
                &json!("config_not_private")
            ),
            "{data}"
        );
        assert_eq!(installation.text(), before);
        assert_eq!(installation.mode(), 0o640);
    }

    /// story:connectors-do-short-invocation: an endpoint with whitespace
    /// around it is refused, not trimmed.
    pub fn an_endpoint_with_surrounding_whitespace_is_refused() {
        let installation = Installation::new();
        let before = installation.configure("");
        for endpoint in [
            " https://gitlab.example.test",
            "https://gitlab.example.test\n",
        ] {
            let (code, data) = refusal(&installation.run(&[
                "connections",
                "connect",
                "gitlab/ref-1",
                "--profile",
                "gitlab.pat",
                "--endpoint",
                endpoint,
            ]));
            assert_eq!(code, Some(2), "{endpoint:?}: {data}");
            assert_eq!(
                (&data["code"], &data["field"]),
                (&json!("invalid_input"), &json!("endpoint")),
                "{data}"
            );
            assert_eq!(installation.text(), before);
        }
    }

    /// story:connectors-do-short-invocation: a disabled connector table is
    /// left as the operator wrote it.
    pub fn a_disabled_connector_table_is_refused() {
        let installation = Installation::new();
        let before = installation.configure("[connectors.gitlab]\nenabled=false\n");
        let (code, data) = refusal(&installation.run(GITLAB));
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(
            (&data["code"], &data["stage"]),
            (&json!("invalid_configuration"), &json!("configuration")),
            "{data}"
        );
        assert_eq!(installation.text(), before);
    }

    /// story:connectors-do-short-invocation: a file of an older format has no
    /// instance tables; it is refused naming its format, which `setup migrate`
    /// rewrites, and not written.
    pub fn an_older_format_is_refused_naming_it() {
        let installation = Installation::new();
        installation.configure("");
        let before = installation
            .text()
            .replace("connectors-local/5", "connectors-local/4");
        fs::write(installation.config(), &before).unwrap();
        let (code, data) = refusal(&installation.run(GITLAB));
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(
            (&data["code"], &data["configuration_format"]),
            (
                &json!("invalid_configuration"),
                &json!("connectors-local/4")
            ),
            "{data}"
        );
        assert_eq!(installation.text(), before);
    }

    /// story:connectors-do-short-invocation: an instance pinned to its
    /// executable selection takes its endpoint from those pins, never from
    /// this command: `--endpoint` there is refused and nothing is written.
    pub fn a_pinned_instance_takes_no_endpoint() {
        let installation = Installation::new();
        let table = "connectors.gitlab.instances.ref-1";
        let before = installation.configure(&format!(
            "[connectors.gitlab]\nenabled=true\n[{table}]\nalias='gitlab-pinned'\ninstance_id='gitlab-pinned'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='/not-installed/connectors-catalog-provider'\nsha256='{}'\nargs=[]\n[{table}.permissions]\nprofiles=['gitlab.pat']\noperations=['gitlab.whoami']\n",
            "a".repeat(64)
        ));
        let (code, data) = refusal(&installation.run(GITLAB));
        assert_eq!(code, Some(2), "{data}");
        assert_eq!(
            (&data["code"], &data["field"]),
            (&json!("invalid_input"), &json!("endpoint")),
            "{data}"
        );
        assert_eq!(installation.text(), before);
    }

    /// story:connectors-do-short-invocation: `--endpoint` is written into a
    /// named instance only: beside the deprecated `--adapter`, or with no
    /// selector at all, it is refused naming `endpoint`.
    pub fn an_endpoint_needs_a_named_instance() {
        let installation = Installation::new();
        let before = installation.configure("");
        for selection in [&["--adapter", "gitlab.ref-1"][..], &[][..]] {
            let mut args = vec!["connections", "connect", "--profile", "gitlab.pat"];
            args.extend(selection);
            args.extend(["--endpoint", URL]);
            let (code, data) = refusal(&installation.run(&args));
            assert_eq!(code, Some(2), "{selection:?}: {data}");
            assert_eq!(
                (&data["code"], &data["field"]),
                (&json!("invalid_input"), &json!("endpoint")),
                "{selection:?}: {data}"
            );
            assert_eq!(installation.text(), before);
        }
    }
}
