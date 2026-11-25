import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:koncet/src/bridge/api.dart';
import 'package:koncet/src/rust/dto.dart';
import 'package:koncet/src/state/providers.dart';

class _FakeApi extends KoncetApi {
  final session = StreamController<SessionFrameDto>.broadcast(sync: true);
  final logs = StreamController<LogBatchDto>.broadcast(sync: true);
  final metrics = StreamController<MetricBatchDto>.broadcast(sync: true);
  bool stopped = false;

  @override
  Future<RunHandleDto> startRun({
    required String projectId,
    required String scenarioId,
    String? seed,
  }) async => RunHandleDto(
    runId: 'run-1',
    projectId: projectId,
    scenarioId: scenarioId,
    scenarioName: 'Happy Path',
    seed: seed ?? '1',
    startedAtMs: 1000,
  );

  @override
  Future<List<EventDto>> getRunEvents(
    String runId, {
    int afterSeq = 0,
    bool includeDebug = false,
    int limit = 1000,
  }) async => [_event(seq: 1, type: 'run.created')];

  @override
  Stream<SessionFrameDto> subscribeSession(String runId) => session.stream;

  @override
  Stream<LogBatchDto> subscribeLogs(String runId) => logs.stream;

  @override
  Stream<MetricBatchDto> subscribeMetrics(String runId) => metrics.stream;

  @override
  Future<RunUpdateDto> stopRun(String runId) async {
    stopped = true;
    return RunUpdateDto(
      frame: _frame(
        clockMs: 2000,
        status: RunStatusDto.completed,
        result: RunResultDto.healthy,
        lastEventSeq: 3,
      ),
      events: [_event(seq: 3, type: 'run.finalised')],
      logs: const [],
      metrics: const [],
      commands: const [],
      finalised: true,
    );
  }
}

void main() {
  test(
    'active run controller merges committed streams and stops once',
    () async {
      final fake = _FakeApi();
      final container = ProviderContainer(
        overrides: [apiProvider.overrideWithValue(fake)],
      );
      addTearDown(container.dispose);
      addTearDown(() async {
        await fake.session.close();
        await fake.logs.close();
        await fake.metrics.close();
      });

      final notifier = container.read(activeRunProvider.notifier);
      final handle = await notifier.start(
        projectId: 'project-1',
        scenarioId: 'scenario-1',
        seed: '42',
      );
      expect(handle.runId, 'run-1');
      expect(
        container.read(activeRunProvider)?.events.single.eventType,
        'run.created',
      );

      fake.session.add(
        SessionFrameDto(
          runId: 'run-1',
          frame: _frame(clockMs: 1000, lastEventSeq: 2),
          events: [_event(seq: 2, type: 'build.stage_started')],
          watermarkSeq: 2,
          finalised: false,
        ),
      );
      fake.logs.add(
        LogBatchDto(
          runId: 'run-1',
          logs: [
            LogDto(
              runId: 'run-1',
              seq: 1,
              simTimeMs: 1000,
              source: LogSourceDto.build,
              level: SeverityDto.info,
              message: 'building',
              contextJson: '{}',
            ),
          ],
          watermarkSeq: 1,
          finalised: false,
        ),
      );
      fake.metrics.add(
        MetricBatchDto(
          runId: 'run-1',
          metrics: [
            MetricDto(
              runId: 'run-1',
              simTimeMs: 1000,
              entityKind: MetricEntityDto.container,
              entityId: 'container-1',
              metric: 'cpu_millicores',
              value: 10,
              unit: 'm',
            ),
          ],
          watermarkMs: 1000,
          finalised: false,
        ),
      );

      final state = container.read(activeRunProvider)!;
      expect(state.frame?.clockMs, 1000);
      expect(state.events.map((event) => event.seq), [1, 2]);
      expect(state.logs.single.message, 'building');
      expect(state.metrics.single.value, 10);

      await notifier.stop();
      expect(fake.stopped, isTrue);
      final finalised = container.read(activeRunProvider)!;
      expect(finalised.finalised, isTrue);
      expect(finalised.frame?.run.status, RunStatusDto.completed);
      expect(finalised.events.last.eventType, 'run.finalised');
    },
  );
}

RunFrameDto _frame({
  required int clockMs,
  int lastEventSeq = 0,
  RunStatusDto status = RunStatusDto.running,
  RunResultDto result = RunResultDto.unknown,
}) => RunFrameDto(
  run: RunDto(
    id: 'run-1',
    projectId: 'project-1',
    scenarioId: 'scenario-1',
    scenarioName: 'Happy Path',
    seed: '42',
    sourceSnapshotId: 'snapshot-1',
    scenarioSchemaVersion: 1,
    scenarioJson: '{}',
    status: status,
    result: result,
    createdAtMs: 1000,
    startedAtMs: 1000,
    simDurationMs: clockMs,
    lastEventSeq: lastEventSeq,
    appVersion: '0.1.0',
    engineVersion: '0.1.0',
  ),
  build: const BuildDto(
    id: 'build-1',
    runId: 'run-1',
    profile: BuildProfileDto.staticWeb,
    stages: [],
  ),
  clockMs: clockMs,
  speed: 1,
  paused: false,
);

EventDto _event({required int seq, required String type}) => EventDto(
  runId: 'run-1',
  seq: seq,
  simTimeMs: 1000,
  wallTimeMs: 2000,
  category: type.split('.').first,
  eventType: type,
  severity: SeverityDto.info,
  source: EventSourceDto.engine,
  schemaVersion: 1,
  payloadJson: '{}',
);
