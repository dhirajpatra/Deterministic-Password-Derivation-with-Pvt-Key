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

Decision D3 is made: **PolyForm Noncommercial License 1.0.0** (`LICENSE`, with the required notice in `NOTICE`). It is
source-available, not OSI "open source", so some channels will not accept it: F-Droid only hosts free/open-source
software, and Apple App Store distribution of noncommercial-licensed code is allowed for the owner but re-distribution
by others is limited by their own noncommercial restriction. Keep `NOTICE` (it carries the `Required Notice:` line that
downstream users must pass on), include third-party notices in every binary release (for example via `cargo about`),
and respect the EFF wordlist's CC BY 3.0 attribution. Dependency licenses are policed by `deny.toml`.
