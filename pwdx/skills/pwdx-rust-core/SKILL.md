---
name: pwdx-rust-core
description: Guide for building the pwdx Rust workspace - crate layout, dependency choices, public API, secret hygiene (zeroize, no unsafe, no logging), error handling, and building the same core as a static CLI binary and a mobile library. Use whenever writing or reviewing Rust code for pwdx, adding a dependency, designing the core API, or wiring the core to the CLI or Flutter FFI. Make sure to consult it before adding any crate or any unsafe block.
---

# pwdx Rust core

The derivation algorithm exists in exactly one place: `crates/core`. Every platform calls it. No UI code, no second implementation.

## 1. Workspace

```
Cargo.toml                  # [workspace] members = ["crates/*"]
crates/core/                # package "pwdx-core"  (lib)
crates/cli/                 # package "pwdx"       (bin, static)
crates/ffi/                 # package "pwdx-ffi"   (cdylib + staticlib, flutter_rust_bridge)
testvectors/                # JSON, consumed by core tests and by the Flutter integration test
```

Rules for `crates/core`:

- `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, no `std::net`, no `reqwest`/`hyper`/`tokio` anywhere in its dependency tree (enforce with `cargo deny`).
- No logging of inputs or outputs, not even at `trace`. Do not depend on `log`/`tracing` in core.
- No global state, no I/O except the explicit key-file parse functions that take `&[u8]`.
- Pure functions and deterministic output, so everything is testable with vectors.

## 2. Dependencies (pin exact versions in Cargo.lock; review updates)

| Purpose | Crate |
|---|---|
| Argon2id | `argon2` |
| HKDF / HMAC / hashes | `hkdf`, `hmac`, `sha2` |
| OpenSSH key parsing and decryption | `ssh-key` (features for ed25519 and encrypted keys) |
| Backup file AEAD | `chacha20poly1305` (XChaCha20-Poly1305) |
| Randomness | `getrandom` |
| Unicode | `unicode-normalization` (NFKC), `idna`, `psl` |
| Secret hygiene | `zeroize` (derive), `secrecy` |
| Wordlist | EFF large list embedded via `include_str!`; check hash in a unit test |

Audit-friendly: prefer RustCrypto crates, keep the tree small, run `cargo audit` and `cargo deny` in CI.

## 3. Public API (suggested)

```rust
pub struct KeySeed(Zeroizing<[u8; 32]>);

pub enum Profile { Alnum, Full, Passphrase, Pin }

pub struct SiteRequest<'a> {
    pub site: &'a str, pub user: &'a str, pub counter: u32,
    pub profile: Profile, pub length: usize,
}

pub fn generate_key() -> Result<KeySeed, Error>;
pub fn import_openssh(bytes: &[u8], passphrase: Option<&[u8]>) -> Result<KeySeed, Error>;
pub fn key_id(k: &KeySeed) -> String;                         // 8 hex chars
pub fn export_backup(k: &KeySeed, pass: &str) -> Result<Vec<u8>, Error>;
pub fn import_backup(bytes: &[u8], pass: &str) -> Result<KeySeed, Error>;
pub fn derive(k: &KeySeed, secret: &str, req: &SiteRequest) -> Result<Zeroizing<String>, Error>;
pub fn self_test() -> Result<(), Error>;                      // runs one embedded known-answer vector
```

Error type: small enum, no secret data inside variants, `Display` messages generic ("decryption failed").
Wrong passphrase vs corrupt file should not be distinguishable for backup files.

## 4. Secret hygiene

- Wrap every secret in `Zeroizing`/`secrecy` types: `S` (as bytes), `stretch`, `master`, `stream`, `K`, and the final password.
- Implement `Debug` for secret types as `"[redacted]"`.
- Use the `argon2` crate's caller-provided memory API if available, so the 64 MiB block array can be zeroized after use.
- No `Clone` on `KeySeed` unless needed; no `Copy`; no `Display`.
- Compare secrets with `subtle::ConstantTimeEq` where comparison is needed (for example `checkint`, public key match).
- Release profile: `panic = "abort"`, `lto = true`, `codegen-units = 1`, `strip = true`, `overflow-checks = true`.
- Panics must never contain secret data (avoid `unwrap` on secret-bearing values; use typed errors).

## 5. Build targets

| Target | Output |
|---|---|
| `x86_64-unknown-linux-musl` | single static CLI binary |
| `x86_64-pc-windows-msvc` | `pwdx.exe` |
| `aarch64-apple-darwin`, `x86_64-apple-darwin` | macOS CLI (optional) |
| `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` | `libpwdx_ffi.so` inside the app |
| `aarch64-apple-ios`, `aarch64-apple-ios-sim` | static library inside the app |

- Mobile cannot ship a standalone executable; the same `core` crate is linked into the app as a library.
- 64 MiB Argon2 memory must succeed on low-end phones; test on the slowest and smallest-RAM device you support. On failure return an error. **Never reduce parameters.**

## 6. CLI behaviour (the desktop "binary")

```
pwdx key new | import <file> | export | id
pwdx get <site> --user <u> [--counter N] [--profile full|alnum|passphrase|pin] [--length L]
pwdx selftest
```

- Read the memorable password from a no-echo TTY prompt only. Do not accept it as a command-line argument or environment variable (shell history, `/proc`).
- Output goes to stdout without a trailing prompt text; offer `--copy` with auto-clear.
- Exit non-zero on any error; print nothing sensitive on error.
