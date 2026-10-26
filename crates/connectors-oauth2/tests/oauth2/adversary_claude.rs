//! Adversary cases for story:claude-subscription-connector against the grant core: the
//! `scope_in_refresh` field the unit added, and the pasted `code#state` split its pasted-code
//! profile relies on. Every token here is a fictional fixture value.
use crate::support::{
    ACCESS_TOKEN, FakeServer, NoTransport, REFRESH_TOKEN, Reply, declaration, query_value,
};
use connectors_oauth2::{Client, RedirectKind, RequestEncoding, Secret, pasted_code};

/// `scope_in_refresh` sends the declared `requested_scopes` joined by a space. A declaration
/// that requests no scope then has nothing to send: RFC 6749 §3.3 makes `scope` one or more
/// scope tokens, so an empty `scope` member is a malformed request, and the core already omits
/// an empty `scope` from the device authorization (`Client::device_authorize`). Either the
/// declaration is refused, or the refresh carries no empty `scope`.
#[test]
fn scope_in_refresh_with_no_requested_scope_sends_no_empty_scope() {
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, None, None)]);
    let mut declared = declaration(Some(&server));
    declared.refresh_encoding = RequestEncoding::Json;
    declared.requested_scopes = Vec::new();
    declared.required_scopes = Vec::new();
    declared.scope_in_refresh = true;
    let Ok(client) = Client::new(declared, None, None, server.transport()) else {
        return;
    };
    client
        .refresh(&Secret::new(REFRESH_TOKEN.to_owned()), &[])
        .unwrap();
    let observed = server.observed();
    assert_eq!(observed.len(), 1);
    let fields = observed[0].fields();
    assert_ne!(
        fields.get("scope").map(String::as_str),
        Some(""),
        "the refresh carried an empty scope: {:?}",
        fields.keys()
    );
}

/// The story's Provider facts: Claude Code splits the pasted line on `#` into exactly a code and
/// a state (`let[h,p]=c.split("#")`), so in `a#b#c` the state is `b`. The core splits on the
/// last `#` instead and accepts `fixture-code#junk#<state>` as the code `fixture-code#junk`,
/// a line Claude Code refuses as a state mismatch. A line with more than one `#` is not the
/// provider's `code#state` and is refused.
#[test]
fn a_pasted_line_with_two_hashes_is_refused() {
    let mut declared = declaration(None);
    declared.redirect_kind = RedirectKind::PastedCode;
    declared.redirect_uri = Some("https://localhost:1/oauth/code/callback".to_owned());
    let client = Client::new(declared, None, None, NoTransport).unwrap();
    let authorization = client
        .begin("https://localhost:1/oauth/code/callback")
        .unwrap();
    let state = query_value(authorization.url(), "state");
    let line = format!("fixture-code#junk#{state}");
    let accepted = pasted_code(&authorization, &line).map(|code| code.expose().to_owned());
    assert!(
        accepted.is_err(),
        "a line with two `#` was accepted as code {:?}",
        accepted.ok()
    );
}
