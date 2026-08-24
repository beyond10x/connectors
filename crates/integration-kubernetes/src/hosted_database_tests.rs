#[cfg(test)]
mod database_tests {
    use super::tests::{owner_with_groups, policy};
    use super::*;
    use protocol::datasource::{Completeness, DatasourceRead, RecordView};

    /// A database-endpoint fake shaped like the real Crossplane provider-sql resources: the
    /// Database CRDs are CLUSTER-scoped, so an item carries no `metadata.namespace`, its spec is
    /// only `{deletionPolicy, providerConfigRef}` — no endpoint facts, no
    /// `writeConnectionSecretToRef` — and the connection-secret reference lives behind the
    /// referenced cluster-scoped ProviderConfig. Everything is full resource JSON, parsed
    /// through the wire types per call, so what the projection drops is proven against the real
    /// shape, not against a convenient struct.
    struct DatabaseReader {
        mysql: Vec<serde_json::Value>,
        postgresql: Vec<serde_json::Value>,
        mysql_provider_configs: Vec<serde_json::Value>,
        postgresql_provider_configs: Vec<serde_json::Value>,
    }

    impl DatabaseReader {
        fn crossplane() -> Self {
            Self {
                mysql: vec![
                    crossplane_database("mysql.sql.crossplane.io", "app-core", true, "sql-default"),
                    crossplane_database(
                        "mysql.sql.crossplane.io",
                        "app-events",
                        false,
                        "sql-default",
                    ),
                    // Its ProviderConfig's connection secret lives in another namespace: the
                    // `b10x` binding must not see it.
                    crossplane_database("mysql.sql.crossplane.io", "foreign-db", true, "sql-other"),
                    // A dangling providerConfigRef: no secret reference, no namespace
                    // association, no descriptor — excluded, not an error.
                    crossplane_database("mysql.sql.crossplane.io", "orphan-db", true, "gone"),
                ],
                postgresql: vec![crossplane_database(
                    "postgresql.sql.crossplane.io",
                    "ledger",
                    true,
                    "sql-default",
                )],
                mysql_provider_configs: vec![
                    provider_config(
                        "mysql.sql.crossplane.io",
                        "sql-default",
                        "MySQLConnectionSecret",
                        "sql-default-conn",
                        "b10x",
                    ),
                    provider_config(
                        "mysql.sql.crossplane.io",
                        "sql-other",
                        "MySQLConnectionSecret",
                        "other-conn",
                        "elsewhere",
                    ),
                ],
                postgresql_provider_configs: vec![provider_config(
                    "postgresql.sql.crossplane.io",
                    "sql-default",
                    "PostgreSQLConnectionSecret",
                    "pg-default-conn",
                    "b10x",
                )],
            }
        }

        /// A cluster without the Crossplane provider serves neither the group nor its
        /// collections; the in-cluster reader maps that to an empty inventory and this fake
        /// stands in for that outcome.
        fn without_crossplane() -> Self {
            Self {
                mysql: Vec::new(),
                postgresql: Vec::new(),
                mysql_provider_configs: Vec::new(),
                postgresql_provider_configs: Vec::new(),
            }
        }

        fn engine_items(&self, engine: DatabaseEngine) -> &[serde_json::Value] {
            match engine {
                DatabaseEngine::Mysql => &self.mysql,
                DatabaseEngine::Postgresql => &self.postgresql,
            }
        }

        fn engine_provider_configs(&self, engine: DatabaseEngine) -> &[serde_json::Value] {
            match engine {
                DatabaseEngine::Mysql => &self.mysql_provider_configs,
                DatabaseEngine::Postgresql => &self.postgresql_provider_configs,
            }
        }

        fn page(&self, engine: DatabaseEngine, limit: u16, cursor: Option<&str>) -> DatabaseList {
            let items = self.engine_items(engine);
            let start: usize = cursor.map_or(0, |cursor| cursor.parse().expect("fake cursor"));
            let end = (start + usize::from(limit)).min(items.len());
            let page = items[start..end]
                .iter()
                .map(|resource| {
                    serde_json::from_value(resource.clone()).expect("fake resource parses")
                })
                .collect();
            DatabaseList {
                items: page,
                next_cursor: (end < items.len()).then(|| end.to_string()),
            }
        }

        fn detail(
            &self,
            engine: DatabaseEngine,
            name: &str,
        ) -> Result<CrossplaneDatabase, DatasourceError> {
            self.engine_items(engine)
                .iter()
                .find(|resource| resource["metadata"]["name"] == name)
                .map(|resource| {
                    serde_json::from_value(resource.clone()).expect("fake resource parses")
                })
                .ok_or_else(|| datasource_not_found("Kubernetes database endpoint was not found"))
        }
    }

    /// One Crossplane provider-sql Database resource, exactly as the cluster serves it:
    /// cluster-scoped (no `metadata.namespace`), and its spec carries no endpoint facts —
    /// only the deletion policy and the ProviderConfig reference.
    fn crossplane_database(
        group: &str,
        name: &str,
        ready: bool,
        provider_config: &str,
    ) -> serde_json::Value {
        let ready_status = if ready { "True" } else { "False" };
        json!({
            "apiVersion": format!("{group}/v1alpha1"),
            "kind": "Database",
            "metadata": {
                "name": name,
                "uid": format!("uid-{name}"),
                "resourceVersion": "17",
                "annotations": {
                    "crossplane.io/external-name": name,
                    "kubectl.kubernetes.io/last-applied-configuration":
                        "{\"spec\":{\"password\":\"hunter2-canary\"}}"
                }
            },
            "spec": {
                "deletionPolicy": "Delete",
                "providerConfigRef": {"name": provider_config}
            },
            "status": {
                "conditions": [
                    {"type": "Ready", "status": ready_status, "reason": "Available"},
                    {"type": "Synced", "status": "True", "reason": "ReconcileSuccess"}
                ]
            }
        })
    }

    /// One Crossplane provider-sql ProviderConfig, exactly as the cluster serves it:
    /// cluster-scoped, its credentials naming the server connection Secret by reference —
    /// name and namespace, never bytes.
    fn provider_config(
        group: &str,
        name: &str,
        source: &str,
        secret_name: &str,
        secret_namespace: &str,
    ) -> serde_json::Value {
        json!({
            "apiVersion": format!("{group}/v1alpha1"),
            "kind": "ProviderConfig",
            "metadata": {
                "name": name,
                "uid": format!("uid-{name}"),
                "resourceVersion": "5"
            },
            "spec": {
                "credentials": {
                    "source": source,
                    "connectionSecretRef": {
                        "name": secret_name,
                        "namespace": secret_namespace
                    }
                }
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
            engine: DatabaseEngine,
            limit: u16,
            cursor: Option<&str>,
        ) -> Result<DatabaseList, DatasourceError> {
            Ok(self.page(engine, limit, cursor))
        }

        async fn database_detail(
            &self,
            engine: DatabaseEngine,
            name: &str,
        ) -> Result<CrossplaneDatabase, DatasourceError> {
            self.detail(engine, name)
        }

        async fn provider_configs(
            &self,
            engine: DatabaseEngine,
        ) -> Result<Vec<CrossplaneProviderConfig>, DatasourceError> {
            Ok(self
                .engine_provider_configs(engine)
                .iter()
                .map(|resource| {
                    serde_json::from_value(resource.clone()).expect("fake provider config parses")
                })
                .collect())
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
        // `foreign-db` (its connection secret lives in `elsewhere`) and `orphan-db` (its
        // providerConfigRef names no ProviderConfig) do not associate with the `b10x`
        // binding and must not appear.
        assert_eq!(page.records.len(), 3, "{:?}", page.records);
        assert!(page
            .records
            .iter()
            .all(|record| record.view == RecordView::Compact));
        assert_eq!(
            page.records[0].key,
            json!({"engine": "mysql", "name": "app-core"})
        );
        // Whole-value equality: the descriptor is exactly {engine, name, provider_config,
        // secret_ref{name, namespace}, ready} — no host, port or database-name facts the
        // resources never declared, and nothing credential-shaped.
        assert_eq!(
            page.records[0].value,
            json!({
                "engine": "mysql",
                "name": "app-core",
                "provider_config": "sql-default",
                "secret_ref": {"name": "sql-default-conn", "namespace": "b10x"},
                "ready": true
            })
        );
        assert_eq!(
            page.records[1].value,
            json!({
                "engine": "mysql",
                "name": "app-events",
                "provider_config": "sql-default",
                "secret_ref": {"name": "sql-default-conn", "namespace": "b10x"},
                "ready": false
            })
        );
        assert_eq!(
            page.records[2].value,
            json!({
                "engine": "postgresql",
                "name": "ledger",
                "provider_config": "sql-default",
                "secret_ref": {"name": "pg-default-conn", "namespace": "b10x"},
                "ready": true
            })
        );
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
        assert_eq!(
            page.records[0].value,
            json!({
                "engine": "postgresql",
                "name": "ledger",
                "provider_config": "sql-default",
                "secret_ref": {"name": "pg-default-conn", "namespace": "b10x"},
                "ready": true
            })
        );

        // A database whose connection secret lives in another namespace exists on the cluster,
        // but not for this binding: not found, not leaked.
        let foreign = backend
            .handle_datasource(
                &context,
                databases_read(
                    binding_ref.clone(),
                    lease.clone(),
                    DatasourceRead::Get {
                        key: json!({"engine": "mysql", "name": "foreign-db"}),
                    },
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(foreign.code, DatasourceErrorCode::NotFound);

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

    /// A cluster without the Crossplane provider serves no Database CRD, the group's own
    /// discovery document is absent, and the reader maps that to an empty inventory: discovery
    /// finds nothing, and nothing is not an error.
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

    /// A wrong-scope regression must not hide behind the 404-means-no-Crossplane mapping
    /// (S-062): when the group's own discovery document IS served and lists the collection, a
    /// 404 on the collection's list is a wrong read path and surfaces as an error — while an
    /// unserved group, or a served group without the collection, stays an empty inventory.
    #[test]
    fn a_404_from_a_served_group_is_an_error_not_an_empty_inventory() {
        // The discovery document as `/apis/mysql.sql.crossplane.io/v1alpha1` really answers it.
        let served: KubernetesApiResourceList = serde_json::from_value(json!({
            "kind": "APIResourceList",
            "apiVersion": "v1",
            "groupVersion": "mysql.sql.crossplane.io/v1alpha1",
            "resources": [
                {
                    "name": "databases",
                    "singularName": "database",
                    "namespaced": false,
                    "kind": "Database",
                    "verbs": ["get", "list", "watch"]
                },
                {
                    "name": "providerconfigs",
                    "singularName": "providerconfig",
                    "namespaced": false,
                    "kind": "ProviderConfig",
                    "verbs": ["get", "list", "watch"]
                }
            ]
        }))
        .expect("the real discovery document parses");
        let error =
            absent_collection_is_empty("mysql.sql.crossplane.io", "databases", Some(&served))
                .unwrap_err();
        assert_eq!(error.code, DatasourceErrorCode::Unavailable);
        assert!(error.message.contains("mysql.sql.crossplane.io"), "{error:?}");
        assert!(error.message.contains("databases"), "{error:?}");

        // The group itself is not served: no Crossplane, and nothing is not an error.
        assert!(absent_collection_is_empty("mysql.sql.crossplane.io", "databases", None).is_ok());

        // The group is served but does not list the collection: still an empty inventory.
        let other: KubernetesApiResourceList =
            serde_json::from_value(json!({"resources": [{"name": "widgets"}]})).unwrap();
        assert!(
            absent_collection_is_empty("mysql.sql.crossplane.io", "databases", Some(&other))
                .is_ok()
        );
    }

    /// Discovery publishes references, never secret bytes (design 15): the raw resources carry
    /// a credential-shaped canary and provider plumbing, and none of it may survive into any
    /// datasource output — while the connection-secret NAME must. The other bindings' inventory
    /// (`foreign-db`, `orphan-db`, `other-conn`) must not surface either.
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
        assert!(list.contains("sql-default-conn"), "{list}");
        for output in [&list, &get] {
            assert!(output.contains("secret_ref"), "{output}");
            assert!(output.contains("provider_config"), "{output}");
            for forbidden in [
                "hunter2",
                "password",
                "last-applied",
                "annotations",
                "external-name",
                "providerConfigRef",
                "deletionPolicy",
                "credentials",
                "connectionSecretRef",
                "MySQLConnectionSecret",
                "foreign-db",
                "orphan-db",
                "other-conn",
                "elsewhere",
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
            json!(["engine", "name", "provider_config", "secret_ref", "ready"])
        );
        // The resources are cluster-scoped and declare no namespace-scoped endpoint facts; the
        // description must not promise what they do not carry.
        assert!(
            !description.description.contains("namespace-scoped"),
            "{}",
            description.description
        );
        assert!(
            description.description.contains("cluster-scoped"),
            "{}",
            description.description
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
