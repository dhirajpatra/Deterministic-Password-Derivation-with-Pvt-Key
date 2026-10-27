#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Dhiraj Patra
# SPDX-License-Identifier: MIT
# Cross-builds the Windows (x86_64) CLI on Linux and packs it as dist/pwdx-<version>-windows-x86_64.zip.
# Needs: a Rust toolchain with the Windows target (`rustup target add x86_64-pc-windows-gnu`), the mingw cross
# compiler (`sudo apt install gcc-mingw-w64-x86-64`) and python3.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' crates/cli/Cargo.toml | head -1)"
TARGET=x86_64-pc-windows-gnu
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-x86_64-w64-mingw32-gcc}"

cargo build --release --locked -p pwdx --target "$TARGET"

name="pwdx-${VERSION}-windows-x86_64"
stage="$(mktemp -d)/$name"
mkdir -p "$stage" dist
cp "target/$TARGET/release/pwdx.exe" "$stage/pwdx.exe"
cp LICENSE NOTICE "$stage/"
cp packaging/windows/README.txt "$stage/README.txt"
python3 scripts/gen_third_party_licenses.py "$stage/THIRD_PARTY_LICENSES.txt" pwdx >/dev/null

python3 - "$stage" "dist/$name.zip" <<'PY'
import os, sys, zipfile
src, out = sys.argv[1], sys.argv[2]
base = os.path.basename(src)
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for f in sorted(os.listdir(src)):
        z.write(os.path.join(src, f), f"{base}/{f}")
print("wrote", out)
PY
( cd dist && sha256sum "$name.zip" | tee "$name.zip.sha256" )
