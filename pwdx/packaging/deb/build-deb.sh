#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
# Builds a .deb of the pwdx CLI for the machine's architecture.
#   MAINTAINER="Your Name <you@example.com>" bash packaging/deb/build-deb.sh
# Output: dist/<package>_<version>_<arch>.deb
#
# NAME CLASH: Debian/Ubuntu already ship /usr/bin/pwdx (package procps), so dpkg would refuse a package that installs
# a file with that name. The package and the installed command are therefore called `pwdx-cli` by default.
# Override with PKG_NAME / BIN_NAME only after you have chosen a final product name (README decision D2).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

PKG="${PKG_NAME:-pwdx-cli}"
BIN="${BIN_NAME:-pwdx-cli}"
MAINTAINER="${MAINTAINER:?set MAINTAINER=\"Your Name <you@example.com>\"}"
UPSTREAM="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' crates/cli/Cargo.toml | head -1)}"
VERSION="${UPSTREAM}-${DEB_REVISION:-1}"
ARCH="$(dpkg --print-architecture)"
HOMEPAGE="${HOMEPAGE:-https://github.com/dhirajpatra}"

cargo build --release --locked -p pwdx

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

chmod 0755 "$stage"
install -Dm755 target/release/pwdx "$stage/usr/bin/$BIN"
install -Dm644 packaging/deb/copyright "$stage/usr/share/doc/$PKG/copyright"
python3 scripts/gen_third_party_licenses.py "$stage/usr/share/doc/$PKG/THIRD_PARTY_LICENSES.txt" pwdx >/dev/null
gzip -9n "$stage/usr/share/doc/$PKG/THIRD_PARTY_LICENSES.txt"
install -Dm644 NOTICE  "$stage/usr/share/doc/$PKG/NOTICE"
install -Dm644 README.md "$stage/usr/share/doc/$PKG/README.md"
install -Dm644 crates/core/data/WORDLIST-LICENSE.txt "$stage/usr/share/doc/$PKG/WORDLIST-LICENSE.txt"

# Debian changelog and manual page
{
  printf '%s (%s) unstable; urgency=medium\n\n  * Release %s.\n\n -- %s  %s\n' "$PKG" "$VERSION" "$UPSTREAM" "$MAINTAINER" "$(date -R)"
} | gzip -9n > "$stage/usr/share/doc/$PKG/changelog.Debian.gz"
chmod 0644 "$stage/usr/share/doc/$PKG/changelog.Debian.gz"
mkdir -p "$stage/usr/share/man/man1"
sed -e "s|@BIN@|$BIN|g" -e "s|@VERSION@|$UPSTREAM|g" -e "s|@DATE@|$(date +%Y-%m-%d)|g" packaging/deb/man.1.in | gzip -9n > "$stage/usr/share/man/man1/$BIN.1.gz"
chmod 0644 "$stage/usr/share/man/man1/$BIN.1.gz"

size_kb="$(du -sk "$stage" | cut -f1)"
mkdir -p "$stage/DEBIAN"
cat > "$stage/DEBIAN/control" <<CONTROL
Package: $PKG
Version: $VERSION
Section: utils
Priority: optional
Architecture: $ARCH
Depends: libc6 (>= 2.34), libgcc-s1
Installed-Size: $size_kb
Maintainer: $MAINTAINER
Homepage: $HOMEPAGE
Description: stateless, offline password derivation
 Turns one memorable password into a different, long, random-looking password
 for every site. The same inputs always give the same result on every device.
 Nothing is stored or sent anywhere; a secret key file that you control is
 mixed into every derivation (Argon2id, HMAC-SHA512, HKDF).
 .
 The installed command is called $BIN. Released under the MIT License.
CONTROL

mkdir -p dist
out="dist/${PKG}_${VERSION}_${ARCH}.deb"
dpkg-deb --root-owner-group --build "$stage" "$out" >/dev/null
echo "built $out"
dpkg-deb --info "$out" | sed -n '1,12p'
