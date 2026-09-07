//! Deterministically project and verify the endpoint contract bundles.
use clap::Parser;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Arguments {
    /// Write the reviewed new contract bundles; otherwise check committed bytes.
    #[arg(long)]
    write: bool,
}

fn bytes(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("schema serializes");
    bytes.push(b'\n');
    bytes
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Arguments::parse();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, version, schema) in [
        (
            "connector-event",
            "v0alpha2",
            protocol::event::schema_v2::event_v2_schema(),
        ),
        (
            "connector-endpoint",
            "v0alpha1",
            protocol::endpoint_schema::endpoint_schema(),
        ),
        (
            "connector-operation",
            "v0alpha4",
            protocol::operation::schema_v4::operation_v4_schema(),
        ),
    ] {
        let directory = format!("contracts/{name}/{version}");
        let schema_path = format!("{directory}/{name}.schema.json");
        let schema = bytes(&schema);
        if args.write {
            fs::create_dir_all(root.join(&directory))?;
            fs::write(root.join(&schema_path), &schema)?;
        }
        if fs::read(root.join(&schema_path))? != schema {
            return Err(format!("contract projection drift: {schema_path}").into());
        }
        let mut files = Vec::new();
        for path in [
            "contracts/artifact-bundle.schema.json".to_owned(),
            format!("{directory}/README.md"),
            schema_path,
        ] {
            let contents = fs::read(root.join(&path))?;
            files.push(json!({"path":path,"bytes":contents.len(),"sha256":format!("{:x}", Sha256::digest(&contents))}));
        }
        let manifest = bytes(
            &json!({"bundle":format!("b10x.{name}"),"version":version,"stability":"alpha","files":files}),
        );
        let path = format!("{directory}/bundle.json");
        if args.write {
            fs::write(root.join(&path), &manifest)?;
        }
        if fs::read(root.join(&path))? != manifest {
            return Err(format!("contract manifest drift: {path}").into());
        }
    }
    Ok(())
}
