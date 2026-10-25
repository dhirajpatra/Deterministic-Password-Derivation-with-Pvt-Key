---
name: pwdx-key-management
description: How pwdx obtains, stores, backs up and transfers its 32-byte secret key K - dedicated random key file (default) and required OpenSSH id_ed25519 import, including passphrase-protected keys. Use whenever working on key generation, parsing openssh-key-v1, OS keystore storage, encrypted backup/export, QR or file transfer between devices, key rotation, or key ID display. Make sure to use it for any question about "which key", "lost key" or "same key on all devices".
---

# pwdx key management

Both key sources reduce to one thing: a **32-byte seed `K`**. Nothing else in the algorithm depends on how it was obtained.

## 1. Dedicated key file (default, recommended)

- Generate `K` with the OS CSPRNG (`getrandom`). Never derive it from the memorable password.
- Never display `K` itself. Show only the **Key ID**: first 4 bytes of `SHA-256("pwdx/v1/kid" ‖ K)` as 8 hex characters. It is a non-secret check value that lets the user confirm two devices hold the same key.
- A dedicated random key is fully quantum-safe because no public key derived from it is ever published.

## 2. OpenSSH `id_ed25519` import (required)

Parse `openssh-key-v1\0`:

```
magic "openssh-key-v1\0"
string ciphername      string kdfname       string kdfoptions
u32    nkeys (must be 1)
string public key blob
string private section (possibly encrypted)
```

Private section after decryption:

```
u32 checkint1, u32 checkint2        // MUST be equal, otherwise wrong passphrase / corrupt
string "ssh-ed25519"
string pub32
string priv64                       // seed32 ‖ pub32
string comment, padding 1,2,3...
```

Rules:

- `K = priv64[0..32]` (the seed).
- Verify: recompute the Ed25519 public key from the seed and compare with `pub32` (and with the public blob).
- Encrypted files: `kdfname = bcrypt`, `kdfoptions = string salt, u32 rounds`; ciphers include `aes256-ctr` (ssh-keygen default), `aes256-gcm@openssh.com`, `chacha20-poly1305@openssh.com`. Prefer the `ssh-key` crate over hand-rolling.
- The SSH passphrase is raw bytes as typed; do **not** NFKC it (OpenSSH does not).
- Reject: other key types (RSA, ECDSA), `sk-ssh-ed25519` hardware keys (no seed), `nkeys != 1`.
- After import, copy `K` into the OS keystore and **forget the file path**. Nothing may depend on `~/.ssh` afterwards.
- Warn at import time: (a) rotating the SSH key later does not change pwdx output, because only the seed copy matters; (b) the SSH public key is published, and a future quantum computer could recover the seed from it, so the dedicated key is the stronger choice.

Future optional mode (not v1): ssh-agent signing of a context string for a hardware-held key. Out of scope until v2.

## 3. Storage on a device

| Platform | Where `K` lives |
|---|---|
| Android | Wrapped by an Android Keystore key (StrongBox when available), user-authentication required (biometric/PIN) |
| iOS | Keychain, `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`, no iCloud sync |
| Windows | DPAPI-protected blob, or Credential Manager |
| Linux | Secret Service (libsecret); fallback is an encrypted file that needs the backup passphrase |

`K` is never written to logs, crash reports, preferences files or backups that sync to a cloud.

## 4. Encrypted backup / transfer file

```
offset  size  field
0       8     magic "PWDXKEY1"
8       1     format version = 1
9       1     kdf id = 1 (Argon2id)
10      4     m_kib (u32 BE)     default 65536
14      4     t     (u32 BE)     default 3
18      1     p                  default 1
19      16    salt (random)
35      24    nonce (random)
59      48    ciphertext = XChaCha20-Poly1305( K ) ‖ 16-byte tag
```

- Encryption key = `Argon2id(NFKC(backup_passphrase), salt, m, t, p, 32)`.
- AAD = bytes `0..59` (the whole header up to the ciphertext).
- Total 107 bytes: fits in a single QR code. File and QR are the only transfer mechanisms; no server ever sees it.
- The backup passphrase is separate from the memorable password and must be strong. Make the UI say so.
- Decryption failure and tampering must produce the same generic error.
- A backup file is untrusted input: reject parameters outside `m_kib` 8192..262144, `t` 1..6, `p` 1..4 before running Argon2, so a crafted file cannot trigger a huge allocation.

## 5. Rotation and loss

- **Loss without backup = every derived password is gone.** Make backup creation a first-run step the user cannot skip silently.
- **Rotating `K` or `S` changes every password.** Offer a guided "rotate all sites" checklist from the local site list; do not pretend it is a one-click operation.
- Per-site rotation uses `counter` only.
