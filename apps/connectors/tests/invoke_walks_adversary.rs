//! Adversary pass 1 of story:invoke-walks-declared-pages, against
//! `contracts/cli/v1alpha1/semantics.md`, "Walking pages", and the loop in
//! `apps/connectors/src/walk.rs`.
//!
//! The pure cases drive `connectors::walk::run` directly. The jailed case runs
//! the production CLI, owner and an adapter child as `invoke_walks.rs` does,
//! with a Zendesk-shaped stand-in whose `ticket.audits` declares the catalog's
//! walk and answers two pages of large audits.
use serde_json::{Value, json};

/// The Zendesk audits walk the catalog declares.
fn audits_walk() -> connectors::walk::Walk {
    serde_json::from_value(json!({"parameter":"page[after]",
        "next":"/meta/after_cursor","records":"/audits",
        "continues":{"pointer":"/meta/has_more","equals":true},
        "ends":[{"name":"has_more_false","when":{"pointer":"/meta/has_more","equals":false}},
                {"name":"no_cursor","absent":"/meta/after_cursor"}]}))
    .unwrap()
}

/// story:invoke-walks-declared-pages: `repeated_position` is "the next
/// position was already sent in this walk". A walk given `resume` sends that
/// cursor on its first page; a provider whose answer names the same cursor
/// again names a position already sent, so the walk stops there, and the same
/// page is not read and written a second time.
#[test]
fn a_resumed_walk_whose_first_page_names_its_own_cursor_stops_as_repeated() {
    use connectors::walk::{self, Page};
    let resume = json!({"ticket_id":7,"page[after]":"stuck"});
    let mut reads = Vec::new();
    let mut taken = Vec::new();
    let walked = walk::run(
        &audits_walk(),
        &resume,
        20,
        None,
        |input: &Value| {
            reads.push(input.clone());
            Page::Answered(json!({"status":200,"provenance":{},
                "body":{"audits":[{"id":1}],"meta":{"has_more":true,"after_cursor":"stuck"}}}))
        },
        |records| {
            taken.extend(records);
            Ok(())
        },
    );
    assert_eq!(
        (reads.len(), taken.len()),
        (1, 1),
        "story:invoke-walks-declared-pages: the cursor the first page sent is a position \
         already sent; reads {reads:?}, records {taken:?}"
    );
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":1,"records":1,"complete":false,"stopped":"repeated_position"}),
        "story:invoke-walks-declared-pages"
    );
}

/// story:invoke-walks-declared-pages: `missing` lists the `--fields` paths no
/// record had. A page the sink refuses is "none of them is counted": its
/// records are not in `records` and not in the output, so a path only they
/// had is one no written record had, and `missing` still names it.
#[test]
fn missing_names_only_what_the_written_records_lacked() {
    use connectors::walk::{self, Page};
    let fields = connectors::shape::Fields::parse("id,extra").unwrap();
    let mut pages = 0;
    let walked = walk::run(
        &audits_walk(),
        &json!({"ticket_id":7}),
        20,
        Some(&fields),
        |input: &Value| {
            Page::Answered(match input.get("page[after]") {
                None => json!({"status":200,"provenance":{},
                    "body":{"audits":[{"id":1}],"meta":{"has_more":true,"after_cursor":"c2"}}}),
                Some(_) => json!({"status":200,"provenance":{},
                    "body":{"audits":[{"id":2,"extra":true}],"meta":{"has_more":false}}}),
            })
        },
        |_| {
            pages += 1;
            if pages > 1 { Err("write") } else { Ok(()) }
        },
    );
    assert_eq!(
        Value::Object(walked.summary()),
        json!({"pages":1,"records":1,"complete":false,"stopped":"write",
            "resume":{"ticket_id":7,"page[after]":"c2"},"missing":["extra"]}),
        "story:invoke-walks-declared-pages: the one record written has no `extra`"
    );
}

#[cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;

#[cfg(target_os = "linux")]
mod jailed {
    use super::{Value, json};
    use crate::mounted::*;
    use connectors_custody::mounted::fixture::{Jail, Map};
    use connectors_host::local::{registry, runtime};
    use std::{
        collections::BTreeSet,
        fs,
        os::unix::{fs::PermissionsExt, process::CommandExt},
        process::Stdio,
    };

    /// The adapter child the owner starts filters to this test's exact name.
    const ADAPTER_FIXTURE: &str = "jailed::zendesk_adapter_fixture";
    const AUDITS: &str = "ticket.audits";
    /// The ticket whose two pages each hold two audits of about `PAD` bytes.
    const PADDED: u64 = 4000;
    const PAD: usize = 1000;

    /// The adapter child the owner starts: this binary, filtered to this test.
    /// The ordinary run does nothing.
    #[test]
    fn zendesk_adapter_fixture() {
        if std::env::args().any(|arg| arg == "--connectors-private-fd") {
            let _ = runtime::serve(3, Zendesk);
        }
    }

    /// Two audits of about `PAD` bytes each, the second page's with `extra`.
    fn padded(first: u64, extra: bool) -> Vec<Value> {
        (first..first + 2)
            .map(|n| {
                let mut audit = json!({"id": n, "ticket_id": PADDED, "pad": "p".repeat(PAD)});
                if extra {
                    audit["extra"] = json!(true);
                }
                audit
            })
            .collect()
    }

    fn catalog(body: Value) -> Value {
        json!({"status": 200, "body": body,
               "provenance": {"instance": "forge-local",
                              "resource": "/api/v2/tickets/{ticket_id}/audits",
                              "observed_at_unix_ms": 1_758_000_000_000_u64,
                              "source_revision": "fixture-source-1"}})
    }

    fn operation() -> connectors_core::Operation {
        let walk = json!({"parameter": "page[after]", "next": "/meta/after_cursor",
            "records": "/audits", "query": {"include_boundary_indicators": true},
            "continues": {"pointer": "/meta/has_more", "equals": true},
            "ends": [{"name": "has_more_false", "when": {"pointer": "/meta/has_more", "equals": false}},
                     {"name": "no_cursor", "absent": "/meta/after_cursor"},
                     {"name": "no_records", "empty": "/audits"}]});
        connectors_core::Operation {
            id: AUDITS.into(),
            description: "ticket.audits of one ticket".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"ticket_id":{"type":"integer"},"page[after]":{"type":"string"},
                    "include_boundary_indicators":{"type":"boolean"}},
                "required":["ticket_id"],"x-connectors-walk":walk}),
            output_schema: json!({"type":"object","required":["status","body","provenance"]}),
        }
    }

    fn bootstrap() -> runtime::Bootstrap {
        let descriptor = connectors_core::Descriptor {
            version: "v1alpha1".into(),
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            revision: REVISION.into(),
            operations: vec![operation()],
            configuration_schema: json!({"type":"object"}),
        };
        runtime::Bootstrap {
            bindings: None,
            credential_revision: None,
            executable: None,
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            protocol: "v1alpha1".into(),
            configuration_revision: "cfg-1".into(),
            provider_authority: "https://fixture.zendesk.invalid".into(),
            descriptor: serde_json::to_string(&descriptor).unwrap(),
            profiles: vec![runtime::Profile {
                id: PROFILE.into(),
                revision: "profile-1".into(),
                purpose: registry::Purpose::DelegatedUser,
                subject: registry::Subject::User,
                scheme: "http_bearer".into(),
                capability: "http-bearer".into(),
                minimum_scopes: BTreeSet::new(),
                evidence_lifetime_ms: 60_000,
                fields: vec![runtime::EntryField {
                    name: "token".into(),
                    label: "Token".into(),
                    max_bytes: 1024,
                }],
                acquisition: None,
            }],
            requirements: vec![runtime::Requirement {
                operation: AUDITS.into(),
                custody: false,
                profile: PROFILE.into(),
                scopes: BTreeSet::new(),
                effect: runtime::Effect::Read,
            }],
        }
    }

    struct Zendesk;

    #[async_trait::async_trait]
    impl runtime::Adapter for Zendesk {
        fn bootstrap(&self) -> runtime::Bootstrap {
            bootstrap()
        }
        fn bootstrap_v2(&self) -> runtime::Result<runtime::Bootstrap> {
            Ok(bootstrap())
        }
        async fn validate(
            &self,
            _: &str,
            _: connectors_sdk::Secret,
        ) -> runtime::Result<runtime::Baseline> {
            let now = connectors_sdk::now_ms();
            Ok(runtime::Baseline {
                binding: None,
                identity: registry::ExternalIdentity {
                    kind: "zendesk.user".into(),
                    subject: "200".into(),
                },
                granted_scopes: None,
                credential_expires_at_ms: None,
                collected_at_ms: now,
                valid_until_ms: now + 50_000,
            })
        }
        async fn invoke(
            &self,
            operation: &str,
            _: &str,
            _: connectors_sdk::Secret,
            input: Value,
        ) -> runtime::Result<Value> {
            let id = input["ticket_id"].as_u64().unwrap_or(0);
            let after = input.get("page[after]").and_then(Value::as_str);
            match (operation, id, after) {
                (AUDITS, PADDED, None) => Ok(catalog(json!({"audits": padded(1, false),
                    "meta": {"has_more": true, "after_cursor": "padded-2"}}))),
                (AUDITS, PADDED, Some("padded-2")) => Ok(catalog(
                    json!({"audits": padded(3, true), "meta": {"has_more": false}}),
                )),
                _ => Err(runtime::Failure::Unsupported),
            }
        }
    }

    fn connected() -> Container {
        bootstrap()
            .validate_for(runtime::PrivateProtocol::V2)
            .expect("the fixture's description passes the child's own readiness check");
        let container = Container::new(ADAPTER_FIXTURE);
        container.deliver(PROFILE, SECRET);
        let uid = connectors_host::local::filesystem::uid();
        let adapter = digest(&fs::read(container.path("/bin-test/adapter")).unwrap());
        let args = serde_json::to_string(&["--exact", ADAPTER_FIXTURE, "--"]).unwrap();
        let table = "connectors.zendesk.instances.support";
        let text = format!(
            "format='connectors-local/5'\nowner_uid={uid}\n{}\n[connectors.zendesk]\nenabled=true\n[{table}]\nalias='{ALIAS}'\ninstance_id='forge-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.permissions]\nprofiles=['{PROFILE}']\noperations=['{AUDITS}']\n[{table}.executable]\npath='/bin-test/adapter'\nsha256='{adapter}'\nargs={args}\n",
            Container::mounted()
        );
        let path = container.path(CONFIG);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        success(&container.run(&["setup", "init"]));
        success(&container.connect(PROFILE));
        container
    }

    /// story:invoke-walks-declared-pages: the sink takes a page's records all
    /// or nothing, and `write` is "a page's records could not be written; none
    /// of them is counted". When the second page fits only in part (a full
    /// disk, a quota or a file size limit: here `RLIMIT_FSIZE`, with `SIGXFSZ`
    /// ignored, so a write past it is short and then `EFBIG`), the file holds
    /// the first page's lines and nothing of the second: no torn NDJSON line.
    #[test]
    fn a_page_that_cannot_be_written_whole_leaves_no_part_of_it() {
        let container = connected();
        // The first page (two audits of about 1 KiB) fits; the second does not.
        let limit = (2 * PAD + 600) as u64;
        let input = json!({"ticket_id": PADDED}).to_string();
        let mut command = container.jail.command("/bin-test/connectors", Map::Own);
        command
            .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
            .args(["invoke", "zendesk/support", AUDITS, "--input-json", &input])
            .args(["--walk", "--out", "/work/limited.ndjson"])
            .env_clear()
            .stdin(Stdio::null());
        // SAFETY: system calls only, on a value captured by copy.
        unsafe {
            command.pre_exec(move || {
                let limit = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
                Ok(())
            });
        }
        let summary = success(&Jail::output(&mut command));
        assert_eq!(
            (&summary["pages"], &summary["records"], &summary["stopped"]),
            (&json!(1), &json!(2), &json!("write")),
            "{summary}"
        );
        let text = fs::read_to_string(container.path("/work/limited.ndjson")).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            text.ends_with('\n')
                && lines.len() == 2
                && lines
                    .iter()
                    .all(|line| serde_json::from_str::<Value>(line).is_ok()),
            "story:invoke-walks-declared-pages: the file holds the {} records counted and no \
             part of the page that was not; it holds {} bytes in {} lines, the last {:?}",
            summary["records"],
            text.len(),
            lines.len(),
            lines.last().map(|line| &line[..line.len().min(80)])
        );
    }
}
