import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';

class LogViewer extends StatefulWidget {
  const LogViewer({super.key, required this.logs, this.live = false});

  final List<LogDto> logs;
  final bool live;

  @override
  State<LogViewer> createState() => _LogViewerState();
}

class _LogViewerState extends State<LogViewer> {
  final _scroll = ScrollController();
  final _search = TextEditingController();
  String _level = 'all';
  String _source = 'all';
  bool _follow = true;

  @override
  void didUpdateWidget(covariant LogViewer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_follow && widget.logs.length != oldWidget.logs.length) {
      _scrollToEnd();
    }
  }

  @override
  void dispose() {
    _scroll.dispose();
    _search.dispose();
    super.dispose();
  }

  void _scrollToEnd() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scroll.hasClients) {
        _scroll.jumpTo(_scroll.position.maxScrollExtent);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final query = _search.text.trim().toLowerCase();
    final filtered = widget.logs.where((log) {
      if (_level != 'all' && log.level.name != _level) return false;
      if (_source != 'all' && log.source.name != _source) return false;
      if (query.isNotEmpty) {
        final haystack = '${log.message} ${log.contextJson}'.toLowerCase();
        if (!haystack.contains(query)) return false;
      }
      return true;
    }).toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.only(bottom: 10),
          child: Wrap(
            spacing: 10,
            runSpacing: 8,
            crossAxisAlignment: WrapCrossAlignment.center,
            children: [
              SizedBox(
                width: 150,
                child: DropdownButtonFormField<String>(
                  initialValue: _level,
                  decoration: const InputDecoration(
                    labelText: 'Level',
                    isDense: true,
                  ),
                  items: const [
                    DropdownMenuItem(value: 'all', child: Text('All levels')),
                    DropdownMenuItem(value: 'debug', child: Text('Debug')),
                    DropdownMenuItem(value: 'info', child: Text('Info')),
                    DropdownMenuItem(value: 'warn', child: Text('Warning')),
                    DropdownMenuItem(value: 'error', child: Text('Error')),
                  ],
                  onChanged: (value) => setState(() => _level = value ?? 'all'),
                ),
              ),
              SizedBox(
                width: 170,
                child: DropdownButtonFormField<String>(
                  initialValue: _source,
                  decoration: const InputDecoration(
                    labelText: 'Source',
                    isDense: true,
                  ),
                  items: [
                    const DropdownMenuItem(
                      value: 'all',
                      child: Text('All sources'),
                    ),
                    for (final source in LogSourceDto.values)
                      DropdownMenuItem(
                        value: source.name,
                        child: Text(_sourceLabel(source)),
                      ),
                  ],
                  onChanged: (value) =>
                      setState(() => _source = value ?? 'all'),
                ),
              ),
              SizedBox(
                width: 260,
                child: TextField(
                  controller: _search,
                  decoration: const InputDecoration(
                    labelText: 'Search',
                    prefixIcon: Icon(Icons.search, size: 18),
                    isDense: true,
                  ),
                  onChanged: (_) => setState(() {}),
                ),
              ),
              Text(
                '${filtered.length} of ${widget.logs.length} lines',
                style: theme.textTheme.bodySmall,
              ),
              const SizedBox(width: 8),
              FilterChip(
                selected: _follow,
                label: const Text('Follow'),
                onSelected: (value) {
                  setState(() => _follow = value);
                  if (value) _scrollToEnd();
                },
              ),
            ],
          ),
        ),
        Expanded(
          child: Card(
            child: filtered.isEmpty
                ? KoncetEmptyState(
                    icon: Icons.subject_outlined,
                    title: widget.logs.isEmpty
                        ? 'No log lines yet'
                        : 'No log lines match the filter',
                    message: widget.logs.isEmpty
                        ? 'Structured logs appear once the session emits them.'
                        : 'Adjust the level, source, or search text.',
                  )
                : Scrollbar(
                    controller: _scroll,
                    child: ListView.builder(
                      controller: _scroll,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 8,
                      ),
                      itemCount: filtered.length,
                      itemBuilder: (context, index) {
                        final log = filtered[index];
                        final tone = toneForSeverity(log.level);
                        return Padding(
                          padding: const EdgeInsets.symmetric(vertical: 2),
                          child: Row(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              SizedBox(
                                width: 74,
                                child: Text(
                                  KoncetSimClock.formatDuration(log.simTimeMs),
                                  style: context.mono.small.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant,
                                  ),
                                ),
                              ),
                              SizedBox(
                                width: 86,
                                child: Text(
                                  labelForLogSource(log.source),
                                  style: context.mono.small.copyWith(
                                    color: theme.colorScheme.onSurfaceVariant,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              Icon(
                                iconForSeverity(log.level),
                                size: 13,
                                color: context.statusColors.foreground(tone),
                              ),
                              const SizedBox(width: 6),
                              Expanded(
                                child: SelectableText(
                                  log.message,
                                  style: context.mono.small.copyWith(
                                    color: log.level == SeverityDto.error
                                        ? context.statusColors.danger
                                        : null,
                                  ),
                                ),
                              ),
                            ],
                          ),
                        );
                      },
                    ),
                  ),
          ),
        ),
      ],
    );
  }

  static String _sourceLabel(LogSourceDto source) => switch (source) {
    LogSourceDto.engine => 'Engine',
    LogSourceDto.build => 'Build',
    LogSourceDto.container => 'Container',
    LogSourceDto.process => 'Process',
    LogSourceDto.proxy => 'Proxy',
    LogSourceDto.firewall => 'Firewall',
    LogSourceDto.deployment => 'Deployment',
  };
}
