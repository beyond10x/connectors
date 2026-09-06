#[cfg(test)]
mod tests {
    use super::*;

    struct RateEgress {
        retry: Option<&'static str>,
        calls: std::sync::atomic::AtomicUsize,
        headers: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl EgressTransport for RateEgress {
        async fn execute(
            &self,
            _: &str,
            request: EgressHttpRequest,
        ) -> Result<service::EgressHttpResponse, service::EgressTransportError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            *self.headers.lock().unwrap() = request.response_headers;
            Ok(service::EgressHttpResponse {
                status: 429,
                headers: self
                    .retry
                    .map(|value| ("retry-after".to_owned(), value.to_owned()))
                    .into_iter()
                    .collect(),
                body: b"SENTINEL-PROVIDER-BODY".to_vec(),
            })
        }

        async fn connect_websocket(
            &self,
            _: &str,
            _: String,
            _: usize,
        ) -> Result<Box<dyn service::EgressWebSocket>, service::EgressTransportError> {
            unreachable!("HTTP operation")
        }
    }

    #[tokio::test]
    async fn rate_stage2_catalog_refusal_keeps_only_trusted_optional_delay() {
        for (retry, delay) in [
            (Some("30"), Some(30_u64)),
            (Some("0"), Some(0)),
            (Some("18446744073709551615"), Some(u64::MAX)),
            (None, None),
            (Some("30, 30"), None),
            (Some("-1"), None),
            (Some("1.5"), None),
            (Some("18446744073709551616"), None),
            (Some("Wed, 21 Oct 2015 07:28:00 GMT"), None),
        ] {
            let mut bound = binding(true);
            bound.provider = catalog::provider(catalog::ProviderKey::id("slack")).unwrap();
            bound.connection_ref = "connection:slack:test".to_owned();
            let mut inner = test_inner(vec![bound]);
            let egress = Arc::new(RateEgress {
                retry,
                calls: std::sync::atomic::AtomicUsize::new(0),
                headers: Mutex::new(Vec::new()),
            });
            inner.egress = egress.clone();
            let address =
                credential_address("local", "com.slack.api", &entry("slack", &[]), "bot_token")
                    .unwrap();
            inner
                .secrets
                .put(
                    &address,
                    &connector_secrets::Secret::new("SENTINEL-FIXTURE"),
                )
                .await
                .unwrap();
            let description = inner.describe("slack-conversations-history").unwrap();
            let advice = description
                .rate_advice
                .as_ref()
                .expect("history publishes advice");
            advice.validate().unwrap();
            assert!(advice.fixed.is_none());
            assert_eq!(
                advice
                    .alternatives
                    .iter()
                    .map(|alternative| alternative.suggested_interval_ms)
                    .collect::<Vec<_>>(),
                [Some(1200), Some(60000), None]
            );
            let error = inner
                .invoke(
                    "slack-conversations-history",
                    "connection:slack:test",
                    &description.description_ref,
                    serde_json::json!({"channel":"C012345"}),
                )
                .await
                .unwrap_err();
            let value = serde_json::to_value(error).unwrap();
            assert_eq!(
                egress.calls.load(std::sync::atomic::Ordering::SeqCst),
                1,
                "{value}"
            );
            assert_eq!(value["code"], "rate_limited");
            assert_eq!(value["retriable"], true);
            assert_eq!(
                value
                    .get("retry_after_seconds")
                    .and_then(serde_json::Value::as_u64),
                delay
            );
            assert_eq!(*egress.headers.lock().unwrap(), ["retry-after"]);
            assert!(!value.to_string().contains("SENTINEL"));
        }
    }

    fn gitlab() -> &'static catalog::Provider {
        catalog::provider(catalog::ProviderKey::id("gitlab")).expect("gitlab is catalogued")
    }

    /// An egress that refuses everything: `search` and `describe` never dispatch, so the tests
    /// covering them need a transport that exists and is never used.
    struct RefusingEgress;

    #[async_trait]
    impl EgressTransport for RefusingEgress {
        async fn execute(
            &self,
            _authority: &str,
            _request: EgressHttpRequest,
        ) -> Result<service::EgressHttpResponse, service::EgressTransportError> {
            Err(service::EgressTransportError::Refused)
        }

        async fn connect_websocket(
            &self,
            _authority: &str,
            _url: String,
            _maximum: usize,
        ) -> Result<Box<dyn service::EgressWebSocket>, service::EgressTransportError> {
            Err(service::EgressTransportError::Refused)
        }
    }

    fn named(name: &str) -> Binding {
        let mut binding = binding(false);
        binding.connection_ref = format!("connection:gitlab:{name}");
        binding.label = name.to_owned();
        binding
    }

    fn binding(allow_writes: bool) -> Binding {
        Binding {
            instance: None,
            provider: gitlab(),
            connection_ref: "connection:gitlab:test".to_owned(),
            label: "GitLab".to_owned(),
            grant_ref: "grant:gitlab:test".to_owned(),
            initiation: InitiationPolicy::platform_only(),
            config: DeclaredConfig::default(),
            allow_writes: false,
        }
        .tap(allow_writes)
    }

    impl Binding {
        fn tap(mut self, allow_writes: bool) -> Self {
            self.allow_writes = allow_writes;
            self
        }
    }

    #[test]
    fn the_default_ceiling_admits_reads_and_refuses_writes() {
        let read_only = binding(false);
        let reads = gitlab()
            .operations
            .iter()
            .filter(|operation| read_only.admits(operation))
            .count();
        let writes = gitlab()
            .operations
            .iter()
            .filter(|operation| !read_only.admits(operation))
            .count();
        assert!(reads > 0, "a read-only grant must still admit something");
        assert!(writes > 0, "gitlab declares write operations to refuse");
        // Raising the ceiling admits everything the catalogue declares, without naming any of it.
        let unrestricted = binding(true);
        assert!(gitlab()
            .operations
            .iter()
            .all(|operation| unrestricted.admits(operation)));
    }

    #[test]
    fn the_ceiling_reads_declared_facts_rather_than_an_operation_list() {
        let read_only = binding(false);
        for operation in gitlab().operations {
            let admitted = read_only.admits(operation);
            let declared_read = matches!(operation.direction, OperationDirection::Read)
                && !matches!(operation.risk, Risk::Destructive)
                && operation
                    .effects
                    .iter()
                    .all(|effect| matches!(effect, HostEffect::Read | HostEffect::Network));
            assert_eq!(
                admitted, declared_read,
                "`{}` was admitted on something other than its declaration",
                operation.id
            );
        }
    }

    #[test]
    fn a_network_effect_is_not_an_escalation_but_a_filesystem_one_would_be() {
        // The distinction the first version of `admits` got wrong. Every declared HTTP call
        // carries `[read, network]`, so treating any effect as disqualifying admitted nothing;
        // treating none as disqualifying would admit an operation that writes this machine's disk.
        let read_only = binding(false);
        let ordinary = gitlab()
            .operations
            .iter()
            .find(|operation| matches!(operation.direction, OperationDirection::Read))
            .expect("gitlab declares reads");
        assert!(ordinary
            .effects
            .iter()
            .any(|effect| matches!(effect, HostEffect::Network)));
        assert!(
            read_only.admits(ordinary),
            "a network effect must not disqualify a read"
        );
        assert!(
            !ordinary
                .effects
                .iter()
                .any(|effect| matches!(effect, HostEffect::Filesystem | HostEffect::Process)),
            "this fixture is only meaningful while gitlab reads stay network-only"
        );
    }

    #[test]
    fn effect_class_comes_from_the_declaration_not_the_method() {
        for operation in gitlab().operations {
            let class = effect_class(operation);
            match operation.direction {
                OperationDirection::Read if !matches!(operation.risk, Risk::Destructive) => {
                    assert_eq!(class, EffectClass::ReadOnly, "{}", operation.id);
                }
                _ => assert_ne!(class, EffectClass::ReadOnly, "{}", operation.id),
            }
        }
    }

    fn entry(provider: &str, endpoints: &[(&str, &str)]) -> CatalogIntegrationConfig {
        CatalogIntegrationConfig {
            provider: provider.to_owned(),
            instance: None,
            label: None,
            grant_ref: format!("grant:{provider}:test"),
            initiation: InitiationConfig::Platform,
            allow_writes: false,
            endpoints: endpoints
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            usernames: BTreeMap::new(),
            operator_approved: true,
            network: connectors_config::NetworkScopeConfig::Public,
            credential: None,
            credential_file: None,
        }
    }

    #[test]
    fn the_aperture_is_derived_from_the_same_declaration_the_request_is() {
        let origins = admitted_origins(&entry(
            "gitlab",
            &[("origin", "https://gitlab.example.test")],
        ))
        .expect("gitlab's services resolve");
        // Both declared services — the API surface and the OAuth surface — live at one origin, so
        // one rule admits the provider without admitting anything else.
        assert_eq!(origins, vec!["https://gitlab.example.test".to_owned()]);
    }

    #[test]
    fn an_unsupplied_url_variable_refuses_rather_than_reaching_a_literal_placeholder() {
        let error = admitted_origins(&entry("gitlab", &[])).expect_err("origin is not supplied");
        assert!(matches!(
            error,
            CatalogIntegrationError::MissingEndpointValue(_, _)
        ));
    }

    #[test]
    fn a_basic_credentials_user_half_reaches_the_resolver_from_the_entry() {
        // The whole bug in one assertion. `jira.api_token` is `scheme = "basic"`, so the assembler
        // asks the ConfigPort for its user half and refuses the mechanism without one. Both
        // constructors used to build a config that could only answer endpoints, so a token that
        // was demonstrably stored produced `MissingCredentialConfig`.
        use connector_resolve::{ConfigField, ConfigPort as _};

        let mut with_user = entry("jira", &[("site", "example")]);
        with_user
            .usernames
            .insert("jira.api_token".to_owned(), "ops@example.test".to_owned());
        let config = declared_config(&with_user);
        assert_eq!(
            config
                .resolve(ConfigField::Username("jira.api_token"))
                .map(|value| value.value().to_owned()),
            Some("ops@example.test".to_owned())
        );

        // And the catalogue is what says this credential has a user half at all, so the test does
        // not depend on anyone remembering that Atlassian uses Basic.
        let jira = catalog::provider(catalog::ProviderKey::id("jira")).expect("jira");
        let credential = jira
            .credential("jira.api_token")
            .expect("the declared token");
        assert!(matches!(
            credential.acquire,
            catalog::Acquisition::BasicJoin { .. }
        ));
        let field = jira
            .config
            .iter()
            .find(|field| field.binds == "username.jira.api_token")
            .expect("the catalogue declares which field is the user half");
        assert!(!field.secret, "a user half is configuration, not a secret");
    }

    #[test]
    fn an_entry_stating_no_user_half_answers_none_rather_than_an_empty_string() {
        // `ConfigPort::resolve` returning `Some("")` would make the assembler compose
        // `base64(":token")` and send it, which fails at the vendor with a message about the
        // credential rather than about the configuration.
        use connector_resolve::{ConfigField, ConfigPort as _};
        let config = declared_config(&entry("jira", &[("site", "example")]));
        assert!(config
            .resolve(ConfigField::Username("jira.api_token"))
            .is_none());
    }

    #[test]
    fn every_declared_user_half_field_names_a_credential_that_actually_wants_one() {
        // A catalogue-wide fence rather than a Jira one: a `username.<credential>` binding whose
        // credential is not a Basic join would be a configuration value nothing ever reads.
        for provider in catalog::providers() {
            for field in provider.config {
                let Some(name) = field.binds.strip_prefix("username.") else {
                    continue;
                };
                let credential = provider.credential(name).unwrap_or_else(|| {
                    panic!(
                        "`{}`'s field `{}` binds an undeclared credential",
                        provider.id, field.name
                    )
                });
                assert!(
                    matches!(credential.acquire, catalog::Acquisition::BasicJoin { .. }),
                    "`{}`'s `{}` is not a Basic join, so it has no user half to configure",
                    provider.id,
                    name
                );
                assert!(
                    !field.secret,
                    "`{}`'s `{}` is not a secret",
                    provider.id, field.name
                );
            }
        }
    }

    #[test]
    fn a_provider_with_a_fixed_base_url_needs_no_configuration() {
        // Most of the catalogue is like this: a SaaS host with nothing to ask the operator.
        let origins = admitted_origins(&entry("sentry", &[])).expect("sentry has fixed hosts");
        assert!(!origins.is_empty());
        assert!(origins.iter().all(|origin| origin.starts_with("https://")));
    }

    fn test_inner(bindings: Vec<Binding>) -> Inner {
        Inner {
            owner: PrincipalContext::local(&protocol::operation::OwnerContext {
                tenant_id: "local".to_owned(),
                agent_id: "a".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot:test".to_owned(),
                // A real digest shape: the principal refuses anything that is not 64 hex digits.
                authority_snapshot_sha256: "0".repeat(64),
            })
            .expect("a local principal"),
            bindings,
            secrets: std::sync::Arc::new(connector_secrets::MemoryStore::new()),
            egress: std::sync::Arc::new(RefusingEgress),
            leases: Mutex::new(BTreeMap::new()),
        }
    }

    #[test]
    fn a_read_only_first_connection_does_not_hide_an_admitted_write() {
        let write = gitlab()
            .operations
            .iter()
            .find(|operation| !binding(false).admits(operation))
            .unwrap();
        let inner = test_inner(vec![named("reader"), named("writer").tap(true)]);
        let description = inner
            .describe(write.id)
            .expect("the second connection admits it");
        assert_eq!(description.connections.len(), 1);
        assert_eq!(description.connections[0].label, "writer");
        assert_eq!(description.approval, ApprovalPosture::Required);
        assert_eq!(
            test_inner(vec![named("reader")])
                .describe(write.id)
                .unwrap_err()
                .code,
            OperationErrorCode::NotGranted
        );
    }

    #[test]
    fn search_and_describe_require_approval_for_every_admitted_write() {
        let inner = test_inner(vec![binding(true)]);
        let rows = inner.search("", u16::MAX);
        assert!(!rows.is_empty());
        for row in rows {
            let expected = match row.effect {
                EffectClass::ReadOnly => ApprovalPosture::NotRequired,
                EffectClass::Mutating | EffectClass::Destructive => ApprovalPosture::Required,
            };
            assert_eq!(row.approval, expected, "{}", row.operation_ref);
            assert_eq!(
                inner.describe(&row.operation_ref).unwrap().approval,
                expected
            );
        }
    }

    #[test]
    fn the_limit_drops_operations_rather_than_the_identities_that_serve_one() {
        // The truncation bug this shape fixes: with three Slack identities, `--limit 1` used to
        // report one operation reachable through one identity. An operation is one row; a caller
        // choosing between identities needs to see all of them or it cannot choose.
        let inner = test_inner(vec![named("first"), named("second"), named("third")]);
        let rows = inner.search("gitlab-user-get", 1);
        assert_eq!(rows.len(), 1, "one operation");
        assert_eq!(
            rows[0].connections.len(),
            3,
            "and every Connection that can serve it, not the first one the limit happened to reach"
        );
    }

    #[test]
    fn two_named_instances_of_one_provider_get_different_addresses() {
        // Two bot identities with the same provider, tenant and credential name must not collide.
        let mut first = entry("slack", &[]);
        first.instance = Some("support-bot".to_owned());
        let mut second = entry("slack", &[]);
        second.instance = Some("assistant-bot".to_owned());
        let a = credential_address("local", "com.slack.api", &first, "bot_token").unwrap();
        let b = credential_address("local", "com.slack.api", &second, "bot_token").unwrap();
        assert!(a.instance().is_some() && b.instance().is_some());
        assert_ne!(
            a.instance().map(InstanceId::as_str),
            b.instance().map(InstanceId::as_str),
            "same provider, same tenant, same credential name — only the instance separates them"
        );
    }

    #[test]
    fn an_unnamed_entry_keeps_the_address_it_had_before_instances_existed() {
        // Adding instance support must not orphan a credential someone connected yesterday. The
        // elided form is what a single-connection deployment already stored.
        let unnamed = entry("gitlab", &[]);
        let address = credential_address("local", "com.gitlab.api", &unnamed, "token").unwrap();
        assert!(
            address.instance().is_none(),
            "no instance segment for an unnamed entry"
        );
        assert_eq!(address.tenant(), "local");
        assert_eq!(address.authority(), "com.gitlab.api");
        assert_eq!(address.credential(), "token");
        assert!(address.is_default_service());
    }

    #[test]
    fn the_instance_derivation_puts_the_provider_in_the_namespace() {
        // Without the provider inside the digest, the same instance name on two providers would
        // address one credential. Named here because the Slack-only derivation this generalises
        // had exactly that shape.
        let slack = instance_for("slack", "shared-name").unwrap();
        let datadog = instance_for("datadog", "shared-name").unwrap();
        assert_ne!(slack.as_str(), datadog.as_str());
        assert_eq!(
            slack.as_str(),
            instance_for("slack", "shared-name").unwrap().as_str()
        );
        assert_eq!(slack.as_str().len(), 36);
    }

    #[test]
    fn every_catalogued_provider_can_address_a_credential() {
        // The generic path can only reach a provider whose credential has an address. Measuring it
        // here means a catalogue change that breaks the property fails a test rather than one
        // person's connect attempt.
        let mut addressable = 0;
        for provider in catalog::providers() {
            if provider.authority.is_some() && !provider.auth.is_empty() {
                assert!(credential_leaf(provider, None).is_ok(), "{}", provider.id);
                addressable += 1;
            }
        }
        assert!(
            addressable >= 56,
            "only {addressable} providers are addressable"
        );
    }

    #[test]
    fn a_request_template_exists_for_every_operation_this_backend_would_offer() {
        // The join this crate is: catalogue facts on one side, request template on the other. If a
        // provider's document is missing an operation the table declares, invocation would refuse
        // at runtime — so it is asserted for a real provider here.
        let document = connector_resolve::document::provider("gitlab").expect("gitlab document");
        for operation in gitlab().operations {
            assert!(
                document.operation(operation.id).is_some(),
                "`{}` is catalogued with no request template",
                operation.id
            );
        }
    }
    #[tokio::test]
    async fn rate_adversary_catalog_checks_binding_and_lease_before_rate_disclosure() {
        let mut bound = binding(true);
        bound.provider = catalog::provider(catalog::ProviderKey::id("slack")).unwrap();
        bound.connection_ref = "connection:slack:test".into();
        let mut inner = test_inner(vec![bound]);
        let egress = Arc::new(RateEgress {
            retry: Some("0"),
            calls: std::sync::atomic::AtomicUsize::new(0),
            headers: Mutex::new(Vec::new()),
        });
        inner.egress = egress.clone();
        let address =
            credential_address("local", "com.slack.api", &entry("slack", &[]), "bot_token")
                .unwrap();
        inner
            .secrets
            .put(
                &address,
                &connector_secrets::Secret::new("SENTINEL-FIXTURE"),
            )
            .await
            .unwrap();
        let description = inner.describe("slack-conversations-history").unwrap();
        for (connection, lease, input) in [
            (
                "connection:absent",
                description.description_ref.as_str(),
                serde_json::json!({"channel":"C012345"}),
            ),
            (
                "connection:slack:test",
                "description:stale",
                serde_json::json!({"channel":"C012345"}),
            ),
            (
                "connection:slack:test",
                description.description_ref.as_str(),
                serde_json::json!({"channel":[]}),
            ),
        ] {
            let error = inner
                .invoke("slack-conversations-history", connection, lease, input)
                .await
                .unwrap_err();
            assert_ne!(error.code, OperationErrorCode::RateLimited);
            assert!(error.retry_after_seconds.is_none());
            assert_eq!(egress.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        }
        let error = inner
            .invoke(
                "slack-conversations-history",
                "connection:slack:test",
                &description.description_ref,
                serde_json::json!({"channel":"C012345"}),
            )
            .await
            .unwrap_err();
        assert_eq!(error.code, OperationErrorCode::RateLimited);
        assert_eq!(error.retry_after_seconds, Some(0));
        assert_eq!(egress.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(*egress.headers.lock().unwrap(), ["retry-after"]);
        assert!(!serde_json::to_string(&error).unwrap().contains("SENTINEL"));
    }

}
