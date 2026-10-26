// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
import 'rust/api.dart' as rust;

/// Turns any error into a short, non-secret message for the user.
String describeError(Object e) {
  if (e is rust.PwdxError) {
    switch (e.kind) {
      case rust.ErrorKind.decrypt:
        return 'Wrong passphrase, or the file is damaged.';
      case rust.ErrorKind.passphraseRequired:
        return 'This key is protected by a passphrase.';
      case rust.ErrorKind.locked:
        return 'The key is locked. Unlock it first.';
      case rust.ErrorKind.kdf:
        return 'Not enough memory to run the key stretching. Close other apps and try again.';
      case rust.ErrorKind.invalidInput:
      case rust.ErrorKind.format:
      case rust.ErrorKind.internal:
        return e.message;
    }
  }
  return 'Something went wrong.';
}
