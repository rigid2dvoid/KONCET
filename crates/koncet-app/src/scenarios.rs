//! Scenario authoring, presets, validation, and revision handling.

use koncet_domain::{
    ProjectId, ProjectTemplate, Scenario, ScenarioId, ScenarioPreset, ScenarioRecord, SourceTree,
    ValidationReport, SCENARIO_SCHEMA_VERSION,
};
use koncet_storage::{ScenarioSummary, MAX_LIST_LIMIT};

use crate::app::App;
use crate::error::{AppError, AppResult};
use crate::projects::{copy_name, profile_for};

/// Preset metadata for the scenario picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetInfo {
    pub key: String,
    pub label: String,
    pub description: String,
}

pub(crate) fn build_record(
    project_id: ProjectId,
    scenario: Scenario,
    now: koncet_domain::TimestampUtc,
) -> ScenarioRecord {
    ScenarioRecord {
        id: ScenarioId::new(),
        project_id,
        revision: 1,
        schema_version: SCENARIO_SCHEMA_VERSION,
        canonical_json: scenario.canonical_json(),
        scenario,
        created_at: now,
        updated_at: now,
    }
}

impl App {
    pub fn list_scenario_presets(&self) -> Vec<PresetInfo> {
        ScenarioPreset::ALL
            .into_iter()
            .map(|preset| PresetInfo {
                key: preset.key().to_string(),
                label: preset.label().to_string(),
                description: preset.description().to_string(),
            })
            .collect()
    }

    pub fn list_scenarios(&self, project_id: ProjectId) -> AppResult<Vec<ScenarioSummary>> {
        Ok(self.database.list_scenarios(project_id, MAX_LIST_LIMIT)?)
    }

    pub fn get_scenario(&self, scenario_id: ScenarioId) -> AppResult<ScenarioRecord> {
        self.database
            .load_scenario(scenario_id)?
            .ok_or_else(|| AppError::not_found("scenario"))
    }

    /// Creates a scenario from a preset. Names are made unique within the project.
    pub fn create_scenario(
        &mut self,
        project_id: ProjectId,
        preset: ScenarioPreset,
        name: Option<String>,
    ) -> AppResult<ScenarioRecord> {
        let detail = self.get_project(project_id)?;
        detail.project.ensure_editable()?;
        let now = self.clock.now();
        let mut scenario = preset.scenario();
        scenario.build.profile = profile_for(detail.project.template_id);
        let desired = name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| preset.label().to_string());
        scenario.name = self.unique_scenario_name(project_id, desired)?;
        let report = scenario.validate();
        if !report.is_valid() {
            return Err(AppError::Validation(report));
        }
        let record = build_record(project_id, scenario, now);
        self.database.create_scenario(&record)?;
        tracing::info!(scenario = %record.id, project = %project_id, "created scenario");
        Ok(record)
    }

    /// Saves an edit using optimistic concurrency.
    pub fn update_scenario(
        &mut self,
        scenario_id: ScenarioId,
        expected_revision: u32,
        mut scenario: Scenario,
    ) -> AppResult<ScenarioRecord> {
        let mut record = self.get_scenario(scenario_id)?;
        let project = self.get_project(record.project_id)?;
        project.project.ensure_editable()?;

        let report = scenario.validate();
        if !report.is_valid() {
            return Err(AppError::Validation(report));
        }
        scenario.name = scenario.name.trim().to_string();
        let now = self.clock.now();
        record.revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| AppError::conflict("revision_overflow", "scenario revision overflow"))?;
        record.schema_version = scenario.schema_version;
        record.canonical_json = scenario.canonical_json();
        record.scenario = scenario;
        record.updated_at = now;
        self.database.update_scenario(&record, expected_revision)?;
        Ok(record)
    }

    pub fn duplicate_scenario(&mut self, scenario_id: ScenarioId) -> AppResult<ScenarioRecord> {
        let source = self.get_scenario(scenario_id)?;
        let project = self.get_project(source.project_id)?;
        project.project.ensure_editable()?;
        let now = self.clock.now();
        let mut scenario = source.scenario;
        scenario.name =
            self.unique_scenario_name(source.project_id, copy_name(&scenario.name, " copy"))?;
        let record = build_record(source.project_id, scenario, now);
        self.database.create_scenario(&record)?;
        Ok(record)
    }

    /// Deletes a scenario unless history or the active run refers to it.
    pub fn delete_scenario(&mut self, scenario_id: ScenarioId) -> AppResult<()> {
        if self
            .active
            .as_ref()
            .is_some_and(|session| session.run().scenario_id == scenario_id)
        {
            return Err(AppError::conflict(
                "active_run_exists",
                "stop the active run before deleting its scenario",
            ));
        }
        self.database.delete_scenario(scenario_id)?;
        Ok(())
    }

    /// Validates a scenario against the project's current source tree.
    pub fn validate_scenario(
        &self,
        project_id: ProjectId,
        scenario: &Scenario,
    ) -> AppResult<ValidationReport> {
        let project = self.get_project(project_id)?;
        Ok(scenario.validate_for_run(&project.files, Some(project.project.template_id)))
    }

    /// Parses a scenario document without saving it. Run constraints are separate.
    pub fn parse_scenario_json(&self, json: &str) -> AppResult<Scenario> {
        Ok(Scenario::from_json(json)?)
    }

    /// Rebuilds the canonical JSON after an external mapper produced a scenario.
    pub fn canonical_scenario_json(&self, scenario: &Scenario) -> AppResult<String> {
        Ok(scenario.canonical_json())
    }

    /// Resolves a profile hint for the source editor, from the project template.
    pub fn project_template(&self, project_id: ProjectId) -> AppResult<ProjectTemplate> {
        Ok(self.get_project(project_id)?.project.template_id)
    }

    /// Convenience for validation UI: the source tree used for run-start checks.
    pub fn project_source_tree(&self, project_id: ProjectId) -> AppResult<SourceTree> {
        Ok(self.get_project(project_id)?.files)
    }

    fn unique_scenario_name(&self, project_id: ProjectId, desired: String) -> AppResult<String> {
        let existing: Vec<String> = self
            .database
            .list_scenarios(project_id, MAX_LIST_LIMIT)?
            .into_iter()
            .map(|summary| summary.name.trim().to_string())
            .collect();
        if !existing.iter().any(|name| name == &desired) {
            return Ok(truncate_name(&desired, 80));
        }
        for counter in 2..=999u32 {
            let candidate = copy_name(&desired, &format!(" {counter}"));
            if !existing.iter().any(|name| name == &candidate) {
                return Ok(candidate);
            }
        }
        Err(AppError::conflict(
            "scenario_name_taken",
            "too many scenarios use this name",
        ))
    }
}

fn truncate_name(name: &str, max: usize) -> String {
    let trimmed = name.trim();
    let truncated: String = trimmed.chars().take(max).collect();
    if truncated.is_empty() {
        "Untitled scenario".to_string()
    } else {
        truncated
    }
}
