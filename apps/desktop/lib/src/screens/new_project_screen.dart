import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/providers.dart';

class NewProjectScreen extends ConsumerStatefulWidget {
  const NewProjectScreen({super.key});

  @override
  ConsumerState<NewProjectScreen> createState() => _NewProjectScreenState();
}

class _NewProjectScreenState extends ConsumerState<NewProjectScreen> {
  final _name = TextEditingController();
  final _description = TextEditingController();
  ProjectTemplateDto _template = ProjectTemplateDto.staticWeb;
  bool _busy = false;
  List<ValidationIssueDto> _issues = const [];

  @override
  void dispose() {
    _name.dispose();
    _description.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final templates = ref.watch(templatesProvider);
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            IconButton(
              tooltip: 'Back to projects',
              onPressed: () => context.go('/projects'),
              icon: const Icon(Icons.arrow_back),
            ),
            const SizedBox(width: 4),
            Text('Create project', style: theme.textTheme.headlineSmall),
          ],
        ),
        const SizedBox(height: 20),
        Expanded(
          child: SingleChildScrollView(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 900),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  KoncetSectionCard(
                    title: 'Identity',
                    description: 'The project is a local authoring workspace. Nothing is deployed to a real host.',
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        TextField(
                          controller: _name,
                          autofocus: true,
                          maxLength: 80,
                          decoration: const InputDecoration(
                            labelText: 'Project name',
                            hintText: 'Payments sandbox',
                          ),
                        ),
                        const SizedBox(height: 12),
                        TextField(
                          controller: _description,
                          maxLines: 2,
                          maxLength: 500,
                          decoration: const InputDecoration(
                            labelText: 'Description (optional)',
                            hintText: 'What this simulation is for',
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 16),
                  KoncetSectionCard(
                    title: 'Template',
                    description: 'The template seeds a virtual source tree and one Happy Path scenario.',
                    child: templates.when(
                      loading: () => const LinearProgressIndicator(),
                      error: (error, stack) => Text('$error'),
                      data: (items) => Column(
                        children: [
                          for (final template in items) ...[
                            _TemplateCard(
                              template: template,
                              selected: _template == _toDto(template.id),
                              onTap: () => setState(
                                () => _template = _toDto(template.id),
                              ),
                            ),
                            if (template != items.last)
                              const SizedBox(height: 10),
                          ],
                        ],
                      ),
                    ),
                  ),
                  if (_issues.isNotEmpty) ...[
                    const SizedBox(height: 16),
                    KoncetSectionCard(
                      title: 'Validation',
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          for (final issue in _issues)
                            KoncetValidationMessage(
                              message: issue.message,
                              path: issue.path,
                              isWarning: issue.severity == 'warning',
                            ),
                        ],
                      ),
                    ),
                  ],
                  const SizedBox(height: 20),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      OutlinedButton(
                        onPressed: _busy ? null : () => context.go('/projects'),
                        child: const Text('Cancel'),
                      ),
                      const SizedBox(width: 8),
                      KoncetButton(
                        label: 'Create project',
                        icon: Icons.add,
                        busy: _busy,
                        onPressed: _create,
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }

  ProjectTemplateDto _toDto(String id) => id == 'http-service'
      ? ProjectTemplateDto.httpService
      : ProjectTemplateDto.staticWeb;

  Future<void> _create() async {
    final name = _name.text.trim();
    if (name.isEmpty) {
      setState(() {
        _issues = const [
          ValidationIssueDto(
            code: 'required',
            path: '/name',
            message: 'Project name is required.',
            severity: 'error',
          ),
        ];
      });
      return;
    }
    setState(() {
      _busy = true;
      _issues = const [];
    });
    try {
      final project = await ref
          .read(apiProvider)
          .createProject(
            name: name,
            description: _description.text.trim(),
            template: _template,
          );
      ref.invalidate(projectsProvider(false));
      ref.invalidate(projectsProvider(true));
      ref.invalidate(projectProvider(project.id));
      if (mounted) context.go('/projects/${project.id}');
    } on BridgeError catch (error) {
      setState(() => _issues = error.issues);
      if (error.issues.isEmpty && mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(error.message)));
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }
}

class _TemplateCard extends StatelessWidget {
  const _TemplateCard({
    required this.template,
    required this.selected,
    required this.onTap,
  });

  final TemplateDto template;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: selected
          ? theme.colorScheme.secondaryContainer
          : theme.colorScheme.surfaceContainerLowest,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
        side: BorderSide(
          color: selected
              ? theme.colorScheme.primary
              : theme.colorScheme.outlineVariant,
          width: selected ? 2 : 1,
        ),
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(10),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Row(
            children: [
              Icon(
                selected
                    ? Icons.radio_button_checked
                    : Icons.radio_button_unchecked,
                size: 20,
                color: selected
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurfaceVariant,
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(template.label, style: theme.textTheme.titleSmall),
                    const SizedBox(height: 2),
                    Text(
                      template.description,
                      style: theme.textTheme.bodySmall,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 12),
              Text(
                '${template.fileCount} files',
                style: theme.textTheme.labelMedium,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
