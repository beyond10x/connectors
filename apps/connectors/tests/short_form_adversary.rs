//! Adversarial cases for story:migrate-writes-the-short-instance-form (wave
//! D298, U2). A short-form instance states `alias` and `instance_id` and is
//! derived by the installed catalog provider; `setup migrate` writes it for a
//! legacy entry whose derived entry equals the pinned one.
//!
//! What these cases hold the change to:
//!
//! - the short form is only written when nothing keyed by the entry's
//!   executable selection is stranded: besides an approval policy, an
//!   operation curation (`<config>.operation-curation.json`) is pinned to it
//!   (`crates/connectors-host/src/local/operation_curation.rs`, `load`);
//! - a short-form instance is a derived instance, not "an instance that
//!   carries the pins of a legacy entry" (`contracts/cli/v1alpha1/semantics.md`,
//!   `connections connect --endpoint`), so repeating its own endpoint is not
//!   refused;
//! - an `instance_id` is an identity claim: no instance of any connector, no
//!   pinned instance and no legacy entry may claim another one's id, in either
//!   direction, whether the file is read with or without deriving.
//!
//! This binary is also the stand-in catalog provider (`support/standin.rs`),
//! so it runs without the standard test harness.
#[cfg(target_os = "linux")]
#[path = "support/standin.rs"]
mod standin;

#[cfg(target_os = "linux")]
fn main() {
    tests::main();
}

#[cfg(not(target_os = "linux"))]
fn main() {}

#[cfg(target_os = "linux")]
mod tests {
    use super::standin::{self, Installation, parsed, refusal, success};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
    };

    const API_BASE: &str = "https://gitlab.example.test/api/v4";
    const ENDPOINT: &str = "https://gitlab.example.test";
    const PROFILE: &str = "gitlab.pat";
    const READ: &str = "gitlab.whoami";

    pub fn main() {
        standin::main(&[
            (
                "a_curated_entry_keeps_the_executable_selection_its_curation_is_pinned_to",
                a_curated_entry_keeps_the_executable_selection_its_curation_is_pinned_to,
            ),
            (
                "a_migrated_short_form_instance_takes_its_own_endpoint_again_in_connect",
                a_migrated_short_form_instance_takes_its_own_endpoint_again_in_connect,
            ),
            (
                "no_instance_claims_another_instances_or_entrys_id_in_either_direction",
                no_instance_claims_another_instances_or_entrys_id_in_either_direction,
            ),
            (
                "an_apply_writes_exactly_the_dry_run_and_keeps_the_original_as_backup",
                an_apply_writes_exactly_the_dry_run_and_keeps_the_original_as_backup,
            ),
        ]);
    }

    fn digest(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    fn private(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// The legacy GitLab entry `alias` of the instance `id` on the native
    /// configuration `native`, pinned at its own revision, started as the
    /// installation's provider.
    fn legacy(installation: &Installation, alias: &str, id: &str) -> String {
        let root = installation.root.path();
        let natives = root.join("native");
        if !natives.exists() {
            fs::create_dir(&natives).unwrap();
            fs::set_permissions(&natives, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let native = standin::native(id, API_BASE, &["api"]);
        let path = natives.join(format!("{alias}.json"));
        private(&path, native.to_string().as_bytes());
        let provider = root.join("bin/connectors-catalog-provider");
        let sha256 = digest(&fs::read(&provider).unwrap());
        format!(
            "[adapters.{alias}]\ninstance_id='{id}'\nadapter_id='catalog'\nconfiguration_revision='{}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.permissions]\nprofiles=['{PROFILE}']\noperations=['{READ}']\n[adapters.{alias}.executable]\npath='{}'\nsha256='{sha256}'\nargs=['--local-config','{}']\n",
            standin::native_revision(&native),
            provider.display(),
            path.display(),
        )
    }

    fn entries(result: &Value) -> BTreeMap<String, Value> {
        result["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| (entry["adapter"].as_str().unwrap().to_owned(), entry.clone()))
            .collect()
    }

    fn curation_path(installation: &Installation) -> PathBuf {
        let mut path = installation.config().into_os_string();
        path.push(".operation-curation.json");
        path.into()
    }

    /// A legacy entry `work` that the stand-in derives unchanged is planned in
    /// the short form. The operator has an operation curation for `work`,
    /// pinned to the entry's executable selection, as `connectors mcp`
    /// requires (`operation_curation::load` answers `stale_description` when
    /// `executable_selection != adapter.selection()`). Like an approval policy
    /// (which keeps the entry pinned, naming `executable`), the curation is
    /// bound to the selection the short form replaces, so the migration must
    /// not change it: the entry stays pinned, or its selection is unchanged.
    pub fn a_curated_entry_keeps_the_executable_selection_its_curation_is_pinned_to() {
        let installation = Installation::new();
        installation.configure(&legacy(&installation, "work", "gitlab-work-local"));
        let planned = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        let work = &entries(&planned)["work"];
        // Precondition: without a curation it is the short form.
        assert_eq!(work["form"], "short", "{work}");
        let selection = work["before"]["executable_selection"]
            .as_str()
            .unwrap()
            .to_owned();
        let curation = json!({
            "format": connectors_core::operation_metadata::CURATION_FORMAT,
            "adapters": [{
                "adapter_alias": "work",
                "executable_selection": selection,
                "bootstrap_sha256": digest(b"the bootstrap the curation was made from"),
                "operations": [],
            }],
        });
        let bytes = serde_json::to_vec(&curation).unwrap();
        assert!(
            connectors_core::operation_metadata::Curation::parse(&bytes).is_ok(),
            "the curation is one the host admits"
        );
        private(&curation_path(&installation), &bytes);

        let planned = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        let work = &entries(&planned)["work"];
        assert_eq!(
            work["after"]["executable_selection"], selection,
            "the migration replaces the executable selection the operation curation of `work` \
             is pinned to, so every curated operation answers stale_description afterwards: {work}"
        );
    }

    /// `setup migrate --apply` writes `work` in the short form with its
    /// non-default endpoint. The instance is a derived one (no pin), so
    /// `connections connect gitlab/work --endpoint <its own endpoint>` is the
    /// no-op a plain derived instance gets and goes on to the credential, as
    /// the change's own `moves_no_connection` (which checks a short-form
    /// instance under its alias and instance id) is written for. It must not
    /// be refused as `invalid_input` naming `endpoint`, the refusal for an
    /// instance carrying a legacy entry's pins.
    pub fn a_migrated_short_form_instance_takes_its_own_endpoint_again_in_connect() {
        let installation = Installation::new();
        installation.configure(&legacy(&installation, "work", "gitlab-work-local"));
        let applied = success(&installation.run(&["setup", "migrate", "--apply"]));
        assert_eq!(entries(&applied)["work"]["form"], "short", "{applied}");
        let migrated = installation.text();
        let instance = &parsed(&migrated)["connectors"]["gitlab"]["instances"]["work"];
        assert_eq!(instance["endpoint"], ENDPOINT.into(), "{migrated}");

        // Control: without `--endpoint` it goes on to the credential.
        let (control_code, control) = refusal(&installation.run(&[
            "connections",
            "connect",
            "gitlab/work",
            "--profile",
            PROFILE,
        ]));
        assert_eq!(control["stage"], "protected_entry", "{control}");

        let (code, data) = refusal(&installation.run(&[
            "connections",
            "connect",
            "gitlab/work",
            "--profile",
            PROFILE,
            "--endpoint",
            ENDPOINT,
        ]));
        assert_eq!(installation.text(), migrated, "nothing is written");
        assert_eq!(
            (code, &data["stage"]),
            (control_code, &control["stage"]),
            "a short-form instance repeating its own endpoint is refused as if it were pinned: {data}"
        );
    }

    /// Every file below claims one instance id twice, across connectors, in
    /// both directions of `<connector>.<instance>`, against a legacy entry's
    /// id and against a pinned instance's id; or names an id no selector
    /// admits. Each is refused by `connectors list` (read without deriving)
    /// and by `instances list` (read deriving through the stand-in), and the
    /// file is unchanged. Ids differing only in case are distinct ids and are
    /// both admitted.
    pub fn no_instance_claims_another_instances_or_entrys_id_in_either_direction() {
        let installation = Installation::new();
        let old = legacy(&installation, "old", "old-local");
        let gitlab =
            |name: &str, keys: &str| format!("[connectors.gitlab.instances.{name}]\n{keys}");
        let github =
            |name: &str, keys: &str| format!("[connectors.github.instances.{name}]\n{keys}");
        let tables = "[connectors.gitlab]\nenabled=true\n[connectors.github]\nenabled=true\n";
        let pinned = old
            .replace(
                "[adapters.old]",
                "[connectors.gitlab.instances.pinned]\nalias='pinned'",
            )
            .replace("[adapters.old.", "[connectors.gitlab.instances.pinned.")
            .replace("instance_id='old-local'", "instance_id='pinned-local'");
        let refused: Vec<(&str, String)> = vec![
            (
                "gitlab instance claims the derived id of github.work",
                format!(
                    "{tables}{}{}",
                    gitlab("a", "alias='a'\ninstance_id='github.work'\n"),
                    github("work", "")
                ),
            ),
            (
                "github instance claims the derived id of gitlab.work",
                format!(
                    "{tables}{}{}",
                    github("a", "instance_id='gitlab.work'\n"),
                    gitlab("work", "")
                ),
            ),
            (
                "two connectors state one id",
                format!(
                    "{tables}{}{}",
                    gitlab("a", "instance_id='shared-id'\n"),
                    github("b", "instance_id='shared-id'\n")
                ),
            ),
            (
                "a short-form instance claims a legacy entry's id",
                format!(
                    "{old}{tables}{}",
                    gitlab("a", "alias='a'\ninstance_id='old-local'\n")
                ),
            ),
            (
                "a short-form instance claims a legacy entry's alias",
                format!(
                    "{old}{tables}{}",
                    gitlab("a", "alias='old'\ninstance_id='a-local'\n")
                ),
            ),
            (
                "a short-form instance claims a pinned instance's id",
                format!(
                    "{tables}{pinned}{}",
                    gitlab("a", "alias='a'\ninstance_id='pinned-local'\n")
                ),
            ),
            (
                "an id with a slash",
                format!("{tables}{}", gitlab("a", "instance_id='gitlab/a'\n")),
            ),
            (
                "an id with a trailing space",
                format!("{tables}{}", gitlab("a", "instance_id='a-local '\n")),
            ),
            (
                "an empty id",
                format!("{tables}{}", gitlab("a", "instance_id=''\n")),
            ),
            (
                "a non-ASCII id",
                format!("{tables}{}", gitlab("a", "instance_id='a-lo\u{0441}al'\n")),
            ),
            (
                "an id with a newline",
                format!("{tables}{}", gitlab("a", "instance_id=\"a\\nlocal\"\n")),
            ),
        ];
        installation.configure("");
        // Each case replaces the file; `setup init` ran once above.
        let rewrite = |body: &str| {
            let text = format!(
                "format='connectors-local/5'\nowner_uid={}\n{body}",
                connectors_host::local::filesystem::uid()
            );
            private(&installation.config(), text.as_bytes());
            text
        };
        for (case, body) in &refused {
            let written = rewrite(body);
            for command in [&["list"][..], &["instances", "list"][..]] {
                let output = installation.run(command);
                assert!(
                    !output.status.success(),
                    "{case}: {command:?} admitted\n{body}\n{}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            assert_eq!(installation.text(), written, "{case}");
        }
        let distinct = format!(
            "{tables}{}{}",
            gitlab("a", "alias='a'\ninstance_id='Case-Local'\n"),
            gitlab("b", "alias='b'\ninstance_id='case-local'\n")
        );
        rewrite(&distinct);
        for command in [&["list"][..], &["instances", "list"][..]] {
            success(&installation.run(command));
        }
    }

    /// Two legacy entries, one short and one stale (pinned): the dry run and
    /// the application list the same entries and the same file, the file is
    /// that text, the backup holds the original byte for byte and owner-only,
    /// and the migrated file migrates to nothing.
    pub fn an_apply_writes_exactly_the_dry_run_and_keeps_the_original_as_backup() {
        let installation = Installation::new();
        let stale = legacy(&installation, "stale", "gitlab-stale-local").replace(
            &format!(
                "configuration_revision='{}'",
                standin::native_revision(&standin::native(
                    "gitlab-stale-local",
                    API_BASE,
                    &["api"]
                ))
            ),
            "configuration_revision='cfg-stale'",
        );
        let body = format!(
            "{}{stale}",
            legacy(&installation, "work", "gitlab-work-local")
        );
        let original = installation.configure(&body);
        let planned = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        let listed = entries(&planned);
        assert_eq!(listed["work"]["form"], "short", "{planned}");
        assert_eq!(
            (&listed["stale"]["form"], &listed["stale"]["pinned_field"]),
            (&json!("pinned"), &json!("configuration_revision")),
            "{planned}"
        );
        assert_eq!(installation.text(), original);
        let applied = success(&installation.run(&["setup", "migrate", "--apply"]));
        assert_eq!(applied["entries"], planned["entries"]);
        assert_eq!(applied["configuration"], planned["configuration"]);
        assert_eq!(
            installation.text(),
            planned["configuration"].as_str().unwrap()
        );
        let backup = PathBuf::from(applied["backup_path"].as_str().unwrap());
        assert_eq!(fs::read_to_string(&backup).unwrap(), original);
        assert_eq!(
            fs::symlink_metadata(&backup).unwrap().permissions().mode() & 0o7777,
            0o600
        );
        let again = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        assert_eq!(again["disposition"], "unchanged", "{again}");
        for (selector, id) in [
            (&["--adapter", "work"][..], "gitlab-work-local"),
            (
                &["--connector", "gitlab", "--instance", "work"][..],
                "gitlab-work-local",
            ),
            (&["--adapter", "stale"][..], "gitlab-stale-local"),
        ] {
            let described =
                success(&installation.run(&[&["instances", "describe"][..], selector].concat()));
            assert!(
                described.to_string().contains(&format!("\"{id}\"")),
                "{selector:?}: {described}"
            );
        }
    }
}
