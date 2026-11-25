import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:material_ui/material_ui.dart';

import 'package:koncet/src/app.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:koncet/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('create project, run, inspect history', (tester) async {
    tester.view.physicalSize = const Size(1440, 900);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);
    await RustLib.init();
    runApp(const ProviderScope(child: WindowLifecycle(child: KoncetApp())));
    await _settle(tester, const Duration(seconds: 5));

    await _waitFor(tester, find.text('Projects'));
    await _waitFor(tester, find.text('Create your first project'));

    await tester.tap(find.text('Create project').first);
    await _settle(tester, const Duration(milliseconds: 400));
    expect(find.text('Create project'), findsWidgets);

    await _waitFor(tester, find.text('HTTP service'));
    await tester.tap(find.text('HTTP service'));
    await _settle(tester, const Duration(milliseconds: 300));
    await tester.enterText(find.byType(TextField).at(0), 'Payments API');
    await tester.enterText(
      find.byType(TextField).at(1),
      'HTTP service deployment fixture',
    );
    await _settle(tester, const Duration(milliseconds: 300));
    await tester.tap(find.text('Create project').last);
    await _waitFor(tester, find.text('Start a run'));

    expect(find.text('Payments API'), findsWidgets);
    expect(find.text('Happy Path'), findsWidgets);
    await _capture(tester, '02-project-overview');

    await tester.tap(find.text('Open source editor'));
    await _waitFor(tester, find.text('Source editor'));
    expect(find.text('Saved'), findsWidgets);
    await tester.tap(find.byTooltip('Back to project'));
    await _waitFor(tester, find.text('Start a run'));

    await tester.tap(find.byTooltip('Open scenario editor').first);
    await _waitFor(tester, find.text('Seed policy'));
    expect(find.text('Canonical JSON'), findsOneWidget);
    await _capture(tester, '03-scenario-editor');
    await tester.tap(find.byTooltip('Back to project'));
    await _waitFor(tester, find.text('Start a run'));

    await tester.tap(find.text('Start run'));
    await _waitFor(tester, find.text('Release conveyor'));

    await tester.tap(find.byTooltip('Simulation speed'));
    await _settle(tester, const Duration(milliseconds: 300));
    await tester.tap(find.text('8×').last);
    await _settle(tester, const Duration(seconds: 1));
    final liveClock = tester.widget<KoncetSimClock>(
      find.byType(KoncetSimClock),
    );
    expect(liveClock.elapsedMs, greaterThan(0));

    await _waitFor(
      tester,
      find.text('Healthy'),
      timeout: const Duration(seconds: 15),
    );
    await _settle(tester, const Duration(milliseconds: 500));
    await _capture(tester, '04-live-server');

    await tester.tap(find.text('Logs'));
    await _settle(tester, const Duration(milliseconds: 400));
    await _capture(tester, '05-logs');

    await tester.tap(find.text('Metrics'));
    await _settle(tester, const Duration(milliseconds: 400));
    await _capture(tester, '06-metrics');

    await tester.tap(find.text('Stop'));
    await _waitFor(tester, find.text('Run again'));
    expect(find.text('Completed'), findsWidgets);

    await tester.tap(find.text('Run history').first);
    await _waitFor(tester, find.text('Payments API'));
    await _capture(tester, '07-run-history');

    await tester.tap(find.text('Happy Path').first);
    await _waitFor(tester, find.text('Release conveyor'));
    await tester.tap(find.text('Details'));
    await _settle(tester, const Duration(milliseconds: 400));
    await tester.dragUntilVisible(
      find.text('Recorded commands'),
      find.byKey(const ValueKey('run-details-scroll')),
      const Offset(0, -300),
    );
    expect(find.text('Recorded commands'), findsOneWidget);
    expect(find.text('Run history'), findsWidgets);
    await _capture(tester, '08-run-details');

    await tester.tap(find.text('Settings').first);
    await _waitFor(tester, find.text('Appearance'));
    expect(find.text('Local storage'), findsOneWidget);
    await _capture(tester, '09-settings');
    await tester.tap(find.text('Projects').first);
    await _waitFor(tester, find.text('Create project'));
    await _capture(tester, '01-projects');
  });
}

Future<void> _settle(WidgetTester tester, Duration duration) async {
  await tester.pump();
  await tester.runAsync(() => Future<void>.delayed(duration));
  await tester.pump(const Duration(milliseconds: 100));
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
  final texts = tester
      .widgetList<Text>(find.byType(Text))
      .map((text) => text.data ?? text.textSpan?.toPlainText())
      .whereType<String>()
      .toList();
  throw TestFailure('Timed out waiting for $finder. Visible texts: $texts');
}

Future<void> _capture(WidgetTester tester, String name) async {
  if (Platform.environment['KONCET_CAPTURE_SCREENSHOTS'] != '1') return;
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
