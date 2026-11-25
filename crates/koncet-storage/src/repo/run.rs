//! Run history, state snapshots, events, logs, metrics, and recorded commands.

use std::collections::BTreeMap;

use koncet_domain::event_types;
use koncet_domain::{
    Build, BuildFailure, BuildProfile, BuildStage, BuildStageName, BuildStageStatus, Container,
    ContainerId, Deployment, DomainEvent, EventSource, FailureReason, ImageId, ImageLayer,
    LogRecord, LogSource, MetricEntity, MetricSample, Process, ProjectId, RecordedCommand,
    RunCommand, RunId, RunResult, RunStatus, RunSummary, ScenarioId, Severity, SimTime,
    SimulatedImage, SimulationRun, SourceSnapshot, TimestampUtc,
};
use rusqlite::{params, params_from_iter, types::Value as SqlValue, OptionalExtension, Row};
use serde_json::Value;

use crate::codec;
use crate::db::Database;
use crate::error::{StorageError, StorageResult};
use crate::repo::project::clamp_limit;

/// Time-stamped state persisted at pause, stop, and finalisation boundaries.
///
/// Kept independent of `koncet-sim` so storage stays below the session layer.
pub struct RunState<'a> {
    pub run: &'a SimulationRun,
    pub build: &'a Build,
    pub image: Option<&'a SimulatedImage>,
    pub container: Option<&'a Container>,
    pub deployment: Option<&'a Deployment>,
    pub at: SimTime,
}

/// Reconstructed run detail.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistedRun {
    pub run: SimulationRun,
    pub build: Option<Build>,
    pub image: Option<SimulatedImage>,
    pub container: Option<Container>,
    pub deployment: Option<Deployment>,
    pub command_trace: Vec<RecordedCommand>,
}

/// Row for the history list.
#[derive(Debug, Clone, PartialEq)]
pub struct RunHistoryItem {
    pub run_id: RunId,
    pub project_id: ProjectId,
    pub project_name: String,
    pub scenario_id: ScenarioId,
    pub scenario_name: String,
    pub status: RunStatus,
    pub result: RunResult,
    pub started_at: TimestampUtc,
    pub ended_at: Option<TimestampUtc>,
    pub sim_duration_ms: u64,
    pub summary: Option<RunSummary>,
}

/// Keyset pagination for run history, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunHistoryFilter {
    pub project_id: Option<ProjectId>,
    /// Exclusive cursor: `(started_at, run_id)` of the last row on the previous page.
    pub before: Option<(TimestampUtc, RunId)>,
    pub limit: u32,
}

/// Filters for log queries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogFilter {
    pub after_id: Option<i64>,
    pub level: Option<Severity>,
    pub source: Option<LogSource>,
    /// Bounded `LIKE` search over the message, per the v0.1 contract.
    pub search: Option<String>,
    pub limit: u32,
}

impl Database {
    /// Starts a run atomically: snapshot, run row, and the initial event batch.
    pub fn start_run(
        &mut self,
        run: &SimulationRun,
        snapshot: &SourceSnapshot,
        initial_events: &[DomainEvent],
    ) -> StorageResult<()> {
        if snapshot.project_id != run.project_id {
            return Err(StorageError::Validation(
                "snapshot and run belong to different projects".into(),
            ));
        }
        let tx = self.conn_mut().transaction()?;
        crate::repo::project::insert_snapshot_rows(&tx, snapshot)?;
        insert_run_row(&tx, run)?;
        append_events_tx(&tx, run.id, initial_events)?;
        let stored = stored_event_seq(&tx, run.id)?;
        if stored != run.last_event_seq as i64 {
            return Err(StorageError::Validation(format!(
                "run expected {0} events but the initial batch ended at {stored}",
                run.last_event_seq
            )));
        }
        tx.commit()?;
        Ok(())
    }

    /// Appends a contiguous batch of domain events and advances `last_event_seq`.
    pub fn append_events(&mut self, run_id: RunId, events: &[DomainEvent]) -> StorageResult<()> {
        if events.is_empty() {
            return Ok(());
        }
        let tx = self.conn_mut().transaction()?;
        append_events_tx(&tx, run_id, events)?;
        tx.commit()?;
        Ok(())
    }

    /// Appends a contiguous batch of structured logs for one run.
    pub fn append_logs(&mut self, logs: &[LogRecord]) -> StorageResult<()> {
        if logs.is_empty() {
            return Ok(());
        }
        let run_id = single_run(logs.iter().map(|log| log.run_id))?;
        let tx = self.conn_mut().transaction()?;
        append_logs_tx(&tx, run_id, logs)?;
        tx.commit()?;
        Ok(())
    }

    /// Appends metric samples. Duplicate `(entity, metric, sim_time)` rows are ignored.
    pub fn append_metrics(&mut self, metrics: &[MetricSample]) -> StorageResult<()> {
        if metrics.is_empty() {
            return Ok(());
        }
        let run_id = single_run(metrics.iter().map(|metric| metric.run_id))?;
        let tx = self.conn_mut().transaction()?;
        append_metrics_tx(&tx, run_id, metrics)?;
        tx.commit()?;
        Ok(())
    }

    /// Persists user commands in order. Commands are part of the replay contract.
    pub fn append_commands(
        &mut self,
        run_id: RunId,
        commands: &[RecordedCommand],
    ) -> StorageResult<()> {
        if commands.is_empty() {
            return Ok(());
        }
        let tx = self.conn_mut().transaction()?;
        append_commands_tx(&tx, run_id, commands)?;
        tx.commit()?;
        Ok(())
    }

    /// Flushes engine output and upserts the state snapshot tables in one transaction.
    pub fn persist_run_state(
        &mut self,
        state: &RunState<'_>,
        events: &[DomainEvent],
        logs: &[LogRecord],
        metrics: &[MetricSample],
    ) -> StorageResult<()> {
        self.persist_session(state, events, logs, metrics, &[])
    }

    /// Like [`Database::persist_run_state`] but records new commands in the same
    /// transaction. Commands are only part of the first flush that observes them.
    pub fn persist_session(
        &mut self,
        state: &RunState<'_>,
        events: &[DomainEvent],
        logs: &[LogRecord],
        metrics: &[MetricSample],
        commands: &[RecordedCommand],
    ) -> StorageResult<()> {
        let run_id = state.run.id;
        if !events.is_empty() {
            if let Some(other) = events.iter().find(|event| event.run_id != run_id) {
                return Err(StorageError::Validation(format!(
                    "event for run {} was passed to run {run_id}",
                    other.run_id
                )));
            }
        }
        if commands.iter().any(|command| command.seq == 0) {
            return Err(StorageError::Validation(
                "recorded commands must start at sequence 1".into(),
            ));
        }

        let tx = self.conn_mut().transaction()?;
        append_events_tx(&tx, run_id, events)?;
        if !logs.is_empty() {
            append_logs_tx(&tx, run_id, logs)?;
        }
        if !metrics.is_empty() {
            append_metrics_tx(&tx, run_id, metrics)?;
        }
        if !commands.is_empty() {
            append_commands_tx(&tx, run_id, commands)?;
        }

        let stored = stored_event_seq(&tx, run_id)?;
        if stored != state.run.last_event_seq as i64 {
            return Err(StorageError::Validation(format!(
                "run {} has {} events but its last sequence is {}; drain the engine before persisting",
                state.run.id,
                stored,
                state.run.last_event_seq
            )));
        }

        update_run_row(&tx, state.run)?;
        upsert_build(&tx, state.run, state.build)?;
        if let Some(image) = state.image {
            insert_image(&tx, image, state.build.id)?;
        }
        if let Some(container) = state.container {
            upsert_container(&tx, container)?;
        }
        if let Some(deployment) = state.deployment {
            upsert_deployment(&tx, deployment, state.at)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Marks runs left active by an abnormal exit as cancelled.
    ///
    /// The appended `run.cancelled` event carries reason `app_terminated`; the
    /// current result is preserved so the history still explains what was observed.
    pub fn reconcile_interrupted_runs(&mut self, at: TimestampUtc) -> StorageResult<Vec<RunId>> {
        let mut statement = self.conn().prepare(
            "SELECT id, sim_duration_ms, last_event_seq FROM runs
             WHERE status IN ('running','paused') ORDER BY started_at, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                codec::parse_id::<RunId>(&row.get::<_, String>(0)?, "run id")?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        let mut interrupted = Vec::new();
        for row in rows {
            interrupted.push(row?);
        }
        drop(statement);

        if interrupted.is_empty() {
            return Ok(Vec::new());
        }

        let tx = self.conn_mut().transaction()?;
        let mut reconciled = Vec::with_capacity(interrupted.len());
        for (run_id, sim_duration_ms, last_event_seq) in interrupted {
            let event = DomainEvent::new(
                run_id,
                (last_event_seq.max(0) as u64) + 1,
                SimTime::from_millis(sim_duration_ms.max(0)),
                at.as_millis(),
                event_types::RUN_CANCELLED,
                Severity::Warn,
                EventSource::Engine,
                serde_json::json!({
                    "reason": FailureReason::AppTerminated.as_str(),
                    "reconciled": true,
                }),
            );
            append_events_tx(&tx, run_id, &[event])?;
            tx.execute(
                "UPDATE runs SET status = 'cancelled', ended_at = ?2, end_reason = ?3
                 WHERE id = ?1",
                params![
                    run_id.to_string(),
                    at.as_millis(),
                    FailureReason::AppTerminated.as_str(),
                ],
            )?;
            reconciled.push(run_id);
        }
        tx.commit()?;
        Ok(reconciled)
    }

    /// Updates only run-level status fields, used for pause/resume acknowledgements.
    pub fn update_run_status(&mut self, run: &SimulationRun) -> StorageResult<()> {
        let changed = self.conn().execute(
            "UPDATE runs SET status = ?2, result = ?3, sim_duration_ms = ?4, ended_at = ?5
             WHERE id = ?1",
            params![
                run.id.to_string(),
                run.status.as_str(),
                run.result.as_str(),
                run.sim_duration_ms as i64,
                run.ended_at.map(TimestampUtc::as_millis),
            ],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound("run"));
        }
        Ok(())
    }

    pub fn load_run(&self, run_id: RunId) -> StorageResult<Option<PersistedRun>> {
        let run = self
            .conn()
            .query_row(
                "SELECT id, project_id, scenario_id, scenario_name, seed, source_snapshot_id,
                        scenario_schema_version, scenario_snapshot_json, status, result,
                        started_at, ended_at, sim_duration_ms, last_event_seq,
                        summary_json, app_version, engine_version
                 FROM runs WHERE id = ?1",
                [run_id.to_string()],
                map_run,
            )
            .optional()?;
        let Some(run) = run else {
            return Ok(None);
        };

        let build = load_build(self.conn(), run_id)?;
        let image = match build.as_ref().and_then(|build| build.image_id) {
            Some(image_id) => load_image_by_id(self.conn(), image_id)?,
            None => load_latest_image(self.conn(), run_id)?,
        };
        let container = load_container(self.conn(), run_id)?;
        let deployment = load_deployment(self.conn(), run_id)?;
        let command_trace = load_commands(self.conn(), run_id)?;

        Ok(Some(PersistedRun {
            run,
            build,
            image,
            container,
            deployment,
            command_trace,
        }))
    }

    pub fn list_run_history(
        &self,
        filter: &RunHistoryFilter,
    ) -> StorageResult<Vec<RunHistoryItem>> {
        let limit = clamp_limit(if filter.limit == 0 {
            crate::repo::project::DEFAULT_LIST_LIMIT
        } else {
            filter.limit
        });
        let project = filter.project_id.map(|id| id.to_string());
        let before_at = filter.before.as_ref().map(|(at, _)| at.as_millis());
        let before_id = filter.before.as_ref().map(|(_, id)| id.to_string());
        let mut statement = self.conn().prepare(
            "SELECT r.id, r.project_id, p.name, r.scenario_id, r.scenario_name,
                    r.status, r.result, r.started_at, r.ended_at, r.sim_duration_ms,
                    r.summary_json
             FROM runs r JOIN projects p ON p.id = r.project_id
             WHERE (?1 IS NULL OR r.project_id = ?1)
               AND (?2 IS NULL OR r.started_at < ?2
                    OR (r.started_at = ?2 AND r.id < ?3))
             ORDER BY r.started_at DESC, r.id DESC
             LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![project, before_at, before_id, limit as i64],
            map_history_item,
        )?;
        let mut history = Vec::new();
        for row in rows {
            history.push(row?);
        }
        Ok(history)
    }

    pub fn count_runs(&self, project_id: Option<ProjectId>) -> StorageResult<u64> {
        let count: i64 = match project_id {
            Some(project_id) => self.conn().query_row(
                "SELECT COUNT(*) FROM runs WHERE project_id = ?1",
                [project_id.to_string()],
                |row| row.get(0),
            )?,
            None => self
                .conn()
                .query_row("SELECT COUNT(*) FROM runs", [], |row| row.get(0))?,
        };
        Ok(count.max(0) as u64)
    }

    pub fn delete_run(&mut self, run_id: RunId) -> StorageResult<()> {
        let changed = self
            .conn()
            .execute("DELETE FROM runs WHERE id = ?1", [run_id.to_string()])?;
        if changed == 0 {
            return Err(StorageError::NotFound("run"));
        }
        Ok(())
    }

    /// Explicit history deletion; returns the number of runs removed.
    pub fn delete_run_history(&mut self, project_id: Option<ProjectId>) -> StorageResult<u64> {
        let removed = match project_id {
            Some(project_id) => self.conn().execute(
                "DELETE FROM runs WHERE project_id = ?1",
                [project_id.to_string()],
            )?,
            None => self.conn().execute("DELETE FROM runs", [])?,
        };
        Ok(removed as u64)
    }

    /// Timeline events ordered by sequence, oldest first.
    pub fn list_events(
        &self,
        run_id: RunId,
        after_seq: u64,
        include_debug: bool,
        limit: u32,
    ) -> StorageResult<Vec<DomainEvent>> {
        let mut statement = self.conn().prepare(
            "SELECT run_id, seq, sim_time_ms, wall_time_ms, category, event_type,
                    severity, source, payload_json, schema_version
             FROM run_events
             WHERE run_id = ?1 AND seq > ?2 AND (?3 = 1 OR severity <> 'debug')
             ORDER BY seq ASC LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![
                run_id.to_string(),
                after_seq as i64,
                if include_debug { 1 } else { 0 },
                clamp_limit(limit) as i64
            ],
            map_event,
        )?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn count_events(&self, run_id: RunId) -> StorageResult<u64> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM run_events WHERE run_id = ?1",
            [run_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    /// Log page ordered by insertion id, oldest first.
    pub fn list_logs(&self, run_id: RunId, filter: &LogFilter) -> StorageResult<Vec<LogRecord>> {
        let limit = clamp_limit(if filter.limit == 0 {
            crate::repo::project::DEFAULT_LIST_LIMIT
        } else {
            filter.limit
        });
        let mut sql = String::from(
            "SELECT id, run_id, seq, sim_time_ms, source, level, message, context_json
             FROM log_lines WHERE run_id = ?1",
        );
        let mut values: Vec<SqlValue> = vec![SqlValue::Text(run_id.to_string())];
        if let Some(after) = filter.after_id {
            sql.push_str(" AND id > ?");
            values.push(SqlValue::Integer(after));
        }
        if let Some(level) = filter.level {
            sql.push_str(" AND level = ?");
            values.push(SqlValue::Text(level.as_str().to_string()));
        }
        if let Some(source) = filter.source {
            sql.push_str(" AND source = ?");
            values.push(SqlValue::Text(source.as_str().to_string()));
        }
        if let Some(search) = filter
            .search
            .as_ref()
            .filter(|text| !text.trim().is_empty())
        {
            sql.push_str(" AND message LIKE ?");
            let escaped = search
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            values.push(SqlValue::Text(format!("%{escaped}%")));
            sql.push_str(" ESCAPE '\\'");
        }
        sql.push_str(" ORDER BY id ASC LIMIT ?");
        values.push(SqlValue::Integer(limit as i64));

        let mut statement = self.conn().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), map_log)?;
        let mut logs = Vec::new();
        for row in rows {
            logs.push(row?);
        }
        Ok(logs)
    }

    /// Most recent `limit` log lines, returned oldest first for stable rendering.
    pub fn tail_logs(&self, run_id: RunId, limit: u32) -> StorageResult<Vec<LogRecord>> {
        let limit = clamp_limit(limit);
        let mut statement = self.conn().prepare(
            "SELECT id, run_id, seq, sim_time_ms, source, level, message, context_json
             FROM (SELECT id, run_id, seq, sim_time_ms, source, level, message, context_json
                   FROM log_lines WHERE run_id = ?1 ORDER BY id DESC LIMIT ?2)
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map(params![run_id.to_string(), limit as i64], map_log)?;
        let mut logs = Vec::new();
        for row in rows {
            logs.push(row?);
        }
        Ok(logs)
    }

    pub fn count_logs(&self, run_id: RunId) -> StorageResult<u64> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM log_lines WHERE run_id = ?1",
            [run_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u64)
    }

    /// Metric series for one entity and metric, oldest first.
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
    ) -> StorageResult<Vec<MetricSample>> {
        let limit = clamp_limit(limit);
        let mut sql = String::from(
            "SELECT run_id, sim_time_ms, entity_type, entity_id, metric, value, unit
             FROM metric_samples
             WHERE run_id = ?1 AND entity_type = ?2 AND entity_id = ?3
               AND sim_time_ms >= ?4 AND sim_time_ms <= ?5",
        );
        let mut values: Vec<SqlValue> = vec![
            SqlValue::Text(run_id.to_string()),
            SqlValue::Text(entity.as_str().to_string()),
            SqlValue::Text(entity_id.to_string()),
            SqlValue::Integer(from_ms),
            SqlValue::Integer(to_ms),
        ];
        if let Some(metric) = metric {
            sql.push_str(" AND metric = ?");
            values.push(SqlValue::Text(metric.to_string()));
        }
        sql.push_str(" ORDER BY sim_time_ms ASC LIMIT ?");
        values.push(SqlValue::Integer(limit as i64));

        let mut statement = self.conn().prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), map_metric)?;
        let mut metrics = Vec::new();
        for row in rows {
            metrics.push(row?);
        }
        Ok(metrics)
    }

    pub fn count_metrics(&self, run_id: RunId) -> StorageResult<u64> {
        let count: i64 = self.conn().query_row(
            "SELECT COUNT(*) FROM metric_samples WHERE run_id = ?1",
            [run_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as u64)
    }
}

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

fn insert_run_row(conn: &rusqlite::Connection, run: &SimulationRun) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO runs(id, project_id, scenario_id, scenario_name, scenario_schema_version,
                          scenario_snapshot_json, source_snapshot_id, seed, app_version,
                          engine_version, status, result, started_at, ended_at,
                          sim_duration_ms, last_event_seq, end_reason, summary_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            run.id.to_string(),
            run.project_id.to_string(),
            run.scenario_id.to_string(),
            run.scenario_name,
            run.scenario_schema_version as i64,
            run.scenario_json,
            run.source_snapshot_id.to_string(),
            run.seed.to_string(),
            run.app_version,
            run.engine_version,
            run.status.as_str(),
            run.result.as_str(),
            run.started_at.unwrap_or(run.created_at).as_millis(),
            run.ended_at.map(TimestampUtc::as_millis),
            run.sim_duration_ms as i64,
            run.last_event_seq as i64,
            run.summary
                .as_ref()
                .and_then(|summary| summary.primary_reason)
                .map(FailureReason::as_str),
            run.summary.as_ref().map(codec::to_json).transpose()?,
        ],
    )?;
    Ok(())
}

fn update_run_row(conn: &rusqlite::Connection, run: &SimulationRun) -> StorageResult<()> {
    let changed = conn.execute(
        "UPDATE runs SET status = ?2, result = ?3, sim_duration_ms = ?4, ended_at = ?5,
                         last_event_seq = MAX(last_event_seq, ?6), end_reason = ?7, summary_json = ?8
         WHERE id = ?1",
        params![
            run.id.to_string(),
            run.status.as_str(),
            run.result.as_str(),
            run.sim_duration_ms as i64,
            run.ended_at.map(TimestampUtc::as_millis),
            run.last_event_seq as i64,
            run.summary
                .as_ref()
                .and_then(|summary| summary.primary_reason)
                .map(FailureReason::as_str),
            run.summary.as_ref().map(codec::to_json).transpose()?,
        ],
    )?;
    if changed == 0 {
        return Err(StorageError::NotFound("run"));
    }
    Ok(())
}

fn append_events_tx(
    conn: &rusqlite::Connection,
    run_id: RunId,
    events: &[DomainEvent],
) -> StorageResult<()> {
    if events.is_empty() {
        return Ok(());
    }
    let start = stored_event_seq(conn, run_id)? + 1;
    let mut statement = conn.prepare(
        "INSERT INTO run_events(run_id, seq, sim_time_ms, wall_time_ms, category,
                                event_type, severity, source, payload_json, schema_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    let mut last = start - 1;
    for (expected, event) in (start..).zip(events) {
        if event.run_id != run_id {
            return Err(StorageError::Validation("event batch mixes runs".into()));
        }
        if event.seq as i64 != expected {
            return Err(StorageError::Validation(format!(
                "event sequence gap: expected {expected}, got {}",
                event.seq
            )));
        }
        statement.execute(params![
            run_id.to_string(),
            event.seq as i64,
            event.sim_time_ms,
            event.wall_time_ms,
            event.category,
            event.event_type,
            event.severity.as_str(),
            event.source.as_str(),
            codec::to_json(&event.payload)?,
            event.schema_version as i64,
        ])?;
        last = event.seq as i64;
    }
    conn.execute(
        "UPDATE runs SET last_event_seq = MAX(last_event_seq, ?2) WHERE id = ?1",
        params![run_id.to_string(), last],
    )?;
    Ok(())
}

fn stored_event_seq(conn: &rusqlite::Connection, run_id: RunId) -> StorageResult<i64> {
    let value: i64 = conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) FROM run_events WHERE run_id = ?1",
        [run_id.to_string()],
        |row| row.get(0),
    )?;
    Ok(value)
}

fn append_logs_tx(
    conn: &rusqlite::Connection,
    run_id: RunId,
    logs: &[LogRecord],
) -> StorageResult<()> {
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) FROM log_lines WHERE run_id = ?1",
        [run_id.to_string()],
        |row| row.get(0),
    )?;
    let start = current + 1;
    let mut statement = conn.prepare(
        "INSERT INTO log_lines(run_id, seq, sim_time_ms, source, level, message, context_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for (expected, record) in (start..).zip(logs) {
        if record.run_id != run_id {
            return Err(StorageError::Validation("log batch mixes runs".into()));
        }
        if record.seq as i64 != expected {
            return Err(StorageError::Validation(format!(
                "log sequence gap: expected {expected}, got {}",
                record.seq
            )));
        }
        let context = if record.context.is_empty() {
            None
        } else {
            Some(codec::to_json(&record.context)?)
        };
        statement.execute(params![
            run_id.to_string(),
            record.seq as i64,
            record.sim_time_ms,
            record.source.as_str(),
            record.level.as_str(),
            record.message,
            context,
        ])?;
    }
    Ok(())
}

fn append_metrics_tx(
    conn: &rusqlite::Connection,
    run_id: RunId,
    metrics: &[MetricSample],
) -> StorageResult<()> {
    let mut statement = conn.prepare(
        "INSERT OR IGNORE INTO metric_samples(run_id, entity_type, entity_id, metric,
                                              sim_time_ms, value, unit)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for sample in metrics {
        if sample.run_id != run_id {
            return Err(StorageError::Validation("metric batch mixes runs".into()));
        }
        statement.execute(params![
            run_id.to_string(),
            sample.entity_kind.as_str(),
            sample.entity_id,
            sample.metric,
            sample.sim_time_ms,
            sample.value,
            sample.unit,
        ])?;
    }
    Ok(())
}

fn append_commands_tx(
    conn: &rusqlite::Connection,
    run_id: RunId,
    commands: &[RecordedCommand],
) -> StorageResult<()> {
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(seq), 0) FROM run_commands WHERE run_id = ?1",
        [run_id.to_string()],
        |row| row.get(0),
    )?;
    let start = current + 1;
    let mut statement = conn.prepare(
        "INSERT INTO run_commands(run_id, seq, sim_time_ms, command_json)
         VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (expected, command) in (start..).zip(commands) {
        if command.seq as i64 != expected {
            return Err(StorageError::Validation(format!(
                "command sequence gap: expected {expected}, got {}",
                command.seq
            )));
        }
        statement.execute(params![
            run_id.to_string(),
            command.seq as i64,
            command.at.as_millis(),
            codec::to_json(&command.command)?,
        ])?;
    }
    Ok(())
}

fn upsert_build(
    conn: &rusqlite::Connection,
    run: &SimulationRun,
    build: &Build,
) -> StorageResult<()> {
    let started = build
        .stages
        .iter()
        .filter_map(|stage| stage.started_at)
        .map(SimTime::as_millis)
        .min();
    let finished = if build.completed() {
        build
            .stages
            .iter()
            .filter_map(|stage| stage.finished_at)
            .map(SimTime::as_millis)
            .max()
    } else {
        None
    };
    let status = if build.failed() {
        "failed"
    } else if build.succeeded() {
        "succeeded"
    } else if build
        .stages
        .iter()
        .any(|stage| stage.status == BuildStageStatus::Running)
    {
        "running"
    } else {
        "queued"
    };
    conn.execute(
        "INSERT INTO builds(id, run_id, ordinal, profile, status, image_id,
                            started_sim_ms, finished_sim_ms, failure_json, created_at)
         VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
            profile = excluded.profile,
            status = excluded.status,
            image_id = excluded.image_id,
            started_sim_ms = excluded.started_sim_ms,
            finished_sim_ms = excluded.finished_sim_ms,
            failure_json = excluded.failure_json",
        params![
            build.id.to_string(),
            run.id.to_string(),
            build.profile.as_str(),
            status,
            build.image_id.map(|id| id.to_string()),
            started,
            finished,
            build.failure.as_ref().map(codec::to_json).transpose()?,
            run.created_at.as_millis(),
        ],
    )?;

    for stage in &build.stages {
        conn.execute(
            "INSERT INTO build_stages(build_id, ordinal, name, status, planned_ms,
                                      started_sim_ms, finished_sim_ms, failure_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(build_id, ordinal) DO UPDATE SET
                name = excluded.name,
                status = excluded.status,
                planned_ms = excluded.planned_ms,
                started_sim_ms = excluded.started_sim_ms,
                finished_sim_ms = excluded.finished_sim_ms,
                failure_json = excluded.failure_json",
            params![
                build.id.to_string(),
                stage.kind.ordinal() as i64,
                stage.kind.as_str(),
                stage.status.as_str(),
                stage.planned_ms as i64,
                stage.started_at.map(SimTime::as_millis),
                stage.finished_at.map(SimTime::as_millis),
                stage.failure.as_ref().map(codec::to_json).transpose()?,
            ],
        )?;
    }
    Ok(())
}

fn insert_image(
    conn: &rusqlite::Connection,
    image: &SimulatedImage,
    build_id: koncet_domain::BuildId,
) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO images(id, run_id, build_id, repository, tag, digest, size_bytes,
                            layers_json, profile, source_hash, engine_version, created_sim_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO NOTHING",
        params![
            image.id.to_string(),
            image.run_id.to_string(),
            build_id.to_string(),
            image.repository,
            image.tag,
            image.digest,
            image.size_bytes as i64,
            codec::to_json(&image.layers)?,
            image.profile.as_str(),
            image.source_hash,
            image.engine_version,
            image.created_at.as_millis(),
        ],
    )?;
    Ok(())
}

fn upsert_container(conn: &rusqlite::Connection, container: &Container) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO containers(id, run_id, image_id, name, state, health, restart_count,
                                config_json, process_json, created_sim_ms, started_sim_ms,
                                stopped_sim_ms, exit_code, failure_reason)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(id) DO UPDATE SET
            state = excluded.state,
            health = excluded.health,
            restart_count = excluded.restart_count,
            config_json = excluded.config_json,
            process_json = excluded.process_json,
            started_sim_ms = excluded.started_sim_ms,
            stopped_sim_ms = excluded.stopped_sim_ms,
            exit_code = excluded.exit_code,
            failure_reason = excluded.failure_reason",
        params![
            container.id.to_string(),
            container.run_id.to_string(),
            container.image_id.to_string(),
            container.name,
            container.state.as_str(),
            container.health.as_str(),
            container.restart_count as i64,
            codec::to_json(&container.config)?,
            codec::to_json(&container.process)?,
            container.created_at.as_millis(),
            container.started_at.map(SimTime::as_millis),
            container.stopped_at.map(SimTime::as_millis),
            container.exit_code,
            container.failure_reason.map(FailureReason::as_str),
        ],
    )?;
    Ok(())
}

fn upsert_deployment(
    conn: &rusqlite::Connection,
    deployment: &Deployment,
    at: SimTime,
) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO deployments(id, run_id, image_id, container_id, service_name, status,
                                 health, primary_reason, revision, uptime_ms,
                                 healthy_since_sim_ms, created_sim_ms, updated_sim_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            health = excluded.health,
            primary_reason = excluded.primary_reason,
            uptime_ms = excluded.uptime_ms,
            healthy_since_sim_ms = excluded.healthy_since_sim_ms,
            updated_sim_ms = excluded.updated_sim_ms,
            revision = deployments.revision + 1
         WHERE deployments.status <> excluded.status
            OR deployments.health <> excluded.health
            OR deployments.primary_reason IS NOT excluded.primary_reason
            OR deployments.uptime_ms <> excluded.uptime_ms
            OR deployments.healthy_since_sim_ms IS NOT excluded.healthy_since_sim_ms",
        params![
            deployment.id.to_string(),
            deployment.run_id.to_string(),
            deployment.image_id.to_string(),
            deployment.container_id.to_string(),
            deployment.service_name,
            deployment.status.as_str(),
            deployment.health.as_str(),
            deployment.primary_reason.map(FailureReason::as_str),
            deployment.uptime_ms as i64,
            deployment.healthy_since.map(SimTime::as_millis),
            deployment.created_at.as_millis(),
            at.as_millis(),
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Loaders
// ---------------------------------------------------------------------------

fn load_build(conn: &rusqlite::Connection, run_id: RunId) -> StorageResult<Option<Build>> {
    let header = conn
        .query_row(
            "SELECT id, run_id, profile, image_id, failure_json
             FROM builds WHERE run_id = ?1 ORDER BY ordinal LIMIT 1",
            [run_id.to_string()],
            |row| {
                Ok((
                    codec::parse_id::<koncet_domain::BuildId>(
                        &row.get::<_, String>(0)?,
                        "build id",
                    )
                    .map_err(to_sql_error)?,
                    codec::parse_enum::<BuildProfile>(&row.get::<_, String>(2)?, "build profile")
                        .map_err(to_sql_error)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((id, profile, image_id, failure_json)) = header else {
        return Ok(None);
    };

    let mut statement = conn.prepare(
        "SELECT name, status, planned_ms, started_sim_ms, finished_sim_ms, failure_json
         FROM build_stages WHERE build_id = ?1 ORDER BY ordinal",
    )?;
    let rows = statement.query_map([id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut stages = Vec::new();
    for row in rows {
        let (name, status, planned_ms, started, finished, failure) = row?;
        let kind: BuildStageName = codec::parse_state(&name, "build stage")?;
        let status: BuildStageStatus = codec::parse_state(&status, "build stage status")?;
        let failure: Option<BuildFailure> = failure
            .as_deref()
            .map(|text| codec::from_json(text, "build failure"))
            .transpose()?;
        stages.push(BuildStage {
            kind,
            status,
            planned_ms: codec::non_negative(planned_ms, "planned duration")?,
            started_at: started.map(SimTime::from_millis),
            finished_at: finished.map(SimTime::from_millis),
            actual_ms: started
                .zip(finished)
                .map(|(started, finished)| (finished - started).max(0) as u64),
            failure,
        });
    }
    let failure: Option<BuildFailure> = failure_json
        .as_deref()
        .map(|text| codec::from_json(text, "build failure"))
        .transpose()?;
    Ok(Some(Build {
        id,
        run_id,
        profile,
        stages,
        image_id: image_id
            .as_deref()
            .map(|text| codec::parse_id::<ImageId>(text, "image id"))
            .transpose()?,
        failure,
    }))
}

fn load_image_by_id(
    conn: &rusqlite::Connection,
    image_id: ImageId,
) -> StorageResult<Option<SimulatedImage>> {
    conn.query_row(
        "SELECT id, run_id, repository, tag, digest, size_bytes, layers_json,
                profile, source_hash, engine_version, created_sim_ms
         FROM images WHERE id = ?1",
        [image_id.to_string()],
        map_image,
    )
    .optional()
    .map_err(StorageError::from)
}

fn load_latest_image(
    conn: &rusqlite::Connection,
    run_id: RunId,
) -> StorageResult<Option<SimulatedImage>> {
    conn.query_row(
        "SELECT id, run_id, repository, tag, digest, size_bytes, layers_json,
                profile, source_hash, engine_version, created_sim_ms
         FROM images WHERE run_id = ?1 ORDER BY created_sim_ms ASC, id ASC LIMIT 1",
        [run_id.to_string()],
        map_image,
    )
    .optional()
    .map_err(StorageError::from)
}

fn load_container(conn: &rusqlite::Connection, run_id: RunId) -> StorageResult<Option<Container>> {
    conn.query_row(
        "SELECT id, run_id, image_id, name, state, health, restart_count, config_json,
                process_json, created_sim_ms, started_sim_ms, stopped_sim_ms, exit_code,
                failure_reason
         FROM containers WHERE run_id = ?1 ORDER BY created_sim_ms, id LIMIT 1",
        [run_id.to_string()],
        map_container,
    )
    .optional()
    .map_err(StorageError::from)
}

fn load_deployment(
    conn: &rusqlite::Connection,
    run_id: RunId,
) -> StorageResult<Option<Deployment>> {
    conn.query_row(
        "SELECT id, run_id, image_id, container_id, service_name, status, health,
                primary_reason, created_sim_ms, healthy_since_sim_ms, uptime_ms
         FROM deployments WHERE run_id = ?1 ORDER BY created_sim_ms, id LIMIT 1",
        [run_id.to_string()],
        map_deployment,
    )
    .optional()
    .map_err(StorageError::from)
}

fn load_commands(
    conn: &rusqlite::Connection,
    run_id: RunId,
) -> StorageResult<Vec<RecordedCommand>> {
    let mut statement = conn.prepare(
        "SELECT seq, sim_time_ms, command_json FROM run_commands
         WHERE run_id = ?1 ORDER BY seq",
    )?;
    let rows = statement.query_map([run_id.to_string()], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut commands = Vec::new();
    for row in rows {
        let (seq, at, json) = row?;
        let command: RunCommand = codec::from_json(&json, "run command")?;
        commands.push(RecordedCommand {
            seq: codec::non_negative(seq, "command sequence")?,
            at: SimTime::from_millis(at),
            command,
        });
    }
    Ok(commands)
}

// ---------------------------------------------------------------------------
// Row mappers
// ---------------------------------------------------------------------------

fn map_run(row: &Row<'_>) -> rusqlite::Result<SimulationRun> {
    let started_at = TimestampUtc::from_millis(row.get(10)?);
    Ok(SimulationRun {
        id: codec::parse_id(&row.get::<_, String>(0)?, "run id").map_err(to_sql_error)?,
        project_id: codec::parse_id(&row.get::<_, String>(1)?, "project id")
            .map_err(to_sql_error)?,
        scenario_id: codec::parse_id(&row.get::<_, String>(2)?, "scenario id")
            .map_err(to_sql_error)?,
        scenario_name: row.get(3)?,
        seed: row
            .get::<_, String>(4)?
            .parse::<u64>()
            .map_err(|_| storage_sql_error("seed is not an unsigned integer"))?,
        source_snapshot_id: codec::parse_id(&row.get::<_, String>(5)?, "snapshot id")
            .map_err(to_sql_error)?,
        scenario_schema_version: codec::positive(row.get(6)?, "scenario schema version")
            .map_err(to_sql_error)?,
        scenario_json: row.get(7)?,
        status: codec::parse_state(&row.get::<_, String>(8)?, "run status")
            .map_err(to_sql_error)?,
        result: codec::parse_state(&row.get::<_, String>(9)?, "run result")
            .map_err(to_sql_error)?,
        created_at: started_at,
        started_at: Some(started_at),
        ended_at: row
            .get::<_, Option<i64>>(11)?
            .map(TimestampUtc::from_millis),
        sim_duration_ms: codec::non_negative(row.get(12)?, "simulation duration")
            .map_err(to_sql_error)?,
        last_event_seq: codec::non_negative(row.get(13)?, "last event sequence")
            .map_err(to_sql_error)?,
        summary: row
            .get::<_, Option<String>>(14)?
            .as_deref()
            .map(|text| codec::from_json::<RunSummary>(text, "run summary"))
            .transpose()
            .map_err(to_sql_error)?,
        app_version: row.get(15)?,
        engine_version: row.get(16)?,
    })
}

fn map_history_item(row: &Row<'_>) -> rusqlite::Result<RunHistoryItem> {
    let summary: Option<RunSummary> = row
        .get::<_, Option<String>>(10)?
        .as_deref()
        .map(|text| codec::from_json(text, "run summary"))
        .transpose()
        .map_err(to_sql_error)?;
    Ok(RunHistoryItem {
        run_id: codec::parse_id(&row.get::<_, String>(0)?, "run id").map_err(to_sql_error)?,
        project_id: codec::parse_id(&row.get::<_, String>(1)?, "project id")
            .map_err(to_sql_error)?,
        project_name: row.get(2)?,
        scenario_id: codec::parse_id(&row.get::<_, String>(3)?, "scenario id")
            .map_err(to_sql_error)?,
        scenario_name: row.get(4)?,
        status: codec::parse_state(&row.get::<_, String>(5)?, "run status")
            .map_err(to_sql_error)?,
        result: codec::parse_state(&row.get::<_, String>(6)?, "run result")
            .map_err(to_sql_error)?,
        started_at: TimestampUtc::from_millis(row.get(7)?),
        ended_at: row.get::<_, Option<i64>>(8)?.map(TimestampUtc::from_millis),
        sim_duration_ms: codec::non_negative(row.get(9)?, "simulation duration")
            .map_err(to_sql_error)?,
        summary,
    })
}

fn map_event(row: &Row<'_>) -> rusqlite::Result<DomainEvent> {
    Ok(DomainEvent {
        run_id: codec::parse_id(&row.get::<_, String>(0)?, "run id").map_err(to_sql_error)?,
        seq: codec::non_negative(row.get(1)?, "event sequence").map_err(to_sql_error)?,
        sim_time_ms: row.get(2)?,
        wall_time_ms: row.get(3)?,
        category: row.get(4)?,
        event_type: row.get(5)?,
        severity: codec::parse_enum(&row.get::<_, String>(6)?, "severity").map_err(to_sql_error)?,
        source: codec::parse_enum(&row.get::<_, String>(7)?, "event source")
            .map_err(to_sql_error)?,
        payload: codec::from_json_value(
            codec::from_json::<Value>(&row.get::<_, String>(8)?, "event payload")
                .map_err(to_sql_error)?,
            "event payload",
        )
        .map_err(to_sql_error)?,
        schema_version: codec::positive(row.get(9)?, "event schema version")
            .map_err(to_sql_error)?,
    })
}

fn map_log(row: &Row<'_>) -> rusqlite::Result<LogRecord> {
    let run_id =
        codec::parse_id::<RunId>(&row.get::<_, String>(1)?, "run id").map_err(to_sql_error)?;
    let context_json: Option<String> = row.get(7)?;
    let context: BTreeMap<String, Value> = context_json
        .as_deref()
        .map(|text| codec::from_json(text, "log context"))
        .transpose()
        .map_err(to_sql_error)?
        .unwrap_or_default();
    Ok(LogRecord {
        run_id,
        seq: codec::non_negative(row.get(2)?, "log sequence").map_err(to_sql_error)?,
        sim_time_ms: row.get(3)?,
        source: codec::parse_enum(&row.get::<_, String>(4)?, "log source").map_err(to_sql_error)?,
        level: codec::parse_enum(&row.get::<_, String>(5)?, "severity").map_err(to_sql_error)?,
        message: row.get(6)?,
        context,
    })
}

fn map_metric(row: &Row<'_>) -> rusqlite::Result<MetricSample> {
    Ok(MetricSample {
        run_id: codec::parse_id(&row.get::<_, String>(0)?, "run id").map_err(to_sql_error)?,
        sim_time_ms: row.get(1)?,
        entity_kind: codec::parse_enum(&row.get::<_, String>(2)?, "metric entity")
            .map_err(to_sql_error)?,
        entity_id: row.get(3)?,
        metric: row.get(4)?,
        value: row.get(5)?,
        unit: row.get(6)?,
    })
}

fn map_image(row: &Row<'_>) -> rusqlite::Result<SimulatedImage> {
    Ok(SimulatedImage {
        id: codec::parse_id(&row.get::<_, String>(0)?, "image id").map_err(to_sql_error)?,
        run_id: codec::parse_id(&row.get::<_, String>(1)?, "run id").map_err(to_sql_error)?,
        repository: row.get(2)?,
        tag: row.get(3)?,
        digest: row.get(4)?,
        size_bytes: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
        layers: codec::from_json::<Vec<ImageLayer>>(&row.get::<_, String>(6)?, "image layers")
            .map_err(to_sql_error)?,
        profile: codec::parse_enum(&row.get::<_, String>(7)?, "build profile")
            .map_err(to_sql_error)?,
        source_hash: row.get(8)?,
        engine_version: row.get(9)?,
        created_at: SimTime::from_millis(row.get(10)?),
    })
}

fn map_container(row: &Row<'_>) -> rusqlite::Result<Container> {
    let id: ContainerId =
        codec::parse_id(&row.get::<_, String>(0)?, "container id").map_err(to_sql_error)?;
    let process: Process =
        codec::from_json(&row.get::<_, String>(8)?, "process").map_err(to_sql_error)?;
    if process.container_id != id {
        return Err(storage_sql_error(
            "stored process belongs to another container",
        ));
    }
    Ok(Container {
        id,
        run_id: codec::parse_id(&row.get::<_, String>(1)?, "run id").map_err(to_sql_error)?,
        image_id: codec::parse_id(&row.get::<_, String>(2)?, "image id").map_err(to_sql_error)?,
        name: row.get(3)?,
        state: codec::parse_state(&row.get::<_, String>(4)?, "container state")
            .map_err(to_sql_error)?,
        health: codec::parse_state(&row.get::<_, String>(5)?, "health status")
            .map_err(to_sql_error)?,
        restart_count: codec::positive_or_zero(row.get(6)?, "restart count")
            .map_err(to_sql_error)?,
        config: codec::from_json(&row.get::<_, String>(7)?, "container config")
            .map_err(to_sql_error)?,
        process,
        created_at: SimTime::from_millis(row.get(9)?),
        started_at: row.get::<_, Option<i64>>(10)?.map(SimTime::from_millis),
        stopped_at: row.get::<_, Option<i64>>(11)?.map(SimTime::from_millis),
        exit_code: row.get(12)?,
        failure_reason: row
            .get::<_, Option<String>>(13)?
            .as_deref()
            .map(|text| codec::parse_state(text, "failure reason"))
            .transpose()
            .map_err(to_sql_error)?,
    })
}

fn map_deployment(row: &Row<'_>) -> rusqlite::Result<Deployment> {
    Ok(Deployment {
        id: codec::parse_id(&row.get::<_, String>(0)?, "deployment id").map_err(to_sql_error)?,
        run_id: codec::parse_id(&row.get::<_, String>(1)?, "run id").map_err(to_sql_error)?,
        image_id: codec::parse_id(&row.get::<_, String>(2)?, "image id").map_err(to_sql_error)?,
        container_id: codec::parse_id(&row.get::<_, String>(3)?, "container id")
            .map_err(to_sql_error)?,
        service_name: row.get(4)?,
        status: codec::parse_state(&row.get::<_, String>(5)?, "deployment status")
            .map_err(to_sql_error)?,
        health: codec::parse_state(&row.get::<_, String>(6)?, "health status")
            .map_err(to_sql_error)?,
        primary_reason: row
            .get::<_, Option<String>>(7)?
            .as_deref()
            .map(|text| codec::parse_state(text, "failure reason"))
            .transpose()
            .map_err(to_sql_error)?,
        created_at: SimTime::from_millis(row.get(8)?),
        healthy_since: row.get::<_, Option<i64>>(9)?.map(SimTime::from_millis),
        uptime_ms: u64::try_from(row.get::<_, i64>(10)?).unwrap_or(0),
    })
}

fn single_run(ids: impl Iterator<Item = RunId>) -> StorageResult<RunId> {
    let mut first = None;
    for id in ids {
        match first {
            None => first = Some(id),
            Some(existing) if existing != id => {
                return Err(StorageError::Validation(
                    "a batch must belong to exactly one run".into(),
                ))
            }
            _ => {}
        }
    }
    first.ok_or_else(|| StorageError::Validation("empty batch".into()))
}

fn storage_sql_error(message: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(StorageError::Corrupt(message.to_string())),
    )
}

fn to_sql_error(error: StorageError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
