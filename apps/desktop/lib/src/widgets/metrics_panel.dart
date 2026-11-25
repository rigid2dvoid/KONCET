import 'package:fl_chart/fl_chart.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';

class MetricsPanel extends StatelessWidget {
  const MetricsPanel({super.key, required this.metrics});

  final List<MetricDto> metrics;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (metrics.isEmpty) {
      return const KoncetEmptyState(
        icon: Icons.show_chart,
        title: 'No metric samples yet',
        message: 'Samples appear after the simulation clock reaches the first monitoring interval.',
      );
    }

    final series = <String, List<MetricDto>>{};
    for (final metric in metrics) {
      series.putIfAbsent(metric.metric, () => []).add(metric);
    }
    for (final samples in series.values) {
      samples.sort((a, b) => a.simTimeMs.compareTo(b.simTimeMs));
    }
    final latest = <String, MetricDto>{
      for (final entry in series.entries) entry.key: entry.value.last,
    };

    final latestContainer = <String, MetricDto>{
      for (final entry in latest.entries)
        if (entry.value.entityKind == MetricEntityDto.container)
          entry.key: entry.value,
    };
    final latestDeployment = <String, MetricDto>{
      for (final entry in latest.entries)
        if (entry.value.entityKind == MetricEntityDto.deployment)
          entry.key: entry.value,
    };

    return ListView(
      children: [
        if (latestContainer.isNotEmpty || latestDeployment.isNotEmpty) ...[
          Text('Latest values', style: theme.textTheme.titleSmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              for (final entry in latestContainer.entries)
                SizedBox(
                  width: 160,
                  child: KoncetMetricTile(
                    label: _metricLabel(entry.key),
                    value: _formatValue(entry.value, compact: true),
                    unit: _unit(entry.key),
                    caption: _entityLabel(entry.value.entityKind),
                  ),
                ),
              for (final entry in latestDeployment.entries)
                SizedBox(
                  width: 160,
                  child: KoncetMetricTile(
                    label: _metricLabel(entry.key),
                    value: _formatValue(entry.value, compact: true),
                    unit: _unit(entry.key),
                    caption: _entityLabel(entry.value.entityKind),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 20),
        ],
        Text('Samples', style: theme.textTheme.titleSmall),
        const SizedBox(height: 8),
        LayoutBuilder(
          builder: (context, constraints) {
            final width = constraints.maxWidth >= 1000
                ? (constraints.maxWidth - 12) / 2
                : constraints.maxWidth;
            return Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                for (final entry in series.entries)
                  SizedBox(
                    width: width,
                    child: _SeriesChart(name: entry.key, samples: entry.value),
                  ),
              ],
            );
          },
        ),
      ],
    );
  }

  static String _metricLabel(String metric) => switch (metric) {
    'cpu_millicores' => 'CPU',
    'memory_mb' => 'Memory',
    'network_rx_kib' => 'Network RX',
    'network_tx_kib' => 'Network TX',
    'restart_count' => 'Restarts',
    'uptime_ms' => 'Uptime',
    'progress_percent' => 'Build progress',
    _ => metric.replaceAll('_', ' '),
  };

  static String _unit(String metric) => switch (metric) {
    'cpu_millicores' => 'm',
    'memory_mb' => 'MiB',
    'network_rx_kib' => 'KiB/s',
    'network_tx_kib' => 'KiB/s',
    'restart_count' => 'count',
    'uptime_ms' => '',
    'progress_percent' => '%',
    _ => '',
  };

  static String _entityLabel(MetricEntityDto entity) => switch (entity) {
    MetricEntityDto.build => 'Build',
    MetricEntityDto.container => 'Container',
    MetricEntityDto.deployment => 'Deployment',
  };

  static String _formatValue(MetricDto metric, {bool compact = false}) {
    if (metric.metric == 'uptime_ms') {
      return compact
          ? formatDurationMs(metric.value.round())
          : formatDurationMs(metric.value.round());
    }
    if (metric.value == metric.value.roundToDouble()) {
      return metric.value.toStringAsFixed(0);
    }
    return metric.value.toStringAsFixed(1);
  }
}

class _SeriesChart extends StatelessWidget {
  const _SeriesChart({required this.name, required this.samples});

  final String name;
  final List<MetricDto> samples;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final spots = [
      for (final sample in samples)
        FlSpot(sample.simTimeMs / 1000, sample.value),
    ];
    if (spots.isEmpty) return const SizedBox.shrink();
    final minX = spots.first.x;
    final maxX = spots.last.x == minX ? minX + 1 : spots.last.x;
    var minY = spots.map((spot) => spot.y).reduce((a, b) => a < b ? a : b);
    var maxY = spots.map((spot) => spot.y).reduce((a, b) => a > b ? a : b);
    if (minY == maxY) {
      minY = minY == 0 ? 0 : minY * 0.9;
      maxY = maxY == 0 ? 1 : maxY * 1.1;
    } else {
      final padding = (maxY - minY) * 0.1;
      minY = (minY - padding).clamp(0, double.infinity);
      maxY += padding;
    }
    final interval = (maxX - minX) / 4;

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Text(
                  MetricsPanel._metricLabel(name),
                  style: theme.textTheme.titleSmall,
                ),
                const SizedBox(width: 8),
                Text(_entity(samples.first), style: theme.textTheme.bodySmall),
                const Spacer(),
                Text(
                  '${samples.length} samples',
                  style: theme.textTheme.bodySmall,
                ),
              ],
            ),
            const SizedBox(height: 10),
            SizedBox(
              height: 180,
              child: LineChart(
                LineChartData(
                  minX: minX,
                  maxX: maxX,
                  minY: minY,
                  maxY: maxY,
                  gridData: FlGridData(
                    show: true,
                    drawVerticalLine: false,
                    horizontalInterval: (maxY - minY) / 3,
                    getDrawingHorizontalLine: (value) => FlLine(
                      color: theme.colorScheme.outlineVariant,
                      strokeWidth: 1,
                    ),
                  ),
                  borderData: FlBorderData(show: false),
                  titlesData: FlTitlesData(
                    topTitles: const AxisTitles(
                      sideTitles: SideTitles(showTitles: false),
                    ),
                    rightTitles: const AxisTitles(
                      sideTitles: SideTitles(showTitles: false),
                    ),
                    leftTitles: AxisTitles(
                      sideTitles: SideTitles(
                        showTitles: true,
                        reservedSize: 44,
                        getTitlesWidget: (value, meta) => Text(
                          _axisValue(name, value),
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                    ),
                    bottomTitles: AxisTitles(
                      sideTitles: SideTitles(
                        showTitles: true,
                        reservedSize: 24,
                        interval: interval <= 0 ? 1 : interval,
                        getTitlesWidget: (value, meta) => Padding(
                          padding: const EdgeInsets.only(top: 4),
                          child: Text(
                            '${value.toStringAsFixed(0)}s',
                            style: theme.textTheme.bodySmall,
                          ),
                        ),
                      ),
                    ),
                  ),
                  lineBarsData: [
                    LineChartBarData(
                      spots: spots,
                      isCurved: false,
                      barWidth: 2,
                      color: theme.colorScheme.primary,
                      dotData: const FlDotData(show: false),
                      belowBarData: BarAreaData(
                        show: true,
                        color: theme.colorScheme.primary.withValues(
                          alpha: 0.08,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  static String _entity(MetricDto metric) => switch (metric.entityKind) {
    MetricEntityDto.build => 'Build',
    MetricEntityDto.container => 'Container',
    MetricEntityDto.deployment => 'Deployment',
  };

  static String _axisValue(String name, double value) {
    if (name == 'uptime_ms') {
      return formatDurationMs(value.round());
    }
    if (value >= 1000) return '${(value / 1000).toStringAsFixed(1)}k';
    return value.toStringAsFixed(0);
  }
}
