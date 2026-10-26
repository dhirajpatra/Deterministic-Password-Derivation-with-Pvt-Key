#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
# Checks a PUBLISHED repository the way a new user would use it, inside a clean Ubuntu container (needs Docker).
#   bash packaging/deb/verify-published.sh https://your-host/path [package] [ubuntu-tags...]
# Default tags: 22.04 24.04. Add more (for example debian:12 via IMAGES="ubuntu:22.04 debian:12").
set -euo pipefail
url="${1:?usage: $0 <repo-url> [package]}"; url="${url%/}"
pkg="${2:-pwdx-cli}"
bin="${BIN_NAME:-$pkg}"
images="${IMAGES:-ubuntu:22.04 ubuntu:24.04}"
command -v docker >/dev/null || { echo "docker not found; run the commands from the repo landing page inside any clean Debian/Ubuntu VM instead"; exit 2; }
for img in $images; do
  echo "==> $img"
  docker run --rm "$img" bash -euc "
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq && apt-get install -y -qq curl ca-certificates gnupg >/dev/null
    install -d -m 0755 /usr/share/keyrings
    curl -fsSL '$url/pwdx-archive-keyring.gpg' -o /usr/share/keyrings/pwdx-archive-keyring.gpg
    gpg --show-keys --fingerprint /usr/share/keyrings/pwdx-archive-keyring.gpg | sed -n '2p'
    echo 'deb [signed-by=/usr/share/keyrings/pwdx-archive-keyring.gpg] $url ./' > /etc/apt/sources.list.d/pwdx.list
    apt-get update -qq
    apt-get install -y -qq $pkg >/dev/null
    $bin selftest
    $bin help | head -1
  "
done
echo "all good: every image installed $pkg and passed its self-test"
