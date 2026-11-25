import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/providers.dart';
import '../shell.dart';

class SourceEditorScreen extends ConsumerStatefulWidget {
  const SourceEditorScreen({super.key, required this.projectId});

  final String projectId;

  @override
  ConsumerState<SourceEditorScreen> createState() => _SourceEditorScreenState();
}

class _SourceEditorScreenState extends ConsumerState<SourceEditorScreen> {
  final _code = TextEditingController();
  final _editorScroll = ScrollController();
  final _gutterScroll = ScrollController();
  Timer? _debounce;
  ProjectDetailDto? _project;
  String? _selected;
  String _savedContent = '';
  bool _initialised = false;
  bool _saving = false;
  String? _saveError;
  String _savedLabel = 'Saved';

  @override
  void initState() {
    super.initState();
    _editorScroll.addListener(_syncGutter);
  }

  @override
  void dispose() {
    _debounce?.cancel();
    final path = _selected;
    if (_dirty && path != null) {
      unawaited(
        ref
            .read(apiProvider)
            .writeSourceFile(widget.projectId, path, _code.text)
            .catchError((_) => _project!),
      );
    }
    _editorScroll
      ..removeListener(_syncGutter)
      ..dispose();
    _gutterScroll.dispose();
    _code.dispose();
    super.dispose();
  }

  bool get _dirty => _selected != null && _code.text != _savedContent;

  void _syncGutter() {
    if (_gutterScroll.hasClients && _editorScroll.hasClients) {
      final target = _editorScroll.offset.clamp(
        0.0,
        _gutterScroll.position.maxScrollExtent,
      );
      _gutterScroll.jumpTo(target);
    }
  }

  @override
  Widget build(BuildContext context) {
    final project = ref.watch(projectProvider(widget.projectId));
    if (!_initialised && project.hasValue) {
      _initialise(project.value!);
    }
    if (!_initialised) {
      return project.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (error, stack) => KoncetErrorState(
          title: 'Source tree could not be loaded',
          message: error.toString(),
          onRetry: () => ref.invalidate(projectProvider(widget.projectId)),
        ),
        data: (_) => const SizedBox.shrink(),
      );
    }
    final detail = _project!;
    final selectedFile = _selected == null
        ? null
        : detail.files.where((file) => file.path == _selected).firstOrNull;
    final theme = Theme.of(context);
    final lineCount = _code.text.split('\n').length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        KoncetPageHeader(
          breadcrumb: 'Projects / ${detail.name}',
          title: 'Source editor',
          description: 'Edit the virtual source tree. Changes autosave locally; a run snapshot never changes after it starts.',
          actions: [
            KoncetButton(
              label: 'Validate',
              icon: Icons.fact_check_outlined,
              variant: KoncetButtonVariant.secondary,
              onPressed: _validate,
            ),
            KoncetButton(
              label: 'Save now',
              icon: Icons.save_outlined,
              busy: _saving,
              onPressed: _dirty ? _save : null,
            ),
            IconButton(
              tooltip: 'Back to project',
              onPressed: () => context.go('/projects/${widget.projectId}'),
              icon: const Icon(Icons.close),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Expanded(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SizedBox(
                width: 280,
                child: Card(
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        Row(
                          children: [
                            Text('Files', style: theme.textTheme.titleSmall),
                            const Spacer(),
                            IconButton(
                              tooltip: 'New file',
                              onPressed: _createFile,
                              icon: const Icon(Icons.add, size: 18),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        Expanded(
                          child: SingleChildScrollView(
                            child: KoncetFileTree(
                              paths: [
                                for (final file in detail.files) file.path,
                              ],
                              selected: _selected,
                              onSelected: _selectFile,
                              trailingBuilder: (path) =>
                                  PopupMenuButton<String>(
                                    tooltip: 'File actions',
                                    padding: EdgeInsets.zero,
                                    iconSize: 16,
                                    onSelected: (value) {
                                      if (value == 'rename') _renameFile(path);
                                      if (value == 'delete') _deleteFile(path);
                                    },
                                    itemBuilder: (context) => const [
                                      PopupMenuItem(
                                        value: 'rename',
                                        child: Text('Rename'),
                                      ),
                                      PopupMenuItem(
                                        value: 'delete',
                                        child: Text('Delete'),
                                      ),
                                    ],
                                  ),
                            ),
                          ),
                        ),
                        const Divider(),
                        Text(
                          '${detail.files.length} files · ${formatBytes(detail.totalBytes)}',
                          style: theme.textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Card(
                  clipBehavior: Clip.antiAlias,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 10,
                        ),
                        decoration: BoxDecoration(
                          color: theme.colorScheme.surfaceContainerLow,
                          border: Border(
                            bottom: BorderSide(
                              color: theme.colorScheme.outlineVariant,
                            ),
                          ),
                        ),
                        child: Row(
                          children: [
                            Icon(
                              Icons.description_outlined,
                              size: 16,
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                            const SizedBox(width: 8),
                            Text(
                              selectedFile?.path ?? 'Select a file',
                              style: context.mono.small,
                            ),
                            const Spacer(),
                            _SaveIndicator(
                              saving: _saving,
                              error: _saveError,
                              label: _savedLabel,
                            ),
                          ],
                        ),
                      ),
                      Expanded(
                        child: selectedFile == null
                            ? const KoncetEmptyState(
                                icon: Icons.description_outlined,
                                title: 'Select a source file',
                                message:
                                    'Choose a file from the tree to edit it.',
                              )
                            : Row(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  Container(
                                    width: 52,
                                    color:
                                        theme.colorScheme.surfaceContainerLow,
                                    child: SingleChildScrollView(
                                      controller: _gutterScroll,
                                      physics:
                                          const NeverScrollableScrollPhysics(),
                                      padding: const EdgeInsets.only(
                                        top: 12,
                                        bottom: 12,
                                        right: 8,
                                      ),
                                      child: Text(
                                        List.generate(
                                          lineCount,
                                          (index) => '${index + 1}',
                                        ).join('\n'),
                                        textAlign: TextAlign.right,
                                        style: context.mono.small.copyWith(
                                          height: 18 / 12,
                                          color: theme
                                              .colorScheme
                                              .onSurfaceVariant
                                              .withValues(alpha: 0.7),
                                        ),
                                      ),
                                    ),
                                  ),
                                  VerticalDivider(
                                    width: 1,
                                    color: theme.colorScheme.outlineVariant,
                                  ),
                                  Expanded(
                                    child: TextField(
                                      controller: _code,
                                      scrollController: _editorScroll,
                                      maxLines: null,
                                      expands: true,
                                      keyboardType: TextInputType.multiline,
                                      textAlignVertical: TextAlignVertical.top,
                                      style: context.mono.small.copyWith(
                                        fontSize: 13,
                                        height: 18 / 13,
                                      ),
                                      decoration: const InputDecoration(
                                        border: InputBorder.none,
                                        enabledBorder: InputBorder.none,
                                        focusedBorder: InputBorder.none,
                                        filled: false,
                                        contentPadding: EdgeInsets.all(12),
                                      ),
                                      onChanged: (_) => _onChanged(),
                                    ),
                                  ),
                                ],
                              ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 6,
                        ),
                        decoration: BoxDecoration(
                          color: theme.colorScheme.surfaceContainerLow,
                          border: Border(
                            top: BorderSide(
                              color: theme.colorScheme.outlineVariant,
                            ),
                          ),
                        ),
                        child: Row(
                          children: [
                            Text(
                              selectedFile == null
                                  ? '—'
                                  : '${selectedFile.language} · $lineCount lines · ${selectedFile.content.length} chars',
                              style: theme.textTheme.bodySmall,
                            ),
                            const Spacer(),
                            Text(
                              'Saved locally · not executed',
                              style: theme.textTheme.bodySmall,
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
        if (_saveError != null) ...[
          const SizedBox(height: 8),
          KoncetValidationMessage(message: _saveError!),
        ],
      ],
    );
  }

  void _initialise(ProjectDetailDto detail) {
    _project = detail;
    _initialised = true;
    if (detail.files.isNotEmpty) {
      _selectFile(detail.files.first.path);
    }
  }

  void _selectFile(String path) {
    final detail = _project;
    if (detail == null) return;
    if (_dirty) {
      unawaited(_save());
    }
    final file = detail.files.where((item) => item.path == path).firstOrNull;
    if (file == null) return;
    setState(() {
      _selected = path;
      _code.text = file.content;
      _savedContent = file.content;
      _saveError = null;
      _savedLabel = 'Saved';
    });
  }

  void _onChanged() {
    _debounce?.cancel();
    _saveError = null;
    _savedLabel = 'Unsaved';
    setState(() {});
    _debounce = Timer(const Duration(milliseconds: 700), () {
      if (_dirty) unawaited(_save());
    });
  }

  Future<void> _save() async {
    final path = _selected;
    final detail = _project;
    if (path == null || detail == null || _saving) return;
    if (_code.text == _savedContent) {
      setState(() => _savedLabel = 'Saved');
      return;
    }
    final content = _code.text;
    _debounce?.cancel();
    setState(() {
      _saving = true;
      _saveError = null;
      _savedLabel = 'Saving…';
    });
    try {
      final updated = await ref
          .read(apiProvider)
          .writeSourceFile(widget.projectId, path, content);
      if (!mounted) return;
      setState(() {
        _project = updated;
        _saving = false;
        if (_selected == path) {
          _savedContent = content;
          _savedLabel = 'Saved';
        }
      });
      ref.invalidate(projectProvider(widget.projectId));
      ref.invalidate(projectsProvider(false));
      ref.invalidate(projectsProvider(true));
      if (_dirty) {
        _debounce = Timer(const Duration(milliseconds: 400), () {
          if (_dirty) unawaited(_save());
        });
      }
    } on BridgeError catch (error) {
      if (!mounted) return;
      setState(() {
        _saving = false;
        if (_selected == path) {
          _saveError = error.message;
          _savedLabel = 'Save failed';
        }
      });
    }
  }

  Future<void> _createFile() async {
    final path = await _promptPath(
      title: 'New source file',
      label: 'Path',
      hint: 'src/main.js',
      confirmLabel: 'Create file',
    );
    if (path == null) return;
    try {
      final updated = await ref
          .read(apiProvider)
          .createSourceFile(widget.projectId, path, '');
      if (!mounted) return;
      setState(() => _project = updated);
      _selectFile(path);
      ref.invalidate(projectProvider(widget.projectId));
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }

  Future<void> _renameFile(String from) async {
    final to = await _promptPath(
      title: 'Rename source file',
      label: 'New path',
      initial: from,
      confirmLabel: 'Rename',
    );
    if (to == null || to == from) return;
    try {
      final updated = await ref
          .read(apiProvider)
          .renameSourceFile(widget.projectId, from, to);
      if (!mounted) return;
      setState(() => _project = updated);
      _selectFile(to);
      ref.invalidate(projectProvider(widget.projectId));
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }

  Future<void> _deleteFile(String path) async {
    final confirmed = await showKoncetConfirm(
      context,
      title: 'Delete $path?',
      message: 'The file is removed from the virtual source tree. Existing run snapshots are unaffected.',
      confirmLabel: 'Delete file',
      destructive: true,
    );
    if (!confirmed) return;
    try {
      final updated = await ref
          .read(apiProvider)
          .deleteSourceFile(widget.projectId, path);
      if (!mounted) return;
      setState(() {
        _project = updated;
        _selected = null;
        _code.clear();
        _savedContent = '';
      });
      if (updated.files.isNotEmpty) _selectFile(updated.files.first.path);
      ref.invalidate(projectProvider(widget.projectId));
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }

  Future<void> _validate() async {
    try {
      final report = await ref
          .read(apiProvider)
          .validateSource(widget.projectId);
      if (!mounted) return;
      await showDialog<void>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(report.valid ? 'Source is valid' : 'Source issues'),
          content: report.issues.isEmpty
              ? const Text('The virtual source tree passes integrity checks.')
              : Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (final issue in report.issues)
                      KoncetValidationMessage(
                        message: issue.message,
                        path: issue.path,
                        isWarning: issue.severity == 'warning',
                      ),
                  ],
                ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('Close'),
            ),
          ],
        ),
      );
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }

  Future<String?> _promptPath({
    required String title,
    required String label,
    required String confirmLabel,
    String? hint,
    String? initial,
  }) async {
    final controller = TextEditingController(text: initial);
    final result = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: SizedBox(
          width: 420,
          child: TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(labelText: label, hintText: hint),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(controller.text.trim()),
            child: Text(confirmLabel),
          ),
        ],
      ),
    );
    controller.dispose();
    if (result == null || result.isEmpty) return null;
    return result;
  }
}

class _SaveIndicator extends StatelessWidget {
  const _SaveIndicator({
    required this.saving,
    required this.error,
    required this.label,
  });

  final bool saving;
  final String? error;
  final String label;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final tone = error != null
        ? context.statusColors.danger
        : saving
        ? context.statusColors.info
        : context.statusColors.healthy;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (saving)
          const SizedBox(
            width: 12,
            height: 12,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        else
          Icon(
            error != null ? Icons.error_outline : Icons.cloud_done_outlined,
            size: 14,
            color: tone,
          ),
        const SizedBox(width: 6),
        Text(label, style: theme.textTheme.bodySmall?.copyWith(color: tone)),
      ],
    );
  }
}
