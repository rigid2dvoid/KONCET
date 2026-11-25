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

    await tester.enterText(find.byType(TextField).at(0), 'UI smoke project');
    await tester.enterText(
      find.byType(TextField).at(1),
      'End-to-end integration fixture',
    );
    await _settle(tester, const Duration(milliseconds: 300));
    await tester.tap(find.text('Create project').last);
    await _waitFor(tester, find.text('Start a simulation'));

    expect(find.text('UI smoke project'), findsWidgets);
    expect(find.text('Happy Path'), findsWidgets);

    await tester.tap(find.text('Open source editor'));
    await _waitFor(tester, find.text('Source editor'));
    expect(find.text('Saved'), findsWidgets);
    await tester.tap(find.byTooltip('Back to project'));
    await _waitFor(tester, find.text('Start a simulation'));

    await tester.tap(find.byTooltip('Open scenario editor').first);
    await _waitFor(tester, find.text('Seed policy'));
    expect(find.text('Canonical JSON'), findsOneWidget);
    await tester.tap(find.byTooltip('Back to project'));
    await _waitFor(tester, find.text('Start a simulation'));

    await tester.tap(find.text('Start run'));
    await _waitFor(tester, find.text('Build pipeline'));

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

    await tester.tap(find.text('Stop'));
    await _waitFor(tester, find.text('Run again'));
    expect(find.text('Completed'), findsWidgets);

    await tester.tap(find.text('Run history').first);
    await _waitFor(tester, find.text('UI smoke project'));

    await tester.tap(find.text('Happy Path').first);
    await _waitFor(tester, find.text('Build pipeline'));
    await tester.tap(find.text('Inspect'));
    await _settle(tester, const Duration(milliseconds: 400));
    await tester.dragUntilVisible(
      find.text('Recorded commands'),
      find.byKey(const ValueKey('run-inspect-scroll')),
      const Offset(0, -300),
    );
    expect(find.text('Recorded commands'), findsOneWidget);
    expect(find.text('Run history'), findsWidgets);

    await tester.tap(find.text('Settings').first);
    await _waitFor(tester, find.text('Appearance'));
    expect(find.text('Local storage'), findsOneWidget);
    await tester.tap(find.text('Projects').first);
    await _waitFor(tester, find.text('Create project'));
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
