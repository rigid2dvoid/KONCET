//! Flutter Rust Bridge surface for KONCET.
//!
//! The bridge is a thin adapter over `koncet-app`: it validates identifiers,
//! maps domain types to versioned DTOs, serialises commands through one
//! composition root, and fan-outs committed run updates to Dart streams.
//!
//! The concrete error type must stay in `Result` so flutter_rust_bridge can
//! translate it into a Dart exception; boxing it would make it an opaque handle.
#![allow(clippy::result_large_err)]

pub mod api;
pub mod dto;
mod error;
mod state;

mod frb_generated;

pub use error::BridgeError;
