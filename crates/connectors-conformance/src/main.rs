//! Live acceptance runner: talks to actual independently started services.
use clap::Parser;
use connectors_client::Client;
use connectors_core::{Descriptor, ErrorCode};
use connectors_host::credentials::CredentialRef;
use connectors_sdk::Credential;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Verify the configured Kubernetes and PostgreSQL live acceptance environment")]
struct Args {
    #[arg(long)]
    token_file: PathBuf,
    #[arg(long, default_value = "http://127.0.0.1:17102/")]
    kubernetes: String,
    #[arg(long, default_value = "http://127.0.0.1:17103/")]
    sql: String,
    /// Gateway address; without it the federated groups are reported as not run.
    #[arg(long)]
    gateway: Option<String>,
    #[arg(long, default_value = "engineering")]
    namespace: String,
    #[arg(long)]
    allow_plaintext: bool,
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn require(condition: bool, message: &'static str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

struct Groups {
    run: Vec<String>,
    not_run: Vec<String>,
}

fn group_label(name: &str, placement: &str) -> String {
    format!("{name}: {placement} supported operation and refusal scenarios")
}

fn groups(with_gateway: bool) -> Groups {
    let mut groups = Groups {
        run: Vec::new(),
        not_run: Vec::new(),
    };
    for name in ["kubernetes", "sql"] {
        groups
            .run
            .push(format!("{name}: authenticated discovery and token refusal"));
        groups.run.push(group_label(name, "direct"));
        if with_gateway {
            groups.run.push(group_label(name, "federated"));
        } else {
            groups.not_run.push(group_label(name, "federated"));
        }
    }
    groups
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let secret = CredentialRef::File {
        path: args.token_file,
    }
    .resolve()
    .await?;
    let token = std::str::from_utf8(&secret.0)
        .map_err(|_| "invalid service token")?
        .to_owned();
    drop(secret);
    let gateway = match &args.gateway {
        Some(url) => {
            let client = Client::new(url, token.clone(), args.allow_plaintext)?;
            let descriptor = client.describe().await?;
            Some((client, descriptor))
        }
        None => None,
    };
    let not_run = groups(gateway.is_some()).not_run;
    let mut checks = Vec::new();
    for (name, url) in [("kubernetes", &args.kubernetes), ("sql", &args.sql)] {
        let direct = Client::new(url, token.clone(), args.allow_plaintext)?;
        let descriptor = direct.describe().await?;
        require(
            descriptor.adapter == name,
            "descriptor adapter identity mismatch",
        )?;
        let denied = Client::new(
            url,
            "deliberately-wrong-fixture-token".into(),
            args.allow_plaintext,
        )?;
        require(
            denied.describe().await.unwrap_err().code == ErrorCode::Unauthorized,
            "unauthenticated discovery was not refused",
        )?;
        checks.push(format!("{name}: authenticated discovery and token refusal"));
        let mut placements = vec![(&direct, &descriptor, "".to_owned(), "direct")];
        if let Some((client, desc)) = &gateway {
            placements.push((client, desc, format!("{name}__"), "federated"));
        }
        for (client, desc, prefix, placement) in placements {
            match name {
                "kubernetes" => kubernetes(client, desc, &prefix, &args.namespace).await?,
                "sql" => sql(client, desc, &prefix).await?,
                _ => unreachable!(),
            }
            checks.push(group_label(name, placement));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"status":"passed","wire":"v1alpha1","checks":checks,"not_run":not_run,"evidence":"live configured upstreams; scoped Kubernetes service account, PostgreSQL reader role"})
        )?
    );
    Ok(())
}

async fn call(
    client: &Client,
    descriptor: &Descriptor,
    prefix: &str,
    operation: &str,
    input: Value,
) -> Result<Value> {
    Ok(client
        .invoke(descriptor, &format!("{prefix}{operation}"), input)
        .await?)
}

async fn kubernetes(
    client: &Client,
    descriptor: &Descriptor,
    prefix: &str,
    namespace: &str,
) -> Result<()> {
    let services = call(
        client,
        descriptor,
        prefix,
        "resources.list",
        json!({"namespace":namespace,"kind":"services","limit":100}),
    )
    .await?;
    require(
        services["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|v| v["metadata"]["name"] == "postgres")),
        "test PostgreSQL service was not discovered",
    )?;
    let endpoints = call(
        client,
        descriptor,
        prefix,
        "endpoints.discover",
        json!({"namespace":namespace,"limit":100}),
    )
    .await?;
    require(
        endpoints["items"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|v| v["service"] == "postgres" && v["port"] == 5432 && v["ready"] == true)
        }),
        "test PostgreSQL endpoint was not discovered",
    )?;
    require(
        endpoints["provenance"]["source_revision"].is_string(),
        "Kubernetes resourceVersion was lost",
    )?;
    let hosts = call(
        client,
        descriptor,
        prefix,
        "hosts.discover",
        json!({"limit":100}),
    )
    .await?;
    require(
        hosts["items"].as_array().is_some_and(|v| !v.is_empty()),
        "no Kubernetes host observations",
    )?;
    let refused = client
        .invoke(
            descriptor,
            &format!("{prefix}endpoints.discover"),
            json!({"namespace":"outside-configured-scope","limit":1}),
        )
        .await
        .unwrap_err();
    require(
        refused.code == ErrorCode::Forbidden,
        "Kubernetes namespace scope was not enforced",
    )
}

async fn sql(client: &Client, descriptor: &Descriptor, prefix: &str) -> Result<()> {
    let schema = call(
        client,
        descriptor,
        prefix,
        "schema.list",
        json!({"schema":"public","limit":100}),
    )
    .await?;
    require(
        schema["rows"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|r| r[1] == "connector_fixture")),
        "PostgreSQL fixture schema was not visible",
    )?;
    let result=call(client,descriptor,prefix,"query.read",json!({"query":"SELECT id, exact_value, nullable_value FROM connector_fixture WHERE id >= $1::integer ORDER BY id","parameters":["1"],"limit":2})).await?;
    require(
        result["rows"][0][1] == "12345678901234567890.123456789" && result["rows"][0][2].is_null(),
        "numeric precision or SQL NULL was lost",
    )?;
    require(
        result["truncated"] == true && result["rows"].as_array().unwrap().len() == 2,
        "SQL limit/truncation mismatch",
    )?;
    let empty = call(
        client,
        descriptor,
        prefix,
        "query.read",
        json!({"query":"SELECT id FROM connector_fixture WHERE false","limit":1}),
    )
    .await?;
    require(
        empty["rows"] == json!([]) && empty["columns"][0]["native_type"] == "int4",
        "empty SQL result lost column schema",
    )?;
    let types=call(client,descriptor,prefix,"query.read",json!({"query":"SELECT $1::text IS NULL AS absent, ARRAY[1,NULL,3]::integer[] AS values, '{\"nested\":[1,null]}'::jsonb AS document","parameters":[null],"limit":1})).await?;
    require(
        types["rows"][0][0] == "true" && types["rows"][0][1] == "{1,NULL,3}",
        "parameter NULL or native array representation was lost",
    )?;
    let document: Value =
        serde_json::from_str(types["rows"][0][2].as_str().ok_or("missing JSON value")?)?;
    require(
        document == json!({"nested":[1,null]}),
        "native JSON representation was lost",
    )?;
    let oversized = client
        .invoke(
            descriptor,
            &format!("{prefix}query.read"),
            json!({"query":"SELECT repeat('x', 5000000)","limit":1}),
        )
        .await
        .unwrap_err();
    require(
        oversized.code == ErrorCode::Capacity,
        "oversized SQL row was not bounded",
    )?;
    for query in [
        "DELETE FROM connector_fixture",
        "SELECT 1; DELETE FROM connector_fixture",
    ] {
        let refused = client
            .invoke(
                descriptor,
                &format!("{prefix}query.read"),
                json!({"query":query,"limit":1}),
            )
            .await;
        require(
            refused.is_err(),
            "mutating/multiple SQL statements were not refused",
        )?;
    }
    let count = call(
        client,
        descriptor,
        prefix,
        "query.read",
        json!({"query":"SELECT count(*) FROM connector_fixture","limit":1}),
    )
    .await?;
    require(
        count["rows"][0][0] == "3",
        "refused query changed the database",
    )?;
    let cancelled = client
        .invoke(
            descriptor,
            &format!("{prefix}query.read"),
            json!({"query":"SELECT pg_sleep(30)","limit":1}),
        )
        .await
        .unwrap_err();
    require(
        cancelled.code == ErrorCode::Timeout,
        "database statement deadline did not fire",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gateway_reports_federated_groups_not_run_by_name() {
        let groups = groups(false);
        assert_eq!(groups.run.len(), 4);
        assert!(groups.run.iter().all(|g| !g.contains("federated")));
        assert_eq!(
            groups.not_run,
            [
                "kubernetes: federated supported operation and refusal scenarios",
                "sql: federated supported operation and refusal scenarios"
            ]
        );
    }

    #[test]
    fn gateway_runs_every_group() {
        let groups = groups(true);
        assert_eq!(groups.run.len(), 6);
        assert!(groups.not_run.is_empty());
    }
}
