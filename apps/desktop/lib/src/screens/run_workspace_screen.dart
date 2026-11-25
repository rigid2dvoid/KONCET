import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/providers.dart';
import '../shell.dart';
import '../widgets/log_viewer.dart';
import '../widgets/metrics_panel.dart';
import '../widgets/timeline_view.dart';

class RunWorkspaceScreen extends ConsumerWidget {
  const RunWorkspaceScreen({super.key, required this.runId});

  final String runId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final active = ref.watch(activeRunProvider);
    final isLive =
        active != null && active.handle.runId == runId && !active.finalised;
    return DefaultTabController(
      length: 4,
      child: isLive
          ? _LiveRunWorkspace(active: active)
          : _HistoricalRunWorkspace(runId: runId),
    );
  }
}

class _ViewData {
  const _ViewData({
    required this.run,
    required this.build,
    required this.image,
    required this.container,
    required this.deployment,
    required this.events,
    required this.logs,
    required this.metrics,
    required this.commands,
    required this.live,
    required this.finalised,
    required this.clockMs,
    this.speed = 1,
    this.paused = false,
  });

  final RunDto run;
  final BuildDto? build;
  final SimulatedImageDto? image;
  final ContainerDto? container;
  final DeploymentDto? deployment;
  final List<EventDto> events;
  final List<LogDto> logs;
  final List<MetricDto> metrics;
  final List<RecordedCommandDto> commands;
  final bool live;
  final bool finalised;
  final int clockMs;
  final int speed;
  final bool paused;
}

class _LiveRunWorkspace extends ConsumerWidget {
  const _LiveRunWorkspace({required this.active});

  final ActiveRunState active;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final frame = active.frame;
    if (frame == null) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          KoncetPageHeader(
            breadcrumb: 'Active run',
            title: active.handle.scenarioName,
            description: 'Waiting for the first simulation frame…',
            actions: [
              IconButton(
                tooltip: 'Back to project',
                onPressed: () =>
                    context.go('/projects/${active.handle.projectId}'),
                icon: const Icon(Icons.close),
              ),
            ],
          ),
          const SizedBox(height: 24),
          const Expanded(child: Center(child: CircularProgressIndicator())),
        ],
      );
    }
    final view = _ViewData(
      run: frame.run,
      build: frame.build,
      image: frame.image,
      container: frame.container,
      deployment: frame.deployment,
      events: active.events,
      logs: active.logs,
      metrics: active.metrics,
      commands: const [],
      live: !active.finalised,
      finalised: active.finalised,
      clockMs: frame.clockMs,
      speed: frame.speed,
      paused: frame.paused,
    );
    return _RunWorkspaceView(
      data: view,
      error: active.error,
      controls: _LiveControls(active: active),
    );
  }
}

class _LiveControls extends ConsumerWidget {
  const _LiveControls({required this.active});

  final ActiveRunState active;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final frame = active.frame;
    final paused = frame?.paused ?? false;
    final finalised = active.finalised;
    final run = frame?.run;
    final controller = ref.read(activeRunProvider.notifier);

    Future<void> runCommand(Future<void> Function() command) async {
      try {
        await command();
      } on BridgeError catch (error) {
        if (context.mounted) showAppError(context, error);
      } on StateError catch (error) {
        if (context.mounted) showAppError(context, error);
      }
    }

    if (finalised) {
      return Wrap(
        spacing: 8,
        children: [
          KoncetStatusBadge(
            label: 'Run finished',
            tone: KoncetStatusTone.stopped,
            icon: Icons.flag_outlined,
          ),
          KoncetButton(
            label: 'Run again',
            icon: Icons.replay,
            variant: KoncetButtonVariant.secondary,
            onPressed: run == null ? null : () => _runAgain(context, ref, run),
          ),
          IconButton(
            tooltip: 'Back to project',
            onPressed: () => context.go('/projects/${active.handle.projectId}'),
            icon: const Icon(Icons.close),
          ),
        ],
      );
    }

    return Wrap(
      spacing: 8,
      runSpacing: 8,
      crossAxisAlignment: WrapCrossAlignment.center,
      children: [
        KoncetButton(
          label: paused ? 'Resume' : 'Pause',
          icon: paused ? Icons.play_arrow : Icons.pause,
          variant: KoncetButtonVariant.secondary,
          onPressed: () =>
              runCommand(paused ? controller.resume : controller.pause),
        ),
        PopupMenuButton<int>(
          tooltip: 'Simulation speed',
          onSelected: (speed) => runCommand(() => controller.setSpeed(speed)),
          itemBuilder: (context) => [
            for (final speed in const [1, 2, 4, 8])
              PopupMenuItem(
                value: speed,
                child: Row(
                  children: [
                    if (frame?.speed == speed)
                      const Icon(Icons.check, size: 16)
                    else
                      const SizedBox(width: 16),
                    const SizedBox(width: 8),
                    Text('$speed×'),
                  ],
                ),
              ),
          ],
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
            decoration: BoxDecoration(
              border: Border.all(
                color: Theme.of(context).colorScheme.outlineVariant,
              ),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.speed, size: 17),
                const SizedBox(width: 6),
                Text('${frame?.speed ?? 1}×'),
                const SizedBox(width: 2),
                const Icon(Icons.arrow_drop_down, size: 18),
              ],
            ),
          ),
        ),
        KoncetButton(
          label: 'Stop',
          icon: Icons.stop_circle_outlined,
          onPressed: () => runCommand(controller.stop),
        ),
        PopupMenuButton<String>(
          tooltip: 'More run actions',
          onSelected: (value) async {
            switch (value) {
              case 'connectivity':
                await runCommand(controller.checkConnectivity);
              case 'restart':
                await runCommand(controller.restartContainer);
              case 'cancel':
                if (!context.mounted) return;
                final confirmed = await showKoncetConfirm(
                  context,
                  title: 'Cancel this run?',
                  message: 'Cancelling finalises the run as cancelled. Collected events, logs, and metrics remain in history.',
                  confirmLabel: 'Cancel run',
                  destructive: true,
                );
                if (confirmed) await runCommand(controller.cancel);
            }
          },
          itemBuilder: (context) => [
            const PopupMenuItem(
              value: 'connectivity',
              child: Text('Check connectivity'),
            ),
            PopupMenuItem(
              value: 'restart',
              enabled: frame?.container?.state == ContainerStateDto.failed,
              child: const Text('Restart failed container'),
            ),
            const PopupMenuDivider(),
            const PopupMenuItem(value: 'cancel', child: Text('Cancel run')),
          ],
        ),
      ],
    );
  }

  Future<void> _runAgain(
    BuildContext context,
    WidgetRef ref,
    RunDto run,
  ) async {
    try {
      final controller = ref.read(activeRunProvider.notifier);
      final handle = await controller.start(
        projectId: run.projectId,
        scenarioId: run.scenarioId,
        seed: run.seed,
      );
      ref.invalidate(runHistoryProvider);
      if (context.mounted) context.go('/runs/${handle.runId}');
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    } catch (error) {
      if (context.mounted) showAppError(context, error);
    }
  }
}

class _HistoricalRunWorkspace extends ConsumerWidget {
  const _HistoricalRunWorkspace({required this.runId});

  final String runId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final detail = ref.watch(runDetailProvider(runId));
    final events = ref.watch(runEventsProvider(runId));
    final logs = ref.watch(runLogsProvider(runId));
    final metrics = ref.watch(historicalMetricsProvider(runId));

    return detail.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (error, stack) => KoncetErrorState(
        title: 'Run could not be loaded',
        message: error.toString(),
        onRetry: () => ref.invalidate(runDetailProvider(runId)),
      ),
      data: (runDetail) {
        final view = _ViewData(
          run: runDetail.run,
          build: runDetail.build,
          image: runDetail.image,
          container: runDetail.container,
          deployment: runDetail.deployment,
          events: events.value ?? const [],
          logs: logs.value ?? const [],
          metrics: metrics.value?.all ?? const [],
          commands: runDetail.commands,
          live: false,
          finalised:
              runDetail.run.status != RunStatusDto.running &&
              runDetail.run.status != RunStatusDto.paused,
          clockMs: runDetail.run.simDurationMs,
        );
        return _RunWorkspaceView(
          data: view,
          controls: _HistoricalControls(data: view),
        );
      },
    );
  }
}

class _HistoricalControls extends ConsumerWidget {
  const _HistoricalControls({required this.data});

  final _ViewData data;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final run = data.run;
    return Wrap(
      spacing: 8,
      children: [
        KoncetButton(
          label: 'Run again',
          icon: Icons.replay,
          variant: KoncetButtonVariant.secondary,
          onPressed: () => _runAgain(context, ref, run),
        ),
        IconButton(
          tooltip: 'Copy run ID',
          onPressed: () async {
            await Clipboard.setData(ClipboardData(text: run.id));
            if (context.mounted) {
              ScaffoldMessenger.of(context)
                  .showSnackBar(const SnackBar(content: Text('Run ID copied')));
            }
          },
          icon: const Icon(Icons.tag),
        ),
        IconButton(
          tooltip: 'Back to history',
          onPressed: () => context.go('/runs'),
          icon: const Icon(Icons.close),
        ),
      ],
    );
  }

  Future<void> _runAgain(
    BuildContext context,
    WidgetRef ref,
    RunDto run,
  ) async {
    try {
      final controller = ref.read(activeRunProvider.notifier);
      final handle = await controller.start(
        projectId: run.projectId,
        scenarioId: run.scenarioId,
        seed: run.seed,
      );
      ref.invalidate(runHistoryProvider);
      if (context.mounted) context.go('/runs/${handle.runId}');
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    } catch (error) {
      if (context.mounted) showAppError(context, error);
    }
  }
}

class _RunWorkspaceView extends ConsumerWidget {
  const _RunWorkspaceView({
    required this.data,
    required this.controls,
    this.error,
  });

  final _ViewData data;
  final Widget controls;
  final String? error;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final run = data.run;
    final theme = Theme.of(context);
    final reason = run.summary?.primaryReason ?? data.deployment?.primaryReason;
    final statusTone = toneForRunStatus(run.status);
    final resultTone = toneForResult(run.result);
    final resultNeedsAttention =
        run.result == RunResultDto.buildFailed ||
        run.result == RunResultDto.runtimeFailed ||
        run.result == RunResultDto.degraded ||
        data.deployment?.status == DeploymentStatusDto.failed ||
        data.deployment?.status == DeploymentStatusDto.degraded;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        KoncetPageHeader(
          breadcrumb: data.live ? 'Active simulation run' : 'Run history',
          title: run.scenarioName,
          description:
              'Seed ${run.seed} · started ${formatTimestamp(run.startedAtMs ?? run.createdAtMs)}',
          actions: [controls],
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            KoncetStatusBadge(
              label: labelForRunStatus(run.status),
              tone: statusTone,
              icon: data.live ? Icons.play_circle_outline : Icons.flag_outlined,
            ),
            const SizedBox(width: 8),
            KoncetStatusBadge(
              label: labelForResult(run.result),
              tone: resultTone,
              icon: Icons.assessment_outlined,
            ),
            const SizedBox(width: 16),
            KoncetSimClock(
              elapsedMs: data.clockMs,
              speed: data.speed,
              paused: data.paused,
            ),
            const Spacer(),
            Text(
              'v${run.appVersion} · engine ${run.engineVersion}',
              style: theme.textTheme.bodySmall,
            ),
          ],
        ),
        if (error != null) ...[
          const SizedBox(height: 8),
          KoncetReasonCallout(
            title: 'Live stream reported an error',
            message: error!,
            tone: KoncetStatusTone.warning,
          ),
        ],
        if (data.run.summary?.primaryReason != null ||
            data.build?.failure != null ||
            data.deployment?.primaryReason != null) ...[
          const SizedBox(height: 10),
          KoncetReasonCallout(
            title: reason != null
                ? labelForFailure(reason)
                : 'Build failed in ${labelForStage(data.build!.failure!.stage)}',
            message: reason != null
                ? explainFailure(reason)
                : data.build!.failure!.message,
            tone: resultNeedsAttention
                ? KoncetStatusTone.danger
                : KoncetStatusTone.warning,
          ),
        ],
        const SizedBox(height: 12),
        SizedBox(
          height: 180,
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Expanded(
                flex: 3,
                child: KoncetSectionCard(
                  title: 'Build pipeline',
                  padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
                  child: data.build == null
                      ? const Text('No build was recorded.')
                      : KoncetPipeline(
                          stages: [
                            for (final stage in data.build!.stages)
                              KoncetStageView(
                                label: labelForStage(stage.kind),
                                status: stageStatus(stage.status),
                                detail: _stageDetail(stage, data.run),
                              ),
                          ],
                        ),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(flex: 2, child: _DeploymentSummary(data: data)),
            ],
          ),
        ),
        const SizedBox(height: 12),
        const TabBar(
          isScrollable: true,
          tabAlignment: TabAlignment.start,
          tabs: [
            Tab(
              icon: Icon(Icons.timeline_outlined, size: 18),
              text: 'Timeline',
            ),
            Tab(icon: Icon(Icons.subject_outlined, size: 18), text: 'Logs'),
            Tab(icon: Icon(Icons.show_chart, size: 18), text: 'Metrics'),
            Tab(
              icon: Icon(Icons.inventory_2_outlined, size: 18),
              text: 'Inspect',
            ),
          ],
        ),
        const SizedBox(height: 12),
        Expanded(
          child: TabBarView(
            children: [
              TimelineView(events: data.events),
              LogViewer(logs: data.logs, live: data.live),
              MetricsPanel(metrics: data.metrics),
              _InspectTab(data: data),
            ],
          ),
        ),
      ],
    );
  }

  static String _stageDetail(BuildStageDto stage, RunDto run) {
    final planned = formatDurationMs(stage.plannedMs);
    return switch (stage.status) {
      BuildStageStatusDto.pending => 'planned $planned',
      BuildStageStatusDto.running => 'running · planned $planned',
      BuildStageStatusDto.succeeded =>
        'actual ${formatDurationMs(stage.actualMs ?? stage.plannedMs)}',
      BuildStageStatusDto.failed =>
        '${stage.failure?.message ?? 'failed'} · after ${formatDurationMs(stage.actualMs ?? 0)}',
      BuildStageStatusDto.skipped => 'skipped',
    };
  }
}

class _DeploymentSummary extends StatelessWidget {
  const _DeploymentSummary({required this.data});

  final _ViewData data;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final deployment = data.deployment;
    final container = data.container;
    return KoncetSectionCard(
      title: 'Deployment',
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              if (deployment != null)
                KoncetStatusBadge(
                  label: labelForDeployment(deployment.status),
                  tone: toneForDeployment(deployment.status),
                  dense: true,
                )
              else
                const KoncetStatusBadge(
                  label: 'Not created',
                  tone: KoncetStatusTone.neutral,
                  dense: true,
                ),
              const SizedBox(width: 8),
              if (container != null)
                KoncetStatusBadge(
                  label: labelForContainer(container.state),
                  tone: toneForContainer(container.state),
                  dense: true,
                ),
            ],
          ),
          const SizedBox(height: 8),
          if (data.image != null)
            Text(
              '${data.image!.repository}:${data.image!.tag}',
              style: context.mono.small,
              overflow: TextOverflow.ellipsis,
            ),
          if (deployment != null)
            Text(
              '${deployment.serviceName}.koncet.local · uptime ${formatDurationMs(deployment.uptimeMs)}',
              style: theme.textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
            ),
          if (container != null)
            Text(
              'Restarts ${container.restartCount} · ${container.process.state.name}',
              style: theme.textTheme.bodySmall,
              overflow: TextOverflow.ellipsis,
            ),
        ],
      ),
    );
  }
}

class _InspectTab extends StatelessWidget {
  const _InspectTab({required this.data});

  final _ViewData data;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListView(
      key: const ValueKey('run-inspect-scroll'),
      children: [
        if (data.image != null)
          KoncetSectionCard(
            title: 'Simulated image',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                KoncetInfoRow(
                  label: 'Reference',
                  value: '${data.image!.repository}:${data.image!.tag}',
                  mono: true,
                ),
                KoncetInfoRow(
                  label: 'Digest',
                  value: data.image!.digest,
                  mono: true,
                ),
                KoncetInfoRow(
                  label: 'Profile',
                  value: data.image!.profile.name,
                ),
                KoncetInfoRow(
                  label: 'Size',
                  value: formatBytes(data.image!.sizeBytes),
                ),
                KoncetInfoRow(
                  label: 'Source hash',
                  value: data.image!.sourceHash,
                  mono: true,
                ),
                const SizedBox(height: 10),
                Text('Layers', style: theme.textTheme.titleSmall),
                const SizedBox(height: 6),
                for (final layer in data.image!.layers)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 3),
                    child: Row(
                      children: [
                        Expanded(
                          child: Text(layer.name, style: context.mono.small),
                        ),
                        Text(
                          formatBytes(layer.sizeBytes),
                          style: theme.textTheme.bodySmall,
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            layer.digest,
                            style: context.mono.small.copyWith(
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
        if (data.container != null) ...[
          const SizedBox(height: 16),
          KoncetSectionCard(
            title: 'Container and process',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                KoncetInfoRow(
                  label: 'Container',
                  value: data.container!.name,
                  mono: true,
                ),
                KoncetInfoRow(
                  label: 'Image ID',
                  value: data.container!.imageId,
                  mono: true,
                ),
                KoncetInfoRow(
                  label: 'State',
                  value:
                      '${labelForContainer(data.container!.state)} · ${data.container!.health.name}',
                ),
                KoncetInfoRow(
                  label: 'Service port',
                  value: '${data.container!.config.port}',
                ),
                KoncetInfoRow(
                  label: 'CPU / memory',
                  value:
                      '${data.container!.config.cpuMillicores} m · ${data.container!.config.memoryMb} MiB',
                ),
                KoncetInfoRow(
                  label: 'Restart policy',
                  value:
                      '${_restartPolicyLabel(data.container!.config.restartPolicy.kind)} · ${data.container!.config.restartPolicy.maxRetries ?? '—'}',
                ),
                KoncetInfoRow(
                  label: 'Health path',
                  value: data.container!.config.healthCheck.path,
                  mono: true,
                ),
                KoncetInfoRow(
                  label: 'Process',
                  value:
                      '${data.container!.process.state.name} · attempt ${data.container!.process.attempt} · pid ${data.container!.process.pid ?? '—'}',
                ),
                if (data.container!.exitCode != null)
                  KoncetInfoRow(
                    label: 'Exit code',
                    value: '${data.container!.exitCode}',
                  ),
              ],
            ),
          ),
        ],
        if (data.commands.isNotEmpty) ...[
          const SizedBox(height: 16),
          KoncetSectionCard(
            title: 'Recorded commands',
            description:
                'Commands are part of the replay contract and appear in order.',
            child: Column(
              children: [
                for (final command in data.commands)
                  KoncetInfoRow(
                    label:
                        '#${command.seq} · ${formatDurationMs(command.atMs)}',
                    value: command.command,
                    mono: true,
                  ),
              ],
            ),
          ),
        ],
        const SizedBox(height: 16),
        KoncetSectionCard(
          title: 'Run identity',
          child: Column(
            children: [
              KoncetInfoRow(label: 'Run ID', value: data.run.id, mono: true),
              KoncetInfoRow(
                label: 'Project ID',
                value: data.run.projectId,
                mono: true,
              ),
              KoncetInfoRow(
                label: 'Scenario ID',
                value: data.run.scenarioId,
                mono: true,
              ),
              KoncetInfoRow(
                label: 'Snapshot',
                value: data.run.sourceSnapshotId,
                mono: true,
              ),
              KoncetInfoRow(label: 'Seed', value: data.run.seed, mono: true),
              KoncetInfoRow(
                label: 'Created',
                value: formatTimestamp(data.run.createdAtMs),
              ),
              if (data.run.endedAtMs != null)
                KoncetInfoRow(
                  label: 'Ended',
                  value: formatTimestamp(data.run.endedAtMs!),
                ),
              KoncetInfoRow(
                label: 'Scenario schema',
                value: 'v${data.run.scenarioSchemaVersion}',
              ),
              KoncetInfoRow(
                label: 'Event sequence',
                value: '${data.run.lastEventSeq}',
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        ExpansionTile(
          title: const Text('Scenario snapshot (JSON)'),
          children: [
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              color: theme.colorScheme.surfaceContainerLowest,
              child: SelectableText(
                _pretty(data.run.scenarioJson),
                style: context.mono.small,
              ),
            ),
          ],
        ),
        const SizedBox(height: 24),
      ],
    );
  }

  static String _restartPolicyLabel(RestartPolicyKindDto kind) =>
      switch (kind) {
        RestartPolicyKindDto.no => 'no',
        RestartPolicyKindDto.onFailure => 'on-failure',
        RestartPolicyKindDto.always => 'always',
      };

  static String _pretty(String json) {
    try {
      return const JsonEncoder.withIndent('  ').convert(jsonDecode(json));
    } on FormatException {
      return json;
    }
  }
}
