//! The local host's custody store: the platform binding of `connectors-custody`
//! (the Secret Service on Linux), or the read-only mounted directory an owner
//! configuration explicitly selects, behind the host's coordinator in
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

/// Passive state of the mounted directory a configuration selects: available
/// only when it passes its checks on a read-only mount. Reads no item.
pub fn inspect_mounted(directory: &Path) -> State {
    #[cfg(target_os = "linux")]
    {
        connectors_custody::mounted::inspect(directory)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = directory;
        State::Unavailable
    }
}

#[cfg(target_os = "linux")]
pub use connectors_custody::secret_service::{CustodyItem, Orphans, Removal};

/// Delete every custody item carrying `authority`; each failed delete is named
/// in the result.
#[cfg(target_os = "linux")]
pub fn custody_remove_authority(
    socket: Option<&Path>,
    authority: uuid::Uuid,
) -> Result<Removal, connectors_custody::Failure> {
    connectors_custody::secret_service::remove_authority_at(socket, authority)
}

/// The custody items on the Secret Service that no store of `known` names;
/// they are deleted only when `remove` is true. Reads attributes, never a value.
#[cfg(target_os = "linux")]
pub fn custody_orphans(
    socket: Option<&Path>,
    known: &std::collections::HashSet<uuid::Uuid>,
    remove: bool,
) -> Result<Orphans, connectors_custody::Failure> {
    connectors_custody::secret_service::orphans_at(socket, known, remove)
}
