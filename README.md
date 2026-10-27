# pwdx — Stateless, Offline Password Derivation

## How to Use 

Instruction site (here[https://dhirajpatra.github.io/pwdx-apt/])

## How it works or want to contribute

> **Status:** Milestone 1 implemented (Rust core + CLI + test vectors). Spec v1 is still a *draft* until the open
> decisions below are settled and the spec is frozen. `pwdx` is a placeholder name. Not yet independently reviewed.

Type one memorable password; get a long, unique, reproducible password for every site. Nothing is stored except an encrypted key file, and nothing ever touches a network.

pwdx turns one password you can remember into a different, long, random-looking password for every site, and the same input always gives the same result on every device. 

There is no vault, no server and no cloud, and nothing you type is ever stored or sent anywhere. 

Each password is derived on your own machine from your memorable password, the site and your username, and mixed with a secret 256-bit key that never leaves your devices. A site that leaks its copy reveals nothing about your other accounts, and nobody can guess your memorable password from a leaked one without also stealing your key file. It is built on a single Rust core using Argon2id, HMAC-SHA512 and HKDF, with the algorithm fully specified and checked against known-answer vectors from an independent implementation. The command-line tool works today, and a Flutter app for desktop and mobile is in progress.

For installation from APT follow the instruction (here[https://dhirajpatra.github.io/pwdx-apt/]). 

```
output = f( memorable password, secret key, site, username, counter, profile )
```

Same inputs → same output, on every platform, every time (idempotent).

## Goals

- **Deterministic and cross-platform.** One Rust core is the only implementation of the algorithm. Linux, Windows, Android and iOS must produce byte-identical output.
- **Per-site passwords.** A site that leaks your password reveals nothing about your other sites.
- **Key-file pepper.** A 256-bit secret key is mixed into every derivation, so a leaked derived password cannot be used to brute-force your memorable password without the key.
- **Key sources.** A dedicated random key file (default) **and** import of an OpenSSH `id_ed25519` (required feature).
- **Offline.** No server, no network permission, no telemetry, no stored passwords.
- **Easy install.** Single static binary on desktop; app packages on mobile (same Rust core as a library).

## Non-goals

- Not a vault for arbitrary secrets (no stored passwords, notes, cards).
- No cloud sync. Moving the key between devices is done with an encrypted file or QR code.
- No claims of "unbreakable" or "quantum-proof". See `skills/pwdx-security-review`.

## How it works (v1 summary)

```
salt    = SHA-256("pwdx/v1/salt" ‖ K)                         K = 32-byte secret seed
stretch = Argon2id(NFKC(S), salt, m=64 MiB, t=3, p=1, 32 B)   S = memorable password
master  = HMAC-SHA512(K, "pwdx/v1/master" ‖ stretch)
stream  = HKDF-SHA512-Expand(master, "pwdx/v1/site" ‖ LP(site) ‖ LP(user) ‖ u32(counter) ‖ LP(profile), 1024)
output  = unbiased encoding of `stream` into the chosen profile (rejection sampling + Fisher-Yates)
```

Full normative detail: `skills/pwdx-derivation-spec/SKILL.md`.

**New here? Read [INSTALL.md](INSTALL.md)** for step-by-step install and run instructions (terminal and app).

## Build and try it

```
cargo test --release --workspace   # 37 tests incl. 30 known-answer vectors from the independent Python reference
cargo build --release           # target/release/pwdx  (~1.7 MB)
pwdx key new                    # or: pwdx key import-ssh ~/.ssh/id_ed25519
pwdx get github.com --user me@example.com
python3 testvectors/reference.py   # regenerates testvectors/v1.json (pip install argon2-cffi)
```

Needs a recent stable Rust. The key file (`~/.config/pwdx/key.pwdxkey`, 107 bytes) is encrypted with its own
passphrase and doubles as your backup; copy it to other devices.

## Implementation status

| Area | State |
|---|---|
| `pwdx-core`: normalization, derivation, 4 profiles, backup file, OpenSSH import, self-test | done, tested |
| CLI (`key new`, `key import-ssh`, `key id`, `get`, `selftest`) | done, tested end to end against the Python reference |
| Known-answer vectors (30 + 11 error cases), independent Python reference | done |
| Statistical bias tests (chi-square, with a deliberately biased control that must be detected) | done |
| Robustness: mutation tests for OpenSSH/backup parsers, normalizer property tests; cargo-fuzz targets in `fuzz/` | tests done; fuzz targets written, **not yet run** (need nightly) |
| `cargo audit` | clean (it flagged `idna 0.5.0`; upgraded to 1.1.0); `cargo deny` policy written, not yet run |
| CI workflow (`.github/workflows/ci.yml`) | written, **not yet run on GitHub**; app jobs marked `continue-on-error` until proven |
| FFI bridge crate (`app/rust`) | done; all vectors pass through its API |
| Flutter app (`app/lib`) | written and syntax-checked; **not compiled or run** (no Flutter SDK available) |
| Static musl / Windows / macOS / Android / iOS builds | not yet built |
| Independent security review | not yet |

## Repository layout

```
pwdx/
├── Cargo.toml                 # workspace
├── crates/
│   ├── core/                  # pwdx-core: pure library, #![forbid(unsafe_code)], no network, no UI   [done]
│   ├── cli/                   # single-binary CLI                                                    [done]

├── app/                       # Flutter UI + rust/ bridge crate (see app/README.md)
├── testvectors/               # v1.json + reference.py (independent Python implementation)
├── fuzz/                      # cargo-fuzz targets (nightly)
├── scripts/                   # bootstrap_app.sh, check_android_no_network.sh
└── skills/                    # implementation guides (this folder)
```

## Skills (implementation guides)

Each folder under `skills/` is a self-contained guide. They are written in the `SKILL.md` format, so you can
read them as normal docs or copy `skills/` into `.claude/skills/` to let Claude Code use them while you build.

| Skill | Read it when you are… |
|---|---|
| `pwdx-derivation-spec` | implementing or changing anything in the algorithm (normative) |
| `pwdx-key-management` | handling the key file, OpenSSH import, backup/export, device transfer |
| `pwdx-rust-core` | laying out crates, choosing dependencies, zeroizing secrets |
| `pwdx-flutter-app` | building the UI, keystore integration, clipboard and screen hardening |
| `pwdx-testing` | writing known-answer vectors, CI matrix, bias and Unicode tests |
| `pwdx-release` | packaging, signing, distributing, versioning and freezing the spec |
| `pwdx-security-review` | threat model, user guidance, pre-release review checklist |

## Open decisions (please settle before freezing v1)

| ID | Decision | Current default |
|---|---|---|
| D1 | Key sites by registrable domain (eTLD+1) or full host. **Needs a decision:** eTLD+1 depends on a Public Suffix List snapshot, which we pin exactly, but it ties v1 to that list forever. Keying on the full host (minus `www.`) avoids that dependency | eTLD+1 (implemented, pinned) |
| D2 | Final app name and bundle ids | `pwdx` placeholder |
| D3 | Source license | **decided:** MIT (permissive; meets the Debian Free Software Guidelines; see `LICENSE`, `NOTICE`) |
| D4 | Default profile and length | `full`, 24 characters |
| D5 | Symbol set for `full` | `!@#$%^&*-_=+` (12 symbols) |
| D6 | Local, non-secret site list (site, user, counter, profile, length) and its encrypted export | yes, optional |

## Roadmap

1. `pwdx-core` + spec v1 + test vectors + CLI
2. Desktop apps (Linux, Windows) with key import and OS keystore
3. Android, then iOS
4. Backup/recovery flow, hardening, packaging
5. Independent security review, then release

## Honest limitations

- Changing the memorable password or the key changes **every** derived password; you would have to rotate every site.
- Losing the key file without a backup means losing every derived password.
- Malware on a device that is unlocked can capture both the memorable password and the key.
- Strength of the whole scheme is bounded by the memorable password if the key file is stolen.

## License

pwdx is released under the **MIT License** (see [LICENSE](LICENSE)). You may use, copy, modify, merge, publish, distribute,
sublicense and sell it, for any purpose, including commercial use, as long as the copyright notice and license text are kept.
There is no warranty. MIT meets the Debian Free Software Guidelines, so the program can be packaged for Debian and Ubuntu.
Third-party components keep their own licenses; see [NOTICE](NOTICE).
