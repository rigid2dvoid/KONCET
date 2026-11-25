import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../state/providers.dart';

class ProjectsScreen extends ConsumerStatefulWidget {
  const ProjectsScreen({super.key});

  @override
  ConsumerState<ProjectsScreen> createState() => _ProjectsScreenState();
}

class _ProjectsScreenState extends ConsumerState<ProjectsScreen> {
  final _searchController = TextEditingController();
  bool _includeArchived = false;

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final projects = ref.watch(projectsProvider(_includeArchived));
    final bootstrap = ref.watch(bootstrapProvider);
    final reconciled = bootstrap.value?.reconciledRuns ?? const <String>[];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        KoncetPageHeader(
          title: 'Projects',
          description: 'Author virtual source trees and scenarios, then observe a deterministic simulated deployment.',
          actions: [
            KoncetButton(
              label: 'Create project',
              icon: Icons.add,
              onPressed: () => context.go('/projects/new'),
            ),
          ],
        ),
        if (reconciled.isNotEmpty) ...[
          const SizedBox(height: 16),
          KoncetReasonCallout(
            title: 'Interrupted runs reconciled',
            message:
                '${reconciled.length} run(s) left active by a previous process were marked cancelled with reason app_terminated. Their last flushed logs and metrics are preserved.',
            tone: KoncetStatusTone.info,
          ),
        ],
        const SizedBox(height: 20),
        Wrap(
          spacing: 16,
          runSpacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            SizedBox(
              width: 320,
              child: TextField(
                controller: _searchController,
                decoration: const InputDecoration(
                  prefixIcon: Icon(Icons.search, size: 18),
                  hintText: 'Search projects',
                ),
                onChanged: (_) => setState(() {}),
              ),
            ),
            FilterChip(
              selected: _includeArchived,
              label: const Text('Include archived'),
              onSelected: (value) => setState(() => _includeArchived = value),
            ),
            if (projects.hasValue)
              Text(
                '${_filtered(projects.value!).length} project(s)',
                style: Theme.of(context).textTheme.bodySmall,
              ),
          ],
        ),
        const SizedBox(height: 16),
        Expanded(
          child: projects.when(
            loading: () => const Center(child: CircularProgressIndicator()),
            error: (error, stack) => KoncetErrorState(
              title: 'Projects could not be loaded',
              message: error.toString(),
              onRetry: () => ref.invalidate(projectsProvider(_includeArchived)),
            ),
            data: (items) {
              final filtered = _filtered(items);
              if (filtered.isEmpty) {
                return KoncetEmptyState(
                  icon: Icons.folder_open_outlined,
                  title: items.isEmpty
                      ? 'Create your first project'
                      : 'No projects match the current filter',
                  message: items.isEmpty
                      ? 'A project contains a virtual source tree and at least one scenario. Nothing is executed on your machine.'
                      : 'Clear the search text or include archived projects.',
                  action: items.isEmpty
                      ? KoncetButton(
                          label: 'Create project',
                          icon: Icons.add,
                          onPressed: () => context.go('/projects/new'),
                        )
                      : null,
                );
              }
              return ListView.separated(
                itemCount: filtered.length,
                separatorBuilder: (_, _) => const SizedBox(height: 10),
                itemBuilder: (context, index) =>
                    _ProjectRow(project: filtered[index]),
              );
            },
          ),
        ),
      ],
    );
  }

  List<ProjectSummaryDto> _filtered(List<ProjectSummaryDto> items) {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) return items;
    return items
        .where(
          (project) =>
              project.name.toLowerCase().contains(query) ||
              project.description.toLowerCase().contains(query),
        )
        .toList();
  }
}

class _ProjectRow extends StatelessWidget {
  const _ProjectRow({required this.project});

  final ProjectSummaryDto project;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final archived = project.archivedAtMs != null;
    return Card(
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: () => context.go('/projects/${project.id}'),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
          child: Row(
            children: [
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHigh,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(
                  Icons.folder_outlined,
                  color: theme.colorScheme.primary,
                  size: 21,
                ),
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Flexible(
                          child: Text(
                            project.name,
                            style: theme.textTheme.titleMedium,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        if (archived) ...[
                          const SizedBox(width: 8),
                          KoncetStatusBadge(
                            label: 'Archived',
                            tone: KoncetStatusTone.stopped,
                            dense: true,
                            icon: Icons.archive_outlined,
                          ),
                        ],
                      ],
                    ),
                    if (project.description.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.only(top: 2),
                        child: Text(
                          project.description,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                    const SizedBox(height: 6),
                    Row(
                      children: [
                        Text(
                          _templateLabel(project.templateId),
                          style: theme.textTheme.labelMedium,
                        ),
                        const SizedBox(width: 14),
                        Text(
                          '${project.fileCount} files · ${formatBytes(project.totalBytes)}',
                          style: theme.textTheme.bodySmall,
                        ),
                        const SizedBox(width: 14),
                        Text(
                          'Updated ${formatDate(project.updatedAtMs)}',
                          style: theme.textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }

  static String _templateLabel(ProjectTemplateDto template) =>
      switch (template) {
        ProjectTemplateDto.staticWeb => 'Static web',
        ProjectTemplateDto.httpService => 'HTTP service',
      };
}
