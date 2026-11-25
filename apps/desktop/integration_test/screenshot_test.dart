import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:koncet/src/app.dart';
import 'package:koncet/src/rust/frb_generated.dart';

/// Captures the README showcase from a pre-seeded demo database.
///
/// Run with:
///   XDG_DATA_HOME=/tmp/koncet-screenshots \
///   KONCET_CAPTURE_SCREENSHOTS=1 \
///   flutter test integration_test/screenshot_test.dart -d linux
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('capture showcase screenshots', (tester) async {
    if (Platform.environment['KONCET_CAPTURE_SCREENSHOTS'] != '1') {
      return;
    }
    tester.view.physicalSize = const Size(1440, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await RustLib.init();
    runApp(const ProviderScope(child: WindowLifecycle(child: KoncetApp())));
    await _waitFor(tester, find.text('Payments API'));
    await tester.tap(find.text('Payments API').first);
    await _waitFor(tester, find.text('Start a run'));
    await _capture(tester, '02-project-overview');

    await tester.tap(find.text('Start run'));
    await _waitFor(tester, find.text('Release conveyor'));
    await _waitFor(
      tester,
      find.text('Healthy'),
      timeout: const Duration(seconds: 15),
    );
    await _settle(tester, const Duration(milliseconds: 500));
    await _capture(tester, '04-live-server');

    await tester.drag(
      find.byKey(const ValueKey('server-board-scroll')),
      const Offset(0, -360),
    );
    await _settle(tester, const Duration(milliseconds: 400));
    await _capture(tester, '04b-traffic-path');

    await tester.tap(find.text('Logs'));
    await _settle(tester, const Duration(milliseconds: 400));
    await _capture(tester, '05-logs');

    await tester.tap(find.text('Metrics'));
    await _settle(tester, const Duration(milliseconds: 400));
    await _capture(tester, '06-metrics');

    await tester.tap(find.text('Stop'));
    await _waitFor(tester, find.text('Run again'));

    await tester.tap(find.text('Run history').first);
    await _waitFor(tester, find.text('Payments API'));
    await _capture(tester, '07-run-history');
  });
}

Future<void> _waitFor(
  WidgetTester tester,
  Finder finder, {
  Duration timeout = const Duration(seconds: 12),
}) async {
  final deadline = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(deadline)) {
    await tester.pump(const Duration(milliseconds: 100));
    if (finder.evaluate().isNotEmpty) return;
    await tester.runAsync(
      () => Future<void>.delayed(const Duration(milliseconds: 150)),
    );
  }
  throw TestFailure('Timed out waiting for $finder');
}

Future<void> _settle(WidgetTester tester, Duration duration) async {
  await tester.pump();
  await tester.runAsync(() => Future<void>.delayed(duration));
  await tester.pump(const Duration(milliseconds: 100));
}

Future<void> _capture(WidgetTester tester, String name) async {
  await tester.pump();
  final view = RendererBinding.instance.renderViews.first;
  // ignore: invalid_use_of_protected_member
  final layer = view.layer! as OffsetLayer;
  final image = await layer.toImage(view.paintBounds);
  final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
  image.dispose();
  if (bytes == null) return;
  final directory =
      Platform.environment['KONCET_SCREENSHOT_DIR'] ??
      '../../docs/assets/screenshots';
  final file = File('$directory/$name.png');
  file.parent.createSync(recursive: true);
  file.writeAsBytesSync(bytes.buffer.asUint8List());
}
