//! Engine implementation: virtual clock, build pipeline, runtime, health, and route.

use koncet_domain::event_types;
use koncet_domain::{
    Build, BuildFailure, BuildStageName, BuildStageStatus, Container, ContainerConfig,
    ContainerState, Deployment, DeploymentStatus, DomainError, DomainEvent, EventSource,
    FailureReason, HealthStatus, LogRecord, LogSource, MetricEntity, MetricSample, ProcessState,
    ProjectId, RecordedCommand, RunCommand, RunId, RunResult, RunStatus, RunSummary, Scenario,
    ScenarioId, Severity, SimTime, SimulatedImage, SimulationRun, SourceSnapshot, TimestampUtc,
    MAX_SEED, SCENARIO_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ids;
use crate::rng::RngStreams;

/// Fixed simulation tick. Never variable.
pub const TICK_MS: i64 = 100;
/// Domain events are never dropped; logs stop at this cap with one notice.
const LOG_CAP: usize = 25_000;
/// Per-run metric cap to bound storage while keeping deterministic output.
const METRIC_CAP: usize = 100_000;

/// Immutable inputs supplied by the application layer.
#[derive(Debug, Clone)]
pub struct SimulationInputs {
    pub run_id: RunId,
    pub project_id: ProjectId,
    pub scenario_id: ScenarioId,
    pub scenario: Scenario,
    pub source_snapshot: SourceSnapshot,
    pub seed: u64,
    pub started_at: TimestampUtc,
    pub app_version: String,
    pub engine_version: String,
}

/// Final or in-progress state, suitable for persistence and golden fixtures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationSnapshot {
    pub run: SimulationRun,
    pub build: Build,
    pub image: Option<SimulatedImage>,
    pub container: Option<Container>,
    pub deployment: Option<Deployment>,
    pub command_trace: Vec<RecordedCommand>,
}

/// Everything produced by one engine instance.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationOutput {
    pub snapshot: SimulationSnapshot,
    pub events: Vec<DomainEvent>,
    pub logs: Vec<LogRecord>,
    pub metrics: Vec<MetricSample>,
}

/// One mutable owner of simulation state.
pub struct SimulationEngine {
    run: SimulationRun,
    scenario: Scenario,
    source: SourceSnapshot,
    build: Build,
    image: Option<SimulatedImage>,
    container: Option<Container>,
    deployment: Option<Deployment>,

    clock: SimTime,
    started_wall_ms: i64,
    paused: bool,
    speed: u8,

    events: Vec<DomainEvent>,
    logs: Vec<LogRecord>,
    metrics: Vec<MetricSample>,
    commands: Vec<RecordedCommand>,
    event_seq: u64,
    log_seq: u64,
    command_seq: u64,

    rng: RngStreams,

    stage_end_at: Option<SimTime>,
    stage_fail_at: Option<SimTime>,
    stage_failure_reported: bool,

    startup_stall_fired: Vec<bool>,
    crash_fired_once: Vec<bool>,
    crash_last_attempt: Vec<Option<u32>>,
    container_started_at: Option<SimTime>,
    startup_ready_at: Option<SimTime>,
    pending_restart_at: Option<SimTime>,

    next_health_check_at: Option<SimTime>,
    health_consecutive_failures: u32,

    next_metric_at: Option<SimTime>,
    route_reported: Option<bool>,

    logs_truncated: bool,
    metrics_truncated: bool,
}

impl SimulationEngine {
    /// Validates inputs and emits the run-start event batch at simulated time zero.
    pub fn start(inputs: SimulationInputs) -> Result<Self, DomainError> {
        if inputs.seed > MAX_SEED {
            return Err(DomainError::validation(
                "out_of_range",
                "/seed",
                "seed must be a 63-bit non-negative integer",
            ));
        }
        if inputs.source_snapshot.project_id != inputs.project_id {
            return Err(DomainError::internal(
                "snapshot_project_mismatch",
                "the source snapshot does not belong to the run project",
            ));
        }

        let source_tree = inputs.source_snapshot.to_source_tree();
        let report = inputs.scenario.validate_for_run(&source_tree, None);
        if let Some(error) = report.to_error() {
            return Err(error);
        }

        let scenario_json = inputs.scenario.canonical_json();
        let metric_interval_ms =
            (inputs.scenario.monitoring.sample_interval_s as i64 * 1_000).max(TICK_MS);
        let mut run = SimulationRun::new(
            inputs.project_id,
            inputs.scenario_id,
            inputs.scenario.name.trim().to_string(),
            inputs.seed,
            inputs.source_snapshot.id,
            SCENARIO_SCHEMA_VERSION,
            scenario_json,
            inputs.started_at,
            inputs.app_version.clone(),
            inputs.engine_version.clone(),
        );
        run.id = inputs.run_id;

        let mut build = Build::new(run.id, inputs.scenario.build.profile);
        build.id = ids::build_id(run.id);

        let fault_count = inputs.scenario.faults.len();
        let mut engine = Self {
            run,
            scenario: inputs.scenario,
            source: inputs.source_snapshot,
            build,
            image: None,
            container: None,
            deployment: None,
            clock: SimTime::ZERO,
            started_wall_ms: inputs.started_at.as_millis(),
            paused: false,
            speed: 1,
            events: Vec::new(),
            logs: Vec::new(),
            metrics: Vec::new(),
            commands: Vec::new(),
            event_seq: 0,
            log_seq: 0,
            command_seq: 0,
            rng: RngStreams::from_seed(inputs.seed),
            stage_end_at: None,
            stage_fail_at: None,
            stage_failure_reported: false,
            startup_stall_fired: vec![false; fault_count],
            crash_fired_once: vec![false; fault_count],
            crash_last_attempt: vec![None; fault_count],
            container_started_at: None,
            startup_ready_at: None,
            pending_restart_at: None,
            next_health_check_at: None,
            health_consecutive_failures: 0,
            next_metric_at: Some(SimTime::from_millis(metric_interval_ms)),
            route_reported: None,
            logs_truncated: false,
            metrics_truncated: false,
        };

        engine.emit_run_start(report);
        Ok(engine)
    }

    fn emit_run_start(&mut self, report: koncet_domain::ValidationReport) {
        let warnings: Vec<Value> = report
            .warnings()
            .map(|issue| {
                json!({
                    "code": issue.code,
                    "path": issue.path,
                })
            })
            .collect();

        self.emit(
            Severity::Info,
            EventSource::Engine,
            event_types::RUN_CREATED,
            json!({
                "project_id": self.run.project_id,
                "scenario_id": self.run.scenario_id,
                "seed": self.run.seed,
                "app_version": self.run.app_version,
                "engine_version": self.run.engine_version,
            }),
        );
        self.emit(
            Severity::Info,
            EventSource::Engine,
            event_types::SOURCE_SNAPSHOT_CREATED,
            json!({
                "snapshot_id": self.source.id,
                "content_hash": self.source.content_hash,
                "file_count": self.source.file_count(),
                "total_bytes": self.source.total_bytes,
            }),
        );
        self.emit(
            Severity::Info,
            EventSource::Engine,
            event_types::SCENARIO_VALIDATED,
            json!({ "warnings": warnings }),
        );
        self.emit(
            Severity::Info,
            EventSource::Engine,
            event_types::RUN_STARTED,
            json!({
                "source_snapshot_id": self.source.id,
                "file_count": self.source.file_count(),
                "total_bytes": self.source.total_bytes,
            }),
        );
        self.emit(
            Severity::Info,
            EventSource::Build,
            event_types::BUILD_QUEUED,
            json!({
                "build_id": self.build.id,
                "profile": self.build.profile.as_str(),
            }),
        );
    }

    // ---------------------------------------------------------------------
    // Public controls and accessors
    // ---------------------------------------------------------------------

    pub fn advance_ticks(&mut self, ticks: u64) -> u64 {
        let mut processed = 0;
        for _ in 0..ticks {
            if self.run.status.is_terminal() || self.paused {
                break;
            }
            self.clock = self.clock.saturating_add(TICK_MS);
            self.step();
            processed += 1;
        }
        processed
    }

    pub fn advance_to(&mut self, time: SimTime) -> u64 {
        if time.as_millis() <= self.clock.as_millis() {
            return 0;
        }
        let ticks = ((time.as_millis() - self.clock.as_millis()) / TICK_MS) as u64;
        self.advance_ticks(ticks)
    }

    /// Runs until terminal or the safety limit. Used by the headless harness.
    pub fn run_to_completion(&mut self, max_ticks: u64) -> u64 {
        self.advance_ticks(max_ticks)
    }

    /// Records and applies a user command at the current tick.
    pub fn submit_command(&mut self, command: RunCommand) -> Result<(), DomainError> {
        if self.run.status.is_terminal() {
            return Err(DomainError::conflict(
                "run_finalised",
                "the run is finalised and accepts no more commands",
            ));
        }
        command.validate()?;

        self.command_seq += 1;
        self.commands.push(RecordedCommand {
            seq: self.command_seq,
            at: self.clock,
            command: command.clone(),
        });
        let details = serde_json::to_value(&command).unwrap_or(Value::Null);
        self.emit(
            Severity::Info,
            EventSource::User,
            event_types::RUN_COMMAND,
            json!({ "command": command.name(), "details": details }),
        );

        match command {
            RunCommand::Pause => {
                if !self.paused {
                    self.run.pause()?;
                    self.paused = true;
                    self.emit(
                        Severity::Info,
                        EventSource::Engine,
                        event_types::RUN_PAUSED,
                        json!({}),
                    );
                }
            }
            RunCommand::Resume => {
                if self.paused {
                    self.run.resume()?;
                    self.paused = false;
                    self.emit(
                        Severity::Info,
                        EventSource::Engine,
                        event_types::RUN_RESUMED,
                        json!({}),
                    );
                }
            }
            RunCommand::SetSpeed { speed } => {
                self.speed = speed;
                self.emit(
                    Severity::Info,
                    EventSource::Engine,
                    event_types::RUN_SPEED_CHANGED,
                    json!({ "speed": speed }),
                );
            }
            RunCommand::Stop => {
                let result = self.current_result();
                self.finish(
                    RunStatus::Completed,
                    result,
                    Some(FailureReason::UserStopped),
                    true,
                );
            }
            RunCommand::Cancel => {
                self.emit(
                    Severity::Warn,
                    EventSource::Engine,
                    event_types::RUN_CANCELLED,
                    json!({ "reason": FailureReason::UserStopped.as_str() }),
                );
                let result = self.current_result();
                self.finish(
                    RunStatus::Cancelled,
                    result,
                    Some(FailureReason::UserStopped),
                    true,
                );
            }
            RunCommand::RestartContainer => {
                self.manual_restart()?;
            }
            RunCommand::CheckConnectivity => {
                self.check_connectivity();
            }
        }
        Ok(())
    }

    pub fn clock(&self) -> SimTime {
        self.clock
    }

    pub fn speed(&self) -> u8 {
        self.speed
    }

    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn is_terminal(&self) -> bool {
        self.run.status.is_terminal()
    }

    pub fn run(&self) -> &SimulationRun {
        &self.run
    }

    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    pub fn build(&self) -> &Build {
        &self.build
    }

    pub fn image(&self) -> Option<&SimulatedImage> {
        self.image.as_ref()
    }

    pub fn container(&self) -> Option<&Container> {
        self.container.as_ref()
    }

    pub fn deployment(&self) -> Option<&Deployment> {
        self.deployment.as_ref()
    }

    pub fn commands(&self) -> &[RecordedCommand] {
        &self.commands
    }

    pub fn events(&self) -> &[DomainEvent] {
        &self.events
    }

    pub fn logs(&self) -> &[LogRecord] {
        &self.logs
    }

    pub fn metrics(&self) -> &[MetricSample] {
        &self.metrics
    }

    pub fn drain_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn drain_logs(&mut self) -> Vec<LogRecord> {
        std::mem::take(&mut self.logs)
    }

    pub fn drain_metrics(&mut self) -> Vec<MetricSample> {
        std::mem::take(&mut self.metrics)
    }

    pub fn snapshot(&self) -> SimulationSnapshot {
        SimulationSnapshot {
            run: self.run.clone(),
            build: self.build.clone(),
            image: self.image.clone(),
            container: self.container.clone(),
            deployment: self.deployment.clone(),
            command_trace: self.commands.clone(),
        }
    }

    pub fn into_output(self) -> SimulationOutput {
        SimulationOutput {
            snapshot: SimulationSnapshot {
                run: self.run,
                build: self.build,
                image: self.image,
                container: self.container,
                deployment: self.deployment,
                command_trace: self.commands,
            },
            events: self.events,
            logs: self.logs,
            metrics: self.metrics,
        }
    }

    // ---------------------------------------------------------------------
    // Tick pipeline, in fixed priority order
    // ---------------------------------------------------------------------

    fn step(&mut self) {
        self.build_step();
        if self.run.status.is_terminal() {
            return;
        }
        self.container_step();
        if self.run.status.is_terminal() {
            return;
        }
        self.process_step();
        if self.run.status.is_terminal() {
            return;
        }
        self.health_step();
        if self.run.status.is_terminal() {
            return;
        }
        self.recompute_deployment();
        self.metric_step();
        self.limit_step();
    }

    fn build_step(&mut self) {
        if self.build.completed() {
            return;
        }

        if let Some(stage) = self.build.current_stage() {
            let kind = stage.kind;
            let planned = stage.planned_ms;
            if !self.stage_failure_reported && self.stage_fail_at.is_some_and(|at| self.clock >= at)
            {
                if let Some(injection) = self
                    .scenario
                    .build
                    .failure_injections
                    .iter()
                    .find(|injection| injection.stage == kind)
                    .cloned()
                {
                    self.stage_failure_reported = true;
                    self.emit(
                        Severity::Warn,
                        EventSource::Build,
                        event_types::BUILD_FAULT_INJECTED,
                        json!({
                            "stage": kind.as_str(),
                            "at_percent": injection.at_percent,
                            "message": injection.message,
                            "code": injection.code(),
                        }),
                    );
                    self.log(
                        LogSource::Build,
                        Severity::Warn,
                        "injected build failure",
                        &[
                            ("stage", json!(kind.as_str())),
                            ("code", json!(injection.code())),
                        ],
                    );
                    let failure = BuildFailure::new(
                        kind,
                        injection.code(),
                        injection.message.clone(),
                        injection.at_percent,
                    );
                    if self
                        .build
                        .finish_stage(kind, self.clock, Some(failure))
                        .is_ok()
                    {
                        self.emit_stage_finished(kind, BuildStageStatus::Failed);
                        self.emit(
                            Severity::Error,
                            EventSource::Build,
                            event_types::BUILD_FAILED,
                            json!({
                                "stage": kind.as_str(),
                                "code": injection.code(),
                                "message": injection.message,
                            }),
                        );
                        self.log(
                            LogSource::Build,
                            Severity::Error,
                            "build failed",
                            &[("stage", json!(kind.as_str()))],
                        );
                        self.finish(
                            RunStatus::Completed,
                            RunResult::BuildFailed,
                            Some(FailureReason::BuildFailed),
                            false,
                        );
                    }
                }
                return;
            }

            if self.stage_end_at.is_some_and(|at| self.clock >= at)
                && self.build.finish_stage(kind, self.clock, None).is_ok()
            {
                self.emit_stage_finished(kind, BuildStageStatus::Succeeded);
                self.log(
                    LogSource::Build,
                    Severity::Info,
                    "build stage succeeded",
                    &[("stage", json!(kind.as_str()))],
                );
                let _ = planned;
                if self.build.succeeded() {
                    self.promote_image();
                }
            }
            return;
        }

        // No stage is running: start the next pending stage at this tick.
        let next = self.build.next_pending().map(|stage| stage.kind);
        let Some(kind) = next else {
            return;
        };
        if self.build.start_stage(kind, self.clock).is_err() {
            return;
        }
        let planned = self
            .build
            .stage(kind)
            .map(|stage| stage.planned_ms)
            .unwrap_or(1);
        self.stage_end_at = Some(self.clock.saturating_add(planned as i64));
        self.stage_fail_at = self
            .scenario
            .build
            .failure_injections
            .iter()
            .find(|injection| injection.stage == kind)
            .map(|injection| {
                let offset = (planned as u128 * injection.at_percent as u128 / 100) as i64;
                self.clock.saturating_add(offset)
            });
        self.stage_failure_reported = false;
        self.emit(
            Severity::Info,
            EventSource::Build,
            event_types::BUILD_STAGE_STARTED,
            json!({
                "stage": kind.as_str(),
                "ordinal": kind.ordinal(),
                "planned_ms": planned,
            }),
        );
        self.log(
            LogSource::Build,
            Severity::Info,
            "build stage started",
            &[("stage", json!(kind.as_str()))],
        );
    }

    fn emit_stage_finished(&mut self, kind: BuildStageName, status: BuildStageStatus) {
        let actual_ms = self
            .build
            .stage(kind)
            .and_then(|stage| stage.actual_ms)
            .unwrap_or(0);
        let severity = match status {
            BuildStageStatus::Failed => Severity::Error,
            BuildStageStatus::Skipped => Severity::Warn,
            _ => Severity::Info,
        };
        self.emit(
            severity,
            EventSource::Build,
            event_types::BUILD_STAGE_FINISHED,
            json!({
                "stage": kind.as_str(),
                "status": status.as_str(),
                "actual_ms": actual_ms,
            }),
        );
    }

    fn promote_image(&mut self) {
        let image = self.make_image();
        self.build.image_id = Some(image.id);
        self.emit(
            Severity::Info,
            EventSource::Build,
            event_types::BUILD_SUCCEEDED,
            json!({
                "build_id": self.build.id,
                "image_id": image.id,
                "digest": image.digest,
            }),
        );
        self.log(
            LogSource::Build,
            Severity::Info,
            "build succeeded",
            &[("image", json!(image.reference()))],
        );
        self.emit(
            Severity::Info,
            EventSource::Build,
            event_types::IMAGE_CREATED,
            json!({
                "image_id": image.id,
                "repository": image.repository,
                "tag": image.tag,
                "digest": image.digest,
                "size_bytes": image.size_bytes,
                "layers": image.layers.iter().map(|layer| json!({
                    "name": layer.name,
                    "digest": layer.digest,
                    "size_bytes": layer.size_bytes,
                })).collect::<Vec<_>>(),
            }),
        );

        self.start_runtime(image);
    }

    fn make_image(&self) -> SimulatedImage {
        let mut image = SimulatedImage::create(
            self.run.id,
            self.run.project_id,
            self.scenario.build.profile,
            &self.source.content_hash,
            self.source.total_bytes,
            &self.run.engine_version,
            self.clock,
        );
        image.id = ids::image_id(self.run.id);
        image.digest = image.compute_digest();
        image
    }

    fn start_runtime(&mut self, image: SimulatedImage) {
        let run_id = self.run.id;
        let config = ContainerConfig::from_service(&self.scenario.service);
        let mut container = Container::new(
            run_id,
            image.id,
            self.scenario.service.name.clone(),
            config,
            self.clock,
        );
        container.id = ids::container_id(run_id);
        container.process.container_id = container.id;

        let mut deployment = Deployment::new(
            run_id,
            image.id,
            container.id,
            self.scenario.service.name.clone(),
            self.clock,
        );
        deployment.id = ids::deployment_id(run_id);

        self.emit(
            Severity::Info,
            EventSource::Container,
            event_types::CONTAINER_CREATED,
            json!({
                "container_id": container.id,
                "name": container.name,
                "image_id": image.id,
            }),
        );
        self.log(
            LogSource::Container,
            Severity::Info,
            "container created",
            &[("container", json!(container.name))],
        );

        self.emit(
            Severity::Info,
            EventSource::Deployment,
            event_types::DEPLOYMENT_CREATED,
            json!({
                "deployment_id": deployment.id,
                "service_name": deployment.service_name,
                "container_id": container.id,
            }),
        );

        if container.start(self.clock).is_ok() {
            self.emit(
                Severity::Info,
                EventSource::Container,
                event_types::CONTAINER_STARTING,
                json!({ "container_id": container.id }),
            );
        }
        let _ = deployment.set_status(
            DeploymentStatus::Deploying,
            None,
            HealthStatus::Starting,
            self.clock,
        );
        self.emit(
            Severity::Info,
            EventSource::Deployment,
            event_types::DEPLOYMENT_DEPLOYING,
            json!({ "deployment_id": deployment.id }),
        );

        self.container_started_at = Some(self.clock);
        let startup_ms = (self.scenario.service.startup_delay_s as i64 * 1_000).max(100);
        self.startup_ready_at = Some(self.clock.saturating_add(startup_ms));
        self.container = Some(container);
        self.deployment = Some(deployment);
        self.image = Some(image);
    }

    fn container_step(&mut self) {
        let Some(container) = self.container.as_ref() else {
            return;
        };
        if matches!(
            container.state,
            ContainerState::Failed | ContainerState::Removed | ContainerState::Stopped
        ) {
            return;
        }

        let now = self.clock;
        let base = self.container_started_at.unwrap_or(now);

        // Startup stalls extend readiness while the container is starting.
        let faults = self.scenario.faults.clone();
        for (index, fault) in faults.iter().enumerate() {
            if fault.kind != koncet_domain::FaultKind::StartupStall {
                continue;
            }
            if self.startup_stall_fired.get(index).copied().unwrap_or(true) {
                continue;
            }
            let at_s = fault.at_s.unwrap_or(0) as i64;
            if now.as_millis() < base.as_millis() + at_s * 1_000 {
                continue;
            }
            self.startup_stall_fired[index] = true;
            let duration_s = fault.duration_s.unwrap_or(1) as i64;
            self.emit(
                Severity::Warn,
                EventSource::Process,
                event_types::PROCESS_FAULT_INJECTED,
                json!({
                    "kind": "startup_stall",
                    "at_s": at_s,
                    "duration_s": duration_s,
                }),
            );
            self.log(
                LogSource::Process,
                Severity::Warn,
                "startup stall injected",
                &[("duration_s", json!(duration_s))],
            );
            let remaining = duration_s * 1_000;
            if self
                .container
                .as_ref()
                .is_some_and(|container| container.state == ContainerState::Starting)
            {
                let ready = self
                    .startup_ready_at
                    .unwrap_or(now)
                    .saturating_add(remaining)
                    .max(now.saturating_add(remaining));
                self.startup_ready_at = Some(ready);
            } else if self
                .container
                .as_ref()
                .is_some_and(|container| container.state == ContainerState::Running)
            {
                let next = now.saturating_add(remaining);
                self.next_health_check_at = Some(next);
                self.set_container_health(HealthStatus::Starting, "startup_stall");
            }
        }

        // Start the supervised process shortly after the container begins starting.
        let process_not_started = self
            .container
            .as_ref()
            .is_some_and(|container| container.process.state == ProcessState::NotStarted);
        if process_not_started && now.as_millis() >= base.as_millis() + TICK_MS {
            let pid = self.rng.next_pid();
            let mut started_attempt = None;
            if let Some(container) = self.container.as_mut() {
                if container.process.start(pid, now).is_ok() {
                    started_attempt = Some(container.process.attempt);
                }
            }
            if let Some(attempt) = started_attempt {
                self.emit(
                    Severity::Info,
                    EventSource::Process,
                    event_types::PROCESS_STARTED,
                    json!({ "attempt": attempt, "pid": pid }),
                );
                self.log(
                    LogSource::Process,
                    Severity::Info,
                    "process started",
                    &[("attempt", json!(attempt))],
                );
            }
        }

        // Move to running once readiness has elapsed and the process exists.
        let ready = self.startup_ready_at.unwrap_or(now);
        let can_run = now >= ready
            && self
                .container
                .as_ref()
                .is_some_and(|container| container.process.is_running());
        if can_run
            && self
                .container
                .as_ref()
                .is_some_and(|container| container.state == ContainerState::Starting)
        {
            let mut running = None;
            if let Some(container) = self.container.as_mut() {
                if container.mark_running(now).is_ok() {
                    running = Some((container.id, container.config.port));
                }
            }
            if let Some((container_id, port)) = running {
                self.emit(
                    Severity::Info,
                    EventSource::Container,
                    event_types::CONTAINER_RUNNING,
                    json!({ "container_id": container_id, "port": port }),
                );
                self.log(
                    LogSource::Container,
                    Severity::Info,
                    "container running",
                    &[("port", json!(port))],
                );
                self.next_health_check_at = Some(now);
            }
        }
    }

    fn process_step(&mut self) {
        if self
            .container
            .as_ref()
            .is_some_and(|container| container.state == ContainerState::Failed)
        {
            return;
        }

        // Execute a scheduled restart before evaluating new faults.
        if self.pending_restart_at.is_some_and(|at| self.clock >= at) {
            self.pending_restart_at = None;
            let pid = self.rng.next_pid();
            let mut restarted = None;
            if let Some(container) = self.container.as_mut() {
                container.record_restart(self.clock);
                if container.process.start(pid, self.clock).is_ok() {
                    container.set_health(HealthStatus::Starting);
                    restarted = Some((
                        container.id,
                        container.process.attempt,
                        container.restart_count,
                    ));
                }
            }
            if let Some((container_id, attempt, restart_count)) = restarted {
                self.emit(
                    Severity::Info,
                    EventSource::Container,
                    event_types::CONTAINER_RESTART_COUNT_CHANGED,
                    json!({
                        "container_id": container_id,
                        "restart_count": restart_count,
                    }),
                );
                self.emit(
                    Severity::Info,
                    EventSource::Process,
                    event_types::PROCESS_STARTED,
                    json!({ "attempt": attempt, "pid": pid }),
                );
                self.log(
                    LogSource::Process,
                    Severity::Info,
                    "process restarted",
                    &[
                        ("attempt", json!(attempt)),
                        ("restarts", json!(restart_count)),
                    ],
                );
            }
            let interval =
                (self.scenario.service.health_check.interval_s as i64 * 1_000).max(TICK_MS);
            self.next_health_check_at = Some(self.clock.saturating_add(interval));
            self.health_consecutive_failures = 0;
        }

        let running = self
            .container
            .as_ref()
            .is_some_and(|container| container.process.is_running());
        if !running {
            return;
        }

        let faults = self.scenario.faults.clone();
        for (index, fault) in faults.iter().enumerate() {
            if fault.kind != koncet_domain::FaultKind::ProcessCrash {
                continue;
            }
            let once = fault.once.unwrap_or(true);
            if once && self.crash_fired_once.get(index).copied().unwrap_or(true) {
                continue;
            }
            let Some(started_at) = self
                .container
                .as_ref()
                .and_then(|container| container.process.started_at)
            else {
                continue;
            };
            let after_s = fault.after_s.unwrap_or(1) as i64;
            if self.clock.as_millis() < started_at.as_millis() + after_s * 1_000 {
                continue;
            }
            let attempt = self
                .container
                .as_ref()
                .map(|container| container.process.attempt)
                .unwrap_or(0);
            if self.crash_last_attempt.get(index).copied().flatten() == Some(attempt) {
                continue;
            }
            if once {
                self.crash_fired_once[index] = true;
            }
            self.crash_last_attempt[index] = Some(attempt);
            let exit_code = fault.exit_code.unwrap_or(1) as i32;
            self.emit(
                Severity::Warn,
                EventSource::Process,
                event_types::PROCESS_FAULT_INJECTED,
                json!({
                    "kind": "process_crash",
                    "after_s": after_s,
                    "duration_s": Value::Null,
                }),
            );
            self.log(
                LogSource::Process,
                Severity::Warn,
                "process crash injected",
                &[("after_s", json!(after_s))],
            );
            self.handle_process_crash(exit_code);
            return;
        }
    }

    fn handle_process_crash(&mut self, exit_code: i32) {
        if let Some(container) = self.container.as_mut() {
            if container.process.exit(exit_code, self.clock).is_err() {
                return;
            }
        } else {
            return;
        }
        self.emit(
            Severity::Warn,
            EventSource::Process,
            event_types::PROCESS_EXITED,
            json!({ "exit_code": exit_code, "reason": "process_crashed" }),
        );
        self.log(
            LogSource::Process,
            Severity::Warn,
            "process exited unexpectedly",
            &[("exit_code", json!(exit_code))],
        );
        self.set_container_health(HealthStatus::Unhealthy, "process_exited");

        let policy = self.scenario.service.restart_policy.clone();
        let restart_count = self
            .container
            .as_ref()
            .map(|container| container.restart_count)
            .unwrap_or(0);
        let max_retries = policy.max_retries.unwrap_or(3);

        match policy.policy {
            koncet_domain::RestartPolicyKind::No => {
                self.fail_container(FailureReason::ContainerFailed, Some(exit_code));
            }
            koncet_domain::RestartPolicyKind::OnFailure if restart_count >= max_retries => {
                let attempts = self
                    .container
                    .as_ref()
                    .map(|container| container.process.attempt)
                    .unwrap_or(0);
                self.emit(
                    Severity::Error,
                    EventSource::Process,
                    event_types::PROCESS_RESTART_EXHAUSTED,
                    json!({ "attempts": attempts, "max_retries": max_retries }),
                );
                self.log(
                    LogSource::Process,
                    Severity::Error,
                    "restart limit reached",
                    &[
                        ("attempts", json!(attempts)),
                        ("max_retries", json!(max_retries)),
                    ],
                );
                self.fail_container(FailureReason::RestartExhausted, Some(exit_code));
            }
            _ => {
                let exponent = restart_count.min(2);
                let delay_ms = 1_000u64 << exponent;
                self.pending_restart_at = Some(self.clock.saturating_add(delay_ms as i64));
                let next_attempt = self
                    .container
                    .as_ref()
                    .map(|container| container.process.attempt + 1)
                    .unwrap_or(1);
                self.emit(
                    Severity::Warn,
                    EventSource::Process,
                    event_types::PROCESS_RESTART_SCHEDULED,
                    json!({
                        "attempt": next_attempt,
                        "delay_ms": delay_ms,
                        "policy": policy.policy.as_str(),
                    }),
                );
                self.log(
                    LogSource::Process,
                    Severity::Warn,
                    "process restart scheduled",
                    &[("delay_ms", json!(delay_ms))],
                );
            }
        }
    }

    fn fail_container(&mut self, reason: FailureReason, exit_code: Option<i32>) {
        let failed = {
            let Some(container) = self.container.as_mut() else {
                return;
            };
            match container.fail(reason, exit_code, self.clock) {
                Ok(()) => Some(container.id),
                Err(_) => None,
            }
        };
        let Some(container_id) = failed else {
            return;
        };
        self.emit(
            Severity::Error,
            EventSource::Container,
            event_types::CONTAINER_FAILED,
            json!({
                "container_id": container_id,
                "reason": reason.as_str(),
                "exit_code": exit_code,
            }),
        );
        self.log(
            LogSource::Container,
            Severity::Error,
            "container failed",
            &[("reason", json!(reason.as_str()))],
        );
        self.recompute_deployment();
        self.finish(
            RunStatus::Failed,
            RunResult::RuntimeFailed,
            Some(reason),
            false,
        );
    }

    fn health_step(&mut self) {
        let Some(next) = self.next_health_check_at else {
            return;
        };
        if self.clock < next {
            return;
        }
        let Some(container) = self.container.as_ref() else {
            return;
        };
        if container.state != ContainerState::Running {
            return;
        }

        let interval = (self.scenario.service.health_check.interval_s as i64 * 1_000).max(TICK_MS);
        let retries = self.scenario.service.health_check.retries;

        // Advance the schedule first so a failing probe can never stall the clock.
        let mut next_at = next.saturating_add(interval);
        if next_at <= self.clock {
            let behind = (self.clock.as_millis() - next_at.as_millis()) / interval;
            next_at = next_at.saturating_add((behind + 1) * interval);
        }
        self.next_health_check_at = Some(next_at);

        let process_running = self
            .container
            .as_ref()
            .is_some_and(|container| container.process.is_running());
        let fault_active = self.scenario.faults.iter().any(|fault| match fault.kind {
            koncet_domain::FaultKind::HealthcheckFailure => {
                let after = fault.after_s.unwrap_or(0) as i64 * 1_000;
                let duration = fault.duration_s.unwrap_or(1) as i64 * 1_000;
                self.clock.as_millis() >= after && self.clock.as_millis() < after + duration
            }
            _ => false,
        });

        let failure_reason = if !process_running {
            Some("process_not_running")
        } else if fault_active {
            Some("injected_healthcheck_failure")
        } else {
            None
        };

        match failure_reason {
            None => {
                self.health_consecutive_failures = 0;
                let latency = self.rng.health_latency_ms();
                self.emit(
                    Severity::Debug,
                    EventSource::Process,
                    event_types::HEALTHCHECK_PASSED,
                    json!({ "consecutive": 0, "latency_ms": latency }),
                );
                self.set_container_health(HealthStatus::Healthy, "probe_passed");
            }
            Some(reason) => {
                self.health_consecutive_failures =
                    self.health_consecutive_failures.saturating_add(1);
                self.emit(
                    Severity::Warn,
                    EventSource::Process,
                    event_types::HEALTHCHECK_FAILED,
                    json!({
                        "consecutive": self.health_consecutive_failures,
                        "reason": reason,
                    }),
                );
                self.log(
                    LogSource::Process,
                    Severity::Warn,
                    "health check failed",
                    &[
                        ("consecutive", json!(self.health_consecutive_failures)),
                        ("reason", json!(reason)),
                    ],
                );
                if self.health_consecutive_failures >= retries {
                    self.set_container_health(HealthStatus::Unhealthy, "healthcheck_failed");
                }
            }
        }
    }

    fn recompute_deployment(&mut self) {
        let (mut next, mut reason, mut health, old_status, old_health, old_reason) = {
            let (Some(container), Some(deployment)) =
                (self.container.as_ref(), self.deployment.as_ref())
            else {
                return;
            };
            let (next, reason, health) = derive_deployment(container, deployment, &self.scenario);
            (
                next,
                reason,
                health,
                deployment.status,
                deployment.health,
                deployment.primary_reason,
            )
        };
        if next == DeploymentStatus::Deploying && old_status == DeploymentStatus::Degraded {
            next = DeploymentStatus::Degraded;
            health = HealthStatus::Unhealthy;
            reason = reason
                .or(old_reason)
                .or(Some(FailureReason::ContainerFailed));
        }

        let Ok(changed) = self
            .deployment
            .as_mut()
            .expect("deployment exists")
            .set_status(next, reason, health, self.clock)
        else {
            return;
        };
        let new_health = self
            .deployment
            .as_ref()
            .map(|deployment| deployment.health)
            .unwrap_or(health);

        if old_health != new_health {
            self.emit(
                Severity::Info,
                EventSource::Deployment,
                event_types::DEPLOYMENT_HEALTH_CHANGED,
                json!({ "from": old_health.as_str(), "to": new_health.as_str() }),
            );
        }

        if changed {
            let severity = match next {
                DeploymentStatus::Failed => Severity::Error,
                DeploymentStatus::Degraded | DeploymentStatus::Stopped => Severity::Warn,
                _ => Severity::Info,
            };
            self.emit(
                severity,
                EventSource::Deployment,
                event_types::DEPLOYMENT_STATUS_CHANGED,
                json!({
                    "from": old_status.as_str(),
                    "to": next.as_str(),
                    "primary_reason": reason.map(FailureReason::as_str),
                }),
            );
            match next {
                DeploymentStatus::Degraded => {
                    self.emit(
                        Severity::Warn,
                        EventSource::Deployment,
                        event_types::DEPLOYMENT_DEGRADED,
                        json!({ "reason": reason.map(FailureReason::as_str) }),
                    );
                    self.log(
                        LogSource::Deployment,
                        Severity::Warn,
                        "deployment degraded",
                        &[("reason", json!(reason.map(FailureReason::as_str)))],
                    );
                }
                DeploymentStatus::Failed => {
                    self.emit(
                        Severity::Error,
                        EventSource::Deployment,
                        event_types::DEPLOYMENT_FAILED,
                        json!({ "reason": reason.map(FailureReason::as_str) }),
                    );
                    self.log(
                        LogSource::Deployment,
                        Severity::Error,
                        "deployment failed",
                        &[("reason", json!(reason.map(FailureReason::as_str)))],
                    );
                }
                DeploymentStatus::Healthy => {
                    self.log(
                        LogSource::Deployment,
                        Severity::Info,
                        "deployment healthy",
                        &[],
                    );
                }
                _ => {}
            }
        }

        if next == DeploymentStatus::Healthy || next == DeploymentStatus::Degraded {
            self.report_route(next, reason);
        }
    }

    fn report_route(&mut self, status: DeploymentStatus, reason: Option<FailureReason>) {
        if status == DeploymentStatus::Degraded
            && !matches!(
                reason,
                Some(FailureReason::FirewallDenied) | Some(FailureReason::RouteUnreachable)
            )
        {
            return;
        }
        let allowed = status == DeploymentStatus::Healthy;
        if self.route_reported == Some(allowed) {
            return;
        }
        let proxy_enabled = self.scenario.proxy.enabled;
        let host = self.scenario.proxy.host.clone();
        let path = self.scenario.proxy.path.clone();
        let target_port = self.scenario.proxy.target_port;
        let decision = self.scenario.firewall.evaluate(
            koncet_domain::TrafficSource::Proxy,
            koncet_domain::TrafficProtocol::Tcp,
            self.scenario.service.port,
        );
        self.route_reported = Some(allowed);

        if !proxy_enabled {
            self.emit(
                Severity::Warn,
                EventSource::Proxy,
                event_types::PROXY_ROUTE_VALIDATED,
                json!({
                    "host": host,
                    "path": path,
                    "target_port": target_port,
                    "warnings": ["proxy_disabled"],
                }),
            );
            self.emit(
                Severity::Warn,
                EventSource::Proxy,
                event_types::PROXY_PROBE_FAILED,
                json!({ "reason": "proxy_disabled", "status": Value::Null }),
            );
            return;
        }

        if allowed {
            self.emit(
                Severity::Info,
                EventSource::Proxy,
                event_types::PROXY_ROUTE_VALIDATED,
                json!({
                    "host": host,
                    "path": path,
                    "target_port": target_port,
                    "warnings": [],
                }),
            );
            self.emit(
                Severity::Debug,
                EventSource::Firewall,
                event_types::FIREWALL_RULE_EVALUATED,
                json!({
                    "rule_id": decision.rule_id,
                    "action": "allow",
                    "source": "proxy",
                    "port": self.scenario.service.port,
                }),
            );
            self.emit(
                Severity::Debug,
                EventSource::Proxy,
                event_types::PROXY_PROBE_SUCCEEDED,
                json!({ "status": 200, "latency_ms": 0 }),
            );
        } else {
            self.emit(
                Severity::Warn,
                EventSource::Proxy,
                event_types::PROXY_ROUTE_VALIDATED,
                json!({
                    "host": host,
                    "path": path,
                    "target_port": target_port,
                    "warnings": ["firewall_denied"],
                }),
            );
            self.emit(
                Severity::Warn,
                EventSource::Firewall,
                event_types::FIREWALL_DENIED,
                json!({
                    "source_alias": "proxy",
                    "port": self.scenario.service.port,
                    "protocol": "tcp",
                    "default_policy": decision.default_policy.as_str(),
                    "rule_id": decision.rule_id,
                }),
            );
            self.emit(
                Severity::Warn,
                EventSource::Proxy,
                event_types::PROXY_PROBE_FAILED,
                json!({ "reason": "firewall_denied", "status": 403 }),
            );
            self.log(
                LogSource::Proxy,
                Severity::Warn,
                "proxy request denied by firewall",
                &[("port", json!(self.scenario.service.port))],
            );
        }
    }

    fn metric_step(&mut self) {
        let Some(next) = self.next_metric_at else {
            return;
        };
        if self.clock < next {
            return;
        }
        let interval = (self.scenario.monitoring.sample_interval_s as i64 * 1_000).max(TICK_MS);
        self.next_metric_at = Some(next.saturating_add(interval));

        if !self.build.completed() {
            let percent = self.build_progress_percent();
            let build_id = self.build.id.to_string();
            self.metric(
                MetricEntity::Build,
                build_id,
                "progress_percent",
                percent,
                "%",
            );
        }

        let container_sample = self.container.as_ref().map(|container| {
            (
                container.id.to_string(),
                container.state,
                container.process.state,
                container.config.cpu_millicores,
                container.config.memory_mb,
                container.restart_count,
            )
        });
        if let Some((container_id, state, process, cpu_limit, memory_limit, restart_count)) =
            container_sample
        {
            if !matches!(state, ContainerState::Removed) {
                let running = state == ContainerState::Running && process == ProcessState::Running;
                let cpu = if running {
                    (cpu_limit as f64 * self.rng.cpu_percent() as f64 / 100.0).round()
                } else {
                    0.0
                };
                let memory = if running {
                    (memory_limit as f64 * self.rng.memory_percent() as f64 / 100.0).round()
                } else {
                    0.0
                };
                self.metric(
                    MetricEntity::Container,
                    container_id.clone(),
                    "cpu_millicores",
                    cpu,
                    "m",
                );
                self.metric(
                    MetricEntity::Container,
                    container_id.clone(),
                    "memory_mb",
                    memory,
                    "MiB",
                );
                if running {
                    let network_rx = self.rng.network_rx_kib() as f64;
                    let network_tx = self.rng.network_tx_kib() as f64;
                    self.metric(
                        MetricEntity::Container,
                        container_id.clone(),
                        "network_rx_kib",
                        network_rx,
                        "KiB/s",
                    );
                    self.metric(
                        MetricEntity::Container,
                        container_id.clone(),
                        "network_tx_kib",
                        network_tx,
                        "KiB/s",
                    );
                }
                self.metric(
                    MetricEntity::Container,
                    container_id,
                    "restart_count",
                    restart_count as f64,
                    "count",
                );
            }
        }

        let deployment_sample = self.deployment.as_ref().map(|deployment| {
            (
                deployment.id.to_string(),
                deployment.uptime_ms,
                deployment.healthy_since,
            )
        });
        if let Some((deployment_id, uptime_ms, healthy_since)) = deployment_sample {
            let live = uptime_ms
                + healthy_since
                    .map(|since| {
                        self.clock
                            .as_millis()
                            .saturating_sub(since.as_millis())
                            .max(0) as u64
                    })
                    .unwrap_or(0);
            self.metric(
                MetricEntity::Deployment,
                deployment_id,
                "uptime_ms",
                live as f64,
                "ms",
            );
        }
    }

    fn build_progress_percent(&self) -> f64 {
        let total: u64 = self.build.stages.iter().map(|stage| stage.planned_ms).sum();
        if total == 0 {
            return 100.0;
        }
        let completed: u64 = self
            .build
            .stages
            .iter()
            .filter(|stage| stage.status == BuildStageStatus::Succeeded)
            .map(|stage| stage.planned_ms)
            .sum();
        let current = self
            .build
            .current_stage()
            .and_then(|stage| {
                stage
                    .started_at
                    .map(|started| (self.clock.as_millis() - started.as_millis()).max(0) as u64)
            })
            .unwrap_or(0)
            .min(
                self.build
                    .current_stage()
                    .map(|stage| stage.planned_ms)
                    .unwrap_or(0),
            );
        (((completed + current) as f64 / total as f64) * 1000.0).round() / 10.0
    }

    fn limit_step(&mut self) {
        let max_ms = self.scenario.limits.max_sim_seconds as i64 * 1_000;
        if self.clock.as_millis() < max_ms || self.run.status.is_terminal() {
            return;
        }
        self.emit(
            Severity::Warn,
            EventSource::Engine,
            event_types::RUN_LIMIT_REACHED,
            json!({ "max_sim_seconds": self.scenario.limits.max_sim_seconds }),
        );
        let result = self.current_result();
        self.finish(RunStatus::Completed, result, None, true);
    }

    fn manual_restart(&mut self) -> Result<(), DomainError> {
        let container_id = {
            let Some(container) = self.container.as_mut() else {
                return Err(DomainError::conflict(
                    "container_missing",
                    "no container exists in this run",
                ));
            };
            if !matches!(container.state, ContainerState::Failed) {
                return Err(DomainError::conflict(
                    "invalid_container_transition",
                    "manual restart requires a failed container",
                ));
            }
            container.manual_restart(self.clock)?;
            container.id
        };
        self.emit(
            Severity::Info,
            EventSource::Container,
            event_types::CONTAINER_STARTING,
            json!({ "container_id": container_id }),
        );
        self.pending_restart_at = None;
        self.health_consecutive_failures = 0;
        self.next_health_check_at = None;
        self.startup_ready_at = Some(
            self.clock
                .saturating_add((self.scenario.service.startup_delay_s as i64 * 1_000).max(100)),
        );
        self.container_started_at = Some(self.clock);
        // A failed run is immutable, so commands are rejected before this point.
        Ok(())
    }

    fn check_connectivity(&mut self) {
        let proxy = self.scenario.proxy.clone();
        let port = self.scenario.service.port;
        let (result, reason, status) = if self.deployment.is_none() {
            ("unavailable", Some("image_missing"), Value::Null)
        } else {
            let container = self.container.as_ref();
            let running = container.is_some_and(|container| {
                container.state == ContainerState::Running && container.process.is_running()
            });
            if !running {
                ("unreachable", Some("route_unreachable"), json!(503))
            } else if self
                .container
                .as_ref()
                .is_some_and(|container| container.health != HealthStatus::Healthy)
            {
                ("unhealthy", Some("healthcheck_failed"), json!(503))
            } else if !proxy.enabled {
                ("unreachable", Some("proxy_disabled"), json!(502))
            } else {
                let decision = self.scenario.firewall.evaluate(
                    koncet_domain::TrafficSource::Proxy,
                    koncet_domain::TrafficProtocol::Tcp,
                    port,
                );
                if decision.allowed {
                    ("ok", None, json!(200))
                } else {
                    self.emit(
                        Severity::Warn,
                        EventSource::Firewall,
                        event_types::FIREWALL_DENIED,
                        json!({
                            "source_alias": "proxy",
                            "port": port,
                            "protocol": "tcp",
                            "default_policy": decision.default_policy.as_str(),
                            "rule_id": decision.rule_id,
                        }),
                    );
                    ("denied", Some("firewall_denied"), json!(403))
                }
            }
        };

        self.emit(
            Severity::Info,
            EventSource::Deployment,
            event_types::DEPLOYMENT_CONNECTIVITY_CHECKED,
            json!({
                "result": result,
                "reason": reason,
                "at_sim_ms": self.clock.as_millis(),
            }),
        );
        self.emit(
            Severity::Debug,
            EventSource::Proxy,
            event_types::PROXY_REQUEST_SIMULATED,
            json!({
                "host": proxy.host,
                "path": proxy.path,
                "result": result,
                "status": status,
            }),
        );
    }

    // ---------------------------------------------------------------------
    // Finalisation and output helpers
    // ---------------------------------------------------------------------

    fn current_result(&self) -> RunResult {
        if self.build.failed() {
            return RunResult::BuildFailed;
        }
        match self.deployment.as_ref().map(|deployment| deployment.status) {
            Some(DeploymentStatus::Healthy) => RunResult::Healthy,
            Some(DeploymentStatus::Degraded) => RunResult::Degraded,
            Some(DeploymentStatus::Failed) => RunResult::RuntimeFailed,
            _ => RunResult::Unknown,
        }
    }

    fn finish(
        &mut self,
        status: RunStatus,
        result: RunResult,
        reason: Option<FailureReason>,
        stop_runtime: bool,
    ) {
        if self.run.status.is_terminal() {
            return;
        }
        if stop_runtime {
            self.stop_runtime();
        }

        let warnings = self
            .events
            .iter()
            .filter(|event| event.severity == Severity::Warn)
            .count() as u64;
        let errors = self
            .events
            .iter()
            .filter(|event| event.severity == Severity::Error)
            .count() as u64;
        let container_restart_count = self
            .container
            .as_ref()
            .map(|container| container.restart_count)
            .unwrap_or(0);
        let summary = RunSummary {
            status,
            result,
            primary_reason: reason,
            sim_duration_ms: self.clock.as_millis().max(0) as u64,
            // Includes the final event emitted immediately below.
            event_count: self.events.len() as u64 + 1,
            log_count: self.logs.len() as u64,
            warning_count: warnings,
            error_count: errors,
            build_stage: self.build.failure.as_ref().map(|failure| failure.stage),
            container_restart_count,
            deployment_status: self.deployment.as_ref().map(|deployment| deployment.status),
        };

        let ended_at = TimestampUtc::from_millis(
            self.started_wall_ms
                .saturating_add(self.clock.as_millis().max(0)),
        );
        if self
            .run
            .finalise(status, result, summary, ended_at)
            .is_err()
        {
            return;
        }
        self.emit(
            Severity::Info,
            EventSource::Engine,
            event_types::RUN_FINALISED,
            json!({
                "status": status.as_str(),
                "result": result.as_str(),
                "reason": reason.map(FailureReason::as_str),
                "sim_duration_ms": self.clock.as_millis().max(0),
            }),
        );
    }

    fn stop_runtime(&mut self) {
        let now = self.clock;
        let mut events: Vec<(Severity, EventSource, &'static str, Value)> = Vec::new();
        if let Some(container) = self.container.as_mut() {
            match container.state {
                ContainerState::Running => {
                    let container_id = container.id;
                    let _ = container.process.exit(0, now);
                    if container.stop(now).is_ok() {
                        events.push((
                            Severity::Info,
                            EventSource::Container,
                            event_types::CONTAINER_STOPPING,
                            json!({ "container_id": container_id, "grace_s": 0 }),
                        ));
                    }
                    if container.mark_stopped(0, false, now).is_ok() {
                        events.push((
                            Severity::Info,
                            EventSource::Container,
                            event_types::CONTAINER_STOPPED,
                            json!({
                                "container_id": container_id,
                                "exit_code": 0,
                                "forced": false,
                            }),
                        ));
                    }
                }
                ContainerState::Starting => {
                    let container_id = container.id;
                    let _ = container.process.exit(0, now);
                    if container
                        .fail(FailureReason::UserStopped, Some(0), now)
                        .is_ok()
                    {
                        events.push((
                            Severity::Warn,
                            EventSource::Container,
                            event_types::CONTAINER_STOPPED,
                            json!({
                                "container_id": container_id,
                                "exit_code": 0,
                                "forced": true,
                            }),
                        ));
                    }
                }
                _ => {}
            }
        }
        for (severity, source, event_type, payload) in events {
            self.emit(severity, source, event_type, payload);
        }

        let mut stopped = None;
        if let Some(deployment) = self.deployment.as_mut() {
            if deployment.status != DeploymentStatus::Stopped {
                let old = deployment.status;
                if deployment
                    .set_status(
                        DeploymentStatus::Stopped,
                        Some(FailureReason::UserStopped),
                        HealthStatus::Unknown,
                        now,
                    )
                    .unwrap_or(false)
                {
                    stopped = Some((deployment.id, old));
                }
            }
        }
        if let Some((deployment_id, old)) = stopped {
            self.emit(
                Severity::Info,
                EventSource::Deployment,
                event_types::DEPLOYMENT_STOPPED,
                json!({
                    "deployment_id": deployment_id,
                    "reason": FailureReason::UserStopped.as_str(),
                }),
            );
            self.emit(
                Severity::Info,
                EventSource::Deployment,
                event_types::DEPLOYMENT_STATUS_CHANGED,
                json!({
                    "from": old.as_str(),
                    "to": "stopped",
                    "primary_reason": FailureReason::UserStopped.as_str(),
                }),
            );
        }
    }

    fn set_container_health(&mut self, next: HealthStatus, reason: &str) {
        let (from, changed) = {
            let Some(container) = self.container.as_mut() else {
                return;
            };
            let from = container.health;
            (from, container.set_health(next))
        };
        if !changed {
            return;
        }
        self.emit(
            Severity::Info,
            EventSource::Process,
            event_types::HEALTHCHECK_STATE_CHANGED,
            json!({
                "from": from.as_str(),
                "to": next.as_str(),
                "reason": reason,
            }),
        );
        self.log(
            LogSource::Process,
            if next == HealthStatus::Healthy {
                Severity::Info
            } else {
                Severity::Warn
            },
            "container health changed",
            &[("from", json!(from.as_str())), ("to", json!(next.as_str()))],
        );
    }

    fn emit(&mut self, severity: Severity, source: EventSource, event_type: &str, payload: Value) {
        self.event_seq = self.event_seq.saturating_add(1);
        let seq = self.event_seq;
        if self.run.record_event_seq(seq).is_err() {
            return;
        }
        let wall_time_ms = self.started_wall_ms.saturating_add(self.clock.as_millis());
        self.events.push(DomainEvent::new(
            self.run.id,
            seq,
            self.clock,
            wall_time_ms,
            event_type,
            severity,
            source,
            payload,
        ));
    }

    fn log(
        &mut self,
        source: LogSource,
        level: Severity,
        message: &str,
        context: &[(&str, Value)],
    ) {
        if self.logs.len() >= LOG_CAP {
            if !self.logs_truncated {
                self.logs_truncated = true;
                self.emit(
                    Severity::Warn,
                    EventSource::Engine,
                    event_types::ENGINE_LOGS_TRUNCATED,
                    json!({ "cap": LOG_CAP }),
                );
            }
            return;
        }
        self.log_seq = self.log_seq.saturating_add(1);
        let mut record = LogRecord::new(
            self.run.id,
            self.log_seq,
            self.clock,
            source,
            level,
            message,
        );
        for (key, value) in context {
            record.context.insert((*key).to_string(), value.clone());
        }
        self.logs.push(record);
    }

    fn metric(
        &mut self,
        entity_kind: MetricEntity,
        entity_id: String,
        metric: &str,
        value: f64,
        unit: &str,
    ) {
        if self.metrics.len() >= METRIC_CAP {
            if !self.metrics_truncated {
                self.metrics_truncated = true;
                self.emit(
                    Severity::Warn,
                    EventSource::Engine,
                    event_types::ENGINE_METRICS_TRUNCATED,
                    json!({ "cap": METRIC_CAP }),
                );
            }
            return;
        }
        self.metrics.push(MetricSample::new(
            self.run.id,
            self.clock,
            entity_kind,
            entity_id,
            metric,
            value,
            unit,
        ));
    }
}

fn derive_deployment(
    container: &Container,
    _deployment: &Deployment,
    scenario: &Scenario,
) -> (DeploymentStatus, Option<FailureReason>, HealthStatus) {
    match container.state {
        ContainerState::Failed | ContainerState::Removed => (
            DeploymentStatus::Failed,
            container
                .failure_reason
                .or(Some(FailureReason::ContainerFailed)),
            HealthStatus::Unhealthy,
        ),
        ContainerState::Created | ContainerState::Starting => {
            (DeploymentStatus::Deploying, None, HealthStatus::Starting)
        }
        ContainerState::Stopping | ContainerState::Stopped => (
            DeploymentStatus::Stopped,
            Some(FailureReason::UserStopped),
            HealthStatus::Unknown,
        ),
        ContainerState::Running => {
            if container.process.state == ProcessState::Failed {
                return (
                    DeploymentStatus::Degraded,
                    Some(FailureReason::ContainerFailed),
                    HealthStatus::Unhealthy,
                );
            }
            match container.health {
                HealthStatus::Healthy => {
                    if !scenario.proxy.enabled {
                        return (
                            DeploymentStatus::Degraded,
                            Some(FailureReason::RouteUnreachable),
                            HealthStatus::Unhealthy,
                        );
                    }
                    let decision = scenario.firewall.evaluate(
                        koncet_domain::TrafficSource::Proxy,
                        koncet_domain::TrafficProtocol::Tcp,
                        scenario.service.port,
                    );
                    if decision.allowed {
                        (DeploymentStatus::Healthy, None, HealthStatus::Healthy)
                    } else {
                        (
                            DeploymentStatus::Degraded,
                            Some(FailureReason::FirewallDenied),
                            HealthStatus::Unhealthy,
                        )
                    }
                }
                HealthStatus::Unhealthy => (
                    DeploymentStatus::Degraded,
                    Some(FailureReason::HealthcheckFailed),
                    HealthStatus::Unhealthy,
                ),
                HealthStatus::Unknown | HealthStatus::Starting => {
                    (DeploymentStatus::Deploying, None, HealthStatus::Starting)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koncet_domain::{
        ProjectId, ProjectTemplate, ScenarioPreset, SourceSnapshot, SourceTree, TimestampUtc,
    };

    fn snapshot(project_id: ProjectId) -> SourceSnapshot {
        let tree = SourceTree::new(ProjectTemplate::StaticWeb.seed_files()).unwrap();
        let mut snapshot = SourceSnapshot::capture(project_id, &tree, TimestampUtc::from_millis(0));
        snapshot.id = koncet_domain::SourceSnapshotId::new();
        snapshot
    }

    fn engine_for(preset: ScenarioPreset) -> SimulationEngine {
        let project_id = ProjectId::new();
        let scenario = preset.scenario();
        SimulationEngine::start(SimulationInputs {
            run_id: RunId::new(),
            project_id,
            scenario_id: ScenarioId::new(),
            scenario,
            source_snapshot: snapshot(project_id),
            seed: 12_345,
            started_at: TimestampUtc::from_millis(1_700_000_000_000),
            app_version: "0.1.0".into(),
            engine_version: "0.1.0".into(),
        })
        .unwrap()
    }

    #[test]
    fn happy_path_becomes_healthy_and_stops_cleanly() {
        let mut engine = engine_for(ScenarioPreset::HappyPath);
        engine.advance_ticks(500);
        let deployment = engine.deployment().expect("deployment exists");
        assert_eq!(deployment.status, DeploymentStatus::Healthy);
        assert_eq!(engine.container().unwrap().health, HealthStatus::Healthy);
        assert!(engine.image().is_some());
        engine.submit_command(RunCommand::Stop).unwrap();
        assert_eq!(engine.run().status, RunStatus::Completed);
        assert_eq!(engine.run().result, RunResult::Healthy);
        assert_eq!(
            engine.deployment().unwrap().status,
            DeploymentStatus::Stopped
        );
        assert_seqs_are_consecutive(engine.events());
    }

    #[test]
    fn build_failure_produces_no_image_or_deployment() {
        let mut engine = engine_for(ScenarioPreset::BuildFailure);
        engine.run_to_completion(1_000);
        let output = engine.into_output();
        assert_eq!(output.snapshot.run.status, RunStatus::Completed);
        assert_eq!(output.snapshot.run.result, RunResult::BuildFailed);
        assert!(output.snapshot.image.is_none());
        assert!(output.snapshot.container.is_none());
        assert!(output.snapshot.deployment.is_none());
        let build = &output.snapshot.build;
        assert_eq!(
            build.stage(BuildStageName::Build).unwrap().status,
            BuildStageStatus::Failed
        );
        assert_eq!(
            build.stage(BuildStageName::Package).unwrap().status,
            BuildStageStatus::Skipped
        );
        assert_eq!(
            output.snapshot.run.summary.unwrap().primary_reason,
            Some(FailureReason::BuildFailed)
        );
    }

    #[test]
    fn blocked_port_is_degraded_and_connectivity_is_denied() {
        let mut engine = engine_for(ScenarioPreset::BlockedPort);
        engine.advance_ticks(500);
        let deployment = engine.deployment().unwrap();
        assert_eq!(deployment.status, DeploymentStatus::Degraded);
        assert_eq!(
            deployment.primary_reason,
            Some(FailureReason::FirewallDenied)
        );
        engine
            .submit_command(RunCommand::CheckConnectivity)
            .unwrap();
        let checked = engine
            .events()
            .iter()
            .rev()
            .find(|event| event.event_type == event_types::DEPLOYMENT_CONNECTIVITY_CHECKED)
            .unwrap();
        assert_eq!(checked.payload["result"], "denied");
        assert_eq!(checked.payload["reason"], "firewall_denied");
    }

    #[test]
    fn crash_loop_exhausts_restarts() {
        let mut engine = engine_for(ScenarioPreset::CrashLoop);
        engine.run_to_completion(2_000);
        let output = engine.into_output();
        assert_eq!(output.snapshot.run.status, RunStatus::Failed);
        assert_eq!(output.snapshot.run.result, RunResult::RuntimeFailed);
        assert_eq!(
            output.snapshot.run.summary.unwrap().primary_reason,
            Some(FailureReason::RestartExhausted)
        );
        let container = output.snapshot.container.unwrap();
        assert_eq!(container.state, ContainerState::Failed);
        assert_eq!(container.restart_count, 3);
        assert_eq!(
            container.failure_reason,
            Some(FailureReason::RestartExhausted)
        );
        assert_seqs_are_consecutive(&output.events);
    }

    #[test]
    fn identical_inputs_produce_identical_output() {
        let project_id = ProjectId::new();
        let scenario_id = ScenarioId::new();
        let run_id = RunId::new();
        let scenario = ScenarioPreset::HappyPath.scenario();
        let source = snapshot(project_id);

        let make = || {
            SimulationEngine::start(SimulationInputs {
                run_id,
                project_id,
                scenario_id,
                scenario: scenario.clone(),
                source_snapshot: source.clone(),
                seed: 99,
                started_at: TimestampUtc::from_millis(1_700_000_000_000),
                app_version: "0.1.0".into(),
                engine_version: "0.1.0".into(),
            })
            .unwrap()
        };

        let mut a = make();
        let mut b = make();
        a.advance_ticks(500);
        b.advance_ticks(500);
        a.submit_command(RunCommand::Stop).unwrap();
        b.submit_command(RunCommand::Stop).unwrap();
        let a = a.into_output();
        let b = b.into_output();
        assert_eq!(a.events, b.events);
        assert_eq!(a.logs, b.logs);
        assert_eq!(a.metrics, b.metrics);
        assert_eq!(a.snapshot, b.snapshot);
    }

    #[test]
    fn running_container_emits_cpu_memory_and_network_rates() {
        let mut engine = engine_for(ScenarioPreset::HappyPath);
        engine.advance_ticks(500);
        for metric in [
            "cpu_millicores",
            "memory_mb",
            "network_rx_kib",
            "network_tx_kib",
        ] {
            assert!(
                engine
                    .metrics()
                    .iter()
                    .any(|sample| sample.metric == metric && sample.value >= 0.0),
                "missing metric {metric}"
            );
        }
    }

    #[test]
    fn pause_freezes_virtual_time() {
        let mut engine = engine_for(ScenarioPreset::HappyPath);
        engine.advance_ticks(100);
        let before = engine.clock();
        engine.submit_command(RunCommand::Pause).unwrap();
        engine.advance_ticks(100);
        assert_eq!(engine.clock(), before);
        engine.submit_command(RunCommand::Resume).unwrap();
        engine.advance_ticks(10);
        assert!(engine.clock() > before);
    }

    #[test]
    fn run_stops_at_the_scenario_limit() {
        let mut engine = engine_for(ScenarioPreset::HappyPath);
        engine.scenario.limits.max_sim_seconds = 60;
        engine.advance_to(SimTime::from_secs(70));
        assert!(engine.run().status.is_terminal());
        assert_eq!(engine.run().status, RunStatus::Completed);
        assert_eq!(engine.run().result, RunResult::Healthy);
    }

    fn assert_seqs_are_consecutive(events: &[DomainEvent]) {
        for (index, event) in events.iter().enumerate() {
            assert_eq!(event.seq, index as u64 + 1);
        }
    }
}
