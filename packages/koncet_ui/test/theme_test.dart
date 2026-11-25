import 'package:flutter_test/flutter_test.dart';
import 'package:koncet_ui/koncet_ui.dart';
import 'package:material_ui/material_ui.dart';

void main() {
  testWidgets('theme exposes fixed Material 3 tokens', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: koncetTheme(Brightness.light),
        home: const Scaffold(
          body: Column(
            children: [
              KoncetSimulatedTag(),
              KoncetStatusBadge(
                label: 'Healthy',
                tone: KoncetStatusTone.healthy,
              ),
              KoncetSimClock(elapsedMs: 62500),
            ],
          ),
        ),
      ),
    );

    final context = tester.element(find.byType(Scaffold));
    final theme = Theme.of(context);
    expect(theme.useMaterial3, isTrue);
    expect(theme.colorScheme.primary, isNotNull);
    expect(theme.extension<KoncetStatusColors>(), isNotNull);
    expect(theme.extension<KoncetMonoText>(), isNotNull);
    expect(theme.textTheme.bodyMedium?.fontFamily, koncetFontFamily);
    expect(
      theme.extension<KoncetMonoText>()!.medium.fontFamily,
      koncetMonoFontFamily,
    );
    expect(find.text('SIMULATED'), findsOneWidget);
    expect(find.text('Healthy'), findsOneWidget);
    expect(find.text('01:02.5'), findsOneWidget);
  });

  test('monospace clock formatting handles hours and tenths', () {
    expect(KoncetSimClock.formatDuration(0), '00:00.0');
    expect(KoncetSimClock.formatDuration(62345), '01:02.3');
    expect(KoncetSimClock.formatDuration(3_723_400), '01:02:03.4');
  });
}
