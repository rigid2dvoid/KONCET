//! Versioned data-transfer objects for the Dart boundary.
//!
//! Every type here is plain data. Identifiers are strings, wall-clock values
//! are epoch milliseconds, simulation time is milliseconds, and JSON documents
//! travel as strings. Enums mirror the closed v0.1 domain vocabulary.

use koncet_app::{
    AppInfo, BackupInfo, DiagnosticsInfo, ProjectSummary, ProjectWithFiles, RunFrame, RunHandle,
    RunHistoryItem, RunUpdate, ScenarioSummary, StorageUsage,
};
use koncet_domain::{
    Build, BuildFailure, BuildProfile, BuildStageName, BuildStageStatus, Container, ContainerState,
    Deployment, DeploymentStatus, DomainEvent, EventSource, FailureReason, HealthStatus, LogRecord,
    LogSource, MetricEntity, MetricSample, ProcessState, ProjectTemplate, RecordedCommand,
    RunResult, RunStatus, RunSummary, Severity, SimulatedImage, SimulationRun, SourceFile,
    SourceSnapshot, SourceTree,
};

use koncet_domain::{RestartPolicy, RestartPolicyKind};

// ---------------------------------------------------------------------------
// Closed enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatusDto {
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl From<RunStatus> for RunStatusDto {
    fn from(value: RunStatus) -> Self {
        match value {
            RunStatus::Running => Self::Running,
            RunStatus::Paused => Self::Paused,
            RunStatus::Completed => Self::Completed,
            RunStatus::Cancelled => Self::Cancelled,
            RunStatus::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunResultDto {
    Healthy,
    Degraded,
    BuildFailed,
    RuntimeFailed,
    Unknown,
}

impl From<RunResult> for RunResultDto {
    fn from(value: RunResult) -> Self {
        match value {
            RunResult::Healthy => Self::Healthy,
            RunResult::Degraded => Self::Degraded,
            RunResult::BuildFailed => Self::BuildFailed,
            RunResult::RuntimeFailed => Self::RuntimeFailed,
            RunResult::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStageNameDto {
    Validate,
    Prepare,
    Build,
    Package,
}

impl From<BuildStageName> for BuildStageNameDto {
    fn from(value: BuildStageName) -> Self {
        match value {
            BuildStageName::Validate => Self::Validate,
            BuildStageName::Prepare => Self::Prepare,
            BuildStageName::Build => Self::Build,
            BuildStageName::Package => Self::Package,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStageStatusDto {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

impl From<BuildStageStatus> for BuildStageStatusDto {
    fn from(value: BuildStageStatus) -> Self {
        match value {
            BuildStageStatus::Pending => Self::Pending,
            BuildStageStatus::Running => Self::Running,
            BuildStageStatus::Succeeded => Self::Succeeded,
            BuildStageStatus::Failed => Self::Failed,
            BuildStageStatus::Skipped => Self::Skipped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfileDto {
    StaticWeb,
    HttpService,
}

impl From<BuildProfile> for BuildProfileDto {
    fn from(value: BuildProfile) -> Self {
        match value {
            BuildProfile::StaticWeb => Self::StaticWeb,
            BuildProfile::HttpService => Self::HttpService,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicyKindDto {
    No,
    OnFailure,
    Always,
}

impl From<RestartPolicyKind> for RestartPolicyKindDto {
    fn from(value: RestartPolicyKind) -> Self {
        match value {
            RestartPolicyKind::No => Self::No,
            RestartPolicyKind::OnFailure => Self::OnFailure,
            RestartPolicyKind::Always => Self::Always,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStateDto {
    NotStarted,
    Running,
    Exited,
    Failed,
}

impl From<ProcessState> for ProcessStateDto {
    fn from(value: ProcessState) -> Self {
        match value {
            ProcessState::NotStarted => Self::NotStarted,
            ProcessState::Running => Self::Running,
            ProcessState::Exited => Self::Exited,
            ProcessState::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStateDto {
    Created,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Removed,
}

impl From<ContainerState> for ContainerStateDto {
    fn from(value: ContainerState) -> Self {
        match value {
            ContainerState::Created => Self::Created,
            ContainerState::Starting => Self::Starting,
            ContainerState::Running => Self::Running,
            ContainerState::Stopping => Self::Stopping,
            ContainerState::Stopped => Self::Stopped,
            ContainerState::Failed => Self::Failed,
            ContainerState::Removed => Self::Removed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatusDto {
    Unknown,
    Starting,
    Healthy,
    Unhealthy,
}

impl From<HealthStatus> for HealthStatusDto {
    fn from(value: HealthStatus) -> Self {
        match value {
            HealthStatus::Unknown => Self::Unknown,
            HealthStatus::Starting => Self::Starting,
            HealthStatus::Healthy => Self::Healthy,
            HealthStatus::Unhealthy => Self::Unhealthy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentStatusDto {
    Pending,
    Deploying,
    Healthy,
    Degraded,
    Failed,
    Stopped,
}

impl From<DeploymentStatus> for DeploymentStatusDto {
    fn from(value: DeploymentStatus) -> Self {
        match value {
            DeploymentStatus::Pending => Self::Pending,
            DeploymentStatus::Deploying => Self::Deploying,
            DeploymentStatus::Healthy => Self::Healthy,
            DeploymentStatus::Degraded => Self::Degraded,
            DeploymentStatus::Failed => Self::Failed,
            DeploymentStatus::Stopped => Self::Stopped,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReasonDto {
    BuildFailed,
    ImageMissing,
    ContainerFailed,
    RestartExhausted,
    HealthcheckFailed,
    RouteUnreachable,
    FirewallDenied,
    ProxyMisconfigured,
    UserStopped,
    AppTerminated,
}

impl From<FailureReason> for FailureReasonDto {
    fn from(value: FailureReason) -> Self {
        match value {
            FailureReason::BuildFailed => Self::BuildFailed,
            FailureReason::ImageMissing => Self::ImageMissing,
            FailureReason::ContainerFailed => Self::ContainerFailed,
            FailureReason::RestartExhausted => Self::RestartExhausted,
            FailureReason::HealthcheckFailed => Self::HealthcheckFailed,
            FailureReason::RouteUnreachable => Self::RouteUnreachable,
            FailureReason::FirewallDenied => Self::FirewallDenied,
            FailureReason::ProxyMisconfigured => Self::ProxyMisconfigured,
            FailureReason::UserStopped => Self::UserStopped,
            FailureReason::AppTerminated => Self::AppTerminated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityDto {
    Debug,
    Info,
    Warn,
    Error,
}

impl From<Severity> for SeverityDto {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Debug => Self::Debug,
            Severity::Info => Self::Info,
            Severity::Warn => Self::Warn,
            Severity::Error => Self::Error,
        }
    }
}

impl From<SeverityDto> for Severity {
    fn from(value: SeverityDto) -> Self {
        match value {
            SeverityDto::Debug => Self::Debug,
            SeverityDto::Info => Self::Info,
            SeverityDto::Warn => Self::Warn,
            SeverityDto::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSourceDto {
    Engine,
    Build,
    Container,
    Process,
    Proxy,
    Firewall,
    Deployment,
    User,
}

impl From<EventSource> for EventSourceDto {
    fn from(value: EventSource) -> Self {
        match value {
            EventSource::Engine => Self::Engine,
            EventSource::Build => Self::Build,
            EventSource::Container => Self::Container,
            EventSource::Process => Self::Process,
            EventSource::Proxy => Self::Proxy,
            EventSource::Firewall => Self::Firewall,
            EventSource::Deployment => Self::Deployment,
            EventSource::User => Self::User,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSourceDto {
    Engine,
    Build,
    Container,
    Process,
    Proxy,
    Firewall,
    Deployment,
}

impl From<LogSource> for LogSourceDto {
    fn from(value: LogSource) -> Self {
        match value {
            LogSource::Engine => Self::Engine,
            LogSource::Build => Self::Build,
            LogSource::Container => Self::Container,
            LogSource::Process => Self::Process,
            LogSource::Proxy => Self::Proxy,
            LogSource::Firewall => Self::Firewall,
            LogSource::Deployment => Self::Deployment,
        }
    }
}

impl From<LogSourceDto> for LogSource {
    fn from(value: LogSourceDto) -> Self {
        match value {
            LogSourceDto::Engine => Self::Engine,
            LogSourceDto::Build => Self::Build,
            LogSourceDto::Container => Self::Container,
            LogSourceDto::Process => Self::Process,
            LogSourceDto::Proxy => Self::Proxy,
            LogSourceDto::Firewall => Self::Firewall,
            LogSourceDto::Deployment => Self::Deployment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricEntityDto {
    Build,
    Container,
    Deployment,
}

impl From<MetricEntity> for MetricEntityDto {
    fn from(value: MetricEntity) -> Self {
        match value {
            MetricEntity::Build => Self::Build,
            MetricEntity::Container => Self::Container,
            MetricEntity::Deployment => Self::Deployment,
        }
    }
}

impl From<MetricEntityDto> for MetricEntity {
    fn from(value: MetricEntityDto) -> Self {
        match value {
            MetricEntityDto::Build => Self::Build,
            MetricEntityDto::Container => Self::Container,
            MetricEntityDto::Deployment => Self::Deployment,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplateDto {
    StaticWeb,
    HttpService,
}

impl From<ProjectTemplate> for ProjectTemplateDto {
    fn from(value: ProjectTemplate) -> Self {
        match value {
            ProjectTemplate::StaticWeb => Self::StaticWeb,
            ProjectTemplate::HttpService => Self::HttpService,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKindDto {
    Manual,
    PreMigration,
}

impl From<koncet_app::BackupKind> for BackupKindDto {
    fn from(value: koncet_app::BackupKind) -> Self {
        match value {
            koncet_app::BackupKind::Manual => Self::Manual,
            koncet_app::BackupKind::PreMigration => Self::PreMigration,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared value objects
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssueDto {
    pub code: String,
    pub path: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReportDto {
    pub valid: bool,
    pub issues: Vec<ValidationIssueDto>,
}

impl From<koncet_domain::ValidationReport> for ValidationReportDto {
    fn from(value: koncet_domain::ValidationReport) -> Self {
        Self {
            valid: value.is_valid(),
            issues: value
                .issues
                .into_iter()
                .map(|issue| ValidationIssueDto {
                    code: issue.code,
                    path: issue.path,
                    message: issue.message,
                    severity: match issue.severity {
                        koncet_domain::IssueSeverity::Error => "error".to_string(),
                        koncet_domain::IssueSeverity::Warning => "warning".to_string(),
                    },
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfoDto {
    pub app_version: String,
    pub engine_version: String,
    pub build_commit: String,
    pub build_date: String,
    pub schema_version: i64,
    pub platform: String,
    pub data_root: String,
    pub reconciled_runs: Vec<String>,
}

impl AppInfoDto {
    pub(crate) fn from_info(info: AppInfo, reconciled_runs: Vec<String>) -> Self {
        Self {
            app_version: info.app_version,
            engine_version: info.engine_version,
            build_commit: info.build_commit,
            build_date: info.build_date,
            schema_version: info.schema_version,
            platform: info.platform,
            data_root: info.data_root,
            reconciled_runs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateDto {
    pub id: String,
    pub label: String,
    pub description: String,
    pub file_count: u32,
}

// ---------------------------------------------------------------------------
// Projects and source
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template_id: ProjectTemplateDto,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub archived_at_ms: Option<i64>,
    pub file_count: u64,
    pub total_bytes: u64,
}

impl From<ProjectSummary> for ProjectSummaryDto {
    fn from(value: ProjectSummary) -> Self {
        Self {
            id: value.id.to_string(),
            name: value.name,
            description: value.description,
            template_id: value.template_id.into(),
            created_at_ms: value.created_at.as_millis(),
            updated_at_ms: value.updated_at.as_millis(),
            archived_at_ms: value.archived_at.map(|at| at.as_millis()),
            file_count: value.file_count,
            total_bytes: value.total_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFileDto {
    pub path: String,
    pub language: String,
    pub content: String,
    pub bytes: u64,
}

impl From<SourceFile> for SourceFileDto {
    fn from(value: SourceFile) -> Self {
        Self {
            language: value.language_hint().to_string(),
            bytes: value.byte_len() as u64,
            path: value.path,
            content: value.content,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTreeDto {
    pub files: Vec<SourceFileDto>,
    pub content_hash: String,
    pub total_bytes: u64,
}

impl From<SourceTree> for SourceTreeDto {
    fn from(value: SourceTree) -> Self {
        Self {
            files: value.files().iter().cloned().map(Into::into).collect(),
            content_hash: value.content_hash(),
            total_bytes: value.total_bytes() as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDetailDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template_id: ProjectTemplateDto,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub archived_at_ms: Option<i64>,
    pub files: Vec<SourceFileDto>,
    pub content_hash: String,
    pub total_bytes: u64,
}

impl From<ProjectWithFiles> for ProjectDetailDto {
    fn from(value: ProjectWithFiles) -> Self {
        let hash = value.files.content_hash();
        let total = value.files.total_bytes() as u64;
        let files = value
            .files
            .files()
            .iter()
            .cloned()
            .map(Into::into)
            .collect();
        Self {
            id: value.project.id.to_string(),
            name: value.project.name,
            description: value.project.description,
            template_id: value.project.template_id.into(),
            created_at_ms: value.project.created_at.as_millis(),
            updated_at_ms: value.project.updated_at.as_millis(),
            archived_at_ms: value.project.archived_at.map(|at| at.as_millis()),
            files,
            content_hash: hash,
            total_bytes: total,
        }
    }
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSummaryDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub schema_version: u32,
    pub revision: u32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl From<ScenarioSummary> for ScenarioSummaryDto {
    fn from(value: ScenarioSummary) -> Self {
        Self {
            id: value.id.to_string(),
            project_id: value.project_id.to_string(),
            name: value.name,
            description: value.description,
            schema_version: value.schema_version,
            revision: value.revision,
            created_at_ms: value.created_at.as_millis(),
            updated_at_ms: value.updated_at.as_millis(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioDetailDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub schema_version: u32,
    pub revision: u32,
    pub scenario_json: String,
    pub canonical_json: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl From<koncet_domain::ScenarioRecord> for ScenarioDetailDto {
    fn from(value: koncet_domain::ScenarioRecord) -> Self {
        Self {
            id: value.id.to_string(),
            project_id: value.project_id.to_string(),
            name: value.scenario.name.clone(),
            description: value.scenario.description.clone(),
            schema_version: value.schema_version,
            revision: value.revision,
            scenario_json: serde_json::to_string(&value.scenario)
                .unwrap_or_else(|_| "{}".to_string()),
            canonical_json: value.canonical_json,
            created_at_ms: value.created_at.as_millis(),
            updated_at_ms: value.updated_at.as_millis(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetDto {
    pub key: String,
    pub label: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHandleDto {
    pub run_id: String,
    pub project_id: String,
    pub scenario_id: String,
    pub scenario_name: String,
    pub seed: String,
    pub started_at_ms: i64,
}

impl From<RunHandle> for RunHandleDto {
    fn from(value: RunHandle) -> Self {
        Self {
            run_id: value.run_id.to_string(),
            project_id: value.project_id.to_string(),
            scenario_id: value.scenario_id.to_string(),
            scenario_name: value.scenario_name,
            seed: value.seed,
            started_at_ms: value.started_at.as_millis(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummaryDto {
    pub status: RunStatusDto,
    pub result: RunResultDto,
    pub primary_reason: Option<FailureReasonDto>,
    pub sim_duration_ms: u64,
    pub event_count: u64,
    pub log_count: u64,
    pub warning_count: u64,
    pub error_count: u64,
    pub build_stage: Option<BuildStageNameDto>,
    pub container_restart_count: u32,
    pub deployment_status: Option<DeploymentStatusDto>,
}

impl From<RunSummary> for RunSummaryDto {
    fn from(value: RunSummary) -> Self {
        Self {
            status: value.status.into(),
            result: value.result.into(),
            primary_reason: value.primary_reason.map(Into::into),
            sim_duration_ms: value.sim_duration_ms,
            event_count: value.event_count,
            log_count: value.log_count,
            warning_count: value.warning_count,
            error_count: value.error_count,
            build_stage: value.build_stage.map(Into::into),
            container_restart_count: value.container_restart_count,
            deployment_status: value.deployment_status.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDto {
    pub id: String,
    pub project_id: String,
    pub scenario_id: String,
    pub scenario_name: String,
    pub seed: String,
    pub source_snapshot_id: String,
    pub scenario_schema_version: u32,
    pub scenario_json: String,
    pub status: RunStatusDto,
    pub result: RunResultDto,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub ended_at_ms: Option<i64>,
    pub sim_duration_ms: u64,
    pub last_event_seq: u64,
    pub summary: Option<RunSummaryDto>,
    pub app_version: String,
    pub engine_version: String,
}

impl From<&SimulationRun> for RunDto {
    fn from(value: &SimulationRun) -> Self {
        Self {
            id: value.id.to_string(),
            project_id: value.project_id.to_string(),
            scenario_id: value.scenario_id.to_string(),
            scenario_name: value.scenario_name.clone(),
            seed: value.seed.to_string(),
            source_snapshot_id: value.source_snapshot_id.to_string(),
            scenario_schema_version: value.scenario_schema_version,
            scenario_json: value.scenario_json.clone(),
            status: value.status.into(),
            result: value.result.into(),
            created_at_ms: value.created_at.as_millis(),
            started_at_ms: value.started_at.map(|at| at.as_millis()),
            ended_at_ms: value.ended_at.map(|at| at.as_millis()),
            sim_duration_ms: value.sim_duration_ms,
            last_event_seq: value.last_event_seq,
            summary: value.summary.clone().map(Into::into),
            app_version: value.app_version.clone(),
            engine_version: value.engine_version.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildFailureDto {
    pub stage: BuildStageNameDto,
    pub code: String,
    pub message: String,
    pub at_percent: u8,
}

impl From<BuildFailure> for BuildFailureDto {
    fn from(value: BuildFailure) -> Self {
        Self {
            stage: value.stage.into(),
            code: value.code,
            message: value.message,
            at_percent: value.at_percent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildStageDto {
    pub kind: BuildStageNameDto,
    pub status: BuildStageStatusDto,
    pub planned_ms: u64,
    pub started_at_ms: Option<i64>,
    pub finished_at_ms: Option<i64>,
    pub actual_ms: Option<u64>,
    pub failure: Option<BuildFailureDto>,
}

impl From<koncet_domain::BuildStage> for BuildStageDto {
    fn from(value: koncet_domain::BuildStage) -> Self {
        Self {
            kind: value.kind.into(),
            status: value.status.into(),
            planned_ms: value.planned_ms,
            started_at_ms: value.started_at.map(|at| at.as_millis()),
            finished_at_ms: value.finished_at.map(|at| at.as_millis()),
            actual_ms: value.actual_ms,
            failure: value.failure.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDto {
    pub id: String,
    pub run_id: String,
    pub profile: BuildProfileDto,
    pub stages: Vec<BuildStageDto>,
    pub image_id: Option<String>,
    pub failure: Option<BuildFailureDto>,
}

impl From<&Build> for BuildDto {
    fn from(value: &Build) -> Self {
        Self {
            id: value.id.to_string(),
            run_id: value.run_id.to_string(),
            profile: value.profile.into(),
            stages: value.stages.iter().cloned().map(Into::into).collect(),
            image_id: value.image_id.map(|id| id.to_string()),
            failure: value.failure.clone().map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageLayerDto {
    pub name: String,
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedImageDto {
    pub id: String,
    pub run_id: String,
    pub repository: String,
    pub tag: String,
    pub digest: String,
    pub layers: Vec<ImageLayerDto>,
    pub size_bytes: u64,
    pub profile: BuildProfileDto,
    pub source_hash: String,
    pub engine_version: String,
    pub created_at_ms: i64,
}

impl From<&SimulatedImage> for SimulatedImageDto {
    fn from(value: &SimulatedImage) -> Self {
        Self {
            id: value.id.to_string(),
            run_id: value.run_id.to_string(),
            repository: value.repository.clone(),
            tag: value.tag.clone(),
            digest: value.digest.clone(),
            layers: value
                .layers
                .iter()
                .map(|layer| ImageLayerDto {
                    name: layer.name.clone(),
                    digest: layer.digest.clone(),
                    size_bytes: layer.size_bytes,
                })
                .collect(),
            size_bytes: value.size_bytes,
            profile: value.profile.into(),
            source_hash: value.source_hash.clone(),
            engine_version: value.engine_version.clone(),
            created_at_ms: value.created_at.as_millis(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartPolicyDto {
    pub kind: RestartPolicyKindDto,
    pub max_retries: Option<u32>,
}

impl From<&RestartPolicy> for RestartPolicyDto {
    fn from(value: &RestartPolicy) -> Self {
        Self {
            kind: value.policy.into(),
            max_retries: value.max_retries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheckDto {
    pub path: String,
    pub interval_s: u32,
    pub timeout_s: u32,
    pub retries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerConfigDto {
    pub cpu_millicores: u32,
    pub memory_mb: u32,
    pub env: Vec<EnvVarDto>,
    pub port: u32,
    pub restart_policy: RestartPolicyDto,
    pub health_check: HealthCheckDto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvVarDto {
    pub key: String,
    pub value: String,
}

impl From<&koncet_domain::ContainerConfig> for ContainerConfigDto {
    fn from(value: &koncet_domain::ContainerConfig) -> Self {
        Self {
            cpu_millicores: value.cpu_millicores,
            memory_mb: value.memory_mb,
            env: value
                .env
                .iter()
                .map(|(key, value)| EnvVarDto {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect(),
            port: value.port,
            restart_policy: (&value.restart_policy).into(),
            health_check: HealthCheckDto {
                path: value.health_check.path.clone(),
                interval_s: value.health_check.interval_s,
                timeout_s: value.health_check.timeout_s,
                retries: value.health_check.retries,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessDto {
    pub container_id: String,
    pub state: ProcessStateDto,
    pub attempt: u32,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub started_at_ms: Option<i64>,
    pub exited_at_ms: Option<i64>,
    pub restart_times_ms: Vec<i64>,
}

impl From<&koncet_domain::Process> for ProcessDto {
    fn from(value: &koncet_domain::Process) -> Self {
        Self {
            container_id: value.container_id.to_string(),
            state: value.state.into(),
            attempt: value.attempt,
            pid: value.pid,
            exit_code: value.exit_code,
            started_at_ms: value.started_at.map(|at| at.as_millis()),
            exited_at_ms: value.exited_at.map(|at| at.as_millis()),
            restart_times_ms: value
                .restart_times
                .iter()
                .map(|at| at.as_millis())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerDto {
    pub id: String,
    pub run_id: String,
    pub image_id: String,
    pub name: String,
    pub state: ContainerStateDto,
    pub health: HealthStatusDto,
    pub restart_count: u32,
    pub config: ContainerConfigDto,
    pub process: ProcessDto,
    pub exit_code: Option<i32>,
    pub failure_reason: Option<FailureReasonDto>,
    pub created_at_ms: i64,
    pub started_at_ms: Option<i64>,
    pub stopped_at_ms: Option<i64>,
}

impl From<&Container> for ContainerDto {
    fn from(value: &Container) -> Self {
        Self {
            id: value.id.to_string(),
            run_id: value.run_id.to_string(),
            image_id: value.image_id.to_string(),
            name: value.name.clone(),
            state: value.state.into(),
            health: value.health.into(),
            restart_count: value.restart_count,
            config: (&value.config).into(),
            process: (&value.process).into(),
            exit_code: value.exit_code,
            failure_reason: value.failure_reason.map(Into::into),
            created_at_ms: value.created_at.as_millis(),
            started_at_ms: value.started_at.map(|at| at.as_millis()),
            stopped_at_ms: value.stopped_at.map(|at| at.as_millis()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeploymentDto {
    pub id: String,
    pub run_id: String,
    pub image_id: String,
    pub container_id: String,
    pub service_name: String,
    pub status: DeploymentStatusDto,
    pub health: HealthStatusDto,
    pub primary_reason: Option<FailureReasonDto>,
    pub created_at_ms: i64,
    pub healthy_since_ms: Option<i64>,
    pub uptime_ms: u64,
}

impl From<&Deployment> for DeploymentDto {
    fn from(value: &Deployment) -> Self {
        Self {
            id: value.id.to_string(),
            run_id: value.run_id.to_string(),
            image_id: value.image_id.to_string(),
            container_id: value.container_id.to_string(),
            service_name: value.service_name.clone(),
            status: value.status.into(),
            health: value.health.into(),
            primary_reason: value.primary_reason.map(Into::into),
            created_at_ms: value.created_at.as_millis(),
            healthy_since_ms: value.healthy_since.map(|at| at.as_millis()),
            uptime_ms: value.uptime_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFrameDto {
    pub run: RunDto,
    pub build: BuildDto,
    pub image: Option<SimulatedImageDto>,
    pub container: Option<ContainerDto>,
    pub deployment: Option<DeploymentDto>,
    pub clock_ms: i64,
    pub speed: u8,
    pub paused: bool,
}

impl From<&RunFrame> for RunFrameDto {
    fn from(value: &RunFrame) -> Self {
        Self {
            run: (&value.run).into(),
            build: (&value.build).into(),
            image: value.image.as_ref().map(Into::into),
            container: value.container.as_ref().map(Into::into),
            deployment: value.deployment.as_ref().map(Into::into),
            clock_ms: value.clock_ms,
            speed: value.speed,
            paused: value.paused,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDto {
    pub run_id: String,
    pub seq: u64,
    pub sim_time_ms: i64,
    pub wall_time_ms: i64,
    pub category: String,
    pub event_type: String,
    pub severity: SeverityDto,
    pub source: EventSourceDto,
    pub schema_version: u32,
    pub payload_json: String,
}

impl From<&DomainEvent> for EventDto {
    fn from(value: &DomainEvent) -> Self {
        Self {
            run_id: value.run_id.to_string(),
            seq: value.seq,
            sim_time_ms: value.sim_time_ms,
            wall_time_ms: value.wall_time_ms,
            category: value.category.clone(),
            event_type: value.event_type.clone(),
            severity: value.severity.into(),
            source: value.source.into(),
            schema_version: value.schema_version,
            payload_json: serde_json::to_string(&value.payload)
                .unwrap_or_else(|_| "{}".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogDto {
    pub run_id: String,
    pub seq: u64,
    pub sim_time_ms: i64,
    pub source: LogSourceDto,
    pub level: SeverityDto,
    pub message: String,
    pub context_json: String,
}

impl From<&LogRecord> for LogDto {
    fn from(value: &LogRecord) -> Self {
        Self {
            run_id: value.run_id.to_string(),
            seq: value.seq,
            sim_time_ms: value.sim_time_ms,
            source: value.source.into(),
            level: value.level.into(),
            message: value.message.clone(),
            context_json: serde_json::to_string(&value.context)
                .unwrap_or_else(|_| "{}".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricDto {
    pub run_id: String,
    pub sim_time_ms: i64,
    pub entity_kind: MetricEntityDto,
    pub entity_id: String,
    pub metric: String,
    pub value: f64,
    pub unit: String,
}

impl From<&MetricSample> for MetricDto {
    fn from(value: &MetricSample) -> Self {
        Self {
            run_id: value.run_id.to_string(),
            sim_time_ms: value.sim_time_ms,
            entity_kind: value.entity_kind.into(),
            entity_id: value.entity_id.clone(),
            metric: value.metric.clone(),
            value: value.value,
            unit: value.unit.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCommandDto {
    pub seq: u64,
    pub at_ms: i64,
    pub command: String,
    pub command_json: String,
}

impl From<&RecordedCommand> for RecordedCommandDto {
    fn from(value: &RecordedCommand) -> Self {
        Self {
            seq: value.seq,
            at_ms: value.at.as_millis(),
            command: value.command.name().to_string(),
            command_json: serde_json::to_string(&value.command)
                .unwrap_or_else(|_| "{}".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunUpdateDto {
    pub frame: RunFrameDto,
    pub events: Vec<EventDto>,
    pub logs: Vec<LogDto>,
    pub metrics: Vec<MetricDto>,
    pub commands: Vec<RecordedCommandDto>,
    pub finalised: bool,
}

impl From<RunUpdate> for RunUpdateDto {
    fn from(value: RunUpdate) -> Self {
        Self {
            frame: (&value.frame).into(),
            events: value.events.iter().map(Into::into).collect(),
            logs: value.logs.iter().map(Into::into).collect(),
            metrics: value.metrics.iter().map(Into::into).collect(),
            commands: value.commands.iter().map(Into::into).collect(),
            finalised: value.finalised,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFrameDto {
    pub run_id: String,
    pub frame: RunFrameDto,
    pub events: Vec<EventDto>,
    pub watermark_seq: u64,
    pub finalised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogBatchDto {
    pub run_id: String,
    pub logs: Vec<LogDto>,
    pub watermark_seq: u64,
    pub finalised: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricBatchDto {
    pub run_id: String,
    pub metrics: Vec<MetricDto>,
    pub watermark_ms: i64,
    pub finalised: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHistoryItemDto {
    pub run_id: String,
    pub project_id: String,
    pub project_name: String,
    pub scenario_id: String,
    pub scenario_name: String,
    pub status: RunStatusDto,
    pub result: RunResultDto,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub sim_duration_ms: u64,
    pub summary: Option<RunSummaryDto>,
}

impl From<RunHistoryItem> for RunHistoryItemDto {
    fn from(value: RunHistoryItem) -> Self {
        Self {
            run_id: value.run_id.to_string(),
            project_id: value.project_id.to_string(),
            project_name: value.project_name,
            scenario_id: value.scenario_id.to_string(),
            scenario_name: value.scenario_name,
            status: value.status.into(),
            result: value.result.into(),
            started_at_ms: value.started_at.as_millis(),
            ended_at_ms: value.ended_at.map(|at| at.as_millis()),
            sim_duration_ms: value.sim_duration_ms,
            summary: value.summary.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunHistoryFilterDto {
    pub project_id: Option<String>,
    pub before_started_at_ms: Option<i64>,
    pub before_run_id: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogFilterDto {
    pub after_id: Option<i64>,
    pub level: Option<SeverityDto>,
    pub source: Option<LogSourceDto>,
    pub search: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricsFilterDto {
    pub entity: MetricEntityDto,
    pub entity_id: String,
    pub metric: Option<String>,
    pub from_ms: i64,
    pub to_ms: i64,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDetailDto {
    pub run: RunDto,
    pub build: Option<BuildDto>,
    pub image: Option<SimulatedImageDto>,
    pub container: Option<ContainerDto>,
    pub deployment: Option<DeploymentDto>,
    pub commands: Vec<RecordedCommandDto>,
}

impl From<koncet_app::PersistedRun> for RunDetailDto {
    fn from(value: koncet_app::PersistedRun) -> Self {
        Self {
            run: (&value.run).into(),
            build: value.build.as_ref().map(Into::into),
            image: value.image.as_ref().map(Into::into),
            container: value.container.as_ref().map(Into::into),
            deployment: value.deployment.as_ref().map(Into::into),
            commands: value.command_trace.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshotFileDto {
    pub path: String,
    pub language: String,
    pub content: String,
    pub content_hash: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshotDto {
    pub id: String,
    pub project_id: String,
    pub files: Vec<SourceSnapshotFileDto>,
    pub content_hash: String,
    pub total_bytes: u64,
    pub created_at_ms: i64,
}

impl From<SourceSnapshot> for SourceSnapshotDto {
    fn from(value: SourceSnapshot) -> Self {
        Self {
            id: value.id.to_string(),
            project_id: value.project_id.to_string(),
            files: value
                .files
                .into_iter()
                .map(|file| SourceSnapshotFileDto {
                    path: file.path,
                    language: file.language,
                    content: file.content,
                    content_hash: file.content_hash,
                    bytes: file.bytes,
                })
                .collect(),
            content_hash: value.content_hash,
            total_bytes: value.total_bytes,
            created_at_ms: value.created_at.as_millis(),
        }
    }
}

// ---------------------------------------------------------------------------
// Settings and system
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingEntryDto {
    pub key: String,
    pub value_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StorageUsageDto {
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub backup_bytes: u64,
    pub project_count: u64,
    pub run_count: u64,
    pub event_count: u64,
    pub log_count: u64,
    pub metric_count: u64,
}

impl From<StorageUsage> for StorageUsageDto {
    fn from(value: StorageUsage) -> Self {
        Self {
            database_bytes: value.database_bytes,
            wal_bytes: value.wal_bytes,
            backup_bytes: value.backup_bytes,
            project_count: value.project_count,
            run_count: value.run_count,
            event_count: value.event_count,
            log_count: value.log_count,
            metric_count: value.metric_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfoDto {
    pub path: String,
    pub kind: BackupKindDto,
    pub created_at_ms: i64,
    pub size_bytes: u64,
}

impl From<BackupInfo> for BackupInfoDto {
    fn from(value: BackupInfo) -> Self {
        Self {
            path: value.path.to_string_lossy().to_string(),
            kind: value.kind.into(),
            created_at_ms: value.created_at.as_millis(),
            size_bytes: value.size_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsDto {
    pub app: AppInfoDto,
    pub integrity_ok: bool,
    pub backup_count: u32,
    pub usage: StorageUsageDto,
}

impl DiagnosticsDto {
    pub(crate) fn from_info(value: DiagnosticsInfo) -> Self {
        Self {
            app: AppInfoDto::from_info(value.app, Vec::new()),
            integrity_ok: value.integrity_ok,
            backup_count: value.backup_count as u32,
            usage: value.usage.into(),
        }
    }
}
