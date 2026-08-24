//! Statement admission: the read-only fence that runs before any connection exists.
//!
//! A statement is admitted only when **all** of these hold, in order:
//!
//! 1. It fits the published character bound and is not blank.
//! 2. Its unquoted keyword tokens avoid the closed deny list below. The tokenizer distinguishes
//!    keywords from string literals and quoted identifiers, so `WHERE action = 'DROP TABLE'`
//!    passes while `DROP TABLE` cannot appear as statement material anywhere — including inside
//!    a subquery or CTE the AST walk might model imprecisely across parser versions.
//! 3. It parses, in the engine's own dialect, as **exactly one** statement.
//! 4. That statement is a read: a `SELECT`-class query (locking clauses and `SELECT INTO`
//!    refused, CTEs walked recursively), or — on MySQL only — a `SHOW`/`DESCRIBE`-class
//!    metadata read. `EXPLAIN` is deliberately refused everywhere: `EXPLAIN ANALYZE` *executes*
//!    the statement it explains, and an explain surface that admits one form but not the other
//!    is a trap.
//!
//! The parse and the token fence are belt and braces for one security decision. The parse gives
//! structure (single statement, statement class, `INTO`/locking positions); the token fence
//! closes the gap between parser versions and vendor syntax the AST may absorb silently. A
//! false refusal — a column literally named `update`, unquoted — is the accepted cost; quoting
//! the identifier lifts it.

use sqlparser::ast::{Query, SetExpr, Statement};
use sqlparser::dialect::{Dialect, MySqlDialect, PostgreSqlDialect};
use sqlparser::keywords::Keyword;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Token, Tokenizer};

use protocol::sql::MAX_STATEMENT_CHARACTERS;

use crate::SqlEngine;

/// Why a statement was refused. Carries the reason only — never wire state, never a credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRefusal {
    /// The reason, phrased for the caller.
    pub reason: String,
}

impl std::fmt::Display for AdmissionRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid_input: {}", self.reason)
    }
}

fn refuse(reason: impl Into<String>) -> AdmissionRefusal {
    AdmissionRefusal {
        reason: reason.into(),
    }
}

/// One statement that passed admission. The text is trimmed with trailing `;` removed, so it can
/// be embedded verbatim in a `DECLARE … CURSOR FOR` without smuggling a second statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedStatement {
    text: String,
}

impl AdmittedStatement {
    /// The admitted statement text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// Keyword tokens no read statement may carry unquoted.
///
/// The list is closed and deliberately broad: statement heads (`INSERT`, `CREATE`, …),
/// transaction control (the driver owns the transaction), session state (`SET`, `RESET`),
/// server administration (`KILL`, `FLUSH`, `PURGE`), and `INTO` — which only occurs in write
/// positions (`INSERT INTO`, `SELECT … INTO`, MySQL `INTO OUTFILE`/`@var`). `UPDATE` here also
/// covers `SELECT … FOR UPDATE` wherever it nests.
const DENIED_KEYWORDS: &[Keyword] = &[
    Keyword::INSERT,
    Keyword::UPDATE,
    Keyword::DELETE,
    Keyword::MERGE,
    Keyword::REPLACE,
    Keyword::DROP,
    Keyword::CREATE,
    Keyword::ALTER,
    Keyword::TRUNCATE,
    Keyword::RENAME,
    Keyword::GRANT,
    Keyword::REVOKE,
    Keyword::CALL,
    Keyword::DO,
    Keyword::LOAD,
    Keyword::LOCK,
    Keyword::UNLOCK,
    Keyword::SET,
    Keyword::RESET,
    Keyword::COPY,
    Keyword::VACUUM,
    Keyword::REINDEX,
    Keyword::LISTEN,
    Keyword::NOTIFY,
    Keyword::PREPARE,
    Keyword::DEALLOCATE,
    Keyword::DECLARE,
    Keyword::KILL,
    Keyword::INTO,
    Keyword::START,
    Keyword::BEGIN,
    Keyword::COMMIT,
    Keyword::ROLLBACK,
    Keyword::SAVEPOINT,
    Keyword::RELEASE,
    Keyword::EXECUTE,
    Keyword::ANALYZE,
    Keyword::OPTIMIZE,
    Keyword::FLUSH,
    Keyword::PURGE,
    Keyword::IMPORT,
    Keyword::INSTALL,
];

fn dialect_for(engine: SqlEngine) -> Box<dyn Dialect> {
    match engine {
        SqlEngine::MySql => Box::new(MySqlDialect {}),
        SqlEngine::Postgres => Box::new(PostgreSqlDialect {}),
    }
}

/// Admit one read statement or refuse it, before any connection or credential resolution.
///
/// # Errors
///
/// [`AdmissionRefusal`] naming the first rule the statement broke.
pub fn admit_read_statement(
    engine: SqlEngine,
    statement: &str,
) -> Result<AdmittedStatement, AdmissionRefusal> {
    let trimmed = statement.trim();
    if trimmed.is_empty() {
        return Err(refuse("the statement is empty"));
    }
    if statement.chars().count() > MAX_STATEMENT_CHARACTERS {
        return Err(refuse(format!(
            "the statement exceeds the {MAX_STATEMENT_CHARACTERS}-character bound"
        )));
    }

    let dialect = dialect_for(engine);

    // The token fence first: it also covers text the parser would refuse less legibly.
    let tokens = Tokenizer::new(dialect.as_ref(), trimmed)
        .tokenize()
        .map_err(|error| refuse(format!("the statement does not tokenize: {error}")))?;
    for token in &tokens {
        if let Token::Word(word) = token {
            if word.quote_style.is_none() && DENIED_KEYWORDS.contains(&word.keyword) {
                return Err(refuse(format!(
                    "the keyword `{}` is not admitted in a read statement; only a single \
                     SELECT/SHOW/DESCRIBE-class read is",
                    word.value.to_uppercase()
                )));
            }
        }
    }

    let statements = Parser::parse_sql(dialect.as_ref(), trimmed)
        .map_err(|error| refuse(format!("the statement does not parse as a read: {error}")))?;
    let mut statements = statements.into_iter();
    let (Some(single), None) = (statements.next(), statements.next()) else {
        return Err(refuse(
            "exactly one statement is admitted; split multi-statement input",
        ));
    };

    match &single {
        Statement::Query(query) => admit_query(query)?,
        Statement::ShowTables { .. }
        | Statement::ShowDatabases { .. }
        | Statement::ShowSchemas { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowVariables { .. }
        | Statement::ExplainTable { .. }
            if engine == SqlEngine::MySql => {}
        other => {
            return Err(refuse(format!(
                "`{}` is not an admitted read statement; only a single \
                 SELECT/SHOW/DESCRIBE-class read is",
                statement_class(other)
            )));
        }
    }

    let text = trimmed.trim_end_matches(';').trim_end().to_owned();
    Ok(AdmittedStatement { text })
}

/// The word used in a refusal for an unadmitted statement class. First keyword-ish word of the
/// statement's own rendering, so the refusal names what the caller sent without echoing it all.
fn statement_class(statement: &Statement) -> String {
    statement
        .to_string()
        .split_whitespace()
        .next()
        .unwrap_or("statement")
        .to_uppercase()
}

fn admit_query(query: &Query) -> Result<(), AdmissionRefusal> {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            admit_query(&cte.query)?;
        }
    }
    if !query.locks.is_empty() {
        return Err(refuse(
            "locking clauses (FOR UPDATE / FOR SHARE) are not admitted in a read statement",
        ));
    }
    admit_set_expr(&query.body)
}

fn admit_set_expr(body: &SetExpr) -> Result<(), AdmissionRefusal> {
    match body {
        SetExpr::Select(select) => {
            if select.into.is_some() {
                return Err(refuse(
                    "SELECT INTO creates or writes an object and is not admitted",
                ));
            }
            Ok(())
        }
        SetExpr::Query(inner) => admit_query(inner),
        SetExpr::SetOperation { left, right, .. } => {
            admit_set_expr(left)?;
            admit_set_expr(right)
        }
        SetExpr::Values(_) => Ok(()),
        // Insert/Update/Delete/Merge bodies, the bare TABLE command, and any variant a newer
        // parser adds: refused. The token fence has normally refused these earlier; this arm is
        // the structural statement of the same rule.
        _ => Err(refuse(
            "the query body is not a SELECT-class read and is not admitted",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admitted(engine: SqlEngine, sql: &str) -> AdmittedStatement {
        admit_read_statement(engine, sql)
            .unwrap_or_else(|refusal| panic!("expected `{sql}` admitted, got: {refusal}"))
    }

    fn refused(engine: SqlEngine, sql: &str) -> AdmissionRefusal {
        admit_read_statement(engine, sql)
            .map(|_| panic!("expected `{sql}` refused"))
            .unwrap_err()
    }

    /// **The acceptance fence: write statements are refused for both engines**, purely locally —
    /// no socket, no credential, nothing to reach.
    #[test]
    fn write_statements_are_refused_before_any_connection() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            for sql in [
                "INSERT INTO t (a) VALUES (1)",
                "UPDATE t SET a = 1",
                "DELETE FROM t",
                "DROP TABLE t",
                "TRUNCATE TABLE t",
                "CREATE TABLE t (a INT)",
                "ALTER TABLE t ADD COLUMN b INT",
                "GRANT SELECT ON t TO reader",
                "REVOKE SELECT ON t FROM reader",
            ] {
                refused(engine, sql);
            }
        }
    }

    #[test]
    fn multi_statement_input_is_refused() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            refused(engine, "SELECT 1; SELECT 2");
            refused(engine, "SELECT 1; DROP TABLE t");
        }
    }

    /// A data-modifying CTE is PostgreSQL's polite way to write from inside a `WITH`; both the
    /// keyword fence and the AST walk refuse it.
    #[test]
    fn a_data_modifying_cte_is_refused() {
        refused(
            SqlEngine::Postgres,
            "WITH d AS (DELETE FROM t RETURNING id) SELECT * FROM d",
        );
        refused(
            SqlEngine::Postgres,
            "WITH d AS (INSERT INTO t (a) VALUES (1) RETURNING id) SELECT * FROM d",
        );
    }

    #[test]
    fn locking_reads_are_refused() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            refused(engine, "SELECT * FROM t FOR UPDATE");
        }
        refused(SqlEngine::Postgres, "SELECT * FROM t FOR SHARE");
        // Nested inside a derived table, where a top-level AST check would miss it.
        refused(
            SqlEngine::Postgres,
            "SELECT * FROM (SELECT * FROM t FOR UPDATE) AS x",
        );
    }

    #[test]
    fn select_into_and_outfile_are_refused() {
        refused(SqlEngine::Postgres, "SELECT * INTO t2 FROM t");
        refused(
            SqlEngine::MySql,
            "SELECT * FROM t INTO OUTFILE '/tmp/exfil'",
        );
        refused(SqlEngine::MySql, "SELECT a FROM t INTO @variable");
    }

    /// `EXPLAIN ANALYZE` executes the statement it explains, so `EXPLAIN` is refused whole.
    #[test]
    fn explain_is_refused_everywhere() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            refused(engine, "EXPLAIN SELECT 1");
            refused(engine, "EXPLAIN ANALYZE UPDATE t SET a = 1");
        }
    }

    #[test]
    fn session_and_transaction_control_is_refused() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            refused(engine, "SET SESSION sql_mode = ''");
            refused(engine, "BEGIN");
            refused(engine, "COMMIT");
            refused(engine, "ROLLBACK");
        }
        refused(SqlEngine::Postgres, "COPY t TO '/tmp/exfil'");
        refused(SqlEngine::MySql, "CALL procedure_name()");
    }

    #[test]
    fn ordinary_reads_are_admitted() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            admitted(engine, "SELECT 1");
            admitted(engine, "SELECT a, b FROM t WHERE a > 1 ORDER BY b LIMIT 10");
            admitted(
                engine,
                "SELECT t.a, u.b FROM t JOIN u ON t.id = u.id WHERE u.b IS NOT NULL",
            );
            admitted(engine, "SELECT a FROM t UNION SELECT a FROM u");
            admitted(engine, "WITH x AS (SELECT 1 AS one) SELECT * FROM x");
        }
        admitted(
            SqlEngine::MySql,
            "WITH RECURSIVE cte (n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM cte WHERE n < 10) \
             SELECT n FROM cte",
        );
    }

    /// A string literal is data, not statement material: the fence must not confuse the two.
    #[test]
    fn write_keywords_inside_string_literals_are_data() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            admitted(
                engine,
                "SELECT * FROM audit WHERE action = 'DROP TABLE users'",
            );
        }
    }

    #[test]
    fn show_and_describe_are_mysql_only() {
        admitted(SqlEngine::MySql, "SHOW TABLES");
        admitted(SqlEngine::MySql, "SHOW DATABASES");
        admitted(SqlEngine::MySql, "SHOW COLUMNS FROM t");
        admitted(SqlEngine::MySql, "DESCRIBE t");
        refused(SqlEngine::Postgres, "SHOW TABLES");
    }

    #[test]
    fn blank_and_oversized_statements_are_refused() {
        for engine in [SqlEngine::MySql, SqlEngine::Postgres] {
            refused(engine, "   ");
            let oversized = format!("SELECT '{}'", "x".repeat(MAX_STATEMENT_CHARACTERS));
            refused(engine, &oversized);
        }
    }

    #[test]
    fn a_trailing_semicolon_is_admitted_and_stripped() {
        let statement = admitted(SqlEngine::Postgres, "SELECT 1;");
        assert_eq!(statement.text(), "SELECT 1");
    }
}
