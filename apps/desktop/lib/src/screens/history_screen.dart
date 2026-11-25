import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/providers.dart';
import '../shell.dart';

class HistoryScreen extends ConsumerStatefulWidget {
  const HistoryScreen({super.key});

  @override
  ConsumerState<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends ConsumerState<HistoryScreen> {
  final _search = TextEditingController();
  String? _projectFilter;
  RunStatusDto? _statusFilter;
  bool _busy = false;

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final history = ref.watch(runHistoryProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        KoncetPageHeader(
          title: 'Run history',
          description: 'Every run stores an immutable source snapshot, scenario revision, recorded commands, and observed output.',
          actions: [
            KoncetButton(
              label: 'Refresh',
              icon: Icons.refresh,
              variant: KoncetButtonVariant.secondary,
              onPressed: () => ref.invalidate(runHistoryProvider),
            ),
            KoncetButton(
              label: 'Clear history',
              icon: Icons.delete_sweep_outlined,
              variant: KoncetButtonVariant.tertiary,
              busy: _busy,
              onPressed: () => _clearHistory(context),
            ),
          ],
        ),
        const SizedBox(height: 16),
        history.when(
          loading: () =>
              const Expanded(child: Center(child: CircularProgressIndicator())),
          error: (error, stack) => Expanded(
            child: KoncetErrorState(
              title: 'Run history could not be loaded',
              message: error.toString(),
              onRetry: () => ref.invalidate(runHistoryProvider),
            ),
          ),
          data: (items) {
            final projects = <String, String>{
              for (final item in items) item.projectId: item.projectName,
            };
            final filtered = _filter(items);
            return Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  _StatsRow(items: items),
                  const SizedBox(height: 16),
                  Wrap(
                    spacing: 10,
                    runSpacing: 8,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      SizedBox(
                        width: 260,
                        child: TextField(
                          controller: _search,
                          decoration: const InputDecoration(
                            labelText: 'Search scenario or project',
                            prefixIcon: Icon(Icons.search, size: 18),
                            isDense: true,
                          ),
                          onChanged: (_) => setState(() {}),
                        ),
                      ),
                      SizedBox(
                        width: 220,
                        child: DropdownButtonFormField<String>(
                          initialValue: _projectFilter ?? 'all',
                          decoration: const InputDecoration(
                            labelText: 'Project',
                            isDense: true,
                          ),
                          items: [
                            const DropdownMenuItem(
                              value: 'all',
                              child: Text('All projects'),
                            ),
                            for (final entry in projects.entries)
                              DropdownMenuItem(
                                value: entry.key,
                                child: Text(
                                  entry.value,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                          ],
                          onChanged: (value) => setState(
                            () => _projectFilter =
                                value == null || value == 'all' ? null : value,
                          ),
                        ),
                      ),
                      SizedBox(
                        width: 180,
                        child: DropdownButtonFormField<String>(
                          initialValue: _statusFilter?.name ?? 'all',
                          decoration: const InputDecoration(
                            labelText: 'Status',
                            isDense: true,
                          ),
                          items: const [
                            DropdownMenuItem(
                              value: 'all',
                              child: Text('All statuses'),
                            ),
                            DropdownMenuItem(
                              value: 'running',
                              child: Text('Running'),
                            ),
                            DropdownMenuItem(
                              value: 'paused',
                              child: Text('Paused'),
                            ),
                            DropdownMenuItem(
                              value: 'completed',
                              child: Text('Completed'),
                            ),
                            DropdownMenuItem(
                              value: 'cancelled',
                              child: Text('Cancelled'),
                            ),
                            DropdownMenuItem(
                              value: 'failed',
                              child: Text('Failed'),
                            ),
                          ],
                          onChanged: (value) => setState(() {
                            _statusFilter = value == null || value == 'all'
                                ? null
                                : RunStatusDto.values.firstWhere(
                                    (status) => status.name == value,
                                  );
                          }),
                        ),
                      ),
                      Text(
                        '${filtered.length} of ${items.length} run(s)',
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  Expanded(
                    child: filtered.isEmpty
                        ? KoncetEmptyState(
                            icon: Icons.history_outlined,
                            title: items.isEmpty
                                ? 'No runs yet'
                                : 'No runs match the current filter',
                            message: items.isEmpty
                                ? 'Start a simulation from a project to build a local run history.'
                                : 'Clear the search or choose a different project or status.',
                            action: items.isEmpty
                                ? KoncetButton(
                                    label: 'Go to projects',
                                    icon: Icons.folder_outlined,
                                    onPressed: () => context.go('/projects'),
                                  )
                                : null,
                          )
                        : Card(
                            clipBehavior: Clip.antiAlias,
                            child: Column(
                              children: [
                                const _HistoryHeader(),
                                Divider(
                                  height: 1,
                                  color: Theme.of(context)
                                      .colorScheme
                                      .outlineVariant,
                                ),
                                Expanded(
                                  child: ListView.separated(
                                    itemCount: filtered.length,
                                    separatorBuilder: (_, _) => Divider(
                                      height: 1,
                                      color: Theme.of(context)
                                          .colorScheme
                                          .outlineVariant,
                                    ),
                                    itemBuilder: (context, index) =>
                                        _HistoryRow(
                                          run: filtered[index],
                                          onDelete: () => _deleteRun(
                                            context,
                                            filtered[index],
                                          ),
                                        ),
                                  ),
                                ),
                              ],
                            ),
                          ),
                  ),
                ],
              ),
            );
          },
        ),
      ],
    );
  }

  List<RunHistoryItemDto> _filter(List<RunHistoryItemDto> items) {
    final query = _search.text.trim().toLowerCase();
    return items.where((item) {
      if (_projectFilter != null && item.projectId != _projectFilter) {
        return false;
      }
      if (_statusFilter != null && item.status != _statusFilter) return false;
      if (query.isNotEmpty) {
        final haystack =
            '${item.scenarioName} ${item.projectName} ${item.runId}'
                .toLowerCase();
        if (!haystack.contains(query)) return false;
      }
      return true;
    }).toList();
  }

  Future<void> _deleteRun(BuildContext context, RunHistoryItemDto run) async {
    final confirmed = await showKoncetConfirm(
      context,
      title: 'Delete this run?',
      message:
          'Deleting ${run.scenarioName} removes its events, logs, metrics, commands, and snapshot from history.',
      confirmLabel: 'Delete run',
      destructive: true,
    );
    if (!confirmed) return;
    try {
      await ref.read(apiProvider).deleteRun(run.runId);
      ref.invalidate(runHistoryProvider);
      ref.invalidate(projectRecentRunsProvider(run.projectId));
      if (context.mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(const SnackBar(content: Text('Run deleted')));
      }
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    }
  }

  Future<void> _clearHistory(BuildContext context) async {
    final scope = _projectFilter == null
        ? 'all projects'
        : 'the selected project';
    final confirmed = await showKoncetConfirm(
      context,
      title: 'Clear run history?',
      message:
          'This permanently deletes runs and their output for $scope. Projects, source trees, and scenarios are kept. Active runs are never deleted.',
      confirmLabel: 'Clear history',
      destructive: true,
    );
    if (!confirmed) return;
    setState(() => _busy = true);
    try {
      final removed = await ref
          .read(apiProvider)
          .deleteRunHistory(projectId: _projectFilter);
      ref.invalidate(runHistoryProvider);
      ref.invalidate(projectRecentRunsProvider);
      if (context.mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text('$removed run(s) deleted')));
      }
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}

class _StatsRow extends StatelessWidget {
  const _StatsRow({required this.items});

  final List<RunHistoryItemDto> items;

  @override
  Widget build(BuildContext context) {
    final healthy = items
        .where((run) => run.result == RunResultDto.healthy)
        .length;
    final degraded = items
        .where((run) => run.result == RunResultDto.degraded)
        .length;
    final failed = items
        .where(
          (run) =>
              run.result == RunResultDto.buildFailed ||
              run.result == RunResultDto.runtimeFailed,
        )
        .length;
    return Wrap(
      spacing: 10,
      runSpacing: 10,
      children: [
        SizedBox(
          width: 150,
          child: KoncetMetricTile(
            label: 'Total runs',
            value: '${items.length}',
          ),
        ),
        SizedBox(
          width: 150,
          child: KoncetMetricTile(label: 'Healthy', value: '$healthy'),
        ),
        SizedBox(
          width: 150,
          child: KoncetMetricTile(label: 'Degraded', value: '$degraded'),
        ),
        SizedBox(
          width: 150,
          child: KoncetMetricTile(label: 'Failed', value: '$failed'),
        ),
      ],
    );
  }
}

class _HistoryHeader extends StatelessWidget {
  const _HistoryHeader();

  @override
  Widget build(BuildContext context) {
    final style = Theme.of(context).textTheme.labelMedium;
    return Container(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 9),
      child: Row(
        children: [
          SizedBox(width: 110, child: Text('Status', style: style)),
          SizedBox(width: 130, child: Text('Result', style: style)),
          Expanded(child: Text('Scenario', style: style)),
          SizedBox(width: 160, child: Text('Started', style: style)),
          SizedBox(width: 90, child: Text('Duration', style: style)),
          SizedBox(width: 50, child: Text('', style: style)),
        ],
      ),
    );
  }
}

class _HistoryRow extends StatelessWidget {
  const _HistoryRow({required this.run, required this.onDelete});

  final RunHistoryItemDto run;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: () => context.go('/runs/${run.runId}'),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        child: Row(
          children: [
            SizedBox(
              width: 110,
              child: Align(
                alignment: Alignment.centerLeft,
                child: KoncetStatusBadge(
                  label: labelForRunStatus(run.status),
                  tone: toneForRunStatus(run.status),
                  dense: true,
                ),
              ),
            ),
            SizedBox(
              width: 130,
              child: Align(
                alignment: Alignment.centerLeft,
                child: KoncetStatusBadge(
                  label: labelForResult(run.result),
                  tone: toneForResult(run.result),
                  dense: true,
                ),
              ),
            ),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    run.scenarioName,
                    style: theme.textTheme.bodyMedium,
                    overflow: TextOverflow.ellipsis,
                  ),
                  Text(
                    run.projectName,
                    style: theme.textTheme.bodySmall,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            SizedBox(
              width: 160,
              child: Text(
                formatTimestamp(run.startedAtMs),
                style: theme.textTheme.bodySmall,
              ),
            ),
            SizedBox(
              width: 90,
              child: Text(
                formatDurationMs(run.simDurationMs),
                style: theme.textTheme.bodySmall,
              ),
            ),
            SizedBox(
              width: 50,
              child: PopupMenuButton<String>(
                tooltip: 'Run actions',
                onSelected: (value) {
                  if (value == 'delete') onDelete();
                  if (value == 'open') context.go('/runs/${run.runId}');
                },
                itemBuilder: (context) => const [
                  PopupMenuItem(value: 'open', child: Text('Open run')),
                  PopupMenuItem(value: 'delete', child: Text('Delete run')),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
