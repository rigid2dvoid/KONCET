import 'dart:convert';

import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';

class TimelineView extends StatefulWidget {
  const TimelineView({super.key, required this.events});

  final List<EventDto> events;

  @override
  State<TimelineView> createState() => _TimelineViewState();
}

class _TimelineViewState extends State<TimelineView> {
  bool _showDebug = false;
  String _query = '';

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final query = _query.trim().toLowerCase();
    final events = widget.events
        .where((event) {
          if (!_showDebug && event.severity == SeverityDto.debug) return false;
          if (query.isEmpty) return true;
          return event.eventType.toLowerCase().contains(query) ||
              event.payloadJson.toLowerCase().contains(query);
        })
        .toList()
        .reversed
        .toList();

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
                width: 280,
                child: TextField(
                  decoration: const InputDecoration(
                    labelText: 'Search events',
                    prefixIcon: Icon(Icons.search, size: 18),
                    isDense: true,
                  ),
                  onChanged: (value) => setState(() => _query = value),
                ),
              ),
              FilterChip(
                selected: _showDebug,
                label: const Text('Show debug'),
                onSelected: (value) => setState(() => _showDebug = value),
              ),
              Text(
                '${events.length} event(s) · newest first',
                style: theme.textTheme.bodySmall,
              ),
            ],
          ),
        ),
        Expanded(
          child: Card(
            child: events.isEmpty
                ? const KoncetEmptyState(
                    icon: Icons.timeline_outlined,
                    title: 'No timeline events',
                    message: 'Events describe build stages, lifecycle transitions, probes, and failures.',
                  )
                : ListView.separated(
                    padding: const EdgeInsets.all(12),
                    itemCount: events.length,
                    separatorBuilder: (_, _) => const Divider(height: 1),
                    itemBuilder: (context, index) =>
                        _EventRow(event: events[index]),
                  ),
          ),
        ),
      ],
    );
  }
}

class _EventRow extends StatelessWidget {
  const _EventRow({required this.event});

  final EventDto event;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final tone = toneForSeverity(event.severity);
    final color = context.statusColors.foreground(tone);
    return Theme(
      data: theme.copyWith(dividerColor: Colors.transparent),
      child: ExpansionTile(
        tilePadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 0),
        childrenPadding: const EdgeInsets.fromLTRB(34, 0, 8, 12),
        leading: Icon(iconForSeverity(event.severity), size: 18, color: color),
        title: Row(
          children: [
            SizedBox(
              width: 74,
              child: Text(
                KoncetSimClock.formatDuration(event.simTimeMs),
                style: context.mono.small.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            Expanded(
              child: Text(
                eventSummary(event),
                style: theme.textTheme.bodyMedium,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const SizedBox(width: 8),
            Text(
              '#${event.seq}',
              style: context.mono.small.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        subtitle: Padding(
          padding: const EdgeInsets.only(left: 74, top: 2),
          child: Text(
            '${event.eventType} · ${event.source.name}',
            style: theme.textTheme.bodySmall,
          ),
        ),
        children: [
          Align(
            alignment: Alignment.centerLeft,
            child: SelectableText(
              _pretty(event.payloadJson),
              style: context.mono.small,
            ),
          ),
        ],
      ),
    );
  }

  static String _pretty(String json) {
    try {
      return const JsonEncoder.withIndent('  ').convert(jsonDecode(json));
    } on FormatException {
      return json;
    }
  }
}
