//! Storage integration tests: migrations, lock, round trips, backup, and history.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use koncet_domain::{
    BuildStageName, MetricEntity, Project, ProjectTemplate, RunCommand, RunId, RunResult,
    ScenarioId, ScenarioPreset, ScenarioRecord, Severity, SourceSnapshot, SourceTree, TimestampUtc,
};
use koncet_sim::{SimulationEngine, SimulationInputs};
use koncet_storage::{
    BackupKind, Database, LogFilter, OpenOptions, RunHistoryFilter, RunState, StorageError,
    StoragePaths,
};
use rusqlite::Connection;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "koncet-storage-{label}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn paths(&self) -> StoragePaths {
        StoragePaths::at(self.path.join("data"))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn open(temp: &TempDir) -> Database {
    Database::open(temp.paths()).unwrap()
}

fn project_and_tree(name: &str) -> (Project, SourceTree) {
    let project = Project::new(
        name,
        "test project",
        ProjectTemplate::StaticWeb,
        TimestampUtc::from_millis(1_700_000_000_000),
    )
    .unwrap();
    let tree = SourceTree::new(ProjectTemplate::StaticWeb.seed_files()).unwrap();
    (project, tree)
}

fn scenario_record(project_id: koncet_domain::ProjectId) -> ScenarioRecord {
    let scenario = ScenarioPreset::HappyPath.scenario();
    ScenarioRecord {
        id: ScenarioId::new(),
        project_id,
        revision: 1,
        schema_version: scenario.schema_version,
        canonical_json: scenario.canonical_json(),
        scenario,
        created_at: TimestampUtc::from_millis(1_700_000_000_000),
        updated_at: TimestampUtc::from_millis(1_700_000_000_000),
    }
}

fn engine_for(
    project_id: koncet_domain::ProjectId,
    scenario_id: ScenarioId,
    run_id: RunId,
    snapshot: SourceSnapshot,
    started_at: TimestampUtc,
) -> SimulationEngine {
    SimulationEngine::start(SimulationInputs {
        run_id,
        project_id,
        scenario_id,
        scenario: ScenarioPreset::HappyPath.scenario(),
        source_snapshot: snapshot,
        seed: 42,
        started_at,
        app_version: "0.1.0".into(),
        engine_version: "0.1.0".into(),
    })
    .unwrap()
}

#[test]
fn open_applies_migration_and_reopens_cleanly() {
    let temp = TempDir::new("open");
    let paths = temp.paths();
    let database = Database::open(paths.clone()).unwrap();

    assert_eq!(database.schema_version().unwrap(), 1);
    assert!(paths.database().exists());
    assert!(paths.lock().exists());
    assert_eq!(
        fs::metadata(paths.root()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(paths.database()).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let reader = database.reader().unwrap();
    let journal: String = reader
        .conn()
        .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal, "wal");
    let foreign_keys: i64 = reader
        .conn()
        .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(foreign_keys, 1);
    drop(reader);

    database.shutdown().unwrap();
    assert!(!paths.lock().exists());

    let reopened = Database::open(paths.clone()).unwrap();
    let migrations: i64 = reopened
        .reader()
        .unwrap()
        .conn()
        .query_row("SELECT COUNT(*) FROM schema_migrations;", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(migrations, 1);
    reopened.shutdown().unwrap();
    assert!(!paths.lock().exists());
}

#[test]
fn project_scenario_and_snapshot_round_trip() {
    let temp = TempDir::new("entities");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("Acme");
    let scenario = scenario_record(project.id);
    database
        .create_project_with_scenario(&project, &tree, Some(&scenario))
        .unwrap();

    let loaded = database.load_project(project.id).unwrap().unwrap();
    assert_eq!(loaded.project, project);
    assert_eq!(loaded.files, tree);

    let projects = database.list_projects(false, 10).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Acme");
    assert_eq!(projects[0].file_count, 3);

    let loaded_scenario = database.load_scenario(scenario.id).unwrap().unwrap();
    assert_eq!(loaded_scenario.scenario, scenario.scenario);
    assert_eq!(
        loaded_scenario.canonical_json,
        scenario.scenario.canonical_json()
    );
    assert_eq!(loaded_scenario.revision, 1);

    let mut edited = loaded_scenario;
    edited.scenario.description = "edited".into();
    edited.updated_at = TimestampUtc::from_millis(1_700_000_100_000);
    database.save_scenario(&mut edited).unwrap();
    assert_eq!(edited.revision, 2);
    assert_eq!(
        database
            .load_scenario(scenario.id)
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    assert!(
        database.update_scenario(&edited, 1).is_err(),
        "stale revision must fail"
    );

    let snapshot = SourceSnapshot::capture(
        project.id,
        &tree,
        TimestampUtc::from_millis(1_700_000_200_000),
    );
    database.insert_source_snapshot(&snapshot).unwrap();
    let loaded_snapshot = database.load_source_snapshot(snapshot.id).unwrap().unwrap();
    assert_eq!(loaded_snapshot, snapshot);
    assert_eq!(
        database
            .list_source_snapshots(project.id, 10)
            .unwrap()
            .len(),
        1
    );

    let mut renamed = project.clone();
    renamed
        .rename("Renamed", TimestampUtc::from_millis(1_700_000_300_000))
        .unwrap();
    database.update_project(&renamed).unwrap();
    assert_eq!(
        database
            .load_project(project.id)
            .unwrap()
            .unwrap()
            .project
            .name,
        "Renamed"
    );

    let replaced = SourceTree::new(vec![koncet_domain::SourceFile::new(
        "index.html",
        "<h1>replaced</h1>",
    )])
    .unwrap();
    database
        .replace_source_files(
            project.id,
            &replaced,
            TimestampUtc::from_millis(1_700_000_400_000),
        )
        .unwrap();
    assert_eq!(
        database.load_project(project.id).unwrap().unwrap().files,
        replaced
    );

    database.delete_project(project.id).unwrap();
    assert!(database.load_project(project.id).unwrap().is_none());
    assert!(database.list_projects(true, 10).unwrap().is_empty());
    assert_eq!(database.usage().unwrap().project_count, 0);
}

#[test]
fn engine_output_round_trips_through_storage() {
    let temp = TempDir::new("run");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("Runtime");
    let scenario = scenario_record(project.id);
    database
        .create_project_with_scenario(&project, &tree, Some(&scenario))
        .unwrap();

    let snapshot = SourceSnapshot::capture(
        project.id,
        &tree,
        TimestampUtc::from_millis(1_700_000_000_000),
    );
    let run_id = RunId::new();
    let mut engine = engine_for(
        project.id,
        scenario.id,
        run_id,
        snapshot.clone(),
        TimestampUtc::from_millis(1_700_000_000_000),
    );

    let initial_events = engine.drain_events();
    database
        .start_run(engine.run(), &snapshot, &initial_events)
        .unwrap();

    engine.advance_ticks(500);
    engine.submit_command(RunCommand::Stop).unwrap();
    assert_eq!(engine.run().result, RunResult::Healthy);

    let events = engine.drain_events();
    let logs = engine.drain_logs();
    let metrics = engine.drain_metrics();
    let commands = engine.commands().to_vec();

    database
        .persist_run_state(
            &RunState {
                run: engine.run(),
                build: engine.build(),
                image: engine.image(),
                container: engine.container(),
                deployment: engine.deployment(),
                at: engine.clock(),
            },
            &events,
            &logs,
            &metrics,
        )
        .unwrap();
    database.append_commands(run_id, &commands).unwrap();

    let persisted = database.load_run(run_id).unwrap().unwrap();
    assert_eq!(&persisted.run, engine.run());
    assert_eq!(&persisted.build.unwrap(), engine.build());
    assert_eq!(&persisted.image.unwrap(), engine.image().unwrap());
    assert_eq!(&persisted.container.unwrap(), engine.container().unwrap());
    assert_eq!(&persisted.deployment.unwrap(), engine.deployment().unwrap());
    assert_eq!(persisted.command_trace, commands);

    let stored_events = database.list_events(run_id, 0, true, 1_000).unwrap();
    assert_eq!(stored_events.len(), initial_events.len() + events.len());
    assert_eq!(stored_events.last().unwrap().event_type, "run.finalised");
    assert_eq!(
        database.list_events(run_id, 0, false, 1_000).unwrap().len(),
        stored_events
            .iter()
            .filter(|event| event.severity != Severity::Debug)
            .count()
    );

    let stored_logs = database
        .list_logs(
            run_id,
            &LogFilter {
                limit: 1_000,
                ..LogFilter::default()
            },
        )
        .unwrap();
    assert_eq!(stored_logs, logs);
    assert_eq!(
        database.tail_logs(run_id, 3).unwrap(),
        logs[logs.len() - 3..].to_vec()
    );
    assert_eq!(database.count_logs(run_id).unwrap(), logs.len() as u64);

    let keys: BTreeSet<(String, String, String)> = metrics
        .iter()
        .map(|sample| {
            (
                sample.entity_kind.as_str().to_string(),
                sample.entity_id.clone(),
                sample.metric.clone(),
            )
        })
        .collect();
    let mut total = 0usize;
    for (kind, entity_id, metric) in keys {
        let entity = match kind.as_str() {
            "build" => MetricEntity::Build,
            "container" => MetricEntity::Container,
            "deployment" => MetricEntity::Deployment,
            other => panic!("unexpected entity {other}"),
        };
        let series = database
            .list_metrics(
                run_id,
                entity,
                &entity_id,
                Some(&metric),
                0,
                i64::MAX,
                1_000,
            )
            .unwrap();
        let expected: Vec<_> = metrics
            .iter()
            .filter(|sample| {
                sample.entity_kind == entity
                    && sample.entity_id == entity_id
                    && sample.metric == metric
            })
            .cloned()
            .collect();
        assert_eq!(series, expected);
        total += series.len();
    }
    assert_eq!(total, metrics.len());

    let history = database
        .list_run_history(&RunHistoryFilter {
            limit: 10,
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].run_id, run_id);
    assert_eq!(history[0].status, engine.run().status);
    assert_eq!(history[0].summary, engine.run().summary);

    database.delete_run(run_id).unwrap();
    assert!(database.load_run(run_id).unwrap().is_none());
    assert_eq!(database.count_events(run_id).unwrap(), 0);
    assert_eq!(database.count_logs(run_id).unwrap(), 0);
    assert_eq!(database.count_metrics(run_id).unwrap(), 0);
}

#[test]
fn event_and_log_sequence_gaps_are_rejected() {
    let temp = TempDir::new("gaps");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("Gaps");
    let scenario = scenario_record(project.id);
    database
        .create_project_with_scenario(&project, &tree, Some(&scenario))
        .unwrap();
    let snapshot = SourceSnapshot::capture(
        project.id,
        &tree,
        TimestampUtc::from_millis(1_700_000_000_000),
    );
    let mut engine = engine_for(
        project.id,
        scenario.id,
        RunId::new(),
        snapshot.clone(),
        TimestampUtc::from_millis(1_700_000_000_000),
    );
    let initial_events = engine.drain_events();
    database
        .start_run(engine.run(), &snapshot, &initial_events)
        .unwrap();

    engine.advance_ticks(10);
    let mut events = engine.drain_events();
    assert!(!events.is_empty());
    let run_id = engine.run().id;
    if events.len() > 1 {
        events.remove(1);
        let error = database.append_events(run_id, &events).unwrap_err();
        assert!(matches!(error, StorageError::Validation(_)));
        assert_eq!(
            database.count_events(run_id).unwrap(),
            initial_events.len() as u64,
            "failed batch must roll back entirely"
        );
    }

    let logs = engine.drain_logs();
    assert!(!logs.is_empty());
    let mut gap = logs.clone();
    gap[0].seq += 100;
    let error = database.append_logs(&gap).unwrap_err();
    assert!(matches!(error, StorageError::Validation(_)));
    database.append_logs(&logs).unwrap();

    let metrics = engine.drain_metrics();
    database.append_metrics(&metrics).unwrap();
    database.append_metrics(&metrics).unwrap();
    assert_eq!(
        database.count_metrics(run_id).unwrap(),
        metrics.len() as u64
    );
}

#[test]
fn history_uses_keyset_pagination_newest_first() {
    let temp = TempDir::new("history");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("History");
    let scenario = scenario_record(project.id);
    database
        .create_project_with_scenario(&project, &tree, Some(&scenario))
        .unwrap();

    let mut runs = Vec::new();
    for offset in [0i64, 1_000, 2_000] {
        let snapshot = SourceSnapshot::capture(
            project.id,
            &tree,
            TimestampUtc::from_millis(1_700_000_000_000 + offset),
        );
        let mut engine = engine_for(
            project.id,
            scenario.id,
            RunId::new(),
            snapshot.clone(),
            TimestampUtc::from_millis(1_700_000_000_000 + offset),
        );
        let events = engine.drain_events();
        database
            .start_run(engine.run(), &snapshot, &events)
            .unwrap();
        runs.push(engine.run().id);
    }
    runs.sort();

    let all = database
        .list_run_history(&RunHistoryFilter {
            limit: 10,
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert_eq!(all.len(), 3);
    assert!(all[0].started_at >= all[1].started_at);
    assert!(all[1].started_at >= all[2].started_at);

    let first = database
        .list_run_history(&RunHistoryFilter {
            limit: 1,
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert_eq!(first.len(), 1);
    let second = database
        .list_run_history(&RunHistoryFilter {
            limit: 1,
            before: Some((first[0].started_at, first[0].run_id)),
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert_eq!(second.len(), 1);
    assert_ne!(first[0].run_id, second[0].run_id);

    let filtered = database
        .list_run_history(&RunHistoryFilter {
            project_id: Some(project.id),
            limit: 10,
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 3);
    let other = database
        .list_run_history(&RunHistoryFilter {
            project_id: Some(koncet_domain::ProjectId::new()),
            limit: 10,
            ..RunHistoryFilter::default()
        })
        .unwrap();
    assert!(other.is_empty());

    assert!(database.delete_scenario(scenario.id).is_err());
    assert_eq!(database.delete_run_history(Some(project.id)).unwrap(), 3);
    database.delete_scenario(scenario.id).unwrap();
    assert!(database.load_scenario(scenario.id).unwrap().is_none());

    database.delete_project(project.id).unwrap();
    assert!(database.load_project(project.id).unwrap().is_none());
}

#[test]
fn settings_are_typed_and_bounded() {
    let temp = TempDir::new("settings");
    let mut database = open(&temp);
    let now = TimestampUtc::from_millis(1_700_000_000_000);

    database.set_setting("theme_mode", &"dark", now).unwrap();
    database.set_setting("default_speed", &2u8, now).unwrap();
    assert_eq!(
        database.get_setting::<String>("theme_mode").unwrap(),
        Some("dark".to_string())
    );
    assert_eq!(
        database.get_setting::<u8>("default_speed").unwrap(),
        Some(2)
    );
    assert_eq!(database.list_settings().unwrap().len(), 2);

    database.delete_setting("theme_mode").unwrap();
    assert!(database
        .get_setting::<String>("theme_mode")
        .unwrap()
        .is_none());

    assert!(database.set_setting("api_token", &"secret", now).is_err());
    assert!(database.set_setting("Theme", &"dark", now).is_err());
    let null = serde_json::Value::Null;
    assert!(database
        .set_setting_json("window_geometry", &null, now)
        .is_err());

    database
        .set_setting(
            "window_geometry",
            &serde_json::json!({"width": 1280, "height": 800}),
            now,
        )
        .unwrap();
    assert_eq!(
        database
            .get_setting::<serde_json::Value>("window_geometry")
            .unwrap()
            .unwrap()["width"],
        1280
    );
}

#[test]
fn backups_are_verified_listed_and_pruned() {
    let temp = TempDir::new("backup");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("Backups");
    database.create_project(&project, &tree).unwrap();

    let first = database
        .backup(
            BackupKind::Manual,
            TimestampUtc::from_millis(1_700_000_000_000),
        )
        .unwrap();
    assert!(first.exists());
    assert_eq!(
        fs::metadata(&first).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let verification = Connection::open(&first).unwrap();
    let check: String = verification
        .query_row("PRAGMA integrity_check;", [], |row| row.get(0))
        .unwrap();
    assert_eq!(check, "ok");
    drop(verification);

    for step in 1..=7 {
        database
            .backup(
                BackupKind::Manual,
                TimestampUtc::from_millis(1_700_000_000_000 + step),
            )
            .unwrap();
    }
    let backups = database.list_backups().unwrap();
    let manual: Vec<_> = backups
        .iter()
        .filter(|backup| backup.kind == BackupKind::Manual)
        .collect();
    assert_eq!(manual.len(), 5);
    assert_eq!(
        manual[0].created_at,
        TimestampUtc::from_millis(1_700_000_000_007)
    );

    database.delete_backup(&manual[0].path).unwrap();
    assert_eq!(database.list_backups().unwrap().len(), 4);

    database
        .backup(
            BackupKind::PreMigration,
            TimestampUtc::from_millis(1_700_000_000_100),
        )
        .unwrap();
    database
        .backup(
            BackupKind::PreMigration,
            TimestampUtc::from_millis(1_700_000_000_200),
        )
        .unwrap();
    let pre: Vec<_> = database
        .list_backups()
        .unwrap()
        .into_iter()
        .filter(|backup| backup.kind == BackupKind::PreMigration)
        .collect();
    assert_eq!(pre.len(), 1);
    assert_eq!(
        pre[0].created_at,
        TimestampUtc::from_millis(1_700_000_000_200)
    );
}

#[test]
fn instance_lock_detects_live_and_stale_owners() {
    let temp = TempDir::new("lock");
    let paths = temp.paths();
    let database = Database::open(paths.clone()).unwrap();
    match Database::open(paths.clone()) {
        Err(StorageError::LockHeld { pid }) => assert_eq!(pid, std::process::id()),
        Err(other) => panic!("expected lock held, got {other}"),
        Ok(_) => panic!("expected lock held, got a second database"),
    }
    database.shutdown().unwrap();

    fs::write(paths.lock(), "999999\nstale-token\n").unwrap();
    match Database::open(paths.clone()) {
        Err(StorageError::LockStale { pid }) => assert_eq!(pid, 999_999),
        Err(other) => panic!("expected stale lock, got {other}"),
        Ok(_) => panic!("expected stale lock, got a database"),
    }
    let recovered = Database::open_with(
        paths.clone(),
        OpenOptions {
            break_stale_lock: true,
            ..OpenOptions::default()
        },
    )
    .unwrap();
    recovered.shutdown().unwrap();
    assert!(!paths.lock().exists());

    fs::write(
        paths.lock(),
        format!("{}\nlive-token\n", std::process::id()),
    )
    .unwrap();
    assert!(matches!(
        Database::open(paths.clone()),
        Err(StorageError::LockHeld { .. })
    ));
    fs::remove_file(paths.lock()).unwrap();
}

#[test]
fn corrupt_database_is_rejected_and_lock_released() {
    let temp = TempDir::new("corrupt");
    let paths = temp.paths();
    fs::create_dir_all(paths.root()).unwrap();
    fs::write(paths.database(), b"this is not a sqlite database").unwrap();
    assert!(Database::open(paths.clone()).is_err());
    assert!(!paths.lock().exists());
}

#[test]
fn read_connection_is_query_only() {
    let temp = TempDir::new("reader");
    let database = open(&temp);
    let reader = database.reader().unwrap();
    assert_eq!(reader.schema_version().unwrap(), 1);
    assert!(reader
        .conn()
        .execute(
            "INSERT INTO app_settings(key, value_json, updated_at) VALUES ('x', '1', 1)",
            [],
        )
        .is_err());
    drop(reader);
    database.shutdown().unwrap();
}

#[test]
fn scenario_used_by_history_cannot_be_deleted() {
    let temp = TempDir::new("scenario-fk");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("FK");
    let scenario = scenario_record(project.id);
    database
        .create_project_with_scenario(&project, &tree, Some(&scenario))
        .unwrap();
    let snapshot = SourceSnapshot::capture(
        project.id,
        &tree,
        TimestampUtc::from_millis(1_700_000_000_000),
    );
    let mut engine = engine_for(
        project.id,
        scenario.id,
        RunId::new(),
        snapshot.clone(),
        TimestampUtc::from_millis(1_700_000_000_000),
    );
    let events = engine.drain_events();
    database
        .start_run(engine.run(), &snapshot, &events)
        .unwrap();

    assert!(matches!(
        database.delete_scenario(scenario.id),
        Err(StorageError::Conflict(_))
    ));
    database.delete_project(project.id).unwrap();
    assert!(database.load_project(project.id).unwrap().is_none());
    assert!(database
        .list_run_history(&RunHistoryFilter::default())
        .unwrap()
        .is_empty());
}

#[test]
fn build_failure_persists_without_image_or_deployment() {
    let temp = TempDir::new("build-failure");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("Failure");
    let mut scenario = scenario_record(project.id);
    scenario.scenario = ScenarioPreset::BuildFailure.scenario();
    scenario.canonical_json = scenario.scenario.canonical_json();
    database
        .create_project_with_scenario(&project, &tree, Some(&scenario))
        .unwrap();
    let snapshot = SourceSnapshot::capture(
        project.id,
        &tree,
        TimestampUtc::from_millis(1_700_000_000_000),
    );
    let run_id = RunId::new();
    let mut engine = SimulationEngine::start(SimulationInputs {
        run_id,
        project_id: project.id,
        scenario_id: scenario.id,
        scenario: scenario.scenario.clone(),
        source_snapshot: snapshot.clone(),
        seed: 7,
        started_at: TimestampUtc::from_millis(1_700_000_000_000),
        app_version: "0.1.0".into(),
        engine_version: "0.1.0".into(),
    })
    .unwrap();
    let initial = engine.drain_events();
    database
        .start_run(engine.run(), &snapshot, &initial)
        .unwrap();
    engine.run_to_completion(2_000);
    let events = engine.drain_events();
    let logs = engine.drain_logs();
    let metrics = engine.drain_metrics();
    database
        .persist_run_state(
            &RunState {
                run: engine.run(),
                build: engine.build(),
                image: engine.image(),
                container: engine.container(),
                deployment: engine.deployment(),
                at: engine.clock(),
            },
            &events,
            &logs,
            &metrics,
        )
        .unwrap();

    let persisted = database.load_run(run_id).unwrap().unwrap();
    assert_eq!(persisted.run.result, RunResult::BuildFailed);
    assert!(persisted.image.is_none());
    assert!(persisted.container.is_none());
    assert!(persisted.deployment.is_none());
    let build = persisted.build.unwrap();
    assert_eq!(
        build.stage(BuildStageName::Build).unwrap().status,
        koncet_domain::BuildStageStatus::Failed
    );
    assert_eq!(
        build.stage(BuildStageName::Package).unwrap().status,
        koncet_domain::BuildStageStatus::Skipped
    );
}

#[test]
fn schema_rejects_invalid_status_text() {
    let temp = TempDir::new("invalid");
    let paths = temp.paths();
    let database = Database::open(paths.clone()).unwrap();
    database.shutdown().unwrap();

    let conn = Connection::open(paths.database()).unwrap();
    assert!(conn
        .execute(
            "INSERT INTO projects(id, name, description, template_id, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000000', 'x', '', 'nope', 0, 0)",
            [],
        )
        .is_ok());
    assert!(conn
        .execute("UPDATE projects SET name = '' WHERE name = 'x'", [])
        .is_err());
    assert!(conn
        .execute(
            "INSERT INTO runs(id, project_id, scenario_id, scenario_name,
                              scenario_schema_version, scenario_snapshot_json,
                              source_snapshot_id, seed, app_version, engine_version,
                              status, result, started_at)
             VALUES ('10000000-0000-0000-0000-000000000000',
                     '00000000-0000-0000-0000-000000000000',
                     '20000000-0000-0000-0000-000000000000', 'x', 1, '{}',
                     '30000000-0000-0000-0000-000000000000', '0', '0.1.0', '0.1.0',
                     'flying', 'healthy', 0)",
            [],
        )
        .is_err());
}

#[test]
fn source_tree_round_trip_preserves_bytes_and_hash() {
    let temp = TempDir::new("tree");
    let mut database = open(&temp);
    let (project, _) = project_and_tree("Tree");
    let tree = SourceTree::new(vec![
        koncet_domain::SourceFile::new("index.html", "<h1>héllo</h1>"),
        koncet_domain::SourceFile::new("styles.css", "body { color: red; }"),
    ])
    .unwrap();
    database.create_project(&project, &tree).unwrap();
    let loaded = database.load_project(project.id).unwrap().unwrap();
    assert_eq!(loaded.files.content_hash(), tree.content_hash());
    assert_eq!(loaded.files.total_bytes(), tree.total_bytes());
}

#[test]
fn usage_reports_row_counts_and_backup_bytes() {
    let temp = TempDir::new("usage");
    let mut database = open(&temp);
    let (project, tree) = project_and_tree("Usage");
    database.create_project(&project, &tree).unwrap();
    database
        .backup(
            BackupKind::Manual,
            TimestampUtc::from_millis(1_700_000_000_000),
        )
        .unwrap();
    let usage = database.usage().unwrap();
    assert_eq!(usage.project_count, 1);
    assert_eq!(usage.run_count, 0);
    assert!(usage.backup_bytes > 0);
    assert!(usage.database_bytes > 0);
}
