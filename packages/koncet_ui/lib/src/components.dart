import 'package:material_ui/material_ui.dart';

import 'theme.dart';

/// Restrained marker that the surrounding data is simulated.
class KoncetSimulatedTag extends StatelessWidget {
  const KoncetSimulatedTag({super.key, this.compact = false});

  final bool compact;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final style = Theme.of(context).textTheme.labelMedium?.copyWith(
      letterSpacing: 0.08 * 12,
      fontWeight: FontWeight.w600,
      color: scheme.onSurfaceVariant,
    );
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: compact ? 6 : 8,
        vertical: compact ? 2 : 3,
      ),
      decoration: BoxDecoration(
        border: Border.all(color: scheme.outlineVariant),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.science_outlined,
            size: compact ? 12 : 14,
            color: scheme.onSurfaceVariant,
          ),
          const SizedBox(width: 4),
          Text('SIMULATED', style: style),
        ],
      ),
    );
  }
}

class KoncetStatusBadge extends StatelessWidget {
  const KoncetStatusBadge({
    super.key,
    required this.label,
    required this.tone,
    this.icon,
    this.dense = false,
  });

  final String label;
  final KoncetStatusTone tone;
  final IconData? icon;
  final bool dense;

  @override
  Widget build(BuildContext context) {
    final colors = context.statusColors;
    final foreground = colors.foreground(tone);
    final text = Theme.of(context).textTheme.labelMedium
        ?.copyWith(color: foreground, fontWeight: FontWeight.w600);
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: dense ? 8 : 10,
        vertical: dense ? 3 : 4,
      ),
      decoration: BoxDecoration(
        color: colors.container(tone),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: dense ? 13 : 15, color: foreground),
            const SizedBox(width: 5),
          ],
          Text(label, style: text),
        ],
      ),
    );
  }
}

class KoncetPageHeader extends StatelessWidget {
  const KoncetPageHeader({
    super.key,
    required this.title,
    this.description,
    this.breadcrumb,
    this.actions = const [],
    this.leading,
  });

  final String title;
  final String? description;
  final String? breadcrumb;
  final List<Widget> actions;
  final Widget? leading;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (breadcrumb != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 6),
            child: Text(
              breadcrumb!,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (leading != null) ...[leading!, const SizedBox(width: 12)],
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: theme.textTheme.headlineSmall),
                  if (description != null)
                    Padding(
                      padding: const EdgeInsets.only(top: 4),
                      child: Text(
                        description!,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                ],
              ),
            ),
            if (actions.isNotEmpty) ...[
              const SizedBox(width: 16),
              Wrap(spacing: 8, children: actions),
            ],
          ],
        ),
      ],
    );
  }
}

class KoncetSectionCard extends StatelessWidget {
  const KoncetSectionCard({
    super.key,
    required this.child,
    this.title,
    this.description,
    this.actions = const [],
    this.padding = const EdgeInsets.all(20),
  });

  final Widget child;
  final String? title;
  final String? description;
  final List<Widget> actions;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: padding,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (title != null)
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(title!, style: theme.textTheme.titleMedium),
                        if (description != null)
                          Padding(
                            padding: const EdgeInsets.only(top: 2),
                            child: Text(
                              description!,
                              style: theme.textTheme.bodySmall,
                            ),
                          ),
                      ],
                    ),
                  ),
                  if (actions.isNotEmpty) Wrap(spacing: 4, children: actions),
                ],
              ),
            if (title != null) const SizedBox(height: 16),
            child,
          ],
        ),
      ),
    );
  }
}

class KoncetEmptyState extends StatelessWidget {
  const KoncetEmptyState({
    super.key,
    required this.icon,
    required this.title,
    required this.message,
    this.action,
  });

  final IconData icon;
  final String title;
  final String message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 460),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 40, color: theme.colorScheme.primary),
              const SizedBox(height: 16),
              Text(
                title,
                style: theme.textTheme.titleLarge,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Text(
                message,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                textAlign: TextAlign.center,
              ),
              if (action != null) ...[const SizedBox(height: 20), action!],
            ],
          ),
        ),
      ),
    );
  }
}

class KoncetErrorState extends StatelessWidget {
  const KoncetErrorState({
    super.key,
    required this.title,
    required this.message,
    this.code,
    this.diagnosticId,
    this.onRetry,
    this.onCopy,
  });

  final String title;
  final String message;
  final String? code;
  final String? diagnosticId;
  final VoidCallback? onRetry;
  final VoidCallback? onCopy;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = context.statusColors;
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Card(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.error_outline, size: 36, color: colors.danger),
                  const SizedBox(height: 12),
                  Text(
                    title,
                    style: theme.textTheme.titleLarge,
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 8),
                  Text(
                    message,
                    style: theme.textTheme.bodyMedium,
                    textAlign: TextAlign.center,
                  ),
                  if (code != null || diagnosticId != null) ...[
                    const SizedBox(height: 12),
                    SelectableText(
                      [
                        if (code != null) 'Code: $code',
                        if (diagnosticId != null) 'Diagnostic: $diagnosticId',
                      ].join('  ·  '),
                      style: context.mono.small.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                      textAlign: TextAlign.center,
                    ),
                  ],
                  const SizedBox(height: 20),
                  Wrap(
                    spacing: 8,
                    children: [
                      if (onRetry != null)
                        FilledButton.icon(
                          onPressed: onRetry,
                          icon: const Icon(Icons.refresh, size: 18),
                          label: const Text('Retry'),
                        ),
                      if (onCopy != null)
                        TextButton.icon(
                          onPressed: onCopy,
                          icon: const Icon(Icons.copy, size: 18),
                          label: const Text('Copy diagnostics'),
                        ),
                    ],
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

enum KoncetButtonVariant { primary, secondary, tertiary, danger }

class KoncetButton extends StatelessWidget {
  const KoncetButton({
    super.key,
    required this.label,
    this.icon,
    this.onPressed,
    this.variant = KoncetButtonVariant.primary,
    this.busy = false,
  });

  final String label;
  final IconData? icon;
  final VoidCallback? onPressed;
  final KoncetButtonVariant variant;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    final iconWidget = busy
        ? const SizedBox(
            width: 16,
            height: 16,
            child: CircularProgressIndicator(strokeWidth: 2),
          )
        : (icon == null ? null : Icon(icon, size: 18));
    final child = Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (iconWidget != null) ...[iconWidget, const SizedBox(width: 8)],
        Text(label),
      ],
    );
    final effective = busy ? null : onPressed;
    return switch (variant) {
      KoncetButtonVariant.primary => FilledButton(
        onPressed: effective,
        child: child,
      ),
      KoncetButtonVariant.secondary => OutlinedButton(
        onPressed: effective,
        child: child,
      ),
      KoncetButtonVariant.tertiary => TextButton(
        onPressed: effective,
        child: child,
      ),
      KoncetButtonVariant.danger => FilledButton(
        style: FilledButton.styleFrom(
          backgroundColor: context.statusColors.danger,
          foregroundColor: Theme.of(context).colorScheme.onError,
        ),
        onPressed: effective,
        child: child,
      ),
    };
  }
}

class KoncetSimClock extends StatelessWidget {
  const KoncetSimClock({
    super.key,
    required this.elapsedMs,
    this.speed = 1,
    this.paused = false,
  });

  final int elapsedMs;
  final int speed;
  final bool paused;

  static String formatDuration(int milliseconds) {
    final totalSeconds = milliseconds ~/ 1000;
    final hours = totalSeconds ~/ 3600;
    final minutes = (totalSeconds % 3600) ~/ 60;
    final seconds = totalSeconds % 60;
    final tenths = (milliseconds % 1000) ~/ 100;
    if (hours > 0) {
      return '${hours.toString().padLeft(2, '0')}:'
          '${minutes.toString().padLeft(2, '0')}:'
          '${seconds.toString().padLeft(2, '0')}.$tenths';
    }
    return '${minutes.toString().padLeft(2, '0')}:'
        '${seconds.toString().padLeft(2, '0')}.$tenths';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(
          paused ? Icons.pause_circle_outline : Icons.timer_outlined,
          size: 18,
          color: theme.colorScheme.onSurfaceVariant,
        ),
        const SizedBox(width: 6),
        Text(formatDuration(elapsedMs), style: context.mono.medium),
        const SizedBox(width: 8),
        Text(
          '$speed×',
          style: theme.textTheme.labelMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }
}

enum KoncetStageStatus { pending, running, succeeded, failed, skipped }

@immutable
class KoncetStageView {
  const KoncetStageView({
    required this.label,
    required this.status,
    this.detail,
  });

  final String label;
  final KoncetStageStatus status;
  final String? detail;
}

class KoncetPipeline extends StatelessWidget {
  const KoncetPipeline({super.key, required this.stages});

  final List<KoncetStageView> stages;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = context.statusColors;
    return Column(
      children: [
        for (var index = 0; index < stages.length; index++)
          Padding(
            padding: EdgeInsets.only(
              bottom: index == stages.length - 1 ? 0 : 6,
            ),
            child: Row(
              children: [
                _stageIcon(colors, stages[index].status),
                const SizedBox(width: 10),
                SizedBox(
                  width: 90,
                  child: Text(
                    stages[index].label,
                    style: theme.textTheme.bodyMedium,
                  ),
                ),
                Expanded(
                  child: Text(
                    stages[index].detail ?? _stageLabel(stages[index].status),
                    style: theme.textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }

  static String _stageLabel(KoncetStageStatus status) => switch (status) {
    KoncetStageStatus.pending => 'Pending',
    KoncetStageStatus.running => 'Running',
    KoncetStageStatus.succeeded => 'Succeeded',
    KoncetStageStatus.failed => 'Failed',
    KoncetStageStatus.skipped => 'Skipped',
  };

  Widget _stageIcon(KoncetStatusColors colors, KoncetStageStatus status) {
    return switch (status) {
      KoncetStageStatus.pending => Icon(
        Icons.radio_button_unchecked,
        size: 16,
        color: colors.stopped,
      ),
      KoncetStageStatus.running => Icon(
        Icons.autorenew,
        size: 16,
        color: colors.info,
      ),
      KoncetStageStatus.succeeded => Icon(
        Icons.check_circle,
        size: 16,
        color: colors.healthy,
      ),
      KoncetStageStatus.failed => Icon(
        Icons.cancel,
        size: 16,
        color: colors.danger,
      ),
      KoncetStageStatus.skipped => Icon(
        Icons.remove_circle_outline,
        size: 16,
        color: colors.stopped,
      ),
    };
  }
}

class KoncetMetricTile extends StatelessWidget {
  const KoncetMetricTile({
    super.key,
    required this.label,
    required this.value,
    this.unit,
    this.caption,
  });

  final String label;
  final String value;
  final String? unit;
  final String? caption;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: theme.colorScheme.outlineVariant),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: theme.textTheme.labelMedium),
          const SizedBox(height: 4),
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text(value, style: context.mono.medium.copyWith(fontSize: 20)),
              if (unit != null) ...[
                const SizedBox(width: 4),
                Text(unit!, style: theme.textTheme.bodySmall),
              ],
            ],
          ),
          if (caption != null)
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Text(caption!, style: theme.textTheme.bodySmall),
            ),
        ],
      ),
    );
  }
}

class KoncetReasonCallout extends StatelessWidget {
  const KoncetReasonCallout({
    super.key,
    required this.title,
    required this.message,
    this.tone = KoncetStatusTone.warning,
    this.action,
  });

  final String title;
  final String message;
  final KoncetStatusTone tone;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final colors = context.statusColors;
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: colors.container(tone),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colors.foreground(tone).withValues(alpha: 0.35),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.info_outline, color: colors.foreground(tone), size: 20),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.textTheme.titleSmall?.copyWith(
                    color: colors.foreground(tone),
                  ),
                ),
                const SizedBox(height: 2),
                Text(message, style: theme.textTheme.bodySmall),
              ],
            ),
          ),
          if (action != null) ...[const SizedBox(width: 8), action!],
        ],
      ),
    );
  }
}

class KoncetValidationMessage extends StatelessWidget {
  const KoncetValidationMessage({
    super.key,
    required this.message,
    this.path,
    this.isWarning = false,
  });

  final String message;
  final String? path;
  final bool isWarning;

  @override
  Widget build(BuildContext context) {
    final colors = context.statusColors;
    final color = isWarning ? colors.warning : colors.danger;
    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            isWarning ? Icons.warning_amber_outlined : Icons.error_outline,
            size: 14,
            color: color,
          ),
          const SizedBox(width: 6),
          Expanded(
            child: Text.rich(
              TextSpan(
                children: [
                  if (path != null && path!.isNotEmpty)
                    TextSpan(
                      text: '$path  ',
                      style: context.mono.small.copyWith(color: color),
                    ),
                  TextSpan(text: message),
                ],
              ),
              style: Theme.of(context).textTheme.bodySmall
                  ?.copyWith(color: color),
            ),
          ),
        ],
      ),
    );
  }
}

class KoncetInfoRow extends StatelessWidget {
  const KoncetInfoRow({
    super.key,
    required this.label,
    required this.value,
    this.mono = false,
  });

  final String label;
  final String value;
  final bool mono;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 150,
            child: Text(
              label,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(
            child: SelectableText(
              value,
              style: mono ? context.mono.small : theme.textTheme.bodyMedium,
            ),
          ),
        ],
      ),
    );
  }
}

Future<bool> showKoncetConfirm(
  BuildContext context, {
  required String title,
  required String message,
  required String confirmLabel,
  bool destructive = false,
}) async {
  final result = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 440),
        child: Text(message),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          style: destructive
              ? FilledButton.styleFrom(
                  backgroundColor: context.statusColors.danger,
                  foregroundColor: Theme.of(context).colorScheme.onError,
                )
              : null,
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(confirmLabel),
        ),
      ],
    ),
  );
  return result ?? false;
}

class KoncetFileTree extends StatelessWidget {
  const KoncetFileTree({
    super.key,
    required this.paths,
    required this.selected,
    required this.onSelected,
    this.trailingBuilder,
  });

  final List<String> paths;
  final String? selected;
  final ValueChanged<String> onSelected;
  final Widget Function(String path)? trailingBuilder;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListView.builder(
      shrinkWrap: true,
      itemCount: paths.length,
      itemBuilder: (context, index) {
        final path = paths[index];
        final isSelected = path == selected;
        return Material(
          color: isSelected
              ? theme.colorScheme.secondaryContainer
              : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
          child: InkWell(
            borderRadius: BorderRadius.circular(6),
            onTap: () => onSelected(path),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 7),
              child: Row(
                children: [
                  Icon(
                    path.contains('.')
                        ? Icons.description_outlined
                        : Icons.folder_outlined,
                    size: 16,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      path,
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontFamily: koncetMonoFontFamily,
                        fontWeight: isSelected ? FontWeight.w600 : null,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  if (trailingBuilder != null) trailingBuilder!(path),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
