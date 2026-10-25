---
name: pwdx-security-review
description: Threat model, attacker scenarios, honest security claims, user guidance and pre-release review checklist for pwdx. Use whenever someone asks how secure the scheme is, what happens if a site leaks a password or a device is stolen, how strong the memorable password must be, what quantum computers change, what wording is safe for marketing or UI text, or when preparing an independent security review. Make sure to consult it before writing any security claim.
---

# pwdx security review

## 1. What the scheme protects against

| Scenario | Outcome |
|---|---|
| A site leaks or logs your derived password | Attacker learns that one password. Other sites differ (per-site `info`). Recovering your memorable password requires the key `K`, which they do not have. |
| Credential stuffing | Fails; no two sites share a password. |
| Attacker steals the **key file/device**, no memorable password | Must guess `S`. Each guess costs one 64 MiB Argon2id run, and they also need a derived password to test against. |
| Attacker steals the key **and** has a leaked derived password | Offline guessing of `S`. Security is now bounded by the entropy of `S`. |
| Phishing site with a different domain | Different domain gives a different password, so the lookalike gets nothing useful. Only if the domain is derived from the real URL or typed correctly; typing the wrong domain just yields a wrong password. |

## 2. What it does not protect against

- Malware or a keylogger on an unlocked device: it can capture `S` and read or use `K`. Game over for that device.
- A weak memorable password combined with a stolen key file.
- Shoulder surfing, screen recording, clipboard sniffers (mitigated, not eliminated).
- Loss of the key (no recovery without a backup).
- Compromise of **both** `S` and `K`: every password is derivable; recovery means rotating every site and generating a new key.
- Dart-heap residue of the typed password (Flutter limitation; see `pwdx-flutter-app`).

## 3. Memorable password guidance (put this in the UI)

Because a thief with the key file can test guesses offline, the memorable password should be a passphrase, not a clever short word.

- Floor: 4 random words (about 51 bits with a 7776-word list); better: 5 to 6 words.
- Random beats clever: pick words with dice or the app's passphrase generator, not song lyrics or personal facts.
- Long and unusual beats short with symbols.
- Never reuse the memorable password anywhere else, and never type it into a site.

## 4. Quantum computers: what to say

- Hashes, HMAC, HKDF and Argon2id lose roughly half their bit strength to Grover's algorithm; a 256-bit secret remains at about 128-bit effective strength, which is out of reach.
- Public-key schemes (including Ed25519) are what Shor's algorithm breaks. The derivation uses the Ed25519 seed only as a secret byte string, never as a signature or key-agreement operation.
- Residual risk: an **SSH public key is published**, so a future quantum computer could recover the seed from it. Hence the dedicated key is the default and the SSH import carries a warning.
- Safe wording: "built from symmetric primitives that are believed to remain strong against known quantum attacks". Unsafe wording: "quantum-proof", "unbreakable", "impossible to hack".
- No "quantum randomness" feature: the OS CSPRNG is sufficient, and determinism is incompatible with fresh randomness at derivation time.

## 5. Claims policy

Allowed: offline, no server, no stored passwords, open to review, per-site unique, memory-hard key stretching, key-file pepper, spec and vectors public.
Not allowed: "uncrackable", "military-grade", "years to break" without stating the assumptions (entropy of `S`, whether `K` is stolen), or any promise about malware-infected devices.

## 6. Pre-release review checklist

Algorithm
- [ ] Spec reviewed by someone who did not write it; reference script and Rust agree
- [ ] No use of the public key as input anywhere; salt derived from the seed
- [ ] No modulo bias; stream cursor handling verified
- [ ] Parameters never auto-reduced; allocation failure is an error

Implementation
- [ ] `forbid(unsafe_code)` in core; secrets zeroized; `Debug` redacted; no secret logging
- [ ] CLI reads the password from a no-echo prompt only; no argv/env input
- [ ] Parsers fuzzed (OpenSSH, backup file, site normalizer)
- [ ] Constant-time comparisons where relevant

Platform
- [ ] Keystore settings verified on each OS (device-only, auth required)
- [ ] Clipboard clearing and sensitive flags verified manually per OS
- [ ] `FLAG_SECURE` and iOS background cover verified
- [ ] No network permission in release manifests; no analytics SDKs

Process
- [ ] Independent security review commissioned before relying on it for critical accounts
- [ ] Vulnerability disclosure contact published
- [ ] Release artifacts signed; checksums published
- [ ] User-facing limitations match section 2

## 7. Responding to a bug report

1. If the derivation output could change, treat it as a spec change: do not silently patch v1.
2. If a bug weakens secrecy (for example bias, leakage), publish `v2` with a fix, keep `v1` for reproducing old passwords, and tell users which sites to rotate.
