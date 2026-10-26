#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
# Creates a signed, flat APT repository directory from .deb files. Upload the directory to any static web host.
#   bash packaging/deb/make-apt-repo.sh <repo-dir> <gpg-key-id> dist/*.deb
# Re-run it with every new version (keep older .deb files in <repo-dir> if you want them to stay installable).
# Needs: apt-utils (apt-ftparchive), gnupg, gzip. Sign on a trusted machine; the signing key never goes on the web host.
set -euo pipefail

repo="${1:?usage: $0 <repo-dir> <gpg-key-id> <deb files...>}"
key="${2:?gpg key id}"
shift 2
[ "$#" -ge 1 ] || { echo "no .deb files given" >&2; exit 1; }

mkdir -p "$repo"
cp -- "$@" "$repo"/
cd "$repo"

apt-ftparchive packages . > Packages
gzip -9kf Packages
apt-ftparchive \
  -o APT::FTPArchive::Release::Origin="pwdx" \
  -o APT::FTPArchive::Release::Label="pwdx" \
  -o APT::FTPArchive::Release::Description="pwdx apt repository" \
  release . > Release

rm -f InRelease Release.gpg
gpg --batch --yes --default-key "$key" --clearsign -o InRelease Release
gpg --batch --yes --default-key "$key" -abs -o Release.gpg Release
gpg --batch --yes --export "$key" > pwdx-archive-keyring.gpg      # binary keyring, for signed-by=
gpg --batch --yes --armor --export "$key" > pwdx-archive-keyring.asc

echo "repository ready in $(pwd):"
ls -1
echo
echo "key fingerprint (publish this on your website, so users can verify the key):"
gpg --batch --fingerprint "$key" | sed -n '2p'
