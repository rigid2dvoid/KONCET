//! Domain-separated deterministic random streams.

use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};

pub(crate) struct RngStreams {
    runtime: ChaCha8Rng,
    monitoring: ChaCha8Rng,
}

impl RngStreams {
    pub(crate) fn from_seed(seed: u64) -> Self {
        Self {
            runtime: stream(seed, "runtime"),
            monitoring: stream(seed, "monitoring"),
        }
    }

    pub(crate) fn next_pid(&mut self) -> u32 {
        self.runtime.gen_range(1_000..=60_000)
    }

    pub(crate) fn health_latency_ms(&mut self) -> u32 {
        self.runtime.gen_range(1..=9)
    }

    /// CPU usage as a rounded percentage of the declared limit.
    pub(crate) fn cpu_percent(&mut self) -> u32 {
        self.monitoring.gen_range(35..=62)
    }

    /// Memory usage as a rounded percentage of the declared limit.
    pub(crate) fn memory_percent(&mut self) -> u32 {
        self.monitoring.gen_range(40..=73)
    }

    /// Inbound application traffic in KiB/s.
    pub(crate) fn network_rx_kib(&mut self) -> u32 {
        self.monitoring.gen_range(18..=420)
    }

    /// Outbound application traffic in KiB/s.
    pub(crate) fn network_tx_kib(&mut self) -> u32 {
        self.monitoring.gen_range(9..=260)
    }
}

fn stream(seed: u64, label: &str) -> ChaCha8Rng {
    let mut hasher = Sha256::new();
    hasher.update(b"koncet/rng/v1|");
    hasher.update(seed.to_le_bytes());
    hasher.update(b"|");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    ChaCha8Rng::from_seed(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_are_reproducible_and_independent() {
        let mut a = RngStreams::from_seed(42);
        let mut b = RngStreams::from_seed(42);
        for _ in 0..10 {
            assert_eq!(a.next_pid(), b.next_pid());
            assert_eq!(a.cpu_percent(), b.cpu_percent());
        }
    }
}
