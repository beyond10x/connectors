//! Adversary cases, pass 2, for the catalogue reads after the correction of
//! pass 1. Every case needs a disposable PostgreSQL 17 server on loopback and
//! is ignored otherwise; each was measured on 2026-10-09.
//!
//! `CONNECTORS_PG_ADVERSARY_PORT` names a server whose database `fixture`
//! holds the setup of `catalogue_adversary.rs` and, created by a superuser:
//!
//! ```sql
//! CREATE TABLE app.part_ref (k int, r int, PRIMARY KEY (k, r)) PARTITION BY RANGE (k);
//! CREATE TABLE app.part_ref_lo PARTITION OF app.part_ref FOR VALUES FROM (0) TO (100);
//! CREATE TABLE app.part_ref_hi PARTITION OF app.part_ref FOR VALUES FROM (100) TO (200);
//! CREATE TABLE app.to_part (x int, y int,
//!   CONSTRAINT to_part_fk FOREIGN KEY (x, y) REFERENCES app.part_ref (k, r));
//! GRANT SELECT ON app.part_ref, app.part_ref_lo, app.part_ref_hi, app.to_part TO reader;
//! CREATE TABLE app.whole (pub int, priv int);
//! CREATE INDEX whole_row ON app.whole ((whole IS NOT NULL));
//! CREATE INDEX whole_pub ON app.whole (pub);
//! GRANT SELECT (pub) ON app.whole TO reader;
//! CREATE TABLE other.colpar (a int UNIQUE, secretcol int UNIQUE);
//! GRANT SELECT (a) ON other.colpar TO reader;
//! CREATE TABLE app.colchild (x int CONSTRAINT colchild_fk REFERENCES other.colpar (secretcol));
//! GRANT SELECT ON app.colchild TO reader;
//! ```
//!
//! The cases after the first three pin the correction of pass 2 and also need,
//! created by a superuser:
//!
//! ```sql
//! CREATE TABLE app.fk_parted (x int, y int,
//!   CONSTRAINT fk_parted_fk FOREIGN KEY (x, y) REFERENCES app.part_ref (k, r))
//!   PARTITION BY RANGE (x);
//! CREATE TABLE app.fk_parted_a PARTITION OF app.fk_parted FOR VALUES FROM (0) TO (200);
//! CREATE TABLE other.colpar2 (a int, s int, UNIQUE (a, s));
//! GRANT SELECT (a) ON other.colpar2 TO reader;
//! CREATE TABLE app.colchild2 (x int, y int,
//!   CONSTRAINT colchild2_fk FOREIGN KEY (x, y) REFERENCES other.colpar2 (a, s));
//! CREATE TABLE app.whole2 (pub int, priv int);
//! CREATE INDEX whole2_pred ON app.whole2 (pub) WHERE whole2 IS NOT NULL;
//! CREATE INDEX whole2_mixed ON app.whole2 (pub, (whole2 IS NOT NULL));
//! CREATE INDEX whole2_expr ON app.whole2 ((pub + 1));
//! CREATE INDEX whole2_pub ON app.whole2 (pub);
//! GRANT SELECT (pub) ON app.whole2 TO reader;
//! GRANT SELECT ON app.fk_parted, app.fk_parted_a, app.colchild2 TO reader;
//! ```
use async_trait::async_trait;
use connectors_core::Result;
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

struct Password(&'static str);
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

fn reader() -> Sql {
    let port: u16 = std::env::var("CONNECTORS_PG_ADVERSARY_PORT")
        .expect("CONNECTORS_PG_ADVERSARY_PORT names the live server's port")
        .parse()
        .unwrap();
    let config = Config {
        engine: Engine::Postgresql,
        host: "127.0.0.1".into(),
        port,
        database: "fixture".into(),
        user: "reader".into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"catalogue-adversary-2","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new(
        "catalogue-adversary-2",
        config,
        effective,
        Arc::new(Password("reader-pw")),
    )
    .unwrap()
}

async fn invoke(sql: &Sql, operation: &str, input: Value) -> Value {
    tokio::time::timeout(Duration::from_secs(15), sql.invoke(operation, input))
        .await
        .expect("server hung")
        .unwrap()
}

/// The contract: "A column in a foreign key carries the constraint name and
/// the referenced schema, table and column paired with it ... a column in
/// several foreign keys has one row per key". `app.to_part` has one foreign
/// key, `to_part_fk`, into the partitioned `app.part_ref`. PostgreSQL 12+
/// also stores one internal clone of it per referenced partition on the
/// referencing table (`conparentid <> 0`, names `to_part_x_y_fkey` and
/// `to_part_x_y_fkey1`, pointing at the partitions). The statement reads every
/// `contype = 'f'` row of `conrelid`, so each column is answered with three
/// foreign keys, two of which no one declared. Measured: six rows for two
/// columns.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_a_foreign_key_into_a_partitioned_table_is_reported_once() {
    let described = invoke(
        &reader(),
        "table.describe",
        json!({"schema":"app","table":"to_part","limit":20}),
    )
    .await;
    assert_eq!(
        described["rows"],
        json!([
            [
                "x",
                "integer",
                "YES",
                null,
                "1",
                null,
                "to_part_fk",
                "app",
                "part_ref",
                "k"
            ],
            [
                "y",
                "integer",
                "YES",
                null,
                "2",
                null,
                "to_part_fk",
                "app",
                "part_ref",
                "r"
            ]
        ])
    );
}

/// The contract (semantics.md, § Catalogue reads): on PostgreSQL "an index
/// follows column visibility: it is left out when any column it is built on is
/// one `table.describe` hides ... a column used in a key expression". The
/// expression of `whole_row` is a whole-row reference, built on `priv` as much
/// as on `pub`; PostgreSQL records it in `pg_depend` with `refobjsubid = 0`,
/// which the statement's `refobjsubid > 0` skips. Measured: `whole_row` is
/// listed for a role that holds `SELECT (pub)` only.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_index_list_leaves_out_a_whole_row_expression_over_a_hidden_column() {
    let indexes = invoke(
        &reader(),
        "index.list",
        json!({"schema":"app","table":"whole","limit":20}),
    )
    .await;
    assert_eq!(
        indexes["rows"],
        json!([["whole_pub", "whole", "1", "pub", "NO", "NO"]])
    );
}

/// The correction's own rule for `index.list` is that the reads never name a
/// column `table.describe` hides. `table.describe` of `other.colpar` hides
/// `secretcol` (the role holds `SELECT (a)` only), yet `table.describe` of
/// `app.colchild` names it as the referenced column of `colchild_fk`, because
/// the foreign-key filter asks for some privilege on the referenced table, not
/// on the referenced column.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_a_foreign_key_does_not_name_a_referenced_column_describe_hides() {
    let sql = reader();
    let parent = invoke(
        &sql,
        "table.describe",
        json!({"schema":"other","table":"colpar","limit":20}),
    )
    .await;
    let parent_columns: Vec<&Value> = parent["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| &row[0])
        .collect();
    assert_eq!(parent_columns, [&json!("a")]);
    let child = invoke(
        &sql,
        "table.describe",
        json!({"schema":"app","table":"colchild","limit":20}),
    )
    .await;
    let named: Vec<&Value> = child["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row[9] == "secretcol")
        .collect();
    assert!(
        named.is_empty(),
        "table.describe names a referenced column table.describe hides: {named:?}"
    );
}

fn key_columns(described: &Value) -> Vec<Value> {
    described["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| json!([row[0], row[6], row[7], row[8], row[9]]))
        .collect()
}

/// The contract: a foreign key a partition inherits from its partitioned
/// parent is the partition's own key and stays reported when the partition is
/// described, while the parent's internal clones into the partitions of a
/// referenced partitioned table are not. `app.fk_parted` declares one key into
/// the partitioned `app.part_ref`; its partition `app.fk_parted_a` inherits it.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_a_partition_keeps_the_foreign_key_it_inherits() {
    let sql = reader();
    for table in ["fk_parted", "fk_parted_a"] {
        let described = invoke(
            &sql,
            "table.describe",
            json!({"schema":"app","table":table,"limit":20}),
        )
        .await;
        assert_eq!(
            key_columns(&described),
            [
                json!(["x", "fk_parted_fk", "app", "part_ref", "k"]),
                json!(["y", "fk_parted_fk", "app", "part_ref", "r"]),
            ],
            "{table}"
        );
    }
}

/// The contract: a foreign key is left out when the role holds no privilege
/// on any one of its referenced columns, so a composite key into a partly
/// hidden unique key is left out on every column, not only the hidden one.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_a_composite_foreign_key_with_one_hidden_referenced_column_is_left_out() {
    let described = invoke(
        &reader(),
        "table.describe",
        json!({"schema":"app","table":"colchild2","limit":20}),
    )
    .await;
    assert_eq!(
        key_columns(&described),
        [
            json!(["x", null, null, null, null]),
            json!(["y", null, null, null, null]),
        ]
    );
}

/// The contract: a whole-row reference counts as a reference to every column
/// wherever the index makes it. PostgreSQL records no `pg_depend` row for a
/// whole-row reference itself (measured on 17: `whole2_pred` and `whole2_mixed`
/// depend on `pub` alone), so a `refobjsubid = 0` row does not find the one in
/// a predicate (`whole2_pred`) or beside a plain key column (`whole2_mixed`).
/// The `refobjsubid = 0` row PostgreSQL does write is the whole-table
/// dependency of an index with no plain key column, which the contract also
/// counts as every column: `whole2_expr`, over `pub` alone, is left out too.
#[tokio::test]
#[ignore = "needs the PostgreSQL server named by CONNECTORS_PG_ADVERSARY_PORT"]
async fn postgresql_index_list_leaves_out_a_whole_row_predicate_over_a_hidden_column() {
    let indexes = invoke(
        &reader(),
        "index.list",
        json!({"schema":"app","table":"whole2","limit":20}),
    )
    .await;
    assert_eq!(
        indexes["rows"],
        json!([["whole2_pub", "whole2", "1", "pub", "NO", "NO"]])
    );
}
