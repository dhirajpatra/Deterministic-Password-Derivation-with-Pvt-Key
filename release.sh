#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
# One command to build, sign and stage a release of the apt repository.
#
#   MAINTAINER="Your Name dhiraj.patra@gmail.com" GPG_KEY=3D93F64918CBBD3B580A4418D52F43A7C1D85283 \
#   REPO_URL=https://dhirajpatra.github.io/pwdx-apt  bash packaging/deb/release.sh
#
# Optional:
#   REPO_DIR=apt-repo      where the repository is staged (older .deb files kept there stay installable)
#   SKIP_TESTS=1           skip `cargo test`
#   UPLOAD_CMD='rsync -av apt-repo/ user@host:/var/www/pwdx/'     run after staging (never use --delete)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

: "${MAINTAINER:?set MAINTAINER=\"Your Name dhiraj.patra@gmail.com\"}"
: "${GPG_KEY:?set GPG_KEY=3D93F64918CBBD3B580A4418D52F43A7C1D85283 (gpg --list-keys --with-colons | awk -F: '/^fpr/ {print \$10; exit}')}"
: "${REPO_URL:?set REPO_URL=https://dhirajpatra.github.io/pwdx-apt (where the repo folder will be served)}"
REPO_DIR="${REPO_DIR:-apt-repo}"
PKG="${PKG_NAME:-pwdx-cli}"
REPO_URL="${REPO_URL%/}"

case "$REPO_URL" in https://*) ;; *) echo "REPO_URL must be https:// (apt repositories should not use plain http)"; exit 1 ;; esac
gpg --batch --list-secret-keys "$GPG_KEY" >/dev/null 2>&1 || { echo "secret key $GPG_KEY not found in your GPG keyring"; exit 1; }

if [ "${SKIP_TESTS:-0}" != 1 ]; then
  echo "==> running tests"
  cargo test --release --workspace --locked
fi

echo "==> building the package"
MAINTAINER="$MAINTAINER" bash packaging/deb/build-deb.sh
deb="$(ls -t dist/${PKG}_*.deb | head -1)"

echo "==> building the signed repository in $REPO_DIR"
bash packaging/deb/make-apt-repo.sh "$REPO_DIR" "$GPG_KEY" "$deb"

fpr="$(gpg --batch --with-colons --fingerprint "$GPG_KEY" | awk -F: '/^fpr/ {print $10; exit}')"
version="$(dpkg-deb --field "$deb" Version)"

# A small landing page so people who open the address in a browser see how to install.
cat > "$REPO_DIR/index.html" <<HTML
<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>pwdx apt repository</title>
<style>body{font:16px/1.5 system-ui,sans-serif;max-width:46rem;margin:2rem auto;padding:0 1rem}pre{background:#f4f4f4;padding:1rem;overflow:auto;border-radius:6px}code{font-family:ui-monospace,monospace}@media(prefers-color-scheme:dark){body{background:#111;color:#ddd}pre{background:#222}}</style>
</head><body>
<h1>pwdx apt repository</h1>
<p>Stateless, offline password derivation. Debian and Ubuntu (22.04 or newer). Current version: <b>$version</b>.</p>
<h2>Install</h2>
<pre><code>sudo install -d -m 0755 /usr/share/keyrings
curl -fsSL $REPO_URL/pwdx-archive-keyring.gpg | sudo tee /usr/share/keyrings/pwdx-archive-keyring.gpg &gt;/dev/null
echo "deb [signed-by=/usr/share/keyrings/pwdx-archive-keyring.gpg] $REPO_URL ./" | sudo tee /etc/apt/sources.list.d/pwdx.list
sudo apt update
sudo apt install $PKG</code></pre>
<p>Then run <code>${BIN_NAME:-pwdx-cli} selftest</code>. Updates arrive through <code>sudo apt update &amp;&amp; sudo apt upgrade</code>.</p>
<h2>Verify the signing key</h2>
<p>Fingerprint: <code>$fpr</code></p>
<pre><code>gpg --show-keys --fingerprint /usr/share/keyrings/pwdx-archive-keyring.gpg</code></pre>
<p>Compare it with the fingerprint published in the project README.</p>
</body></html>
HTML

echo
echo "==> staged: $REPO_DIR  (version $version)"
echo "    signing key fingerprint: $fpr   <- publish this in your README and on your website"
if [ -n "${UPLOAD_CMD:-}" ]; then
  echo "==> uploading: $UPLOAD_CMD"
  bash -c "$UPLOAD_CMD"
else
  echo "    next: upload the CONTENTS of $REPO_DIR/ so that $REPO_URL/InRelease is reachable, then run:"
  echo "    bash packaging/deb/verify-published.sh $REPO_URL $PKG"
fi
