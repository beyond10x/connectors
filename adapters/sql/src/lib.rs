use async_trait::async_trait;
use bytes::BytesMut;
use connectors_contracts::{Column, QueryResult};
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{Adapter, Credential, decode, encode, instance_descriptor, provenance};
use rustls::pki_types::{CertificateDer, pem::PemObject};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc, time::Duration};

pub mod auth;
pub mod mysql;
use tokio::{sync::oneshot, time::Instant};
use tokio_postgres::{
    CancelToken, Client, NoTls,
    types::{Format, IsNull, ToSql, Type},
};
use tokio_postgres_rustls::MakeRustlsConnect;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const QUERY_TIMEOUT: Duration = Duration::from_secs(10);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// The database engine a connection speaks. PostgreSQL is the default, so a
/// configuration written before the engine existed selects it unchanged; the
/// field is omitted from the effective configuration for PostgreSQL so that
/// configuration revisions recorded earlier keep their value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[default]
    Postgresql,
    Mysql,
}
impl Engine {
    pub const ALL: [Engine; 2] = [Engine::Postgresql, Engine::Mysql];
    pub fn is_postgresql(&self) -> bool {
        *self == Engine::Postgresql
    }
    /// The engine's configuration name, which is also the scheme of its
    /// provider authority (`<scheme>://<host>:<port>/<database>`).
    pub fn name(self) -> &'static str {
        match self {
            Engine::Postgresql => "postgresql",
            Engine::Mysql => "mysql",
        }
    }
    /// The one password profile a connection on this engine advertises.
    pub fn profile_id(self) -> &'static str {
        match self {
            Engine::Postgresql => auth::PROFILE_ID,
            Engine::Mysql => auth::MYSQL_PROFILE_ID,
        }
    }
    pub fn profile_label(self) -> &'static str {
        match self {
            Engine::Postgresql => "PostgreSQL password",
            Engine::Mysql => "MySQL password",
        }
    }
    /// The kind of the principal a validated session authenticates.
    pub fn identity_kind(self) -> &'static str {
        match self {
            Engine::Postgresql => "postgresql.role",
            Engine::Mysql => "mysql.user",
        }
    }
    /// The result profile the engine's operations carry in its descriptor.
    pub fn result_profile(self) -> &'static str {
        match self {
            Engine::Postgresql => "postgresql-native-text",
            Engine::Mysql => "mysql-native-text",
        }
    }
    /// The statements that bound a read's session before the caller's
    /// statement is prepared.
    pub fn session_statements(self) -> &'static [&'static str] {
        match self {
            Engine::Postgresql => &[POSTGRESQL_SESSION],
            Engine::Mysql => &[mysql::SESSION_READ_ONLY, mysql::SESSION_BOUNDS],
        }
    }
}

/// A read must return at least one and at most this many columns on either
/// engine, so a statement that returns no rows, such as any write, is refused
/// before it is executed.
pub const MAX_COLUMNS: usize = 256;
const POSTGRESQL_SESSION: &str = "SET LOCAL statement_timeout = '10s'; SET LOCAL lock_timeout = '2s'; SET LOCAL search_path = public, pg_catalog";
/// `schema.list` on PostgreSQL: the column metadata of one schema of the
/// connected database visible to the configured role.
const POSTGRESQL_SCHEMA_QUERY: &str = "SELECT table_schema, table_name, column_name, data_type, udt_name, is_nullable, ordinal_position FROM information_schema.columns WHERE table_schema = $1 ORDER BY table_name, ordinal_position";

fn column_bounds(columns: usize) -> Result<()> {
    if columns == 0 || columns > MAX_COLUMNS {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "query must return between one and 256 columns",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Engine::is_postgresql")]
    pub engine: Engine,
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
        let mut descriptor = instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            &effective_configuration,
        )?;
        // The operations are the same on both engines; the profile names how a
        // cell is written, which is the engine's own.
        for operation in &mut descriptor.operations {
            operation.profile = config.engine.result_profile().into();
        }
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
            .connect_timeout(CONNECT_TIMEOUT)
            .application_name("connectors-sql");
        if self.config.allow_plaintext {
            config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            let (client, connection) = config.connect(NoTls).await.map_err(database_error)?;
            let cancel = client.cancel_token();
            Ok((
                client,
                ConnectionTask {
                    task: tokio::spawn(async move {
                        let _ = connection.await;
                    }),
                    cancel,
                    tls: None,
                },
            ))
        } else {
            config.ssl_mode(tokio_postgres::config::SslMode::Require);
            let roots = database_roots(self.config.ca_file.as_deref())?;
            let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(|_| Error::internal())?
            .with_root_certificates(roots)
            .with_no_client_auth();
            let tls = MakeRustlsConnect::new(tls);
            let (client, connection) = config.connect(tls.clone()).await.map_err(database_error)?;
            let cancel = client.cancel_token();
            Ok((
                client,
                ConnectionTask {
                    task: tokio::spawn(async move {
                        let _ = connection.await;
                    }),
                    cancel,
                    tls: Some(tls),
                },
            ))
        }
    }
    /// Native credential validation. The server accepts or rejects the password
    /// during the startup exchange, so opening one session and closing it is the
    /// whole check: no statement runs and no business read is performed. A
    /// rejected password surfaces as the provider's own authentication failure.
    pub async fn validate_session(&self) -> Result<()> {
        if self.config.engine == Engine::Mysql {
            return mysql::validate_session(&self.config, &*self.password).await;
        }
        let (client, connection) = tokio::time::timeout(CONNECT_TIMEOUT, self.connect())
            .await
            .map_err(|_| query_timeout())??;
        connection.close(client, false).await;
        Ok(())
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
        if self.config.engine == Engine::Mysql {
            return mysql::query(
                &self.config,
                &*self.password,
                &self.descriptor.instance,
                args,
            )
            .await;
        }
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let (mut client, connection) = tokio::time::timeout(CONNECT_TIMEOUT, self.connect())
            .await
            .map_err(|_| query_timeout())??;
        // Reserve cleanup time inside the 15 s request bound, even after a slow
        // connection. A caller cannot extend this deadline with set_config().
        let work_deadline = (Instant::now() + QUERY_TIMEOUT).min(deadline - CLEANUP_TIMEOUT);
        let instance = self.descriptor.instance.clone();
        let database = self.config.database.clone();
        let (mut send, receive) = oneshot::channel();
        // The supervisor outlives a dropped invocation long enough to cancel.
        // Merely aborting the connection driver can leave PostgreSQL executing.
        tokio::spawn(async move {
            let result = tokio::select! {
                biased;
                _ = send.closed() => None,
                result = tokio::time::timeout_at(work_deadline, Self::read(&mut client, args, &instance, &database)) => {
                    Some(result.unwrap_or_else(|_| Err(query_timeout())))
                }
            };
            let cancel = result.as_ref().is_none_or(|result| result.is_err());
            connection.close(client, cancel).await;
            if let Some(result) = result {
                let _ = send.send(result);
            }
        });
        receive.await.map_err(|_| Error::internal())?
    }

    async fn read(
        client: &mut Client,
        args: Query,
        instance: &str,
        database: &str,
    ) -> Result<Value> {
        let transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .await
            .map_err(database_error)?;
        transaction
            .batch_execute(POSTGRESQL_SESSION)
            .await
            .map_err(database_error)?;
        let query = args
            .query
            .trim()
            .strip_suffix(';')
            .unwrap_or(args.query.trim());
        let original = transaction.prepare(query).await.map_err(database_error)?;
        column_bounds(original.columns().len())?;
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
            provenance: provenance(instance, database, None),
        })
    }
}

fn query_timeout() -> Error {
    Error::new(ErrorCode::Timeout, "database request timed out")
}

struct ConnectionTask {
    task: tokio::task::JoinHandle<()>,
    cancel: CancelToken,
    tls: Option<MakeRustlsConnect>,
}
impl ConnectionTask {
    async fn close(mut self, client: Client, cancel: bool) {
        let cleanup = async {
            if cancel {
                // The token retains the actual connected address and backend
                // key; reuse the captured TLS roots, never a mutable CA path.
                if let Some(tls) = self.tls.take() {
                    let _ = self.cancel.cancel_query(tls).await;
                } else {
                    let _ = self.cancel.cancel_query(NoTls).await;
                }
            }
            drop(client);
            let _ = (&mut self.task).await;
        };
        // CancelRequest has no acknowledgement. Drain the original driver when
        // possible, but cap cleanup even if cancellation or the server stalls.
        let _ = tokio::time::timeout(CLEANUP_TIMEOUT, cleanup).await;
    }
}
impl Drop for ConnectionTask {
    fn drop(&mut self) {
        self.task.abort();
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
        connectors_sdk::validate(&self.descriptor.operation(operation)?.input_schema, &input)?;
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
                let query = match self.config.engine {
                    Engine::Postgresql => POSTGRESQL_SCHEMA_QUERY,
                    Engine::Mysql => mysql::SCHEMA_QUERY,
                };
                Query {
                    query: query.into(),
                    parameters: vec![Some(args.schema)],
                    limit: args.limit,
                }
            }
            _ => {
                return Err(Error::new(
                    ErrorCode::NotFound,
                    "operation is not implemented",
                ));
            }
        };
        self.query(query).await
    }
}

fn database_roots(path: Option<&std::path::Path>) -> Result<rustls::RootCertStore> {
    let Some(path) = path else {
        return Ok(rustls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        ));
    };
    let mut roots = rustls::RootCertStore::empty();
    for cert in database_certificates(path)? {
        roots
            .add(cert)
            .map_err(|_| Error::invalid("invalid database CA"))?;
    }
    Ok(roots)
}

/// The certificates of a configured CA bundle, read once per session. Both
/// engines refuse an unreadable, malformed or empty bundle the same way.
fn database_certificates(path: &std::path::Path) -> Result<Vec<CertificateDer<'static>>> {
    let bytes =
        std::fs::read(path).map_err(|_| Error::invalid("configured database CA unavailable"))?;
    let certificates = CertificateDer::pem_slice_iter(&bytes)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| Error::invalid("invalid database CA"))?;
    if certificates.is_empty() {
        return Err(Error::invalid(
            "configured database CA contains no certificates",
        ));
    }
    Ok(certificates)
}

fn database_error(error: tokio_postgres::Error) -> Error {
    sqlstate_error(error.code().map(|c| c.code()))
}
fn sqlstate_error(state: Option<&str>) -> Error {
    let code = match state {
        Some("0A000") => ErrorCode::Unsupported,
        Some("57014") => ErrorCode::Timeout,
        Some("25006" | "42501") => ErrorCode::Forbidden,
        Some("28P01" | "28000") => ErrorCode::Unauthorized,
        Some("53300") => ErrorCode::Capacity,
        Some(c) if c.starts_with("42") || c.starts_with("22") => ErrorCode::InvalidInput,
        _ => ErrorCode::Unavailable,
    };
    let answered = matches!(code, ErrorCode::Timeout | ErrorCode::Capacity);
    let error = Error::new(code, "database could not complete the requested read");
    // The database's statement timeout and its connection limit are its own
    // answers to a sent request; the adapter's deadlines (`query_timeout`) are not.
    if answered { error.answered() } else { error }
}

#[cfg(test)]
mod error_tests {
    use super::*;
    #[test]
    fn the_databases_statement_timeout_and_capacity_answers_are_marked_and_own_deadlines_are_not() {
        for (state, code, answered) in [
            ("57014", ErrorCode::Timeout, true),
            ("53300", ErrorCode::Capacity, true),
            ("42501", ErrorCode::Forbidden, false),
            ("42601", ErrorCode::InvalidInput, false),
            ("08006", ErrorCode::Unavailable, false),
        ] {
            let error = sqlstate_error(Some(state));
            assert_eq!(error.code, code, "{state}");
            assert_eq!(error.upstream_answer, answered, "{state}");
        }
        let error = sqlstate_error(None);
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert!(!error.upstream_answer);
        // The adapter's own connect and work deadlines are not the database's.
        assert!(!query_timeout().upstream_answer);
    }
}

#[cfg(test)]
mod tls_tests {
    use super::database_roots;
    #[test]
    fn configured_ca_replaces_public_roots_and_requires_certificates() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ca");
        assert!(database_roots(None).unwrap().len() > 1);
        std::fs::write(&path, cert.cert.pem()).unwrap();
        let roots = database_roots(Some(&path)).unwrap();
        assert_eq!(roots.len(), 1);
        assert!(
            !roots
                .roots
                .iter()
                .any(|root| webpki_roots::TLS_SERVER_ROOTS.contains(root))
        );
        std::fs::write(&path, "").unwrap();
        assert!(database_roots(Some(&path)).is_err());
        std::fs::write(
            &path,
            "-----BEGIN CERTIFICATE-----\ninvalid\n-----END CERTIFICATE-----",
        )
        .unwrap();
        assert!(database_roots(Some(&path)).is_err());
    }
}
