import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:material_ui/material_ui.dart';
import 'package:window_manager/window_manager.dart';

import 'src/app.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  _registerBundledLicenses();
  await RustLib.init();

  final window = WindowManager.instance.getCurrent();
  if (window != null) {
    window.title = 'KONCET';
    window.minimumSize = const Size(1024, 720).toNative();
    window.setSize(const Size(1280, 820).toNative(), false);
    window.center();
    window.show();
    window.focus();
  }

  runApp(const ProviderScope(child: WindowLifecycle(child: KoncetApp())));
}

void _registerBundledLicenses() {
  LicenseRegistry.addLicense(() async* {
    const entries = [
      ('KONCET', 'assets/licenses/GPL-3.0-or-later.txt'),
      ('Inter', 'assets/licenses/OFL-Inter.txt'),
      ('JetBrains Mono', 'assets/licenses/OFL-JetBrainsMono.txt'),
    ];
    for (final (name, asset) in entries) {
      final text = await rootBundle.loadString(asset);
      yield LicenseEntryWithLineBreaks([name], text);
    }
    final notices = await rootBundle.loadString(
      'assets/licenses/THIRD_PARTY_NOTICES.md',
    );
    yield LicenseEntryWithLineBreaks(['KONCET third-party notices'], notices);
  });
}
