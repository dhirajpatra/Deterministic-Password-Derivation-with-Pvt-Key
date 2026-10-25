---
name: pwdx-flutter-app
description: Guide for the pwdx Flutter UI on Linux, Windows, Android and iOS - screens, flutter_rust_bridge integration, OS keystore and biometrics, clipboard handling, screen-capture blocking, keyboard privacy, no-network configuration, and the Dart memory limitation. Use whenever building or reviewing any UI, platform channel, permission, manifest, entitlement or clipboard code for pwdx. Make sure to use it before adding any Flutter plugin.
---

# pwdx Flutter app

The UI is a thin shell. It collects inputs, calls the Rust core, shows or copies the result. It never implements any part of the algorithm and never normalizes text itself.

## 1. Screens

1. **First run:** choose key source (generate dedicated key / import `id_ed25519` / restore backup). Force creating a backup before continuing; show the Key ID.
2. **Derive:** fields for memorable password (obscured), site, username; counter, profile and length under "Advanced". Result is hidden by default; "Copy" and "Show" buttons.
3. **Sites:** optional local, non-secret list (site, user, counter, profile, length). Stored locally; export is encrypted with the backup passphrase.
4. **Key:** Key ID, backup/export, QR transfer, rotate-all checklist, delete key.
5. **About:** spec version, build hash, `self_test` button.

At startup call `self_test()` (one embedded known-answer vector). If it fails, block derivation and show a clear error. It catches broken builds and toolchain bugs.

## 2. Rust integration

- `flutter_rust_bridge` v2 over `crates/ffi`; async calls so the ~1 s Argon2 run does not block the UI thread.
- Expose only the API from `pwdx-rust-core`. Pass secrets as `Uint8List`/`String` and clear references right after the call.
- Show a progress indicator during derivation. Do not fall back to cheaper parameters on slow devices.

## 3. Key storage and unlock

Follow the table in `pwdx-key-management`. Packages: `flutter_secure_storage` (set iOS accessibility to `unlocked_this_device`, disable iCloud sync) and `local_auth` for biometrics. Android: require user authentication on the keystore key and prefer StrongBox. Auto-lock after a short idle time and whenever the app goes to the background; require unlock again.

## 4. Input hardening for the memorable password

- `obscureText: true`, `enableSuggestions: false`, `autocorrect: false`, `enableIMEPersonalizedLearning: false`, no autofill hints.
- Do not put the password in route arguments, state restoration, or text controllers that outlive the screen; dispose and clear controllers after each derivation.
- **Known limitation:** Dart `String`s are immutable and cannot be zeroized, so the typed password can linger in the Dart heap until garbage collected. Minimize its lifetime and say this honestly in `pwdx-security-review`. For maximum assurance on desktop, recommend the CLI, which reads the password in Rust and zeroizes it.

## 5. Clipboard

- Auto-clear after about 30 seconds (best effort; on iOS and recent Android you cannot always read the clipboard to confirm it is still ours).
- Android 13+: mark clip as sensitive (`ClipDescription.EXTRA_IS_SENSITIVE`) so previews are hidden.
- iOS: use `UIPasteboard` with `localOnly` and an `expirationDate`.
- Windows: exclude from clipboard history and cloud clipboard (`ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory = 0`, `CanUploadToCloudClipboard = 0`).
- Never log or toast the password value.

## 6. Screen and app-switcher privacy

- Android: `FLAG_SECURE` on the window (blocks screenshots and recents thumbnails).
- iOS: cover the UI with a blank view in `applicationWillResignActive`.
- Desktop: clear sensitive fields on focus loss or lock.

## 7. No network, anywhere

- Android: do **not** declare `INTERNET` in the main/release manifest. Flutter adds it to the *debug* manifest only; verify the merged release manifest in CI.
- iOS: no networking entitlements or ATS exceptions needed; add no analytics or crash SDKs.
- Linux Flatpak: no `--share=network` in `finish-args`; minimal filesystem access (portal file chooser for key import).
- Windows: ship no update checker in v1; tell users to verify downloads by checksum.
- No Firebase, no Sentry, no telemetry, no remote fonts or images.

## 8. Plugins policy

Every Flutter plugin is code you must trust with secrets. Allow only: `flutter_rust_bridge`, `flutter_secure_storage`, `local_auth`, a QR generate/scan pair (scanner needs camera permission only, on demand), and a file picker. Anything else needs a written justification in the PR.

## 9. Accessibility and UX safety

- Offer "Show password" with a timed re-hide; warn on `pin` profile weakness.
- Explain, in plain words, that changing the memorable password or the key changes every site's password.
- Never claim "unhackable" in UI text.
