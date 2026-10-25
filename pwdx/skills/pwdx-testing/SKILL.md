---
name: pwdx-testing
description: Test strategy for pwdx - known-answer test vectors, vector file format, independent reference cross-checks, Unicode normalization cases, unbiased-encoding statistics, key import fixtures, and the cross-platform CI matrix that proves idempotence on Linux, Windows, Android and iOS. Use whenever writing tests, generating or changing test vectors, setting up CI, or investigating a platform mismatch. Make sure to consult it before declaring any derivation change done.
---

# pwdx testing

Idempotence is a testable property, so it must be enforced by tests, not by hope. A frozen vector file is the contract between all platforms.

## 1. Vector file format (`testvectors/v1.json`)

```json
{
  "spec": "v1",
  "vectors": [
    {
      "id": "full-basic-01",
      "key_hex": "<64 hex chars>",
      "secret": "<memorable password, UTF-8>",
      "site": "example.com", "user": "alice@example.com",
      "counter": 1, "profile": "full", "length": 24,
      "expected": "<password>"
    }
  ]
}
```

- Keys in vectors are throwaway test keys. Never use a real key or real password.
- Every vector is consumed by the Rust unit tests, the CLI tests and the Flutter integration test (through the FFI).
- Once published, vectors are append-only. Editing an existing `expected` value means the spec changed, which means v2.

## 2. Vector coverage

1. One vector per profile (`alnum`, `full`, `passphrase`, `pin`) at default length, plus min and max length for the character profiles.
2. Counter 1, 2, and a large value (for example `4294967295`).
3. Different `user` and different `site` change the output (assert inequality in a property test, plus fixed vectors).
4. Normalization cases, expecting **identical** output: `é` (U+00E9) vs `e` + U+0301; fullwidth `ＡＢＣ１２３` vs `ABC123`; ligature `ﬁ` vs `fi`; sites `HTTPS://WWW.Example.COM/path?q=1` vs `example.com`; Unicode username forms.
5. Cases expecting **different** output: trailing space in `S`; different case in `user`.
6. Emoji, CJK, RTL and combining-mark passwords.
7. Rejection cases: control characters in `S`, empty site or user, length out of range, counter 0.
8. Key sources: the same seed obtained from a dedicated key, from an unencrypted OpenSSH key, and from a passphrase-protected OpenSSH key must give identical derived output.

## 3. Independent cross-check

Generate the first vectors with a **second, independent implementation** (for example a short Python script using `argon2-cffi`, `hmac`/`hashlib`, and a small HKDF-Expand), not with the Rust code being tested. Two implementations agreeing is the evidence that the spec is unambiguous. Keep that script in the repo as `testvectors/reference.py`; it is documentation, not shipped code.

Also run each primitive against its official vectors: Argon2 (RFC 9106), HKDF (RFC 5869), HMAC-SHA512 (RFC 4231), XChaCha20-Poly1305 (draft/crate vectors).

## 4. Key import fixtures

Create fixtures with the real tools and commit them:

```
ssh-keygen -t ed25519 -N ""        -f fixtures/plain   -C test
ssh-keygen -t ed25519 -N "hunter2" -f fixtures/protected -C test
```

Test: parse OK, seed matches, public key matches, wrong passphrase fails with a generic error, RSA/ECDSA/`sk-` keys are rejected, truncated and bit-flipped files fail cleanly (no panic).

Backup file: round trip; wrong passphrase and tampered header or ciphertext give the same error; header AAD binding verified by flipping a header byte.

## 5. Property and statistical tests

- Idempotence: 1,000 random inputs derived twice, equal.
- Every character-class rule satisfied for 100,000 derivations per profile.
- Length is exact; alphabet membership is exact; no characters outside the profile.
- Bias: derive a large number of passwords with random inputs; chi-square test of the first character and of each class-guaranteed position against uniform. Failing p-values should be rare and re-runnable with a new seed.
- `pick` unit tests with tiny synthetic streams: values at and around `limit`, to prove the rejection branch and byte consumption order.
- Fuzz (`cargo-fuzz`): OpenSSH parser, backup parser, site normalizer. Required: no panics, no hangs.

## 6. CI matrix

| Job | Runs |
|---|---|
| Linux (ubuntu) | `cargo test --workspace`, clippy, fmt, `cargo audit`, `cargo deny`, fuzz smoke |
| Windows | `cargo test --workspace`; CLI against vectors |
| macOS | `cargo test`; iOS simulator integration test via Flutter |
| Android | emulator job running the Flutter integration test over FFI against `v1.json` |
| Release build check | static musl CLI builds; merged Android release manifest has no `INTERNET`; Flatpak manifest has no network |

The gate: **identical output for every vector on every platform**, or the build fails.

Run `self_test()` (one embedded vector) in debug and release builds so toolchain or optimization bugs are caught immediately.

## 7. When a vector fails

1. Re-read the "Common mistakes" list in `pwdx-derivation-spec`.
2. Dump intermediate values (`salt`, `stretch`, `master`, first 32 bytes of `stream`) from both implementations **on test keys only**, and compare them step by step.
3. Fix the implementation, not the vector (unless the spec text itself was wrong, which then needs a documented spec erratum before release).
