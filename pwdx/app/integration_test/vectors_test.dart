// The cross-platform idempotence gate: every known-answer vector must reproduce exactly through the real
// Rust library on the platform this test runs on (Linux, Windows, macOS, Android emulator, iOS simulator).
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:pwdx_app/src/rust/api.dart' as rust;
import 'package:pwdx_app/src/rust/frb_generated.dart';

import 'vectors_data.dart';

Uint8List hex(String s) => Uint8List.fromList([
  for (var i = 0; i < s.length; i += 2)
    int.parse(s.substring(i, i + 2), radix: 16),
]);

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => RustLib.init());

  final data = jsonDecode(vectorsJson) as Map<String, dynamic>;

  testWidgets('self-test passes', (tester) async {
    await rust.selfTest();
    expect(rust.specVersion(), 'v1');
  });

  testWidgets('all known-answer vectors match', (tester) async {
    final vectors = (data['vectors'] as List).cast<Map<String, dynamic>>();
    expect(vectors.length, greaterThanOrEqualTo(30));
    for (final v in vectors) {
      await rust.unlockWithKeyBytes(key: hex(v['key_hex'] as String));
      final got = await rust.derivePassword(
        secret: v['secret'] as String,
        site: v['site'] as String,
        user: v['user'] as String,
        counter: v['counter'] as int,
        profile: rust.ProfileKind.values.byName(v['profile'] as String),
        length: v['length'] as int,
      );
      expect(got, v['expected'], reason: 'vector ${v['id']}');
      expect(
        rust.normalizeSite(site: v['site'] as String),
        v['site_normalized'],
        reason: 'site of ${v['id']}',
      );
    }
    rust.lock();
  });

  testWidgets('invalid inputs are rejected', (tester) async {
    final errors = (data['errors'] as List).cast<Map<String, dynamic>>();
    for (final e in errors) {
      await rust.unlockWithKeyBytes(key: hex(e['key_hex'] as String));
      await expectLater(
        rust.derivePassword(
          secret: e['secret'] as String,
          site: e['site'] as String,
          user: e['user'] as String,
          counter: e['counter'] as int,
          profile: rust.ProfileKind.values.byName(e['profile'] as String),
          length: e['length'] as int,
        ),
        throwsA(isA<rust.PwdxError>()),
        reason: 'case ${e['id']}',
      );
    }
    rust.lock();
  });

  testWidgets('locked session refuses to derive', (tester) async {
    rust.lock();
    await expectLater(
      rust.derivePassword(
        secret: 'a b c d',
        site: 'example.com',
        user: 'u',
        counter: 1,
        profile: rust.ProfileKind.full,
        length: 24,
      ),
      throwsA(
        predicate(
          (e) => e is rust.PwdxError && e.kind == rust.ErrorKind.locked,
        ),
      ),
    );
  });
}
