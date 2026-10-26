// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../app.dart';
import '../errors.dart';
import '../key_store.dart';
import '../rust/api.dart' as rust;
import 'backup_io.dart';
import 'dialogs.dart';

enum _Step { choose, backup }

class OnboardingScreen extends StatefulWidget {
  const OnboardingScreen({super.key, required this.app});
  final AppController app;

  @override
  State<OnboardingScreen> createState() => _OnboardingScreenState();
}

class _OnboardingScreenState extends State<OnboardingScreen> {
  _Step _step = _Step.choose;
  rust.KeyInfo? _info;
  String? _note;
  bool _busy = false;
  bool _saved = false;
  bool _confirmed = false;
  String? _error;

  Future<void> _run(Future<void> Function() body) async {
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await body();
    } catch (e) {
      _error = describeError(e);
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _generate() => _run(() async {
    _info = await rust.createKey();
    _note = 'A new random key was created.';
    _step = _Step.backup;
  });

  Future<void> _importSsh() async {
    final file = await pickFileBytes(
      widget.app,
      title: 'Select your id_ed25519 file',
    );
    if (file == null) return;
    await _run(() async {
      try {
        rust.KeyInfo info;
        try {
          info = await rust.importSshKey(file: file, passphrase: null);
        } on rust.PwdxError catch (e) {
          if (e.kind != rust.ErrorKind.passphraseRequired || !mounted) rethrow;
          final pass = await askPassphrase(
            context,
            title: 'SSH key passphrase',
          );
          if (pass == null) return;
          final bytes = Uint8List.fromList(utf8.encode(pass));
          try {
            info = await rust.importSshKey(file: file, passphrase: bytes);
          } finally {
            zero(bytes);
          }
        }
        _info = info;
        _note =
            'Your SSH key was imported. Only its 32-byte seed is used. A dedicated pwdx key is the stronger choice, '
            'because an SSH public key is published.';
        _step = _Step.backup;
      } finally {
        zero(file);
      }
    });
  }

  Future<void> _restore() async {
    final file = await pickFileBytes(
      widget.app,
      title: 'Select your .pwdxkey backup',
    );
    if (file == null || !mounted) return;
    final pass = await askPassphrase(context, title: 'Backup passphrase');
    if (pass == null) return;
    await _run(() async {
      final info = await rust.unlockWithBackup(backup: file, passphrase: pass);
      await _storeAndFinish(info.keyId);
    });
  }

  Future<void> _storeAndFinish(String keyId) async {
    final bytes = await rust.exportKeyBytesForKeystore();
    try {
      await widget.app.keyStore.save(bytes);
    } finally {
      zero(bytes);
    }
    widget.app.completeOnboarding(keyId);
  }

  Future<void> _saveBackup() async {
    final pass = await askPassphrase(
      context,
      title: 'Backup passphrase',
      message: 'This passphrase encrypts your key file. Make it long and different from your memorable password.',
      confirm: true,
      minLength: 12,
    );
    if (pass == null) return;
    await _run(() async {
      final bytes = await rust.exportBackupFile(passphrase: pass);
      final ok = await saveBackupFile(
        widget.app,
        bytes,
        'pwdx-key-${_info!.keyId}.pwdxkey',
      );
      _saved = ok;
      if (!ok) _error = 'The backup was not saved.';
    });
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Set up pwdx')),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(20),
          children: [
            if (_step == _Step.choose) ..._choose(text) else ..._backup(text),
            if (_busy)
              const Padding(
                padding: EdgeInsets.only(top: 24),
                child: Center(child: CircularProgressIndicator()),
              ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
          ],
        ),
      ),
    );
  }

  List<Widget> _choose(TextTheme text) => [
    Text('Your key', style: text.titleLarge),
    const SizedBox(height: 8),
    const Text(
      'pwdx mixes a secret 256-bit key into every password it derives. Use the same key on every device '
      'and you will get the same passwords everywhere. Nothing is ever sent anywhere.',
    ),
    const SizedBox(height: 24),
    FilledButton.icon(
      onPressed: _busy ? null : _generate,
      icon: const Icon(Icons.add),
      label: const Text('Create a new key (recommended)'),
    ),
    const SizedBox(height: 12),
    OutlinedButton.icon(
      onPressed: _busy ? null : _importSsh,
      icon: const Icon(Icons.vpn_key_outlined),
      label: const Text('Import an OpenSSH id_ed25519 key'),
    ),
    const SizedBox(height: 12),
    OutlinedButton.icon(
      onPressed: _busy ? null : _restore,
      icon: const Icon(Icons.restore),
      label: const Text('Restore from a backup file'),
    ),
  ];

  List<Widget> _backup(TextTheme text) => [
    Text('Back up your key', style: text.titleLarge),
    const SizedBox(height: 8),
    if (_note != null) Text(_note!),
    const SizedBox(height: 12),
    Card(
      child: ListTile(
        leading: const Icon(Icons.fingerprint),
        title: Text('Key ID: ${_info?.keyId ?? ''}'),
        subtitle: const Text(
          'Not secret. The same ID on two devices means the same key.',
        ),
      ),
    ),
    const SizedBox(height: 12),
    const Text(
      'If you lose this key and have no backup, every password derived from it is gone for good. '
      'Save the encrypted backup file somewhere safe, away from this device.',
    ),
    const SizedBox(height: 16),
    FilledButton.icon(
      onPressed: _busy ? null : _saveBackup,
      icon: const Icon(Icons.save_alt),
      label: Text(_saved ? 'Save another copy' : 'Save backup file'),
    ),
    const SizedBox(height: 8),
    CheckboxListTile(
      value: _confirmed,
      onChanged: _saved ? (v) => setState(() => _confirmed = v ?? false) : null,
      title: const Text('I saved the backup file in a safe place'),
      controlAffinity: ListTileControlAffinity.leading,
    ),
    const SizedBox(height: 8),
    FilledButton(
      onPressed: (_busy || !_saved || !_confirmed)
          ? null
          : () => _run(() => _storeAndFinish(_info!.keyId)),
      child: const Text('Finish'),
    ),
  ];
}
