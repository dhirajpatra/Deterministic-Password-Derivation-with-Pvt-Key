#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
# One-time setup of the Flutter platform scaffolding around the hand-written code in app/.
#
# STATUS: written but NOT yet run end-to-end (the author's sandbox has no Flutter SDK). Read it before running;
# if a step fails, the manual equivalent is described in app/README.md.
#
# What it does:
#   1. installs flutter_rust_bridge_codegen at the exact version the Rust crate uses
#   2. lets the codegen create a throw-away template app, and copies only the platform scaffolding from it
#      (android/ ios/ linux/ macos/ windows/ rust_builder/): never our lib/, rust/ or pubspec.yaml
#   3. applies the small hardening patches (no backups, biometric permission, FLAG_SECURE, secure clipboard)
#   4. runs flutter pub get and generates the Dart bindings
set -euo pipefail

FRB_VERSION="2.13.0"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/app"
ORG="${PWDX_ORG:-dev.pwdx}"

command -v flutter >/dev/null || { echo "flutter not found on PATH"; exit 1; }
command -v cargo   >/dev/null || { echo "cargo not found on PATH"; exit 1; }

echo "==> installing flutter_rust_bridge_codegen $FRB_VERSION"
cargo install flutter_rust_bridge_codegen --version "$FRB_VERSION" --locked

echo "==> creating template scaffolding"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
( cd "$TMP" && flutter_rust_bridge_codegen create pwdx_app --org "$ORG" )
for d in android ios linux macos windows rust_builder; do
  if [ -d "$TMP/pwdx_app/$d" ] && [ ! -d "$APP/$d" ]; then cp -R "$TMP/pwdx_app/$d" "$APP/$d"; fi
done
[ -f "$APP/.metadata" ] || cp "$TMP/pwdx_app/.metadata" "$APP/.metadata" 2>/dev/null || true
[ -f "$APP/analysis_options.yaml" ] || cp "$TMP/pwdx_app/analysis_options.yaml" "$APP/" 2>/dev/null || true

echo "==> applying platform patches"
python3 - "$APP" <<'PY'
import glob, os, re, sys
app = sys.argv[1]

# Android: no backups (the key must not leave the device through cloud backup), biometric permission, no INTERNET.
manifest = os.path.join(app, "android/app/src/main/AndroidManifest.xml")
if os.path.exists(manifest):
    s = open(manifest, encoding="utf-8").read()
    if "USE_BIOMETRIC" not in s:
        s = re.sub(r"(<manifest[^>]*>)", r'\1\n    <uses-permission android:name="android.permission.USE_BIOMETRIC"/>', s, count=1)
    if "android:allowBackup" not in s:
        s = s.replace("<application", '<application\n        android:allowBackup="false"', 1)
    open(manifest, "w", encoding="utf-8").write(s)
    if "permission.INTERNET" in s:
        print("WARNING: main manifest requests INTERNET; remove it"); 

# Android: MainActivity with FLAG_SECURE + secure clipboard channel
acts = glob.glob(os.path.join(app, "android/app/src/main/kotlin/**/MainActivity.kt"), recursive=True)
if acts:
    act = acts[0]
    old = open(act, encoding="utf-8").read()
    pkg = re.search(r"^package\s+([\w.]+)", old, re.M).group(1)
    new = open(os.path.join(app, "platform_patches/android/MainActivity.kt"), encoding="utf-8").read().replace("PACKAGE_NAME", pkg)
    open(act, "w", encoding="utf-8").write(new)

# iOS: Face ID usage text + secure clipboard channel source
plist = os.path.join(app, "ios/Runner/Info.plist")
if os.path.exists(plist):
    s = open(plist, encoding="utf-8").read()
    if "NSFaceIDUsageDescription" not in s:
        s = s.replace("</dict>\n</plist>", "\t<key>NSFaceIDUsageDescription</key>\n\t<string>Unlock your pwdx key</string>\n</dict>\n</plist>")
        open(plist, "w", encoding="utf-8").write(s)
    dst = os.path.join(app, "ios/Runner/PwdxSecureChannel.swift")
    if not os.path.exists(dst):
        open(dst, "w", encoding="utf-8").write(open(os.path.join(app, "platform_patches/ios/PwdxSecureChannel.swift"), encoding="utf-8").read())
        print("NOTE: add ios/Runner/PwdxSecureChannel.swift to the Runner target in Xcode and call PwdxSecureChannel.register(...) (see the file header)")
PY

echo "==> generating Dart bindings"
( cd "$APP" && flutter pub get && flutter_rust_bridge_codegen generate )

echo "==> done. Next: cd app && flutter run -d <linux|windows|macos|android|ios>"
