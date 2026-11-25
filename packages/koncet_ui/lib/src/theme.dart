import 'package:material_ui/material_ui.dart';

/// Bundled font families. Flutter namespaces package fonts with the package name.
const koncetFontFamily = 'packages/koncet_ui/Inter';
const koncetMonoFontFamily = 'packages/koncet_ui/JetBrains Mono';

/// Semantic status tones. Colour is never the only carrier of meaning.
enum KoncetStatusTone {
  healthy,
  active,
  warning,
  danger,
  stopped,
  info,
  neutral,
}

/// Fixed, accessible status colours for one brightness.
@immutable
class KoncetStatusColors extends ThemeExtension<KoncetStatusColors> {
  const KoncetStatusColors({
    required this.healthy,
    required this.active,
    required this.warning,
    required this.danger,
    required this.stopped,
    required this.info,
    required this.neutral,
    required this.healthyContainer,
    required this.activeContainer,
    required this.warningContainer,
    required this.dangerContainer,
    required this.stoppedContainer,
    required this.infoContainer,
  });

  final Color healthy;
  final Color active;
  final Color warning;
  final Color danger;
  final Color stopped;
  final Color info;
  final Color neutral;
  final Color healthyContainer;
  final Color activeContainer;
  final Color warningContainer;
  final Color dangerContainer;
  final Color stoppedContainer;
  final Color infoContainer;

  Color foreground(KoncetStatusTone tone) => switch (tone) {
    KoncetStatusTone.healthy => healthy,
    KoncetStatusTone.active => active,
    KoncetStatusTone.warning => warning,
    KoncetStatusTone.danger => danger,
    KoncetStatusTone.stopped => stopped,
    KoncetStatusTone.info => info,
    KoncetStatusTone.neutral => neutral,
  };

  Color container(KoncetStatusTone tone) => switch (tone) {
    KoncetStatusTone.healthy => healthyContainer,
    KoncetStatusTone.active => activeContainer,
    KoncetStatusTone.warning => warningContainer,
    KoncetStatusTone.danger => dangerContainer,
    KoncetStatusTone.stopped => stoppedContainer,
    KoncetStatusTone.info => infoContainer,
    KoncetStatusTone.neutral => stoppedContainer,
  };

  @override
  KoncetStatusColors copyWith({
    Color? healthy,
    Color? active,
    Color? warning,
    Color? danger,
    Color? stopped,
    Color? info,
    Color? neutral,
    Color? healthyContainer,
    Color? activeContainer,
    Color? warningContainer,
    Color? dangerContainer,
    Color? stoppedContainer,
    Color? infoContainer,
  }) {
    return KoncetStatusColors(
      healthy: healthy ?? this.healthy,
      active: active ?? this.active,
      warning: warning ?? this.warning,
      danger: danger ?? this.danger,
      stopped: stopped ?? this.stopped,
      info: info ?? this.info,
      neutral: neutral ?? this.neutral,
      healthyContainer: healthyContainer ?? this.healthyContainer,
      activeContainer: activeContainer ?? this.activeContainer,
      warningContainer: warningContainer ?? this.warningContainer,
      dangerContainer: dangerContainer ?? this.dangerContainer,
      stoppedContainer: stoppedContainer ?? this.stoppedContainer,
      infoContainer: infoContainer ?? this.infoContainer,
    );
  }

  @override
  KoncetStatusColors lerp(ThemeExtension<KoncetStatusColors>? other, double t) {
    if (other is! KoncetStatusColors) return this;
    Color mix(Color a, Color b) => Color.lerp(a, b, t) ?? a;
    return KoncetStatusColors(
      healthy: mix(healthy, other.healthy),
      active: mix(active, other.active),
      warning: mix(warning, other.warning),
      danger: mix(danger, other.danger),
      stopped: mix(stopped, other.stopped),
      info: mix(info, other.info),
      neutral: mix(neutral, other.neutral),
      healthyContainer: mix(healthyContainer, other.healthyContainer),
      activeContainer: mix(activeContainer, other.activeContainer),
      warningContainer: mix(warningContainer, other.warningContainer),
      dangerContainer: mix(dangerContainer, other.dangerContainer),
      stoppedContainer: mix(stoppedContainer, other.stoppedContainer),
      infoContainer: mix(infoContainer, other.infoContainer),
    );
  }
}

const _lightStatus = KoncetStatusColors(
  healthy: Color(0xFF1B7F4B),
  active: Color(0xFF00696E),
  warning: Color(0xFF8A5A00),
  danger: Color(0xFFB3261E),
  stopped: Color(0xFF5C6462),
  info: Color(0xFF0061A4),
  neutral: Color(0xFF5C6462),
  healthyContainer: Color(0xFFDDF3E5),
  activeContainer: Color(0xFFD6EFED),
  warningContainer: Color(0xFFFFEFC7),
  dangerContainer: Color(0xFFFFE1DE),
  stoppedContainer: Color(0xFFE6ECEA),
  infoContainer: Color(0xFFDCEEFF),
);

const _darkStatus = KoncetStatusColors(
  healthy: Color(0xFF6FD79A),
  active: Color(0xFF80D5D0),
  warning: Color(0xFFF2C14E),
  danger: Color(0xFFFFB4AB),
  stopped: Color(0xFFB8C4C1),
  info: Color(0xFF9ECAFF),
  neutral: Color(0xFFB8C4C1),
  healthyContainer: Color(0xFF153D2A),
  activeContainer: Color(0xFF103B3A),
  warningContainer: Color(0xFF3D2F0B),
  dangerContainer: Color(0xFF4A1512),
  stoppedContainer: Color(0xFF2A3230),
  infoContainer: Color(0xFF10304D),
);

/// Monospace text styles used by clocks, logs, identifiers, and seeds.
@immutable
class KoncetMonoText extends ThemeExtension<KoncetMonoText> {
  const KoncetMonoText({required this.small, required this.medium});

  final TextStyle small;
  final TextStyle medium;

  @override
  KoncetMonoText copyWith({TextStyle? small, TextStyle? medium}) =>
      KoncetMonoText(small: small ?? this.small, medium: medium ?? this.medium);

  @override
  KoncetMonoText lerp(ThemeExtension<KoncetMonoText>? other, double t) {
    if (other is! KoncetMonoText) return this;
    return KoncetMonoText(
      small: TextStyle.lerp(small, other.small, t) ?? small,
      medium: TextStyle.lerp(medium, other.medium, t) ?? medium,
    );
  }
}

/// Builds the fixed KONCET Material 3 theme.
ThemeData koncetTheme(Brightness brightness) {
  final scheme = ColorScheme.fromSeed(
    seedColor: const Color(0xFF00696E),
    brightness: brightness,
    surface: brightness == Brightness.light
        ? const Color(0xFFF4FAF9)
        : const Color(0xFF0D1514),
  );
  final base = brightness == Brightness.light
      ? Typography.material2021(platform: TargetPlatform.linux).black
      : Typography.material2021(platform: TargetPlatform.linux).white;
  final text = base
      .apply(fontFamily: koncetFontFamily, bodyColor: scheme.onSurface)
      .copyWith(
        displaySmall: base.displaySmall?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w600,
          fontSize: 32,
          height: 40 / 32,
          color: scheme.onSurface,
        ),
        headlineSmall: base.headlineSmall?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w600,
          fontSize: 24,
          height: 32 / 24,
          color: scheme.onSurface,
        ),
        titleLarge: base.titleLarge?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w600,
          fontSize: 20,
          height: 28 / 20,
          color: scheme.onSurface,
        ),
        titleMedium: base.titleMedium?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w500,
          fontSize: 16,
          height: 24 / 16,
          color: scheme.onSurface,
        ),
        titleSmall: base.titleSmall?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w500,
          fontSize: 14,
          height: 20 / 14,
          color: scheme.onSurface,
        ),
        bodyLarge: base.bodyLarge?.copyWith(
          fontFamily: koncetFontFamily,
          fontSize: 16,
          height: 24 / 16,
          color: scheme.onSurface,
        ),
        bodyMedium: base.bodyMedium?.copyWith(
          fontFamily: koncetFontFamily,
          fontSize: 14,
          height: 20 / 14,
          color: scheme.onSurface,
        ),
        bodySmall: base.bodySmall?.copyWith(
          fontFamily: koncetFontFamily,
          fontSize: 12,
          height: 16 / 12,
          color: scheme.onSurfaceVariant,
        ),
        labelLarge: base.labelLarge?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w500,
          fontSize: 14,
          height: 20 / 14,
          color: scheme.onSurface,
        ),
        labelMedium: base.labelMedium?.copyWith(
          fontFamily: koncetFontFamily,
          fontWeight: FontWeight.w500,
          fontSize: 12,
          height: 16 / 12,
          color: scheme.onSurfaceVariant,
        ),
      );

  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    colorScheme: scheme,
    scaffoldBackgroundColor: scheme.surface,
    visualDensity: VisualDensity.compact,
    textTheme: text,
    extensions: <ThemeExtension<dynamic>>[
      brightness == Brightness.light ? _lightStatus : _darkStatus,
      KoncetMonoText(
        small: TextStyle(
          fontFamily: koncetMonoFontFamily,
          fontSize: 12,
          height: 18 / 12,
          color: scheme.onSurface,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
        medium: TextStyle(
          fontFamily: koncetMonoFontFamily,
          fontSize: 13,
          height: 20 / 13,
          fontWeight: FontWeight.w500,
          color: scheme.onSurface,
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
      ),
    ],
    cardTheme: CardThemeData(
      margin: EdgeInsets.zero,
      elevation: 0,
      color: scheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: scheme.outlineVariant),
      ),
    ),
    dividerTheme: DividerThemeData(
      color: scheme.outlineVariant,
      thickness: 1,
      space: 1,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: scheme.surfaceContainerLowest,
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: scheme.outlineVariant),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: scheme.outlineVariant),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(8),
        borderSide: BorderSide(color: scheme.primary, width: 2),
      ),
    ),
    navigationRailTheme: NavigationRailThemeData(
      backgroundColor: scheme.surfaceContainerLow,
      indicatorColor: scheme.secondaryContainer,
      selectedIconTheme: IconThemeData(color: scheme.onSecondaryContainer),
      selectedLabelTextStyle: text.labelMedium?.copyWith(
        color: scheme.onSurface,
        fontWeight: FontWeight.w600,
      ),
      unselectedLabelTextStyle: text.labelMedium,
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      backgroundColor: scheme.inverseSurface,
      contentTextStyle: text.bodyMedium?.copyWith(
        color: scheme.onInverseSurface,
      ),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: scheme.surfaceContainerHigh,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
  );
}

extension KoncetThemeAccess on BuildContext {
  KoncetStatusColors get statusColors =>
      Theme.of(this).extension<KoncetStatusColors>()!;

  KoncetMonoText get mono => Theme.of(this).extension<KoncetMonoText>()!;
}
