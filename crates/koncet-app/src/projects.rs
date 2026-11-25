//! Project, template, and virtual source-tree use cases.

use koncet_domain::{
    validate_source_path, BuildProfile, DomainError, Project, ProjectTemplate, ScenarioPreset,
    SourceFile, SourceTree, ValidationReport,
};
use koncet_storage::{ProjectSummary, ProjectWithFiles, ScenarioSummary, MAX_LIST_LIMIT};

use crate::app::App;
use crate::error::{AppError, AppResult};
use crate::scenarios::build_record;

/// Descriptor for the New Project template picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    pub file_count: usize,
}

/// Input accepted by the New Project flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProjectInput {
    pub name: String,
    pub description: String,
    pub template: ProjectTemplate,
}

impl App {
    pub fn list_templates(&self) -> Vec<TemplateInfo> {
        ProjectTemplate::ALL
            .into_iter()
            .map(|template| TemplateInfo {
                id: template.as_str().to_string(),
                label: template.label().to_string(),
                description: template.description().to_string(),
                file_count: template.seed_files().len(),
            })
            .collect()
    }

    pub fn list_projects(
        &self,
        include_archived: bool,
        limit: u32,
    ) -> AppResult<Vec<ProjectSummary>> {
        Ok(self.database.list_projects(include_archived, limit)?)
    }

    pub fn get_project(&self, project_id: koncet_domain::ProjectId) -> AppResult<ProjectWithFiles> {
        self.database
            .load_project(project_id)?
            .ok_or_else(|| AppError::not_found("project"))
    }

    /// Creates the project, template source files, and its Happy Path scenario atomically.
    pub fn create_project(&mut self, input: CreateProjectInput) -> AppResult<ProjectWithFiles> {
        let now = self.clock.now();
        let project = Project::new(input.name, input.description, input.template, now)?;
        let files = SourceTree::new(input.template.seed_files())?;
        let mut baseline = ScenarioPreset::HappyPath.scenario();
        baseline.build.profile = profile_for(input.template);
        if input.template == ProjectTemplate::HttpService {
            baseline.description = "Build and expose one simulated HTTP service.".to_string();
        }
        let scenario = build_record(project.id, baseline, now);
        self.database
            .create_project_with_scenario(&project, &files, Some(&scenario))?;
        tracing::info!(project = %project.id, template = input.template.as_str(), "created project");
        Ok(ProjectWithFiles { project, files })
    }

    pub fn rename_project(
        &mut self,
        project_id: koncet_domain::ProjectId,
        name: String,
    ) -> AppResult<ProjectWithFiles> {
        let now = self.clock.now();
        let mut detail = self.get_project(project_id)?;
        detail.project.ensure_editable()?;
        detail.project.rename(name, now)?;
        self.database.update_project(&detail.project)?;
        self.get_project(project_id)
    }

    pub fn set_project_description(
        &mut self,
        project_id: koncet_domain::ProjectId,
        description: String,
    ) -> AppResult<ProjectWithFiles> {
        let now = self.clock.now();
        let mut detail = self.get_project(project_id)?;
        detail.project.ensure_editable()?;
        detail.project.set_description(description, now)?;
        self.database.update_project(&detail.project)?;
        self.get_project(project_id)
    }

    pub fn archive_project(
        &mut self,
        project_id: koncet_domain::ProjectId,
    ) -> AppResult<ProjectWithFiles> {
        let now = self.clock.now();
        let mut detail = self.get_project(project_id)?;
        detail.project.archive(now)?;
        self.database.update_project(&detail.project)?;
        self.get_project(project_id)
    }

    pub fn unarchive_project(
        &mut self,
        project_id: koncet_domain::ProjectId,
    ) -> AppResult<ProjectWithFiles> {
        let now = self.clock.now();
        let mut detail = self.get_project(project_id)?;
        detail.project.unarchive(now)?;
        self.database.update_project(&detail.project)?;
        self.get_project(project_id)
    }

    /// Deletes a project and all of its history. Refuses while its run is active.
    pub fn delete_project(&mut self, project_id: koncet_domain::ProjectId) -> AppResult<()> {
        if self
            .active
            .as_ref()
            .is_some_and(|session| session.run().project_id == project_id)
        {
            return Err(AppError::conflict(
                "active_run_exists",
                "stop the active run before deleting its project",
            ));
        }
        self.database.delete_project(project_id)?;
        tracing::info!(project = %project_id, "deleted project");
        Ok(())
    }

    /// Copies the project, its source tree, and every scenario.
    pub fn duplicate_project(
        &mut self,
        project_id: koncet_domain::ProjectId,
    ) -> AppResult<ProjectWithFiles> {
        let now = self.clock.now();
        let source = self.get_project(project_id)?;
        let summaries = self.database.list_scenarios(project_id, MAX_LIST_LIMIT)?;
        let records = self.load_scenario_records(&summaries)?;
        let project = Project::new(
            copy_name(&source.project.name, " copy"),
            source.project.description.clone(),
            source.project.template_id,
            now,
        )?;
        let copies: Vec<_> = records
            .into_iter()
            .map(|record| {
                let mut copy = build_record(project.id, record.scenario, now);
                copy.id = koncet_domain::ScenarioId::new();
                copy
            })
            .collect();
        self.database
            .create_project_with_scenarios(&project, &source.files, &copies)?;
        tracing::info!(project = %project.id, source = %project_id, "duplicated project");
        Ok(ProjectWithFiles {
            project,
            files: source.files,
        })
    }

    /// Replaces one file's content. The path must already exist.
    pub fn write_source_file(
        &mut self,
        project_id: koncet_domain::ProjectId,
        path: String,
        content: String,
    ) -> AppResult<ProjectWithFiles> {
        self.mutate_source(project_id, |files| {
            let file = files
                .iter_mut()
                .find(|file| file.path == path)
                .ok_or_else(|| AppError::not_found("source file"))?;
            file.content = content;
            Ok(())
        })
    }

    pub fn create_source_file(
        &mut self,
        project_id: koncet_domain::ProjectId,
        path: String,
        content: String,
    ) -> AppResult<ProjectWithFiles> {
        if let Err((code, message)) = validate_source_path(&path) {
            return Err(AppError::Domain(DomainError::validation(
                code, "/path", message,
            )));
        }
        self.mutate_source(project_id, |files| {
            if files.iter().any(|file| file.path == path) {
                return Err(AppError::conflict(
                    "source_file_exists",
                    "a source file already uses this path",
                ));
            }
            files.push(SourceFile::new(path, content));
            Ok(())
        })
    }

    pub fn rename_source_file(
        &mut self,
        project_id: koncet_domain::ProjectId,
        from: String,
        to: String,
    ) -> AppResult<ProjectWithFiles> {
        if let Err((code, message)) = validate_source_path(&to) {
            return Err(AppError::Domain(DomainError::validation(
                code, "/path", message,
            )));
        }
        self.mutate_source(project_id, |files| {
            if files.iter().any(|file| file.path == to) {
                return Err(AppError::conflict(
                    "source_file_exists",
                    "a source file already uses the new path",
                ));
            }
            let file = files
                .iter_mut()
                .find(|file| file.path == from)
                .ok_or_else(|| AppError::not_found("source file"))?;
            file.path = to;
            Ok(())
        })
    }

    pub fn delete_source_file(
        &mut self,
        project_id: koncet_domain::ProjectId,
        path: String,
    ) -> AppResult<ProjectWithFiles> {
        self.mutate_source(project_id, |files| {
            let index = files
                .iter()
                .position(|file| file.path == path)
                .ok_or_else(|| AppError::not_found("source file"))?;
            files.remove(index);
            Ok(())
        })
    }

    /// Integrity report for the editable tree. Scenario validation is separate.
    pub fn validate_source(
        &self,
        project_id: koncet_domain::ProjectId,
    ) -> AppResult<ValidationReport> {
        let project = self.get_project(project_id)?;
        let mut report = ValidationReport::new();
        if project.files.files().is_empty() {
            report.warning(
                "empty_source_tree",
                "/files",
                "the source tree is empty; scenarios will fail run-start validation",
            );
        }
        Ok(report)
    }

    pub fn list_project_scenarios(
        &self,
        project_id: koncet_domain::ProjectId,
    ) -> AppResult<Vec<ScenarioSummary>> {
        Ok(self.database.list_scenarios(project_id, MAX_LIST_LIMIT)?)
    }

    /// Convenience for the project overview: recent runs for one project.
    pub fn list_recent_project_runs(
        &self,
        project_id: koncet_domain::ProjectId,
        limit: u32,
    ) -> AppResult<Vec<koncet_storage::RunHistoryItem>> {
        let filter = koncet_storage::RunHistoryFilter {
            project_id: Some(project_id),
            before: None,
            limit,
        };
        Ok(self.database.list_run_history(&filter)?)
    }

    fn mutate_source<F>(
        &mut self,
        project_id: koncet_domain::ProjectId,
        mutate: F,
    ) -> AppResult<ProjectWithFiles>
    where
        F: FnOnce(&mut Vec<SourceFile>) -> AppResult<()>,
    {
        let now = self.clock.now();
        let detail = self.get_project(project_id)?;
        detail.project.ensure_editable()?;
        let mut files = detail.files.files().to_vec();
        mutate(&mut files)?;
        let tree = SourceTree::new(files)?;
        self.database.replace_source_files(project_id, &tree, now)?;
        self.get_project(project_id)
    }

    fn load_scenario_records(
        &self,
        summaries: &[ScenarioSummary],
    ) -> AppResult<Vec<koncet_domain::ScenarioRecord>> {
        let mut records = Vec::with_capacity(summaries.len());
        for summary in summaries {
            let record = self
                .database
                .load_scenario(summary.id)?
                .ok_or_else(|| AppError::not_found("scenario"))?;
            records.push(record);
        }
        Ok(records)
    }
}

/// The build profile implied by a project template.
pub(crate) const fn profile_for(template: ProjectTemplate) -> BuildProfile {
    match template {
        ProjectTemplate::StaticWeb => BuildProfile::StaticWeb,
        ProjectTemplate::HttpService => BuildProfile::HttpService,
    }
}

/// Truncates the base so `base + suffix` stays within the 80-character name limit.
pub(crate) fn copy_name(name: &str, suffix: &str) -> String {
    const MAX: usize = 80;
    let suffix_len = suffix.chars().count();
    let budget = MAX.saturating_sub(suffix_len).max(1);
    let base: String = name.trim().chars().take(budget).collect();
    let base = base.trim_end();
    let base = if base.is_empty() { "Untitled" } else { base };
    format!("{base}{suffix}")
}
