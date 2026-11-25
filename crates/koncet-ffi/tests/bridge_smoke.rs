//! End-to-end bridge smoke test. One test per process because the bridge holds
//! a process-wide database instance.

use std::fs;

use koncet_ffi::api::*;
use koncet_ffi::dto::{ProjectTemplateDto, RunHistoryFilterDto, RunStatusDto};

#[test]
fn bridge_smoke_flow() {
    let root = std::env::temp_dir().join(format!("koncet-ffi-smoke-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let info = init_app_at(root.to_string_lossy().to_string(), false).unwrap();
    assert_eq!(info.schema_version, 1);
    assert_eq!(info.app_version, "0.2.0");
    assert!(info.reconciled_runs.is_empty());

    assert_eq!(list_templates().unwrap().len(), 2);
    let project = create_project(
        "Bridge".into(),
        "smoke".into(),
        ProjectTemplateDto::StaticWeb,
    )
    .unwrap();
    let scenarios = list_scenarios(project.id.clone()).unwrap();
    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0].name, "Happy Path");
    assert!(project.files.iter().any(|file| file.path == "index.html"));

    let run = start_run(
        project.id.clone(),
        scenarios[0].id.clone(),
        Some("42".into()),
    )
    .unwrap();
    assert_eq!(run.seed, "42");
    assert_eq!(
        get_active_run_id().unwrap().as_deref(),
        Some(run.run_id.as_str())
    );

    let mut finalised = false;
    for _ in 0..20 {
        if let Some(update) = advance_session(60_000).unwrap() {
            if update.frame.deployment.is_some() {
                assert!(update.frame.image.is_some());
            }
            if update.finalised {
                finalised = true;
                break;
            }
        }
    }
    assert!(!finalised, "happy path keeps running until stopped");

    let paused = pause_run(run.run_id.clone()).unwrap();
    assert!(paused.frame.paused);
    set_run_speed(run.run_id.clone(), 8).unwrap();
    let stopped = stop_run(run.run_id.clone()).unwrap();
    assert!(stopped.finalised);
    assert_eq!(stopped.frame.run.status, RunStatusDto::Completed);

    let detail = get_run(run.run_id.clone()).unwrap();
    assert!(detail.image.is_some());
    assert!(detail.container.is_some());
    assert!(detail.deployment.is_some());
    assert!(detail.run.summary.is_some());
    assert_eq!(detail.commands.len(), 3);

    let history = list_run_history(RunHistoryFilterDto {
        limit: 10,
        ..RunHistoryFilterDto::default()
    })
    .unwrap();
    assert_eq!(history.len(), 1);
    let events = get_run_events(run.run_id.clone(), 0, true, 10_000).unwrap();
    assert!(events
        .iter()
        .any(|event| event.event_type == "build.stage_finished"));
    let logs = tail_run_logs(run.run_id.clone(), 5).unwrap();
    assert!(!logs.is_empty());

    let backup = create_backup().unwrap();
    assert!(std::path::Path::new(&backup.path).exists());
    assert_eq!(list_backups().unwrap().len(), 1);

    shutdown_app().unwrap();
    assert_eq!(get_active_run_id().unwrap(), None);
    let _ = fs::remove_dir_all(&root);
}
