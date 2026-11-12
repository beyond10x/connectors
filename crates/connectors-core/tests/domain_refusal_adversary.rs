//! story:google-read-only-profiles-and-company-domain, second adversarial
//! pass: the account-domain refusal driven from the contract that names it
//! (contracts/cli/v1alpha1/semantics.md, the paragraph after "Bound.").
use connectors_core::reason;

fn domains(list: &[&str]) -> Vec<String> {
    list.iter().map(|domain| (*domain).to_owned()).collect()
}

/// semantics.md: the host "still withholds it when it holds a piece of the
/// protected document". A client secret or refresh token is never a public
/// name, whatever its shape; one made of lower-case letters and dots is echoed
/// whole here and must be withheld.
#[test]
fn adv2_a_domain_refusal_echoing_a_dotted_client_secret_is_withheld() {
    for (field, value) in [
        ("client_secret", "correct.horse.battery.staple"),
        ("refresh_token", "fixture-refresh.0a1b2c3d4e5f6a7b"),
        ("token", "hunter2.example"),
    ] {
        let document =
            format!(r#"{{"client_id":"fixture-client-id.apps.example.test","{field}":"{value}"}}"#);
        let text = reason::domain_refusal(Some(value), &domains(&["corp.test"])).unwrap();
        assert!(reason::is_domain_refusal(&text), "{text}");
        assert!(
            reason::domain_refusal_carries_credential(&text, document.as_bytes()),
            "{field} echoed whole and kept: {text}"
        );
    }
}

/// semantics.md: for "a claim that is not a lower-case host name" the refusal
/// is `the account's domain is not an admitted domain`, naming nothing.
#[test]
fn adv2_an_upper_case_hd_claim_takes_the_unnamed_form() {
    let text = reason::domain_refusal(Some("Other.Example"), &domains(&["corp.test"])).unwrap();
    assert_eq!(
        text,
        "the account's domain is not an admitted domain; admitted: corp.test"
    );
}

/// The admitted list is the instance's `allowed_domains`, which configuration
/// accepts only as host names of 63-byte labels, at least two of them, none
/// starting or ending in `-`, never repeated. The host must not accept an
/// "admitted" entry no configuration can hold: a 64-byte lower-case hex run,
/// a lone `-`, a dot run, a single label.
#[test]
fn adv2_the_host_refuses_an_admitted_entry_no_configuration_can_hold() {
    let hex = "0123456789abcdef".repeat(4);
    for entry in [
        hex.as_str(),
        "-",
        "...",
        "localhost",
        "-a-.test",
        "corp.test, corp.test",
    ] {
        let forged = format!("the account's domain is not an admitted domain; admitted: {entry}");
        assert!(!reason::is_domain_refusal(&forged), "{forged}");
    }
}
