//! Round 3 of the security review of
//! story:standing-grant-binds-a-pattern-or-a-value-set, after fix round 2:
//! the reader matches a pattern member with the PikeVM alone and against no
//! value longer than the member's `max_value_bytes`. Every case drives the
//! public reader `local::standing::parse`, the check of
//! `approvals grant-standing` (`local::standing::check_request`) or the
//! served host, from contracts/service/compatibility.md § 2.1 and the guides.
//!
//! - A value over the bound is outside every pattern, as `approval_required`.
//! - The whole value matches, as `regex::Regex` matches `\A(?:pattern)\z`.
//! - One function computes the bound; a stored one is refused.
//! - What one check, one read and both together cost for the families the
//!   weight prices by a constant.
//! - The patterns the texts call refused are refused.
//! - The long values of the cases fix round 2 edited are outside their
//!   grants by the bound alone.
//! - What `list-standing` shows of a member.
#![cfg(unix)]
use crate::support::{LIVE, PEAK};
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result as CoreResult, WIRE_VERSION,
    v1alpha2::{ErrorCode, InvokeResponse},
};
use connectors_host::{
    local::standing::{self, Bound, Grants, Refused, Uncovered, Unreadable, Uses},
    server::{ServiceState, router_with_state},
};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const INSTANCE: &str = "leaf";
const OPERATION: &str = "issue.transition.run";
const LATER: i64 = 2_000_000_000_000;
const TOKEN: &str = "service-token";

/// compatibility.md § 2.1: "no more than 5 MiB at any moment".
const STATED_BOUND: isize = 5 * 1024 * 1024;
/// compatibility.md § 2.1: "a file of 64 such members 3,950,582 bytes".
const STATED_ONE_READ: isize = 3_950_582;

/// One class of 64 byte ranges, the characters of odd code point from U+0001
/// to U+007F: the class of the contract's costliest pattern.
const CLASS_64: &str = r"[\x01\x03\x05\x07\x09\x0b\x0d\x0f\x11\x13\x15\x17\x19\x1b\x1d\x1f!#%')+\-/13579;=?ACEGIKMOQSUWY\[\]_acegikmoqsuwy{}\x7f]";

fn live() -> isize {
    LIVE.with(Cell::get)
}

/// The CPU time this thread has used.
fn cpu() -> Duration {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid timespec this call writes.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    assert_eq!(status, 0);
    Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
}

fn grant(id: &str, target: Value) -> Value {
    json!({
        "id": id, "instance": INSTANCE, "operation": OPERATION,
        "target": target,
        "not_after_unix_ms": LATER, "max_per_hour": 1
    })
}

fn document(grants: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"format": standing::FORMAT, "grants": grants, "revoked": []}))
        .unwrap()
}

/// A grant file of `members` grants, each with the one target member
/// `{"pointer": "/k", "pattern": pattern}`.
fn file_of(pattern: &str, members: usize) -> Vec<u8> {
    document(
        (0..members)
            .map(|n| {
                grant(
                    &format!("g{n}"),
                    json!([{"pointer": "/k", "pattern": pattern}]),
                )
            })
            .collect(),
    )
}

/// The grants of a file whose one member holds `pattern`; `None` when the
/// reader refuses the file as malformed.
fn taken(pattern: &str) -> Option<Grants> {
    match standing::parse(&file_of(pattern, 1)) {
        Ok(grants) => Some(grants),
        Err(Unreadable::Malformed) => None,
        Err(other) => panic!("{pattern:?}: {other:?}"),
    }
}

/// The `max_value_bytes` of the first target member of the first grant.
fn bound_of(grants: &Grants) -> usize {
    match &grants.grants()[0].target[0].bound {
        Bound::Pattern(pattern) => pattern.max_value_bytes(),
        other => panic!("not a pattern member: {other:?}"),
    }
}

/// How many grants cover the write `{"k": value}`, or why none does.
fn covering(grants: &Grants, value: &Value) -> Result<usize, Uncovered> {
    grants
        .covering(INSTANCE, OPERATION, &json!({"k": value}), 1_000)
        .map(|covering| covering.len())
}

/// The answer of `approvals grant-standing`'s check for a request whose one
/// member is `member`.
fn requested(member: &Value) -> Result<(), Refused> {
    let request = json!({
        "operation": OPERATION, "target": [member],
        "not_after_unix_ms": LATER, "max_per_hour": 1
    });
    standing::check_request(&serde_json::to_vec(&request).unwrap(), INSTANCE)
}

/// `\A(?:pattern)\z` as the `regex` crate reads it with all its engines,
/// under limits no pattern here meets.
fn reference(pattern: &str) -> regex::Regex {
    regex::RegexBuilder::new(&format!(r"\A(?:{pattern})\z"))
        .size_limit(1 << 28)
        .dfa_size_limit(1 << 28)
        .build()
        .unwrap_or_else(|error| panic!("{pattern:?}: {error}"))
}

/// The largest `n` of 1 to 4,096 for which `holds(n)`, which holds for 1 and
/// for every `n` under the largest.
fn largest(holds: impl Fn(usize) -> bool) -> usize {
    assert!(holds(1));
    let (mut low, mut high) = (1, 4_097);
    while low + 1 < high {
        let middle = (low + high) / 2;
        if holds(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

// ---------------------------------------------------------------------------
// I1: fail closed at the bound.
// ---------------------------------------------------------------------------

/// A value one byte over a member's `max_value_bytes` is outside the member
/// whatever the pattern would answer, also a pattern that matches every
/// string: the reason is `TooLong`, the value is not matched (a megabyte is
/// refused in no time), and the bound counts bytes, not characters. A value
/// of exactly the bound is matched. A value that is no string is another
/// target, never too long. Another grant that admits the long value still
/// covers the write, and only that grant does.
#[test]
fn a_value_over_its_bound_is_outside_every_pattern() {
    for pattern in [
        "^(?s:.*)$",
        "^(?s).*$",
        "^(?s:.*?)$",
        r"^[\s\S]*$",
        "^(?s:.*)|x$",
        "^(?:(?s:.)*)*$",
        "^(?U)(?s:.*)$",
        "^(?m)(?s:.*)$",
        "^(?s:.*)\n?$",
    ] {
        let grants = taken(pattern).unwrap_or_else(|| panic!("{pattern:?} is not taken"));
        let bound = bound_of(&grants);
        assert!(bound >= 64, "{pattern:?}: {bound}");
        let whole = reference(pattern);
        let (at, over) = ("a".repeat(bound), "a".repeat(bound + 1));
        assert!(whole.is_match(&at) && whole.is_match(&over), "{pattern:?}");
        assert_eq!(
            covering(&grants, &json!(at)),
            Ok(1),
            "{pattern:?} at {bound}"
        );
        assert_eq!(
            covering(&grants, &json!(over)),
            Err(Uncovered::TooLong),
            "{pattern:?} one byte over {bound}"
        );
        // Bytes, not characters: fewer characters than the bound, more bytes.
        let wide = "\u{e9}".repeat(bound / 2 + 1);
        assert!(wide.chars().count() <= bound && wide.len() > bound);
        assert!(whole.is_match(&wide), "{pattern:?}");
        assert_eq!(
            covering(&grants, &json!(wide)),
            Err(Uncovered::TooLong),
            "{pattern:?} on {} characters of {} bytes",
            wide.chars().count(),
            wide.len()
        );
        // The program is not run on a longer value.
        let far = json!({"k": "a".repeat(1 << 20)});
        let started = cpu();
        let answer = grants.covering(INSTANCE, OPERATION, &far, 1_000);
        let took = cpu() - started;
        assert_eq!(answer.err(), Some(Uncovered::TooLong), "{pattern:?}");
        assert!(took < Duration::from_millis(100), "{pattern:?}: {took:?}");
        for other in [json!(12), json!(null), json!([at]), json!({"a": at})] {
            assert_eq!(
                covering(&grants, &other),
                Err(Uncovered::OtherTarget),
                "{pattern:?} on {other}"
            );
        }
    }
    // A long value another grant names exactly is covered by that grant only.
    let pattern = "^(?s:.*)$";
    let over = "a".repeat(bound_of(&taken(pattern).unwrap()) + 1);
    let file = document(vec![
        grant("g-any", json!([{"pointer": "/k", "pattern": pattern}])),
        grant("g-exact", json!([{"pointer": "/k", "value": over}])),
        grant("g-any-2", json!([{"pointer": "/k", "pattern": pattern}])),
    ]);
    assert!(file.len() <= standing::FILE_LIMIT, "{}", file.len());
    let grants = standing::parse(&file).unwrap();
    let ids: Vec<String> = grants
        .covering(INSTANCE, OPERATION, &json!({"k": over}), 1_000)
        .unwrap()
        .into_iter()
        .map(|grant| grant.id.clone())
        .collect();
    assert_eq!(ids, ["g-exact"]);
}

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> CoreResult<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

struct Fixture {
    requests: Arc<AtomicUsize>,
}
#[async_trait]
impl Adapter for Fixture {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: INSTANCE.into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![Operation {
                id: OPERATION.into(),
                description: format!("{OPERATION} fixture"),
                contract: "operations/v1alpha1".into(),
                profile: "mutation".into(),
                input_schema: json!({"type":"object"}),
                output_schema: json!({"type":"object"}),
            }],
        }
    }
    async fn invoke(&self, operation: &str, _input: Value) -> CoreResult<Value> {
        let number = self.requests.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(json!({"operation": operation, "number": number}))
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    requests: Arc<AtomicUsize>,
    _root: tempfile::TempDir,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A served host with standing grants read from `grants`, a path under
/// `root` (as `http_standing_grant_adversary.rs` starts one).
async fn start(root: tempfile::TempDir, grants: PathBuf) -> Host {
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        requests: requests.clone(),
    });
    let state = Arc::new(
        ServiceState::open(&path, INSTANCE, "fixture", "rev-1")
            .unwrap()
            .with_standing_grants(Some(grants))
            .unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_state(adapter, Arc::new(FixedSecret), Some(state.clone()));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        endpoint: format!("http://{address}/v1alpha2/invoke"),
        task,
        state,
        requests,
        _root: root,
    }
}

fn put(path: &Path, document: &[u8]) {
    let staged = path.with_extension("new");
    std::fs::write(&staged, document).unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::rename(&staged, path).unwrap();
}

async fn invoke(
    endpoint: &str,
    request_id: &str,
    key: &str,
    input: Value,
) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let body = json!({"version": "v1alpha2", "request_id": request_id, "operation": OPERATION,
        "revision": "rev-1", "input": input, "idempotency_key": key});
    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(&body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode_keyed(&bytes).unwrap())
}

/// The served host, end to end: a keyed write whose target value is one byte
/// over the member's `max_value_bytes` is answered as a write to another
/// target is (403 `approval_required`, the same message, nothing to retry
/// after), before any attempt, use or provider request. The same write with a
/// value of exactly the bound is sent.
#[tokio::test]
async fn a_served_write_over_the_bound_is_approval_required_and_reaches_no_provider() {
    let pattern = "^DEV-[0-9]+$";
    let bound = bound_of(&taken(pattern).unwrap());
    let ticket = |bytes: usize| json!({"k": format!("DEV-{}", "1".repeat(bytes - 4))});
    let root = tempfile::tempdir().unwrap();
    let grants = root.path().join("grants.json");
    let mut file: Value = serde_json::from_slice(&file_of(pattern, 1)).unwrap();
    file["grants"][0]["max_per_hour"] = json!(5);
    put(&grants, &serde_json::to_vec(&file).unwrap());
    let store = root.path().join("state");
    let host = start(root, grants).await;
    let rows = || {
        Uses::existing(&store)
            .unwrap()
            .map(|uses| uses.rows().unwrap())
    };
    let refusal = |response: &InvokeResponse| {
        response
            .error
            .as_ref()
            .map(|error| (error.code, error.message.clone(), error.retry_after_seconds))
    };

    let (status, long) = invoke(&host.endpoint, "w-1", "key-1", ticket(bound + 1)).await;
    assert_eq!(status, 403, "{long:?}");
    assert_eq!(
        refusal(&long).map(|refusal| refusal.0),
        Some(ErrorCode::ApprovalRequired),
        "{long:?}"
    );
    assert!(long.audit_ref.is_none() && long.mutation.is_none());
    assert_eq!(host.state.attempt_count().unwrap(), 0);
    assert_eq!(host.requests.load(Ordering::SeqCst), 0);
    assert_eq!(rows(), Some(0));

    // The answer names no axis: it is the answer to another target.
    let (status, other) = invoke(&host.endpoint, "w-2", "key-2", json!({"k": "OPS-12"})).await;
    assert_eq!(status, 403, "{other:?}");
    assert_eq!(refusal(&long), refusal(&other));

    // The control: a value of exactly the bound is inside the grant.
    let (status, admitted) = invoke(&host.endpoint, "w-3", "key-3", ticket(bound)).await;
    assert_eq!((status, refusal(&admitted)), (200, None), "{admitted:?}");
    assert_eq!(host.requests.load(Ordering::SeqCst), 1);
    assert_eq!(rows(), Some(1));
}

// ---------------------------------------------------------------------------
// I2: the whole value.
// ---------------------------------------------------------------------------

/// The reader answers as `regex::Regex` does on `\A(?:pattern)\z` for every
/// value within the bound, on the shapes that make a part of a value enough
/// under a search: a trailing line feed, the multi-line and CRLF flags, an
/// empty alternate, a top-level alternation, a pattern that itself holds
/// `\A`, `\z` or `$`, a comment under `(?x)`. Each base value is also tried
/// with a prefix and a suffix; no answer may differ.
#[test]
fn the_whole_value_matches_under_every_anchor_a_signer_can_write() {
    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("^a$", vec!["a", ""]),
        ("^a\n?$", vec!["a", "a\n"]),
        ("^(?m)a$", vec!["a", "a\nb"]),
        ("^(?m:a$)$", vec!["a"]),
        ("^(?m)a$\n^b$", vec!["a\nb", "a"]),
        ("^(?m)^a$|^b$", vec!["a", "b"]),
        ("^(?m:^a$\n?)+$", vec!["a", "a\na", "a\n"]),
        ("^(?Rm)a$", vec!["a", "a\r"]),
        ("^(?R)a.$", vec!["ab", "a\r"]),
        ("^|a$", vec!["", "a"]),
        ("^a|$", vec!["", "a"]),
        ("^a||b$", vec!["", "a", "b"]),
        ("^(?:a|)$", vec!["", "a"]),
        ("^(?:|a)+$", vec!["", "aa"]),
        ("^a|b$", vec!["a", "b", "ab"]),
        ("^a$|^b$", vec!["a", "b"]),
        (r"^a\z$", vec!["a"]),
        (r"^a\z|b$", vec!["a", "b"]),
        (r"^\Aa$", vec!["a"]),
        (r"^a|\Ab\z|c$", vec!["a", "b", "c"]),
        ("^a$$", vec!["a"]),
        ("^^a$", vec!["a"]),
        ("^a$b$", vec!["a", "ab"]),
        ("^a(?m:$)\n(?m:^)b$", vec!["a\nb"]),
        ("^(?x) a | b # c\n$", vec!["a", "b"]),
        ("^(?x)a#$\n$", vec!["a"]),
        ("^(?U)a*$", vec!["", "aa"]),
        ("^a*?$", vec!["", "aa"]),
        ("^$", vec![""]),
        ("^(?:)$", vec![""]),
        ("^.$", vec!["a", "\n", "\r"]),
        ("^(?s:.)$", vec!["a", "\n"]),
        ("^[^a]$", vec!["b", "\n"]),
        (r"^\$$", vec!["$"]),
        (r"^\b\w+\b$", vec!["ab", "a b"]),
        ("^DEV-[0-9]+$", vec!["DEV-12"]),
        (
            "^Release review of (?s:.*)$",
            vec!["Release review of 2.46.8"],
        ),
        ("^(?i-u:sk)-[0-9]+$", vec!["SK-7", "sk-7"]),
    ];
    let around = [
        "\n", "\r", "\r\n", "x", "a", "b", " ", "\u{e9}", "\u{2028}", "$",
    ];
    let (mut compared, mut matched) = (0, 0);
    for (pattern, bases) in cases {
        let grants = taken(pattern).unwrap_or_else(|| panic!("{pattern:?} is not taken"));
        let (bound, whole) = (bound_of(&grants), reference(pattern));
        let mut values: Vec<String> = bases.iter().map(|base| (*base).to_owned()).collect();
        for base in &bases {
            for part in around {
                values.push(format!("{base}{part}"));
                values.push(format!("{part}{base}"));
                values.push(format!("{part}{base}{part}"));
            }
            values.push(format!("{base}{base}"));
            values.push(format!("{base}\n{base}"));
        }
        values.extend(around.iter().map(|part| (*part).to_owned()));
        for value in values {
            assert!(value.len() <= bound);
            let answer = covering(&grants, &json!(value)).is_ok();
            assert_eq!(
                answer,
                whole.is_match(&value),
                "{pattern:?} on {value:?}: the reader and regex::Regex on the wrap differ"
            );
            compared += 1;
            matched += usize::from(answer);
        }
    }
    println!("{compared} answers compared, {matched} of them a match");
    assert!(
        matched >= 60 && compared - matched >= 500,
        "{matched} of {compared}"
    );
}

// ---------------------------------------------------------------------------
// I3: one function, no stored bound.
// ---------------------------------------------------------------------------

/// The file does not hold `max_value_bytes` (ess/domains/standing_approvals.yaml,
/// contracts/cli/v1alpha1/semantics.md "which the command computes from the
/// pattern and the file does not hold"): a member that stores one is refused,
/// by the reader and by `grant-standing`, also when the number is the one the
/// reader computes. So a file carries no bound of its own, and the member
/// `list-standing` shows is no member a file may hold.
#[test]
fn a_stored_max_value_bytes_is_refused_not_read() {
    let pattern = "^DEV-[0-9]+$";
    let computed = bound_of(&taken(pattern).unwrap());
    let plain = json!({"pointer": "/k", "pattern": pattern});
    assert!(standing::parse(&document(vec![grant("g1", json!([plain]))])).is_ok());
    assert_eq!(requested(&plain), Ok(()));
    for stored in [computed, computed + 1, 1 << 40, 63, 0] {
        let member = json!({"pointer": "/k", "pattern": pattern, "max_value_bytes": stored});
        assert_eq!(
            standing::parse(&document(vec![grant("g1", json!([member]))])).err(),
            Some(Unreadable::Malformed),
            "a file member that stores {stored}"
        );
        assert_eq!(
            requested(&member),
            Err(Refused::InvalidInput),
            "a request member that stores {stored}"
        );
    }
    // What `list-standing` shows of the member is such a member.
    let grants = taken(pattern).unwrap();
    let listed = grants.grants()[0].target[0].listed();
    assert_eq!(listed["max_value_bytes"], json!(computed));
    assert_eq!(
        standing::parse(&document(vec![grant("g1", json!([listed]))])).err(),
        Some(Unreadable::Malformed)
    );
    // Nor does a grant or the document hold one.
    let mut with = grant("g1", json!([plain]));
    with["max_value_bytes"] = json!(1_u64 << 40);
    assert_eq!(
        standing::parse(&document(vec![with])).err(),
        Some(Unreadable::Malformed)
    );
}

/// `grant-standing` refuses exactly the patterns the reader runs on under 64
/// bytes: over three families of Unicode word boundaries that cross the
/// figure, the command's answer is the reader's `max_value_bytes >= 64` for
/// every size, so the two use one number. The reader takes each pattern, its
/// bound never rises with the size, and a value over it is `TooLong`.
#[test]
fn grant_standing_signs_exactly_the_patterns_the_reader_runs_on_64_bytes() {
    for (boundaries, sizes) in [(16, 55..=85), (8, 120..=150), (4, 250..=275)] {
        let (mut signed, mut refused, mut last) = (0, 0, usize::MAX);
        for size in sizes {
            let pattern = format!("^(?:{}-){{{size}}}$", r"\B".repeat(boundaries));
            let grants = taken(&pattern).unwrap_or_else(|| panic!("{pattern:?} is not taken"));
            let bound = bound_of(&grants);
            assert!(bound <= last, "{pattern:?}: {bound} after {last}");
            last = bound;
            let answer = requested(&json!({"pointer": "/k", "pattern": pattern}));
            assert_eq!(
                answer.is_ok(),
                bound >= 64,
                "{pattern:?} is matched against {bound} bytes and the command answers {answer:?}"
            );
            if answer.is_ok() {
                signed += 1;
            } else {
                assert_eq!(answer, Err(Refused::InvalidInput));
                refused += 1;
            }
            if bound < size {
                assert_eq!(
                    covering(&grants, &json!("-".repeat(size))),
                    Err(Uncovered::TooLong),
                    "{pattern:?}"
                );
            }
        }
        assert!(
            signed > 0 && refused > 0,
            "{boundaries} boundaries: {signed} signed, {refused} refused, last bound {last}"
        );
    }
}

// ---------------------------------------------------------------------------
// I4: what one check, one read and both together cost.
// ---------------------------------------------------------------------------

/// The CPU time of one check of the write `{"k": value}` against a file of 64
/// members of `pattern`, where the value is `unit` repeated up to the
/// member's `max_value_bytes`; the least of two checks when the first took
/// over 800 ms. Printed with the bound, and asserted under the second a
/// checked write may take.
fn checked_at_the_bound(pattern: &str, unit: &str) -> Duration {
    assert!(pattern.len() <= 256, "{} bytes: {pattern}", pattern.len());
    let file = file_of(pattern, 64);
    assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
    let grants = standing::parse(&file).unwrap_or_else(|refusal| panic!("{pattern}: {refusal:?}"));
    let bound = bound_of(&grants);
    let value = unit.repeat(bound / unit.len());
    assert!(value.len() <= bound && value.len() + unit.len() > bound);
    let expected = if reference(pattern).is_match(&value) {
        64
    } else {
        0
    };
    let input = json!({"k": value});
    let check = || {
        let started = cpu();
        let covering = grants
            .covering(INSTANCE, OPERATION, &input, 1_000)
            .map_or(0, |covering| covering.len());
        (cpu() - started, covering)
    };
    let (mut took, covering) = check();
    assert_eq!(covering, expected, "{pattern} on {} bytes", value.len());
    if took > Duration::from_millis(800) {
        took = took.min(check().0);
    }
    println!(
        "CHECK {took:>12?} for 64 members at {} of {bound} bytes, {covering} covering: {pattern}",
        value.len()
    );
    took
}

/// Runs each family at its bound and fails with every family that took a
/// second or more of CPU time. The first family is the contract's own
/// (`^x*x{150}$`), for the load of the host at this run.
fn families_stay_under_a_second(families: &[(String, &str)]) {
    let base = checked_at_the_bound("^x*x{150}$", "x");
    let over: Vec<String> = families
        .iter()
        .filter_map(|(pattern, unit)| {
            let took = checked_at_the_bound(pattern, unit);
            (took >= Duration::from_secs(1)).then(|| format!("{took:?}: {pattern}"))
        })
        .collect();
    assert!(
        over.is_empty() && base < Duration::from_secs(1),
        "one check of 64 members at their bounds took a second or more of CPU time \
         (`^x*x{{150}}$` took {base:?} in this run): {over:#?}"
    );
}

/// One check of 64 members, each on a value of its bound, for the Unicode
/// word boundary, which the weight prices at 32: on ASCII characters that are
/// no word character (two table searches a boundary and byte), on word
/// characters of one, two and four bytes, on a character of two bytes that is
/// no word character, for `\b` at every byte, for a half boundary, and for a
/// pattern that is boundaries almost only.
#[test]
fn one_check_at_the_bound_takes_under_a_second_for_unicode_word_boundaries() {
    let eight = r"\B".repeat(8);
    families_stay_under_a_second(&[
        (format!("^-*(?:{eight}-){{10}}$"), "-"),
        (format!("^-*(?:{eight}-){{40}}$"), "-"),
        (format!("^(?:{}-)*$", r"\B".repeat(120)), "-"),
        (format!("^x*(?:{eight}x){{10}}$"), "x"),
        (format!("^\u{e9}*(?:{eight}\u{e9}){{10}}$"), "\u{e9}"),
        (format!("^\u{a1}*(?:{eight}\u{a1}){{10}}$"), "\u{a1}"),
        (
            format!("^\u{10400}*(?:{eight}\u{10400}){{10}}$"),
            "\u{10400}",
        ),
        (format!("^[x-]*(?:{}[x-]){{10}}$", r"\b".repeat(8)), "x-"),
        (
            format!("^-*(?:{}-){{10}}$", r"\b{start-half}".repeat(8)),
            "-",
        ),
    ]);
}

/// The same for a class of 64 byte ranges on its last character (every
/// transition is searched), which the weight prices at 1 a transition, and
/// for unions of empty alternates, which it prices at 3 an alternate.
#[test]
fn one_check_at_the_bound_takes_under_a_second_for_classes_and_unions() {
    families_stay_under_a_second(&[
        (format!("^(?:{CLASS_64}*){{10}}$"), "\x7f"),
        (format!("^(?:{CLASS_64}*){{40}}$"), "\x7f"),
        (format!("^(?:{CLASS_64}*){{40}}$"), "}"),
        (format!("^(?:(?:{})x*){{40}}$", "|".repeat(16)), "x"),
        (format!("^(?:(?:{})x)*$", "|".repeat(240)), "x"),
        (format!("^(?:(?:{})x*){{4}}$", "|".repeat(60)), "x"),
        (
            "^(?:(?:[a-x]y|[b-x]z|[c-x]0|[d-x]1|[e-x]2|[f-x]3|[g-x]4|x)*){10}$".to_owned(),
            "x",
        ),
    ]);
}

/// The same for what the weight prices as a plain state: multi-line and CRLF
/// anchors, the ASCII word boundary, nested counted repetition, lazy
/// repetition, and a run of `.` on characters of two bytes.
#[test]
fn one_check_at_the_bound_takes_under_a_second_for_anchors_and_nested_repetition() {
    families_stay_under_a_second(&[
        ("^(?m:(?:$*^*\n*){50})$".to_owned(), "\n"),
        ("^(?Rm:(?:(?:$*^*\r*\n*)*){20})$".to_owned(), "\r\n"),
        (r"^(?-u:(?:\b*[x-]*){60})$".to_owned(), "x-"),
        ("^(?:(?:(?:x*){5}){5}){6}$".to_owned(), "x"),
        ("^(?:(?:(?:x?){6}){6}){6}x*$".to_owned(), "x"),
        ("^(?:x*?){150}$".to_owned(), "x"),
        ("^(?:.*){62}$".to_owned(), "\u{e9}"),
        ("^(?s:.*)a?x{150}$".to_owned(), "x"),
    ]);
}

/// Acceptance 25 of the story (one read of 64 patterns in under a second of
/// CPU time), on the one property whose class the translator sorts from
/// several tables: `\p{age=16.0}` is the ranges of every age table up to
/// Unicode 16.0, one after another, sorted for each use, and a pattern holds
/// it 20 times in 247 bytes. The class is cut to `a`, so the program is
/// small and the reader takes the file. The time is the least of three
/// reads. The read of the suite's own costliest class (82 times `\pL`) is
/// timed beside it for the load of the host at this run, and so is what the
/// compiler costs at the largest size taken; those are printed only.
#[test]
fn one_read_of_a_file_of_age_classes_takes_under_a_second() {
    let timed = |pattern: &str| {
        assert!(pattern.len() <= 256, "{} bytes", pattern.len());
        let file = file_of(pattern, 64);
        assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
        let read = || {
            let started = cpu();
            let grants = standing::parse(&file);
            let took = cpu() - started;
            assert_eq!(
                grants.map(|grants| grants.grants().len()).ok(),
                Some(64),
                "{pattern}"
            );
            took
        };
        let first = read();
        let least = first.min(read()).min(read());
        println!("READ {least:?} for 64 members (first read {first:?}): {pattern}");
        least
    };
    let known = timed(&format!("^[{}]$", r"\pL".repeat(82)));
    for make in [
        (|n| format!(r"^\w{{{n}}}$")) as fn(usize) -> String,
        |n| format!(r"^\pL{{{n}}}$"),
        |n| format!(r"^\pN{{{n}}}$"),
        |n| format!(r"^\d{{{n}}}$"),
        |n| format!("^[^a]{{{n}}}$"),
        |n| format!("^(?s:.){{{n}}}$"),
    ] {
        timed(&make(largest(|n| taken(&make(n)).is_some())));
    }
    let ages = [
        format!("^[{}&&a]$", r"\p{age=16.0}".repeat(20)),
        format!("^[^{}a]$", r"\P{age=16.0}".repeat(20)),
    ];
    // Added in fix round 3 (F1 of this review): the two contracts state the
    // read figure beside the check figure, for the second pattern as it is
    // written here. The time itself is held to a second only, below.
    assert_eq!(ages[1].len(), 246);
    for (name, text) in [
        (
            "contracts/service/compatibility.md",
            include_str!("../../../../contracts/service/compatibility.md"),
        ),
        (
            "contracts/cli/v1alpha1/semantics.md",
            include_str!("../../../../contracts/cli/v1alpha1/semantics.md"),
        ),
    ] {
        for stated in [
            r"`^[^\P{age=16.0}...a]$` with `\P{age=16.0}` written 20 times (246 bytes)",
            "about 360 ms of CPU time",
            "611 ms in one run under more load (measured, not a proven ceiling)",
        ] {
            assert!(text.contains(stated), "{name} does not state: {stated}");
        }
    }
    let over: Vec<String> = ages
        .iter()
        .filter_map(|pattern| {
            let took = timed(pattern);
            (took >= Duration::from_secs(1)).then(|| format!("{took:?}: {pattern}"))
        })
        .collect();
    assert!(
        over.is_empty(),
        "one read of a file within the stated limits took a second or more of CPU time (the \
         read of 82 times `\\pL` took {known:?} in this run): {over:#?}"
    );
}

/// compatibility.md § 2.1: one read and one checked write hold "no more than
/// 5 MiB at any moment". Measured for shapes the texts do not name: the
/// leading any-run before the class of 64 ranges at the largest size taken, a
/// large Unicode class beside that class, the age class, 60 classes of `\w`
/// nested in one another, and a file of 63 members of the costliest pattern
/// with one member of the highest peak. The figures are printed.
#[test]
fn one_read_and_one_checked_write_peak_under_5_mib_for_other_shapes() {
    let sized = |make: &dyn Fn(usize) -> String| {
        make(largest(|n| {
            let pattern = make(n);
            pattern.len() <= 256 && taken(&pattern).is_some()
        }))
    };
    let costliest = format!("^{CLASS_64}{{113}}$");
    let any_run = sized(&|n| format!("^(?s:.*){CLASS_64}{{{n}}}$"));
    let word = sized(&|n| format!(r"^\w?{CLASS_64}{{{n}}}$"));
    let letters = sized(&|n| format!(r"^(?s:.*)\pL?{CLASS_64}{{{n}}}$"));
    let age = format!("^[{}&&a]$", r"\p{age=16.0}".repeat(20));
    let nested = format!("^{}{}$", r"[\w".repeat(60), "]".repeat(60));
    let mut files: Vec<(String, Vec<u8>)> = [&costliest, &any_run, &word, &letters, &age, &nested]
        .into_iter()
        .map(|pattern| (pattern.clone(), file_of(pattern, 64)))
        .collect();
    let mixed: Vec<Value> = (0..63)
        .map(|n| {
            grant(
                &format!("g{n}"),
                json!([{"pointer": "/k", "pattern": costliest}]),
            )
        })
        .collect();
    for last in [&any_run, &nested] {
        let mut members = mixed.clone();
        members.push(grant("g63", json!([{"pointer": "/k", "pattern": last}])));
        files.push((
            format!("63 of the costliest and then {last}"),
            document(members),
        ));
    }
    let mut over = Vec::new();
    for (name, file) in &files {
        assert!(file.len() <= standing::FILE_LIMIT, "{} bytes", file.len());
        let input = json!({"k": "a".repeat(113)});
        let before = live();
        PEAK.with(|peak| peak.set(before));
        let grants = standing::parse(file).unwrap_or_else(|refusal| panic!("{name}: {refusal:?}"));
        let covering = grants
            .covering(INSTANCE, OPERATION, &input, 1_000)
            .map_or(0, |covering| covering.len());
        let holds = live() - before;
        let peak = PEAK.with(Cell::get) - before;
        drop(grants);
        println!(
            "HOLDS {holds} bytes after one read and one checked write, {peak} at the most, \
             {covering} covering: {name}"
        );
        assert!(holds <= peak, "{name}");
        if peak > STATED_BOUND || holds > STATED_ONE_READ + 65_536 {
            over.push(format!("{holds} held, {peak} at the most: {name}"));
        }
    }
    assert!(
        over.is_empty(),
        "one read and one checked write hold over {STATED_BOUND} bytes at some moment, or over \
         the stated {STATED_ONE_READ} and 65,536 bytes after it: {over:#?}"
    );
}

// ---------------------------------------------------------------------------
// I5: no pattern the texts call refused is taken.
// ---------------------------------------------------------------------------

/// The fold rules of the five texts, as written in docs/local-approvals.md
/// and contracts/service/compatibility.md § 2.1, held against the reader and
/// against `grant-standing`: a fold while Unicode matching is on is refused;
/// behind a bare `(?i-u)` and inside `(?i-u:...)` a `.`, a negated class and
/// `\W` are refused, in every spelling of a negated class found; `\d`, `\w`
/// and `\s` are ASCII only there; the named examples are taken or refused as
/// the texts say, and a fold admits no look-alike.
#[test]
fn the_fold_rules_of_the_texts_are_the_readers() {
    let both = |pattern: &str| {
        let file = taken(pattern);
        let request = requested(&json!({"pointer": "/k", "pattern": pattern}));
        assert_eq!(
            file.is_some(),
            request.is_ok(),
            "{pattern:?}: the reader and grant-standing differ ({request:?})"
        );
        file
    };
    for refused in [
        // A fold while Unicode matching is on.
        "^(?i)dev$",
        "^(?i:dev)$",
        "^(?i-u)de(?u)v$",
        "^(?i-u:de(?u)v)$",
        "^(?i-u:de(?u:v))$",
        "^(?-u)(?i)de(?u)v$",
        "^dev(?i)$",
        "^(?:a|(?i)b)$",
        "^(?i-u)Release review of (?s:.*)$",
        // Byte mode behind a bare fold and inside the group.
        "^(?i-u).$",
        "^(?i-u)a.*$",
        "^(?i-u:.)$",
        "^(?i-u)(?s:.)$",
        "^(?i-u)[^/]$",
        "^(?i-u)[^/]+$",
        "^(?i-u:[^/])$",
        r"^(?i-u)\W$",
        r"^(?i-u:\W)$",
        // Other spellings of a negated class there.
        r"^(?i-u)\D$",
        r"^(?i-u)\S$",
        r"^(?i-u)[^\W]$",
        r"^(?i-u)[^\D]$",
        r"^(?i-u)[\W]$",
        r"^(?i-u)[^\x00-\x7f]$",
        r"^(?i-u)[^\x80-\xff]$",
        "^(?i-u)[^[^a]]$",
        "^(?i-u)[[:^alpha:]]$",
        "^(?i-u)[^[:^alpha:]]$",
        "^(?i-u)[a&&[^b]]$",
    ] {
        assert!(
            both(refused).is_none(),
            "{refused:?} is taken; the texts call it refused"
        );
    }
    let admits = |grants: &Grants, value: &str| covering(grants, &json!(value)).is_ok();
    // A difference from the ASCII range is no negated class: it is taken and
    // admits ASCII only, without the letter in either case.
    let but_a = both(r"^(?i-u)[\x00-\x7f--a]$").expect("a class of ASCII characters");
    assert!(admits(&but_a, "b") && admits(&but_a, "/") && admits(&but_a, "\n"));
    for outside in ["a", "A", "\u{e9}", "\u{212a}", "\u{80}"] {
        assert!(!admits(&but_a, outside), "{outside:?}");
    }
    let dev = both("^(?i-u)dev$").expect("the texts call it taken");
    assert!(admits(&dev, "DEV") && admits(&dev, "dEv") && !admits(&dev, "dev "));
    let sk = both("^(?i-u:sk)-[0-9]+$").expect("the texts call it taken");
    assert!(admits(&sk, "SK-7") && admits(&sk, "sk-70") && admits(&sk, "Sk-7"));
    for look_alike in [
        "\u{17f}k-7",
        "s\u{212a}-7",
        "\u{17f}\u{212a}-7",
        "sk-\u{667}",
    ] {
        assert!(!admits(&sk, look_alike), "{look_alike:?}");
    }
    let review = both("^(?i-u:release review of )(?s:.*)$").expect("the texts call it taken");
    assert!(admits(&review, "RELEASE Review of \u{e9}\n2") && !admits(&review, "Release"));
    // ASCII only behind the fold: an Arabic-Indic digit, a letter outside
    // ASCII and a no-break space are a digit, a word character and a space
    // of Unicode, and none of them here.
    for (pattern, inside, outside) in [
        (r"^(?i-u)\d$", "7", "\u{667}"),
        (r"^(?i-u)\w$", "k", "\u{e9}"),
        (r"^(?i-u)\s$", " ", "\u{a0}"),
        (r"^(?i-u:\d)$", "7", "\u{667}"),
        (r"^(?i-u)[\d\w\s]$", "_", "\u{212a}"),
        (r"^(?i-u)k$", "K", "\u{212a}"),
        (r"^(?i-u)[a-z]$", "S", "\u{17f}"),
    ] {
        let grants = both(pattern).unwrap_or_else(|| panic!("{pattern:?} is not taken"));
        assert!(admits(&grants, inside), "{pattern:?} on {inside:?}");
        assert!(!admits(&grants, outside), "{pattern:?} on {outside:?}");
        // With Unicode on, the same class admits the character.
        let unicode = pattern.replace("(?i-u)", "").replace("(?i-u:", "(?:");
        if unicode.contains('\\') {
            let grants = both(&unicode).unwrap_or_else(|| panic!("{unicode:?}"));
            assert!(admits(&grants, outside), "{unicode:?} on {outside:?}");
        }
    }
}

/// The reader takes a pattern without a capture group exactly when the
/// `regex` crate compiles its wrap under the same limit (compatibility.md
/// § 2.1: "it takes the patterns the `regex` crate compiles under that
/// limit"): for each family the largest size the reader takes is the largest
/// the crate compiles with `size_limit` 65,536. A reader that dropped the
/// reverse program, or sized the forward one otherwise, takes a larger one.
#[test]
fn the_reader_takes_what_the_regex_crate_compiles_under_the_size_limit() {
    type Family = fn(usize) -> String;
    let families: [(&str, Family); 13] = [
        // Added in fix round 3 (F4 of this review): the three families the
        // guide names, held to the guide's sentence below.
        ("\\w{n}", |n| format!(r"^\w{{{n}}}$")),
        ("\\pL{n}", |n| format!(r"^\pL{{{n}}}$")),
        ("[A-Za-z0-9_]{n}", |n| format!("^[A-Za-z0-9_]{{{n}}}$")),
        ("x{n}", |n| format!("^x{{{n}}}$")),
        ("any x{n}", |n| format!("^(?s:.*)x{{{n}}}$")),
        ("[a-z]{n}", |n| format!("^[a-z]{{{n}}}$")),
        (".?{n}", |n| format!("^(?:.?){{{n}}}$")),
        ("x?{n}", |n| format!("^(?:x?){{{n}}}$")),
        ("\\d{n}", |n| format!(r"^\d{{{n}}}$")),
        ("\\b-{n}", |n| format!(r"^(?:\b-){{{n}}}$")),
        ("alternates{n}", |n| format!("^(?:a|bc|def){{{n}}}$")),
        ("fold{n}", |n| format!("^(?i-u:x*x{{{n}}})$")),
        ("class64{n}", |n| format!("^{CLASS_64}{{{n}}}$")),
    ];
    let mut taken_up_to = std::collections::BTreeMap::new();
    for (name, make) in families {
        let reader = largest(|n| taken(&make(n)).is_some());
        let elsewhere = largest(|n| {
            regex::RegexBuilder::new(&format!(r"\A(?:{})\z", make(n)))
                .size_limit(65_536)
                .dfa_size_limit(65_536)
                .build()
                .is_ok()
        });
        println!("LARGEST {name}: the reader takes {reader}, the regex crate {elsewhere}");
        assert!(reader < 4_096, "{name}: no size is refused");
        assert_eq!(
            reader, elsewhere,
            "{name}: the reader takes up to {reader} and the regex crate compiles up to \
             {elsewhere} under the same limit"
        );
        taken_up_to.insert(name, reader);
    }
    // docs/local-approvals.md, "A grant for many targets": the sentence on a
    // counted repetition of a Unicode class states these sizes.
    assert_eq!(
        (taken_up_to["\\w{n}"], taken_up_to["\\pL{n}"]),
        (1, 1),
        "the guide calls `^\\w{{2}}$` and `^\\pL{{2}}$` refused"
    );
    assert!(taken(r"^\w{2}$").is_none() && taken(r"^\pL{2}$").is_none());
    let sentence = format!(
        "A counted repetition of a Unicode class passes the compiled-size limit quickly: \
         `^\\w{{2}}$` and `^\\pL{{2}}$` are refused, and `\\d` is taken up to {} repetitions. \
         Write the ASCII class where ASCII is meant: `[A-Za-z0-9_]` is taken up to {} \
         repetitions.",
        taken_up_to["\\d{n}"], taken_up_to["[A-Za-z0-9_]{n}"]
    );
    let guide = include_str!("../../../../docs/local-approvals.md")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        guide.contains(&sentence),
        "docs/local-approvals.md does not state: {sentence}"
    );
}

// ---------------------------------------------------------------------------
// I6: the cases fix round 2 edited.
// ---------------------------------------------------------------------------

/// Fix round 2 changed five cases from "all 64 grants cover the long value"
/// to "none does" (its edits 3, 4, 5, 8 and 12). Each of those long values
/// matches its pattern as the `regex` crate reads the wrap, and the reader's
/// one reason for the refusal is `TooLong`: so each edited assertion holds by
/// the work bound alone and fails on a reader without it.
#[test]
fn the_long_values_of_the_edited_cases_are_outside_by_the_bound_alone() {
    let groups = "()".repeat(100);
    for (edit, pattern, value) in [
        (3, "^(?s:.*)a?x{2007}$".to_owned(), "x".repeat(3_000)),
        (
            4,
            "^DEV-[0-9]+$".to_owned(),
            format!("DEV-{}", "1".repeat(60_000)),
        ),
        (
            4,
            "^Release review of (?s:.*)$".to_owned(),
            format!("Release review of {}", "line \u{e9}\n".repeat(60_000 / 7)),
        ),
        (5, "^x*x{2000}$".to_owned(), "x".repeat(4_000)),
        (
            8,
            format!(r"^{groups}[^\x00-\x7f]?-{{1725}}$"),
            "-".repeat(1_725),
        ),
        (12, "^(?s:.*)(?:a?x{2004}|b)$".to_owned(), "x".repeat(2_004)),
    ] {
        let grants = taken(&pattern).unwrap_or_else(|| panic!("{pattern:?} is not taken"));
        let bound = bound_of(&grants);
        assert!(value.len() > bound, "edit {edit}: {pattern:?}, {bound}");
        assert!(
            reference(&pattern).is_match(&value),
            "edit {edit}: {pattern:?} does not match its long value, so no bound is needed to \
             refuse it"
        );
        assert_eq!(
            covering(&grants, &json!(value)),
            Err(Uncovered::TooLong),
            "edit {edit}: {pattern:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// I7: what `list-standing` shows of a member.
// ---------------------------------------------------------------------------

/// A listed member is the member as the file holds it, and a pattern member
/// with `max_value_bytes`, the reader's own number, beside it: no other key
/// is added to any of the three forms, and what is written back to a file
/// (the grant as it serializes) holds no such key.
#[test]
fn a_listed_member_adds_max_value_bytes_to_a_pattern_and_nothing_else() {
    let target = json!([
        {"pointer": "/k", "pattern": "^Release review of (?s:.*)$"},
        {"pointer": "/c", "value": {"id": "C1", "n": 7}},
        {"pointer": "/s", "one_of": ["a", null, 3]},
        {"pointer": "/n", "value": null}
    ]);
    let grants = standing::parse(&document(vec![grant("g1", target.clone())])).unwrap();
    let granted = &grants.grants()[0];
    let listed: Vec<Value> = granted
        .target
        .iter()
        .map(|member| member.listed())
        .collect();
    let mut expected = target.clone();
    expected[0]["max_value_bytes"] = json!(bound_of(&grants));
    assert_eq!(json!(listed), expected);
    assert_eq!(expected[0]["max_value_bytes"], json!(8_192));
    let written = serde_json::to_value(granted).unwrap();
    assert_eq!(written["target"], target);
    assert!(!written.to_string().contains("max_value_bytes"));
}
