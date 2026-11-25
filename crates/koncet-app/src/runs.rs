//! Run lifecycle: start, advance, control, finalise, and engine-output flushes.

use koncet_domain::{
    DomainError, ProjectId, RunCommand, RunId, ScenarioId, SourceSnapshot, TimestampUtc, MAX_SEED,
};
use koncet_sim::{SimulationEngine, SimulationInputs};
use koncet_storage::{PersistedRun, RunState};

use crate::app::App;
use crate::error::{AppError, AppResult};
use crate::session::{RunFrame, RunSession, RunUpdate};

/// Start request from the UI. `seed` overrides the scenario seed policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRunInput {
    pub project_id: ProjectId,
    pub scenario_id: ScenarioId,
    pub seed: Option<String>,
}

/// Durable run identity returned as soon as the run row is committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHandle {
    pub run_id: RunId,
    pub project_id: ProjectId,
    pub scenario_id: ScenarioId,
    pub scenario_name: String,
    pub seed: String,
    pub started_at: TimestampUtc,
}

impl App {
    /// Captures snapshots, persists the run, and starts the in-process engine.
    pub fn start_run(&mut self, input: StartRunInput) -> AppResult<RunHandle> {
        self.check_active()?;
        let project = self.get_project(input.project_id)?;
        project.project.ensure_editable()?;
        let scenario = self.get_scenario(input.scenario_id)?;
        if scenario.project_id != project.project.id {
            return Err(AppError::conflict(
                "scenario_project_mismatch",
                "the scenario belongs to another project",
            ));
        }

        let report = scenario
            .scenario
            .validate_for_run(&project.files, Some(project.project.template_id));
        if !report.is_valid() {
            return Err(AppError::Validation(report));
        }

        let seed = match input.seed {
            Some(text) => parse_seed(&text)?,
            None => scenario.scenario.resolve_seed(rand::random::<u64>())?,
        };
        if seed > MAX_SEED {
            return Err(AppError::Domain(DomainError::validation(
                "out_of_range",
                "/seed",
                "seed must be a 63-bit non-negative integer",
            )));
        }

        let now = self.clock.now();
        let snapshot = SourceSnapshot::capture(project.project.id, &project.files, now);
        let run_id = RunId::new();
        let mut engine = SimulationEngine::start(SimulationInputs {
            run_id,
            project_id: project.project.id,
            scenario_id: scenario.id,
            scenario: scenario.scenario.clone(),
            source_snapshot: snapshot.clone(),
            seed,
            started_at: now,
            app_version: self.app_version().to_string(),
            engine_version: self.engine_version().to_string(),
        })?;
        let initial_events = engine.drain_events();
        self.database
            .start_run(engine.run(), &snapshot, &initial_events)?;

        let handle = RunHandle {
            run_id,
            project_id: project.project.id,
            scenario_id: scenario.id,
            scenario_name: engine.run().scenario_name.clone(),
            seed: seed.to_string(),
            started_at: now,
        };
        self.active = Some(RunSession::new(engine));
        tracing::info!(
            run = %run_id,
            project = %project.project.id,
            scenario = %scenario.id,
            seed,
            "started run"
        );
        Ok(handle)
    }

    /// Advances the active session by wall-clock milliseconds at its current speed.
    pub fn advance_active(&mut self, elapsed_ms: u64) -> AppResult<Option<RunUpdate>> {
        let Some(session) = self.active.as_mut() else {
            return Ok(None);
        };
        session.advance(elapsed_ms);
        self.flush_active()
    }

    /// Drains and persists engine output, then returns the same batch for display.
    ///
    /// Events and commands are durable before the update is returned. A failed
    /// transaction is retained and retried by the next flush. When the engine is
    /// terminal, the session is checkpointed and released.
    pub fn flush_active(&mut self) -> AppResult<Option<RunUpdate>> {
        if let Some(pending) = self.pending.take() {
            if let Err(error) = self.persist_update(&pending) {
                self.pending = Some(pending);
                return Err(error);
            }
            self.finish_update(&pending)?;
            return Ok(Some(pending));
        }

        let Some(session) = self.active.as_mut() else {
            return Ok(None);
        };
        let events = session.drain_events();
        let logs = session.drain_logs();
        let metrics = session.drain_metrics();
        let commands = session.commands_to_persist().to_vec();
        if events.is_empty() && logs.is_empty() && metrics.is_empty() && commands.is_empty() {
            return Ok(None);
        }
        let frame = session.frame();
        let finalised = session.is_terminal();
        let update = RunUpdate {
            frame,
            events,
            logs,
            metrics,
            commands,
            finalised,
        };
        if let Err(error) = self.persist_update(&update) {
            self.pending = Some(update);
            return Err(error);
        }
        self.finish_update(&update)?;
        Ok(Some(update))
    }

    fn persist_update(&mut self, update: &RunUpdate) -> AppResult<()> {
        let state = RunState {
            run: &update.frame.run,
            build: &update.frame.build,
            image: update.frame.image.as_ref(),
            container: update.frame.container.as_ref(),
            deployment: update.frame.deployment.as_ref(),
            at: koncet_domain::SimTime::from_millis(update.frame.clock_ms),
        };
        self.database.persist_session(
            &state,
            &update.events,
            &update.logs,
            &update.metrics,
            &update.commands,
        )?;
        Ok(())
    }

    fn finish_update(&mut self, update: &RunUpdate) -> AppResult<()> {
        if let Some(session) = self.active.as_mut() {
            session.mark_commands_persisted();
        }
        if update.finalised {
            self.database.checkpoint()?;
            self.active = None;
        }
        Ok(())
    }

    pub fn pause_run(&mut self, run_id: RunId) -> AppResult<RunUpdate> {
        self.submit_command(run_id, RunCommand::Pause)
    }

    pub fn resume_run(&mut self, run_id: RunId) -> AppResult<RunUpdate> {
        self.submit_command(run_id, RunCommand::Resume)
    }

    pub fn set_run_speed(&mut self, run_id: RunId, speed: u8) -> AppResult<RunUpdate> {
        self.submit_command(run_id, RunCommand::SetSpeed { speed })
    }

    pub fn restart_container(&mut self, run_id: RunId) -> AppResult<RunUpdate> {
        self.submit_command(run_id, RunCommand::RestartContainer)
    }

    pub fn check_connectivity(&mut self, run_id: RunId) -> AppResult<RunUpdate> {
        self.submit_command(run_id, RunCommand::CheckConnectivity)
    }

    /// Graceful stop: finalises the run as completed.
    pub fn stop_run(&mut self, run_id: RunId) -> AppResult<RunUpdate> {
        let update = self.submit_command(run_id, RunCommand::Stop)?;
        self.require_finalised(update)
    }

    /// Cancels the run: finalises as cancelled.
    pub fn cancel_run(&mut self, run_id: RunId) -> AppResult<RunUpdate> {
        let update = self.submit_command(run_id, RunCommand::Cancel)?;
        self.require_finalised(update)
    }

    pub fn active_run(&self) -> Option<&RunSession> {
        self.active.as_ref()
    }

    /// Snapshot for a late subscriber without draining output.
    pub fn session_frame(&self) -> Option<RunFrame> {
        self.active.as_ref().map(RunSession::frame)
    }

    /// Historical run detail. Never starts the engine.
    pub fn get_run(&self, run_id: RunId) -> AppResult<PersistedRun> {
        self.database
            .load_run(run_id)?
            .ok_or_else(|| AppError::not_found("run"))
    }

    fn submit_command(&mut self, run_id: RunId, command: RunCommand) -> AppResult<RunUpdate> {
        let Some(session) = self.active.as_mut() else {
            return Err(AppError::conflict(
                "run_not_active",
                "the run is not active in this process",
            ));
        };
        if session.run_id() != run_id {
            return Err(AppError::conflict(
                "run_not_active",
                "another run is active",
            ));
        }
        session.submit(command)?;
        self.flush_active()?.ok_or_else(|| {
            AppError::internal(
                "session_lost",
                "the active session disappeared while flushing",
            )
        })
    }

    fn require_finalised(&self, update: RunUpdate) -> AppResult<RunUpdate> {
        if update.finalised {
            Ok(update)
        } else {
            Err(AppError::internal(
                "run_not_finalised",
                "the engine accepted a terminal command without finalising",
            ))
        }
    }
}

/// Parses a decimal seed and enforces the 63-bit bound.
pub fn parse_seed(text: &str) -> AppResult<u64> {
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.chars().all(|character| character.is_ascii_digit()) {
        return Err(AppError::Domain(DomainError::validation(
            "invalid_format",
            "/seed",
            "seed must be a non-negative decimal integer",
        )));
    }
    let seed = trimmed.parse::<u64>().map_err(|_| {
        AppError::Domain(DomainError::validation(
            "out_of_range",
            "/seed",
            "seed is too large",
        ))
    })?;
    if seed > MAX_SEED {
        return Err(AppError::Domain(DomainError::validation(
            "out_of_range",
            "/seed",
            "seed must be a 63-bit non-negative integer",
        )));
    }
    Ok(seed)
}
