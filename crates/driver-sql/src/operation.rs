use serde::Deserialize;
use serde_json::Value;

use crate::{credentials::CredentialSource, SqlConnectionConfig, SqlDriverError, SqlEngine};

/// A parsed, bounded read operation. Obtain it before resolving credentials or opening a route.
#[derive(Debug)]
pub struct AdmittedSqlOperation {
    engine: SqlEngine,
    input: Input,
}

#[derive(Debug)]
enum Input {
    Query(protocol::sql::SqlQueryInput),
    Schemas,
    Tables(String),
    Table(String, String),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryInput {
    statement: String,
    #[serde(default)]
    max_rows: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TablesInput {
    schema: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TableInput {
    schema: String,
    table: String,
}

fn invalid() -> SqlDriverError {
    SqlDriverError::InvalidInput {
        reason: "operation or input does not match the admitted SQL provider".into(),
    }
}

/// Validate a catalog SQL operation before runtime credentials or transport are acquired.
///
/// # Errors
/// Returns `InvalidInput` for an unknown operation, engine mismatch, write, or invalid bound.
pub fn admit_operation(
    engine: SqlEngine,
    operation_ref: &str,
    input: Value,
) -> Result<AdmittedSqlOperation, SqlDriverError> {
    let operations = match engine {
        SqlEngine::MySql => protocol::sql::MYSQL_OPERATIONS,
        SqlEngine::Postgres => protocol::sql::POSTGRESQL_OPERATIONS,
    };
    let index = operations
        .iter()
        .position(|id| *id == operation_ref)
        .ok_or_else(invalid)?;
    let input = match index {
        0 => {
            let value: QueryInput = serde_json::from_value(input).map_err(|_| invalid())?;
            crate::admission::admit_read_statement(engine, &value.statement).map_err(
                |refusal| SqlDriverError::InvalidInput {
                    reason: refusal.reason,
                },
            )?;
            crate::effective_max_rows(value.max_rows)?;
            Input::Query(protocol::sql::SqlQueryInput {
                statement: value.statement,
                max_rows: value.max_rows,
            })
        }
        1 => {
            if !input.as_object().is_some_and(|fields| fields.is_empty()) {
                return Err(invalid());
            }
            Input::Schemas
        }
        2 => {
            let value: TablesInput = serde_json::from_value(input).map_err(|_| invalid())?;
            crate::require_identifier("schema", &value.schema)?;
            Input::Tables(value.schema)
        }
        _ => {
            let value: TableInput = serde_json::from_value(input).map_err(|_| invalid())?;
            crate::require_identifier("schema", &value.schema)?;
            crate::require_identifier("table", &value.table)?;
            Input::Table(value.schema, value.table)
        }
    };
    Ok(AdmittedSqlOperation { engine, input })
}

impl AdmittedSqlOperation {
    /// Execute using an already admitted runtime binding and current credential source.
    ///
    /// # Errors
    /// Refuses an engine mismatch and propagates bounded driver failures without secret values.
    pub async fn execute(
        self,
        config: &SqlConnectionConfig,
        credentials: &dyn CredentialSource,
    ) -> Result<Value, SqlDriverError> {
        if self.engine != config.engine {
            return Err(invalid());
        }
        let result = match self.input {
            Input::Query(input) => {
                serde_json::to_value(crate::run_query(config, credentials, &input).await?)
            }
            Input::Schemas => serde_json::to_value(crate::list_schemas(config, credentials).await?),
            Input::Tables(schema) => {
                serde_json::to_value(crate::list_tables(config, credentials, &schema).await?)
            }
            Input::Table(schema, table) => serde_json::to_value(
                crate::describe_table(config, credentials, &schema, &table).await?,
            ),
        };
        result.map_err(|_| SqlDriverError::Query {
            detail: "database result encoding failed".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn discovery_cannot_make_unknown_writes_or_cross_provider_operations_callable() {
        for (engine, prefix) in [
            (SqlEngine::MySql, "mysql"),
            (SqlEngine::Postgres, "postgresql"),
        ] {
            assert!(admit_operation(
                engine,
                &format!("{prefix}-query"),
                json!({"statement":"DELETE FROM data"})
            )
            .is_err());
            assert!(admit_operation(
                engine,
                &format!("{prefix}-query"),
                json!({"statement":"SELECT 1", "max_rows":0})
            )
            .is_err());
            assert!(admit_operation(
                engine,
                &format!("{prefix}-query"),
                json!({"statement":"SELECT 1", "host":"caller-host"})
            )
            .is_err());
            assert!(admit_operation(
                engine,
                &format!("{prefix}-query"),
                json!({"statement":"SELECT 1"})
            )
            .is_ok());
        }
        assert!(admit_operation(
            SqlEngine::Postgres,
            "mysql-query",
            json!({"statement":"SELECT 1"})
        )
        .is_err());
        assert!(admit_operation(
            SqlEngine::MySql,
            "mysql-schemas-list",
            json!({"database":"other"})
        )
        .is_err());
    }
}
