// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! The 32-byte secret seed `K` (see `skills/pwdx-key-management`).

use crate::error::Error;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// 32-byte secret key material. Zeroized on drop; never printed.
pub struct KeySeed(Zeroizing<[u8; 32]>);

impl KeySeed {
    /// Wrap existing key bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        KeySeed(Zeroizing::new(bytes))
    }

    /// Generate a fresh key from the OS CSPRNG.
    pub fn generate() -> Result<Self, Error> {
        let mut b = Zeroizing::new([0u8; 32]);
        getrandom::getrandom(&mut *b).map_err(|_| Error::Random)?;
        Ok(KeySeed(b))
    }

    /// Parse 64 hex characters. Intended for tests and tooling only.
    pub fn from_hex(hex: &str) -> Result<Self, Error> {
        let h = hex.as_bytes();
        if h.len() != 64 {
            return Err(Error::InvalidInput("key hex must be 64 characters"));
        }
        let nib = |c: u8| -> Result<u8, Error> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                b'A'..=b'F' => Ok(c - b'A' + 10),
                _ => Err(Error::InvalidInput("key hex has a non-hex character")),
            }
        };
        let mut out = Zeroizing::new([0u8; 32]);
        for i in 0..32 {
            out[i] = (nib(h[2 * i])? << 4) | nib(h[2 * i + 1])?;
        }
        Ok(KeySeed(out))
    }

    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Non-secret check value: first 4 bytes of `SHA-256("pwdx/v1/kid" || K)` as 8 lowercase hex characters.
    pub fn key_id(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"pwdx/v1/kid");
        h.update(*self.0);
        let d = h.finalize();
        d[..4].iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// The raw 32 key bytes, for storing in an OS keystore. Handle with care: wrap in `Zeroizing` and never log.
pub fn key_bytes(k: &KeySeed) -> zeroize::Zeroizing<[u8; 32]> {
    zeroize::Zeroizing::new(*k.as_bytes())
}

impl core::fmt::Debug for KeySeed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("KeySeed([redacted])")
    }
}

impl PartialEq for KeySeed {
    fn eq(&self, other: &Self) -> bool {
        use subtle::ConstantTimeEq;
        self.0.ct_eq(&*other.0).into()
    }
}
