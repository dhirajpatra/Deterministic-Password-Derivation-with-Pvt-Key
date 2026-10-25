import 'dart:async';

import 'package:flutter/services.dart';

/// Copies a password so that it is hidden from clipboard previews / kept off synced clipboards where the platform
/// allows it (Android and iOS via native code in platform_patches/), and clears it after a delay.
///
/// Limitations: the timer cannot fire while the app is suspended in the background, and on desktop the plain
/// clipboard is used (clipboard managers can still record it).
class SecureClipboard {
  static const _channel = MethodChannel('pwdx/secure');
  static Timer? _timer;

  static Future<void> copy(
    String text, {
    Duration clearAfter = const Duration(seconds: 30),
  }) async {
    try {
      await _channel.invokeMethod<void>('copySensitive', {'text': text});
    } on MissingPluginException {
      await Clipboard.setData(ClipboardData(text: text));
    }
    _timer?.cancel();
    _timer = Timer(clearAfter, clear);
  }

  /// Clears the clipboard unconditionally (it may also remove something else the user copied meanwhile).
  static Future<void> clear() async {
    _timer?.cancel();
    try {
      await _channel.invokeMethod<void>('clearClipboard');
    } on MissingPluginException {
      await Clipboard.setData(const ClipboardData(text: ''));
    }
  }
}
