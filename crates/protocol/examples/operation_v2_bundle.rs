//! Reproduce the additive operation bundle without touching a frozen predecessor.
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
use std::{error::Error, fs, path::Path};

#[derive(Parser)]
#[command(about = "Reproduce or check ConnectorOperation v0alpha2 bundle artifacts")]
struct Args {
    #[command(subcommand)]
    mode: Mode,
}
#[derive(Subcommand)]
enum Mode {
    Check,
    Write,
}
#[derive(Deserialize)]
struct Vectors {
    cases: Vec<Vector>,
}
#[derive(Deserialize)]
struct Vector {
    name: String,
    kind: String,
    valid: bool,
    schema_valid: bool,
    frame: Value,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let prefix = "contracts/connector-operation/v0alpha2";
    let schema = protocol::operation::schema::operation_v2_schema();
    let schema_bytes = pretty(&schema)?;
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)?;
    let vectors: Vectors =
        serde_json::from_slice(&fs::read(root.join(prefix).join("vectors.json"))?)?;
    for vector in &vectors.cases {
        let schema_valid = validator.is_valid(&vector.frame);
        let rust_valid = match vector.kind.as_str() {
            "request" => serde_json::from_value::<protocol::operation::wire::RequestEnvelope>(
                vector.frame.clone(),
            )
            .is_ok_and(|frame| frame.validate().is_ok()),
            "response" => serde_json::from_value::<protocol::operation::wire::ResponseEnvelope>(
                vector.frame.clone(),
            )
            .is_ok_and(|frame| frame.validate().is_ok()),
            _ => return Err(format!("unknown vector kind: {}", vector.kind).into()),
        };
        if rust_valid != vector.valid || schema_valid != vector.schema_valid {
            return Err(format!(
                "vector {} disagrees: Rust={rust_valid}, schema={schema_valid}",
                vector.name
            )
            .into());
        }
    }
    let schema_path = format!("{prefix}/connector-operation.schema.json");
    let paths = [
        "contracts/artifact-bundle.schema.json".to_owned(),
        format!("{prefix}/README.md"),
        schema_path.clone(),
        format!("{prefix}/vectors.schema.json"),
        format!("{prefix}/vectors.json"),
    ];
    let mut files = Vec::new();
    for path in paths {
        let bytes = if path == schema_path {
            schema_bytes.clone()
        } else {
            fs::read(root.join(&path))?
        };
        files.push(json!({"path":path, "bytes":bytes.len(), "sha256":format!("{:x}", Sha256::digest(&bytes))}));
    }
    let manifest = pretty(
        &json!({"bundle":"b10x.connector-operation", "version":"v0alpha2",
        "stability":"alpha", "files":files}),
    )?;
    for (path, bytes) in [
        (schema_path, schema_bytes),
        (format!("{prefix}/bundle.json"), manifest),
    ] {
        match args.mode {
            Mode::Check => {
                if fs::read(root.join(&path))? != bytes {
                    return Err(format!("artifact drift: {path}").into());
                }
            }
            Mode::Write => fs::write(root.join(&path), bytes)?,
        }
        println!(
            "{path}: {}",
            match args.mode {
                Mode::Check => "matches",
                Mode::Write => "written",
            }
        );
    }
    println!(
        "{} vectors checked by Rust and Draft 2020-12",
        vectors.cases.len()
    );
    Ok(())
}
fn pretty(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
