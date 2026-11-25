//! Deterministic, headless simulation engine.
//!
//! The engine performs no I/O and never reads the clock, the filesystem, the
//! network, or the environment. Callers provide an immutable source snapshot,
//! a validated scenario, a seed, and a fixed run identifier. Advancing the engine
//! is a pure function of those inputs and the recorded command trace.

mod engine;
mod ids;
mod rng;

pub use engine::{
    SimulationEngine, SimulationInputs, SimulationOutput, SimulationSnapshot, TICK_MS,
};
