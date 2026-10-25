# pwdx — stateless, offline password derivation

> **Status:** pre-implementation. Spec v1 is a draft until test vectors are published and frozen.
> `pwdx` is a placeholder name.

Type one memorable password; get a long, unique, reproducible password for every site. Nothing is stored
except an encrypted key file, and nothing ever touches a network.

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

## Repository layout (planned)

```
pwdx/
├── Cargo.toml                 # workspace
├── crates/
│   ├── core/                  # pwdx-core: pure library, #![forbid(unsafe_code)], no network, no UI
│   ├── cli/                   # static single-binary CLI
│   └── ffi/                   # flutter_rust_bridge glue (cdylib/staticlib)
├── app/                       # Flutter UI (Linux, Windows, Android, iOS)
├── testvectors/               # frozen known-answer vectors (JSON)
├── spec/                      # frozen spec versions (v1.md, ...)
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
| D1 | Key sites by registrable domain (eTLD+1) or full host | eTLD+1 |
| D2 | Final app name and bundle ids | `pwdx` placeholder |
| D3 | Source license | undecided (own license; not derived from Spectre) |
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
