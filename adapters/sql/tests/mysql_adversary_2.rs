//! Adversary cases, pass 2, for the MySQL engine. The fixture cases run in the
//! suite; the live cases need a disposable MySQL 8.0 server named by
//! `CONNECTORS_MYSQL_ADVERSARY_PORT` on loopback (user `reader`, password
//! `reader-pw`, databases `fixture` and `other`) and are ignored otherwise.
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_core::Result;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use fixture::*;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command, sync::Arc};

struct Password(&'static str);
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

fn adapter(port: u16, password: &'static str) -> Sql {
    let config = Config {
        engine: Engine::Mysql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"mysql-fixture","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "mysql-fixture",
        config,
        effective,
        Arc::new(Password(password)),
    )
    .unwrap()
}

async fn invoke(sql: &Sql, operation: &str, input: Value) -> Result<Value> {
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        sql.invoke(operation, input),
    )
    .await
    .expect("MySQL server hung")
}

/// The unit brief, outcome 4: "`schema.list` on MySQL reads
/// `information_schema` for the connected database". On MySQL a schema is a
/// database, so a caller naming another one is answered with that database's
/// columns, and the answer's provenance names the connected database as the
/// resource read. On PostgreSQL every schema lives inside the connected
/// database, so the same provenance is true there; on MySQL it is not.
#[tokio::test]
async fn mysql_schema_list_of_another_database_is_not_labelled_as_the_connected_one() {
    let text_column = |name| col(name, VAR_STRING, 0, UTF8MB4, 0);
    let script = Script {
        columns: vec![
            text_column("table_schema"),
            text_column("table_name"),
            text_column("column_name"),
            text_column("data_type"),
            text_column("udt_name"),
            text_column("is_nullable"),
            col("ordinal_position", LONG, UNSIGNED, BINARY_CHARSET, 0),
        ],
        rows: vec![vec![
            text("other"),
            text("secret"),
            text("s"),
            text("varchar"),
            text("varchar(10)"),
            text("YES"),
            int4(2),
        ]],
        params: 1,
        ..Script::default()
    };
    let server = start(script).await;
    let sql = adapter(server.port, PASSWORD);
    let answer = invoke(&sql, "schema.list", json!({"schema":"other","limit":10})).await;
    let bound = server.log.lock().unwrap().executions.clone();
    // Decided: a MySQL connection reads only its configured database, so naming
    // another one is refused before any statement runs, and no answer can carry
    // the connected database's provenance over another database's rows.
    match answer {
        Ok(result) => panic!(
            "schema.list of another database answered {} under provenance resource {} after binding {bound:?}",
            result["rows"], result["provenance"]["resource"]
        ),
        Err(error) => assert_eq!(
            error.code,
            connectors_core::ErrorCode::InvalidInput,
            "bound {bound:?}"
        ),
    }
    assert!(
        bound.is_empty(),
        "a refused schema.list still executed {bound:?}"
    );
}

/// `semantics.md` keeps PostgreSQL's promise that timestamps are preserved as
/// lossless native text; `mysql-native-text` writes TIMESTAMP as
/// `YYYY-MM-DDTHH:MM:SS` in the session time zone with no offset, and the
/// adapter leaves the session time zone at the server's default. On a server in
/// a DST zone (the default `time_zone = SYSTEM` on a host in local time), two
/// distinct instants an hour apart are written as the same text.
#[tokio::test]
#[ignore = "needs a disposable MySQL 8.0 server (CONNECTORS_MYSQL_ADVERSARY_PORT) with time_zone Europe/Berlin"]
async fn live_mysql_distinct_timestamps_are_written_distinctly() {
    let port: u16 = std::env::var("CONNECTORS_MYSQL_ADVERSARY_PORT")
        .expect("CONNECTORS_MYSQL_ADVERSARY_PORT")
        .parse()
        .unwrap();
    let sql = adapter(port, "reader-pw");
    let result = invoke(
        &sql,
        "query.read",
        json!({"query":"SELECT id, at FROM fixture.instants ORDER BY id","limit":10}),
    )
    .await
    .unwrap();
    let cells: Vec<_> = result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row[1].clone())
        .collect();
    assert_eq!(cells.len(), 2, "{result}");
    assert_ne!(
        cells[0], cells[1],
        "TIMESTAMP 2026-10-25 00:30:00 UTC and 01:30:00 UTC are written as {cells:?}"
    );
}

/// The schema of one model type, generated now by the pinned `ess`, as
/// `tests/ess_model.rs` does.
fn schema(out: &Path, name: &str) -> Value {
    let path = out.join(format!(
        "schemas/schema/types/connectors_sql.{name}.schema.json"
    ));
    if !path.exists() {
        let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
        let status = Command::new(ess)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args([
                "generate", "--path", "spec/ess", "--kind", "schema", "--out",
            ])
            .arg(out.join("schemas"))
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "ess generate exited {status}");
    }
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn admitted_by_executable(directory: &Path, configuration: &Value) -> bool {
    let path = directory.join("sql.json");
    fs::write(&path, serde_json::to_vec(configuration).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors-sql"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap()
        .status
        .success()
}

/// `connection.yaml` bounds `host`, `database` and `user` by `.count`, which ESS
/// and the generated JSON Schema (`maxLength`) count in characters; the
/// executable (`src/local.rs`) bounds them in bytes. `tests/ess_model.rs` claims
/// the model admits exactly the configurations the executable admits, and
/// checks only ASCII values, where the two counts agree.
#[test]
fn the_model_and_the_executable_count_the_same_units() {
    let out = tempfile::tempdir().unwrap();
    let model = schema(out.path(), "connection.LocalConfiguration");
    let validator = jsonschema::validator_for(&model).unwrap();
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    connectors_host::local::filesystem::directory(&directory, true, true).unwrap();
    // 300 characters, 600 bytes: inside the model's 512, outside the executable's.
    let wide = "\u{e9}".repeat(300);
    for field in ["host", "database", "user"] {
        let mut document = json!({
            "format":"connectors-sql-local/1",
            "instance":"fixture-sql",
            "engine":"mysql",
            "host":"db.example.test",
            "port":3306,
            "database":"fixture",
            "user":"reader",
            "allow_plaintext":false
        });
        document[field] = json!(wide);
        let model_admits = validator.is_valid(&document);
        let executable_admits = admitted_by_executable(&directory, &document);
        assert_eq!(
            model_admits,
            executable_admits,
            "{field} of 300 characters ({} bytes): model admits {model_admits}, executable admits {executable_admits}",
            wide.len()
        );
    }
}
