//! Adversary cases for story:mysql-query-read-provenance: the documented option (grants are
//! the boundary, provenance names the connected database) read against the wire and against
//! the pages that describe the adapter.
#![cfg(unix)]
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use serde_json::json;
use std::{path::Path, sync::Arc};

struct Password;
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(fixture::PASSWORD.as_bytes().to_vec()))
    }
}

fn mysql(port: u16) -> Sql {
    let config = Config {
        engine: Engine::Mysql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"fixture-sql","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new("fixture-sql", config, effective, Arc::new(Password)).unwrap()
}

/// The contract says the adapter "neither refuses it nor reports it": the other-database
/// statement reaches the server as written, on a session logged in to the connected
/// database, and the answer is labelled with the connected database.
#[tokio::test]
async fn other_database_statement_reaches_the_server_as_written() {
    let server = fixture::start(fixture::Script::default()).await;
    let answer = mysql(server.port)
        .invoke(
            "query.read",
            json!({"query": "SELECT * FROM other.t", "limit": 10}),
        )
        .await
        .unwrap();
    assert_eq!(answer["provenance"]["resource"], "fixture");
    let log = server.log.lock().unwrap();
    assert!(
        log.logins.iter().all(|l| l.database == "fixture"),
        "{:?}",
        log.logins
    );
    assert_eq!(
        log.prepares.first().map(String::as_str),
        Some("SELECT * FROM other.t"),
        "{:?}",
        log.prepares
    );
}

fn words(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The unit's contract paragraph states that a MySQL `query.read` may read another database
/// the configured user is granted. The adapter's website page must not tell a reader the
/// opposite.
#[test]
fn website_page_does_not_claim_a_mysql_connection_reads_only_its_database() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let contract = words(&root.join("adapters/sql/contracts/reads/v1alpha1/semantics.md"));
    assert!(
        contract.contains("On MySQL a statement may name a table in another database"),
        "the contract no longer states the documented option"
    );
    let page = words(&root.join("website/docs/reference/adapters/sql/index.md"));
    assert!(
        !page.contains("A MySQL connection reads only its configured database"),
        "website/docs/reference/adapters/sql/index.md says a MySQL connection reads only its \
         configured database, while the reads contract says query.read may read another one"
    );
}
