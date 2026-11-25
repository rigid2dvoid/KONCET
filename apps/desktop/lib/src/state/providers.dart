import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../bridge/api.dart';
import '../rust/dto.dart';

final apiProvider = Provider<KoncetApi>((ref) => const KoncetApi());

/// Startup result. A failure keeps the recovery screen in control.
class BootstrapController extends AsyncNotifier<AppInfoDto> {
  @override
  Future<AppInfoDto> build() => ref.read(apiProvider).initApp();

  Future<void> retry({bool allowStaleLock = false}) async {
    state = const AsyncLoading();
    state = await AsyncValue.guard(
      () => ref.read(apiProvider).initApp(allowStaleLock: allowStaleLock),
    );
  }
}

final bootstrapProvider =
    AsyncNotifierProvider<BootstrapController, AppInfoDto>(
      BootstrapController.new,
    );

final templatesProvider = FutureProvider<List<TemplateDto>>(
  (ref) => ref.read(apiProvider).listTemplates(),
);

final scenarioPresetsProvider = FutureProvider<List<PresetDto>>(
  (ref) => ref.read(apiProvider).listScenarioPresets(),
);

final projectsProvider = FutureProvider.family<List<ProjectSummaryDto>, bool>(
  (ref, includeArchived) => ref
      .read(apiProvider)
      .listProjects(includeArchived: includeArchived, limit: 500),
);

final projectProvider = FutureProvider.family<ProjectDetailDto, String>(
  (ref, projectId) => ref.read(apiProvider).getProject(projectId),
);

final projectScenariosProvider =
    FutureProvider.family<List<ScenarioSummaryDto>, String>(
      (ref, projectId) => ref.read(apiProvider).listScenarios(projectId),
    );

final projectRecentRunsProvider =
    FutureProvider.family<List<RunHistoryItemDto>, String>(
      (ref, projectId) =>
          ref.read(apiProvider).listRunHistory(projectId: projectId, limit: 10),
    );

final runHistoryProvider = FutureProvider<List<RunHistoryItemDto>>(
  (ref) => ref.read(apiProvider).listRunHistory(limit: 500),
);

final scenarioProvider = FutureProvider.family<ScenarioDetailDto, String>(
  (ref, scenarioId) => ref.read(apiProvider).getScenario(scenarioId),
);

final runDetailProvider = FutureProvider.family<RunDetailDto, String>(
  (ref, runId) => ref.read(apiProvider).getRun(runId),
);

final runEventsProvider = FutureProvider.family<List<EventDto>, String>(
  (ref, runId) => ref
      .read(apiProvider)
      .getRunEvents(runId, includeDebug: false, limit: 10000),
);

final runLogsProvider = FutureProvider.family<List<LogDto>, String>(
  (ref, runId) => ref.read(apiProvider).tailRunLogs(runId, limit: 5000),
);

class MetricQuery {
  const MetricQuery({
    required this.runId,
    required this.entity,
    required this.entityId,
  });

  final String runId;
  final MetricEntityDto entity;
  final String entityId;

  @override
  bool operator ==(Object other) =>
      other is MetricQuery &&
      other.runId == runId &&
      other.entity == entity &&
      other.entityId == entityId;

  @override
  int get hashCode => Object.hash(runId, entity, entityId);
}

final runMetricsProvider = FutureProvider.family<List<MetricDto>, MetricQuery>(
  (ref, query) => ref
      .read(apiProvider)
      .getRunMetrics(
        query.runId,
        entity: query.entity,
        entityId: query.entityId,
        limit: 20000,
      ),
);

class HistoricalMetrics {
  const HistoricalMetrics({
    this.build = const [],
    this.container = const [],
    this.deployment = const [],
  });

  final List<MetricDto> build;
  final List<MetricDto> container;
  final List<MetricDto> deployment;

  List<MetricDto> get all => [...build, ...container, ...deployment];
}

final historicalMetricsProvider =
    FutureProvider.family<HistoricalMetrics, String>((ref, runId) async {
      final detail = await ref.read(apiProvider).getRun(runId);
      final api = ref.read(apiProvider);
      final buildId = detail.build?.id;
      final containerId = detail.container?.id;
      final deploymentId = detail.deployment?.id;
      final build = buildId == null
          ? const <MetricDto>[]
          : await api.getRunMetrics(
              runId,
              entity: MetricEntityDto.build,
              entityId: buildId,
              limit: 20000,
            );
      final container = containerId == null
          ? const <MetricDto>[]
          : await api.getRunMetrics(
              runId,
              entity: MetricEntityDto.container,
              entityId: containerId,
              limit: 20000,
            );
      final deployment = deploymentId == null
          ? const <MetricDto>[]
          : await api.getRunMetrics(
              runId,
              entity: MetricEntityDto.deployment,
              entityId: deploymentId,
              limit: 20000,
            );
      return HistoricalMetrics(
        build: build,
        container: container,
        deployment: deployment,
      );
    });

final settingsEntriesProvider = FutureProvider<List<SettingEntryDto>>(
  (ref) => ref.read(apiProvider).listSettings(),
);

final usageProvider = FutureProvider<StorageUsageDto>(
  (ref) => ref.read(apiProvider).getDataUsage(),
);

final backupsProvider = FutureProvider<List<BackupInfoDto>>(
  (ref) => ref.read(apiProvider).listBackups(),
);

final diagnosticsProvider = FutureProvider<DiagnosticsDto>(
  (ref) => ref.read(apiProvider).getDiagnosticsInfo(),
);

/// Bounded live state for the single active run.
class ActiveRunState {
  const ActiveRunState({
    required this.handle,
    this.frame,
    this.events = const [],
    this.logs = const [],
    this.metrics = const [],
    this.finalised = false,
    this.error,
  });

  final RunHandleDto handle;
  final RunFrameDto? frame;
  final List<EventDto> events;
  final List<LogDto> logs;
  final List<MetricDto> metrics;
  final bool finalised;
  final String? error;

  ActiveRunState copyWith({
    RunFrameDto? frame,
    List<EventDto>? events,
    List<LogDto>? logs,
    List<MetricDto>? metrics,
    bool? finalised,
    String? error,
    bool clearError = false,
  }) => ActiveRunState(
    handle: handle,
    frame: frame ?? this.frame,
    events: events ?? this.events,
    logs: logs ?? this.logs,
    metrics: metrics ?? this.metrics,
    finalised: finalised ?? this.finalised,
    error: clearError ? null : (error ?? this.error),
  );
}

class ActiveRunController extends Notifier<ActiveRunState?> {
  StreamSubscription<SessionFrameDto>? _session;
  StreamSubscription<LogBatchDto>? _logSubscription;
  StreamSubscription<MetricBatchDto>? _metricSubscription;

  @override
  ActiveRunState? build() {
    ref.onDispose(_cancelSubscriptions);
    return null;
  }

  Future<RunHandleDto> start({
    required String projectId,
    required String scenarioId,
    String? seed,
  }) async {
    final current = state;
    if (current != null && !current.finalised) {
      throw StateError('A simulation run is already active.');
    }
    _cancelSubscriptions();
    state = null;
    final api = ref.read(apiProvider);
    final handle = await api.startRun(
      projectId: projectId,
      scenarioId: scenarioId,
      seed: seed,
    );
    state = ActiveRunState(handle: handle);
    final initial = await api.getRunEvents(handle.runId, limit: 10000);
    final currentState = state;
    if (currentState != null && currentState.handle.runId == handle.runId) {
      state = currentState.copyWith(events: _mergeEvents(const [], initial));
    }
    _session = api
        .subscribeSession(handle.runId)
        .listen(
          _onFrame,
          onError: (Object error) {
            final current = state;
            if (current != null) {
              state = current.copyWith(error: error.toString());
            }
          },
        );
    _logSubscription = api
        .subscribeLogs(handle.runId)
        .listen(
          _onLogs,
          onError: (Object error) {
            final current = state;
            if (current != null) {
              state = current.copyWith(error: error.toString());
            }
          },
        );
    _metricSubscription = api
        .subscribeMetrics(handle.runId)
        .listen(
          _onMetrics,
          onError: (Object error) {
            final current = state;
            if (current != null) {
              state = current.copyWith(error: error.toString());
            }
          },
        );
    return handle;
  }

  Future<void> pause() => _command((api) => api.pauseRun(_runId));

  Future<void> resume() => _command((api) => api.resumeRun(_runId));

  Future<void> setSpeed(int speed) =>
      _command((api) => api.setRunSpeed(_runId, speed));

  Future<void> restartContainer() =>
      _command((api) => api.restartContainer(_runId));

  Future<void> checkConnectivity() =>
      _command((api) => api.checkConnectivity(_runId));

  Future<void> stop() => _command((api) => api.stopRun(_runId));

  Future<void> cancel() => _command((api) => api.cancelRun(_runId));

  void clear() {
    _cancelSubscriptions();
    state = null;
  }

  void dismissError() {
    final current = state;
    if (current != null) {
      state = current.copyWith(clearError: true);
    }
  }

  String get _runId {
    final current = state;
    if (current == null) {
      throw StateError('No simulation run is active.');
    }
    return current.handle.runId;
  }

  Future<void> _command(
    Future<RunUpdateDto> Function(KoncetApi api) call,
  ) async {
    final update = await call(ref.read(apiProvider));
    _applyUpdate(update);
  }

  void _applyUpdate(RunUpdateDto update) {
    final current = state;
    if (current == null || current.handle.runId != update.frame.run.id) return;
    state = current.copyWith(
      frame: update.frame,
      events: _mergeEvents(current.events, update.events),
      logs: _mergeLogs(current.logs, update.logs),
      metrics: _mergeMetrics(current.metrics, update.metrics),
      finalised: update.finalised,
      clearError: true,
    );
    if (update.finalised) {
      _afterFinalise(current);
    }
  }

  void _onFrame(SessionFrameDto frame) {
    final current = state;
    if (current == null || current.handle.runId != frame.runId) return;
    state = current.copyWith(
      frame: frame.frame,
      events: _mergeEvents(current.events, frame.events),
      finalised: frame.finalised,
      clearError: true,
    );
    if (frame.finalised) {
      _afterFinalise(current);
    }
  }

  void _afterFinalise(ActiveRunState state) {
    _cancelSubscriptions();
    ref.invalidate(projectRecentRunsProvider(state.handle.projectId));
    ref.invalidate(runHistoryProvider);
  }

  void _onLogs(LogBatchDto batch) {
    final current = state;
    if (current == null || current.handle.runId != batch.runId) return;
    state = current.copyWith(
      logs: _mergeLogs(current.logs, batch.logs),
      clearError: true,
    );
  }

  void _onMetrics(MetricBatchDto batch) {
    final current = state;
    if (current == null || current.handle.runId != batch.runId) return;
    state = current.copyWith(
      metrics: _mergeMetrics(current.metrics, batch.metrics),
    );
  }

  void _cancelSubscriptions() {
    _session?.cancel();
    _logSubscription?.cancel();
    _metricSubscription?.cancel();
    _session = null;
    _logSubscription = null;
    _metricSubscription = null;
  }
}

final activeRunProvider =
    NotifierProvider<ActiveRunController, ActiveRunState?>(
      ActiveRunController.new,
    );

List<EventDto> _mergeEvents(List<EventDto> current, List<EventDto> incoming) {
  if (incoming.isEmpty) return current;
  final bySeq = <int, EventDto>{for (final event in current) event.seq: event};
  for (final event in incoming) {
    bySeq[event.seq] = event;
  }
  final merged = bySeq.values.toList()..sort((a, b) => a.seq.compareTo(b.seq));
  return merged;
}

List<LogDto> _mergeLogs(List<LogDto> current, List<LogDto> incoming) {
  if (incoming.isEmpty) return current;
  final bySeq = <int, LogDto>{for (final log in current) log.seq: log};
  for (final log in incoming) {
    bySeq[log.seq] = log;
  }
  final merged = bySeq.values.toList()..sort((a, b) => a.seq.compareTo(b.seq));
  const keep = 5000;
  if (merged.length > keep) {
    return merged.sublist(merged.length - keep);
  }
  return merged;
}

List<MetricDto> _mergeMetrics(
  List<MetricDto> current,
  List<MetricDto> incoming,
) {
  if (incoming.isEmpty) return current;
  final seen = <String>{
    for (final metric in current)
      '${metric.entityKind.name}|${metric.entityId}|${metric.metric}|${metric.simTimeMs}',
  };
  final merged = [...current];
  for (final metric in incoming) {
    final key =
        '${metric.entityKind.name}|${metric.entityId}|${metric.metric}|${metric.simTimeMs}';
    if (seen.add(key)) merged.add(metric);
  }
  const keep = 20000;
  if (merged.length > keep) {
    return merged.sublist(merged.length - keep);
  }
  return merged;
}
