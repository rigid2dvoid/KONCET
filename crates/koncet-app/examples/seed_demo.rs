//! Seed a local database with representative runs for screenshots and demos.
//!
//! Run against a temporary XDG data directory:
//!   XDG_DATA_HOME=/tmp/koncet-demo cargo run -p koncet-app --example seed_demo
//!
//! Then launch KONCET with the same XDG_DATA_HOME. This tool never executes
//! application source and never touches the network.

use std::sync::Arc;

use koncet_app::{App, CreateProjectInput, StartRunInput, SystemClock};
use koncet_domain::{ProjectId, ProjectTemplate, RunId, ScenarioId, ScenarioPreset};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::open_default(Arc::new(SystemClock))?;
    if !app.list_projects(true, 10)?.is_empty() {
        eprintln!(
            "Refusing to seed: this data directory already contains projects. Use a fresh XDG_DATA_HOME."
        );
        return Ok(());
    }

    let marketing = app.create_project(CreateProjectInput {
        name: "Marketing site".into(),
        description: "Static landing page deployed through a staged build.".into(),
        template: ProjectTemplate::StaticWeb,
    })?;
    let api = app.create_project(CreateProjectInput {
        name: "Payments API".into(),
        description: "An HTTP service with a health endpoint and restart policy.".into(),
        template: ProjectTemplate::HttpService,
    })?;

    let marketing_scenarios = app.list_scenarios(marketing.project.id)?;
    let api_scenarios = app.list_scenarios(api.project.id)?;
    let marketing_happy = marketing_scenarios[0].id;
    let api_happy = api_scenarios[0].id;

    let build_failure =
        app.create_scenario(marketing.project.id, ScenarioPreset::BuildFailure, None)?;
    let blocked_port =
        app.create_scenario(marketing.project.id, ScenarioPreset::BlockedPort, None)?;
    let crash_loop = app.create_scenario(api.project.id, ScenarioPreset::CrashLoop, None)?;

    let healthy = run_to_healthy(&mut app, api.project.id, api_happy, "424242")?;
    let marketing_ok = run_to_healthy(&mut app, marketing.project.id, marketing_happy, "5")?;
    run_build_failure(&mut app, marketing.project.id, build_failure.id)?;
    run_blocked_port(&mut app, marketing.project.id, blocked_port.id)?;
    run_crash_loop(&mut app, api.project.id, crash_loop.id)?;

    app.set_setting("default_speed", &8)?;
    app.shutdown()?;

    println!("Demo data seeded.");
    println!("HEALTHY_RUN={healthy}");
    println!("MARKETING_RUN={marketing_ok}");
    println!("Marketing project: {}", marketing.project.id);
    println!("API project: {}", api.project.id);
    Ok(())
}

fn advance(app: &mut App, times: usize) {
    for _ in 0..times {
        let _ = app.advance_active(60_000).expect("advance demo run");
        if app.active_run_id().is_none() {
            break;
        }
    }
}

fn run_to_healthy(
    app: &mut App,
    project: ProjectId,
    scenario: ScenarioId,
    seed: &str,
) -> Result<RunId, Box<dyn std::error::Error>> {
    let handle = app.start_run(StartRunInput {
        project_id: project,
        scenario_id: scenario,
        seed: Some(seed.into()),
    })?;
    app.set_run_speed(handle.run_id, 8)?;
    advance(app, 3);
    if app.active_run_id().is_some() {
        app.stop_run(handle.run_id)?;
    }
    Ok(handle.run_id)
}

fn run_build_failure(
    app: &mut App,
    project: ProjectId,
    scenario: ScenarioId,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.start_run(StartRunInput {
        project_id: project,
        scenario_id: scenario,
        seed: Some("7".into()),
    })?;
    app.set_run_speed(handle.run_id, 8)?;
    advance(app, 3);
    Ok(())
}

fn run_blocked_port(
    app: &mut App,
    project: ProjectId,
    scenario: ScenarioId,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.start_run(StartRunInput {
        project_id: project,
        scenario_id: scenario,
        seed: Some("11".into()),
    })?;
    app.set_run_speed(handle.run_id, 8)?;
    advance(app, 3);
    if app.active_run_id().is_some() {
        app.stop_run(handle.run_id)?;
    }
    Ok(())
}

fn run_crash_loop(
    app: &mut App,
    project: ProjectId,
    scenario: ScenarioId,
) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.start_run(StartRunInput {
        project_id: project,
        scenario_id: scenario,
        seed: Some("3".into()),
    })?;
    app.set_run_speed(handle.run_id, 8)?;
    advance(app, 6);
    if app.active_run_id().is_some() {
        app.stop_run(handle.run_id)?;
    }
    Ok(())
}
