use async_trait::async_trait;
use bytes::BytesMut;
use connectors_contracts::{Column, QueryResult};
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{Adapter, Credential, decode, encode, instance_descriptor, provenance};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio_postgres::{
    Client, NoTls,
    types::{Format, IsNull, ToSql, Type},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    #[serde(default)]
    pub allow_plaintext: bool,
    pub ca_file: Option<PathBuf>,
}
pub struct Sql {
    config: Config,
    password: Arc<dyn Credential>,
    descriptor: Descriptor,
}

impl Sql {
    pub fn new(
        instance: &str,
        config: Config,
        effective_configuration: Value,
        password: Arc<dyn Credential>,
    ) -> Result<Self> {
        if config.host.is_empty()
            || config.host.starts_with('/')
            || config.port == 0
            || config.database.is_empty()
            || config.user.is_empty()
        {
            return Err(Error::invalid("invalid database binding"));
        }
        let descriptor = instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            &effective_configuration,
        )?;
        connectors_sdk::verify_handlers(&descriptor, &["schema.list", "query.read"])?;
        Ok(Self {
            descriptor,
            config,
            password,
        })
    }
    async fn connect(&self) -> Result<(Client, ConnectionTask)> {
        let mut config = tokio_postgres::Config::new();
        let password = self.password.resolve().await?;
        config
            .host(&self.config.host)
            .port(self.config.port)
            .dbname(&self.config.database)
            .user(&self.config.user)
            .password(&password.0)
            .connect_timeout(Duration::from_secs(5))
            .application_name("connectors-sql");
        if self.config.allow_plaintext {
            config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            let (client, connection) = config.connect(NoTls).await.map_err(database_error)?;
            Ok((
                client,
                ConnectionTask(tokio::spawn(async move {
                    let _ = connection.await;
                })),
            ))
        } else {
            config.ssl_mode(tokio_postgres::config::SslMode::Require);
            let mut roots =
                rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            if let Some(path) = &self.config.ca_file {
                let bytes = std::fs::read(path)
                    .map_err(|_| Error::invalid("configured database CA unavailable"))?;
                for cert in rustls_pemfile::certs(&mut bytes.as_slice()) {
                    roots
                        .add(cert.map_err(|_| Error::invalid("invalid database CA"))?)
                        .map_err(|_| Error::invalid("invalid database CA"))?;
                }
            }
            let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(|_| Error::internal())?
            .with_root_certificates(roots)
            .with_no_client_auth();
            let (client, connection) = config
                .connect(tokio_postgres_rustls::MakeRustlsConnect::new(tls))
                .await
                .map_err(database_error)?;
            Ok((
                client,
                ConnectionTask(tokio::spawn(async move {
                    let _ = connection.await;
                })),
            ))
        }
    }
    async fn query(&self, args: Query) -> Result<Value> {
        if args.query.is_empty()
            || args.query.len() > 8192
            || args.parameters.len() > 64
            || !(1..=1000).contains(&args.limit)
            || args.parameters.iter().flatten().any(|p| p.len() > 8192)
        {
            return Err(Error::invalid(
                "query, parameters or row limit exceed bounds",
            ));
        }
        let (mut client, _connection) = self.connect().await?;
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(database_error)?;
        transaction.batch_execute("SET LOCAL statement_timeout = '10s'; SET LOCAL lock_timeout = '2s'; SET LOCAL search_path = public, pg_catalog").await.map_err(database_error)?;
        let query = args
            .query
            .trim()
            .strip_suffix(';')
            .unwrap_or(args.query.trim());
        let original = transaction.prepare(query).await.map_err(database_error)?;
        if original.columns().is_empty() || original.columns().len() > 256 {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "query must return between one and 256 columns",
            ));
        }
        if original.params().len() != args.parameters.len() {
            return Err(Error::invalid("parameter count does not match query"));
        }
        let columns = original
            .columns()
            .iter()
            .map(|c| Column {
                name: c.name().into(),
                native_type: c.type_().name().into(),
            })
            .collect::<Vec<_>>();
        let aliases = (0..columns.len())
            .map(|i| format!("c{i}"))
            .collect::<Vec<_>>();
        let casts = aliases
            .iter()
            .map(|c| format!("result.{c}::text"))
            .collect::<Vec<_>>()
            .join(",");
        let size = aliases
            .iter()
            .map(|c| format!("COALESCE(octet_length(result.{c}::text),0)::bigint"))
            .collect::<Vec<_>>()
            .join("+");
        // Prepared single-statement subquery plus server-generated positional aliases.
        // PostgreSQL's text representation is explicit in the native-text profile.
        let wrapped = format!(
            "SELECT CASE WHEN ({size}) > {} THEN NULL::text[] ELSE ARRAY[{casts}] END FROM ({query}) AS result({})",
            connectors_core::RESPONSE_LIMIT,
            aliases.join(",")
        );
        let statement = transaction
            .prepare(&wrapped)
            .await
            .map_err(database_error)?;
        let values = args
            .parameters
            .into_iter()
            .map(TextParameter)
            .collect::<Vec<_>>();
        let parameters = values
            .iter()
            .map(|v| v as &(dyn ToSql + Sync))
            .collect::<Vec<_>>();
        let portal = transaction
            .bind(&statement, &parameters)
            .await
            .map_err(database_error)?;
        let mut truncated = false;
        let mut rows = Vec::new();
        let mut total = 0;
        loop {
            // Fetch one bounded row at a time, not up to 1001 unbounded rows.
            let result = transaction
                .query_portal(&portal, 1)
                .await
                .map_err(database_error)?;
            let Some(row) = result.first() else { break };
            if rows.len() == args.limit as usize {
                truncated = true;
                break;
            }
            let values: Option<Vec<Option<String>>> = row.try_get(0).map_err(database_error)?;
            let values = values
                .ok_or_else(|| Error::new(ErrorCode::Capacity, "query row exceeds byte limit"))?;
            let values = values
                .into_iter()
                .map(|v| v.map(Value::String).unwrap_or(Value::Null))
                .collect::<Vec<_>>();
            total += serde_json::to_vec(&values)
                .map_err(|_| Error::internal())?
                .len();
            if total > connectors_core::RESPONSE_LIMIT - 65536 {
                return Err(Error::new(
                    ErrorCode::Capacity,
                    "query result exceeds byte limit",
                ));
            }
            rows.push(values);
        }
        transaction.rollback().await.map_err(database_error)?;
        encode(QueryResult {
            columns,
            rows,
            truncated,
            provenance: provenance(&self.descriptor.instance, &self.config.database, None),
        })
    }
}

struct ConnectionTask(tokio::task::JoinHandle<()>);
impl Drop for ConnectionTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Debug)]
struct TextParameter(Option<String>);
impl ToSql for TextParameter {
    fn to_sql(
        &self,
        _ty: &Type,
        out: &mut BytesMut,
    ) -> std::result::Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match &self.0 {
            Some(value) => {
                out.extend_from_slice(value.as_bytes());
                Ok(IsNull::No)
            }
            None => Ok(IsNull::Yes),
        }
    }
    fn accepts(_ty: &Type) -> bool {
        true
    }
    fn encode_format(&self, _ty: &Type) -> Format {
        Format::Text
    }
    tokio_postgres::types::to_sql_checked!();
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    query: String,
    #[serde(default)]
    parameters: Vec<Option<String>>,
    limit: u16,
}

#[async_trait]
impl Adapter for Sql {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let query = match operation {
            "query.read" => decode(input)?,
            "schema.list" => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Schema {
                    schema: String,
                    limit: u16,
                }
                let args: Schema = decode(input)?;
                Query{query:"SELECT table_schema, table_name, column_name, data_type, udt_name, is_nullable, ordinal_position FROM information_schema.columns WHERE table_schema = $1 ORDER BY table_name, ordinal_position".into(),parameters:vec![Some(args.schema)],limit:args.limit}
            }
            _ => {
                return Err(Error::new(
                    ErrorCode::NotFound,
                    "operation is not implemented",
                ));
            }
        };
        tokio::time::timeout(Duration::from_secs(15), self.query(query))
            .await
            .map_err(|_| Error::new(ErrorCode::Timeout, "database request timed out"))?
    }
}

fn database_error(error: tokio_postgres::Error) -> Error {
    let code = match error.code().map(|c| c.code()) {
        Some("57014") => ErrorCode::Timeout,
        Some("25006" | "42501") => ErrorCode::Forbidden,
        Some("28P01" | "28000") => ErrorCode::Unauthorized,
        Some("53300") => ErrorCode::Capacity,
        Some(c) if c.starts_with("42") || c.starts_with("22") => ErrorCode::InvalidInput,
        _ => ErrorCode::Unavailable,
    };
    Error::new(code, "database could not complete the requested read")
}
