//! Time and duration newtypes.
//!
//! Simulation time is virtual and monotonic; wall time is metadata only. The two
//! are deliberately different types so they cannot be mixed accidentally.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Monotonic simulated milliseconds since run start.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct SimTime(i64);

impl SimTime {
    pub const ZERO: Self = Self(0);

    pub const fn from_millis(value: i64) -> Self {
        Self(value)
    }

    pub const fn from_secs(value: i64) -> Self {
        Self(value.saturating_mul(1_000))
    }

    pub const fn as_millis(self) -> i64 {
        self.0
    }

    pub const fn as_secs(self) -> i64 {
        self.0 / 1_000
    }

    pub const fn saturating_add(self, millis: i64) -> Self {
        Self(self.0.saturating_add(millis))
    }

    pub const fn saturating_sub(self, millis: i64) -> Self {
        Self(self.0.saturating_sub(millis))
    }

    /// Rounds down to a whole number of seconds, for display.
    pub const fn whole_secs(self) -> i64 {
        self.0.div_euclid(1_000)
    }
}

impl fmt::Display for SimTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.0;
        let sign = if total < 0 { "-" } else { "" };
        let total = total.abs();
        let seconds = total / 1_000;
        let millis = total % 1_000;
        write!(f, "{sign}{seconds}.{millis:03}s")
    }
}

/// UTC wall-clock timestamp in epoch milliseconds.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct TimestampUtc(i64);

impl TimestampUtc {
    pub const fn from_millis(value: i64) -> Self {
        Self(value)
    }

    pub const fn as_millis(self) -> i64 {
        self.0
    }
}

impl fmt::Display for TimestampUtc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A duration expressed in simulated milliseconds.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct DurationMs(u64);

impl DurationMs {
    pub const ZERO: Self = Self(0);

    pub const fn from_millis(value: u64) -> Self {
        Self(value)
    }

    pub const fn from_secs(value: u64) -> Self {
        Self(value.saturating_mul(1_000))
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_time_is_displayed_with_millisecond_precision() {
        assert_eq!(SimTime::from_millis(62_345).to_string(), "62.345s");
        assert_eq!(SimTime::from_secs(2).to_string(), "2.000s");
    }

    #[test]
    fn saturation_never_panics() {
        assert_eq!(
            SimTime::from_millis(i64::MAX).saturating_add(1).as_millis(),
            i64::MAX
        );
        assert_eq!(DurationMs::from_secs(u64::MAX).as_millis(), u64::MAX);
    }
}
