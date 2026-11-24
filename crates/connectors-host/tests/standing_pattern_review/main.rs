//! The review cases of story:standing-grant-binds-a-pattern-or-a-value-set
//! in one test binary: one module for each review round and reviewer, as
//! each reviewer wrote it, and `support` with the counting allocator they
//! share.
#![cfg(unix)]

mod adversary_r1;
mod adversary_r2;
mod adversary_r3;
mod security_r1;
mod security_r2;
mod security_r3;
mod support;
