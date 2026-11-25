import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/app_settings.dart';
import '../state/providers.dart';
import '../shell.dart';

class ProjectDetailScreen extends ConsumerStatefulWidget {
  const ProjectDetailScreen({super.key, required this.projectId});

  final String projectId;

  @override
  ConsumerState<ProjectDetailScreen> createState() =>
      _ProjectDetailScreenState();
}

class _ProjectDetailScreenState extends ConsumerState<ProjectDetailScreen> {
  final _seed = TextEditingController();
  String? _selectedScenarioId;
  bool _starting = false;
  List<ValidationIssueDto> _issues = const [];

  @override
  void dispose() {
    _seed.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final project = ref.watch(projectProvider(widget.projectId));
    final scenarios = ref.watch(projectScenariosProvider(widget.projectId));
    final recentRuns = ref.watch(projectRecentRunsProvider(widget.projectId));
    final active = ref.watch(activeRunProvider);

    return project.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (error, stack) => KoncetErrorState(
        title: 'Project could not be loaded',
        message: error.toString(),
        onRetry: () => ref.invalidate(projectProvider(widget.projectId)),
      ),
      data: (detail) {
        final archived = detail.archivedAtMs != null;
        final items = scenarios.value ?? const <ScenarioSummaryDto>[];
        final selectedId =
            _selectedScenarioId ?? (items.isEmpty ? null : items.first.id);
        final activeForProject =
            active != null &&
            !active.finalised &&
            active.handle.projectId == widget.projectId;

        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            KoncetPageHeader(
              breadcrumb: 'Projects',
              title: detail.name,
              description: detail.description.isEmpty
                  ? null
                  : detail.description,
              actions: [
                if (archived)
                  KoncetStatusBadge(
                    label: 'Archived',
                    tone: KoncetStatusTone.stopped,
                    icon: Icons.archive_outlined,
                  ),
                PopupMenuButton<String>(
                  tooltip: 'Project actions',
                  onSelected: (value) => _onAction(value, detail),
                  itemBuilder: (context) => [
                    const PopupMenuItem(
                      value: 'edit',
                      child: Text('Edit details'),
                    ),
                    const PopupMenuItem(
                      value: 'duplicate',
                      child: Text('Duplicate project'),
                    ),
                    PopupMenuItem(
                      value: archived ? 'unarchive' : 'archive',
                      child: Text(archived ? 'Unarchive' : 'Archive'),
                    ),
                    const PopupMenuDivider(),
                    const PopupMenuItem(
                      value: 'delete',
                      child: Text('Delete project'),
                    ),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 20),
            if (archived) ...[
              KoncetReasonCallout(
                title: 'Project is archived',
                message: 'Archived projects are read-only. Unarchive the project to edit source or start a run.',
                tone: KoncetStatusTone.stopped,
                action: TextButton(
                  onPressed: () => _onAction('unarchive', detail),
                  child: const Text('Unarchive'),
                ),
              ),
              const SizedBox(height: 16),
            ],
            if (activeForProject) ...[
              KoncetReasonCallout(
                title: 'A run is active for this project',
                message:
                    '${active.handle.scenarioName} · seed ${active.handle.seed}. Starting another run is blocked until it finishes or is stopped.',
                tone: KoncetStatusTone.active,
                action: TextButton(
                  onPressed: () => context.go('/runs/${active.handle.runId}'),
                  child: const Text('Open'),
                ),
              ),
              const SizedBox(height: 16),
            ],
            Expanded(
              child: SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    _ScenariosCard(
                      projectId: widget.projectId,
                      scenarios: items,
                      loading: scenarios.isLoading,
                      disabled: archived || activeForProject,
                      selectedId: selectedId,
                      onSelect: (id) =>
                          setState(() => _selectedScenarioId = id),
                      onOpen: (id) => context.go(
                        '/projects/${widget.projectId}/scenarios/$id',
                      ),
                      onChanged: _refreshScenarios,
                    ),
                    const SizedBox(height: 16),
                    KoncetSectionCard(
                      title: 'Start a run',
                      description: 'The selected scenario and current source become an immutable snapshot. Nothing leaves this machine.',
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          if (selectedId == null)
                            Text(
                              'Create a scenario below before starting a run.',
                              style: Theme.of(context).textTheme.bodySmall,
                            )
                          else
                            Row(
                              children: [
                                Icon(
                                  Icons.play_circle_outline,
                                  size: 18,
                                  color: Theme.of(context).colorScheme.primary,
                                ),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: Text(
                                    'Selected scenario: ${items.where((scenario) => scenario.id == selectedId).firstOrNull?.name ?? 'Unknown'}',
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodyMedium,
                                  ),
                                ),
                              ],
                            ),
                          const SizedBox(height: 12),
                          Wrap(
                            spacing: 12,
                            runSpacing: 12,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: [
                              SizedBox(
                                width: 300,
                                child: DropdownButtonFormField<String>(
                                  initialValue: selectedId,
                                  decoration: const InputDecoration(
                                    labelText: 'Scenario',
                                  ),
                                  items: [
                                    for (final scenario in items)
                                      DropdownMenuItem(
                                        value: scenario.id,
                                        child: Text(scenario.name),
                                      ),
                                  ],
                                  onChanged: (value) => setState(
                                    () => _selectedScenarioId = value,
                                  ),
                                ),
                              ),
                              SizedBox(
                                width: 280,
                                child: TextField(
                                  controller: _seed,
                                  decoration: const InputDecoration(
                                    labelText: 'Seed override (optional)',
                                    hintText: 'Decimal, up to 63 bits',
                                    helperText: 'Optional. Reuse a seed to replay the run.',
                                  ),
                                ),
                              ),
                              KoncetButton(
                                label: 'Start run',
                                icon: Icons.play_arrow,
                                busy: _starting,
                                onPressed:
                                    archived ||
                                        selectedId == null ||
                                        activeForProject
                                    ? null
                                    : () => _start(selectedId),
                              ),
                            ],
                          ),
                          if (activeForProject) ...[
                            const SizedBox(height: 8),
                            Text(
                              'A run is already active for this project. Open it from the callout above.',
                              style: Theme.of(context).textTheme.bodySmall,
                            ),
                          ],
                          if (_issues.isNotEmpty) ...[
                            const SizedBox(height: 10),
                            for (final issue in _issues)
                              KoncetValidationMessage(
                                message: issue.message,
                                path: issue.path,
                                isWarning: issue.severity == 'warning',
                              ),
                          ],
                        ],
                      ),
                    ),
                    const SizedBox(height: 16),
                    KoncetSectionCard(
                      title: 'Virtual source',
                      description: 'A validated in-memory file tree. KONCET never writes project source to disk or executes it.',
                      actions: [
                        KoncetButton(
                          label: 'Open source editor',
                          icon: Icons.code,
                          variant: KoncetButtonVariant.secondary,
                          onPressed: () => context.go(
                            '/projects/${widget.projectId}/source',
                          ),
                        ),
                      ],
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text(
                                '${detail.files.length} files',
                                style: Theme.of(context).textTheme.bodyMedium,
                              ),
                              const SizedBox(width: 16),
                              Text(
                                formatBytes(detail.totalBytes),
                                style: Theme.of(context).textTheme.bodySmall,
                              ),
                              const SizedBox(width: 16),
                              Expanded(
                                child: Text(
                                  detail.contentHash,
                                  style: context.mono.small.copyWith(
                                    color: Theme.of(context)
                                        .colorScheme
                                        .onSurfaceVariant,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 12),
                          Wrap(
                            spacing: 8,
                            runSpacing: 8,
                            children: [
                              for (final file in detail.files.take(12))
                                Chip(
                                  visualDensity: VisualDensity.compact,
                                  avatar: const Icon(
                                    Icons.description_outlined,
                                    size: 14,
                                  ),
                                  label: Text(
                                    file.path,
                                    style: context.mono.small,
                                  ),
                                ),
                            ],
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 16),
                    KoncetSectionCard(
                      title: 'Recent runs',
                      actions: [
                        KoncetButton(
                          label: 'All history',
                          variant: KoncetButtonVariant.tertiary,
                          onPressed: () => context.go('/runs'),
                        ),
                      ],
                      child: recentRuns.when(
                        loading: () => const LinearProgressIndicator(),
                        error: (error, stack) => Text('$error'),
                        data: (runs) => runs.isEmpty
                            ? Text(
                                'No runs yet. Start a run above.',
                                style: Theme.of(context).textTheme.bodySmall,
                              )
                            : Column(
                                children: [
                                  for (final run in runs)
                                    _RecentRunRow(run: run),
                                ],
                              ),
                      ),
                    ),
                    const SizedBox(height: 16),
                    KoncetSectionCard(
                      title: 'Project information',
                      child: ExpansionTile(
                        tilePadding: EdgeInsets.zero,
                        childrenPadding: const EdgeInsets.only(top: 4),
                        title: const Text('Metadata and identifiers'),
                        children: [
                          KoncetInfoRow(
                            label: 'Template',
                            value: _templateLabel(detail.templateId),
                          ),
                          KoncetInfoRow(
                            label: 'Created',
                            value: formatTimestamp(detail.createdAtMs),
                          ),
                          KoncetInfoRow(
                            label: 'Updated',
                            value: formatTimestamp(detail.updatedAtMs),
                          ),
                          KoncetInfoRow(
                            label: 'Project ID',
                            value: detail.id,
                            mono: true,
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 16),
                  ],
                ),
              ),
            ),
          ],
        );
      },
    );
  }

  Future<void> _start(String scenarioId) async {
    setState(() {
      _starting = true;
      _issues = const [];
    });
    try {
      final controller = ref.read(activeRunProvider.notifier);
      final handle = await controller.start(
        projectId: widget.projectId,
        scenarioId: scenarioId,
        seed: _seed.text.trim().isEmpty ? null : _seed.text.trim(),
      );
      final speed = ref.read(appSettingsProvider).value?.defaultSpeed ?? 1;
      if (speed != 1) {
        try {
          await controller.setSpeed(speed);
        } catch (_) {
          // The run is already durable; speed can be changed from the workspace.
        }
      }
      ref.invalidate(projectRecentRunsProvider(widget.projectId));
      ref.invalidate(runHistoryProvider);
      if (mounted) context.go('/runs/${handle.runId}');
    } on BridgeError catch (error) {
      setState(() => _issues = error.issues);
      if (error.issues.isEmpty && mounted) showAppError(context, error);
    } catch (error) {
      if (mounted) showAppError(context, error);
    } finally {
      if (mounted) setState(() => _starting = false);
    }
  }

  void _refreshScenarios() {
    ref.invalidate(projectScenariosProvider(widget.projectId));
    ref.invalidate(projectProvider(widget.projectId));
    ref.invalidate(projectRecentRunsProvider(widget.projectId));
  }

  Future<void> _onAction(String action, ProjectDetailDto detail) async {
    final api = ref.read(apiProvider);
    try {
      switch (action) {
        case 'edit':
          final updated = await _editProject(detail);
          if (updated == null || !mounted) return;
          if (updated.name != detail.name) {
            await api.renameProject(detail.id, updated.name);
          }
          if (updated.description != detail.description) {
            await api.setProjectDescription(detail.id, updated.description);
          }
          ref.invalidate(projectProvider(detail.id));
          ref.invalidate(projectsProvider(false));
          ref.invalidate(projectsProvider(true));
        case 'duplicate':
          final copy = await api.duplicateProject(detail.id);
          ref.invalidate(projectsProvider(false));
          ref.invalidate(projectsProvider(true));
          if (mounted) context.go('/projects/${copy.id}');
        case 'archive':
          await api.archiveProject(detail.id);
          ref.invalidate(projectProvider(detail.id));
          ref.invalidate(projectsProvider(false));
          ref.invalidate(projectsProvider(true));
        case 'unarchive':
          await api.unarchiveProject(detail.id);
          ref.invalidate(projectProvider(detail.id));
          ref.invalidate(projectsProvider(false));
          ref.invalidate(projectsProvider(true));
        case 'delete':
          if (!mounted) return;
          final confirmed = await showKoncetConfirm(
            context,
            title: 'Delete ${detail.name}?',
            message: 'This permanently removes the project, its source, scenarios, snapshots, and all run history. This cannot be undone.',
            confirmLabel: 'Delete project',
            destructive: true,
          );
          if (!confirmed) return;
          await api.deleteProject(detail.id);
          ref.invalidate(projectsProvider(false));
          ref.invalidate(projectsProvider(true));
          if (mounted) context.go('/projects');
      }
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }

  Future<({String name, String description})?> _editProject(
    ProjectDetailDto detail,
  ) async {
    final name = TextEditingController(text: detail.name);
    final description = TextEditingController(text: detail.description);
    final result = await showDialog<({String name, String description})>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Edit project'),
        content: SizedBox(
          width: 460,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: name,
                autofocus: true,
                maxLength: 80,
                decoration: const InputDecoration(labelText: 'Name'),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: description,
                maxLines: 3,
                maxLength: 500,
                decoration: const InputDecoration(labelText: 'Description'),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop((
              name: name.text.trim(),
              description: description.text.trim(),
            )),
            child: const Text('Save changes'),
          ),
        ],
      ),
    );
    name.dispose();
    description.dispose();
    return result;
  }

  static String _templateLabel(ProjectTemplateDto template) =>
      switch (template) {
        ProjectTemplateDto.staticWeb => 'Static web',
        ProjectTemplateDto.httpService => 'HTTP service',
      };
}

class _ScenariosCard extends ConsumerWidget {
  const _ScenariosCard({
    required this.projectId,
    required this.scenarios,
    required this.loading,
    required this.disabled,
    required this.selectedId,
    required this.onSelect,
    required this.onOpen,
    required this.onChanged,
  });

  final String projectId;
  final List<ScenarioSummaryDto> scenarios;
  final bool loading;
  final bool disabled;
  final String? selectedId;
  final ValueChanged<String> onSelect;
  final ValueChanged<String> onOpen;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return KoncetSectionCard(
      title: 'Scenarios',
      description: 'Scenarios are versioned configurations. Edits use optimistic revisions and the Rust validator is authoritative.',
      actions: [
        KoncetButton(
          label: 'New scenario',
          icon: Icons.add,
          variant: KoncetButtonVariant.secondary,
          onPressed: disabled ? null : () => _create(context, ref),
        ),
      ],
      child: loading
          ? const LinearProgressIndicator()
          : scenarios.isEmpty
          ? const Text('No scenarios in this project.')
          : Column(
              children: [
                for (final scenario in scenarios)
                  _ScenarioRow(
                    scenario: scenario,
                    selected: scenario.id == selectedId,
                    disabled: disabled,
                    onSelect: () => onSelect(scenario.id),
                    onOpen: () => onOpen(scenario.id),
                    onDuplicate: () => _duplicate(context, ref, scenario),
                    onDelete: () => _delete(context, ref, scenario),
                  ),
              ],
            ),
    );
  }

  Future<void> _create(BuildContext context, WidgetRef ref) async {
    final presets = await ref.read(scenarioPresetsProvider.future);
    if (!context.mounted) return;
    final key = await showDialog<String>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('New scenario'),
        children: [
          for (final preset in presets)
            SimpleDialogOption(
              onPressed: () => Navigator.of(context).pop(preset.key),
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(preset.label),
                    const SizedBox(height: 2),
                    Text(
                      preset.description,
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
    if (key == null) return;
    try {
      await ref.read(apiProvider).createScenario(projectId, key);
      onChanged();
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    }
  }

  Future<void> _duplicate(
    BuildContext context,
    WidgetRef ref,
    ScenarioSummaryDto scenario,
  ) async {
    try {
      await ref.read(apiProvider).duplicateScenario(scenario.id);
      onChanged();
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    }
  }

  Future<void> _delete(
    BuildContext context,
    WidgetRef ref,
    ScenarioSummaryDto scenario,
  ) async {
    final confirmed = await showKoncetConfirm(
      context,
      title: 'Delete ${scenario.name}?',
      message: 'The scenario is removed from the project. Scenarios referenced by run history cannot be deleted.',
      confirmLabel: 'Delete scenario',
      destructive: true,
    );
    if (!confirmed) return;
    try {
      await ref.read(apiProvider).deleteScenario(scenario.id);
      onChanged();
    } on BridgeError catch (error) {
      if (context.mounted) showAppError(context, error);
    }
  }
}

class _ScenarioRow extends StatelessWidget {
  const _ScenarioRow({
    required this.scenario,
    required this.selected,
    required this.disabled,
    required this.onSelect,
    required this.onOpen,
    required this.onDuplicate,
    required this.onDelete,
  });

  final ScenarioSummaryDto scenario;
  final bool selected;
  final bool disabled;
  final VoidCallback onSelect;
  final VoidCallback onOpen;
  final VoidCallback onDuplicate;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: selected
            ? theme.colorScheme.secondaryContainer.withValues(alpha: 0.45)
            : theme.colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Row(
        children: [
          InkResponse(
            onTap: onSelect,
            radius: 18,
            child: Tooltip(
              message: 'Select scenario',
              child: Icon(
                selected
                    ? Icons.radio_button_checked
                    : Icons.radio_button_unchecked,
                size: 18,
                color: selected
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: InkWell(
              onTap: onSelect,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(scenario.name, style: theme.textTheme.titleSmall),
                  if (scenario.description.isNotEmpty)
                    Text(
                      scenario.description,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall,
                    ),
                ],
              ),
            ),
          ),
          Text('rev ${scenario.revision}', style: theme.textTheme.labelMedium),
          const SizedBox(width: 12),
          IconButton(
            tooltip: 'Open scenario editor',
            onPressed: onOpen,
            icon: const Icon(Icons.edit_outlined, size: 18),
          ),
          PopupMenuButton<String>(
            tooltip: 'Scenario actions',
            onSelected: (value) {
              if (value == 'duplicate') onDuplicate();
              if (value == 'delete') onDelete();
            },
            itemBuilder: (context) => [
              const PopupMenuItem(value: 'duplicate', child: Text('Duplicate')),
              PopupMenuItem(
                value: 'delete',
                enabled: !disabled,
                child: const Text('Delete'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _RecentRunRow extends StatelessWidget {
  const _RecentRunRow({required this.run});

  final RunHistoryItemDto run;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: () => context.go('/runs/${run.runId}'),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 4),
        child: Row(
          children: [
            KoncetStatusBadge(
              label: labelForRunStatus(run.status),
              tone: toneForRunStatus(run.status),
              dense: true,
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                run.scenarioName,
                style: theme.textTheme.bodyMedium,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            Text(
              formatTimestamp(run.startedAtMs),
              style: theme.textTheme.bodySmall,
            ),
            const SizedBox(width: 16),
            SizedBox(
              width: 80,
              child: Text(
                formatDurationMs(run.simDurationMs),
                style: theme.textTheme.bodySmall,
                textAlign: TextAlign.right,
              ),
            ),
            const SizedBox(width: 8),
            Icon(
              Icons.chevron_right,
              size: 18,
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ],
        ),
      ),
    );
  }
}
