//! Historical reads. These never instantiate the engine or mutate a run.

use koncet_domain::{
    DomainEvent, LogRecord, MetricEntity, MetricSample, ProjectId, RunId, SourceSnapshot,
    SourceSnapshotId,
};
use koncet_storage::{LogFilter, RunHistoryFilter, RunHistoryItem};

use crate::app::App;
use crate::error::{AppError, AppResult};

impl App {
    pub fn list_run_history(&self, filter: &RunHistoryFilter) -> AppResult<Vec<RunHistoryItem>> {
        Ok(self.database.list_run_history(filter)?)
    }

    pub fn count_runs(&self, project_id: Option<ProjectId>) -> AppResult<u64> {
        Ok(self.database.count_runs(project_id)?)
    }

    pub fn delete_run(&mut self, run_id: RunId) -> AppResult<()> {
        if self.active_run_id() == Some(run_id) {
            return Err(AppError::conflict(
                "active_run_exists",
                "stop the active run before deleting it",
            ));
        }
        self.database.delete_run(run_id)?;
        Ok(())
    }

    /// Deletes all runs, or all runs of one project. Refuses to remove an active run.
    pub fn delete_run_history(&mut self, project_id: Option<ProjectId>) -> AppResult<u64> {
        if let Some(session) = self.active.as_ref() {
            let active_project = session.run().project_id;
            if project_id.is_none_or(|project| project == active_project) {
                return Err(AppError::conflict(
                    "active_run_exists",
                    "stop the active run before clearing history",
                ));
            }
        }
        Ok(self.database.delete_run_history(project_id)?)
    }

    pub fn list_events(
        &self,
        run_id: RunId,
        after_seq: u64,
        include_debug: bool,
        limit: u32,
    ) -> AppResult<Vec<DomainEvent>> {
        Ok(self
            .database
            .list_events(run_id, after_seq, include_debug, limit)?)
    }

    pub fn count_events(&self, run_id: RunId) -> AppResult<u64> {
        Ok(self.database.count_events(run_id)?)
    }

    pub fn list_logs(&self, run_id: RunId, filter: &LogFilter) -> AppResult<Vec<LogRecord>> {
        Ok(self.database.list_logs(run_id, filter)?)
    }

    pub fn tail_logs(&self, run_id: RunId, limit: u32) -> AppResult<Vec<LogRecord>> {
        Ok(self.database.tail_logs(run_id, limit)?)
    }

    pub fn count_logs(&self, run_id: RunId) -> AppResult<u64> {
        Ok(self.database.count_logs(run_id)?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn list_metrics(
        &self,
        run_id: RunId,
        entity: MetricEntity,
        entity_id: &str,
        metric: Option<&str>,
        from_ms: i64,
        to_ms: i64,
        limit: u32,
    ) -> AppResult<Vec<MetricSample>> {
        Ok(self
            .database
            .list_metrics(run_id, entity, entity_id, metric, from_ms, to_ms, limit)?)
    }

    pub fn count_metrics(&self, run_id: RunId) -> AppResult<u64> {
        Ok(self.database.count_metrics(run_id)?)
    }

    pub fn get_source_snapshot(&self, snapshot_id: SourceSnapshotId) -> AppResult<SourceSnapshot> {
        self.database
            .load_source_snapshot(snapshot_id)?
            .ok_or_else(|| AppError::not_found("source snapshot"))
    }
}
