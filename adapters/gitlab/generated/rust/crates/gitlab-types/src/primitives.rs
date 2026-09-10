// generated from gitlab v1
// model digest ede8787ae4303e8248e5bd6de3675b12866d725713b94852f37db003d4982392
// contract digest 9020464eeb141c4af3f7cd1c704975f1a25aa17da6a44f4b61ff3a5c25400334
// do not edit: regenerate with `ess synthesize`

//! How the specification's primitives are spelled in this workspace.
//!
//! Four map onto types that already mean exactly the same thing: `String` stays `String`,
//! `Boolean` is `bool`, `Integer` is `i64`, `Bytes` is `Vec<u8>`. The four below have no `std`
//! equivalent, and no dependency is taken for them — this workspace builds from exactly its
//! committed bytes. Each is a transparent wrapper over its wire rendering, distinct from `String`
//! and from each other for the same reason the specification's own newtypes are distinct from
//! their representations: a value's meaning is not its shape.

/// An exact decimal, carried as its wire rendering — a decimal string such as `10.50`.
///
/// Never a float: money does not round the way a float does. Equality and order are over the
/// rendering, so `1.5` and `1.50` are different values here; arithmetic is deliberately absent,
/// because what a decimal *does* is behaviour, and behaviour is not synthesised.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal(pub String);

/// An instant, carried as its wire rendering — RFC 3339, such as `2026-01-01T00:00:00Z`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub String);

/// A length of time, carried as its wire rendering — an ISO 8601 duration such as `P30D`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(pub String);

/// A UUID, carried as its canonical textual rendering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uuid(pub String);
