use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Validate a Connectors adapter specification and project its descriptor")]
struct Args {
    #[arg(long)]
    specification: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    check: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let descriptor = connectors_spec::compile(&std::fs::read(&args.specification)?)?;
    let bytes = connectors_spec::descriptor_bytes(&descriptor)?;
    if args.check {
        if std::fs::read(&args.output)? != bytes {
            return Err("generated descriptor drift".into());
        }
    } else {
        if let Some(parent) = args.output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&args.output, bytes)?;
    }
    println!(
        "{}: {} implemented-binding obligations; descriptor {}",
        descriptor.adapter,
        descriptor.operations.len(),
        if args.check { "matches" } else { "written" }
    );
    Ok(())
}
