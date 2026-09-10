//! Private approval mechanics. Embedders supply authenticated subject resolution,
//! current policy guards, qualified time and purpose-specific signing custody.
//! Neither a signature nor a spend receipt is permission to call a provider.
mod proof;
mod spend;
mod types;
pub use proof::{Signer, verify};
pub use spend::{SpendReceipt, Store};
pub use types::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Refused,
    Unavailable,
    MetadataUnavailable,
    OutcomeUnknown,
    Replayed,
    Capacity,
}
pub type Result<T> = std::result::Result<T, Failure>;
