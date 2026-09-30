//! Adversary cases for story:failed-connect-reports-its-cause. A reply that
//! arrives but is not a well-formed answer says nothing definite about whether
//! the owner committed the publication or revalidation, so the caller must
//! still receive `outcome_unknown` (semantics: the publication acknowledgement
//! itself is uncertain). Each case drives the production owner client against
//! a stand-in owner socket.
use connectors_host::local::{
    config::{Config, Paths},
    owner::{Client, Code},
    registry, runtime,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    time::Duration,
};

const ACQUISITION: &str = "acquisition-adversary-1";

fn frame_bytes(control: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(&(control.len() as u32).to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame.extend_from_slice(control);
    frame
}

fn frame(control: &Value) -> Vec<u8> {
    frame_bytes(&serde_json::to_vec(control).unwrap())
}

fn read_control(stream: &mut UnixStream) -> Value {
    let mut sizes = [0u8; 12];
    stream.read_exact(&mut sizes).unwrap();
    let size = |at: usize| u32::from_be_bytes(sizes[at..at + 4].try_into().unwrap()) as usize;
    let mut control = vec![0; size(0)];
    stream.read_exact(&mut control).unwrap();
    let mut rest = vec![0; size(4) + size(8)];
    stream.read_exact(&mut rest).unwrap();
    serde_json::from_slice(&control).unwrap()
}

fn own_build() -> String {
    let image = fs::read("/proc/self/exe").unwrap();
    ring::digest::digest(&ring::digest::SHA256, &image)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn profile() -> runtime::Profile {
    runtime::Profile {
        id: "fixture.basic".into(),
        revision: "profile-1".into(),
        purpose: registry::Purpose::DelegatedUser,
        subject: registry::Subject::User,
        scheme: "http_basic".into(),
        capability: "http-basic".into(),
        minimum_scopes: BTreeSet::new(),
        evidence_lifetime_ms: 60_000,
        fields: vec![runtime::EntryField {
            name: "token".into(),
            label: "Token".into(),
            max_bytes: 1024,
        }],
        acquisition: None,
    }
}

/// What the stand-in owner writes after the work request arrived.
#[derive(Clone)]
enum Answer {
    /// These exact bytes, then the stream closes.
    Raw(Vec<u8>),
    /// Nothing; the stream stays open until the client gives up.
    Silent,
}

fn serve(
    answer: Answer,
    capture_ms: u64,
) -> (tempfile::TempDir, Paths, std::thread::JoinHandle<()>) {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    let socket = paths.state.join("owner.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
    let build = own_build();
    let owner = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let hello = read_control(&mut stream);
        stream
            .write_all(&frame(&json!({
                "kind": "hello",
                "version": hello["version"],
                "challenge": hello["challenge"],
                "host_incarnation": uuid::Uuid::new_v4().to_string(),
                "authority": hello["authority"],
                "build": build,
            })))
            .unwrap();
        loop {
            let request = read_control(&mut stream);
            if request["kind"] == "begin" {
                stream
                    .write_all(&frame(&json!({
                        "kind": "capture",
                        "acquisition": ACQUISITION,
                        "expires_at_ms": connectors_sdk::now_ms() + capture_ms,
                        "profile": profile(),
                    })))
                    .unwrap();
                continue;
            }
            match &answer {
                Answer::Raw(bytes) => {
                    let _ = stream.write_all(bytes);
                }
                Answer::Silent => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let mut sink = [0u8; 1];
                    let _ = stream.read(&mut sink);
                }
            }
            return;
        }
    });
    (root, paths, owner)
}

fn connect(answer: Answer, capture_ms: u64) -> connectors_host::local::owner::Error {
    let (_root, paths, owner) = serve(answer, capture_ms);
    let capture = Client::connect(&paths, false)
        .unwrap()
        .begin("forge", Some("fixture.basic".into()), None, None)
        .unwrap();
    let error = capture
        .complete(&connectors_sdk::Secret(br#"{"token":"fixture"}"#.to_vec()))
        .unwrap_err();
    owner.join().unwrap();
    error
}

fn revalidate(answer: Answer, deadline_ms: u64) -> connectors_host::local::owner::Error {
    let (_root, paths, owner) = serve(answer, 60_000);
    let error = Client::connect(&paths, false)
        .unwrap()
        .revalidate(
            "forge",
            "connection-1",
            "revision-1",
            connectors_sdk::now_ms() + deadline_ms,
        )
        .unwrap_err();
    owner.join().unwrap();
    error
}

/// A well-formed JSON control that is not any owner reply.
fn unknown_kind() -> Answer {
    Answer::Raw(frame(
        &json!({"kind": "published", "connection": "connection-1"}),
    ))
}

/// A control frame whose bytes are not JSON at all.
fn not_json() -> Answer {
    Answer::Raw(frame_bytes(b"\x00\x01not-json"))
}

/// A `failed` reply whose error carries a field the client does not know.
fn failed_with_unknown_field() -> Answer {
    Answer::Raw(frame(
        &json!({"kind": "failed", "error": {"code": "unavailable", "detail": "x"}}),
    ))
}

/// A header announcing more control bytes than ever arrive before the close.
fn cut_mid_frame() -> Answer {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&64u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(&0u32.to_be_bytes());
    bytes.extend_from_slice(br#"{"kind":"succ"#);
    Answer::Raw(bytes)
}

#[test]
fn a_connect_answered_with_an_unknown_reply_kind_is_outcome_unknown() {
    let error = connect(unknown_kind(), 60_000);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
    assert_eq!(error.acquisition.as_deref(), Some(ACQUISITION));
}

#[test]
fn a_connect_answered_with_a_non_json_frame_is_outcome_unknown() {
    let error = connect(not_json(), 60_000);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}

#[test]
fn a_connect_answered_with_an_unreadable_failed_reply_is_outcome_unknown() {
    let error = connect(failed_with_unknown_field(), 60_000);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}

#[test]
fn a_connect_reply_cut_mid_frame_is_outcome_unknown() {
    let error = connect(cut_mid_frame(), 60_000);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}

#[test]
fn a_connect_whose_reply_times_out_is_outcome_unknown() {
    let error = connect(Answer::Silent, 1_500);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}

#[test]
fn a_revalidation_answered_with_an_unknown_reply_kind_is_outcome_unknown() {
    let error = revalidate(unknown_kind(), 10_000);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}

#[test]
fn a_revalidation_reply_cut_mid_frame_is_outcome_unknown() {
    let error = revalidate(cut_mid_frame(), 10_000);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}

#[test]
fn a_revalidation_whose_reply_times_out_is_outcome_unknown() {
    let error = revalidate(Answer::Silent, 1_500);
    assert_eq!(error.code, Code::OutcomeUnknown, "{error:?}");
}
