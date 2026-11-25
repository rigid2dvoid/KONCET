//! Wall-clock access behind a trait so application tests are deterministic.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use koncet_domain::TimestampUtc;

/// Supplies wall-clock timestamps. The simulation itself never reads this.
pub trait Clock: fmt::Debug + Send + Sync {
    fn now(&self) -> TimestampUtc;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> TimestampUtc {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        TimestampUtc::from_millis(millis)
    }
}
