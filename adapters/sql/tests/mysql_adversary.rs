//! Adversary cases for the MySQL engine: each drives the implementation from a
//! document the unit wrote about itself (the reads contract, the ESS model).
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

fn adapter(port: u16) -> Sql {
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
        Arc::new(Password(PASSWORD)),
    )
    .unwrap()
}

/// `semantics.md` (MySQL engine) and `connectors_sql.reads.CellRendering`
/// (`float_text`): a floating value is written as "the shortest text that reads
/// back as the same value". 1e300 reads back from the five characters `1e300`;
/// a live MySQL 8.0 server through this adapter wrote it as 301 characters.
#[tokio::test]
async fn mysql_float_text_is_the_shortest_text_that_reads_back() {
    let script = Script {
        columns: vec![
            col("large", DOUBLE, 0, BINARY_CHARSET, 31),
            col("small", DOUBLE, 0, BINARY_CHARSET, 31),
        ],
        rows: vec![vec![double(1e300), double(1e-300)]],
        ..Script::default()
    };
    let server = start(script).await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        adapter(server.port).invoke(
            "query.read",
            json!({"query":"SELECT large, small FROM t","limit":1}),
        ),
    )
    .await
    .expect("MySQL fixture hung")
    .unwrap();
    for (index, expected) in [(0, 1e300_f64), (1, 1e-300_f64)] {
        let text = result["rows"][0][index].as_str().unwrap();
        assert_eq!(text.parse::<f64>().unwrap(), expected, "reads back");
        let shortest = format!("{expected:e}");
        assert!(
            text.len() <= shortest.len(),
            "column {index}: {} characters where {shortest:?} ({} characters) reads back as the same value",
            text.len(),
            shortest.len()
        );
    }
}

/// `tests/ess_model.rs` claims the model admits exactly the configurations the
/// executable admits. `"ca_file": null` is a configuration the executable
/// admits (the unit's own `tests/mysql_local.rs` documents use it), and the
/// generated `connectors_sql.connection.LocalConfiguration` schema refuses it:
/// `ca_file` is `{"type":"string"}`. That test's helper turns a null change
/// into a removed member, so it never presents this document.
#[test]
fn the_model_and_the_executable_agree_on_a_null_ca_file() {
    let out = tempfile::tempdir().unwrap();
    let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
    let status = Command::new(ess)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "generate", "--path", "spec/ess", "--kind", "schema", "--out",
        ])
        .arg(out.path())
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let schema: Value = serde_json::from_slice(
        &fs::read(
            out.path()
                .join("schema/types/connectors_sql.connection.LocalConfiguration.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("private");
    connectors_host::local::filesystem::directory(&directory, true, true).unwrap();
    for engine in ["mysql", "postgresql"] {
        let document = json!({
            "format":"connectors-sql-local/1",
            "instance":"fixture-sql",
            "engine": engine,
            "host":"db.example.test",
            "port":3306,
            "database":"fixture",
            "user":"reader",
            "allow_plaintext":false,
            "ca_file":null
        });
        let executable = bootstrap(&directory, &document);
        let model: Vec<String> = validator
            .iter_errors(&document)
            .map(|e| e.to_string())
            .collect();
        assert_eq!(
            model.is_empty(),
            executable,
            "{engine}: the executable admits={executable}, the model's errors={model:?}"
        );
    }
}

fn bootstrap(directory: &Path, configuration: &Value) -> bool {
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
