//! The first of two executables that check that the macOS login keychain lets
//! another binary read an item this one created, with the access the custody
//! binding creates items with.
//!
//! `keychain_probe_writer write` stores a fixed test value under a fresh
//! random scope and prints `<authority> <allocation> <version>`;
//! `keychain_probe_reader` reads that version from another executable;
//! `keychain_probe_writer delete <authority> <allocation> <version>` removes
//! the item if the reader could not.

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    probe::main()
}

#[cfg(not(target_os = "macos"))]
fn main() -> std::process::ExitCode {
    eprintln!("keychain_probe_writer: the keychain probe runs on macOS only");
    std::process::ExitCode::from(2)
}

#[cfg(target_os = "macos")]
mod probe {
    use clap::Parser;
    use connectors_custody::{Backend, Material, Scope, Version, keychain::Keychain};
    use std::{
        process::ExitCode,
        time::{Duration, Instant},
    };
    use uuid::Uuid;

    /// The value the probe stores; it is a test value, never a credential.
    const VALUE: &[u8] = b"connectors-custody-keychain-probe";

    #[derive(Parser)]
    #[command(
        name = "keychain_probe_writer",
        about = "Store or delete the keychain probe item"
    )]
    enum Command {
        /// Store the probe value under a fresh random scope and print its ids.
        Write,
        /// Delete the probe item this executable stored.
        Delete {
            authority: Uuid,
            allocation: Uuid,
            version: Uuid,
        },
    }

    fn until() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }

    pub fn main() -> ExitCode {
        match Command::parse() {
            Command::Write => {
                let (authority, allocation, id) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
                let scope = Scope::new(authority, allocation).expect("random ids are not nil");
                let version = Version::new(scope, id).expect("a random id is not nil");
                let written = Keychain::open(scope).and_then(|keychain| {
                    keychain.write_new(version, &Material::from_slice(VALUE), until(), || Ok(()))
                });
                match written {
                    Ok(()) => {
                        println!("{authority} {allocation} {id}");
                        ExitCode::SUCCESS
                    }
                    Err(failure) => {
                        eprintln!("keychain_probe_writer: write: {failure:?}");
                        ExitCode::FAILURE
                    }
                }
            }
            Command::Delete {
                authority,
                allocation,
                version,
            } => {
                let deleted = Scope::new(authority, allocation)
                    .and_then(|scope| Ok((scope, Version::new(scope, version)?)))
                    .and_then(|(scope, version)| {
                        Keychain::open(scope)?.delete(version, until(), || Ok(()))
                    });
                match deleted {
                    Ok(()) => {
                        println!("writer delete: ok");
                        ExitCode::SUCCESS
                    }
                    Err(failure) => {
                        eprintln!("keychain_probe_writer: delete: {failure:?}");
                        ExitCode::FAILURE
                    }
                }
            }
        }
    }
}
