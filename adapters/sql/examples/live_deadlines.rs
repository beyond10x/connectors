//! Explicit local PostgreSQL regression; not part of the offline test suite.
//! cargo run --locked --offline -p connectors-sql --example live_deadlines
use async_trait::async_trait;
use connectors_core::{ErrorCode, Result as ConnectorResult};
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_sql::{Config, Sql};
use serde_json::json;
use std::{path::Path, process::Command, sync::Arc, time::Duration};
use tokio::time::Instant;
use tokio_postgres::{Client, NoTls};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
struct FixtureCredential;
#[async_trait]
impl Credential for FixtureCredential {
    async fn resolve(&self) -> ConnectorResult<Secret> {
        // The disposable database uses trust authentication on a loopback port.
        Ok(Secret(b"disposable-fixture".to_vec()))
    }
}
fn docker(args: &[&str]) -> Result<String> {
    let output = Command::new("docker").args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "docker {}: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().into())
}
struct Container(String);
impl Drop for Container {
    fn drop(&mut self) {
        let result = docker(&["stop", "--time", "2", &self.0]);
        println!(
            "{}",
            json!({"cleanup_container":self.0,"stopped_and_auto_removed":result.is_ok()})
        );
    }
}
fn adapter(port: u16, ca: Option<&Path>, plaintext: bool) -> Result<Sql> {
    let config = Config {
        host: "127.0.0.1".into(),
        port,
        database: "postgres".into(),
        user: "reader".into(),
        allow_plaintext: plaintext,
        ca_file: ca.map(Path::to_owned),
    };
    let effective = json!({"service":{"instance":"sql-live","listen":"127.0.0.1:0","service_credential":{"kind":"environment","name":"UNUSED"}},"password":{"kind":"environment","name":"UNUSED"},"adapter":config});
    Ok(Sql::new(
        "sql-live",
        config,
        effective,
        Arc::new(FixtureCredential),
    )?)
}
async fn active(admin: &Client) -> Result<i64> {
    Ok(admin.query_one("SELECT count(*) FROM pg_stat_activity WHERE application_name='connectors-sql' AND state='active'", &[]).await?.get(0))
}
async fn wait_active(admin: &Client, wanted: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if (active(admin).await? > 0) == wanted {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("unexpected surviving or missing SQL backend".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
async fn timeout_case(admin: &Client, sql: &Sql, label: &str, query: &str) -> Result<()> {
    let started = Instant::now();
    let error = sql
        .invoke("query.read", json!({"query":query,"limit":10}))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(started.elapsed() < Duration::from_secs(15));
    wait_active(admin, false).await?;
    println!(
        "{}",
        json!({"case":label,"elapsed_ms":started.elapsed().as_millis(),"error":error.code,"active_backends":active(admin).await?})
    );
    Ok(())
}

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    let endpoint = match std::env::var("DOCKER_HOST") {
        Ok(endpoint) => endpoint,
        Err(_) => docker(&[
            "context",
            "inspect",
            "--format",
            "{{.Endpoints.docker.Host}}",
        ])?,
    };
    if !endpoint.starts_with("unix://") {
        return Err("live regression requires a local Unix-socket Docker daemon".into());
    }
    let image = docker(&[
        "image",
        "inspect",
        "postgres:17-alpine",
        "--format",
        "{{.Id}}",
    ])?;
    let temp = tempfile::Builder::new().prefix("sql-live-").tempdir()?;
    let name = format!(
        "connectors-{}",
        temp.path()
            .file_name()
            .ok_or("missing temporary directory name")?
            .to_str()
            .ok_or("invalid temporary name")?
    );
    docker(&[
        "run",
        "-d",
        "--rm",
        "--pull=never",
        "--name",
        &name,
        "--label",
        "connectors-v2.review-regression=true",
        "--memory",
        "512m",
        "--tmpfs",
        "/var/lib/postgresql/data:rw,size=256m",
        "-e",
        "POSTGRES_HOST_AUTH_METHOD=trust",
        "-p",
        "127.0.0.1::5432",
        &image,
    ])?;
    let container = Container(name);
    let address = docker(&["port", &container.0, "5432/tcp"])?;
    let port = address
        .rsplit(':')
        .next()
        .ok_or("missing published port")?
        .parse()?;
    let deadline = Instant::now() + Duration::from_secs(30);
    let (admin, connection) = loop {
        let mut config = tokio_postgres::Config::new();
        config
            .host("127.0.0.1")
            .port(port)
            .user("postgres")
            .dbname("postgres")
            .connect_timeout(Duration::from_secs(1));
        match config.connect(NoTls).await {
            Ok(connection) => break connection,
            Err(error) if Instant::now() >= deadline => return Err(error.into()),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    };
    let driver = tokio::spawn(async move {
        let _ = connection.await;
    });
    let version: String = admin.query_one("SELECT version()", &[]).await?.get(0);
    println!(
        "{}",
        json!({"database":version,"image":image,"container":container.0})
    );
    admin.batch_execute("CREATE ROLE reader LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE; CREATE TABLE public.review_sample(id integer); INSERT INTO public.review_sample VALUES (42); GRANT SELECT ON public.review_sample TO reader").await?;

    let plain = adapter(port, None, true)?;
    let schema = plain
        .invoke("schema.list", json!({"schema":"public","limit":100}))
        .await?;
    assert!(
        schema["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row[1] == "review_sample" && row[2] == "id")
    );
    println!(
        "{}",
        json!({"case":"schema-list","rows":schema["rows"].as_array().unwrap().len()})
    );
    timeout_case(
        &admin,
        &plain,
        "plain-control",
        "SELECT pg_sleep(30)::text AS value",
    )
    .await?;
    let bypass = "SELECT set_config('statement_timeout','0',true) AS value UNION ALL SELECT pg_sleep(40)::text";
    timeout_case(&admin, &plain, "plain-timeout-bypass", bypass).await?;
    let dropped = adapter(port, None, true)?;
    let call = tokio::spawn(async move {
        dropped
            .invoke(
                "query.read",
                json!({"query":"SELECT pg_sleep(40)","limit":1}),
            )
            .await
    });
    wait_active(&admin, true).await?;
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    wait_active(&admin, false).await?;
    println!(
        "{}",
        json!({"case":"dropped-invocation","active_backends":active(&admin).await?})
    );

    let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])?;
    let cert_path = temp.path().join("server.crt");
    let key_path = temp.path().join("server.key");
    std::fs::write(&cert_path, cert.cert.pem())?;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&key_path)?
        .write_all(cert.signing_key.serialize_pem().as_bytes())?;
    docker(&[
        "cp",
        cert_path.to_str().ok_or("invalid certificate path")?,
        &format!("{}:/tmp/review.crt", container.0),
    ])?;
    docker(&[
        "cp",
        key_path.to_str().ok_or("invalid key path")?,
        &format!("{}:/tmp/review.key", container.0),
    ])?;
    docker(&[
        "exec",
        &container.0,
        "chown",
        "postgres:postgres",
        "/tmp/review.key",
        "/tmp/review.crt",
    ])?;
    docker(&["exec", &container.0, "chmod", "600", "/tmp/review.key"])?;
    for query in [
        "ALTER SYSTEM SET ssl_cert_file='/tmp/review.crt'",
        "ALTER SYSTEM SET ssl_key_file='/tmp/review.key'",
        "ALTER SYSTEM SET ssl='on'",
    ] {
        admin.batch_execute(query).await?;
    }
    admin.query_one("SELECT pg_reload_conf()", &[]).await?;
    let tls_deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let ssl: String = admin.query_one("SHOW ssl", &[]).await?.get(0);
        if ssl == "on" {
            break;
        }
        if Instant::now() >= tls_deadline {
            return Err("PostgreSQL did not enable TLS".into());
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let tls = Arc::new(adapter(port, Some(&cert_path), false)?);
    let result = tls
        .invoke(
            "query.read",
            json!({"query":"SELECT ssl FROM pg_stat_ssl WHERE pid=pg_backend_pid()","limit":1}),
        )
        .await?;
    assert_eq!(result["rows"][0][0], "true");
    let untrusted = adapter(port, None, false)?;
    assert!(
        untrusted
            .invoke("query.read", json!({"query":"SELECT 1","limit":1}))
            .await
            .is_err()
    );
    println!(
        "{}",
        json!({"case":"sql-tls","encrypted":true,"untrusted_ca_refused":true})
    );

    // Capture the valid CA at connect time, then replace its file while the query
    // is running. Cancellation must reuse that captured trust, not reread it.
    let running = tls.clone();
    let started = Instant::now();
    let call = tokio::spawn(async move {
        running
            .invoke("query.read", json!({"query":bypass,"limit":10}))
            .await
    });
    wait_active(&admin, true).await?;
    let unrelated = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])?;
    std::fs::write(&cert_path, unrelated.cert.pem())?;
    assert_eq!(call.await?.unwrap_err().code, ErrorCode::Timeout);
    wait_active(&admin, false).await?;
    println!(
        "{}",
        json!({"case":"tls-timeout-bypass-after-ca-file-replacement","elapsed_ms":started.elapsed().as_millis(),"active_backends":active(&admin).await?})
    );
    drop(admin);
    let _ = tokio::time::timeout(Duration::from_secs(2), driver).await;
    drop(container);
    Ok(())
}
