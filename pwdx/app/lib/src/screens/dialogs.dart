// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
import 'package:flutter/material.dart';

/// Asks for a passphrase. Returns null if cancelled. With [confirm] the user must type it twice.
Future<String?> askPassphrase(
  BuildContext context, {
  required String title,
  String? message,
  bool confirm = false,
  int minLength = 0,
}) async {
  final a = TextEditingController();
  final b = TextEditingController();
  String? error;
  final result = await showDialog<String>(
    context: context,
    barrierDismissible: false,
    builder: (ctx) => StatefulBuilder(
      builder: (ctx, setState) => AlertDialog(
        title: Text(title),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (message != null)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: Text(message),
                ),
              TextField(
                controller: a,
                obscureText: true,
                autofocus: true,
                autocorrect: false,
                enableSuggestions: false,
                enableIMEPersonalizedLearning: false,
                decoration: const InputDecoration(labelText: 'Passphrase'),
              ),
              if (confirm)
                TextField(
                  controller: b,
                  obscureText: true,
                  autocorrect: false,
                  enableSuggestions: false,
                  enableIMEPersonalizedLearning: false,
                  decoration: const InputDecoration(
                    labelText: 'Repeat passphrase',
                  ),
                ),
              if (error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    error!,
                    style: TextStyle(color: Theme.of(ctx).colorScheme.error),
                  ),
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              if (a.text.isEmpty) {
                setState(() => error = 'Enter a passphrase.');
              } else if (a.text.runes.length < minLength) {
                setState(() => error = 'Use at least $minLength characters.');
              } else if (confirm && a.text != b.text) {
                setState(() => error = 'The passphrases do not match.');
              } else {
                Navigator.pop(ctx, a.text);
              }
            },
            child: const Text('OK'),
          ),
        ],
      ),
    ),
  );
  a.dispose();
  b.dispose();
  return result;
}

void showMessage(BuildContext context, String text) {
  ScaffoldMessenger.of(context)
    ..hideCurrentSnackBar()
    ..showSnackBar(SnackBar(content: Text(text)));
}

Future<bool> confirmDialog(
  BuildContext context, {
  required String title,
  required String message,
  String action = 'Continue',
}) async {
  final ok = await showDialog<bool>(
    context: context,
    builder: (ctx) => AlertDialog(
      title: Text(title),
      content: Text(message),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(ctx, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(ctx, true),
          child: Text(action),
        ),
      ],
    ),
  );
  return ok ?? false;
}
