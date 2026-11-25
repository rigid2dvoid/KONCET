import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

import 'rust/error.dart';
import 'screens/startup.dart';
import 'state/providers.dart';

class AppShell extends ConsumerWidget {
  const AppShell({super.key, required this.location, required this.child});

  final String location;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final bootstrap = ref.watch(bootstrapProvider);
    return Scaffold(
      appBar: bootstrap.hasValue ? _buildTopBar(context, ref) : null,
      body: bootstrap.when(
        data: (_) => _buildWorkspace(context, ref),
        loading: () => const SplashScreen(),
        error: (error, stack) => RecoveryScreen(error: error),
      ),
    );
  }

  int get _selectedIndex {
    if (location.startsWith('/runs')) return 1;
    if (location.startsWith('/settings')) return 2;
    return 0;
  }

  PreferredSizeWidget _buildTopBar(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    return AppBar(
      toolbarHeight: 56,
      titleSpacing: 20,
      title: LayoutBuilder(
        builder: (context, constraints) => Row(
          children: [
            Text(
              'KONCET',
              style: theme.textTheme.titleLarge?.copyWith(
                letterSpacing: 0.08 * 20,
                fontWeight: FontWeight.w600,
              ),
            ),
            if (constraints.maxWidth > 420) ...[
              const SizedBox(width: 14),
              Flexible(
                child: Text(
                  'Deployment simulation workbench',
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        const KoncetSimulatedTag(compact: true),
        const SizedBox(width: 8),
        const _ActiveRunIndicator(),
        const SizedBox(width: 16),
      ],
      bottom: PreferredSize(
        preferredSize: const Size.fromHeight(1),
        child: Divider(height: 1, color: theme.colorScheme.outlineVariant),
      ),
    );
  }

  Widget _buildWorkspace(BuildContext context, WidgetRef ref) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final expanded = constraints.maxWidth >= 1280;
        return Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            NavigationRail(
              extended: expanded,
              minExtendedWidth: 232,
              selectedIndex: _selectedIndex,
              labelType: expanded
                  ? NavigationRailLabelType.none
                  : NavigationRailLabelType.all,
              onDestinationSelected: (index) {
                switch (index) {
                  case 0:
                    context.go('/projects');
                  case 1:
                    context.go('/runs');
                  case 2:
                    context.go('/settings');
                }
              },
              destinations: const [
                NavigationRailDestination(
                  icon: Icon(Icons.folder_outlined),
                  selectedIcon: Icon(Icons.folder),
                  label: Text('Projects'),
                ),
                NavigationRailDestination(
                  icon: Icon(Icons.history_outlined),
                  selectedIcon: Icon(Icons.history),
                  label: Text('Run history'),
                ),
                NavigationRailDestination(
                  icon: Icon(Icons.settings_outlined),
                  selectedIcon: Icon(Icons.settings),
                  label: Text('Settings'),
                ),
              ],
            ),
            VerticalDivider(
              width: 1,
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
            Expanded(
              child: Align(
                alignment: Alignment.topCenter,
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 1440),
                  child: Padding(
                    padding: EdgeInsets.all(
                      constraints.maxWidth >= 1440 ? 32 : 24,
                    ),
                    child: child,
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

class _ActiveRunIndicator extends ConsumerWidget {
  const _ActiveRunIndicator();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final active = ref.watch(activeRunProvider);
    if (active == null) return const SizedBox.shrink();
    final theme = Theme.of(context);
    final tone = active.finalised
        ? KoncetStatusTone.stopped
        : (active.frame?.paused == true
              ? KoncetStatusTone.warning
              : KoncetStatusTone.active);
    final label = active.finalised
        ? 'Run finished'
        : active.frame?.paused == true
        ? 'Paused'
        : 'Run active';
    return Tooltip(
      message: 'Open the active simulation run',
      child: InkWell(
        borderRadius: BorderRadius.circular(999),
        onTap: () => context.go('/runs/${active.handle.runId}'),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          decoration: BoxDecoration(
            color: context.statusColors.container(tone),
            borderRadius: BorderRadius.circular(999),
            border: Border.all(
              color: context.statusColors
                  .foreground(tone)
                  .withValues(alpha: 0.35),
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 7,
                height: 7,
                decoration: BoxDecoration(
                  color: context.statusColors.foreground(tone),
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 7),
              Text(
                active.finalised
                    ? '${active.handle.scenarioName} finished'
                    : active.frame == null
                    ? label
                    : '${active.handle.scenarioName}  ${KoncetSimClock.formatDuration(active.frame!.clockMs)}',
                style: theme.textTheme.labelMedium?.copyWith(
                  color: context.statusColors.foreground(tone),
                ),
              ),
              const SizedBox(width: 4),
              Icon(
                Icons.chevron_right,
                size: 15,
                color: context.statusColors.foreground(tone),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Snackbar helper used by bridge error handlers across screens.
void showAppError(BuildContext context, Object error) {
  final message = switch (error) {
    BridgeError() => '${error.message} (${error.code})',
    StateError() => error.message,
    _ => error.toString(),
  };
  ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(content: Text(message), duration: const Duration(seconds: 6)),
  );
}
