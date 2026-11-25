//! The one active simulation session.
//!
//! The session owns the engine. It never touches storage or the wall clock; the
//! application layer drains output, persists it, then publishes the same batch.

use koncet_domain::{
    Build, Container, Deployment, DomainEvent, LogRecord, MetricSample, RecordedCommand, RunId,
    RunSummary, SimTime, SimulatedImage, SimulationRun,
};
use koncet_sim::{SimulationEngine, TICK_MS};

/// Upper bound for one `advance` call, protecting the UI from a stalled timer.
pub const MAX_TICKS_PER_ADVANCE: u64 = 6_000;

/// Compact state for rendering the run workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct RunFrame {
    pub run: SimulationRun,
    pub build: Build,
    pub image: Option<SimulatedImage>,
    pub container: Option<Container>,
    pub deployment: Option<Deployment>,
    pub clock_ms: i64,
    pub speed: u8,
    pub paused: bool,
}

/// One drained batch of engine output, already durable when returned.
#[derive(Debug, Clone, PartialEq)]
pub struct RunUpdate {
    pub frame: RunFrame,
    pub events: Vec<DomainEvent>,
    pub logs: Vec<LogRecord>,
    pub metrics: Vec<MetricSample>,
    pub commands: Vec<RecordedCommand>,
    pub finalised: bool,
}

pub struct RunSession {
    engine: SimulationEngine,
    persisted_commands: usize,
}

impl RunSession {
    pub fn new(engine: SimulationEngine) -> Self {
        Self {
            engine,
            persisted_commands: 0,
        }
    }

    pub fn run_id(&self) -> RunId {
        self.engine.run().id
    }

    pub fn run(&self) -> &SimulationRun {
        self.engine.run()
    }

    pub fn summary(&self) -> Option<&RunSummary> {
        self.engine.run().summary.as_ref()
    }

    pub fn is_terminal(&self) -> bool {
        self.engine.is_terminal()
    }

    pub fn is_paused(&self) -> bool {
        self.engine.is_paused()
    }

    pub fn speed(&self) -> u8 {
        self.engine.speed()
    }

    pub fn clock(&self) -> SimTime {
        self.engine.clock()
    }

    /// Advances by wall-clock milliseconds multiplied by the selected speed.
    ///
    /// Returns the number of simulated ticks processed. Paused and terminal
    /// sessions never advance.
    pub fn advance(&mut self, elapsed_ms: u64) -> u64 {
        if elapsed_ms == 0 || self.engine.is_paused() || self.engine.is_terminal() {
            return 0;
        }
        let ticks = elapsed_ms.saturating_mul(self.engine.speed() as u64) / TICK_MS as u64;
        self.engine.advance_ticks(ticks.min(MAX_TICKS_PER_ADVANCE))
    }

    pub fn submit(
        &mut self,
        command: koncet_domain::RunCommand,
    ) -> Result<(), koncet_domain::DomainError> {
        self.engine.submit_command(command)
    }

    /// Commands recorded since the last successful persistence.
    pub fn commands_to_persist(&self) -> &[RecordedCommand] {
        &self.engine.commands()[self.persisted_commands..]
    }

    pub fn mark_commands_persisted(&mut self) {
        self.persisted_commands = self.engine.commands().len();
    }

    pub fn drain_events(&mut self) -> Vec<DomainEvent> {
        self.engine.drain_events()
    }

    pub fn drain_logs(&mut self) -> Vec<LogRecord> {
        self.engine.drain_logs()
    }

    pub fn drain_metrics(&mut self) -> Vec<MetricSample> {
        self.engine.drain_metrics()
    }

    pub fn frame(&self) -> RunFrame {
        let engine = &self.engine;
        RunFrame {
            run: engine.run().clone(),
            build: engine.build().clone(),
            image: engine.image().cloned(),
            container: engine.container().cloned(),
            deployment: engine.deployment().cloned(),
            clock_ms: engine.clock().as_millis(),
            speed: engine.speed(),
            paused: engine.is_paused(),
        }
    }
}
