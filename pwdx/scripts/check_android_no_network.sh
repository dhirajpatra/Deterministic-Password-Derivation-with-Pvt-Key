#!/usr/bin/env bash
# Fails if a release APK declares the INTERNET permission (the app must be fully offline).
# Usage: scripts/check_android_no_network.sh path/to/app-release.apk
set -euo pipefail
apk="${1:?usage: $0 app-release.apk}"
aapt="$(ls "${ANDROID_HOME:-$ANDROID_SDK_ROOT}"/build-tools/*/aapt2 2>/dev/null | sort -V | tail -1)"
[ -n "$aapt" ] || { echo "aapt2 not found (set ANDROID_HOME)"; exit 2; }
perms="$("$aapt" dump permissions "$apk")"
echo "$perms"
if echo "$perms" | grep -q "android.permission.INTERNET"; then
  echo "FAIL: release APK requests INTERNET" >&2
  exit 1
fi
echo "OK: no INTERNET permission"
