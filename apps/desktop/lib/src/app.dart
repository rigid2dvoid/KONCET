import 'dart:async';
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';
import 'package:nativeapi/nativeapi.dart' show Rectangle;
import 'package:window_manager/window_manager.dart';

import 'router.dart';
import 'state/app_settings.dart';
import 'state/providers.dart';

class KoncetApp extends ConsumerWidget {
  const KoncetApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    final reduceMotion = ref.watch(appSettingsProvider).value?.reduceMotion;
    return MaterialApp.router(
      title: 'KONCET',
      debugShowCheckedModeBanner: false,
      theme: koncetTheme(Brightness.light),
      darkTheme: koncetTheme(Brightness.dark),
      themeMode: themeMode,
      routerConfig: ref.watch(routerProvider),
      builder: (context, child) {
        if (reduceMotion != true) return child ?? const SizedBox.shrink();
        final media = MediaQuery.of(context);
        return MediaQuery(
          data: media.copyWith(disableAnimations: true),
          child: child ?? const SizedBox.shrink(),
        );
      },
    );
  }
}

/// Intercepts window close so an active run is stopped and SQLite is
/// checkpointed before the process exits.
class WindowLifecycle extends ConsumerStatefulWidget {
  const WindowLifecycle({super.key, required this.child});

  final Widget child;

  @override
  ConsumerState<WindowLifecycle> createState() => _WindowLifecycleState();
}

class _WindowLifecycleState extends ConsumerState<WindowLifecycle> {
  Window? _window;
  ListenerId? _listener;
  bool _closing = false;
  bool _readyToClose = false;
  bool _geometryRestored = false;

  @override
  void initState() {
    super.initState();
    _window = WindowManager.instance.getCurrent();
    _listener = _window?.addListener(_onWindowEvent);
  }

  @override
  void dispose() {
    final listener = _listener;
    if (listener != null) _window?.removeListener(listener);
    super.dispose();
  }

  Future<void> _onWindowEvent(WindowEvent event) async {
    if (event is! WindowCloseRequestedEvent) return;
    if (_readyToClose) return;
    event.request.cancel();
    if (_closing) return;
    await _handleClose();
  }

  Future<void> _handleClose() async {
    if (_closing) return;
    final active = ref.read(activeRunProvider);
    if (active != null && !active.finalised) {
      final context = rootNavigatorKey.currentContext;
      if (context != null) {
        final proceed = await showKoncetConfirm(
          context,
          title: 'Stop the active run and quit?',
          message: 'The run will be finalised as completed with reason user_stopped. Its logs, metrics, and events are flushed before KONCET exits.',
          confirmLabel: 'Stop run and quit',
        );
        if (!proceed) return;
      }
    }
    _closing = true;
    try {
      await _saveGeometry();
      await ref.read(apiProvider).shutdownApp();
    } catch (_) {
      // The process is closing; the database lock is released on exit.
    }
    _readyToClose = true;
    _window?.close();
  }

  Future<void> _saveGeometry() async {
    try {
      final window = _window;
      if (window == null) return;
      final bounds = window.bounds;
      await ref
          .read(apiProvider)
          .setSettingJson(
            'window_geometry',
            jsonEncode({
              'x': bounds.x.round(),
              'y': bounds.y.round(),
              'width': bounds.width.round(),
              'height': bounds.height.round(),
            }),
          );
    } catch (_) {
      // Geometry is best-effort and never blocks shutdown.
    }
  }

  Future<void> _restoreGeometry() async {
    if (_geometryRestored) return;
    _geometryRestored = true;
    try {
      final json = await ref
          .read(apiProvider)
          .getSettingJson('window_geometry');
      if (json == null) return;
      final value = jsonDecode(json);
      if (value is! Map<String, dynamic>) return;
      final width = value['width'];
      final height = value['height'];
      final x = value['x'];
      final y = value['y'];
      if (width is! int || height is! int || width < 1024 || height < 720) {
        return;
      }
      final window = _window;
      if (window == null) return;
      window.bounds = Rectangle(
        x: (x is int ? x : 80).toDouble(),
        y: (y is int ? y : 80).toDouble(),
        width: width.toDouble(),
        height: height.toDouble(),
      );
    } catch (_) {
      // A stale geometry value is ignored; the default window remains.
    }
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(bootstrapProvider, (previous, next) {
      if (next.hasValue) {
        unawaited(_restoreGeometry());
      }
    });
    return widget.child;
  }
}
