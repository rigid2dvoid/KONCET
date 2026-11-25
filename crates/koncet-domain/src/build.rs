//! Build stages and immutable simulated image metadata.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::DomainError;
use crate::id::{BuildId, ImageId, RunId};
use crate::project::hex_lower;
use crate::scenario::BuildProfile;
use crate::status::{BuildStageName, BuildStageStatus};
use crate::time::SimTime;

/// A build belongs to exactly one run and has fixed v0.1 stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    pub id: BuildId,
    pub run_id: RunId,
    pub profile: BuildProfile,
    pub stages: Vec<BuildStage>,
    pub image_id: Option<ImageId>,
    pub failure: Option<BuildFailure>,
}

impl Build {
    pub fn new(run_id: RunId, profile: BuildProfile) -> Self {
        let stages = BuildStageName::ORDERED
            .iter()
            .map(|kind| BuildStage::pending(*kind))
            .collect();
        Self {
            id: BuildId::new(),
            run_id,
            profile,
            stages,
            image_id: None,
            failure: None,
        }
    }

    pub fn stage(&self, kind: BuildStageName) -> Option<&BuildStage> {
        self.stages.iter().find(|stage| stage.kind == kind)
    }

    pub fn stage_mut(&mut self, kind: BuildStageName) -> Option<&mut BuildStage> {
        self.stages.iter_mut().find(|stage| stage.kind == kind)
    }

    /// Starts a pending stage. Returns a conflict for any other state.
    pub fn start_stage(&mut self, kind: BuildStageName, at: SimTime) -> Result<(), DomainError> {
        let stage = self
            .stage_mut(kind)
            .ok_or_else(|| DomainError::internal("unknown_build_stage", "stage is not in v1"))?;
        if stage.status != BuildStageStatus::Pending {
            return Err(DomainError::conflict(
                "invalid_stage_transition",
                format!("cannot start {} while it is {}", kind, stage.status),
            ));
        }
        stage.status = BuildStageStatus::Running;
        stage.started_at = Some(at);
        Ok(())
    }

    /// Completes a running stage, marking later pending stages skipped on failure.
    pub fn finish_stage(
        &mut self,
        kind: BuildStageName,
        at: SimTime,
        failure: Option<BuildFailure>,
    ) -> Result<(), DomainError> {
        let stage = self
            .stage_mut(kind)
            .ok_or_else(|| DomainError::internal("unknown_build_stage", "stage is not in v1"))?;
        if stage.status != BuildStageStatus::Running {
            return Err(DomainError::conflict(
                "invalid_stage_transition",
                format!("cannot finish {} while it is {}", kind, stage.status),
            ));
        }
        stage.finished_at = Some(at);
        stage.actual_ms = Some(
            (at.as_millis() - stage.started_at.map(SimTime::as_millis).unwrap_or(0)).max(0) as u64,
        );
        match failure {
            Some(failure) => {
                stage.status = BuildStageStatus::Failed;
                stage.failure = Some(failure.clone());
                if self.failure.is_none() {
                    self.failure = Some(failure);
                }
                let mut later = false;
                for stage in &mut self.stages {
                    if stage.kind == kind {
                        later = true;
                        continue;
                    }
                    if later && stage.status == BuildStageStatus::Pending {
                        stage.status = BuildStageStatus::Skipped;
                    }
                }
            }
            None => stage.status = BuildStageStatus::Succeeded,
        }
        Ok(())
    }

    pub fn succeeded(&self) -> bool {
        self.failure.is_none()
            && self
                .stages
                .iter()
                .all(|stage| stage.status == BuildStageStatus::Succeeded)
    }

    pub fn failed(&self) -> bool {
        self.failure.is_some()
    }

    pub fn completed(&self) -> bool {
        self.succeeded() || self.failed()
    }

    pub fn current_stage(&self) -> Option<&BuildStage> {
        self.stages
            .iter()
            .find(|stage| stage.status == BuildStageStatus::Running)
    }

    pub fn next_pending(&self) -> Option<&BuildStage> {
        self.stages
            .iter()
            .find(|stage| stage.status == BuildStageStatus::Pending)
    }

    pub fn skipped_count(&self) -> usize {
        self.stages
            .iter()
            .filter(|stage| stage.status == BuildStageStatus::Skipped)
            .count()
    }
}

/// A single build stage with deterministic timing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildStage {
    pub kind: BuildStageName,
    pub status: BuildStageStatus,
    pub planned_ms: u64,
    pub started_at: Option<SimTime>,
    pub finished_at: Option<SimTime>,
    pub actual_ms: Option<u64>,
    pub failure: Option<BuildFailure>,
}

impl BuildStage {
    pub fn pending(kind: BuildStageName) -> Self {
        Self {
            kind,
            status: BuildStageStatus::Pending,
            planned_ms: kind.default_duration_ms(),
            started_at: None,
            finished_at: None,
            actual_ms: None,
            failure: None,
        }
    }

    pub fn with_planned_ms(mut self, planned_ms: u64) -> Self {
        self.planned_ms = planned_ms.max(1);
        self
    }
}

/// A stable build failure code plus an explanatory message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildFailure {
    pub stage: BuildStageName,
    pub code: String,
    pub message: String,
    pub at_percent: u8,
}

impl BuildFailure {
    pub fn new(
        stage: BuildStageName,
        code: impl Into<String>,
        message: impl Into<String>,
        at_percent: u8,
    ) -> Self {
        Self {
            stage,
            code: code.into(),
            message: message.into(),
            at_percent,
        }
    }
}

/// An ordered, immutable image layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageLayer {
    pub name: String,
    pub digest: String,
    pub size_bytes: u64,
}

impl ImageLayer {
    pub fn new(name: impl Into<String>, size_bytes: u64, source_hash: &str) -> Self {
        let name = name.into();
        let digest = sha256_text(&format!(
            "koncet/layer/v1|{name}|{size_bytes}|{source_hash}"
        ));
        Self {
            name,
            digest: format!("sha256:{digest}"),
            size_bytes,
        }
    }
}

/// Immutable simulated image. It is never pushed, pulled, or resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatedImage {
    pub id: ImageId,
    pub run_id: RunId,
    pub repository: String,
    pub tag: String,
    pub digest: String,
    pub layers: Vec<ImageLayer>,
    pub size_bytes: u64,
    pub profile: BuildProfile,
    pub source_hash: String,
    pub engine_version: String,
    pub created_at: SimTime,
}

impl SimulatedImage {
    /// Builds deterministic metadata from the snapshot hash and profile.
    ///
    /// The digest covers the canonical manifest; nothing is read from disk.
    pub fn create(
        run_id: RunId,
        project_id: crate::id::ProjectId,
        profile: BuildProfile,
        source_hash: &str,
        source_bytes: u64,
        engine_version: &str,
        created_at: SimTime,
    ) -> Self {
        let repository = format!("koncet/{}/app", project_id.short());
        let tag = format!("run-{}", run_id.short());
        let base_size = profile.baseline_image_bytes();
        let layers = vec![
            ImageLayer::new(format!("base:{}", profile.as_str()), base_size, source_hash),
            ImageLayer::new("app:sources", (source_bytes + 4096).max(4096), source_hash),
        ];
        let size_bytes = layers.iter().map(|layer| layer.size_bytes).sum();
        let mut image = Self {
            id: ImageId::new(),
            run_id,
            repository,
            tag,
            digest: String::new(),
            layers,
            size_bytes,
            profile,
            source_hash: source_hash.to_string(),
            engine_version: engine_version.to_string(),
            created_at,
        };
        image.digest = image.compute_digest();
        image
    }

    /// Canonical manifest, sorted keys, no wall-clock values.
    pub fn manifest(&self) -> serde_json::Value {
        serde_json::json!({
            "engine_version": self.engine_version,
            "layers": self.layers.iter().map(|layer| serde_json::json!({
                "digest": layer.digest,
                "name": layer.name,
                "size_bytes": layer.size_bytes,
            })).collect::<Vec<_>>(),
            "profile": self.profile.as_str(),
            "schema_version": 1,
            "source_hash": self.source_hash,
        })
    }

    pub fn compute_digest(&self) -> String {
        let manifest = serde_json::to_vec(&self.manifest()).expect("manifest is serialisable");
        format!("sha256:{}", hex_lower(&Sha256::digest(manifest)))
    }

    pub fn reference(&self) -> String {
        format!("{}:{}", self.repository, self.tag)
    }

    pub fn display_size(&self) -> String {
        format_bytes(self.size_bytes)
    }

    /// Most significant byte first, lower-case repository validation.
    pub fn validate_repository(repository: &str) -> Result<(), DomainError> {
        if repository.is_empty() || repository.len() > 255 {
            return Err(DomainError::validation(
                "invalid_format",
                "/image/repository",
                "image repository must be between 1 and 255 characters",
            ));
        }
        let first = repository.chars().next().unwrap_or(' ');
        if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
            return Err(DomainError::validation(
                "invalid_format",
                "/image/repository",
                "image repository must start with a lowercase letter or digit",
            ));
        }
        if !repository.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '/' | '-')
        }) {
            return Err(DomainError::validation(
                "invalid_format",
                "/image/repository",
                "image repository may contain lowercase letters, digits, '.', '_', '/', and '-'",
            ));
        }
        Ok(())
    }

    pub fn validate_tag(tag: &str) -> Result<(), DomainError> {
        let mut chars = tag.chars();
        let Some(first) = chars.next() else {
            return Err(DomainError::validation(
                "required",
                "/image/tag",
                "image tag is required",
            ));
        };
        let valid_first = first.is_ascii_alphanumeric() || first == '_';
        let valid_rest = chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
        if !valid_first || !valid_rest || tag.len() > 128 {
            return Err(DomainError::validation(
                "invalid_format",
                "/image/tag",
                "image tag must match [A-Za-z0-9_][A-Za-z0-9._-]{0,127}",
            ));
        }
        Ok(())
    }
}

impl fmt::Display for SimulatedImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reference())
    }
}

fn sha256_text(value: &str) -> String {
    hex_lower(&Sha256::digest(value.as_bytes()))
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ProjectId;

    #[test]
    fn stage_failure_skips_later_stages() {
        let mut build = Build::new(RunId::new(), BuildProfile::StaticWeb);
        build
            .start_stage(BuildStageName::Validate, SimTime::ZERO)
            .unwrap();
        build
            .finish_stage(BuildStageName::Validate, SimTime::from_secs(2), None)
            .unwrap();
        build
            .start_stage(BuildStageName::Prepare, SimTime::from_secs(2))
            .unwrap();
        build
            .finish_stage(
                BuildStageName::Prepare,
                SimTime::from_secs(7),
                Some(BuildFailure::new(
                    BuildStageName::Prepare,
                    "injected_build_failure",
                    "boom",
                    50,
                )),
            )
            .unwrap();
        assert!(build.failed());
        assert_eq!(
            build.stage(BuildStageName::Build).unwrap().status,
            BuildStageStatus::Skipped
        );
        assert_eq!(
            build.stage(BuildStageName::Package).unwrap().status,
            BuildStageStatus::Skipped
        );
        assert_eq!(build.skipped_count(), 2);
    }

    #[test]
    fn image_digest_is_stable_and_content_addressed() {
        let run = RunId::new();
        let project = ProjectId::new();
        let a = SimulatedImage::create(
            run,
            project,
            BuildProfile::StaticWeb,
            "abc",
            100,
            "0.1.0",
            SimTime::from_secs(10),
        );
        let b = SimulatedImage::create(
            run,
            project,
            BuildProfile::StaticWeb,
            "abc",
            100,
            "0.1.0",
            SimTime::from_secs(10),
        );
        assert_eq!(a.digest, b.digest);
        assert!(a.digest.starts_with("sha256:"));
        assert_eq!(a.digest.len(), 71);
        assert!(a.reference().contains(&run.short()));
    }

    #[test]
    fn repository_and_tag_validation_rejects_uppercase() {
        assert!(SimulatedImage::validate_repository("koncet/abc/app").is_ok());
        assert!(SimulatedImage::validate_repository("Koncet/app").is_err());
        assert!(SimulatedImage::validate_tag("run-abc123").is_ok());
        assert!(SimulatedImage::validate_tag("bad tag").is_err());
    }

    #[test]
    fn byte_formatting_is_human_readable() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(18 * 1024 * 1024), "18.0 MiB");
    }
}
