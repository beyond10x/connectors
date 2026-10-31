//! story:invoke-walks-declared-pages: conformance cases for `invoke --walk`
//! (`contracts/cli/v1alpha1/semantics.md`, "Walking pages").
//!
//! The loop cases call `connectors::walk::run`, the loop `invoke --walk`
//! runs, with pages built in the test. The jailed case runs the production CLI,
//! owner and an adapter child (this test binary) as `invoke_walks.rs` does.
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

/// Counts the bytes each thread holds, so a case can read what one call
/// on its own thread held at most.
struct Counting;

thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn held(by: isize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get() + by;
        live.set(now);
        let _ = PEAK.try_with(|peak| {
            if now > peak.get() {
                peak.set(now);
            }
        });
    });
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            held(layout.size() as isize);
        }
        ptr
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            held(layout.size() as isize);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        held(-(layout.size() as isize));
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            held(new_size as isize - layout.size() as isize);
        }
        moved
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// The most bytes this thread held while `run` ran, above what it held before.
fn peak_while<T>(run: impl FnOnce() -> T) -> (T, isize) {
    let before = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(before));
    let value = run();
    (value, PEAK.with(Cell::get) - before)
}

fn walk(value: Value) -> connectors::walk::Walk {
    serde_json::from_value(value).unwrap()
}

/// story:invoke-walks-declared-pages: a file walk writes each page and keeps
/// none of it, so what it holds does not grow with the number of pages: a
/// provider naming a distinct 2 KiB position on every page is walked to the
/// 10,000-page limit holding what a 100-page walk holds.
#[test]
fn a_file_walk_holds_no_more_memory_after_ten_thousand_pages_than_after_one_hundred() {
    use connectors::walk::{self, Page};
    const POSITION: usize = 2 * 1024;
    let declared = walk(
        json!({"parameter":"cursor","next":"/next","records":"/items",
        "ends":[{"name":"done","when":{"pointer":"/done","equals":true}}]}),
    );
    let walked_to = |pages: u32| {
        peak_while(|| {
            let mut served = 0_u32;
            let walked = walk::run(
                &declared,
                &json!({}),
                pages,
                None,
                |_: &Value| {
                    served += 1;
                    let next = format!("{served:08}{}", "p".repeat(POSITION - 8));
                    Page::Answered(json!({"status":200,
                        "body":{"items":[{"n":served}],"next":next,"done":false},
                        "provenance":{}}))
                },
                |_records| Ok(()),
            );
            walked.pages
        })
    };
    let (few, held_few) = walked_to(100);
    let (many, held_many) = walked_to(10_000);
    assert_eq!((few, many), (100, 10_000), "every page is read");
    assert!(
        held_many - held_few < 2 * 1024 * 1024,
        "story:invoke-walks-declared-pages: a 10,000-page walk held {held_many} bytes at most, \
         a 100-page walk {held_few}: the difference grows with the pages read"
    );
}

/// story:invoke-walks-declared-pages: a walk given its position in the input
/// (as `resume` is given back) never sends that position again: a provider
/// that names the same position stops the walk as `repeated_position` after
/// the one page read with it.
#[test]
fn a_walk_resumed_at_a_position_never_sends_that_position_twice() {
    use connectors::walk::{self, Page};
    let declared = walk(
        json!({"parameter":"cursor","next":"/next","records":"/items",
        "ends":[{"name":"no_next","absent":"/next"}]}),
    );
    let mut sent = Vec::new();
    let mut written = 0;
    let walked = walk::run(
        &declared,
        &json!({"cursor":"c7"}),
        20,
        None,
        |input: &Value| {
            sent.push(input["cursor"].clone());
            Page::Answered(json!({"status":200,
                "body":{"items":[{"id":7}],"next":"c7"},"provenance":{}}))
        },
        |records| {
            written += records.len();
            Ok(())
        },
    );
    assert_eq!(
        sent,
        [json!("c7")],
        "story:invoke-walks-declared-pages: the position the walk started at was sent again"
    );
    assert_eq!(written, 1, "the page read with it is written once");
    assert_eq!(
        walked.stopped.as_ref().map(|stopped| stopped.name()),
        Some("repeated_position")
    );
}

#[cfg(target_os = "linux")]
#[path = "support/mounted.rs"]
mod mounted;

#[cfg(target_os = "linux")]
mod jailed {
    use super::{Value, json};
    use crate::mounted::*;
    use connectors_host::local::{registry, runtime};
    use std::{collections::BTreeSet, fs, io::Write, os::unix::fs::PermissionsExt, path::Path};

    /// The adapter child the owner starts filters to this test's exact name.
    const ADAPTER_FIXTURE: &str = "jailed::security_walk_adapter_fixture";
    const AUDITS: &str = "ticket.audits";
    const CURSOR: &str = "fixture-audit-cursor-2";

    /// The adapter child the owner starts: this binary, filtered to this test.
    /// The ordinary run does nothing.
    #[test]
    fn security_walk_adapter_fixture() {
        if std::env::args().any(|arg| arg == "--connectors-private-fd") {
            let _ = runtime::serve(3, Zendesk);
        }
    }

    fn catalog(body: Value) -> Value {
        json!({"status": 200, "body": body,
               "provenance": {"instance": "forge-local",
                              "resource": "/api/v2/tickets/{ticket_id}/audits",
                              "observed_at_unix_ms": 1_758_000_000_000_u64,
                              "source_revision": "fixture-source-1"}})
    }

    fn bootstrap() -> runtime::Bootstrap {
        let walk = json!({"parameter": "page[after]", "next": "/meta/after_cursor",
            "records": "/audits", "query": {"include_boundary_indicators": true},
            "continues": {"pointer": "/meta/has_more", "equals": true},
            "ends": [{"name": "has_more_false", "when": {"pointer": "/meta/has_more", "equals": false}},
                     {"name": "no_cursor", "absent": "/meta/after_cursor"},
                     {"name": "no_records", "empty": "/audits"}]});
        let descriptor = connectors_core::Descriptor {
            version: "v1alpha1".into(),
            instance: "forge-local".into(),
            adapter: "catalog".into(),
            revision: REVISION.into(),
            operations: vec![connectors_core::Operation {
                id: AUDITS.into(),
                description: "ticket.audits of one ticket".into(),
                contract: "operations/v1alpha1".into(),
                profile: "resource".into(),
                input_schema: json!({"type":"object","additionalProperties":false,
                    "properties":{"ticket_id":{"type":"integer"},
                        "page[size]":{"type":"integer"},"page[after]":{"type":"string"},
                        "include_boundary_indicators":{"type":"boolean"}},
                    "required":["ticket_id"],"x-connectors-walk":walk}),
                output_schema: json!({"type":"object","required":["status","body","provenance"]}),
            }],
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

    /// Two pages of audits; each invocation appends its input to `/work/inputs`.
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
            _: &str,
            _: &str,
            _: connectors_sdk::Secret,
            input: Value,
        ) -> runtime::Result<Value> {
            let mut log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(Path::new("/work/inputs"))
                .unwrap();
            writeln!(log, "{input}").unwrap();
            let ticket = input["ticket_id"].clone();
            Ok(match input.get("page[after]") {
                None => catalog(json!({"audits": [{"id": 9101, "ticket_id": ticket}],
                    "meta": {"has_more": true, "after_cursor": CURSOR}})),
                Some(_) => catalog(json!({"audits": [{"id": 9102, "ticket_id": ticket}],
                    "meta": {"has_more": false}})),
            })
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

    /// story:invoke-walks-declared-pages: each page is admitted as a plain
    /// invoke is, so a document a plain invoke refuses (an object naming one
    /// key twice) is refused by a walk the same way, before any page is read
    /// and without an `--out` file.
    #[test]
    fn a_walk_refuses_the_duplicate_key_document_a_plain_invoke_refuses() {
        let container = connected();
        let document = r#"{"ticket_id":101,"ticket_id":102}"#;
        let invoke = |extra: &[&str]| {
            let mut args = vec![
                "invoke",
                "zendesk/support",
                AUDITS,
                "--input-json",
                document,
            ];
            args.extend(extra);
            container.run(&args)
        };
        let plain = invoke(&[]);
        let _ = refusal(&plain);
        let envelope: Value = serde_json::from_slice(&plain.stderr).unwrap();
        assert_eq!(envelope["error"]["code"], "cli_dynamic_input", "{envelope}");
        assert!(
            container.seen("inputs").is_empty(),
            "the plain invoke read nothing"
        );
        let walked = invoke(&["--walk", "--out", "/work/walked.ndjson"]);
        assert_eq!(
            container.seen("inputs"),
            Vec::<String>::new(),
            "story:invoke-walks-declared-pages: the walk read pages of a document a plain invoke refuses: {}",
            String::from_utf8_lossy(&walked.stdout)
        );
        assert_eq!(walked.status.code(), plain.status.code(), "{walked:?}");
        assert_eq!(
            String::from_utf8_lossy(&walked.stderr),
            String::from_utf8_lossy(&plain.stderr)
        );
        assert!(walked.stdout.is_empty(), "{walked:?}");
        assert!(!container.path("/work/walked.ndjson").exists());
    }
}
