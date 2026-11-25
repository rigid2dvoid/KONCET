//! Pure domain types and invariants for KONCET.
//!
//! This crate performs no I/O, reads no clock, and contains no simulation loop.

pub mod build;
pub mod error;
pub mod event;
pub mod id;
pub mod project;
pub mod run;
pub mod scenario;
pub mod status;
pub mod time;

pub use build::*;
pub use error::*;
pub use event::*;
pub use id::*;
pub use project::*;
pub use run::*;
pub use scenario::*;
pub use status::*;
pub use time::*;

/// Version of the simulation semantics. Kept equal to the product version in v0.1.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The reserved suffix for all simulated hostnames.
pub const SIMULATED_HOST_SUFFIX: &str = ".koncet.local";
