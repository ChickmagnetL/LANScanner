#![forbid(unsafe_code)]

//! Domain logic crate.
//!
//! Phase 1 only establishes the workspace boundary so later phases can add
//! scanner, credential, network, docker, and SSH modules without reshuffling
//! the architecture again.

pub mod credential;
pub mod docker;
pub mod network;
pub mod scanner;
pub mod ssh;
