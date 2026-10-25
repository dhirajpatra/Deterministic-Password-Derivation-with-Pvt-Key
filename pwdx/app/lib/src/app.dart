import 'dart:async';

import 'package:flutter/material.dart';

import 'errors.dart';
import 'key_store.dart';
import 'rust/api.dart' as rust;
import 'screens/derive_screen.dart';
import 'screens/onboarding_screen.dart';
import 'screens/unlock_screen.dart';

enum AppPhase { booting, fatal, needsKey, locked, unlocked }

/// Owns the lock state. The key itself lives only in the Rust session; this class decides when it is unlocked.
class AppController extends ChangeNotifier with WidgetsBindingObserver {
  AppController() {
    WidgetsBinding.instance.addObserver(this);
  }

  static const idleLimit = Duration(minutes: 2);

  final KeyStore keyStore = KeyStore();
  final DeviceGate gate = DeviceGate();

  AppPhase phase = AppPhase.booting;
  String? fatalMessage;
  String? keyId;
  bool shielded = false;

  bool _suspendAutoLock = false;
  Timer? _idle;

  Future<void> init() async {
    try {
      await rust.selfTest();
    } catch (_) {
      fatalMessage = 'The built-in self-test failed. This build cannot be trusted, so password derivation is disabled.';
      phase = AppPhase.fatal;
      notifyListeners();
      return;
    }
    phase = await keyStore.hasKey() ? AppPhase.locked : AppPhase.needsKey;
    notifyListeners();
  }

  /// Runs [action] (a system dialog, file picker or biometric prompt) without auto-locking while the app is paused.
  Future<T> runExternal<T>(Future<T> Function() action) async {
    _suspendAutoLock = true;
    try {
      return await action();
    } finally {
      _suspendAutoLock = false;
    }
  }

  /// Returns null on success, otherwise a message for the user.
  Future<String?> unlock() async {
    try {
      final ok = await runExternal(
        () => gate.authenticate('Unlock your pwdx key'),
      );
      if (!ok) return 'Authentication was cancelled or failed.';
      final bytes = await keyStore.read();
      if (bytes == null) {
        phase = AppPhase.needsKey;
        notifyListeners();
        return 'There is no key on this device.';
      }
      try {
        final info = await rust.unlockWithKeyBytes(key: bytes);
        keyId = info.keyId;
      } finally {
        zero(bytes);
      }
      phase = AppPhase.unlocked;
      _resetIdle();
      notifyListeners();
      return null;
    } catch (e) {
      return describeError(e);
    }
  }

  void lockNow() {
    if (phase != AppPhase.unlocked) return;
    rust.lock();
    _idle?.cancel();
    phase = AppPhase.locked;
    notifyListeners();
  }

  void completeOnboarding(String id) {
    keyId = id;
    phase = AppPhase.unlocked;
    _resetIdle();
    notifyListeners();
  }

  Future<void> deleteKeyFromDevice() async {
    rust.lock();
    _idle?.cancel();
    await keyStore.delete();
    keyId = null;
    phase = AppPhase.needsKey;
    notifyListeners();
  }

  /// Call on user activity to postpone the idle lock.
  void touch() {
    if (phase == AppPhase.unlocked) _resetIdle();
  }

  void _resetIdle() {
    _idle?.cancel();
    _idle = Timer(idleLimit, lockNow);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final cover = state != AppLifecycleState.resumed;
    if (cover != shielded) {
      shielded = cover;
      notifyListeners();
    }
    final leaving =
        state == AppLifecycleState.paused ||
        state == AppLifecycleState.hidden ||
        state == AppLifecycleState.detached;
    if (leaving && !_suspendAutoLock) lockNow();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _idle?.cancel();
    super.dispose();
  }
}

class PwdxApp extends StatefulWidget {
  const PwdxApp({super.key});

  @override
  State<PwdxApp> createState() => _PwdxAppState();
}

class _PwdxAppState extends State<PwdxApp> {
  final AppController app = AppController();
  final GlobalKey<NavigatorState> navKey = GlobalKey<NavigatorState>();

  @override
  void initState() {
    super.initState();
    app.addListener(_onChange);
    app.init();
  }

  void _onChange() {
    // When the key locks, drop any screens pushed on top of the home screen.
    if (app.phase != AppPhase.unlocked) {
      navKey.currentState?.popUntil((r) => r.isFirst);
    }
  }

  @override
  void dispose() {
    app.removeListener(_onChange);
    app.dispose();
    super.dispose();
  }

  Widget _home() {
    switch (app.phase) {
      case AppPhase.booting:
        return const Scaffold(body: Center(child: CircularProgressIndicator()));
      case AppPhase.fatal:
        return Scaffold(
          body: Center(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Text(
                app.fatalMessage ?? 'Fatal error',
                textAlign: TextAlign.center,
              ),
            ),
          ),
        );
      case AppPhase.needsKey:
        return OnboardingScreen(app: app);
      case AppPhase.locked:
        return UnlockScreen(app: app);
      case AppPhase.unlocked:
        return DeriveScreen(app: app);
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: app,
      builder: (context, _) => MaterialApp(
        title: 'pwdx',
        navigatorKey: navKey,
        theme: ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true),
        darkTheme: ThemeData(
          colorSchemeSeed: Colors.indigo,
          useMaterial3: true,
          brightness: Brightness.dark,
        ),
        builder: (context, child) => Listener(
          behavior: HitTestBehavior.translucent,
          onPointerDown: (_) => app.touch(),
          child: Stack(
            children: [
              child ?? const SizedBox.shrink(),
              if (app.shielded) const Positioned.fill(child: _PrivacyShield()),
            ],
          ),
        ),
        home: _home(),
      ),
    );
  }
}

/// Opaque cover shown whenever the app is not in the foreground, so app-switcher snapshots show nothing.
class _PrivacyShield extends StatelessWidget {
  const _PrivacyShield();

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: const Center(child: Icon(Icons.lock_outline, size: 64)),
    );
  }
}
