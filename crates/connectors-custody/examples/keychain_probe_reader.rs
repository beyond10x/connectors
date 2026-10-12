//! The second of two executables that observe how the macOS login keychain
//! treats an item `keychain_probe_writer` created when this other binary
//! reads it, with the access the custody binding creates items with today.
//!
//! `keychain_probe_reader <authority> <allocation> <version>` reads that
//! version with user interaction disallowed and prints `read: ok`, or the
//! `Failure` variant and the keychain's status code; then it deletes the item
//! if it can and prints that outcome. Every observation exits 0: the outcome
//! is the result. A crash exits otherwise.

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    probe::main()
}

#[cfg(not(target_os = "macos"))]
fn main() -> std::process::ExitCode {
    eprintln!("keychain_probe_reader: the keychain probe runs on macOS only");
    std::process::ExitCode::from(2)
}

#[cfg(target_os = "macos")]
mod probe {
    use clap::Parser;
    use connectors_custody::{Backend, Scope, Version, keychain::Keychain};
    use std::{
        process::ExitCode,
        time::{Duration, Instant},
    };
    use uuid::Uuid;

    /// The value `keychain_probe_writer` stores.
    const VALUE: &[u8] = b"connectors-custody-keychain-probe";

    #[derive(Parser)]
    #[command(
        name = "keychain_probe_reader",
        about = "Read and delete the keychain probe item another executable stored"
    )]
    struct Arguments {
        authority: Uuid,
        allocation: Uuid,
        version: Uuid,
    }

    pub fn main() -> ExitCode {
        let arguments = Arguments::parse();
        let Ok(scope) = Scope::new(arguments.authority, arguments.allocation) else {
            eprintln!("keychain_probe_reader: nil scope ids");
            return ExitCode::from(2);
        };
        let Ok(version) = Version::new(scope, arguments.version) else {
            eprintln!("keychain_probe_reader: nil version id");
            return ExitCode::from(2);
        };
        let keychain = match Keychain::open(scope) {
            Ok(keychain) => keychain,
            Err(failure) => {
                println!("read: {failure:?} (the login keychain did not open)");
                return ExitCode::SUCCESS;
            }
        };
        match keychain.read_observed(version) {
            (Ok(material), _) if material.ct_eq(VALUE) => println!("read: ok"),
            (Ok(_), _) => println!("read: ok, but not the value the writer stored"),
            (Err(failure), Some(status)) => println!("read: {failure:?}, status {status}"),
            (Err(failure), None) => println!("read: {failure:?}, no keychain status"),
        }
        let until = Instant::now() + Duration::from_secs(10);
        match keychain.delete(version, until, || Ok(())) {
            Ok(()) => println!("reader delete: ok"),
            Err(failure) => println!("reader delete: {failure:?}"),
        }
        ExitCode::SUCCESS
    }
}
