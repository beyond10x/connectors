#[cfg(test)]
mod database_tests {
    use super::tests::{owner_with_groups, policy};
    use super::*;
    use protocol::datasource::{Completeness, DatasourceRead, RecordView};

    /// A database-endpoint fake shaped like the real Crossplane provider-sql resources: each
    /// item is full resource JSON — deletionPolicy, providerConfigRef, annotations with a
    /// credential-shaped canary — parsed through the wire types per call, so what the
    /// projection drops is proven against the real shape, not against a convenient struct.
    struct DatabaseReader {
        mysql: Vec<serde_json::Value>,
        postgresql: Vec<serde_json::Value>,
    }

    impl DatabaseReader {
        fn crossplane() -> Self {
            Self {
                mysql: vec![
                    crossplane_database(
                        "mysql.sql.crossplane.io",
                        "app-core",
                        true,
                        Some(json!({"name": "app-core-conn", "namespace": "b10x"})),
                        json!({}),
                    ),
                    crossplane_database(
                        "mysql.sql.crossplane.io",
                        "app-events",
                        false,
                        None,
                        json!({}),
                    ),
                ],
                postgresql: vec![crossplane_database(
                    "postgresql.sql.crossplane.io",
                    "ledger",
                    true,
                    Some(json!({"name": "ledger-conn"})),
                    json!({
                        "host": "postgresql.endpoints.svc.cluster.local",
                        "port": 5432,
                        "database": "ledger"
                    }),
                )],
            }
        }

        /// The in-cluster reader maps a 404 on the group's list endpoint — the CRD is not
        /// installed — to an empty inventory; this fake stands in for that outcome.
        fn without_crossplane() -> Self {
            Self {
                mysql: Vec::new(),
                postgresql: Vec::new(),
            }
        }

        fn engine_items(&self, engine: DatabaseEngine) -> &[serde_json::Value] {
            match engine {
                DatabaseEngine::Mysql => &self.mysql,
                DatabaseEngine::Postgresql => &self.postgresql,
            }
        }
    }

    /// One Crossplane provider-sql Database resource, exactly as the cluster serves it.
    fn crossplane_database(
        group: &str,
        name: &str,
        ready: bool,
        secret: Option<serde_json::Value>,
        at_provider: serde_json::Value,
    ) -> serde_json::Value {
        let ready_status = if ready { "True" } else { "False" };
        let mut spec = json!({
            "deletionPolicy": "Orphan",
            "forProvider": {},
            "providerConfigRef": {"name": "sql-default"}
        });
        if let Some(secret) = secret {
            spec["writeConnectionSecretToRef"] = secret;
        }
        json!({
            "apiVersion": format!("{group}/v1alpha1"),
            "kind": "Database",
            "metadata": {
                "name": name,
                "namespace": "b10x",
                "uid": format!("uid-{name}"),
                "resourceVersion": "17",
                "annotations": {
                    "crossplane.io/external-name": name,
                    "kubectl.kubernetes.io/last-applied-configuration":
                        "{\"spec\":{\"password\":\"hunter2-canary\"}}"
                }
            },
            "spec": spec,
            "status": {
                "atProvider": at_provider,
                "conditions": [
                    {"type": "Ready", "status": ready_status, "reason": "Available"},
                    {"type": "Synced", "status": "True", "reason": "ReconcileSuccess"}
                ]
            }
        })
    }

    #[async_trait]
    impl DeploymentReader for DatabaseReader {
        async fn read(
            &self,
            _namespace: &str,
            _name: &str,
        ) -> Result<DeploymentStatus, OperationError> {
            Err(unavailable("deployment status is not part of this test"))
        }

        async fn list_databases(
            &self,
            namespace: &str,
            engine: DatabaseEngine,
            limit: u16,
            cursor: Option<&str>,
        ) -> Result<DatabaseList, DatasourceError> {
            assert_eq!(namespace, "b10x");
            let items = self.engine_items(engine);
            let start: usize = cursor.map_or(0, |cursor| cursor.parse().expect("fake cursor"));
            let end = (start + usize::from(limit)).min(items.len());
            let page = items[start..end]
                .iter()
                .map(|resource| {
                    serde_json::from_value(resource.clone()).expect("fake resource parses")
                })
                .collect();
            Ok(DatabaseList {
                items: page,
                next_cursor: (end < items.len()).then(|| end.to_string()),
            })
        }

        async fn database_detail(
            &self,
            _namespace: &str,
            engine: DatabaseEngine,
            name: &str,
        ) -> Result<CrossplaneDatabase, DatasourceError> {
            self.engine_items(engine)
                .iter()
                .find(|resource| resource["metadata"]["name"] == name)
                .map(|resource| {
                    serde_json::from_value(resource.clone()).expect("fake resource parses")
                })
                .ok_or_else(|| {
                    datasource_not_found("Kubernetes database endpoint was not found")
                })
        }
    }

    fn database_backend(reader: DatabaseReader) -> KubernetesStatusBackend {
        KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            policy(),
            Vec::new(),
            Arc::new(reader),
        )
        .unwrap()
    }

    async fn databases_lease(
        backend: &KubernetesStatusBackend,
        context: &PrincipalContext,
    ) -> String {
        let DatasourceResult::Describe(description) = backend
            .handle_datasource(
                context,
                DatasourceRequest::Describe(DatasourceDescribeRequest {
                    datasource_ref: DATABASES_DATASOURCE.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("database datasource description expected");
        };
        description.description_ref
    }

    fn databases_read(
        binding_ref: String,
        description_ref: String,
        read: DatasourceRead,
    ) -> DatasourceRequest {
        DatasourceRequest::Read(ReadRequest {
            datasource_ref: DATABASES_DATASOURCE.to_owned(),
            binding_ref,
            description_ref,
            read,
        })
    }

    #[tokio::test]
    async fn database_endpoint_bindings_appear_per_admitted_namespace_only() {
        let backend = KubernetesStatusBackend::with_reader(
            "tenant-dev".to_owned(),
            vec![
                KubernetesNamespaceAccessConfig {
                    namespace: "b10x".to_owned(),
                    read_groups: vec!["dev".to_owned()],
                    restart_groups: Vec::new(),
                },
                KubernetesNamespaceAccessConfig {
                    namespace: "platform-ops".to_owned(),
                    read_groups: vec!["ops".to_owned()],
                    restart_groups: Vec::new(),
                },
            ],
            Vec::new(),
            Arc::new(DatabaseReader::crossplane()),
        )
        .unwrap();
        let bindings_request = || {
            DatasourceRequest::Bindings(BindingSearchRequest {
                datasource_ref: DATABASES_DATASOURCE.to_owned(),
                query: String::new(),
                limit: 16,
            })
        };
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let DatasourceResult::Bindings { bindings } = backend
            .handle_datasource(&context, bindings_request())
            .await
            .unwrap()
        else {
            panic!("database bindings expected");
        };
        assert_eq!(bindings.len(), 1, "{bindings:?}");
        assert_eq!(bindings[0].datasource_ref, DATABASES_DATASOURCE);
        assert_eq!(bindings[0].label, "b10x");
        assert_eq!(
            bindings[0].binding_ref,
            database_namespace_binding_ref("b10x")
        );
        // The database binding is its own seam, not the workload binding under a new name.
        assert_ne!(bindings[0].binding_ref, namespace_binding_ref("b10x"));

        let outsider = owner_with_groups("tenant-dev", ["unrelated"]);
        let DatasourceResult::Bindings { bindings } = backend
            .handle_datasource(&outsider, bindings_request())
            .await
            .unwrap()
        else {
            panic!("database bindings expected");
        };
        assert!(bindings.is_empty(), "{bindings:?}");
    }

    #[tokio::test]
    async fn database_endpoint_list_derives_descriptors_from_both_engines() {
        let backend = database_backend(DatabaseReader::crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = databases_lease(&backend, &context).await;
        let DatasourceResult::Read(page) = backend
            .handle_datasource(
                &context,
                databases_read(
                    database_namespace_binding(CONNECTION, "b10x").binding_ref,
                    lease,
                    DatasourceRead::List {
                        limit: 25,
                        cursor: None,
                    },
                ),
            )
            .await
            .unwrap()
        else {
            panic!("database endpoint page expected");
        };
        assert_eq!(page.datasource_ref, DATABASES_DATASOURCE);
        assert_eq!(page.completeness, Completeness::Complete);
        assert!(page.next_cursor.is_none());
        assert_eq!(page.records.len(), 3);
        assert!(page
            .records
            .iter()
            .all(|record| record.view == RecordView::Compact));
        assert_eq!(
            page.records[0].key,
            json!({"engine": "mysql", "name": "app-core"})
        );
        let values: Vec<&serde_json::Value> =
            page.records.iter().map(|record| &record.value).collect();
        assert_eq!(values[0]["engine"], "mysql");
        assert_eq!(values[0]["name"], "app-core");
        assert_eq!(values[0]["ready"], true);
        assert_eq!(values[0]["secret_ref"]["name"], "app-core-conn");
        assert_eq!(values[0]["secret_ref"]["namespace"], "b10x");
        // provider-sql resources carry no endpoint facts; the descriptor says so honestly.
        assert_eq!(values[0]["host"], serde_json::Value::Null);
        assert_eq!(values[0]["port"], serde_json::Value::Null);
        assert_eq!(values[0]["database"], serde_json::Value::Null);
        assert_eq!(values[1]["engine"], "mysql");
        assert_eq!(values[1]["name"], "app-events");
        assert_eq!(values[1]["ready"], false);
        assert_eq!(values[1]["secret_ref"], serde_json::Value::Null);
        assert_eq!(values[2]["engine"], "postgresql");
        assert_eq!(values[2]["name"], "ledger");
        assert_eq!(values[2]["ready"], true);
        assert_eq!(values[2]["host"], "postgresql.endpoints.svc.cluster.local");
        assert_eq!(values[2]["port"], 5432);
        assert_eq!(values[2]["database"], "ledger");
        assert_eq!(values[2]["secret_ref"]["name"], "ledger-conn");
        assert_eq!(values[2]["secret_ref"]["namespace"], serde_json::Value::Null);
        protocol::datasource::ResponseEnvelope::success(
            "request-databases",
            DatasourceResult::Read(page),
        )
        .validate()
        .expect("a database endpoint page must validate");
    }

    #[tokio::test]
    async fn database_endpoint_listing_pages_across_both_engines() {
        let backend = database_backend(DatabaseReader::crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = databases_lease(&backend, &context).await;
        let binding_ref = database_namespace_binding(CONNECTION, "b10x").binding_ref;
        let mut names = Vec::new();
        let mut cursor = None;
        let mut pages = 0;
        loop {
            let DatasourceResult::Read(page) = backend
                .handle_datasource(
                    &context,
                    databases_read(
                        binding_ref.clone(),
                        lease.clone(),
                        DatasourceRead::List { limit: 1, cursor },
                    ),
                )
                .await
                .unwrap()
            else {
                panic!("database endpoint page expected");
            };
            pages += 1;
            assert!(page.records.len() <= 1, "{:?}", page.records);
            names.extend(page.records.iter().map(|record| {
                format!(
                    "{}/{}",
                    record.value["engine"].as_str().unwrap(),
                    record.value["name"].as_str().unwrap()
                )
            }));
            match page.next_cursor {
                Some(next) => cursor = Some(next),
                None => break,
            }
            assert!(pages < 10, "paging must terminate");
        }
        assert_eq!(
            names,
            ["mysql/app-core", "mysql/app-events", "postgresql/ledger"]
        );
    }

    #[tokio::test]
    async fn database_endpoint_get_returns_one_descriptor_by_name() {
        let backend = database_backend(DatabaseReader::crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = databases_lease(&backend, &context).await;
        let binding_ref = database_namespace_binding(CONNECTION, "b10x").binding_ref;
        let DatasourceResult::Read(page) = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref.clone(),
                    lease.clone(),
                    DatasourceRead::Get {
                        key: json!({"engine": "postgresql", "name": "ledger"}),
                    },
                ),
            )
            .await
            .unwrap()
        else {
            panic!("database endpoint get expected");
        };
        assert_eq!(page.records.len(), 1);
        assert_eq!(page.records[0].view, RecordView::Detail);
        assert_eq!(page.records[0].value["engine"], "postgresql");
        assert_eq!(
            page.records[0].value["host"],
            "postgresql.endpoints.svc.cluster.local"
        );
        assert_eq!(page.records[0].value["secret_ref"]["name"], "ledger-conn");

        let missing = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref.clone(),
                    lease.clone(),
                    DatasourceRead::Get {
                        key: json!({"engine": "mysql", "name": "ledger"}),
                    },
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(missing.code, DatasourceErrorCode::NotFound);

        let invalid = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref.clone(),
                    lease,
                    DatasourceRead::Get {
                        key: json!({"engine": "oracle", "name": "ledger"}),
                    },
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(invalid.code, DatasourceErrorCode::InvalidInput);

        let stale = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref,
                    "description:kubernetes:datasource:stale".to_owned(),
                    DatasourceRead::Get {
                        key: json!({"engine": "postgresql", "name": "ledger"}),
                    },
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(stale.code, DatasourceErrorCode::StaleAuthority);
    }

    #[tokio::test]
    async fn a_database_read_on_a_non_admitted_namespace_is_not_granted() {
        let backend = database_backend(DatabaseReader::crossplane());
        let outsider = owner_with_groups("tenant-dev", ["unrelated"]);
        let refused = backend
            .handle_datasource(
                &outsider,
                databases_read(
                    database_namespace_binding(CONNECTION, "b10x").binding_ref,
                    databases_description_ref(&outsider),
                    DatasourceRead::List {
                        limit: 5,
                        cursor: None,
                    },
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, DatasourceErrorCode::NotGranted);
        assert!(refused.message.contains("b10x"), "{refused:?}");
        assert!(refused.message.contains(DATABASES_DATASOURCE), "{refused:?}");
        assert!(refused.retriable, "{refused:?}");
    }

    /// A cluster without the Crossplane provider serves no Database CRD, the group's list
    /// endpoint answers 404, and the reader maps that to an empty inventory: discovery finds
    /// nothing, and nothing is not an error.
    #[tokio::test]
    async fn a_cluster_without_crossplane_discovers_nothing() {
        let backend = database_backend(DatabaseReader::without_crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = databases_lease(&backend, &context).await;
        let DatasourceResult::Read(page) = backend
            .handle_datasource(
                &context,
                databases_read(
                    database_namespace_binding(CONNECTION, "b10x").binding_ref,
                    lease,
                    DatasourceRead::List {
                        limit: 25,
                        cursor: None,
                    },
                ),
            )
            .await
            .unwrap()
        else {
            panic!("an empty discovery page expected, not an error");
        };
        assert!(page.records.is_empty(), "{:?}", page.records);
        assert!(page.next_cursor.is_none());
        assert_eq!(page.completeness, Completeness::Complete);
    }

    /// Discovery publishes references, never secret bytes (design 15): the raw resources carry
    /// a credential-shaped canary and provider plumbing, and none of it may survive into any
    /// datasource output — while the connection-secret NAME must.
    #[tokio::test]
    async fn no_secret_value_ever_appears_in_database_endpoint_output() {
        let backend = database_backend(DatabaseReader::crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let lease = databases_lease(&backend, &context).await;
        let binding_ref = database_namespace_binding(CONNECTION, "b10x").binding_ref;
        let list = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref.clone(),
                    lease.clone(),
                    DatasourceRead::List {
                        limit: 25,
                        cursor: None,
                    },
                ),
            )
            .await
            .unwrap();
        let get = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref,
                    lease,
                    DatasourceRead::Get {
                        key: json!({"engine": "mysql", "name": "app-core"}),
                    },
                ),
            )
            .await
            .unwrap();
        let list = serde_json::to_string(&list).unwrap();
        let get = serde_json::to_string(&get).unwrap();
        assert!(list.contains("app-core-conn"), "{list}");
        for output in [&list, &get] {
            assert!(output.contains("secret_ref"), "{output}");
            for forbidden in [
                "hunter2",
                "password",
                "last-applied",
                "annotations",
                "external-name",
                "providerConfigRef",
                "deletionPolicy",
            ] {
                assert!(!output.contains(forbidden), "`{forbidden}` leaked: {output}");
            }
        }
    }

    #[tokio::test]
    async fn search_lists_the_databases_datasource_under_its_terms() {
        let backend = database_backend(DatabaseReader::crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let search = |query: &str| {
            DatasourceRequest::Search(DatasourceSearchRequest {
                query: query.to_owned(),
                limit: 16,
            })
        };
        for query in ["", "database", "sql", "crossplane", "endpoint"] {
            let DatasourceResult::Search { definitions } = backend
                .handle_datasource(&context, search(query))
                .await
                .unwrap()
            else {
                panic!("datasource search result expected");
            };
            assert!(
                definitions
                    .iter()
                    .any(|definition| definition.datasource_ref == DATABASES_DATASOURCE),
                "`{query}`: {definitions:?}"
            );
        }
        // The workload projection is still published beside it.
        let DatasourceResult::Search { definitions } = backend
            .handle_datasource(&context, search(""))
            .await
            .unwrap()
        else {
            panic!("datasource search result expected");
        };
        assert_eq!(
            definitions
                .iter()
                .filter(|definition| definition.datasource_ref == DATASOURCE)
                .count(),
            1,
            "{definitions:?}"
        );

        let outsider = owner_with_groups("tenant-dev", ["unrelated"]);
        let DatasourceResult::Search { definitions } = backend
            .handle_datasource(&outsider, search("database"))
            .await
            .unwrap()
        else {
            panic!("datasource search result expected");
        };
        assert!(definitions.is_empty(), "{definitions:?}");
    }

    #[tokio::test]
    async fn database_datasource_description_names_the_projection_for_read_principals_only() {
        let backend = database_backend(DatabaseReader::crossplane());
        let context = owner_with_groups("tenant-dev", ["dev"]);
        let DatasourceResult::Describe(description) = backend
            .handle_datasource(
                &context,
                DatasourceRequest::Describe(DatasourceDescribeRequest {
                    datasource_ref: DATABASES_DATASOURCE.to_owned(),
                }),
            )
            .await
            .unwrap()
        else {
            panic!("database datasource description expected");
        };
        assert_eq!(description.summary.datasource_ref, DATABASES_DATASOURCE);
        assert_eq!(description.key_schema["required"], json!(["engine", "name"]));
        assert_eq!(
            description.compact_schema["required"],
            json!(["engine", "name", "host", "port", "database", "secret_ref", "ready"])
        );
        assert_eq!(description.description_ref, databases_description_ref(&context));
        // Two datasources, two leases: staleness of one must not refuse the other.
        assert_ne!(description.description_ref, datasource_description_ref(&context));

        let outsider = owner_with_groups("tenant-dev", ["unrelated"]);
        let refused = backend
            .handle_datasource(
                &outsider,
                DatasourceRequest::Describe(DatasourceDescribeRequest {
                    datasource_ref: DATABASES_DATASOURCE.to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, DatasourceErrorCode::NotGranted);
        assert!(refused.message.contains(DATABASES_DATASOURCE), "{refused:?}");
    }
}
