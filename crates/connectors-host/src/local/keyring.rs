//! The local host's custody store: the platform binding of `connectors-custody`
//! (the Secret Service on Linux), behind the host's coordinator in
//! [`custody`]. Inspection never activates a service, unlocks a collection,
//! prompts, or reads credentials.
pub mod custody;

use std::path::Path;

pub use connectors_custody::{CustodyBackend, PLATFORM, State};

pub fn inspect() -> State {
    inspect_at(None)
}
pub fn inspect_at(socket: Option<&Path>) -> State {
    connectors_custody::inspect(socket)
}
