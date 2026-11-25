import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../rust/error.dart';
import '../state/providers.dart';

class SplashScreen extends StatelessWidget {
  const SplashScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            'KONCET',
            style: theme.textTheme.displaySmall?.copyWith(
              letterSpacing: 0.08 * 32,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Deployment simulation workbench',
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 24),
          const SizedBox(
            width: 160,
            child: LinearProgressIndicator(minHeight: 2),
          ),
        ],
      ),
    );
  }
}

class RecoveryScreen extends ConsumerStatefulWidget {
  const RecoveryScreen({super.key, required this.error});

  final Object error;

  @override
  ConsumerState<RecoveryScreen> createState() => _RecoveryScreenState();
}

class _RecoveryScreenState extends ConsumerState<RecoveryScreen> {
  bool _busy = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final error = widget.error;
    final bridgeError = error is BridgeError ? error : null;
    final code = bridgeError?.code;
    final likelyLock = code == 'lock_stale' || code == 'lock_held';

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 620),
        child: Card(
          child: Padding(
            padding: const EdgeInsets.all(28),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.health_and_safety_outlined,
                      color: context.statusColors.warning,
                      size: 28,
                    ),
                    const SizedBox(width: 10),
                    Text(
                      'KONCET could not start',
                      style: theme.textTheme.titleLarge,
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Text(
                  bridgeError?.message ?? 'The local data store could not be opened. No run has started.',
                  style: theme.textTheme.bodyMedium,
                ),
                if (code != null) ...[
                  const SizedBox(height: 10),
                  SelectableText(
                    'Code: $code'
                    '${bridgeError?.diagnosticId.isNotEmpty == true ? '  ·  Diagnostic: ${bridgeError!.diagnosticId}' : ''}',
                    style: context.mono.small.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
                const SizedBox(height: 16),
                Text(
                  likelyLock
                      ? 'Another process may be using the data directory, or a previous KONCET process did not exit cleanly. Only remove a stale lock if no other KONCET window is open.'
                      : 'Your data has not been deleted. Retry after resolving the condition shown above, or copy the diagnostics for support.',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                const SizedBox(height: 22),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    FilledButton.icon(
                      onPressed: _busy ? null : () => _retry(false),
                      icon: const Icon(Icons.refresh, size: 18),
                      label: const Text('Retry'),
                    ),
                    if (likelyLock)
                      OutlinedButton.icon(
                        onPressed: _busy ? null : () => _retry(true),
                        icon: const Icon(Icons.lock_open, size: 18),
                        label: const Text('Remove stale lock and retry'),
                      ),
                    TextButton.icon(
                      onPressed: _copyDiagnostics,
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
    );
  }

  Future<void> _retry(bool allowStaleLock) async {
    setState(() => _busy = true);
    await ref
        .read(bootstrapProvider.notifier)
        .retry(allowStaleLock: allowStaleLock);
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _copyDiagnostics() async {
    final error = widget.error;
    final bridgeError = error is BridgeError ? error : null;
    final text = [
      'KONCET startup failure',
      'code: ${bridgeError?.code ?? 'unknown'}',
      'message: ${bridgeError?.message ?? error}',
      'target: ${bridgeError?.target ?? ''}',
      'diagnostic: ${bridgeError?.diagnosticId ?? ''}',
    ].join('\n');
    await Clipboard.setData(ClipboardData(text: text));
    if (mounted) {
      ScaffoldMessenger.of(context)
          .showSnackBar(const SnackBar(content: Text('Diagnostics copied')));
    }
  }
}
