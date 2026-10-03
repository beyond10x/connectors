//! Real pinned catalog child, token exchange and business HTTP under private/3.
use super::*;
use connectors_host::local::runtime::{PrivateProtocol, ReadBudget};

fn child(provider: &OAuthProvider) -> Child {
    let mut selection = provider.selection();
    selection.private_protocol = Some(PrivateProtocol::V3);
    Child::spawn(&selection).unwrap()
}
fn read(child: &mut Child, budget: ReadBudget) -> Result<Value, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let bytes = child.invoke_bounded(
        "project.get",
        &revision,
        "one",
        &entry(),
        br#"{"id":"org/project"}"#,
        budget,
    )?;
    assert!(!carries_oauth_material(&bytes));
    serde_json::from_slice(&bytes).map_err(|_| Failure::Protocol)
}

#[test]
fn bounded_read_executes_through_exact_selected_child() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = child(&provider);
    let budget = ReadBudget::start(2000, 1500, 65_536, 4_194_304).unwrap();
    assert_eq!(read(&mut child, budget).unwrap()["body"]["id"], 7);
    assert_eq!(provider.token_requests(), 1);
    assert_eq!(provider.requests().len(), 2);
    let incarnation = child.incarnation().to_owned();
    child.stop(&incarnation).unwrap();
    assert!(provider_children(&provider.config).is_empty());
}

#[test]
fn bounded_read_requires_explicit_selection_and_never_falls_back() {
    let provider = OAuthProvider::new(Source::Api);
    let mut legacy = spawn(&provider);
    assert_eq!(
        read(
            &mut legacy,
            ReadBudget::start(2000, 1500, 100, 1000).unwrap()
        )
        .unwrap_err(),
        Failure::Unsupported
    );
    let mut selected = child(&provider);
    assert_eq!(
        project(&mut selected, &entry()).unwrap_err(),
        Failure::Unsupported
    );
    assert!(provider.requests().is_empty());
}

#[test]
fn bounded_read_keeps_one_provider_cutoff_across_oauth_and_api() {
    let provider = OAuthProvider::new(Source::Api);
    {
        let mut state = provider.state.lock().unwrap();
        state.token_delay_ms = 1000;
        state.api_delay_ms = 1000;
    }
    let mut child = child(&provider);
    let error = read(
        &mut child,
        ReadBudget::start(6000, 1500, 65_536, 4_194_304).unwrap(),
    )
    .unwrap_err();
    assert_eq!(error, Failure::ProviderTimeout);
    assert_eq!(provider.token_requests(), 1);
    assert_eq!(
        provider.requests().len(),
        2,
        "both stages must execute to check cutoff inheritance"
    );
    assert!(
        provider_children(&provider.config).is_empty(),
        "timed-out child was not reaped"
    );
}

#[test]
fn bounded_read_preserves_execution_time_spent_before_native_dispatch() {
    let provider = OAuthProvider::new(Source::Api);
    provider.state.lock().unwrap().api_delay_ms = 1400;
    let mut child = child(&provider);
    let budget = ReadBudget::start(2000, 2000, 65_536, 4_194_304).unwrap();
    std::thread::sleep(Duration::from_millis(1100));
    let error = read(&mut child, budget).unwrap_err();
    assert!(matches!(
        error,
        Failure::Timeout | Failure::ProviderTimeout | Failure::Unavailable
    ));
    assert_eq!(provider.requests().len(), 2);
    assert!(provider_children(&provider.config).is_empty());
}

#[test]
fn bounded_read_limits_and_expiry_refuse_without_reset_or_truncation() {
    let provider = OAuthProvider::new(Source::Api);
    let mut child = child(&provider);
    assert_eq!(
        read(&mut child, ReadBudget::start(2000, 1500, 1, 1000).unwrap()).unwrap_err(),
        Failure::InvalidInput
    );
    let expired = ReadBudget::start(20, 10, 100, 1000).unwrap();
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(read(&mut child, expired).unwrap_err(), Failure::Timeout);
    assert!(provider.requests().is_empty());
    assert_eq!(
        read(&mut child, ReadBudget::start(2000, 1500, 100, 1).unwrap()).unwrap_err(),
        Failure::Capacity
    );
    assert_eq!(provider.requests().len(), 2);
}
