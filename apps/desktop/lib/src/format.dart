import 'dart:convert';

import 'package:intl/intl.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import 'rust/dto.dart';

final _dateTime = DateFormat.yMMMd().add_Hm();

String formatTimestamp(int epochMs) => _dateTime.format(
  DateTime.fromMillisecondsSinceEpoch(epochMs, isUtc: false),
);

String formatDate(int epochMs) =>
    DateFormat.yMMMd().format(DateTime.fromMillisecondsSinceEpoch(epochMs));

String formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  const units = ['KiB', 'MiB', 'GiB', 'TiB'];
  var value = bytes / 1024;
  var index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index++;
  }
  return '${value.toStringAsFixed(value >= 100 ? 0 : 1)} ${units[index]}';
}

String formatDurationMs(int milliseconds) {
  final seconds = milliseconds ~/ 1000;
  if (seconds < 60) {
    final tenths = (milliseconds / 100).round();
    return '${(tenths / 10).toStringAsFixed(1)} s';
  }
  final minutes = seconds ~/ 60;
  final rest = seconds % 60;
  if (minutes < 60) return '${minutes}m ${rest}s';
  final hours = minutes ~/ 60;
  return '${hours}h ${minutes % 60}m';
}

KoncetStatusTone toneForRunStatus(RunStatusDto status) => switch (status) {
  RunStatusDto.running => KoncetStatusTone.active,
  RunStatusDto.paused => KoncetStatusTone.warning,
  RunStatusDto.completed => KoncetStatusTone.healthy,
  RunStatusDto.cancelled => KoncetStatusTone.stopped,
  RunStatusDto.failed => KoncetStatusTone.danger,
};

String labelForRunStatus(RunStatusDto status) => switch (status) {
  RunStatusDto.running => 'Running',
  RunStatusDto.paused => 'Paused',
  RunStatusDto.completed => 'Completed',
  RunStatusDto.cancelled => 'Cancelled',
  RunStatusDto.failed => 'Failed',
};

KoncetStatusTone toneForResult(RunResultDto result) => switch (result) {
  RunResultDto.healthy => KoncetStatusTone.healthy,
  RunResultDto.degraded => KoncetStatusTone.warning,
  RunResultDto.buildFailed => KoncetStatusTone.danger,
  RunResultDto.runtimeFailed => KoncetStatusTone.danger,
  RunResultDto.unknown => KoncetStatusTone.neutral,
};

String labelForResult(RunResultDto result) => switch (result) {
  RunResultDto.healthy => 'Healthy',
  RunResultDto.degraded => 'Degraded',
  RunResultDto.buildFailed => 'Build failed',
  RunResultDto.runtimeFailed => 'Runtime failed',
  RunResultDto.unknown => 'Unknown',
};

KoncetStatusTone toneForDeployment(DeploymentStatusDto status) =>
    switch (status) {
      DeploymentStatusDto.healthy => KoncetStatusTone.healthy,
      DeploymentStatusDto.degraded => KoncetStatusTone.warning,
      DeploymentStatusDto.failed => KoncetStatusTone.danger,
      DeploymentStatusDto.stopped => KoncetStatusTone.stopped,
      DeploymentStatusDto.deploying => KoncetStatusTone.info,
      DeploymentStatusDto.pending => KoncetStatusTone.neutral,
    };

String labelForDeployment(DeploymentStatusDto status) => switch (status) {
  DeploymentStatusDto.healthy => 'Healthy',
  DeploymentStatusDto.degraded => 'Degraded',
  DeploymentStatusDto.failed => 'Failed',
  DeploymentStatusDto.stopped => 'Stopped',
  DeploymentStatusDto.deploying => 'Deploying',
  DeploymentStatusDto.pending => 'Pending',
};

KoncetStatusTone toneForContainer(ContainerStateDto state) => switch (state) {
  ContainerStateDto.running => KoncetStatusTone.healthy,
  ContainerStateDto.starting => KoncetStatusTone.info,
  ContainerStateDto.stopping => KoncetStatusTone.warning,
  ContainerStateDto.failed => KoncetStatusTone.danger,
  ContainerStateDto.stopped => KoncetStatusTone.stopped,
  ContainerStateDto.removed => KoncetStatusTone.stopped,
  ContainerStateDto.created => KoncetStatusTone.neutral,
};

String labelForContainer(ContainerStateDto state) => switch (state) {
  ContainerStateDto.running => 'Running',
  ContainerStateDto.starting => 'Starting',
  ContainerStateDto.stopping => 'Stopping',
  ContainerStateDto.failed => 'Failed',
  ContainerStateDto.stopped => 'Stopped',
  ContainerStateDto.removed => 'Removed',
  ContainerStateDto.created => 'Created',
};

KoncetStatusTone toneForHealth(HealthStatusDto health) => switch (health) {
  HealthStatusDto.healthy => KoncetStatusTone.healthy,
  HealthStatusDto.starting => KoncetStatusTone.info,
  HealthStatusDto.unhealthy => KoncetStatusTone.danger,
  HealthStatusDto.unknown => KoncetStatusTone.neutral,
};

KoncetStageStatus stageStatus(BuildStageStatusDto status) => switch (status) {
  BuildStageStatusDto.pending => KoncetStageStatus.pending,
  BuildStageStatusDto.running => KoncetStageStatus.running,
  BuildStageStatusDto.succeeded => KoncetStageStatus.succeeded,
  BuildStageStatusDto.failed => KoncetStageStatus.failed,
  BuildStageStatusDto.skipped => KoncetStageStatus.skipped,
};

String labelForStage(BuildStageNameDto stage) => switch (stage) {
  BuildStageNameDto.validate => 'Validate',
  BuildStageNameDto.prepare => 'Prepare',
  BuildStageNameDto.build => 'Build',
  BuildStageNameDto.package => 'Package',
};

KoncetStatusTone toneForSeverity(SeverityDto severity) => switch (severity) {
  SeverityDto.debug => KoncetStatusTone.neutral,
  SeverityDto.info => KoncetStatusTone.info,
  SeverityDto.warn => KoncetStatusTone.warning,
  SeverityDto.error => KoncetStatusTone.danger,
};

String labelForSeverity(SeverityDto severity) => switch (severity) {
  SeverityDto.debug => 'Debug',
  SeverityDto.info => 'Info',
  SeverityDto.warn => 'Warning',
  SeverityDto.error => 'Error',
};

IconData iconForSeverity(SeverityDto severity) => switch (severity) {
  SeverityDto.debug => Icons.bug_report_outlined,
  SeverityDto.info => Icons.info_outline,
  SeverityDto.warn => Icons.warning_amber_outlined,
  SeverityDto.error => Icons.error_outline,
};

String labelForLogSource(LogSourceDto source) => switch (source) {
  LogSourceDto.engine => 'Engine',
  LogSourceDto.build => 'Build',
  LogSourceDto.container => 'Container',
  LogSourceDto.process => 'Process',
  LogSourceDto.proxy => 'Proxy',
  LogSourceDto.firewall => 'Firewall',
  LogSourceDto.deployment => 'Deployment',
};

String labelForFailure(FailureReasonDto reason) => switch (reason) {
  FailureReasonDto.buildFailed => 'Build failed',
  FailureReasonDto.imageMissing => 'Image missing',
  FailureReasonDto.containerFailed => 'Container failed',
  FailureReasonDto.restartExhausted => 'Restart limit reached',
  FailureReasonDto.healthcheckFailed => 'Health check failed',
  FailureReasonDto.routeUnreachable => 'Route unreachable',
  FailureReasonDto.firewallDenied => 'Firewall denied',
  FailureReasonDto.proxyMisconfigured => 'Proxy misconfigured',
  FailureReasonDto.userStopped => 'Stopped by user',
  FailureReasonDto.appTerminated => 'Application terminated',
};

String explainFailure(FailureReasonDto reason) => switch (reason) {
  FailureReasonDto.buildFailed => 'The simulated build stopped before an image could be produced. Inspect the failed stage and its log.',
  FailureReasonDto.imageMissing =>
    'Deployment referenced an image that was not produced by the build.',
  FailureReasonDto.containerFailed => 'The simulated process exited with a failure code. Check the container and process state.',
  FailureReasonDto.restartExhausted => 'The process crashed repeatedly until the restart policy limit was reached.',
  FailureReasonDto.healthcheckFailed => 'The simulated health endpoint never reported healthy within the retry budget.',
  FailureReasonDto.routeUnreachable => 'The proxy has no usable route to the service port. Check the proxy target and service port.',
  FailureReasonDto.firewallDenied => 'The firewall denied proxy traffic to the service. Add an allow rule for the proxy source.',
  FailureReasonDto.proxyMisconfigured =>
    'The proxy listener or target configuration is invalid for this service.',
  FailureReasonDto.userStopped => 'The operator stopped this run. The observed state is preserved in history.',
  FailureReasonDto.appTerminated => 'KONCET was closed while the run was active. The run was reconciled as cancelled on the next start.',
};

String eventSummary(EventDto event) {
  try {
    final value = jsonDecode(event.payloadJson);
    if (value is Map<String, dynamic>) {
      final message = value['message'];
      if (message is String && message.isNotEmpty) return message;
      final reason = value['reason'];
      if (reason is String && reason.isNotEmpty) {
        return '${event.eventType} · $reason';
      }
      final stage = value['stage'];
      if (stage is String) return '${event.eventType} · $stage';
    }
  } on FormatException {
    // Unknown payloads render generically.
  }
  return event.eventType;
}
