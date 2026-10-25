---
name: pwdx-derivation-spec
description: Normative v1 specification of the pwdx password-derivation algorithm (inputs, Unicode normalization, Argon2id, HMAC, HKDF, unbiased encoding, profiles, versioning). Use whenever implementing, reviewing, debugging or changing how a password is derived, whenever outputs differ between platforms, or whenever someone proposes changing a constant, alphabet, parameter or ordering. Make sure to consult this before touching any derivation code, even for "small" tweaks.
---

# pwdx derivation spec v1 (normative)

The words MUST / MUST NOT are meant literally. If code and this document disagree, the document wins,
until a new spec version is released.

## 1. Inputs

| Name | Type | Notes |
|---|---|---|
| `S` | Unicode text | memorable password; never stored or logged |
| `K` | 32 bytes | secret seed from a key file or an imported OpenSSH Ed25519 key (see `pwdx-key-management`) |
| `site` | text | normalized per section 2 |
| `user` | text | normalized per section 2 |
| `counter` | u32 | >= 1; increments to rotate a site password |
| `profile` | id + length | e.g. `full:24`; section 6 |

## 2. Normalization

All text becomes UTF-8 after normalization.

- `S`: Unicode **NFKC**. MUST NOT trim, case-fold or collapse whitespace. Reject control characters (Unicode category Cc).
- `site`: trim; strip scheme, credentials, path, query, port; strip a leading `www.`; convert to IDNA (punycode, UTS 46); lowercase with ASCII-only lowercasing after IDNA (avoids locale bugs such as Turkish dotless i). Then, per decision D1, reduce to the registrable domain (eTLD+1) using the Public Suffix List. Spec v1 pins one PSL snapshot date inside the spec; the snapshot only changes in a new spec version.
- `user`: NFKC, trim leading/trailing whitespace, case preserved.
- Empty `site` or `user` after normalization MUST be an error (use an explicit constant such as `-` if a site has no username; the constant is chosen by the caller, not by the library).
- Length-prefix every variable-length field: `LP(x) = u32_be(len_in_bytes(x)) ‖ x`.

## 3. Derivation

```
salt    = SHA-256( "pwdx/v1/salt" ‖ K )                                  // 32 bytes; never the public key
stretch = Argon2id( password = UTF8(NFKC(S)), salt = salt,
                    m = 65536 KiB, t = 3, p = 1, outlen = 32, version = 0x13 )
master  = HMAC-SHA512( key = K, msg = "pwdx/v1/master" ‖ stretch )      // 64 bytes
info    = "pwdx/v1/site" ‖ LP(site) ‖ LP(user) ‖ u32_be(counter) ‖ LP(profile_tag)
stream  = HKDF-SHA512-Expand( PRK = master, info = info, L = 1024 )     // no Extract step
```

- `profile_tag` is the ASCII string `<profile_id>:<length>`, for example `full:24`, `passphrase:8`, `pin:6`.
- Domain-separation strings are ASCII, with no terminator byte.
- If the Argon2 implementation exposes the `secret` parameter, v1 does NOT use it (keeps one canonical form).
- The implementation MUST NOT lower Argon2 parameters when memory is scarce. If allocation fails, return an error. A fallback would silently change every password.

## 4. The `stream` cursor

`stream` is consumed strictly sequentially through a single cursor that is never reset. If the stream
is exhausted, return `Error::StreamExhausted` (practically unreachable: a 128-char `full` password uses roughly 300 bytes).

## 5. Unbiased selection

```
pick(n):                       // uniform integer in [0, n), 1 <= n <= 65536
  if n <= 256:
      limit = 256 - (256 % n)
      loop: b = next_u8();                 if b < limit: return b % n
  else:
      limit = 65536 - (65536 % n)
      loop: v = u16_be(next_u8(), next_u8()); if v < limit: return v % n
```

Never use `b % n` without the rejection test (modulo bias).

## 6. Profiles

Alphabets are ASCII and have a fixed order. Class order matters.

| Profile id | Classes (in order) | Length | Default | Notes |
|---|---|---|---|---|
| `alnum` | `A-Z`, `a-z`, `0-9` | 8..128 | 24 | ~143 bits at 24 |
| `full` | `A-Z`, `a-z`, `0-9`, `!@#$%^&*-_=+` | 8..128 | 24 | ~149 bits at 24 (D4, D5) |
| `passphrase` | EFF large wordlist (7776 words), separator `-`, lowercase | 4..16 words | 8 | ~103 bits at 8 words |
| `pin` | `0-9` | 4..12 | 6 | ~20 bits at 6; UI MUST warn it is weak |

Character-class profiles (`alnum`, `full`):

```
out  = []
for class in classes:  out.push( class[ pick(len(class)) ] )        // one guaranteed char per class
full = concat(classes)
while len(out) < L:    out.push( full[ pick(len(full)) ] )
for i in (L-1) down to 1:  j = pick(i+1);  swap(out[i], out[j])      // Fisher-Yates
return join(out)
```

`passphrase`: for each word `words[pick(7776)]`, joined with `-`. No shuffle, no extra classes.
`pin`: `L` digits, each `pick(10)`. No class guarantee.

## 7. Versioning and freezing

- Every constant in this document belongs to spec **v1**. After the first release, v1 is frozen: no parameter, alphabet, ordering, normalization or string change, however small.
- Any change ships as `v2` with new domain strings (`pwdx/v2/...`). The app keeps v1 available permanently so old passwords stay reproducible.
- The app stores no passwords, but it SHOULD store, per site (non-secret): `site, user, counter, profile, length, spec_version`.

## 8. Common mistakes (check these first when outputs differ)

1. Salting with the public key instead of the seed.
2. Trimming or case-folding `S`; skipping NFKC; normalizing in the UI layer instead of the core.
3. Little-endian length prefixes or counter (everything is big-endian).
4. Running HKDF-Extract on `master`, or restarting the stream cursor between steps.
5. Modulo bias in `pick`, or `pick` using a different byte width than specified.
6. Locale-dependent lowercasing of `site`.
7. Changing the symbol set order, or building `full` as a set (unordered) instead of an ordered string.
8. Platform-specific Argon2 parameter "tuning".
9. Forgetting that profile **and length** are inside `info`.

## 9. Required API shape (core)

```rust
pub fn derive(key: &KeySeed, secret: &str, req: &SiteRequest) -> Result<Zeroizing<String>, Error>;
```

`SiteRequest { site, user, counter, profile, length }`. All normalization happens inside `derive`.
