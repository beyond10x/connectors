//! What a read-only MySQL 8 server answers to the reads of a caller that pages with its own
//! `LIMIT … OFFSET …`. Every test is ignored: it needs a disposable MySQL 8 server on
//! loopback, named by `CONNECTORS_MYSQL_REPLICA_PORT`, prepared as below and then put in
//! `super_read_only`. That server stands in for a managed replica; no managed service is
//! tested here. The server behind `CONNECTORS_MYSQL_ADVERSARY_PORT` is written to by another
//! test and cannot be this one.
//!
//! Start the server and prepare it as `root`, with the statements between the two marker
//! lines (the `mysql` client reads `DELIMITER`):
//!
//! ```sh
//! docker run -d --name connectors-mysql-replica -e MYSQL_ROOT_PASSWORD=root-pw \
//!   -p 127.0.0.1:33306:3306 mysql:8
//! docker exec -i connectors-mysql-replica mysql -uroot -proot-pw < replica.sql
//! CONNECTORS_MYSQL_REPLICA_PORT=33306 cargo test --locked -p connectors-sql \
//!   --test mysql_replica -- --ignored --nocapture
//! ```
//!
//! The base table has a unique `id` and no primary key, and its 250 rows are inserted from
//! 250 down to 1, so the order the table is stored in is not the order of `id`: a page that
//! lost the statement's `ORDER BY` would hold other rows. Each joined table holds at most one
//! row per base `id`, so the joins add no row. The function writes with its definer's
//! rights, so the caller's grants do not refuse it.
//!
//! ```sql
//! -- replica.sql: begin
//! CREATE DATABASE fixture;
//! CREATE USER 'reader'@'%' IDENTIFIED BY 'reader-pw';
//! GRANT SELECT, EXECUTE ON fixture.* TO 'reader'@'%';
//! CREATE TABLE fixture.item (
//!   id INT NULL, name VARCHAR(64) NULL, created_at DATETIME NOT NULL,
//!   removed_at DATETIME NULL, UNIQUE KEY item_id (id));
//! CREATE TABLE fixture.item_detail (item_id INT NULL UNIQUE, checked_at DATETIME NOT NULL);
//! CREATE TABLE fixture.item_owner (item_id INT NULL UNIQUE, owner_id INT NOT NULL);
//! CREATE TABLE fixture.item_label (item_id INT NULL UNIQUE, label_id INT NOT NULL);
//! INSERT INTO fixture.item (id, name, created_at)
//!   WITH RECURSIVE n (i) AS (SELECT 250 UNION ALL SELECT i - 1 FROM n WHERE i > 1)
//!   SELECT i, CONCAT('item-', i), TIMESTAMP('2026-01-01 00:00:00') + INTERVAL i MINUTE FROM n;
//! INSERT INTO fixture.item_detail
//!   SELECT id, created_at + INTERVAL 1 HOUR FROM fixture.item WHERE id % 2 = 0 ORDER BY id DESC;
//! INSERT INTO fixture.item_owner
//!   SELECT id, id % 7 FROM fixture.item WHERE id % 3 = 0 ORDER BY id DESC;
//! INSERT INTO fixture.item_label
//!   SELECT id, id % 5 FROM fixture.item WHERE id <= 100 ORDER BY id DESC;
//! SET GLOBAL log_bin_trust_function_creators = ON;
//! DELIMITER //
//! CREATE FUNCTION fixture.write_a_row() RETURNS INT
//!   MODIFIES SQL DATA SQL SECURITY DEFINER
//! BEGIN
//!   INSERT INTO fixture.item_label (item_id, label_id) VALUES (NULL, 0);
//!   RETURN 1;
//! END//
//! DELIMITER ;
//! SET GLOBAL super_read_only = ON;
//! -- replica.sql: end
//! ```
use async_trait::async_trait;
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Engine, Sql, mysql::SESSION_READ_ONLY};
use mysql_async::prelude::Queryable;
use serde_json::{Value, json};
use std::{collections::BTreeSet, sync::Arc, time::Duration};

const VARIABLE: &str = "CONNECTORS_MYSQL_REPLICA_PORT";
const DATABASE: &str = "fixture";
const USER: &str = "reader";
const PASSWORD: &str = "reader-pw";

/// The caller's statement: one base table, three `LEFT JOIN`s, a correlated `COUNT(*)`,
/// the filters `IS NULL`, `IS NOT NULL` and `!= ''`, seven columns, ordered by the key.
const STATEMENT: &str = "SELECT i.id, i.name, i.created_at, d.checked_at, o.owner_id, l.label_id, \
     (SELECT COUNT(*) FROM item_detail AS c WHERE c.item_id = i.id) AS details \
     FROM item AS i \
     LEFT JOIN item_detail AS d ON d.item_id = i.id \
     LEFT JOIN item_owner AS o ON o.item_id = i.id \
     LEFT JOIN item_label AS l ON l.item_id = i.id \
     WHERE i.removed_at IS NULL AND i.name IS NOT NULL AND i.name != '' \
     ORDER BY i.id";

struct Password;
#[async_trait]
impl Credential for Password {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(PASSWORD.as_bytes().to_vec()))
    }
}

fn port() -> u16 {
    std::env::var(VARIABLE)
        .unwrap_or_else(|_| panic!("{VARIABLE} names the server's loopback port"))
        .parse()
        .unwrap()
}

fn adapter() -> Sql {
    let config = Config {
        engine: Engine::Mysql,
        host: "127.0.0.1".into(),
        port: port(),
        database: DATABASE.into(),
        user: USER.into(),
        allow_plaintext: true,
        ca_file: None,
    };
    let effective = json!({"service":{"instance":"mysql-replica","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Sql::new("mysql-replica", config, effective, Arc::new(Password)).unwrap()
}

async fn read(sql: &Sql, query: &str, limit: u16) -> Result<Value> {
    tokio::time::timeout(
        Duration::from_secs(15),
        sql.invoke("query.read", json!({"query": query, "limit": limit})),
    )
    .await
    .expect("MySQL server hung")
}

/// The first cell of every row, as the number it is written as.
fn ids(result: &Value) -> Vec<u32> {
    result["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row[0].as_str().unwrap().parse().unwrap())
        .collect()
}

/// Print whether a page arrived in the statement's order; the tests assert its rows as a set.
fn report_sequence(page: &str, ids: &[u32]) {
    if ids.is_sorted() {
        println!("{page}: the rows arrived in ascending id order");
    } else {
        println!("{page}: the rows did not arrive in ascending id order: {ids:?}");
    }
}

/// A session of the test's own, as the configured user, to read the server's error number:
/// the connector's answer carries a code and never the server's number or message.
async fn session() -> mysql_async::Conn {
    let options = mysql_async::OptsBuilder::default()
        .ip_or_hostname("127.0.0.1")
        .tcp_port(port())
        .user(Some(USER))
        .pass(Some(PASSWORD))
        .db_name(Some(DATABASE))
        .prefer_socket(false);
    mysql_async::Conn::new(options).await.unwrap()
}

/// The server's error number for one statement, or `None` when it ran.
async fn error_number(session: &mut mysql_async::Conn, statement: &str) -> Option<u16> {
    match session.query_drop(statement).await {
        Ok(()) => None,
        Err(mysql_async::Error::Server(error)) => Some(error.code),
        Err(other) => panic!("{statement}: not a server answer: {other}"),
    }
}

#[tokio::test]
#[ignore = "needs a disposable MySQL 8 server in super_read_only (CONNECTORS_MYSQL_REPLICA_PORT), prepared as the head of this file says"]
async fn a_read_only_server_accepts_the_session_and_answers_a_read() {
    let sql = adapter();
    sql.validate_session().await.unwrap();
    let mut direct = session().await;
    let version: Option<String> = direct.query_first("SELECT VERSION()").await.unwrap();
    println!("server version: {}", version.unwrap());
    direct.disconnect().await.unwrap();

    let variable = read(&sql, "SELECT @@global.super_read_only", 1)
        .await
        .unwrap();
    assert_eq!(variable["rows"], json!([["1"]]), "{variable}");
    assert_eq!(variable["truncated"], false);

    let first = read(&sql, "SELECT id FROM item ORDER BY id", 5)
        .await
        .unwrap();
    assert_eq!(first["rows"].as_array().unwrap().len(), 5, "{first}");
    assert_eq!(first["truncated"], true);
}

#[tokio::test]
#[ignore = "needs a disposable MySQL 8 server in super_read_only (CONNECTORS_MYSQL_REPLICA_PORT), prepared as the head of this file says"]
async fn a_write_through_a_function_is_refused_by_a_read_only_server() {
    const WRITE: &str = "SELECT write_a_row()";
    // The server's own refusal, in a session that is not READ ONLY.
    let mut direct = session().await;
    let server = error_number(&mut direct, WRITE).await;
    println!("without the read-only session: MySQL error {server:?}");
    // The refusal a session of the connector meets: its first statement is this one.
    assert_eq!(error_number(&mut direct, SESSION_READ_ONLY).await, None);
    let in_session = error_number(&mut direct, WRITE).await;
    println!("in a READ ONLY session: MySQL error {in_session:?}");
    direct.disconnect().await.unwrap();
    assert!(server.is_some(), "the server ran the write");
    assert!(in_session.is_some(), "the read-only session ran the write");

    let error = read(&adapter(), WRITE, 1).await.unwrap_err();
    println!("query.read answered: {:?}", error.code);
    assert_eq!(error.code, ErrorCode::Forbidden);

    // No row was written: the function's table holds what the setup put there.
    let written = read(
        &adapter(),
        "SELECT COUNT(*) FROM item_label WHERE item_id IS NULL",
        1,
    )
    .await
    .unwrap();
    assert_eq!(written["rows"], json!([["0"]]), "{written}");
}

#[tokio::test]
#[ignore = "needs a disposable MySQL 8 server in super_read_only (CONNECTORS_MYSQL_REPLICA_PORT), prepared as the head of this file says"]
async fn the_first_page_holds_the_first_hundred_rows() {
    let page = read(&adapter(), STATEMENT, 100).await.unwrap();
    let ids = ids(&page);
    report_sequence("first page", &ids);
    assert_eq!(page["columns"].as_array().unwrap().len(), 7, "{page}");
    assert_eq!(ids.len(), 100, "{ids:?}");
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>(),
        (1..=100).collect::<BTreeSet<_>>()
    );
    assert_eq!(page["truncated"], true);
}

#[tokio::test]
#[ignore = "needs a disposable MySQL 8 server in super_read_only (CONNECTORS_MYSQL_REPLICA_PORT), prepared as the head of this file says"]
async fn a_page_selected_by_limit_and_offset_holds_its_rows() {
    let page = read(
        &adapter(),
        &format!("{STATEMENT} LIMIT 100 OFFSET 100"),
        100,
    )
    .await
    .unwrap();
    let ids = ids(&page);
    report_sequence("page LIMIT 100 OFFSET 100", &ids);
    assert_eq!(ids.len(), 100, "{ids:?}");
    assert_eq!(
        ids.iter().copied().collect::<BTreeSet<_>>(),
        (101..=200).collect::<BTreeSet<_>>()
    );
    assert_eq!(page["truncated"], false);
}

#[tokio::test]
#[ignore = "needs a disposable MySQL 8 server in super_read_only (CONNECTORS_MYSQL_REPLICA_PORT), prepared as the head of this file says"]
async fn a_count_over_the_statement_answers_its_rows() {
    let count = read(
        &adapter(),
        &format!("SELECT COUNT(*) AS n FROM ({STATEMENT}) AS matched"),
        1,
    )
    .await
    .unwrap();
    assert_eq!(count["rows"], json!([["250"]]), "{count}");
    assert_eq!(count["truncated"], false);
}
