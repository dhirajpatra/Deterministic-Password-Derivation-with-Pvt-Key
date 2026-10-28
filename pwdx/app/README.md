# pwdx app (Flutter + Rust)

Hand-written Flutter UI around the Rust bridge crate in `rust/` (package `rust_lib_pwdx_app`, a thin wrapper over
`pwdx-core`). **Status: the Rust bridge is built and tested (all 30 vectors pass through its API); the Dart code is
syntax-checked only. It has not been compiled or run, because the authoring environment had no Flutter SDK.**
Expect a first round of small fixes (plugin API drift, platform scaffolding) when you build it.

## Build

```
bash scripts/bootstrap_app.sh        # once: platform scaffolding, patches, Dart bindings (read it first)
cd app && flutter run -d linux       # or windows, macos, android, ios
flutter test integration_test/vectors_test.dart -d linux    # the cross-platform idempotence gate
```

Manual equivalent of the bootstrap script: install `flutter_rust_bridge_codegen` at exactly the version in
`rust/Cargo.toml` (2.13.0), run `flutter_rust_bridge_codegen create` in a scratch folder, copy `android/ ios/ linux/
macos/ windows/ rust_builder/` into `app/` (keep our `lib/`, `rust/`, `pubspec.yaml`), apply `platform_patches/`,
then `flutter pub get && flutter_rust_bridge_codegen generate`.

## Build without installing Flutter (GitHub Actions)

Push the whole project to a GitHub repository (the app needs the folders next to `app/`). Then open **Actions > app-release >
Run workflow**. When it finishes, download from the run page, under *Artifacts*: `pwdx-app-windows-x64` (zip),
`pwdx-app-android` (APK, signed with Flutter's debug key, fine for installing on your own phone) and `pwdx-app-linux-x64`
(tar.gz). Each comes with a `.sha256` file. The workflow is unverified: if a job fails, send the error text from its log.
Windows needs the Microsoft Visual C++ Redistributable installed (most PCs already have it).

## How it works

- The 32-byte key lives in the Rust session (`Arc<KeySeed>`, zeroized on drop). Dart never holds a key object.
- The OS keystore (Keychain / Android Keystore-backed storage / Credential Manager / libsecret) stores the key at rest,
  behind biometrics or the screen-lock PIN where available (`local_auth`).
- On first run you create a key (or import `id_ed25519`, or restore a backup) and **must** save an encrypted backup file.
- The memorable password is cleared from its text field after each derivation; the result hides itself after 30 s.
- The app locks when it goes to the background and after 2 minutes idle; an opaque cover hides app-switcher snapshots.
- At start-up `selfTest()` runs one known-answer vector; if it fails, derivation is disabled.

## Known limitations (be honest about these)

- **Dart strings cannot be zeroized.** The typed memorable password and the returned password live in the Dart heap
  until garbage-collected, and the key bytes pass through Dart briefly when read from the keystore. For maximum
  assurance on desktop, use the CLI (`pwdx get`), which reads and wipes secrets in Rust.
- Android: `FLAG_SECURE` and a sensitive-flagged clipboard come from `platform_patches/android/MainActivity.kt`.
  iOS: the local-only, expiring clipboard needs `PwdxSecureChannel.swift` wired into the app delegate (manual step).
  Windows/Linux/macOS: plain clipboard (clipboard managers may record it) and no screenshot blocking yet.
- The clipboard timer cannot run while the app is suspended in the background.
- Saved-site list is local only; encrypted export of it (decision D6) and QR transfer of the backup are not built yet.
- Plugin versions in `pubspec.yaml` are the ones the author knew; run `flutter pub upgrade` and fix API drift if needed.
