//! Adversary cases, pass 1, for the catalogue reads (`database.list`,
//! `table.list`, `table.describe`, `index.list`). The fixture case runs in the
//! suite. The live cases need disposable servers on loopback and are ignored
//! otherwise; each was measured against PostgreSQL 17 and MySQL 8.0 on
//! 2026-10-09.
//!
//! `CONNECTORS_PG_ADVERSARY_PORT` names a PostgreSQL 17 server whose database
//! `fixture` holds, created by a superuser:
//!
//! ```sql
//! CREATE ROLE reader LOGIN PASSWORD 'reader-pw';
//! CREATE SCHEMA app; CREATE SCHEMA other;
//! CREATE TABLE other.parent (a int, b int, PRIMARY KEY (b, a));
//! CREATE TABLE app.child2 (p int, q int,
//!   CONSTRAINT z_fk FOREIGN KEY (q, p) REFERENCES other.parent (b, a),
//!   CONSTRAINT a_fk FOREIGN KEY (p, q) REFERENCES other.parent (b, a));
//! CREATE TABLE app."Orders" (id int PRIMARY KEY);
//! CREATE TABLE app.orders (id int PRIMARY KEY, total numeric(10,2));
//! CREATE INDEX orders_expr ON app.orders ((id + 1), lower(total::text));
//! CREATE INDEX orders_incl ON app.orders (total) INCLUDE (id);
//! CREATE UNIQUE INDEX orders_total_u ON app.orders (total);
//! CREATE TABLE app.gen (a int, b int GENERATED ALWAYS AS (a * 2) STORED);
//! CREATE TABLE app.partial (pub int, priv int);
//! CREATE INDEX partial_priv ON app.partial (priv);
//! CREATE INDEX partial_expr ON app.partial ((priv + 1));
//! CREATE INDEX partial_incl ON app.partial (pub) INCLUDE (priv);
//! CREATE INDEX partial_pred ON app.partial (pub) WHERE priv > 0;
//! CREATE INDEX partial_pub ON app.partial (pub);
//! CREATE SCHEMA hidden;
//! CREATE TABLE hidden.secret (s int PRIMARY KEY);
//! CREATE TABLE other.unseen (u int PRIMARY KEY);
//! CREATE TABLE app.refs (id int PRIMARY KEY,
//!   s int CONSTRAINT refs_secret_fk REFERENCES hidden.secret (s),
//!   u int CONSTRAINT refs_unseen_fk REFERENCES other.unseen (u),
//!   o int CONSTRAINT refs_orders_fk REFERENCES app.orders (id));
//! GRANT USAGE ON SCHEMA app, other TO reader;
//! GRANT SELECT ON other.parent, app.child2, app."Orders", app.orders, app.gen TO reader;
//! GRANT SELECT (pub) ON app.partial TO reader;
//! GRANT SELECT ON app.refs, hidden.secret TO reader;
//! ```
//!
//! The role holds `SELECT` on `hidden.secret` but no `USAGE` on `hidden`, and
//! `USAGE` on `other` but no privilege on `other.unseen`.
//!
//! `CONNECTORS_MYSQL_ADVERSARY_PORT` names a MySQL 8.0 server (default
//! `lower_case_table_names = 0`) holding, created by root:
//!
//! ```sql
//! CREATE USER 'reader'@'%' IDENTIFIED BY 'reader-pw';
//! CREATE DATABASE fixture;
//! CREATE TABLE fixture.`Orders` (id int PRIMARY KEY, upper_only int);
//! CREATE TABLE fixture.orders (id int PRIMARY KEY, total decimal(10,2));
//! GRANT SELECT ON fixture.* TO 'reader'@'%';
//! ```
#[path = "mysql/fixture.rs"]
mod fixture;

use async_trait::async_trait;
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use fixture::*;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

struct Password(&'static str);
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

fn adapter(engine: Engine, port: u16, password: &'static str) -> Sql {
    let config = Config {
        engine,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"catalogue-adversary","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "catalogue-adversary",
        config,
        effective,
        Arc::new(Password(password)),
    )
    .unwrap()
}

fn port(variable: &str) -> u16 {
    std::env::var(variable)
        .unwrap_or_else(|_| panic!("{variable} names the live server's port"))
        .parse()
        .unwrap()
}

async fn invoke(sql: &Sql, operation: &str, input: Value) -> Result<Value> {
    tokio::time::timeout(Duration::from_secs(15), sql.invoke(operation, input))
        .await
        .expect("server hung")
}

/// MySQL stores identifiers in `utf8mb3`, so a table name with a
/// supplementary character cannot exist. Measured on MySQL 8.0.46: binding
/// such a name against `information_schema.COLUMNS.TABLE_NAME` fails with
/// error 3988 (`HY000`, "Conversion from collation utf8mb4_0900_ai_ci into
/// utf8mb3_bin impossible for parameter"). The contract
/// (`contracts/reads/v1alpha1/semantics.md`, § Catalogue reads) says a table
/// that does not exist is `not_found`; the caller's own input is at worst
/// `invalid_input`. `unavailable` tells the caller the server is at fault and
/// invites a retry that can never succeed.
#[tokio::test]
async fn mysql_table_describe_of_a_name_mysql_cannot_hold_is_not_unavailable() {
    let server = start(Script {
        columns: describe_columns(),
        rows: vec![],
        params: 2,
        execute_error: Some((3988, "HY000")),
        ..Script::default()
    })
    .await;
    let error = invoke(
        &adapter(Engine::Mysql, server.port, PASSWORD),
        "table.describe",
        json!({"table":"\u{1F600}","limit":10}),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error.code, ErrorCode::NotFound | ErrorCode::InvalidInput),
        "a table name MySQL cannot hold is answered as {:?}",
        error.code
    );
}

/// The contract: "`column_default` the engine's own text of the default or
/// null". A stored generated column has no default; PostgreSQL keeps its
/// generation expression in `pg_attrdef`, and `information_schema.columns`
/// reports `column_default` null for it (the expression is
/// `generation_expression`). MySQL's `COLUMN_DEFAULT` is null for the same
/// column. Measured: PostgreSQL answers `"(a * 2)"` as the default.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_generated_column_is_not_reported_as_a_default() {
    let sql = adapter(
        Engine::Postgresql,
        port("CONNECTORS_PG_ADVERSARY_PORT"),
        "reader-pw",
    );
    let result = invoke(
        &sql,
        "table.describe",
        json!({"schema":"app","table":"gen","limit":10}),
    )
    .await
    .unwrap();
    let b = result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row[0] == "b")
        .expect("column b is described");
    assert_eq!(b[3], Value::Null, "generated column b: {b}");
}

/// The contract: "Visibility is the engine's: on PostgreSQL a relation or
/// column is listed when the role owns it (or is a member of its owner) or
/// holds a privilege on it". `table.describe` honours that per column (the
/// role holds `SELECT (pub)` only, and `priv` is not described), but
/// `index.list` checks the table alone and names `priv`, the column
/// `table.describe` hides.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_index_list_does_not_name_a_column_the_role_cannot_see() {
    let sql = adapter(
        Engine::Postgresql,
        port("CONNECTORS_PG_ADVERSARY_PORT"),
        "reader-pw",
    );
    let described = invoke(
        &sql,
        "table.describe",
        json!({"schema":"app","table":"partial","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        described["rows"],
        json!([[
            "pub", "integer", "YES", null, "1", null, null, null, null, null
        ]])
    );
    let indexes = invoke(
        &sql,
        "index.list",
        json!({"schema":"app","table":"partial","limit":10}),
    )
    .await
    .unwrap();
    let named: Vec<&Value> = indexes["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row[3].as_str().is_some_and(|c| c.contains("priv")))
        .collect();
    assert!(
        named.is_empty(),
        "index.list names a column the role holds no privilege on: {named:?}"
    );
}

/// The contract: "A table with no row is refused as `not_found`, whether it
/// does not exist or ...". MySQL forbids a table name ending in a space, so
/// `orders ` does not exist; `information_schema`'s `utf8mb3_bin` is a PAD
/// SPACE collation, so `TABLE_NAME = ?` matches `orders` and the read answers
/// another table's columns under the name asked for. PostgreSQL answers
/// `not_found` for the same input.
#[tokio::test]
#[ignore = "needs the MySQL server named by CONNECTORS_MYSQL_ADVERSARY_PORT"]
async fn mysql_table_describe_of_a_name_with_a_trailing_space_is_not_found() {
    let sql = adapter(
        Engine::Mysql,
        port("CONNECTORS_MYSQL_ADVERSARY_PORT"),
        "reader-pw",
    );
    let exact = invoke(&sql, "table.describe", json!({"table":"orders","limit":10}))
        .await
        .unwrap();
    assert_eq!(exact["rows"].as_array().unwrap().len(), 2);
    let padded = invoke(
        &sql,
        "table.describe",
        json!({"table":"orders ","limit":10}),
    )
    .await;
    assert_eq!(
        padded.as_ref().map_err(|e| e.code.clone()),
        Err(ErrorCode::NotFound),
        "describe of `orders ` answered {padded:?}"
    );
    // A name MySQL cannot hold is `not_found` on `index.list` too, before any
    // statement runs (the decision on this pass), not an empty answer.
    let indexes = invoke(&sql, "index.list", json!({"table":"orders ","limit":10})).await;
    assert_eq!(
        indexes.as_ref().map_err(|e| e.code.clone()),
        Err(ErrorCode::NotFound),
        "index.list of `orders ` answered {indexes:?}"
    );
}

/// The live counterpart of the fixture case above: MySQL answers 3988 to a
/// table name with a supplementary character on both table reads, and each
/// read answers `not_found`.
#[tokio::test]
#[ignore = "needs the MySQL server named by CONNECTORS_MYSQL_ADVERSARY_PORT"]
async fn mysql_a_name_mysql_cannot_represent_is_not_found_on_a_live_server() {
    let sql = adapter(
        Engine::Mysql,
        port("CONNECTORS_MYSQL_ADVERSARY_PORT"),
        "reader-pw",
    );
    for operation in ["table.describe", "index.list"] {
        let answer = invoke(
            &sql,
            operation,
            json!({"table":"orders\u{1F600}","limit":10}),
        )
        .await;
        assert_eq!(
            answer.as_ref().map_err(|e| e.code.clone()),
            Err(ErrorCode::NotFound),
            "{operation} answered {answer:?}"
        );
    }
}

/// The whole rule `index.list` follows on PostgreSQL: an index is left out
/// when any column it is built on (a key column, an INCLUDE column, or a
/// column in a key expression or in its predicate) is one `table.describe`
/// hides. Only `partial_pub`, built on `pub` alone, remains.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_index_list_leaves_out_every_index_over_a_hidden_column() {
    let sql = adapter(
        Engine::Postgresql,
        port("CONNECTORS_PG_ADVERSARY_PORT"),
        "reader-pw",
    );
    let indexes = invoke(
        &sql,
        "index.list",
        json!({"schema":"app","table":"partial","limit":20}),
    )
    .await
    .unwrap();
    assert_eq!(
        indexes["rows"],
        json!([["partial_pub", "partial", "1", "pub", "NO", "NO"]])
    );
    // Without a table, the schema-wide answer applies the same rule.
    let every = invoke(&sql, "index.list", json!({"schema":"app","limit":100}))
        .await
        .unwrap();
    let partial: Vec<&Value> = every["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row[1] == "partial")
        .collect();
    assert_eq!(
        partial,
        [&json!(["partial_pub", "partial", "1", "pub", "NO", "NO"])]
    );
}

/// `table.describe` reports a foreign key only when the role holds `USAGE` on
/// the referenced schema and some privilege on the referenced table, as
/// `information_schema` does. `refs_secret_fk` points into `hidden`, where the
/// role holds `SELECT` on the table but no `USAGE` on the schema;
/// `refs_unseen_fk` points at `other.unseen`, whose schema the role may use but
/// on which it holds no privilege. Both columns are still described, with no
/// foreign key; `refs_orders_fk` points at a visible table and is reported.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_foreign_keys_into_what_the_role_cannot_see_are_left_out() {
    let sql = adapter(
        Engine::Postgresql,
        port("CONNECTORS_PG_ADVERSARY_PORT"),
        "reader-pw",
    );
    let refs = invoke(
        &sql,
        "table.describe",
        json!({"schema":"app","table":"refs","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        refs["rows"],
        json!([
            [
                "id", "integer", "NO", null, "1", "1", null, null, null, null
            ],
            [
                "s", "integer", "YES", null, "2", null, null, null, null, null
            ],
            [
                "u", "integer", "YES", null, "3", null, null, null, null, null
            ],
            [
                "o",
                "integer",
                "YES",
                null,
                "4",
                null,
                "refs_orders_fk",
                "app",
                "orders",
                "id"
            ]
        ])
    );
}

/// Green on 2026-10-09; kept as live evidence for the brief's known doubts.
/// `ROWS FROM (unnest(conkey), unnest(confkey))` pairs a composite foreign key
/// column by column even when key order differs from column order and the
/// referenced table is in another schema; `indkey::smallint[]` with ordinality
/// and `pg_get_indexdef` give expression key parts and stop before INCLUDE
/// columns; mixed-case names are exact.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_composite_keys_expressions_and_case_on_a_live_server() {
    let sql = adapter(
        Engine::Postgresql,
        port("CONNECTORS_PG_ADVERSARY_PORT"),
        "reader-pw",
    );
    let child = invoke(
        &sql,
        "table.describe",
        json!({"schema":"app","table":"child2","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        child["rows"],
        json!([
            [
                "p", "integer", "YES", null, "1", null, "a_fk", "other", "parent", "b"
            ],
            [
                "p", "integer", "YES", null, "1", null, "z_fk", "other", "parent", "a"
            ],
            [
                "q", "integer", "YES", null, "2", null, "a_fk", "other", "parent", "a"
            ],
            [
                "q", "integer", "YES", null, "2", null, "z_fk", "other", "parent", "b"
            ]
        ])
    );
    let indexes = invoke(
        &sql,
        "index.list",
        json!({"schema":"app","table":"orders","limit":20}),
    )
    .await
    .unwrap();
    assert_eq!(
        indexes["rows"],
        json!([
            ["orders_expr", "orders", "1", "(id + 1)", "NO", "NO"],
            [
                "orders_expr",
                "orders",
                "2",
                "lower(total::text)",
                "NO",
                "NO"
            ],
            ["orders_incl", "orders", "1", "total", "NO", "NO"],
            ["orders_pkey", "orders", "1", "id", "YES", "YES"],
            ["orders_total_u", "orders", "1", "total", "YES", "NO"]
        ])
    );
    let upper = invoke(
        &sql,
        "table.describe",
        json!({"schema":"app","table":"Orders","limit":10}),
    )
    .await
    .unwrap();
    assert_eq!(
        upper["rows"],
        json!([[
            "id", "integer", "NO", null, "1", "1", null, null, null, null
        ]])
    );
}

/// Green on 2026-10-09; kept as live evidence for the brief's known doubt on
/// `TABLE_NAME = COALESCE(?, TABLE_NAME)`: with `lower_case_table_names = 0`
/// the metadata collation is `utf8mb3_bin`, so `Orders` and `orders` are two
/// tables, and an absent table reads every table.
#[tokio::test]
#[ignore = "needs the MySQL server named by CONNECTORS_MYSQL_ADVERSARY_PORT"]
async fn mysql_table_names_are_case_sensitive_on_a_live_server() {
    let sql = adapter(
        Engine::Mysql,
        port("CONNECTORS_MYSQL_ADVERSARY_PORT"),
        "reader-pw",
    );
    let upper = invoke(&sql, "table.describe", json!({"table":"Orders","limit":10}))
        .await
        .unwrap();
    assert_eq!(
        upper["rows"],
        json!([
            ["id", "int", "NO", null, "1", "1", null, null, null, null],
            [
                "upper_only",
                "int",
                "YES",
                null,
                "2",
                null,
                null,
                null,
                null,
                null
            ]
        ])
    );
    let indexes = invoke(&sql, "index.list", json!({"table":"Orders","limit":10}))
        .await
        .unwrap();
    assert_eq!(
        indexes["rows"],
        json!([["PRIMARY", "Orders", "1", "id", "YES", "YES"]])
    );
    let missing = invoke(&sql, "table.describe", json!({"table":"ORDERS","limit":10})).await;
    assert_eq!(missing.map_err(|e| e.code).err(), Some(ErrorCode::NotFound));
}
