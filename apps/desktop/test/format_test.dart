import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:koncet/src/format.dart';
import 'package:koncet/src/rust/dto.dart';

void main() {
  test('durations and bytes are human readable', () {
    expect(formatDurationMs(950), '1.0 s');
    expect(formatDurationMs(65_000), '1m 5s');
    expect(formatDurationMs(3_600_000), '1h 0m');
    expect(formatBytes(512), '512 B');
    expect(formatBytes(2048), '2.0 KiB');
  });

  test('event summaries prefer message and fall back to type', () {
    final base = EventDto(
      runId: 'run',
      seq: 1,
      simTimeMs: 1_000,
      wallTimeMs: 2_000,
      category: 'build',
      eventType: 'build.failed',
      severity: SeverityDto.error,
      source: EventSourceDto.build,
      schemaVersion: 1,
      payloadJson: jsonEncode({'message': 'compiler failure'}),
    );
    expect(eventSummary(base), 'compiler failure');
    final fallback = EventDto(
      runId: 'run',
      seq: 2,
      simTimeMs: 1_000,
      wallTimeMs: 2_000,
      category: 'engine',
      eventType: 'engine.unknown',
      severity: SeverityDto.info,
      source: EventSourceDto.engine,
      schemaVersion: 1,
      payloadJson: '{}',
    );
    expect(eventSummary(fallback), 'engine.unknown');
  });

  test('status mappings cover every closed enum', () {
    for (final status in RunStatusDto.values) {
      expect(labelForRunStatus(status), isNotEmpty);
    }
    for (final result in RunResultDto.values) {
      expect(labelForResult(result), isNotEmpty);
    }
    for (final reason in FailureReasonDto.values) {
      expect(labelForFailure(reason), isNotEmpty);
      expect(explainFailure(reason), isNotEmpty);
    }
  });
}
