import '../rust/api.dart' as rust;
import '../rust/dto.dart';

/// Thin, testable façade over the generated bridge.
///
/// Screens and controllers depend on this class; tests can substitute a fake
/// without touching generated code or Rust.
class KoncetApi {
  const KoncetApi();

  Future<AppInfoDto> initApp({bool allowStaleLock = false}) =>
      rust.initApp(allowStaleLock: allowStaleLock);

  Future<AppInfoDto> initAppAt({
    required String dataRoot,
    bool allowStaleLock = false,
  }) => rust.initAppAt(dataRoot: dataRoot, allowStaleLock: allowStaleLock);

  Future<void> shutdownApp() => rust.shutdownApp();

  Future<AppInfoDto> getAppInfo() => rust.getAppInfo();

  Future<List<TemplateDto>> listTemplates() => rust.listTemplates();

  Future<List<ProjectSummaryDto>> listProjects({
    bool includeArchived = false,
    int limit = 200,
  }) => rust.listProjects(includeArchived: includeArchived, limit: limit);

  Future<ProjectDetailDto> getProject(String projectId) =>
      rust.getProject(projectId: projectId);

  Future<ProjectDetailDto> createProject({
    required String name,
    required String description,
    required ProjectTemplateDto template,
  }) => rust.createProject(
    name: name,
    description: description,
    template: template,
  );

  Future<ProjectDetailDto> renameProject(String projectId, String name) =>
      rust.renameProject(projectId: projectId, name: name);

  Future<ProjectDetailDto> setProjectDescription(
    String projectId,
    String description,
  ) => rust.setProjectDescription(
    projectId: projectId,
    description: description,
  );

  Future<ProjectDetailDto> archiveProject(String projectId) =>
      rust.archiveProject(projectId: projectId);

  Future<ProjectDetailDto> unarchiveProject(String projectId) =>
      rust.unarchiveProject(projectId: projectId);

  Future<void> deleteProject(String projectId) =>
      rust.deleteProject(projectId: projectId);

  Future<ProjectDetailDto> duplicateProject(String projectId) =>
      rust.duplicateProject(projectId: projectId);

  Future<ProjectDetailDto> writeSourceFile(
    String projectId,
    String path,
    String content,
  ) => rust.writeSourceFile(projectId: projectId, path: path, content: content);

  Future<ProjectDetailDto> createSourceFile(
    String projectId,
    String path,
    String content,
  ) =>
      rust.createSourceFile(projectId: projectId, path: path, content: content);

  Future<ProjectDetailDto> renameSourceFile(
    String projectId,
    String from,
    String to,
  ) => rust.renameSourceFile(projectId: projectId, from: from, to: to);

  Future<ProjectDetailDto> deleteSourceFile(String projectId, String path) =>
      rust.deleteSourceFile(projectId: projectId, path: path);

  Future<ValidationReportDto> validateSource(String projectId) =>
      rust.validateSource(projectId: projectId);

  Future<List<PresetDto>> listScenarioPresets() => rust.listScenarioPresets();

  Future<List<ScenarioSummaryDto>> listScenarios(String projectId) =>
      rust.listScenarios(projectId: projectId);

  Future<ScenarioDetailDto> getScenario(String scenarioId) =>
      rust.getScenario(scenarioId: scenarioId);

  Future<ScenarioDetailDto> createScenario(
    String projectId,
    String preset, {
    String? name,
  }) => rust.createScenario(projectId: projectId, preset: preset, name: name);

  Future<ScenarioDetailDto> updateScenario(
    String scenarioId,
    int expectedRevision,
    String scenarioJson,
  ) => rust.updateScenario(
    scenarioId: scenarioId,
    expectedRevision: expectedRevision,
    scenarioJson: scenarioJson,
  );

  Future<ScenarioDetailDto> duplicateScenario(String scenarioId) =>
      rust.duplicateScenario(scenarioId: scenarioId);

  Future<void> deleteScenario(String scenarioId) =>
      rust.deleteScenario(scenarioId: scenarioId);

  Future<ValidationReportDto> validateScenario(
    String projectId,
    String scenarioJson,
  ) => rust.validateScenario(projectId: projectId, scenarioJson: scenarioJson);

  Future<String> canonicalScenarioJson(String scenarioJson) =>
      rust.canonicalScenarioJson(scenarioJson: scenarioJson);

  Future<RunHandleDto> startRun({
    required String projectId,
    required String scenarioId,
    String? seed,
  }) => rust.startRun(projectId: projectId, scenarioId: scenarioId, seed: seed);

  Future<RunUpdateDto> pauseRun(String runId) => rust.pauseRun(runId: runId);

  Future<RunUpdateDto> resumeRun(String runId) => rust.resumeRun(runId: runId);

  Future<RunUpdateDto> setRunSpeed(String runId, int speed) =>
      rust.setRunSpeed(runId: runId, speed: speed);

  Future<RunUpdateDto> restartContainer(String runId) =>
      rust.restartContainer(runId: runId);

  Future<RunUpdateDto> checkConnectivity(String runId) =>
      rust.checkConnectivity(runId: runId);

  Future<RunUpdateDto> stopRun(String runId) => rust.stopRun(runId: runId);

  Future<RunUpdateDto> cancelRun(String runId) => rust.cancelRun(runId: runId);

  Future<String?> getActiveRunId() => rust.getActiveRunId();

  Future<RunFrameDto?> getActiveFrame() => rust.getActiveFrame();

  Stream<SessionFrameDto> subscribeSession(String runId) =>
      rust.subscribeSessionStream(runId: runId);

  Stream<LogBatchDto> subscribeLogs(String runId) =>
      rust.subscribeLogsStream(runId: runId);

  Stream<MetricBatchDto> subscribeMetrics(String runId) =>
      rust.subscribeMetricsStream(runId: runId);

  Future<RunDetailDto> getRun(String runId) => rust.getRun(runId: runId);

  Future<List<RunHistoryItemDto>> listRunHistory({
    String? projectId,
    int? beforeStartedAtMs,
    String? beforeRunId,
    int limit = 200,
  }) => rust.listRunHistory(
    filter: RunHistoryFilterDto(
      projectId: projectId,
      beforeStartedAtMs: beforeStartedAtMs,
      beforeRunId: beforeRunId,
      limit: limit,
    ),
  );

  Future<int> countRuns({String? projectId}) =>
      rust.countRuns(projectId: projectId);

  Future<void> deleteRun(String runId) => rust.deleteRun(runId: runId);

  Future<int> deleteRunHistory({String? projectId}) =>
      rust.deleteRunHistory(projectId: projectId);

  Future<List<EventDto>> getRunEvents(
    String runId, {
    int afterSeq = 0,
    bool includeDebug = false,
    int limit = 1000,
  }) => rust.getRunEvents(
    runId: runId,
    afterSeq: afterSeq,
    includeDebug: includeDebug,
    limit: limit,
  );

  Future<int> countRunEvents(String runId) => rust.countRunEvents(runId: runId);

  Future<List<LogDto>> getRunLogs(
    String runId, {
    int? afterId,
    SeverityDto? level,
    LogSourceDto? source,
    String? search,
    int limit = 1000,
  }) => rust.getRunLogs(
    runId: runId,
    filter: LogFilterDto(
      afterId: afterId,
      level: level,
      source: source,
      search: search,
      limit: limit,
    ),
  );

  Future<List<LogDto>> tailRunLogs(String runId, {int limit = 500}) =>
      rust.tailRunLogs(runId: runId, limit: limit);

  Future<int> countRunLogs(String runId) => rust.countRunLogs(runId: runId);

  Future<List<MetricDto>> getRunMetrics(
    String runId, {
    required MetricEntityDto entity,
    required String entityId,
    String? metric,
    int fromMs = 0,
    int toMs = 1 << 62,
    int limit = 10000,
  }) => rust.getRunMetrics(
    runId: runId,
    filter: MetricsFilterDto(
      entity: entity,
      entityId: entityId,
      metric: metric,
      fromMs: fromMs,
      toMs: toMs,
      limit: limit,
    ),
  );

  Future<int> countRunMetrics(String runId) =>
      rust.countRunMetrics(runId: runId);

  Future<SourceSnapshotDto> getSourceSnapshot(String snapshotId) =>
      rust.getSourceSnapshot(snapshotId: snapshotId);

  Future<List<SettingEntryDto>> listSettings() => rust.listSettings();

  Future<String?> getSettingJson(String key) => rust.getSettingJson(key: key);

  Future<void> setSettingJson(String key, String valueJson) =>
      rust.setSettingJson(key: key, valueJson: valueJson);

  Future<bool> deleteSetting(String key) => rust.deleteSetting(key: key);

  Future<StorageUsageDto> getDataUsage() => rust.getDataUsage();

  Future<BackupInfoDto> createBackup() => rust.createBackup();

  Future<List<BackupInfoDto>> listBackups() => rust.listBackups();

  Future<void> deleteBackup(String path) => rust.deleteBackup(path: path);

  Future<void> checkIntegrity() => rust.checkIntegrity();

  Future<void> quickCheck() => rust.quickCheck();

  Future<DiagnosticsDto> getDiagnosticsInfo() => rust.getDiagnosticsInfo();
}
