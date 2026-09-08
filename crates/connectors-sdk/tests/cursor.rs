use connectors_sdk::Cursors;
use serde_json::json;

#[test]
fn cursors_cannot_be_rebound_forged_or_reused_after_restart() {
    let cursors = Cursors::default();
    let context = json!({"source":"one","query":"x","identity":"reader"});
    let token = cursors.issue(&context, "next-page".into()).unwrap();
    assert_eq!(cursors.read(&context, &token).unwrap(), "next-page");
    assert!(
        cursors
            .read(
                &json!({"source":"two","query":"x","identity":"reader"}),
                &token
            )
            .is_err()
    );
    assert!(Cursors::default().read(&context, &token).is_err());
    let mut bytes = token.into_bytes();
    bytes[5] = if bytes[5] == b'a' { b'b' } else { b'a' };
    assert!(
        cursors
            .read(&context, std::str::from_utf8(&bytes).unwrap())
            .is_err()
    );
}
