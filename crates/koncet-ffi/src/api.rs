//! Public bridge API. Every function is asynchronous on the Dart side unless it
//! returns a `Stream`, and every failure is a [`BridgeError`].

use std::path::PathBuf;
use std::sync::Arc;

use koncet_app::{
    App, Clock, CreateProjectInput, LogFilter, OpenOptions, RunHistoryFilter, StartRunInput,
    SystemClock,
};
use koncet_domain::{
    LogSource, ProjectId, ProjectTemplate, RunId, ScenarioId, ScenarioPreset, TimestampUtc,
};

use crate::dto::*;
use crate::error::{guard, BridgeError, BridgeResult};
use crate::frb_generated::StreamSink;
use crate::state::{
    active_frame, active_run_id, install, publish, subscribe_logs, subscribe_metrics,
    subscribe_session, take_app, with_app,
};

// ---------------------------------------------------------------------------
// Bootstrap
// ---------------------------------------------------------------------------

/// Opens the default XDG database and reconciles interrupted runs.
///
/// Pass `allowStaleLock: true` only after the user has confirmed that a stale
/// lock may be removed on the recovery screen.
pub fn init_app(allow_stale_lock: bool) -> BridgeResult<AppInfoDto> {
    guard(|| {
        if let Some(info) = existing_info()? {
            return Ok(info);
        }
        let options = OpenOptions {
            break_stale_lock: allow_stale_lock,
            ..OpenOptions::default()
        };
        let app = App::open_default_with(system_clock(), options)?;
        finish_init(app)
    })
}

/// Opens a database at an explicit root. Intended for diagnostics and tests.
#[doc(hidden)]
pub fn init_app_at(data_root: String, allow_stale_lock: bool) -> BridgeResult<AppInfoDto> {
    guard(|| {
        if let Some(info) = existing_info()? {
            return Ok(info);
        }
        let options = OpenOptions {
            break_stale_lock: allow_stale_lock,
            ..OpenOptions::default()
        };
        let app = App::open_at(PathBuf::from(data_root), system_clock(), options)?;
        finish_init(app)
    })
}

fn finish_init(app: App) -> BridgeResult<AppInfoDto> {
    let info = app.info()?;
    let reconciled = app
        .reconciled_runs()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    install(app)?;
    Ok(AppInfoDto::from_info(info, reconciled))
}

fn existing_info() -> BridgeResult<Option<AppInfoDto>> {
    let bridge = crate::state::lock_bridge();
    let Some(app) = bridge.app.as_ref() else {
        return Ok(None);
    };
    let info = app.info()?;
    let reconciled = app
        .reconciled_runs()
        .iter()
        .map(ToString::to_string)
        .collect();
    Ok(Some(AppInfoDto::from_info(info, reconciled)))
}

/// Stops any active run, checkpoints SQLite, and closes the bridge.
pub fn shutdown_app() -> BridgeResult<()> {
    guard(|| {
        if let Some(app) = take_app()? {
            app.shutdown()?;
        }
        Ok(())
    })
}

pub fn get_app_info() -> BridgeResult<AppInfoDto> {
    with_app(|app| {
        let info = app.info()?;
        let reconciled = app
            .reconciled_runs()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((info, reconciled))
    })
    .map(|(info, reconciled)| AppInfoDto::from_info(info, reconciled))
}

fn system_clock() -> Arc<dyn Clock> {
    Arc::new(SystemClock)
}

// ---------------------------------------------------------------------------
// Projects and source
// ---------------------------------------------------------------------------

pub fn list_templates() -> BridgeResult<Vec<TemplateDto>> {
    with_app(|app| {
        Ok(app
            .list_templates()
            .into_iter()
            .map(|template| TemplateDto {
                id: template.id,
                label: template.label,
                description: template.description,
                file_count: template.file_count as u32,
            })
            .collect())
    })
}

pub fn list_projects(include_archived: bool, limit: u32) -> BridgeResult<Vec<ProjectSummaryDto>> {
    with_app(|app| app.list_projects(include_archived, limit))
        .map(|items| items.into_iter().map(Into::into).collect())
}

pub fn get_project(project_id: String) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.get_project(project_id)).map(Into::into)
}

pub fn get_source_tree(project_id: String) -> BridgeResult<SourceTreeDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.project_source_tree(project_id)).map(Into::into)
}

pub fn read_source_file(project_id: String, path: String) -> BridgeResult<SourceFileDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| {
        let tree = app.project_source_tree(project_id)?;
        tree.get(&path)
            .cloned()
            .map(Into::into)
            .ok_or_else(|| koncet_app::AppError::not_found("source file"))
    })
}

pub fn create_project(
    name: String,
    description: String,
    template: ProjectTemplateDto,
) -> BridgeResult<ProjectDetailDto> {
    let template = template_from_dto(template);
    with_app(|app| {
        app.create_project(CreateProjectInput {
            name,
            description,
            template,
        })
    })
    .map(Into::into)
}

pub fn rename_project(project_id: String, name: String) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.rename_project(project_id, name)).map(Into::into)
}

pub fn set_project_description(
    project_id: String,
    description: String,
) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.set_project_description(project_id, description)).map(Into::into)
}

pub fn archive_project(project_id: String) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.archive_project(project_id)).map(Into::into)
}

pub fn unarchive_project(project_id: String) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.unarchive_project(project_id)).map(Into::into)
}

pub fn delete_project(project_id: String) -> BridgeResult<()> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.delete_project(project_id))
}

pub fn duplicate_project(project_id: String) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.duplicate_project(project_id)).map(Into::into)
}

pub fn write_source_file(
    project_id: String,
    path: String,
    content: String,
) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.write_source_file(project_id, path, content)).map(Into::into)
}

pub fn create_source_file(
    project_id: String,
    path: String,
    content: String,
) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.create_source_file(project_id, path, content)).map(Into::into)
}

pub fn rename_source_file(
    project_id: String,
    from: String,
    to: String,
) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.rename_source_file(project_id, from, to)).map(Into::into)
}

pub fn delete_source_file(project_id: String, path: String) -> BridgeResult<ProjectDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.delete_source_file(project_id, path)).map(Into::into)
}

pub fn validate_source(project_id: String) -> BridgeResult<ValidationReportDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.validate_source(project_id)).map(Into::into)
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

pub fn list_scenario_presets() -> BridgeResult<Vec<PresetDto>> {
    with_app(|app| {
        Ok(app
            .list_scenario_presets()
            .into_iter()
            .map(|preset| PresetDto {
                key: preset.key,
                label: preset.label,
                description: preset.description,
            })
            .collect())
    })
}

pub fn list_scenarios(project_id: String) -> BridgeResult<Vec<ScenarioSummaryDto>> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| app.list_scenarios(project_id))
        .map(|items| items.into_iter().map(Into::into).collect())
}

pub fn get_scenario(scenario_id: String) -> BridgeResult<ScenarioDetailDto> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    with_app(|app| app.get_scenario(scenario_id)).map(Into::into)
}

pub fn create_scenario(
    project_id: String,
    preset: String,
    name: Option<String>,
) -> BridgeResult<ScenarioDetailDto> {
    let project_id = parse_project_id(&project_id)?;
    let preset = parse_preset(&preset)?;
    with_app(|app| app.create_scenario(project_id, preset, name)).map(Into::into)
}

pub fn update_scenario(
    scenario_id: String,
    expected_revision: u32,
    scenario_json: String,
) -> BridgeResult<ScenarioDetailDto> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    with_app(|app| {
        let scenario = app.parse_scenario_json(&scenario_json)?;
        app.update_scenario(scenario_id, expected_revision, scenario)
    })
    .map(Into::into)
}

pub fn duplicate_scenario(scenario_id: String) -> BridgeResult<ScenarioDetailDto> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    with_app(|app| app.duplicate_scenario(scenario_id)).map(Into::into)
}

pub fn delete_scenario(scenario_id: String) -> BridgeResult<()> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    with_app(|app| app.delete_scenario(scenario_id))
}

pub fn validate_scenario(
    project_id: String,
    scenario_json: String,
) -> BridgeResult<ValidationReportDto> {
    let project_id = parse_project_id(&project_id)?;
    with_app(|app| {
        let scenario = app.parse_scenario_json(&scenario_json)?;
        app.validate_scenario(project_id, &scenario)
    })
    .map(Into::into)
}

pub fn canonical_scenario_json(scenario_json: String) -> BridgeResult<String> {
    with_app(|app| {
        let scenario = app.parse_scenario_json(&scenario_json)?;
        app.canonical_scenario_json(&scenario)
    })
}

// ---------------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------------

pub fn start_run(
    project_id: String,
    scenario_id: String,
    seed: Option<String>,
) -> BridgeResult<RunHandleDto> {
    let project_id = parse_project_id(&project_id)?;
    let scenario_id = parse_scenario_id(&scenario_id)?;
    with_app(|app| {
        app.start_run(StartRunInput {
            project_id,
            scenario_id,
            seed,
        })
    })
    .map(Into::into)
}

/// Advances the active session by wall-clock milliseconds at the current speed.
pub fn advance_session(elapsed_ms: u64) -> BridgeResult<Option<RunUpdateDto>> {
    with_app(|app| app.advance_active(elapsed_ms)).map(|update| update.map(publish))
}

pub fn pause_run(run_id: String) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.pause_run(run_id)).map(publish)
}

pub fn resume_run(run_id: String) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.resume_run(run_id)).map(publish)
}

pub fn set_run_speed(run_id: String, speed: u8) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.set_run_speed(run_id, speed)).map(publish)
}

pub fn restart_container(run_id: String) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.restart_container(run_id)).map(publish)
}

pub fn check_connectivity(run_id: String) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.check_connectivity(run_id)).map(publish)
}

pub fn stop_run(run_id: String) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.stop_run(run_id)).map(publish)
}

pub fn cancel_run(run_id: String) -> BridgeResult<RunUpdateDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.cancel_run(run_id)).map(publish)
}

pub fn get_active_run_id() -> BridgeResult<Option<String>> {
    guard(|| Ok(active_run_id().map(|id| id.to_string())))
}

pub fn get_active_frame() -> BridgeResult<Option<RunFrameDto>> {
    guard(|| Ok(active_frame().as_ref().map(Into::into)))
}

pub fn subscribe_session_stream(
    run_id: String,
    sink: StreamSink<SessionFrameDto>,
) -> BridgeResult<()> {
    let run_id = parse_run_id(&run_id)?;
    subscribe_session(run_id, sink)
}

pub fn subscribe_logs_stream(run_id: String, sink: StreamSink<LogBatchDto>) -> BridgeResult<()> {
    let run_id = parse_run_id(&run_id)?;
    subscribe_logs(run_id, sink)
}

pub fn subscribe_metrics_stream(
    run_id: String,
    sink: StreamSink<MetricBatchDto>,
) -> BridgeResult<()> {
    let run_id = parse_run_id(&run_id)?;
    subscribe_metrics(run_id, sink)
}

pub fn get_run(run_id: String) -> BridgeResult<RunDetailDto> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.get_run(run_id)).map(Into::into)
}

pub fn list_run_history(filter: RunHistoryFilterDto) -> BridgeResult<Vec<RunHistoryItemDto>> {
    let filter = history_filter(filter)?;
    with_app(|app| app.list_run_history(&filter))
        .map(|items| items.into_iter().map(Into::into).collect())
}

pub fn count_runs(project_id: Option<String>) -> BridgeResult<u64> {
    let project_id = match project_id {
        Some(value) => Some(parse_project_id(&value)?),
        None => None,
    };
    with_app(|app| app.count_runs(project_id))
}

pub fn delete_run(run_id: String) -> BridgeResult<()> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.delete_run(run_id))
}

pub fn delete_run_history(project_id: Option<String>) -> BridgeResult<u64> {
    let project_id = match project_id {
        Some(value) => Some(parse_project_id(&value)?),
        None => None,
    };
    with_app(|app| app.delete_run_history(project_id))
}

pub fn get_run_events(
    run_id: String,
    after_seq: u64,
    include_debug: bool,
    limit: u32,
) -> BridgeResult<Vec<EventDto>> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.list_events(run_id, after_seq, include_debug, limit))
        .map(|events| events.iter().map(Into::into).collect())
}

pub fn count_run_events(run_id: String) -> BridgeResult<u64> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.count_events(run_id))
}

pub fn get_run_logs(run_id: String, filter: LogFilterDto) -> BridgeResult<Vec<LogDto>> {
    let run_id = parse_run_id(&run_id)?;
    let filter = log_filter(filter);
    with_app(|app| app.list_logs(run_id, &filter)).map(|logs| logs.iter().map(Into::into).collect())
}

pub fn tail_run_logs(run_id: String, limit: u32) -> BridgeResult<Vec<LogDto>> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.tail_logs(run_id, limit)).map(|logs| logs.iter().map(Into::into).collect())
}

pub fn count_run_logs(run_id: String) -> BridgeResult<u64> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.count_logs(run_id))
}

pub fn get_run_metrics(run_id: String, filter: MetricsFilterDto) -> BridgeResult<Vec<MetricDto>> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| {
        app.list_metrics(
            run_id,
            filter.entity.into(),
            &filter.entity_id,
            filter.metric.as_deref(),
            filter.from_ms,
            filter.to_ms,
            filter.limit,
        )
    })
    .map(|metrics| metrics.iter().map(Into::into).collect())
}

pub fn count_run_metrics(run_id: String) -> BridgeResult<u64> {
    let run_id = parse_run_id(&run_id)?;
    with_app(|app| app.count_metrics(run_id))
}

pub fn get_source_snapshot(snapshot_id: String) -> BridgeResult<SourceSnapshotDto> {
    let snapshot_id = snapshot_id
        .parse()
        .map_err(|_| BridgeError::invalid_id("source snapshot"))?;
    with_app(|app| app.get_source_snapshot(snapshot_id)).map(Into::into)
}

// ---------------------------------------------------------------------------
// Settings and system
// ---------------------------------------------------------------------------

pub fn list_settings() -> BridgeResult<Vec<SettingEntryDto>> {
    with_app(|app| app.get_settings()).map(|settings| {
        settings
            .into_iter()
            .map(|(key, value)| SettingEntryDto {
                key,
                value_json: serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
            })
            .collect()
    })
}

pub fn get_setting_json(key: String) -> BridgeResult<Option<String>> {
    with_app(|app| app.get_setting_json(&key)).map(|value| {
        value.map(|value| serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()))
    })
}

pub fn set_setting_json(key: String, value_json: String) -> BridgeResult<()> {
    let value = serde_json::from_str(&value_json).map_err(|error| {
        BridgeError::new(
            "invalid_json",
            format!("setting value is not valid JSON: {error}"),
            &key,
            false,
        )
    })?;
    with_app(|app| app.set_setting_json(&key, &value))
}

pub fn delete_setting(key: String) -> BridgeResult<bool> {
    with_app(|app| app.delete_setting(&key))
}

pub fn get_data_usage() -> BridgeResult<StorageUsageDto> {
    with_app(|app| app.storage_usage()).map(Into::into)
}

pub fn create_backup() -> BridgeResult<BackupInfoDto> {
    with_app(|app| app.create_manual_backup()).map(Into::into)
}

pub fn list_backups() -> BridgeResult<Vec<BackupInfoDto>> {
    with_app(|app| app.list_backups()).map(|items| items.into_iter().map(Into::into).collect())
}

pub fn delete_backup(path: String) -> BridgeResult<()> {
    with_app(|app| app.delete_backup(&PathBuf::from(path)))
}

pub fn check_integrity() -> BridgeResult<()> {
    with_app(|app| app.check_integrity())
}

pub fn quick_check() -> BridgeResult<()> {
    with_app(|app| app.quick_check())
}

pub fn get_diagnostics_info() -> BridgeResult<DiagnosticsDto> {
    with_app(|app| app.diagnostics()).map(DiagnosticsDto::from_info)
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_project_id(value: &str) -> BridgeResult<ProjectId> {
    value
        .parse()
        .map_err(|_| BridgeError::invalid_id("project"))
}

fn parse_scenario_id(value: &str) -> BridgeResult<ScenarioId> {
    value
        .parse()
        .map_err(|_| BridgeError::invalid_id("scenario"))
}

fn parse_run_id(value: &str) -> BridgeResult<RunId> {
    value.parse().map_err(|_| BridgeError::invalid_id("run"))
}

fn template_from_dto(value: ProjectTemplateDto) -> ProjectTemplate {
    match value {
        ProjectTemplateDto::StaticWeb => ProjectTemplate::StaticWeb,
        ProjectTemplateDto::HttpService => ProjectTemplate::HttpService,
    }
}

fn parse_preset(value: &str) -> BridgeResult<ScenarioPreset> {
    ScenarioPreset::ALL
        .iter()
        .find(|preset| preset.key() == value)
        .copied()
        .ok_or_else(|| {
            BridgeError::new(
                "invalid_preset",
                "the scenario preset is not available",
                "scenario preset",
                false,
            )
        })
}

fn history_filter(filter: RunHistoryFilterDto) -> BridgeResult<RunHistoryFilter> {
    let before = match (filter.before_started_at_ms, filter.before_run_id) {
        (None, None) => None,
        (Some(at), Some(run_id)) => Some((TimestampUtc::from_millis(at), parse_run_id(&run_id)?)),
        _ => {
            return Err(BridgeError::new(
                "invalid_cursor",
                "a history cursor needs both a timestamp and a run id",
                "cursor",
                false,
            ))
        }
    };
    let project_id = match filter.project_id {
        Some(value) => Some(parse_project_id(&value)?),
        None => None,
    };
    Ok(RunHistoryFilter {
        project_id,
        before,
        limit: filter.limit,
    })
}

fn log_filter(filter: LogFilterDto) -> LogFilter {
    LogFilter {
        after_id: filter.after_id,
        level: filter.level.map(Into::into),
        source: filter.source.map(LogSource::from),
        search: filter
            .search
            .map(|search| search.trim().to_string())
            .filter(|search| !search.is_empty()),
        limit: filter.limit,
    }
}
