import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';

import '../rust/dto.dart';
import 'providers.dart';

enum ThemePreference { system, light, dark }

class AppSettings {
  const AppSettings({
    this.theme = ThemePreference.system,
    this.defaultSpeed = 1,
    this.reduceMotion = false,
  });

  final ThemePreference theme;
  final int defaultSpeed;
  final bool reduceMotion;

  AppSettings copyWith({
    ThemePreference? theme,
    int? defaultSpeed,
    bool? reduceMotion,
  }) => AppSettings(
    theme: theme ?? this.theme,
    defaultSpeed: defaultSpeed ?? this.defaultSpeed,
    reduceMotion: reduceMotion ?? this.reduceMotion,
  );
}

class AppSettingsController extends AsyncNotifier<AppSettings> {
  @override
  Future<AppSettings> build() async {
    final entries = await ref.read(apiProvider).listSettings();
    return _parse(entries);
  }

  AppSettings _parse(List<SettingEntryDto> entries) {
    final values = {for (final entry in entries) entry.key: entry.valueJson};
    var settings = const AppSettings();

    final themeText = _string(values['theme_mode']);
    settings = settings.copyWith(
      theme: switch (themeText) {
        'light' => ThemePreference.light,
        'dark' => ThemePreference.dark,
        _ => ThemePreference.system,
      },
    );

    final speedValue = _int(values['default_speed']);
    if (speedValue != null && const [1, 2, 4, 8].contains(speedValue)) {
      settings = settings.copyWith(defaultSpeed: speedValue);
    }

    settings = settings.copyWith(
      reduceMotion: _bool(values['reduce_motion']) ?? false,
    );
    return settings;
  }

  Future<void> setTheme(ThemePreference preference) async {
    await _write('theme_mode', jsonEncode(preference.name));
    final current = state.value ?? const AppSettings();
    state = AsyncData(current.copyWith(theme: preference));
  }

  Future<void> setDefaultSpeed(int speed) async {
    await _write('default_speed', jsonEncode(speed));
    final current = state.value ?? const AppSettings();
    state = AsyncData(current.copyWith(defaultSpeed: speed));
  }

  Future<void> setReduceMotion(bool reduceMotion) async {
    await _write('reduce_motion', jsonEncode(reduceMotion));
    final current = state.value ?? const AppSettings();
    state = AsyncData(current.copyWith(reduceMotion: reduceMotion));
  }

  Future<void> _write(String key, String valueJson) async {
    await ref.read(apiProvider).setSettingJson(key, valueJson);
    ref.invalidate(settingsEntriesProvider);
  }

  String? _string(String? json) {
    if (json == null) return null;
    try {
      final value = jsonDecode(json);
      return value is String ? value : null;
    } on FormatException {
      return null;
    }
  }

  int? _int(String? json) {
    if (json == null) return null;
    try {
      final value = jsonDecode(json);
      return value is int ? value : null;
    } on FormatException {
      return null;
    }
  }

  bool? _bool(String? json) {
    if (json == null) return null;
    try {
      final value = jsonDecode(json);
      return value is bool ? value : null;
    } on FormatException {
      return null;
    }
  }
}

final appSettingsProvider =
    AsyncNotifierProvider<AppSettingsController, AppSettings>(
      AppSettingsController.new,
    );

final themeModeProvider = Provider<ThemeMode>((ref) {
  final preference = ref.watch(appSettingsProvider).value?.theme;
  return switch (preference) {
    ThemePreference.light => ThemeMode.light,
    ThemePreference.dark => ThemeMode.dark,
    _ => ThemeMode.system,
  };
});
