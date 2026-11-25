//! Application composition root.
//!
//! `App` owns the single database writer, the optional active run session, the
//! wall clock, and the version metadata recorded into runs. Use cases live in
//! sibling modules; the FFI layer is a thin adapter over this surface.

use std::sync::Arc;

use koncet_domain::RunId;
use koncet_storage::{Database, OpenOptions, StoragePaths};

use crate::clock::{Clock, SystemClock};
use crate::error::{AppError, AppResult};
use crate::session::{RunFrame, RunSession, RunUpdate};

const BUILD_COMMIT: &str = match option_env!("KONCET_BUILD_COMMIT") {
    Some(commit) => commit,
    None => "development",
};

const BUILD_DATE: &str = match option_env!("KONCET_BUILD_DATE") {
    Some(date) => date,
    None => "local build",
};

/// Immutable startup information for the About and diagnostics screens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    pub app_version: String,
    pub engine_version: String,
    pub build_commit: String,
    pub build_date: String,
    pub schema_version: i64,
    pub platform: String,
    pub data_root: String,
}

pub struct App {
    pub(crate) database: Database,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) active: Option<RunSession>,
    /// Drained output kept until its transaction commits. A failed flush is
    /// retried on the next call instead of losing events.
    pub(crate) pending: Option<RunUpdate>,
    reconciled_runs: Vec<RunId>,
    app_version: String,
    engine_version: String,
}

impl App {
    /// Opens the default XDG database.
    pub fn open_default(clock: Arc<dyn Clock>) -> AppResult<Self> {
        Self::open(StoragePaths::resolve()?, clock)
    }

    /// Opens the default XDG database with explicit recovery options.
    pub fn open_default_with(clock: Arc<dyn Clock>, options: OpenOptions) -> AppResult<Self> {
        Self::open_with(StoragePaths::resolve()?, options, clock)
    }

    /// Opens a database rooted at an explicit directory. Used by tests and tools.
    pub fn open_at(
        root: std::path::PathBuf,
        clock: Arc<dyn Clock>,
        options: OpenOptions,
    ) -> AppResult<Self> {
        Self::open_with(StoragePaths::at(root), options, clock)
    }

    /// Opens a database with default options and reconciles interrupted runs.
    pub fn open(paths: StoragePaths, clock: Arc<dyn Clock>) -> AppResult<Self> {
        Self::open_with(paths, OpenOptions::default(), clock)
    }

    pub fn open_with(
        paths: StoragePaths,
        options: OpenOptions,
        clock: Arc<dyn Clock>,
    ) -> AppResult<Self> {
        let mut database = Database::open_with(paths, options)?;
        let reconciled_runs = database.reconcile_interrupted_runs(clock.now())?;
        if !reconciled_runs.is_empty() {
            tracing::warn!(
                runs = reconciled_runs.len(),
                "reconciled runs left active by an abnormal exit"
            );
        }
        Ok(Self {
            database,
            clock,
            active: None,
            pending: None,
            reconciled_runs,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            engine_version: koncet_domain::ENGINE_VERSION.to_string(),
        })
    }

    pub fn info(&self) -> AppResult<AppInfo> {
        Ok(AppInfo {
            app_version: self.app_version.clone(),
            engine_version: self.engine_version.clone(),
            build_commit: BUILD_COMMIT.to_string(),
            build_date: BUILD_DATE.to_string(),
            schema_version: self.database.schema_version()?,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            data_root: self.database.paths().root().to_string_lossy().to_string(),
        })
    }

    /// Runs recovered from a previous process, in reconciliation order.
    pub fn reconciled_runs(&self) -> &[RunId] {
        &self.reconciled_runs
    }

    pub fn active_run_id(&self) -> Option<RunId> {
        self.active.as_ref().map(RunSession::run_id)
    }

    pub fn active_frame(&self) -> Option<RunFrame> {
        self.active.as_ref().map(RunSession::frame)
    }

    pub fn clock_now(&self) -> koncet_domain::TimestampUtc {
        self.clock.now()
    }

    /// Stops an active run, then checkpoints and releases the database.
    ///
    /// The UI must obtain explicit confirmation before calling this; the core
    /// still performs a graceful stop so no run is silently abandoned.
    pub fn shutdown(mut self) -> AppResult<()> {
        if self.active.is_some() {
            self.flush_active()?;
        }
        if let Some(run_id) = self.active.as_ref().map(RunSession::run_id) {
            self.stop_run(run_id)?;
        }
        self.database.shutdown()?;
        Ok(())
    }

    pub(crate) fn check_active(&self) -> AppResult<()> {
        if self.active.is_some() {
            Err(AppError::conflict(
                "active_run_exists",
                "another simulation run is already active",
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn app_version(&self) -> &str {
        &self.app_version
    }

    pub(crate) fn engine_version(&self) -> &str {
        &self.engine_version
    }
}

/// Shared clock helper for tests and embedders that use a fixed clock.
#[allow(dead_code)]
pub(crate) fn system_clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}
