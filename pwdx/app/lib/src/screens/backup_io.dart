import 'dart:io';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';

import '../app.dart';

/// Lets the user pick a file and returns its bytes (null if cancelled). The caller should zero the bytes if they are sensitive.
Future<Uint8List?> pickFileBytes(AppController app, {String? title}) async {
  final result = await app.runExternal(
    () => FilePicker.platform.pickFiles(dialogTitle: title, withData: true),
  );
  if (result == null || result.files.isEmpty) return null;
  return result.files.single.bytes;
}

/// Saves an (encrypted) key backup through the system file dialog. Returns true if the user saved it.
Future<bool> saveBackupFile(
  AppController app,
  Uint8List bytes,
  String fileName,
) async {
  final path = await app.runExternal(
    () => FilePicker.platform.saveFile(
      dialogTitle: 'Save your key backup',
      fileName: fileName,
      bytes: bytes,
    ),
  );
  if (path == null) return false;
  // On desktop the dialog only returns a path; write the bytes ourselves.
  if (Platform.isLinux || Platform.isWindows || Platform.isMacOS) {
    await File(path).writeAsBytes(bytes, flush: true);
  }
  return true;
}
