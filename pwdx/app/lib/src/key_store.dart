import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:local_auth/local_auth.dart';

/// Holds the 32-byte key in the platform keystore (Keychain, Android Keystore-backed storage, Credential
/// Manager / libsecret). The key never leaves the keystore except to be handed to the Rust session.
class KeyStore {
  static const _name = 'pwdx.key.v1';

  final FlutterSecureStorage _storage = const FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    iOptions: IOSOptions(
      accessibility: KeychainAccessibility.unlocked_this_device,
    ),
  );

  Future<bool> hasKey() => _storage.containsKey(key: _name);

  Future<void> save(Uint8List key) =>
      _storage.write(key: _name, value: base64Encode(key));

  /// Returns the key bytes; the caller must zero them after use.
  Future<Uint8List?> read() async {
    final v = await _storage.read(key: _name);
    return v == null ? null : base64Decode(v);
  }

  Future<void> delete() => _storage.delete(key: _name);
}

/// Device-credential gate (biometrics or screen-lock PIN) in front of the keystore.
/// Where the platform cannot authenticate (for example Linux), the gate is open and the keystore alone protects the key.
class DeviceGate {
  final LocalAuthentication _auth = LocalAuthentication();

  Future<bool> authenticate(String reason) async {
    try {
      if (!await _auth.isDeviceSupported()) return true;
      return await _auth.authenticate(
        localizedReason: reason,
        options: const AuthenticationOptions(stickyAuth: true),
      );
    } on MissingPluginException {
      return true;
    } on PlatformException {
      return false;
    }
  }
}

void zero(Uint8List bytes) => bytes.fillRange(0, bytes.length, 0);
