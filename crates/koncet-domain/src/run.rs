//! Run aggregate, containers, processes, deployments, and recorded commands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::id::{
    ContainerId, DeploymentId, ImageId, ProjectId, RunId, ScenarioId, SourceSnapshotId,
};
use crate::scenario::{HealthCheck, RestartPolicy, ServiceConfig};
use crate::status::{
    ContainerState, DeploymentStatus, FailureReason, HealthStatus, RunResult, RunStatus,
};
use crate::time::{SimTime, TimestampUtc};

/// Wall-clock and version metadata for one execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationRun {
    pub id: RunId,
    pub project_id: ProjectId,
    pub scenario_id: ScenarioId,
    pub scenario_name: String,
    pub seed: u64,
    pub source_snapshot_id: SourceSnapshotId,
    pub scenario_schema_version: u32,
    pub scenario_json: String,
    pub status: RunStatus,
    pub result: RunResult,
    pub created_at: TimestampUtc,
    pub started_at: Option<TimestampUtc>,
    pub ended_at: Option<TimestampUtc>,
    pub sim_duration_ms: u64,
    pub last_event_seq: u64,
    pub summary: Option<RunSummary>,
    pub app_version: String,
    pub engine_version: String,
}

impl SimulationRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_id: ProjectId,
        scenario_id: ScenarioId,
        scenario_name: impl Into<String>,
        seed: u64,
        source_snapshot_id: SourceSnapshotId,
        scenario_schema_version: u32,
        scenario_json: impl Into<String>,
        created_at: TimestampUtc,
        app_version: impl Into<String>,
        engine_version: impl Into<String>,
    ) -> Self {
        let now = created_at;
        Self {
            id: RunId::new(),
            project_id,
            scenario_id,
            scenario_name: scenario_name.into(),
            seed,
            source_snapshot_id,
            scenario_schema_version,
            scenario_json: scenario_json.into(),
            status: RunStatus::Running,
            result: RunResult::Unknown,
            created_at: now,
            started_at: Some(now),
            ended_at: None,
            sim_duration_ms: 0,
            last_event_seq: 0,
            summary: None,
            app_version: app_version.into(),
            engine_version: engine_version.into(),
        }
    }

    pub const fn is_active(&self) -> bool {
        self.status.is_active()
    }

    pub fn pause(&mut self) -> Result<(), DomainError> {
        match self.status {
            RunStatus::Running => {
                self.status = RunStatus::Paused;
                Ok(())
            }
            RunStatus::Paused => Ok(()),
            status => Err(DomainError::conflict(
                "invalid_run_transition",
                format!("cannot pause a {status} run"),
            )),
        }
    }

    pub fn resume(&mut self) -> Result<(), DomainError> {
        match self.status {
            RunStatus::Paused => {
                self.status = RunStatus::Running;
                Ok(())
            }
            RunStatus::Running => Ok(()),
            status => Err(DomainError::conflict(
                "invalid_run_transition",
                format!("cannot resume a {status} run"),
            )),
        }
    }

    /// Finalises a run exactly once. Terminal runs are immutable.
    pub fn finalise(
        &mut self,
        status: RunStatus,
        result: RunResult,
        summary: RunSummary,
        ended_at: TimestampUtc,
    ) -> Result<(), DomainError> {
        if !status.is_terminal() {
            return Err(DomainError::internal(
                "non_terminal_finalise",
                "finalise requires a terminal status",
            ));
        }
        if self.status.is_terminal() {
            return Err(DomainError::conflict(
                "run_already_finalised",
                "the run is already finalised",
            ));
        }
        if summary.status != status || summary.result != result {
            return Err(DomainError::internal(
                "summary_mismatch",
                "summary must agree with the final run status and result",
            ));
        }
        self.status = status;
        self.result = result;
        self.ended_at = Some(ended_at);
        self.sim_duration_ms = summary.sim_duration_ms;
        self.summary = Some(summary);
        Ok(())
    }

    /// Records the next event sequence number, rejecting gaps and duplicates.
    pub fn record_event_seq(&mut self, seq: u64) -> Result<(), DomainError> {
        let expected = self.last_event_seq + 1;
        if seq != expected {
            return Err(DomainError::internal(
                "event_sequence_gap",
                format!("expected event sequence {expected}, got {seq}"),
            ));
        }
        self.last_event_seq = seq;
        Ok(())
    }

    pub fn elapsed_ms(&self, sim_time: SimTime) -> u64 {
        sim_time.as_millis().max(0) as u64
    }
}

/// Immutable final summary for history lists and run detail headers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    pub status: RunStatus,
    pub result: RunResult,
    pub primary_reason: Option<FailureReason>,
    pub sim_duration_ms: u64,
    pub event_count: u64,
    pub log_count: u64,
    pub warning_count: u64,
    pub error_count: u64,
    pub build_stage: Option<crate::status::BuildStageName>,
    pub container_restart_count: u32,
    pub deployment_status: Option<DeploymentStatus>,
}

impl Default for RunSummary {
    fn default() -> Self {
        Self {
            status: RunStatus::Running,
            result: RunResult::Unknown,
            primary_reason: None,
            sim_duration_ms: 0,
            event_count: 0,
            log_count: 0,
            warning_count: 0,
            error_count: 0,
            build_stage: None,
            container_restart_count: 0,
            deployment_status: None,
        }
    }
}

/// Controls accepted by the engine. Every affecting command is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum RunCommand {
    Pause,
    Resume,
    SetSpeed { speed: u8 },
    Stop,
    Cancel,
    RestartContainer,
    CheckConnectivity,
}

impl RunCommand {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::SetSpeed { .. } => "set_speed",
            Self::Stop => "stop",
            Self::Cancel => "cancel",
            Self::RestartContainer => "restart_container",
            Self::CheckConnectivity => "check_connectivity",
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if let Self::SetSpeed { speed } = self {
            if !matches!(speed, 1 | 2 | 4 | 8) {
                return Err(DomainError::validation(
                    "out_of_range",
                    "/command/speed",
                    "speed must be one of 1, 2, 4, or 8",
                ));
            }
        }
        Ok(())
    }
}

/// A command stamped with the simulation tick at which it was accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedCommand {
    pub seq: u64,
    pub at: SimTime,
    pub command: RunCommand,
}

/// Container runtime configuration derived from the scenario service section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub cpu_millicores: u32,
    pub memory_mb: u32,
    pub env: BTreeMap<String, String>,
    pub port: u32,
    pub restart_policy: RestartPolicy,
    pub health_check: HealthCheck,
}

impl ContainerConfig {
    pub fn from_service(service: &ServiceConfig) -> Self {
        Self {
            cpu_millicores: service.cpu_millicores,
            memory_mb: service.memory_mb,
            env: service.env.clone(),
            port: service.port,
            restart_policy: service.restart_policy.clone(),
            health_check: service.health_check.clone(),
        }
    }
}

/// Supervised process inside one container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Process {
    pub container_id: ContainerId,
    pub state: ProcessState,
    /// One-based attempt counter; increases with every start.
    pub attempt: u32,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub started_at: Option<SimTime>,
    pub exited_at: Option<SimTime>,
    pub restart_times: Vec<SimTime>,
}

impl Process {
    pub fn new(container_id: ContainerId) -> Self {
        Self {
            container_id,
            state: ProcessState::NotStarted,
            attempt: 0,
            pid: None,
            exit_code: None,
            started_at: None,
            exited_at: None,
            restart_times: Vec::new(),
        }
    }

    pub fn start(&mut self, pid: u32, at: SimTime) -> Result<(), DomainError> {
        if matches!(self.state, ProcessState::Running) {
            return Err(DomainError::conflict(
                "process_already_running",
                "the process is already running",
            ));
        }
        self.state = ProcessState::Running;
        self.attempt += 1;
        self.pid = Some(pid);
        self.exit_code = None;
        self.started_at = Some(at);
        self.exited_at = None;
        Ok(())
    }

    /// Applies a normal or failure exit. A zero code means `Exited`; otherwise
    /// the process is `Failed`.
    pub fn exit(&mut self, code: i32, at: SimTime) -> Result<(), DomainError> {
        if self.state != ProcessState::Running {
            return Err(DomainError::conflict(
                "process_not_running",
                "cannot exit a process that is not running",
            ));
        }
        self.exit_code = Some(code);
        self.exited_at = Some(at);
        self.state = if code == 0 {
            ProcessState::Exited
        } else {
            ProcessState::Failed
        };
        Ok(())
    }

    pub fn record_restart(&mut self, at: SimTime) {
        self.restart_times.push(at);
    }

    pub const fn is_running(&self) -> bool {
        matches!(self.state, ProcessState::Running)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    NotStarted,
    Running,
    Exited,
    Failed,
}

/// Runtime child entity, one per deployment in v0.1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: ContainerId,
    pub run_id: RunId,
    pub image_id: ImageId,
    pub name: String,
    pub state: ContainerState,
    pub health: HealthStatus,
    pub restart_count: u32,
    pub config: ContainerConfig,
    pub process: Process,
    pub exit_code: Option<i32>,
    pub failure_reason: Option<FailureReason>,
    pub created_at: SimTime,
    pub started_at: Option<SimTime>,
    pub stopped_at: Option<SimTime>,
}

impl Container {
    pub fn new(
        run_id: RunId,
        image_id: ImageId,
        name: impl Into<String>,
        config: ContainerConfig,
        created_at: SimTime,
    ) -> Self {
        let id = ContainerId::new();
        let mut process = Process::new(id);
        process.state = ProcessState::NotStarted;
        Self {
            id,
            run_id,
            image_id,
            name: name.into(),
            state: ContainerState::Created,
            health: HealthStatus::Unknown,
            restart_count: 0,
            config,
            process,
            exit_code: None,
            failure_reason: None,
            created_at,
            started_at: None,
            stopped_at: None,
        }
    }

    pub fn start(&mut self, at: SimTime) -> Result<(), DomainError> {
        match self.state {
            ContainerState::Created | ContainerState::Stopped | ContainerState::Failed => {
                self.state = ContainerState::Starting;
                self.health = HealthStatus::Starting;
                self.exit_code = None;
                self.failure_reason = None;
                self.started_at = Some(at);
                self.stopped_at = None;
                Ok(())
            }
            state => Err(DomainError::conflict(
                "invalid_container_transition",
                format!("cannot start a container while it is {state}"),
            )),
        }
    }

    pub fn mark_running(&mut self, at: SimTime) -> Result<(), DomainError> {
        if self.state != ContainerState::Starting {
            return Err(DomainError::conflict(
                "invalid_container_transition",
                format!("container must be starting, not {}", self.state),
            ));
        }
        self.state = ContainerState::Running;
        self.health = HealthStatus::Starting;
        self.started_at.get_or_insert(at);
        Ok(())
    }

    pub fn stop(&mut self, _at: SimTime) -> Result<(), DomainError> {
        if self.state != ContainerState::Running {
            return Err(DomainError::conflict(
                "invalid_container_transition",
                format!("cannot stop a container while it is {}", self.state),
            ));
        }
        self.state = ContainerState::Stopping;
        Ok(())
    }

    pub fn mark_stopped(
        &mut self,
        exit_code: i32,
        forced: bool,
        at: SimTime,
    ) -> Result<(), DomainError> {
        if self.state != ContainerState::Stopping {
            return Err(DomainError::conflict(
                "invalid_container_transition",
                format!("container must be stopping, not {}", self.state),
            ));
        }
        self.state = ContainerState::Stopped;
        self.health = HealthStatus::Unknown;
        self.exit_code = Some(exit_code);
        self.stopped_at = Some(at);
        if forced {
            self.failure_reason = Some(FailureReason::UserStopped);
        }
        Ok(())
    }

    pub fn fail(
        &mut self,
        reason: FailureReason,
        exit_code: Option<i32>,
        at: SimTime,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.state,
            ContainerState::Starting | ContainerState::Running | ContainerState::Stopping
        ) {
            return Err(DomainError::conflict(
                "invalid_container_transition",
                format!("cannot fail a container while it is {}", self.state),
            ));
        }
        self.state = ContainerState::Failed;
        self.health = HealthStatus::Unhealthy;
        self.exit_code = exit_code;
        self.failure_reason = Some(reason);
        self.stopped_at = Some(at);
        Ok(())
    }

    pub fn remove(&mut self) -> Result<(), DomainError> {
        if !matches!(self.state, ContainerState::Stopped | ContainerState::Failed) {
            return Err(DomainError::conflict(
                "invalid_container_transition",
                format!("cannot remove a container while it is {}", self.state),
            ));
        }
        self.state = ContainerState::Removed;
        Ok(())
    }

    pub fn manual_restart(&mut self, at: SimTime) -> Result<(), DomainError> {
        if self.state != ContainerState::Failed {
            return Err(DomainError::conflict(
                "invalid_container_transition",
                "manual restart requires a failed container",
            ));
        }
        self.state = ContainerState::Starting;
        self.health = HealthStatus::Starting;
        self.exit_code = None;
        self.failure_reason = None;
        self.started_at = Some(at);
        self.stopped_at = None;
        Ok(())
    }

    pub fn record_restart(&mut self, at: SimTime) {
        self.restart_count = self.restart_count.saturating_add(1);
        self.process.record_restart(at);
    }

    pub fn set_health(&mut self, health: HealthStatus) -> bool {
        if self.health == health {
            return false;
        }
        self.health = health;
        true
    }

    pub const fn is_live(&self) -> bool {
        self.state.is_live()
    }
}

/// Read-mostly aggregate describing the exposed service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deployment {
    pub id: DeploymentId,
    pub run_id: RunId,
    pub image_id: ImageId,
    pub container_id: ContainerId,
    pub service_name: String,
    pub status: DeploymentStatus,
    pub health: HealthStatus,
    pub primary_reason: Option<FailureReason>,
    pub created_at: SimTime,
    pub healthy_since: Option<SimTime>,
    pub uptime_ms: u64,
}

impl Deployment {
    pub fn new(
        run_id: RunId,
        image_id: ImageId,
        container_id: ContainerId,
        service_name: impl Into<String>,
        created_at: SimTime,
    ) -> Self {
        Self {
            id: DeploymentId::new(),
            run_id,
            image_id,
            container_id,
            service_name: service_name.into(),
            status: DeploymentStatus::Pending,
            health: HealthStatus::Unknown,
            primary_reason: None,
            created_at,
            healthy_since: None,
            uptime_ms: 0,
        }
    }

    /// Applies a derived status change. Returns false when unchanged.
    pub fn set_status(
        &mut self,
        next: DeploymentStatus,
        reason: Option<FailureReason>,
        health: HealthStatus,
        at: SimTime,
    ) -> Result<bool, DomainError> {
        if next == self.status {
            self.primary_reason = reason;
            self.health = health;
            return Ok(false);
        }
        if !self.can_transition(next) {
            return Err(DomainError::conflict(
                "invalid_deployment_transition",
                format!("cannot move deployment from {} to {next}", self.status),
            ));
        }
        self.accrue_uptime(at);
        if next == DeploymentStatus::Healthy {
            self.healthy_since = Some(at);
        } else {
            self.healthy_since = None;
        }
        self.status = next;
        self.health = health;
        self.primary_reason = reason;
        Ok(true)
    }

    pub const fn can_transition(&self, next: DeploymentStatus) -> bool {
        matches!(
            (self.status, next),
            (DeploymentStatus::Pending, DeploymentStatus::Deploying)
                | (DeploymentStatus::Pending, DeploymentStatus::Failed)
                | (DeploymentStatus::Pending, DeploymentStatus::Stopped)
                | (DeploymentStatus::Deploying, DeploymentStatus::Healthy)
                | (DeploymentStatus::Deploying, DeploymentStatus::Degraded)
                | (DeploymentStatus::Deploying, DeploymentStatus::Failed)
                | (DeploymentStatus::Deploying, DeploymentStatus::Stopped)
                | (DeploymentStatus::Healthy, DeploymentStatus::Degraded)
                | (DeploymentStatus::Healthy, DeploymentStatus::Failed)
                | (DeploymentStatus::Healthy, DeploymentStatus::Stopped)
                | (DeploymentStatus::Degraded, DeploymentStatus::Healthy)
                | (DeploymentStatus::Degraded, DeploymentStatus::Failed)
                | (DeploymentStatus::Degraded, DeploymentStatus::Stopped)
                | (DeploymentStatus::Failed, DeploymentStatus::Stopped)
        )
    }

    pub fn accrue_uptime(&mut self, now: SimTime) {
        if let Some(since) = self.healthy_since {
            let delta = now.as_millis().saturating_sub(since.as_millis()).max(0) as u64;
            self.uptime_ms = self.uptime_ms.saturating_add(delta);
            self.healthy_since = Some(now);
        }
    }

    pub fn stop(&mut self, at: SimTime, reason: FailureReason) -> Result<bool, DomainError> {
        let health = self.health;
        self.set_status(DeploymentStatus::Stopped, Some(reason), health, at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::ScenarioPreset;
    use crate::status::RestartPolicyKind;

    fn config() -> ContainerConfig {
        ContainerConfig::from_service(&ScenarioPreset::HappyPath.scenario().service)
    }

    #[test]
    fn run_lifecycle_is_enforced() {
        let mut run = SimulationRun::new(
            ProjectId::new(),
            ScenarioId::new(),
            "Happy Path",
            7,
            SourceSnapshotId::new(),
            1,
            "{}",
            TimestampUtc::from_millis(1),
            "0.1.0",
            "0.1.0",
        );
        run.pause().unwrap();
        run.resume().unwrap();
        let summary = RunSummary {
            status: RunStatus::Completed,
            result: RunResult::Healthy,
            ..RunSummary::default()
        };
        run.finalise(
            RunStatus::Completed,
            RunResult::Healthy,
            summary,
            TimestampUtc::from_millis(2),
        )
        .unwrap();
        assert!(run.pause().is_err());
        assert!(run
            .finalise(
                RunStatus::Cancelled,
                RunResult::Unknown,
                RunSummary {
                    status: RunStatus::Cancelled,
                    ..RunSummary::default()
                },
                TimestampUtc::from_millis(3),
            )
            .is_err());
    }

    #[test]
    fn event_sequences_must_be_consecutive() {
        let mut run = SimulationRun::new(
            ProjectId::new(),
            ScenarioId::new(),
            "s",
            1,
            SourceSnapshotId::new(),
            1,
            "{}",
            TimestampUtc::from_millis(0),
            "0.1.0",
            "0.1.0",
        );
        run.record_event_seq(1).unwrap();
        assert!(run.record_event_seq(3).is_err());
        run.record_event_seq(2).unwrap();
    }

    #[test]
    fn container_cannot_be_removed_while_running() {
        let mut container =
            Container::new(RunId::new(), ImageId::new(), "web", config(), SimTime::ZERO);
        assert!(container
            .start(SimTime::ZERO)
            .and_then(|_| container.mark_running(SimTime::from_secs(1)))
            .is_ok());
        assert!(container.remove().is_err());
        container.stop(SimTime::from_secs(2)).unwrap();
        container
            .mark_stopped(0, false, SimTime::from_secs(2))
            .unwrap();
        container.remove().unwrap();
        assert_eq!(container.state, ContainerState::Removed);
    }

    #[test]
    fn restart_counter_never_decreases() {
        let mut container =
            Container::new(RunId::new(), ImageId::new(), "web", config(), SimTime::ZERO);
        container.record_restart(SimTime::from_secs(1));
        container.record_restart(SimTime::from_secs(2));
        assert_eq!(container.restart_count, 2);
        assert_eq!(container.process.restart_times.len(), 2);
    }

    #[test]
    fn deployment_transitions_follow_the_diagram() {
        let mut deployment = Deployment::new(
            RunId::new(),
            ImageId::new(),
            ContainerId::new(),
            "web",
            SimTime::ZERO,
        );
        deployment
            .set_status(
                DeploymentStatus::Deploying,
                None,
                HealthStatus::Starting,
                SimTime::from_secs(1),
            )
            .unwrap();
        deployment
            .set_status(
                DeploymentStatus::Healthy,
                None,
                HealthStatus::Healthy,
                SimTime::from_secs(5),
            )
            .unwrap();
        deployment.accrue_uptime(SimTime::from_secs(9));
        assert_eq!(deployment.uptime_ms, 4_000);
        deployment
            .set_status(
                DeploymentStatus::Degraded,
                Some(FailureReason::FirewallDenied),
                HealthStatus::Healthy,
                SimTime::from_secs(10),
            )
            .unwrap();
        assert_eq!(deployment.uptime_ms, 5_000);
        assert!(deployment
            .set_status(
                DeploymentStatus::Deploying,
                None,
                HealthStatus::Starting,
                SimTime::from_secs(11),
            )
            .is_err());
    }

    #[test]
    fn commands_validate_speed_and_restart_policy_is_carried_over() {
        assert!(RunCommand::SetSpeed { speed: 8 }.validate().is_ok());
        assert!(RunCommand::SetSpeed { speed: 3 }.validate().is_err());
        assert_eq!(config().restart_policy.policy, RestartPolicyKind::OnFailure);
    }
}
