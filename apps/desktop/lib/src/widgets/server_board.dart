import 'dart:math' as math;

import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../scenario_view.dart';

/// A live, schematic view of the simulated host.
///
/// The board is presentation only: every colour and label is derived from
/// committed run state. It never invents a real container, socket, or process.
class ServerBoard extends StatefulWidget {
  const ServerBoard({
    super.key,
    required this.run,
    required this.build,
    required this.image,
    required this.container,
    required this.deployment,
    required this.events,
    required this.metrics,
    required this.clockMs,
    required this.live,
    required this.finalised,
    required this.paused,
  });

  final RunDto run;
  final BuildDto? build;
  final SimulatedImageDto? image;
  final ContainerDto? container;
  final DeploymentDto? deployment;
  final List<EventDto> events;
  final List<MetricDto> metrics;
  final int clockMs;
  final bool live;
  final bool finalised;
  final bool paused;

  @override
  State<ServerBoard> createState() => _ServerBoardState();
}

class _ServerBoardState extends State<ServerBoard>
    with SingleTickerProviderStateMixin {
  late final AnimationController _flow;
  late ScenarioView? _scenario;

  @override
  void initState() {
    super.initState();
    _scenario = ScenarioView.parse(widget.run.scenarioJson);
    _flow = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 2200),
    );
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _syncAnimation();
  }

  @override
  void didUpdateWidget(covariant ServerBoard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.run.scenarioJson != widget.run.scenarioJson) {
      _scenario = ScenarioView.parse(widget.run.scenarioJson);
    }
    _syncAnimation();
  }

  void _syncAnimation() {
    final disabled = MediaQuery.maybeOf(context)?.disableAnimations ?? false;
    final shouldAnimate =
        widget.live && !widget.paused && !widget.finalised && !disabled;
    if (shouldAnimate) {
      if (!_flow.isAnimating) _flow.repeat();
    } else {
      _flow.stop();
      _flow.value = 0;
    }
  }

  @override
  void dispose() {
    _flow.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final scenario = _scenario;
    final reason =
        widget.run.summary?.primaryReason ?? widget.deployment?.primaryReason;
    final routeBlocked =
        reason == FailureReasonDto.firewallDenied ||
        reason == FailureReasonDto.proxyMisconfigured ||
        reason == FailureReasonDto.routeUnreachable;
    final deploymentTone = _deploymentTone(widget.deployment);
    final appTone = _appTone(widget.container);
    final routeTone = routeBlocked
        ? KoncetStatusTone.danger
        : deploymentTone == KoncetStatusTone.healthy
        ? KoncetStatusTone.healthy
        : deploymentTone;

    return ListView(
      key: const ValueKey('server-board-scroll'),
      padding: EdgeInsets.zero,
      children: [
        _BoardBanner(
          live: widget.live,
          finalised: widget.finalised,
          paused: widget.paused,
          animation: _flow,
          clockMs: widget.clockMs,
          host: scenario?.serviceName ?? 'app',
        ),
        const SizedBox(height: 12),
        _PipelineConveyor(
          buildInfo: widget.build,
          image: widget.image,
          container: widget.container,
          deployment: widget.deployment,
          animation: _flow,
          live: widget.live,
          paused: widget.paused,
        ),
        const SizedBox(height: 12),
        LayoutBuilder(
          builder: (context, constraints) {
            if (constraints.maxWidth < 900) {
              return Column(
                children: [
                  _HostRack(
                    scenario: scenario,
                    buildInfo: widget.build,
                    image: widget.image,
                    container: widget.container,
                    deployment: widget.deployment,
                    metrics: widget.metrics,
                    routeTone: routeTone,
                    reason: reason,
                  ),
                  const SizedBox(height: 12),
                  _ResourceMonitor(
                    metrics: widget.metrics,
                    deployment: widget.deployment,
                  ),
                ],
              );
            }
            return IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    flex: 3,
                    child: _HostRack(
                      scenario: scenario,
                      buildInfo: widget.build,
                      image: widget.image,
                      container: widget.container,
                      deployment: widget.deployment,
                      metrics: widget.metrics,
                      routeTone: routeTone,
                      reason: reason,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    flex: 2,
                    child: _ResourceMonitor(
                      metrics: widget.metrics,
                      deployment: widget.deployment,
                    ),
                  ),
                ],
              ),
            );
          },
        ),
        const SizedBox(height: 12),
        LayoutBuilder(
          builder: (context, constraints) {
            if (constraints.maxWidth < 900) {
              return Column(
                children: [
                  _TrafficPath(
                    scenario: scenario,
                    deployment: widget.deployment,
                    container: widget.container,
                    reason: reason,
                    routeTone: routeTone,
                    appTone: appTone,
                    animation: _flow,
                    live: widget.live,
                    paused: widget.paused,
                  ),
                  const SizedBox(height: 12),
                  _ActivityFeed(events: widget.events),
                ],
              );
            }
            return IntrinsicHeight(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Expanded(
                    flex: 3,
                    child: _TrafficPath(
                      scenario: scenario,
                      deployment: widget.deployment,
                      container: widget.container,
                      reason: reason,
                      routeTone: routeTone,
                      appTone: appTone,
                      animation: _flow,
                      live: widget.live,
                      paused: widget.paused,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    flex: 2,
                    child: _ActivityFeed(events: widget.events),
                  ),
                ],
              ),
            );
          },
        ),
        const SizedBox(height: 12),
      ],
    );
  }

  static KoncetStatusTone _deploymentTone(DeploymentDto? deployment) {
    if (deployment == null) return KoncetStatusTone.neutral;
    return switch (deployment.status) {
      DeploymentStatusDto.healthy => KoncetStatusTone.healthy,
      DeploymentStatusDto.degraded => KoncetStatusTone.warning,
      DeploymentStatusDto.failed => KoncetStatusTone.danger,
      DeploymentStatusDto.stopped => KoncetStatusTone.stopped,
      DeploymentStatusDto.deploying => KoncetStatusTone.info,
      DeploymentStatusDto.pending => KoncetStatusTone.neutral,
    };
  }

  static KoncetStatusTone _appTone(ContainerDto? container) {
    if (container == null) return KoncetStatusTone.neutral;
    return switch (container.state) {
      ContainerStateDto.running => KoncetStatusTone.healthy,
      ContainerStateDto.starting => KoncetStatusTone.info,
      ContainerStateDto.stopping => KoncetStatusTone.warning,
      ContainerStateDto.failed => KoncetStatusTone.danger,
      ContainerStateDto.stopped ||
      ContainerStateDto.removed ||
      ContainerStateDto.created => KoncetStatusTone.stopped,
    };
  }
}

class _BoardBanner extends StatelessWidget {
  const _BoardBanner({
    required this.live,
    required this.finalised,
    required this.paused,
    required this.animation,
    required this.clockMs,
    required this.host,
  });

  final bool live;
  final bool finalised;
  final bool paused;
  final Animation<double> animation;
  final int clockMs;
  final String host;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final tone = finalised
        ? KoncetStatusTone.stopped
        : paused
        ? KoncetStatusTone.warning
        : live
        ? KoncetStatusTone.active
        : KoncetStatusTone.neutral;
    final colors = context.statusColors;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Row(
        children: [
          Icon(Icons.dns_outlined, size: 22, color: colors.foreground(tone)),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Operations board · simulated host $host',
                  style: theme.textTheme.titleSmall,
                ),
                const SizedBox(height: 2),
                Text(
                  'Build, registry, container, proxy, and health are all virtual. No real server is touched.',
                  style: theme.textTheme.bodySmall,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          _PulseDot(animation: animation, enabled: live && !paused, tone: tone),
          const SizedBox(width: 8),
          Text(
            finalised
                ? 'FINAL'
                : paused
                ? 'PAUSED'
                : live
                ? 'LIVE'
                : 'REPLAY',
            style: context.mono.small.copyWith(
              color: colors.foreground(tone),
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(width: 14),
          Text(
            KoncetSimClock.formatDuration(clockMs),
            style: context.mono.medium,
          ),
          const SizedBox(width: 12),
          const KoncetSimulatedTag(compact: true),
        ],
      ),
    );
  }
}

class _PulseDot extends StatelessWidget {
  const _PulseDot({
    required this.animation,
    required this.enabled,
    required this.tone,
  });

  final Animation<double> animation;
  final bool enabled;
  final KoncetStatusTone tone;

  @override
  Widget build(BuildContext context) {
    final color = context.statusColors.foreground(tone);
    if (!enabled) {
      return Container(
        width: 8,
        height: 8,
        decoration: BoxDecoration(color: color, shape: BoxShape.circle),
      );
    }
    return AnimatedBuilder(
      animation: animation,
      builder: (context, child) {
        final opacity = 0.55 + 0.45 * math.sin(animation.value * math.pi * 2);
        return Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            color: color.withValues(alpha: opacity),
            shape: BoxShape.circle,
            boxShadow: [
              BoxShadow(
                color: color.withValues(alpha: 0.25 * opacity),
                blurRadius: 8,
                spreadRadius: 2,
              ),
            ],
          ),
        );
      },
    );
  }
}

class _PipelineConveyor extends StatelessWidget {
  const _PipelineConveyor({
    required this.buildInfo,
    required this.image,
    required this.container,
    required this.deployment,
    required this.animation,
    required this.live,
    required this.paused,
  });

  final BuildDto? buildInfo;
  final SimulatedImageDto? image;
  final ContainerDto? container;
  final DeploymentDto? deployment;
  final Animation<double> animation;
  final bool live;
  final bool paused;

  @override
  Widget build(BuildContext context) {
    final nodes = <_PipelineNode>[];
    for (final stage in buildInfo?.stages ?? const <BuildStageDto>[]) {
      nodes.add(
        _PipelineNode(
          icon: _stageIcon(stage.kind),
          label: labelForStage(stage.kind),
          detail: _stageDetail(stage),
          tone: _stageTone(stage.status),
        ),
      );
    }
    nodes.add(
      _PipelineNode(
        icon: Icons.inventory_2_outlined,
        label: 'Image',
        detail: image == null ? 'not produced' : image!.tag,
        tone: image != null
            ? KoncetStatusTone.healthy
            : buildInfo?.failure != null
            ? KoncetStatusTone.danger
            : KoncetStatusTone.neutral,
      ),
    );
    nodes.add(
      _PipelineNode(
        icon: Icons.widgets_outlined,
        label: 'Container',
        detail: container == null
            ? 'not created'
            : labelForContainer(container!.state),
        tone: container == null
            ? KoncetStatusTone.neutral
            : toneForContainer(container!.state),
      ),
    );
    nodes.add(
      _PipelineNode(
        icon: Icons.rocket_launch_outlined,
        label: 'Deployment',
        detail: deployment == null
            ? 'not created'
            : labelForDeployment(deployment!.status),
        tone: deployment == null
            ? KoncetStatusTone.neutral
            : toneForDeployment(deployment!.status),
      ),
    );

    return KoncetSectionCard(
      title: 'Release conveyor',
      description: 'From deterministic build stages to the simulated service that is serving traffic.',
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            for (var index = 0; index < nodes.length; index++) ...[
              _ConveyorNode(node: nodes[index]),
              if (index < nodes.length - 1)
                _FlowLine(
                  animation: animation,
                  active:
                      nodes[index].tone == KoncetStatusTone.healthy ||
                      nodes[index].tone == KoncetStatusTone.info,
                  color: _toneColor(context, nodes[index + 1].tone),
                  width: 34,
                ),
            ],
          ],
        ),
      ),
    );
  }

  static IconData _stageIcon(BuildStageNameDto stage) => switch (stage) {
    BuildStageNameDto.validate => Icons.fact_check_outlined,
    BuildStageNameDto.prepare => Icons.tune,
    BuildStageNameDto.build => Icons.construction_outlined,
    BuildStageNameDto.package => Icons.inventory_2_outlined,
  };

  static String _stageDetail(BuildStageDto stage) => switch (stage.status) {
    BuildStageStatusDto.pending =>
      'planned ${formatDurationMs(stage.plannedMs)}',
    BuildStageStatusDto.running => 'running…',
    BuildStageStatusDto.succeeded => formatDurationMs(
      stage.actualMs ?? stage.plannedMs,
    ),
    BuildStageStatusDto.failed => 'failed',
    BuildStageStatusDto.skipped => 'skipped',
  };

  static KoncetStatusTone _stageTone(BuildStageStatusDto status) =>
      switch (status) {
        BuildStageStatusDto.pending => KoncetStatusTone.neutral,
        BuildStageStatusDto.running => KoncetStatusTone.info,
        BuildStageStatusDto.succeeded => KoncetStatusTone.healthy,
        BuildStageStatusDto.failed => KoncetStatusTone.danger,
        BuildStageStatusDto.skipped => KoncetStatusTone.stopped,
      };

  static Color _toneColor(BuildContext context, KoncetStatusTone tone) =>
      context.statusColors.foreground(tone);
}

class _PipelineNode {
  const _PipelineNode({
    required this.icon,
    required this.label,
    required this.detail,
    required this.tone,
  });

  final IconData icon;
  final String label;
  final String detail;
  final KoncetStatusTone tone;
}

class _ConveyorNode extends StatelessWidget {
  const _ConveyorNode({required this.node});

  final _PipelineNode node;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = context.statusColors.foreground(node.tone);
    return SizedBox(
      width: 92,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 34,
            height: 34,
            decoration: BoxDecoration(
              color: context.statusColors.container(node.tone),
              borderRadius: BorderRadius.circular(9),
              border: Border.all(color: color.withValues(alpha: 0.35)),
            ),
            child: Icon(node.icon, size: 18, color: color),
          ),
          const SizedBox(height: 5),
          Text(
            node.label,
            style: theme.textTheme.labelMedium,
            textAlign: TextAlign.center,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          Text(
            node.detail,
            style: context.mono.small.copyWith(
              fontSize: 10,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
        ],
      ),
    );
  }
}

class _HostRack extends StatelessWidget {
  const _HostRack({
    required this.scenario,
    required this.buildInfo,
    required this.image,
    required this.container,
    required this.deployment,
    required this.metrics,
    required this.routeTone,
    required this.reason,
  });

  final ScenarioView? scenario;
  final BuildDto? buildInfo;
  final SimulatedImageDto? image;
  final ContainerDto? container;
  final DeploymentDto? deployment;
  final List<MetricDto> metrics;
  final KoncetStatusTone routeTone;
  final FailureReasonDto? reason;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final proxyEnabled = scenario?.proxyEnabled ?? true;
    final proxyTone = !proxyEnabled
        ? KoncetStatusTone.stopped
        : routeTone == KoncetStatusTone.danger
        ? KoncetStatusTone.danger
        : deployment?.status == DeploymentStatusDto.healthy
        ? KoncetStatusTone.healthy
        : routeTone;
    final appTone = container == null
        ? KoncetStatusTone.neutral
        : toneForContainer(container!.state);
    final deploymentTone = deployment == null
        ? KoncetStatusTone.neutral
        : toneForDeployment(deployment!.status);
    final buildTone = buildInfo?.failure != null
        ? KoncetStatusTone.danger
        : image != null
        ? KoncetStatusTone.healthy
        : KoncetStatusTone.info;
    final uptimeMs =
        _latestMetric(
          metrics,
          'uptime_ms',
          MetricEntityDto.deployment,
        )?.value.round() ??
        deployment?.uptimeMs ??
        0;

    return KoncetSectionCard(
      title: 'Host stack',
      description: 'One simulated host · numbered slots mirror the services that would run in a single-server deployment.',
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          _RackSlot(
            number: '01',
            icon: Icons.hub_outlined,
            title: proxyEnabled ? 'Reverse proxy' : 'Direct service',
            detail: proxyEnabled
                ? ':${scenario!.proxyListenerPort} → :${scenario!.proxyTargetPort}'
                : 'proxy disabled · :${scenario?.servicePort ?? 3000}',
            tone: proxyTone,
            trailing: proxyEnabled ? 'nginx' : 'direct',
          ),
          _RackSlot(
            number: '02',
            icon: Icons.widgets_outlined,
            title: scenario?.serviceName ?? 'app',
            detail: image == null
                ? 'image not produced'
                : '${image!.repository}:${image!.tag}',
            tone: appTone,
            trailing: container == null
                ? labelForContainer(ContainerStateDto.created)
                : labelForContainer(container!.state),
          ),
          _RackSlot(
            number: '03',
            icon: Icons.rocket_launch_outlined,
            title: 'Deployment',
            detail: deployment == null
                ? 'waiting for a healthy container'
                : '${deployment!.serviceName}.koncet.local · ${formatDurationMs(uptimeMs)}',
            tone: deploymentTone,
            trailing: deployment == null
                ? 'pending'
                : labelForDeployment(deployment!.status),
          ),
          _RackSlot(
            number: '04',
            icon: Icons.monitor_heart_outlined,
            title: 'Observability',
            detail: metrics.isEmpty
                ? 'waiting for samples'
                : '${metrics.length} samples · every ${scenario?.monitoringIntervalS ?? 5}s',
            tone: metrics.isEmpty
                ? KoncetStatusTone.neutral
                : KoncetStatusTone.info,
            trailing: buildTone == KoncetStatusTone.danger
                ? 'degraded'
                : 'collecting',
          ),
          const SizedBox(height: 4),
          Align(
            alignment: Alignment.centerLeft,
            child: Text(
              reason == null
                  ? 'All displayed state comes from the deterministic run.'
                  : 'Primary reason: ${labelForFailure(reason!)}',
              style: theme.textTheme.bodySmall?.copyWith(
                color: reason == null
                    ? theme.colorScheme.onSurfaceVariant
                    : context.statusColors.danger,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _RackSlot extends StatelessWidget {
  const _RackSlot({
    required this.number,
    required this.icon,
    required this.title,
    required this.detail,
    required this.tone,
    required this.trailing,
  });

  final String number;
  final IconData icon;
  final String title;
  final String detail;
  final KoncetStatusTone tone;
  final String trailing;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = context.statusColors.foreground(tone);
    return Container(
      margin: const EdgeInsets.only(bottom: 6),
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(7),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Row(
        children: [
          Text(
            number,
            style: context.mono.small.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
              fontSize: 10,
            ),
          ),
          const SizedBox(width: 8),
          Icon(icon, size: 16, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.textTheme.labelMedium,
                  overflow: TextOverflow.ellipsis,
                ),
                Text(
                  detail,
                  style: context.mono.small.copyWith(
                    fontSize: 10,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Text(
            trailing,
            style: context.mono.small.copyWith(fontSize: 10, color: color),
          ),
          const SizedBox(width: 6),
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          ),
        ],
      ),
    );
  }
}

class _ResourceMonitor extends StatelessWidget {
  const _ResourceMonitor({required this.metrics, required this.deployment});

  final List<MetricDto> metrics;
  final DeploymentDto? deployment;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final cpu = _series('cpu_millicores');
    final memory = _series('memory_mb');
    final rx = _series('network_rx_kib');
    final tx = _series('network_tx_kib');
    final restarts = _latest('restart_count')?.value;
    final uptimeMs =
        _latestMetric(
          metrics,
          'uptime_ms',
          MetricEntityDto.deployment,
        )?.value.round() ??
        deployment?.uptimeMs ??
        0;
    final hasSamples = metrics.isNotEmpty;

    return KoncetSectionCard(
      title: 'Resource monitor',
      description: 'Simulated container samples, never host metrics.',
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      child: !hasSamples
          ? const Center(
              child: Text('Waiting for the first monitoring sample…'),
            )
          : Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                _MetricSpark(
                  label: 'CPU',
                  value: _last(cpu),
                  suffix: 'm',
                  values: cpu,
                  tone: KoncetStatusTone.info,
                ),
                _MetricSpark(
                  label: 'Memory',
                  value: _last(memory),
                  suffix: 'MiB',
                  values: memory,
                  tone: KoncetStatusTone.active,
                ),
                _MetricSpark(
                  label: 'Net RX',
                  value: _last(rx),
                  suffix: 'KiB/s',
                  values: rx,
                  tone: KoncetStatusTone.healthy,
                ),
                _MetricSpark(
                  label: 'Net TX',
                  value: _last(tx),
                  suffix: 'KiB/s',
                  values: tx,
                  tone: KoncetStatusTone.warning,
                ),
                const SizedBox(height: 4),
                Row(
                  children: [
                    Text('Restarts', style: theme.textTheme.bodySmall),
                    const Spacer(),
                    Text(
                      restarts == null ? '—' : restarts.toStringAsFixed(0),
                      style: context.mono.small,
                    ),
                    const SizedBox(width: 16),
                    Text('Uptime', style: theme.textTheme.bodySmall),
                    const Spacer(),
                    Text(
                      deployment == null || uptimeMs == 0
                          ? '—'
                          : formatDurationMs(uptimeMs),
                      style: context.mono.small,
                    ),
                  ],
                ),
              ],
            ),
    );
  }

  List<double> _series(String metric) => [
    for (final sample in metrics)
      if (sample.metric == metric &&
          sample.entityKind == MetricEntityDto.container)
        sample.value,
  ];

  double? _last(List<double> values) => values.isEmpty ? null : values.last;

  MetricDto? _latest(String metric) {
    MetricDto? found;
    for (final sample in metrics) {
      if (sample.metric == metric &&
          sample.entityKind == MetricEntityDto.container) {
        found = sample;
      }
    }
    return found;
  }
}

MetricDto? _latestMetric(
  List<MetricDto> metrics,
  String name,
  MetricEntityDto entity,
) {
  MetricDto? found;
  for (final sample in metrics) {
    if (sample.metric == name && sample.entityKind == entity) {
      found = sample;
    }
  }
  return found;
}

class _MetricSpark extends StatelessWidget {
  const _MetricSpark({
    required this.label,
    required this.value,
    required this.suffix,
    required this.values,
    required this.tone,
  });

  final String label;
  final double? value;
  final String suffix;
  final List<double> values;
  final KoncetStatusTone tone;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = context.statusColors.foreground(tone);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          SizedBox(
            width: 52,
            child: Text(label, style: theme.textTheme.bodySmall),
          ),
          Expanded(
            child: SizedBox(
              height: 22,
              child: CustomPaint(
                painter: _SparklinePainter(values: values, color: color),
              ),
            ),
          ),
          const SizedBox(width: 8),
          SizedBox(
            width: 66,
            child: Text(
              value == null ? '—' : '${value!.toStringAsFixed(0)} $suffix',
              style: context.mono.small.copyWith(fontSize: 10),
              textAlign: TextAlign.right,
            ),
          ),
        ],
      ),
    );
  }
}

class _SparklinePainter extends CustomPainter {
  const _SparklinePainter({required this.values, required this.color});

  final List<double> values;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (values.length < 2) {
      final paint = Paint()
        ..color = color.withValues(alpha: 0.35)
        ..strokeWidth = 1;
      canvas.drawLine(
        Offset(0, size.height / 2),
        Offset(size.width, size.height / 2),
        paint,
      );
      return;
    }
    final visible = values.length > 24
        ? values.sublist(values.length - 24)
        : values;
    final min = visible.reduce(math.min);
    final max = visible.reduce(math.max);
    final range = max == min ? 1.0 : max - min;
    final path = Path();
    for (var index = 0; index < visible.length; index++) {
      final x = size.width * index / (visible.length - 1);
      final y =
          size.height -
          ((visible[index] - min) / range) * (size.height - 4) -
          2;
      if (index == 0) {
        path.moveTo(x, y);
      } else {
        path.lineTo(x, y);
      }
    }
    final line = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5
      ..strokeCap = StrokeCap.round;
    canvas.drawPath(path, line);
  }

  @override
  bool shouldRepaint(covariant _SparklinePainter oldDelegate) =>
      oldDelegate.values != values || oldDelegate.color != color;
}

class _TrafficPath extends StatelessWidget {
  const _TrafficPath({
    required this.scenario,
    required this.deployment,
    required this.container,
    required this.reason,
    required this.routeTone,
    required this.appTone,
    required this.animation,
    required this.live,
    required this.paused,
  });

  final ScenarioView? scenario;
  final DeploymentDto? deployment;
  final ContainerDto? container;
  final FailureReasonDto? reason;
  final KoncetStatusTone routeTone;
  final KoncetStatusTone appTone;
  final Animation<double> animation;
  final bool live;
  final bool paused;

  @override
  Widget build(BuildContext context) {
    final proxyEnabled = scenario?.proxyEnabled ?? true;
    final blocked =
        reason == FailureReasonDto.firewallDenied ||
        reason == FailureReasonDto.proxyMisconfigured ||
        reason == FailureReasonDto.routeUnreachable;
    final appRunning = container?.state == ContainerStateDto.running;
    final healthTone = deployment == null
        ? KoncetStatusTone.neutral
        : toneForDeployment(deployment!.status);
    final firewallTone = reason == FailureReasonDto.firewallDenied
        ? KoncetStatusTone.danger
        : proxyEnabled
        ? KoncetStatusTone.healthy
        : KoncetStatusTone.neutral;

    final nodes = <Widget>[
      _TrafficNode(
        icon: Icons.language,
        label: 'Browser',
        detail: 'simulated request',
        tone: blocked ? KoncetStatusTone.neutral : KoncetStatusTone.info,
      ),
    ];
    if (proxyEnabled) {
      nodes.add(
        _TrafficNode(
          icon: Icons.hub_outlined,
          label: 'Nginx',
          detail: ':${scenario!.proxyListenerPort} · ${scenario!.proxyHost}',
          tone: blocked ? KoncetStatusTone.danger : routeTone,
        ),
      );
      nodes.add(
        _TrafficNode(
          icon: Icons.shield_outlined,
          label: 'Firewall',
          detail:
              '${scenario!.firewallDefaultPolicy} · ${scenario!.firewallAllowCount} allow',
          tone: firewallTone,
        ),
      );
    }
    nodes.add(
      _TrafficNode(
        icon: Icons.widgets_outlined,
        label: scenario?.serviceName ?? 'App',
        detail:
            ':${scenario?.servicePort ?? 3000} · ${container == null ? 'not created' : labelForContainer(container!.state)}',
        tone: appTone,
      ),
    );
    nodes.add(
      _TrafficNode(
        icon: Icons.monitor_heart_outlined,
        label: 'Health',
        detail: deployment == null
            ? 'not deployed'
            : labelForDeployment(deployment!.status),
        tone: healthTone,
      ),
    );

    return KoncetSectionCard(
      title: 'Traffic path',
      description: 'Browser → proxy → firewall → application → health. The moving dots are simulated requests.',
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              KoncetStatusBadge(
                label: blocked
                    ? 'Blocked'
                    : healthTone == KoncetStatusTone.stopped
                    ? 'Stopped'
                    : healthTone == KoncetStatusTone.danger
                    ? 'Failed'
                    : appRunning && healthTone == KoncetStatusTone.healthy
                    ? 'Reachable'
                    : healthTone == KoncetStatusTone.neutral
                    ? 'Not deployed'
                    : 'Degraded',
                tone: blocked ? KoncetStatusTone.danger : healthTone,
                dense: true,
              ),
              const SizedBox(width: 8),
              if (reason != null)
                Flexible(
                  child: Text(
                    labelForFailure(reason!),
                    style: Theme.of(context).textTheme.bodySmall
                        ?.copyWith(color: context.statusColors.danger),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
            ],
          ),
          const SizedBox(height: 10),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: [
                for (var index = 0; index < nodes.length; index++) ...[
                  nodes[index],
                  if (index < nodes.length - 1)
                    _FlowLine(
                      animation: animation,
                      active: !blocked && live && !paused,
                      color: index == nodes.length - 2
                          ? context.statusColors.foreground(healthTone)
                          : context.statusColors.foreground(
                              blocked
                                  ? KoncetStatusTone.danger
                                  : KoncetStatusTone.info,
                            ),
                      width: 24,
                    ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _TrafficNode extends StatelessWidget {
  const _TrafficNode({
    required this.icon,
    required this.label,
    required this.detail,
    required this.tone,
  });

  final IconData icon;
  final String label;
  final String detail;
  final KoncetStatusTone tone;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final color = context.statusColors.foreground(tone);
    return Container(
      width: 102,
      padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 8),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(9),
        border: Border.all(color: color.withValues(alpha: 0.35)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 18, color: color),
          const SizedBox(height: 4),
          Text(
            label,
            style: theme.textTheme.labelMedium,
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
          ),
          Text(
            detail,
            style: context.mono.small.copyWith(
              fontSize: 9,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _ActivityFeed extends StatelessWidget {
  const _ActivityFeed({required this.events});

  final List<EventDto> events;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final recent = events.reversed.take(5).toList();
    return KoncetSectionCard(
      title: 'Live activity',
      description: 'Newest events from the simulated pipeline.',
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 10),
      child: recent.isEmpty
          ? Text('No activity yet.', style: theme.textTheme.bodySmall)
          : Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                for (final event in recent)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 3),
                    child: Row(
                      children: [
                        SizedBox(
                          width: 58,
                          child: Text(
                            KoncetSimClock.formatDuration(event.simTimeMs),
                            style: context.mono.small.copyWith(fontSize: 9),
                          ),
                        ),
                        Icon(
                          iconForSeverity(event.severity),
                          size: 13,
                          color: context.statusColors.foreground(
                            toneForSeverity(event.severity),
                          ),
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            eventSummary(event),
                            style: theme.textTheme.bodySmall,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
    );
  }
}

class _FlowLine extends StatelessWidget {
  const _FlowLine({
    required this.animation,
    required this.active,
    required this.color,
    required this.width,
  });

  final Animation<double> animation;
  final bool active;
  final Color color;
  final double width;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      height: 28,
      child: AnimatedBuilder(
        animation: animation,
        builder: (context, child) => CustomPaint(
          painter: _FlowLinePainter(
            progress: animation.value,
            active: active,
            color: color,
          ),
        ),
      ),
    );
  }
}

class _FlowLinePainter extends CustomPainter {
  const _FlowLinePainter({
    required this.progress,
    required this.active,
    required this.color,
  });

  final double progress;
  final bool active;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final y = size.height / 2;
    final line = Paint()
      ..color = color.withValues(alpha: active ? 0.35 : 0.2)
      ..strokeWidth = 1;
    canvas.drawLine(Offset(0, y), Offset(size.width, y), line);
    if (!active) return;

    final dotPaint = Paint()..color = color;
    for (var index = 0; index < 3; index++) {
      final offset = (progress + index / 3) % 1.0;
      final x = offset * size.width;
      canvas.drawCircle(Offset(x, y), 2.2, dotPaint);
    }
  }

  @override
  bool shouldRepaint(covariant _FlowLinePainter oldDelegate) =>
      oldDelegate.progress != progress ||
      oldDelegate.active != active ||
      oldDelegate.color != color;
}
