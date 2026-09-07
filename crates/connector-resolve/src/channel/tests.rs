use super::*;
use connector_secrets::{CredentialRef, MemoryStore, Secret};

struct Settings;
impl ConfigPort for Settings {
    fn resolve(&self, field: ConfigField<'_>) -> Option<ConfigValue> {
        let value = match field {
            ConfigField::Endpoint("host") => "ari.example.test",
            ConfigField::Username("asterisk.password") => "fixture-user",
            ConfigField::ChannelQuery {
                channel: "ari-events",
                parameter: "app",
            } => "fixture-app",
            _ => return None,
        };
        Some(ConfigValue::operator_approved(value))
    }
}

#[tokio::test]
async fn endpoint_channel_preserves_declared_auth_queries_and_event_interpretation() {
    let provider = catalog::provider(catalog::ProviderKey::id("asterisk")).unwrap();
    let channel = provider.channel("ari-events").unwrap();
    let credential = provider
        .auth
        .iter()
        .find(|auth| auth.name == "asterisk.password")
        .unwrap();
    let reference = CredentialRef::new(
        "fixture",
        provider.authority.unwrap(),
        "default",
        credential.leaf,
    )
    .unwrap();
    let secrets = MemoryStore::new();
    secrets
        .put(&reference, &Secret::new("fixture-password"))
        .await
        .unwrap();
    let original = channel_plan(provider, channel, "fixture", None, &secrets, &Settings)
        .await
        .unwrap();
    let routed = channel_plan_for_endpoint(
        provider,
        channel,
        "fixture",
        None,
        &secrets,
        &Settings,
        "http://asterisk.apps.svc:8088/ari",
    )
    .await
    .unwrap();
    assert_eq!(
        routed.url.expose_secret(),
        "ws://asterisk.apps.svc:8088/ari/events?app=fixture-app&subscribeAll=false"
    );
    assert_eq!(original.headers, routed.headers);
    assert_eq!(original.wire_events, routed.wire_events);
    assert_eq!(original.discriminator, routed.discriminator);
    assert_eq!(original.declared_base_url, routed.declared_base_url);
    assert!(routed
        .headers
        .values()
        .any(|value| value.expose_secret().starts_with("Basic ")));
    assert!(!format!("{routed:?}").contains("fixture-password"));
}

#[test]
fn endpoint_base_rejects_credentials_and_ambiguous_authorities() {
    for base in [
        "file:///tmp/socket",
        "http://user:password@example.test",
        "http://host/path?token=value",
        "https://host/#fragment",
        "http://host\\@other",
        "http://host/{placeholder}",
        "http://host/\npath",
    ] {
        assert!(validated_endpoint_base(base, "fixture").is_err(), "{base}");
    }
    assert!(validated_endpoint_base("http://service.apps.svc:8088/ari", "fixture").is_ok());
}
