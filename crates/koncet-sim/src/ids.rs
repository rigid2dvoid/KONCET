//! Deterministic child identifiers derived from the run identifier.
//!
//! The application generates the run UUIDv7. Child entities must be stable across
//! replays of the same inputs, so their UUIDv7-shaped values are derived from the
//! run UUID plus a stable label instead of the wall clock.

use koncet_domain::{BuildId, ContainerId, DeploymentId, ImageId, RunId};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn derive(run_id: RunId, label: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"koncet/id/v1|");
    hasher.update(run_id.as_uuid().as_bytes());
    hasher.update(b"|");
    hasher.update(label.as_bytes());
    let digest = hasher.finalize();

    let mut bytes = [0u8; 16];
    // Keep the run's UUIDv7 timestamp prefix so the value remains time-ordered
    // relative to its owner, then use hash bytes for the remaining fields.
    bytes[..6].copy_from_slice(&run_id.as_uuid().as_bytes()[..6]);
    bytes[6..].copy_from_slice(&digest[..10]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(crate) fn build_id(run_id: RunId) -> BuildId {
    BuildId::from_uuid(derive(run_id, "build/0"))
}

pub(crate) fn image_id(run_id: RunId) -> ImageId {
    ImageId::from_uuid(derive(run_id, "image/0"))
}

pub(crate) fn container_id(run_id: RunId) -> ContainerId {
    ContainerId::from_uuid(derive(run_id, "container/0"))
}

pub(crate) fn deployment_id(run_id: RunId) -> DeploymentId {
    DeploymentId::from_uuid(derive(run_id, "deployment/0"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_ids_are_stable_and_version_seven() {
        let run = RunId::new();
        let first = build_id(run);
        let second = build_id(run);
        assert_eq!(first, second);
        assert_eq!(first.as_uuid().get_version_num(), 7);
        assert_ne!(build_id(run).as_uuid(), image_id(run).as_uuid());
    }
}
