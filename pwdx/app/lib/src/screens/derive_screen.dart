// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
import 'dart:async';

import 'package:flutter/material.dart';

import '../app.dart';
import '../errors.dart';
import '../rust/api.dart' as rust;
import '../secure_clipboard.dart';
import '../site_store.dart';
import 'dialogs.dart';
import 'key_screen.dart';

class DeriveScreen extends StatefulWidget {
  const DeriveScreen({super.key, required this.app});
  final AppController app;

  @override
  State<DeriveScreen> createState() => _DeriveScreenState();
}

class _DeriveScreenState extends State<DeriveScreen> {
  final _secret = TextEditingController();
  final _site = TextEditingController();
  final _user = TextEditingController();
  final _store = SiteStore();

  List<SavedSite> _saved = [];
  int _counter = 1;
  rust.ProfileKind _profile = rust.ProfileKind.full;
  late int _length;

  String? _result;
  bool _reveal = false;
  bool _busy = false;
  String? _error;
  Timer? _hideTimer;

  static const _profileLabels = {
    rust.ProfileKind.full: 'Letters, digits and symbols',
    rust.ProfileKind.alnum: 'Letters and digits only',
    rust.ProfileKind.passphrase: 'Words (passphrase)',
    rust.ProfileKind.pin: 'PIN (weak)',
  };

  @override
  void initState() {
    super.initState();
    _length = rust.profileInfo(profile: _profile).defaultLength;
    _reload();
  }

  @override
  void dispose() {
    _secret.dispose();
    _site.dispose();
    _user.dispose();
    _hideTimer?.cancel();
    super.dispose();
  }

  Future<void> _reload() async {
    final list = await _store.load();
    if (mounted) setState(() => _saved = list);
  }

  String? get _keyedOn {
    final s = _site.text.trim();
    if (s.isEmpty) return null;
    try {
      return rust.normalizeSite(site: s);
    } catch (_) {
      return null;
    }
  }

  Future<void> _derive() async {
    if (_secret.text.isEmpty ||
        _site.text.trim().isEmpty ||
        _user.text.trim().isEmpty) {
      setState(
        () => _error =
            'Enter your memorable password, the site and your username.',
      );
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
      _result = null;
    });
    try {
      final pw = await rust.derivePassword(
        secret: _secret.text,
        site: _site.text,
        user: _user.text,
        counter: _counter,
        profile: _profile,
        length: _length,
      );
      if (!mounted) return;
      _secret.clear(); // keep the memorable password in memory as briefly as possible
      _hideTimer?.cancel();
      _hideTimer = Timer(const Duration(seconds: 30), () {
        if (mounted) setState(() => _result = null);
      });
      setState(() {
        _result = pw;
        _reveal = false;
      });
    } catch (e) {
      if (mounted) setState(() => _error = describeError(e));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _copy() async {
    final r = _result;
    if (r == null) return;
    await SecureClipboard.copy(r);
    if (mounted)
      showMessage(context, 'Copied. The clipboard is cleared in 30 seconds.');
  }

  Future<void> _saveSite() async {
    if (_site.text.trim().isEmpty || _user.text.trim().isEmpty) return;
    await _store.upsert(
      SavedSite(
        site: _site.text.trim(),
        user: _user.text.trim(),
        counter: _counter,
        profile: _profile,
        length: _length,
      ),
    );
    await _reload();
    if (mounted) showMessage(context, 'Site saved (no password is stored).');
  }

  SavedSite? get _matchingSaved {
    for (final s in _saved) {
      if (s.site == _site.text.trim() && s.user == _user.text.trim()) return s;
    }
    return null;
  }

  Future<void> _forget() async {
    final m = _matchingSaved;
    if (m == null) return;
    await _store.remove(m);
    await _reload();
  }

  void _apply(SavedSite s) {
    setState(() {
      _site.text = s.site;
      _user.text = s.user;
      _counter = s.counter;
      _profile = s.profile;
      _length = s.length;
      _result = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final info = rust.profileInfo(profile: _profile);
    final unit = _profile == rust.ProfileKind.passphrase
        ? 'words'
        : 'characters';
    final keyed = _keyedOn;
    return Scaffold(
      appBar: AppBar(
        title: const Text('pwdx'),
        actions: [
          IconButton(
            tooltip: 'Key and backup',
            icon: const Icon(Icons.vpn_key_outlined),
            onPressed: () => Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) => KeyScreen(app: widget.app),
              ),
            ),
          ),
          IconButton(
            tooltip: 'Lock',
            icon: const Icon(Icons.lock_outline),
            onPressed: widget.app.lockNow,
          ),
        ],
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            if (_saved.isNotEmpty) ...[
              DropdownButtonFormField<SavedSite>(
                decoration: const InputDecoration(labelText: 'Saved sites'),
                items: [
                  for (final s in _saved)
                    DropdownMenuItem(
                      value: s,
                      child: Text(s.label, overflow: TextOverflow.ellipsis),
                    ),
                ],
                onChanged: (s) {
                  if (s != null) _apply(s);
                },
              ),
              const SizedBox(height: 12),
            ],
            TextField(
              controller: _secret,
              obscureText: true,
              autocorrect: false,
              enableSuggestions: false,
              enableIMEPersonalizedLearning: false,
              keyboardType: TextInputType.visiblePassword,
              decoration: const InputDecoration(
                labelText: 'Memorable password',
                helperText: 'Never stored. Cleared after each use.',
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _site,
              autocorrect: false,
              enableSuggestions: false,
              keyboardType: TextInputType.url,
              onChanged: (_) => setState(() {}),
              decoration: InputDecoration(
                labelText: 'Site',
                hintText: 'github.com',
                helperText: keyed == null ? null : 'Keyed on: $keyed',
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _user,
              autocorrect: false,
              enableSuggestions: false,
              keyboardType: TextInputType.emailAddress,
              onChanged: (_) => setState(() {}),
              decoration: const InputDecoration(labelText: 'Username or email'),
            ),
            const SizedBox(height: 8),
            ExpansionTile(
              title: const Text('Options'),
              tilePadding: EdgeInsets.zero,
              children: [
                Row(
                  children: [
                    const Expanded(
                      child: Text(
                        'Counter (increase to get a new password for this site)',
                      ),
                    ),
                    IconButton(
                      onPressed: _counter > 1
                          ? () => setState(() => _counter--)
                          : null,
                      icon: const Icon(Icons.remove),
                    ),
                    Text('$_counter'),
                    IconButton(
                      onPressed: () => setState(() => _counter++),
                      icon: const Icon(Icons.add),
                    ),
                  ],
                ),
                DropdownButtonFormField<rust.ProfileKind>(
                  key: ValueKey(_profile),
                  initialValue: _profile,
                  decoration: const InputDecoration(labelText: 'Password type'),
                  items: [
                    for (final p in rust.ProfileKind.values)
                      DropdownMenuItem(
                        value: p,
                        child: Text(_profileLabels[p]!),
                      ),
                  ],
                  onChanged: (p) {
                    if (p == null) return;
                    setState(() {
                      _profile = p;
                      _length = rust.profileInfo(profile: p).defaultLength;
                    });
                  },
                ),
                if (_profile == rust.ProfileKind.pin)
                  Padding(
                    padding: const EdgeInsets.only(top: 8),
                    child: Text(
                      'A PIN has very little strength. Use it only where nothing else is allowed.',
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ),
                Row(
                  children: [
                    Text('Length: $_length $unit'),
                    Expanded(
                      child: Slider(
                        min: info.minLength.toDouble(),
                        max: info.maxLength.toDouble(),
                        divisions: info.maxLength - info.minLength,
                        value: _length
                            .clamp(info.minLength, info.maxLength)
                            .toDouble(),
                        label: '$_length',
                        onChanged: (v) => setState(() => _length = v.round()),
                      ),
                    ),
                  ],
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Wrap(
                    spacing: 8,
                    children: [
                      TextButton.icon(
                        onPressed: _saveSite,
                        icon: const Icon(Icons.bookmark_add_outlined),
                        label: const Text(
                          'Remember these settings for this site',
                        ),
                      ),
                      if (_matchingSaved != null)
                        TextButton(
                          onPressed: _forget,
                          child: const Text('Forget this site'),
                        ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            FilledButton.icon(
              onPressed: _busy ? null : _derive,
              icon: _busy
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.key),
              label: Text(_busy ? 'Deriving…' : 'Derive password'),
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            if (_result != null)
              Card(
                margin: const EdgeInsets.only(top: 16),
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _reveal
                            ? _result!
                            : '•' *
                                  (_result!.length > 24 ? 24 : _result!.length),
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          fontSize: 16,
                        ),
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: [
                          FilledButton.tonalIcon(
                            onPressed: _copy,
                            icon: const Icon(Icons.copy),
                            label: const Text('Copy'),
                          ),
                          const SizedBox(width: 8),
                          OutlinedButton.icon(
                            onPressed: () => setState(() => _reveal = !_reveal),
                            icon: Icon(
                              _reveal ? Icons.visibility_off : Icons.visibility,
                            ),
                            label: Text(_reveal ? 'Hide' : 'Show'),
                          ),
                        ],
                      ),
                      const SizedBox(height: 4),
                      const Text(
                        'This result disappears after 30 seconds.',
                        style: TextStyle(fontSize: 12),
                      ),
                    ],
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
