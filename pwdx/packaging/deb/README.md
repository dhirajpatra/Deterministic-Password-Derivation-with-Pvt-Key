# Publishing pwdx as an apt package

Goal: users on Debian, Ubuntu and derivatives install with `apt`, without GitHub and without building anything.

## What is and is not possible

| Route | Result | Verdict |
|---|---|---|
| Your own signed apt repository on any HTTPS web host | users add the repo once (3 short commands), then `sudo apt install pwdx-cli` and normal `apt upgrade` | **Do this first.** Tested, see below. |
| A single `.deb` file | `sudo apt install ./pwdx-cli_0.1.0-1_amd64.deb` (apt resolves dependencies) | Simplest; no repo, but no automatic updates. |
| Official Debian / Ubuntu archive (plain `sudo apt install pwdx`, nothing to add) | **possible in principle**: the MIT license meets the Debian Free Software Guidelines | A long road, not a quick step: you need a Debian *source* package (`debian/` directory), every Rust dependency packaged separately in Debian (or vendored under Debian's strict rules), an offline reproducible build, an ITP bug, a sponsor, and review. Expect weeks to months. |
| Launchpad PPA (Ubuntu) | possible; built from a source package | Same source-package work as above, but no Debian review. |

The license is no longer the blocker. What stands between you and a no-setup `sudo apt install` is the Debian packaging process,
not the licensing. Start with your own repository; it gives users a working install today.

## Naming: `pwdx` already exists on Debian/Ubuntu

The `procps` package already installs **`/usr/bin/pwdx`** (it prints a process's working directory). dpkg refuses to install a
package that overwrites another package's file, and even if you tried, you would break a system tool. This script therefore
installs the command as **`pwdx-cli`** (package `pwdx-cli`). The product name is still the placeholder `pwdx` (decision D2); pick a
final, unique name before you publish widely, then build with `PKG_NAME=... BIN_NAME=...`. Check the name is free with
`apt-cache search --names-only '^yourname'` and `command -v yourname` on a stock system.

## 1. One-time: create the repository signing key

On a trusted machine (not the web server):

```bash
gpg --quick-gen-key "pwdx apt repository <you@example.com>" ed25519 sign 2y
gpg --list-keys --with-colons | awk -F: '/^fpr/ {print $10; exit}'      # prints the key fingerprint
```

Protect the key with a passphrase, back up the secret key and the revocation certificate offline, and publish the
**fingerprint** on your website and README so users can verify it. A 2-year expiry forces periodic rotation; extend it with
`gpg --quick-set-expire`.

## 2. Build the package

```bash
MAINTAINER="Your Name <you@example.com>" bash packaging/deb/build-deb.sh
# -> dist/pwdx-cli_0.1.0-1_amd64.deb
```

The package depends on `libc6 (>= 2.34)`, so it installs on Ubuntu 22.04+ and Debian 12+ (and newer). For other CPU types
(arm64, e.g. Raspberry Pi or ARM servers), run the same script on an arm64 machine or runner; the repository can hold both.
Bump `version` in `crates/cli/Cargo.toml` for each release.

## 3. Build the signed repository

```bash
bash packaging/deb/make-apt-repo.sh ./apt-repo <KEY_FINGERPRINT> dist/*.deb
```

This creates `Packages`, `Packages.gz`, `Release`, a signed `InRelease` and `Release.gpg`, and the public keyring
(`pwdx-archive-keyring.gpg` binary and `.asc` armored). Keep older `.deb` files in `apt-repo/` if you want old versions to stay
installable; re-run the script for every release.

## 4. Upload

Upload the contents of `apt-repo/` to any static HTTPS host so that they are served as plain files:
your own web server (nginx or Apache), Cloudflare Pages or R2, Amazon S3 with CloudFront, Netlify, GitLab Pages, Codeberg Pages,
and so on. GitHub is not required. Keep the signing key off the host: only the public files go online.

## 5. What your users run (once)

Replace `YOUR-HOST` with your address:

```bash
sudo install -d -m 0755 /usr/share/keyrings
curl -fsSL https://YOUR-HOST/apt/pwdx-archive-keyring.gpg | sudo tee /usr/share/keyrings/pwdx-archive-keyring.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/pwdx-archive-keyring.gpg] https://YOUR-HOST/apt ./" | sudo tee /etc/apt/sources.list.d/pwdx.list
sudo apt update
sudo apt install pwdx-cli
```

Then they run `pwdx-cli selftest` and follow INSTALL.md (use `pwdx-cli` wherever it says `pwdx`). Updates arrive with
`sudo apt update && sudo apt upgrade`. To remove: `sudo apt remove pwdx-cli`; their key file is never touched.

`signed-by=` limits the key to this one repository only, which is the recommended, safe form (do not use the old `apt-key`).
Security-minded users should compare the key fingerprint with the one you published:

```bash
gpg --show-keys --fingerprint /usr/share/keyrings/pwdx-archive-keyring.gpg
```

## What was tested (and what was not)

Tested in a sandbox (Ubuntu 24.04, root) with a throwaway signing key, using a `file://` repository in place of `https://`:

- the `.deb` builds (the `copyright` file is Debian machine-readable format) and installs through `apt install pwdx-cli`; `pwdx-cli selftest` passes; `apt remove` cleans up;
- the system's own `/usr/bin/pwdx` (procps) is left untouched;
- apt **refuses** a repository that is unsigned, one signed by a different key than `signed-by=` trusts, and one whose package
  index was modified after signing (Hash Sum mismatch).

`lintian` (Debian's policy checker) reports a clean package apart from one warning that only matters for an official Debian upload
(`initial-upload-closes-no-bugs`, which wants an ITP bug number). The package ships a Debian-format `copyright` file, a changelog,
a man page, and `THIRD_PARTY_LICENSES.txt.gz` with the notices of all 91 Rust crates built into the binary.

Not tested: a real HTTPS host; Debian 12 or Ubuntu 22.04 specifically; arm64; an official Debian-style source package.
