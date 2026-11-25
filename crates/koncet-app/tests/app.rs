//! Application-layer integration tests against a real SQLite file.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use koncet_app::{App, Clock, CreateProjectInput, LogFilter, RunHistoryFilter, StartRunInput};
use koncet_domain::{
    DeploymentStatus, FailureReason, ProjectTemplate, RunResult, RunStatus, ScenarioPreset,
    TimestampUtc,
};
use koncet_storage::StoragePaths;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "koncet-app-{label}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn storage(&self) -> StoragePaths {
        StoragePaths::at(self.path.join("data"))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug, Clone, Copy)]
struct FixedClock(i64);

impl Clock for FixedClock {
    fn now(&self) -> TimestampUtc {
        TimestampUtc::from_millis(self.0)
    }
}

const START: i64 = 1_700_000_000_000;

fn open(temp: &TempDir, at: i64) -> App {
    App::open(temp.storage(), Arc::new(FixedClock(at))).unwrap()
}

struct Fixture {
    project_id: koncet_domain::ProjectId,
    scenario_id: koncet_domain::ScenarioId,
}

fn project(app: &mut App, name: &str) -> Fixture {
    let created = app
        .create_project(CreateProjectInput {
            name: name.to_string(),
            description: "integration fixture".into(),
            template: ProjectTemplate::StaticWeb,
        })
        .unwrap();
    let scenarios = app.list_scenarios(created.project.id).unwrap();
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0].name, "Happy Path");
    Fixture {
        project_id: created.project.id,
        scenario_id: scenarios[0].id,
    }
}

fn start(app: &mut App, fixture: &Fixture, seed: &str) -> koncet_app::RunHandle {
    app.start_run(StartRunInput {
        project_id: fixture.project_id,
        scenario_id: fixture.scenario_id,
        seed: Some(seed.to_string()),
    })
    .unwrap()
}

#[test]
fn project_creation_duplication_and_source_editing() {
    let temp = TempDir::new("projects");
    let mut app = open(&temp, START);

    assert_eq!(app.list_templates().len(), 2);
    let fixture = project(&mut app, "Acme");
    let detail = app.get_project(fixture.project_id).unwrap();
    assert_eq!(detail.files.files().len(), 3);

    let renamed = app
        .rename_project(fixture.project_id, "Acme Two".into())
        .unwrap();
    assert_eq!(renamed.project.name, "Acme Two");
    let described = app
        .set_project_description(fixture.project_id, "Updated".into())
        .unwrap();
    assert_eq!(described.project.description, "Updated");

    let edited = app
        .write_source_file(
            fixture.project_id,
            "index.html".into(),
            "<h1>edited</h1>".into(),
        )
        .unwrap();
    assert!(edited
        .files
        .get("index.html")
        .unwrap()
        .content
        .contains("edited"));
    let created = app
        .create_source_file(
            fixture.project_id,
            "src/app.js".into(),
            "console.log(1);".into(),
        )
        .unwrap();
    assert_eq!(created.files.files().len(), 4);
    assert!(app
        .create_source_file(fixture.project_id, "src/app.js".into(), "x".into())
        .is_err());
    let renamed_file = app
        .rename_source_file(
            fixture.project_id,
            "src/app.js".into(),
            "src/main.js".into(),
        )
        .unwrap();
    assert!(renamed_file.files.get("src/main.js").is_some());
    let removed = app
        .delete_source_file(fixture.project_id, "src/main.js".into())
        .unwrap();
    assert_eq!(removed.files.files().len(), 3);
    assert!(app
        .delete_source_file(fixture.project_id, "missing.txt".into())
        .is_err());

    let duplicate = app.duplicate_project(fixture.project_id).unwrap();
    assert_eq!(duplicate.project.name, "Acme Two copy");
    assert_eq!(app.list_projects(false, 10).unwrap().len(), 2);
    assert_eq!(app.list_scenarios(duplicate.project.id).unwrap().len(), 1);

    app.archive_project(fixture.project_id).unwrap();
    assert!(app
        .write_source_file(fixture.project_id, "index.html".into(), "blocked".into())
        .is_err());
    app.unarchive_project(fixture.project_id).unwrap();
    app.delete_project(duplicate.project.id).unwrap();
    assert_eq!(app.list_projects(true, 10).unwrap().len(), 1);
    app.shutdown().unwrap();
}

#[test]
fn happy_path_run_is_persisted_and_stopped_gracefully() {
    let temp = TempDir::new("happy");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Happy");
    let handle = start(&mut app, &fixture, "42");
    assert_eq!(handle.seed, "42");

    let conflict = app
        .start_run(StartRunInput {
            project_id: fixture.project_id,
            scenario_id: fixture.scenario_id,
            seed: Some("1".into()),
        })
        .unwrap_err();
    assert_eq!(conflict.code(), "active_run_exists");

    app.set_run_speed(handle.run_id, 8).unwrap();
    let update = app.advance_active(60_000).unwrap().unwrap();
    assert!(!update.finalised);
    let deployment = update.frame.deployment.as_ref().unwrap();
    assert_eq!(deployment.status, DeploymentStatus::Healthy);
    assert_eq!(deployment.service_name, "web");
    assert!(update.frame.image.is_some());
    assert!(!update.logs.is_empty());
    assert!(!update.metrics.is_empty());

    let final_update = app.stop_run(handle.run_id).unwrap();
    assert!(final_update.finalised);
    assert_eq!(final_update.frame.run.status, RunStatus::Completed);
    assert_eq!(final_update.frame.run.result, RunResult::Healthy);
    assert_eq!(
        final_update
            .frame
            .run
            .summary
            .as_ref()
            .unwrap()
            .primary_reason,
        Some(FailureReason::UserStopped)
    );
    assert!(app.active_run_id().is_none());

    let persisted = app.get_run(handle.run_id).unwrap();
    assert_eq!(persisted.run.seed, 42);
    assert!(persisted.image.is_some());
    assert!(persisted.container.is_some());
    assert!(persisted.deployment.is_some());
    assert_eq!(persisted.command_trace.len(), 2);
    assert_eq!(persisted.command_trace[0].seq, 1);
    assert_eq!(persisted.command_trace[1].seq, 2);

    let history = app
        .list_run_history(&RunHistoryFilter {
            limit: 10,
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].project_name, "Happy");
    assert_eq!(history[0].status, RunStatus::Completed);

    let events = app.list_events(handle.run_id, 0, true, 1_000).unwrap();
    assert_eq!(events.last().unwrap().event_type, "run.finalised");
    assert_eq!(
        app.count_events(handle.run_id).unwrap(),
        events.len() as u64
    );
    let logs = app
        .list_logs(
            handle.run_id,
            &LogFilter {
                limit: 1_000,
                ..LogFilter::default()
            },
        )
        .unwrap();
    assert_eq!(logs.len(), app.count_logs(handle.run_id).unwrap() as usize);
    assert_eq!(
        app.tail_logs(handle.run_id, 3).unwrap(),
        logs[logs.len() - 3..].to_vec()
    );

    app.shutdown().unwrap();

    let reopened = open(&temp, START + 1_000);
    assert_eq!(reopened.info().unwrap().schema_version, 1);
    let persisted = reopened.get_run(handle.run_id).unwrap();
    assert_eq!(persisted.run.result, RunResult::Healthy);
    assert_eq!(
        reopened
            .list_run_history(&RunHistoryFilter::default())
            .unwrap()
            .len(),
        1
    );
    reopened.shutdown().unwrap();
}

#[test]
fn pause_resume_and_connectivity_are_recorded_in_order() {
    let temp = TempDir::new("controls");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Controls");
    let handle = start(&mut app, &fixture, "7");

    let paused = app.pause_run(handle.run_id).unwrap();
    assert!(paused.frame.paused);
    assert_eq!(paused.frame.run.status, RunStatus::Paused);
    let clock_before = paused.frame.clock_ms;
    assert!(app.advance_active(10_000).unwrap().is_none());
    assert_eq!(app.session_frame().unwrap().clock_ms, clock_before);

    let resumed = app.resume_run(handle.run_id).unwrap();
    assert!(!resumed.frame.paused);
    assert_eq!(resumed.frame.run.status, RunStatus::Running);
    let speed = app.set_run_speed(handle.run_id, 4).unwrap();
    assert_eq!(speed.frame.speed, 4);
    let connectivity = app.check_connectivity(handle.run_id).unwrap();
    assert!(connectivity
        .events
        .iter()
        .any(|event| event.event_type == "deployment.connectivity_checked"));

    let final_update = app.stop_run(handle.run_id).unwrap();
    assert!(final_update.finalised);
    let commands = app.get_run(handle.run_id).unwrap().command_trace;
    assert_eq!(commands.len(), 5);
    assert_eq!(
        commands
            .iter()
            .map(|command| command.seq)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(commands[0].command.name(), "pause");
    assert_eq!(commands[4].command.name(), "stop");
    app.shutdown().unwrap();
}

#[test]
fn build_failure_finalises_without_image_or_deployment() {
    let temp = TempDir::new("build-failure");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Failure");
    let scenario = app
        .create_scenario(fixture.project_id, ScenarioPreset::BuildFailure, None)
        .unwrap();
    let handle = app
        .start_run(StartRunInput {
            project_id: fixture.project_id,
            scenario_id: scenario.id,
            seed: Some("11".into()),
        })
        .unwrap();
    app.set_run_speed(handle.run_id, 8).unwrap();
    let update = app.advance_active(20_000).unwrap().unwrap();
    assert!(update.finalised);
    assert_eq!(update.frame.run.result, RunResult::BuildFailed);
    assert!(update.frame.image.is_none());
    assert!(update.frame.container.is_none());
    assert!(update.frame.deployment.is_none());
    assert!(app.active_run_id().is_none());

    let persisted = app.get_run(handle.run_id).unwrap();
    assert!(persisted.build.unwrap().failed());
    assert!(persisted.image.is_none());

    // A new run can start after the failure finalised.
    let next = start(&mut app, &fixture, "12");
    assert_ne!(next.run_id, handle.run_id);
    app.stop_run(next.run_id).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn crash_loop_exhausts_restarts_and_finalises() {
    let temp = TempDir::new("crash");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Crash");
    let scenario = app
        .create_scenario(fixture.project_id, ScenarioPreset::CrashLoop, None)
        .unwrap();
    let handle = app
        .start_run(StartRunInput {
            project_id: fixture.project_id,
            scenario_id: scenario.id,
            seed: Some("3".into()),
        })
        .unwrap();
    app.set_run_speed(handle.run_id, 8).unwrap();

    let mut finalised = None;
    for _ in 0..50 {
        if let Some(update) = app.advance_active(60_000).unwrap() {
            if update.finalised {
                finalised = Some(update);
                break;
            }
        }
    }
    let update = finalised.expect("crash loop must finalise");
    assert_eq!(update.frame.run.result, RunResult::RuntimeFailed);
    assert_eq!(update.frame.container.as_ref().unwrap().restart_count, 3);
    assert_eq!(
        update.frame.deployment.as_ref().unwrap().status,
        DeploymentStatus::Failed
    );
    assert!(app.active_run_id().is_none());
    app.shutdown().unwrap();
}

#[test]
fn source_edits_do_not_change_the_active_snapshot() {
    let temp = TempDir::new("snapshot");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Snapshot");
    let handle = start(&mut app, &fixture, "99");
    app.advance_active(1_000).unwrap();

    app.write_source_file(
        fixture.project_id,
        "index.html".into(),
        "<h1>changed after start</h1>".into(),
    )
    .unwrap();

    app.set_run_speed(handle.run_id, 8).unwrap();
    app.advance_active(60_000).unwrap();
    app.stop_run(handle.run_id).unwrap();

    let persisted = app.get_run(handle.run_id).unwrap();
    let snapshot = app
        .get_source_snapshot(persisted.run.source_snapshot_id)
        .unwrap();
    let index = snapshot
        .files
        .iter()
        .find(|file| file.path == "index.html")
        .unwrap();
    assert!(index.content.contains("Hello from KONCET"));
    assert!(!index.content.contains("changed after start"));
    app.shutdown().unwrap();
}

#[test]
fn interrupted_runs_are_reconciled_on_startup() {
    let temp = TempDir::new("reconcile");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Interrupted");
    let handle = start(&mut app, &fixture, "5");
    app.advance_active(2_000).unwrap();
    drop(app);

    let reopened = open(&temp, START + 5_000);
    assert_eq!(reopened.reconciled_runs(), &[handle.run_id]);
    let persisted = reopened.get_run(handle.run_id).unwrap();
    assert_eq!(persisted.run.status, RunStatus::Cancelled);
    assert_eq!(
        persisted.run.summary.as_ref().map(|summary| summary.status),
        None
    );
    assert!(persisted.run.ended_at.is_some());
    let events = reopened.list_events(handle.run_id, 0, true, 1_000).unwrap();
    let cancelled = events.last().unwrap();
    assert_eq!(cancelled.event_type, "run.cancelled");
    assert_eq!(
        cancelled
            .payload
            .get("reason")
            .and_then(|value| value.as_str()),
        Some("app_terminated")
    );
    let history = reopened
        .list_run_history(&RunHistoryFilter::default())
        .unwrap();
    assert_eq!(history[0].status, RunStatus::Cancelled);
    reopened.shutdown().unwrap();
}

#[test]
fn scenario_revisions_are_optimistic_and_validated() {
    let temp = TempDir::new("scenarios");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Scenarios");

    let record = app.get_scenario(fixture.scenario_id).unwrap();
    assert_eq!(record.revision, 1);
    let mut scenario = record.scenario.clone();
    scenario.description = "edited".into();
    let updated = app
        .update_scenario(record.id, record.revision, scenario.clone())
        .unwrap();
    assert_eq!(updated.revision, 2);

    let stale = app
        .update_scenario(record.id, 1, scenario.clone())
        .unwrap_err();
    assert_eq!(stale.code(), "conflict");

    let mut invalid = scenario;
    invalid.name = "  ".into();
    let error = app.update_scenario(record.id, 2, invalid).unwrap_err();
    assert_eq!(error.code(), "validation_failed");
    assert_eq!(app.get_scenario(record.id).unwrap().revision, 2);

    let duplicate = app.duplicate_scenario(record.id).unwrap();
    assert!(duplicate.scenario.name.starts_with("Happy Path"));
    assert_ne!(duplicate.id, record.id);
    assert_eq!(duplicate.revision, 1);

    let presets = app.list_scenario_presets();
    assert_eq!(presets.len(), 4);
    assert!(presets.iter().any(|preset| preset.key == "crash-loop"));

    app.delete_scenario(duplicate.id).unwrap();
    assert!(app.get_scenario(duplicate.id).is_err());
    app.shutdown().unwrap();
}

#[test]
fn settings_backups_and_diagnostics_work() {
    let temp = TempDir::new("settings");
    let mut app = open(&temp, START);
    project(&mut app, "Storage");

    app.set_setting("theme_mode", &"dark").unwrap();
    assert_eq!(
        app.get_setting::<String>("theme_mode").unwrap(),
        Some("dark".to_string())
    );
    app.set_setting_json("window_geometry", &serde_json::json!({"width": 1280}))
        .unwrap();
    assert_eq!(app.get_settings().unwrap().len(), 2);
    assert!(app.set_setting("api_token", &"secret").is_err());
    assert!(app.delete_setting("theme_mode").unwrap());
    assert!(!app.delete_setting("theme_mode").unwrap());

    let backup = app.create_manual_backup().unwrap();
    assert!(backup.path.exists());
    assert_eq!(app.list_backups().unwrap().len(), 1);
    let usage = app.storage_usage().unwrap();
    assert_eq!(usage.project_count, 1);
    assert!(usage.backup_bytes > 0);
    app.check_integrity().unwrap();

    let diagnostics = app.diagnostics().unwrap();
    assert!(diagnostics.integrity_ok);
    assert_eq!(diagnostics.backup_count, 1);
    assert_eq!(diagnostics.app.schema_version, 1);

    app.delete_backup(&backup.path).unwrap();
    assert!(app.list_backups().unwrap().is_empty());
    app.shutdown().unwrap();
}

#[test]
fn seeds_are_validated_and_normalised() {
    assert_eq!(koncet_app::parse_seed("0").unwrap(), 0);
    assert_eq!(koncet_app::parse_seed(" 42 ").unwrap(), 42);
    assert!(koncet_app::parse_seed("").is_err());
    assert!(koncet_app::parse_seed("-1").is_err());
    assert!(koncet_app::parse_seed("abc").is_err());
    assert!(koncet_app::parse_seed("9223372036854775808").is_err());

    let temp = TempDir::new("seeds");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Seeds");
    let handle = start(&mut app, &fixture, "0");
    assert_eq!(handle.seed, "0");
    app.stop_run(handle.run_id).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn history_deletion_respects_the_active_run() {
    let temp = TempDir::new("deletion");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Deletion");
    let handle = start(&mut app, &fixture, "1");
    assert_eq!(
        app.delete_run(handle.run_id).unwrap_err().code(),
        "active_run_exists"
    );
    assert_eq!(
        app.delete_run_history(Some(fixture.project_id))
            .unwrap_err()
            .code(),
        "active_run_exists"
    );
    assert_eq!(
        app.delete_project(fixture.project_id).unwrap_err().code(),
        "active_run_exists"
    );
    app.stop_run(handle.run_id).unwrap();
    assert_eq!(app.delete_run_history(Some(fixture.project_id)).unwrap(), 1);
    assert!(app
        .list_run_history(&RunHistoryFilter::default())
        .unwrap()
        .is_empty());
    app.delete_project(fixture.project_id).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn http_service_template_gets_a_runnable_baseline() {
    let temp = TempDir::new("http-service");
    let mut app = open(&temp, START);
    let created = app
        .create_project(CreateProjectInput {
            name: "API".into(),
            description: String::new(),
            template: ProjectTemplate::HttpService,
        })
        .unwrap();
    assert!(created.files.get("src/service.js").is_some());
    let scenarios = app.list_scenarios(created.project.id).unwrap();
    let record = app.get_scenario(scenarios[0].id).unwrap();
    assert_eq!(
        record.scenario.build.profile,
        koncet_domain::BuildProfile::HttpService
    );

    let handle = app
        .start_run(StartRunInput {
            project_id: created.project.id,
            scenario_id: record.id,
            seed: Some("123".into()),
        })
        .unwrap();
    app.set_run_speed(handle.run_id, 8).unwrap();
    app.advance_active(60_000).unwrap();
    let frame = app.session_frame().unwrap();
    assert_eq!(
        frame.deployment.as_ref().unwrap().status,
        DeploymentStatus::Healthy
    );
    app.stop_run(handle.run_id).unwrap();
    app.shutdown().unwrap();
}

#[test]
fn shutdown_stops_an_active_run_and_persists_the_summary() {
    let temp = TempDir::new("shutdown");
    let mut app = open(&temp, START);
    let fixture = project(&mut app, "Shutdown");
    let handle = start(&mut app, &fixture, "21");
    app.set_run_speed(handle.run_id, 8).unwrap();
    app.advance_active(30_000).unwrap();
    app.shutdown().unwrap();

    let reopened = open(&temp, START + 1_000);
    let persisted = reopened.get_run(handle.run_id).unwrap();
    assert_eq!(persisted.run.status, RunStatus::Completed);
    assert_eq!(persisted.run.result, RunResult::Healthy);
    assert_eq!(
        persisted
            .run
            .summary
            .as_ref()
            .and_then(|summary| summary.primary_reason),
        Some(FailureReason::UserStopped)
    );
    assert!(!persisted.command_trace.is_empty());
    reopened.shutdown().unwrap();
}
