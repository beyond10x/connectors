//! The declared configuration a connection supplies, behind the resolver's port.
//!
//! A provider's `base_url` carries `{variable}` placeholders — `https://{subdomain}.zendesk.com`,
//! and for a self-managed GitLab the whole origin. The catalogue declares *which* variables exist
//! and what they mean; the operator supplies the values. This is the adapter between the two.
//!
//! # Operator approval is carried, not assumed
//!
//! [`ConfigField::approval`](catalog::ConfigField) marks fields whose value must be activated by
//! deployment policy before it can influence a request — a self-managed GitLab origin is the
//! shipped case, because pointing a credential at a different host than the one it was issued for
//! is how a token leaves the network it belongs to. In the personal posture the operator *is* the
//! deployment, so approval is expressible — but it is expressed, not skipped: a value is
//! `operator_approved` only when the configuration says so.

use std::collections::BTreeMap;

use connector_resolve::{ConfigField, ConfigPort, ConfigValue};

/// One connection's configuration values, keyed as the resolver asks for them.
#[derive(Debug, Clone, Default)]
pub struct DeclaredConfig {
    endpoints: BTreeMap<String, String>,
    usernames: BTreeMap<String, String>,
    approved: bool,
}

impl DeclaredConfig {
    /// Values the operator supplied, and whether they carry operator approval.
    #[must_use]
    pub fn new(endpoints: BTreeMap<String, String>, approved: bool) -> Self {
        Self {
            endpoints,
            usernames: BTreeMap::new(),
            approved,
        }
    }

    /// The non-secret user half of a `basic` credential, named by the credential it joins.
    #[must_use]
    pub fn with_username(mut self, credential: impl Into<String>, value: impl Into<String>) -> Self {
        self.usernames.insert(credential.into(), value.into());
        self
    }

    #[must_use]
    pub fn endpoint(&self, name: &str) -> Option<&str> {
        self.endpoints.get(name).map(String::as_str)
    }

    fn value(&self, raw: &str) -> ConfigValue {
        if self.approved {
            ConfigValue::operator_approved(raw)
        } else {
            ConfigValue::proposed(raw)
        }
    }
}

impl ConfigPort for DeclaredConfig {
    fn resolve(&self, field: ConfigField<'_>) -> Option<ConfigValue> {
        match field {
            ConfigField::Endpoint(name) => self.endpoints.get(name).map(|raw| self.value(raw)),
            ConfigField::Username(credential) => {
                self.usernames.get(credential).map(|raw| self.value(raw))
            }
            // Channel handshake values belong to the event surface, which this operations-only
            // slice does not serve. Answering `None` is the correct refusal rather than a guess.
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_endpoint_value_reaches_the_resolver_under_its_declared_name() {
        let config = DeclaredConfig::new(
            BTreeMap::from([("origin".to_owned(), "https://gitlab.example".to_owned())]),
            true,
        );
        let value = config
            .resolve(ConfigField::Endpoint("origin"))
            .expect("the declared origin resolves");
        assert!(value.is_operator_approved());
    }

    #[test]
    fn a_value_without_operator_approval_says_so_rather_than_claiming_it() {
        // The distinction is load-bearing: a field the catalogue marks `approval = operator` is
        // refused when the value is only proposed, and silently promoting it here would defeat the
        // one check that stops a credential being pointed at another host.
        let config = DeclaredConfig::new(
            BTreeMap::from([("origin".to_owned(), "https://gitlab.example".to_owned())]),
            false,
        );
        let value = config
            .resolve(ConfigField::Endpoint("origin"))
            .expect("the value is present");
        assert!(!value.is_operator_approved());
    }

    #[test]
    fn an_unsupplied_variable_is_absent_rather_than_empty() {
        let config = DeclaredConfig::default();
        assert!(config.resolve(ConfigField::Endpoint("subdomain")).is_none());
    }
}
