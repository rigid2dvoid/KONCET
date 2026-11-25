import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import '../format.dart';
import '../rust/dto.dart';
import '../rust/error.dart';
import '../state/app_settings.dart';
import '../state/providers.dart';
import '../shell.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  bool _backupBusy = false;
  bool _diagnosticsBusy = false;
  String? _diagnosticsResult;

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(appSettingsProvider);
    final usage = ref.watch(usageProvider);
    final backups = ref.watch(backupsProvider);
    final bootstrap = ref.watch(bootstrapProvider);
    final info = bootstrap.value;
    final current = settings.value ?? const AppSettings();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const KoncetPageHeader(
          title: 'Settings',
          description: 'Preferences, local storage, backups, and diagnostics. KONCET performs no network calls and sends no telemetry.',
        ),
        const SizedBox(height: 16),
        Expanded(
          child: SingleChildScrollView(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                KoncetSectionCard(
                  title: 'Appearance',
                  child: Column(
                    children: [
                      Wrap(
                        spacing: 16,
                        runSpacing: 12,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: [
                          SizedBox(
                            width: 240,
                            child: DropdownButtonFormField<ThemePreference>(
                              initialValue: current.theme,
                              decoration: const InputDecoration(
                                labelText: 'Theme',
                              ),
                              items: const [
                                DropdownMenuItem(
                                  value: ThemePreference.system,
                                  child: Text('Follow system'),
                                ),
                                DropdownMenuItem(
                                  value: ThemePreference.light,
                                  child: Text('Light'),
                                ),
                                DropdownMenuItem(
                                  value: ThemePreference.dark,
                                  child: Text('Dark'),
                                ),
                              ],
                              onChanged: (value) {
                                if (value != null) {
                                  ref
                                      .read(appSettingsProvider.notifier)
                                      .setTheme(value);
                                }
                              },
                            ),
                          ),
                          SizedBox(
                            width: 320,
                            child: SwitchListTile(
                              contentPadding: EdgeInsets.zero,
                              title: const Text('Reduce motion'),
                              subtitle: const Text(
                                'Minimize optional transitions.',
                              ),
                              value: current.reduceMotion,
                              onChanged: (value) => ref
                                  .read(appSettingsProvider.notifier)
                                  .setReduceMotion(value),
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
                KoncetSectionCard(
                  title: 'Simulation defaults',
                  child: Wrap(
                    spacing: 16,
                    runSpacing: 12,
                    children: [
                      SizedBox(
                        width: 220,
                        child: DropdownButtonFormField<int>(
                          initialValue: current.defaultSpeed,
                          decoration: const InputDecoration(
                            labelText: 'Default speed',
                          ),
                          items: const [
                            DropdownMenuItem(value: 1, child: Text('1×')),
                            DropdownMenuItem(value: 2, child: Text('2×')),
                            DropdownMenuItem(value: 4, child: Text('4×')),
                            DropdownMenuItem(value: 8, child: Text('8×')),
                          ],
                          onChanged: (value) {
                            if (value != null) {
                              ref
                                  .read(appSettingsProvider.notifier)
                                  .setDefaultSpeed(value);
                            }
                          },
                        ),
                      ),
                      const Padding(
                        padding: EdgeInsets.only(top: 10),
                        child: Text(
                          'Applied when a run starts. The seed override remains optional.',
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
                KoncetSectionCard(
                  title: 'Local storage',
                  description: 'All data stays in the XDG data directory. Backups are verified SQLite copies.',
                  actions: [
                    KoncetButton(
                      label: 'Create backup',
                      icon: Icons.backup_outlined,
                      busy: _backupBusy,
                      onPressed: _createBackup,
                    ),
                  ],
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      usage.when(
                        loading: () => const LinearProgressIndicator(),
                        error: (error, stack) => Text('$error'),
                        data: (value) => Wrap(
                          spacing: 10,
                          runSpacing: 10,
                          children: [
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Database',
                                value: formatBytes(value.databaseBytes),
                              ),
                            ),
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Backups',
                                value: formatBytes(value.backupBytes),
                              ),
                            ),
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Projects',
                                value: '${value.projectCount}',
                              ),
                            ),
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Runs',
                                value: '${value.runCount}',
                              ),
                            ),
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Events',
                                value: '${value.eventCount}',
                              ),
                            ),
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Logs',
                                value: '${value.logCount}',
                              ),
                            ),
                            SizedBox(
                              width: 148,
                              child: KoncetMetricTile(
                                label: 'Metrics',
                                value: '${value.metricCount}',
                              ),
                            ),
                          ],
                        ),
                      ),
                      if (info != null) ...[
                        const SizedBox(height: 12),
                        KoncetInfoRow(
                          label: 'Data directory',
                          value: info.dataRoot,
                          mono: true,
                        ),
                      ],
                      const SizedBox(height: 16),
                      Row(
                        children: [
                          Text(
                            'Backups',
                            style: Theme.of(context).textTheme.titleSmall,
                          ),
                          const Spacer(),
                          TextButton(
                            onPressed: () => ref.invalidate(backupsProvider),
                            child: const Text('Refresh'),
                          ),
                          TextButton.icon(
                            onPressed: _diagnosticsBusy ? null : _runCheck,
                            icon: const Icon(Icons.health_and_safety, size: 17),
                            label: const Text('Integrity check'),
                          ),
                        ],
                      ),
                      if (_diagnosticsResult != null)
                        KoncetValidationMessage(
                          message: _diagnosticsResult!,
                          isWarning: !_diagnosticsResult!.contains('passed'),
                        ),
                      backups.when(
                        loading: () => const LinearProgressIndicator(),
                        error: (error, stack) => Text('$error'),
                        data: (items) => items.isEmpty
                            ? Text(
                                'No backups yet.',
                                style: Theme.of(context).textTheme.bodySmall,
                              )
                            : Column(
                                children: [
                                  for (final backup in items)
                                    _BackupRow(
                                      backup: backup,
                                      onDelete: () => _deleteBackup(backup),
                                    ),
                                ],
                              ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
                KoncetSectionCard(
                  title: 'About',
                  child: Column(
                    children: [
                      KoncetInfoRow(
                        label: 'KONCET',
                        value: info == null ? 'Loading…' : info.appVersion,
                        mono: true,
                      ),
                      KoncetInfoRow(
                        label: 'Build',
                        value: info == null
                            ? '—'
                            : '${info.buildCommit} · ${info.buildDate}',
                        mono: true,
                      ),
                      KoncetInfoRow(
                        label: 'Engine',
                        value: info?.engineVersion ?? '—',
                        mono: true,
                      ),
                      KoncetInfoRow(
                        label: 'Schema',
                        value: info == null ? '—' : 'v${info.schemaVersion}',
                      ),
                      KoncetInfoRow(
                        label: 'Platform',
                        value: info?.platform ?? '—',
                      ),
                      KoncetInfoRow(
                        label: 'Licence',
                        value: 'GPL-3.0-or-later · no auto-update',
                      ),
                      const SizedBox(height: 8),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: Text(
                          'KONCET is a deterministic simulation workbench. It does not run containers, open sockets, change firewalls, or contact external services.',
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 8,
                        children: [
                          OutlinedButton.icon(
                            onPressed: _copyDiagnostics,
                            icon: const Icon(Icons.copy, size: 18),
                            label: const Text('Copy diagnostics'),
                          ),
                          OutlinedButton.icon(
                            onPressed: () =>
                                ref.invalidate(diagnosticsProvider),
                            icon: const Icon(
                              Icons.monitor_heart_outlined,
                              size: 18,
                            ),
                            label: const Text('Refresh diagnostics'),
                          ),
                          OutlinedButton.icon(
                            onPressed: () => showLicensePage(
                              context: context,
                              applicationName: 'KONCET',
                              applicationVersion: info?.appVersion,
                              applicationLegalese: 'GPL-3.0-or-later · deployment simulation workbench',
                            ),
                            icon: const Icon(
                              Icons.description_outlined,
                              size: 18,
                            ),
                            label: const Text('Licences'),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Future<void> _createBackup() async {
    setState(() => _backupBusy = true);
    try {
      final backup = await ref.read(apiProvider).createBackup();
      ref.invalidate(backupsProvider);
      ref.invalidate(usageProvider);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Backup created · ${formatBytes(backup.sizeBytes)}'),
          ),
        );
      }
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    } finally {
      if (mounted) setState(() => _backupBusy = false);
    }
  }

  Future<void> _deleteBackup(BackupInfoDto backup) async {
    final confirmed = await showKoncetConfirm(
      context,
      title: 'Delete this backup?',
      message: 'The backup file is removed from the managed backup directory. The current database is untouched.',
      confirmLabel: 'Delete backup',
      destructive: true,
    );
    if (!confirmed) return;
    try {
      await ref.read(apiProvider).deleteBackup(backup.path);
      ref.invalidate(backupsProvider);
      ref.invalidate(usageProvider);
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }

  Future<void> _runCheck() async {
    setState(() => _diagnosticsBusy = true);
    try {
      await ref.read(apiProvider).checkIntegrity();
      await ref.read(apiProvider).quickCheck();
      if (mounted) {
        setState(() => _diagnosticsResult = 'SQLite integrity check passed.');
      }
    } on BridgeError catch (error) {
      if (mounted) {
        setState(
          () => _diagnosticsResult =
              'Integrity check failed: ${error.message} (${error.code})',
        );
      }
    } finally {
      if (mounted) setState(() => _diagnosticsBusy = false);
    }
  }

  Future<void> _copyDiagnostics() async {
    try {
      final diagnostics = await ref.read(apiProvider).getDiagnosticsInfo();
      final text = [
        'KONCET diagnostics',
        'app: ${diagnostics.app.appVersion} (${diagnostics.app.buildCommit}, ${diagnostics.app.buildDate})',
        'engine: ${diagnostics.app.engineVersion}',
        'schema: ${diagnostics.app.schemaVersion}',
        'platform: ${diagnostics.app.platform}',
        'integrity: ${diagnostics.integrityOk}',
        'backups: ${diagnostics.backupCount}',
        'projects: ${diagnostics.usage.projectCount}',
        'runs: ${diagnostics.usage.runCount}',
        'events: ${diagnostics.usage.eventCount}',
        'logs: ${diagnostics.usage.logCount}',
        'metrics: ${diagnostics.usage.metricCount}',
      ].join('\n');
      await Clipboard.setData(ClipboardData(text: text));
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(const SnackBar(content: Text('Diagnostics copied')));
      }
    } on BridgeError catch (error) {
      if (mounted) showAppError(context, error);
    }
  }
}

class _BackupRow extends StatelessWidget {
  const _BackupRow({required this.backup, required this.onDelete});

  final BackupInfoDto backup;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final name = backup.path.split('/').last;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        children: [
          Icon(
            Icons.archive_outlined,
            size: 17,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Expanded(child: Text(name, style: context.mono.small)),
          SizedBox(
            width: 90,
            child: Text(
              backup.kind == BackupKindDto.manual ? 'Manual' : 'Migration',
              style: theme.textTheme.bodySmall,
            ),
          ),
          SizedBox(
            width: 148,
            child: Text(
              formatTimestamp(backup.createdAtMs),
              style: theme.textTheme.bodySmall,
            ),
          ),
          SizedBox(
            width: 80,
            child: Text(
              formatBytes(backup.sizeBytes),
              style: theme.textTheme.bodySmall,
              textAlign: TextAlign.right,
            ),
          ),
          IconButton(
            tooltip: 'Delete backup',
            onPressed: onDelete,
            icon: const Icon(Icons.delete_outline, size: 18),
          ),
        ],
      ),
    );
  }
}
