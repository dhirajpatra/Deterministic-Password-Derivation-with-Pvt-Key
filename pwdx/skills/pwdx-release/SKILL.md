---
name: pwdx-release
description: Packaging, signing, distribution and versioning plan for pwdx on Windows, Linux, Android and iOS, plus reproducible builds, checksums, supply-chain checks and the spec-freeze process. Use whenever preparing a release, choosing an installer format, setting up GitHub Actions release jobs, signing artifacts, submitting to a store, or tagging a spec version. Make sure to consult it before publishing any binary.
---

# pwdx release

## 1. Artifacts per platform

| Platform | Primary | Also |
|---|---|---|
| Windows | signed MSIX or MSI; standalone `pwdx.exe` CLI | winget manifest |
| Linux | Flatpak (no network permission), AppImage | `.deb`; static musl CLI tarball |
| Android | signed AAB (Play) and APK (GitHub Releases) | F-Droid (needs reproducible, fully open build) |
| iOS | TestFlight first, then App Store | requires paid Apple Developer account |
| macOS (optional) | notarized CLI/app | |

iOS is the biggest friction point. Plan for TestFlight to validate before App Store review.

## 2. Build pipeline (GitHub Actions)

- One workflow builds all targets from a tagged commit.
- Pinned toolchains (`rust-toolchain.toml`, pinned Flutter version), `--locked` Cargo builds, vendored or hash-verified dependencies.
- Same CI vectors gate as `pwdx-testing`; a failing vector blocks the release.
- Supply-chain checks on every release: `cargo audit`, `cargo deny` (licenses, bans, sources), SBOM (for example `cargo auditable` or CycloneDX), and a review of any dependency version bump in the diff.
- Reproducible builds where feasible (Linux CLI, Android for F-Droid). Document the build container.

## 3. Signing and integrity

- Windows code signing certificate (avoids SmartScreen warnings); Android release keystore kept offline with a backup; Apple distribution certificate.
- Publish `SHA256SUMS` and a detached signature (`minisign` or `cosign`) with every GitHub release. Put the signing public key in the README and on a second channel (website/profile).
- The app has no auto-update in v1; the release page explains how to verify checksums.

## 4. Versioning

Two independent version numbers:

- **App version:** SemVer for the apps (`1.4.2`).
- **Spec version:** `v1`, `v2`... Shown in the About screen. Changes only when the algorithm changes.

Spec freeze process:

1. Reference script and Rust core agree on all vectors.
2. Vectors committed; independent review of `spec/v1.md`.
3. Tag `spec-v1` (annotated, signed) and protect the tag.
4. From then on, any change to constants or encoding is a `v2` design with a new `pwdx/v2/...` domain string. App releases must keep `v1` derivation forever.

## 5. Release checklist

- [ ] All vectors pass on Linux, Windows, macOS/iOS simulator, Android emulator
- [ ] `self_test` passes in release builds
- [ ] Merged Android release manifest has no `INTERNET`; Flatpak has no network; no analytics/crash SDKs
- [ ] `cargo audit`, `cargo deny`, SBOM generated
- [ ] Artifacts signed; `SHA256SUMS` + signature published
- [ ] README states limitations honestly; no "unbreakable" or "quantum-proof" claims
- [ ] Security review items from `pwdx-security-review` closed or documented
- [ ] Backup/restore tested end to end on every platform
- [ ] Changelog notes spec version and whether outputs changed (they must not within v1)

## 6. Licensing

Decision D3 is made: **MIT** (`LICENSE`, copyright and third-party list in `NOTICE`, an `SPDX-License-Identifier: MIT` line in
every first-party source file). It meets the Debian Free Software Guidelines, so Debian/Ubuntu packaging, F-Droid and the app
stores are all possible. The only obligation on users is to keep the copyright notice and license text. There is no copyleft
and no restriction on commercial use: anyone may build a closed-source product from this code. MIT has no explicit patent
grant; Apache-2.0 would add one (a one-file swap if you ever want it). Every binary release must ship `LICENSE`, `NOTICE` and
`THIRD_PARTY_LICENSES.txt` (generate with `python3 scripts/gen_third_party_licenses.py`; the .deb build does it). Keep the EFF
wordlist's CC BY 3.0 attribution. Dependency licenses are policed by `deny.toml`.
