// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
import 'package:flutter/material.dart';

import '../app.dart';
import '../errors.dart';
import '../rust/api.dart' as rust;
import 'backup_io.dart';
import 'dialogs.dart';

class KeyScreen extends StatefulWidget {
  const KeyScreen({super.key, required this.app});
  final AppController app;

  @override
  State<KeyScreen> createState() => _KeyScreenState();
}

class _KeyScreenState extends State<KeyScreen> {
  bool _busy = false;

  Future<void> _exportBackup() async {
    final pass = await askPassphrase(
      context,
      title: 'Backup passphrase',
      message: 'Choose a long passphrase for the backup file. It is separate from your memorable password.',
      confirm: true,
      minLength: 12,
    );
    if (pass == null || !mounted) return;
    setState(() => _busy = true);
    try {
      final bytes = await rust.exportBackupFile(passphrase: pass);
      final ok = await saveBackupFile(
        widget.app,
        bytes,
        'pwdx-key-${widget.app.keyId}.pwdxkey',
      );
      if (mounted)
        showMessage(
          context,
          ok ? 'Backup saved.' : 'The backup was not saved.',
        );
    } catch (e) {
      if (mounted) showMessage(context, describeError(e));
    }
    if (mounted) setState(() => _busy = false);
  }

  Future<void> _selfTest() async {
    try {
      await rust.selfTest();
      if (mounted)
        showMessage(context, 'Self-test passed (spec ${rust.specVersion()}).');
    } catch (_) {
      if (mounted)
        showMessage(context, 'Self-test FAILED. Do not use this build.');
    }
  }

  Future<void> _delete() async {
    final ok = await confirmDialog(
      context,
      title: 'Remove key from this device?',
      message: 'Only do this if you have a backup file. Without the key, none of your derived passwords can be recreated.',
      action: 'Remove',
    );
    if (!ok) return;
    await widget.app.deleteKeyFromDevice();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Key')),
      body: ListView(
        children: [
          ListTile(
            leading: const Icon(Icons.fingerprint),
            title: Text('Key ID: ${widget.app.keyId ?? ''}'),
            subtitle: const Text(
              'Not secret. Check that every device shows the same ID.',
            ),
          ),
          ListTile(
            leading: const Icon(Icons.save_alt),
            title: const Text('Export encrypted backup'),
            subtitle: const Text(
              'Also how you move the key to another device.',
            ),
            enabled: !_busy,
            onTap: _exportBackup,
          ),
          ListTile(
            leading: const Icon(Icons.verified_outlined),
            title: const Text('Run self-test'),
            subtitle: Text('Spec ${rust.specVersion()}'),
            onTap: _selfTest,
          ),
          ListTile(
            leading: Icon(
              Icons.delete_outline,
              color: Theme.of(context).colorScheme.error,
            ),
            title: const Text('Remove key from this device'),
            onTap: _delete,
          ),
          const Padding(
            padding: EdgeInsets.all(16),
            child: Text(
              'Good to know: changing your memorable password or your key changes every derived password, so you would have '
              'to update every site. A thief who gets both your key file and one of your derived passwords can try to guess your '
              'memorable password offline, so make it a long passphrase (4+ random words).\n\n'
              'pwdx is free software released under the MIT License and comes with no warranty.',
            ),
          ),
        ],
      ),
    );
  }
}
