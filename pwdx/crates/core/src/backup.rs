//! Encrypted key backup / transfer file (107 bytes, see `skills/pwdx-key-management`).
//!
//! ```text
//! 0   8  magic "PWDXKEY1"     8  1 version=1     9  1 kdf id=1 (Argon2id)
//! 10  4  m_kib (BE)          14  4 t (BE)       18  1 p
//! 19 16  salt                35 24 nonce        59 48 XChaCha20-Poly1305(K) || tag
//! ```
//! The AAD is bytes `0..59`.

use crate::error::Error;
use crate::key::KeySeed;
use crate::normalize;
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

const MAGIC: &[u8; 8] = b"PWDXKEY1";
const HEADER_LEN: usize = 59;
/// Total size of a backup file in bytes.
pub const BACKUP_LEN: usize = HEADER_LEN + 48;
const DEFAULT_M_KIB: u32 = 65536;
const DEFAULT_T: u32 = 3;
const DEFAULT_P: u8 = 1;
/// Upper bounds accepted from an untrusted file (prevents denial of service via huge parameters).
const MAX_M_KIB: u32 = 262_144;
const MAX_T: u32 = 6;
const MAX_P: u8 = 4;

fn wrap_key(pass: &str, salt: &[u8], m: u32, t: u32, p: u8) -> Result<Zeroizing<[u8; 32]>, Error> {
    let pass = normalize::secret(pass)?;
    let params = Params::new(m, t, p as u32, Some(32)).map_err(|_| Error::Kdf)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(pass.as_bytes(), salt, &mut *key)
        .map_err(|_| Error::Kdf)?;
    Ok(key)
}

/// Encrypt `key` under a backup passphrase.
pub fn export_backup(key: &KeySeed, passphrase: &str) -> Result<Vec<u8>, Error> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    getrandom::getrandom(&mut salt).map_err(|_| Error::Random)?;
    getrandom::getrandom(&mut nonce).map_err(|_| Error::Random)?;
    export_with(
        key,
        passphrase,
        salt,
        nonce,
        DEFAULT_M_KIB,
        DEFAULT_T,
        DEFAULT_P,
    )
}

fn export_with(
    key: &KeySeed,
    pass: &str,
    salt: [u8; 16],
    nonce: [u8; 24],
    m: u32,
    t: u32,
    p: u8,
) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(BACKUP_LEN);
    out.extend_from_slice(MAGIC);
    out.push(1);
    out.push(1);
    out.extend_from_slice(&m.to_be_bytes());
    out.extend_from_slice(&t.to_be_bytes());
    out.push(p);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    debug_assert_eq!(out.len(), HEADER_LEN);

    let wk = wrap_key(pass, &salt, m, t, p)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*wk));
    let ct = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: key.as_bytes(),
                aad: &out[..HEADER_LEN],
            },
        )
        .map_err(|_| Error::Decrypt)?;
    out.extend_from_slice(&ct);
    debug_assert_eq!(out.len(), BACKUP_LEN);
    Ok(out)
}

/// Decrypt a backup file. Wrong passphrase and tampering give the same error.
pub fn import_backup(bytes: &[u8], passphrase: &str) -> Result<KeySeed, Error> {
    if bytes.len() != BACKUP_LEN || &bytes[..8] != MAGIC {
        return Err(Error::Format("not a pwdx key backup"));
    }
    if bytes[8] != 1 || bytes[9] != 1 {
        return Err(Error::Format("unsupported backup version"));
    }
    let m = u32::from_be_bytes(bytes[10..14].try_into().unwrap());
    let t = u32::from_be_bytes(bytes[14..18].try_into().unwrap());
    let p = bytes[18];
    if !(8192..=MAX_M_KIB).contains(&m) || !(1..=MAX_T).contains(&t) || !(1..=MAX_P).contains(&p) {
        return Err(Error::Format("backup parameters are out of range"));
    }
    let salt = &bytes[19..35];
    let nonce = &bytes[35..59];

    let wk = wrap_key(passphrase, salt, m, t, p)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&*wk));
    let pt = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: &bytes[HEADER_LEN..],
                aad: &bytes[..HEADER_LEN],
            },
        )
        .map_err(|_| Error::Decrypt)?;
    let pt = Zeroizing::new(pt);
    let arr: [u8; 32] = pt.as_slice().try_into().map_err(|_| Error::Decrypt)?;
    Ok(KeySeed::from_bytes(arr))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Small parameters keep unit tests fast; the production default is 64 MiB / t=3.
    const M: u32 = 8192;

    #[test]
    fn round_trip_and_tamper_detection() {
        let k = KeySeed::from_bytes([7u8; 32]);
        let f = export_with(&k, "backup passphrase", [1u8; 16], [2u8; 24], M, 1, 1).unwrap();
        assert_eq!(f.len(), BACKUP_LEN);
        assert_eq!(import_backup(&f, "backup passphrase").unwrap(), k);
        assert_eq!(import_backup(&f, "wrong").unwrap_err(), Error::Decrypt);
        for i in [20usize, 40, 60, 100] {
            let mut g = f.clone();
            g[i] ^= 1;
            assert_eq!(
                import_backup(&g, "backup passphrase").unwrap_err(),
                Error::Decrypt,
                "byte {i}"
            );
        }
        let mut h = f.clone();
        h[0] ^= 1;
        assert!(matches!(
            import_backup(&h, "backup passphrase"),
            Err(Error::Format(_))
        ));
        assert!(matches!(
            import_backup(&f[..50], "x"),
            Err(Error::Format(_))
        ));
    }

    #[test]
    fn passphrase_is_nfkc_normalized() {
        let k = KeySeed::from_bytes([9u8; 32]);
        let f = export_with(&k, "caf\u{e9} secret", [3u8; 16], [4u8; 24], M, 1, 1).unwrap();
        assert_eq!(import_backup(&f, "cafe\u{301} secret").unwrap(), k);
    }

    #[test]
    fn rejects_out_of_range_parameters() {
        let k = KeySeed::from_bytes([1u8; 32]);
        let mut f = export_with(&k, "pass phrase", [0u8; 16], [0u8; 24], M, 1, 1).unwrap();
        f[10..14].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(matches!(
            import_backup(&f, "pass phrase"),
            Err(Error::Format(_))
        ));
    }

    #[test]
    fn mutated_backups_never_panic_and_never_decrypt_to_a_different_key() {
        let k = KeySeed::from_bytes([5u8; 32]);
        let good = export_with(&k, "fuzz passphrase", [8u8; 16], [9u8; 24], M, 1, 1).unwrap();
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        for _ in 0..600 {
            let mut f = good.clone();
            match next() % 4 {
                0 => {
                    let i = (next() as usize) % f.len();
                    f[i] ^= 1 << (next() % 8);
                }
                1 => {
                    let n = (next() as usize) % f.len();
                    f.truncate(n);
                }
                2 => f.extend((0..(next() % 8)).map(|_| next() as u8)),
                _ => {
                    for _ in 0..3 {
                        let i = (next() as usize) % f.len();
                        f[i] = next() as u8;
                    }
                }
            }
            if let Ok(got) = import_backup(&f, "fuzz passphrase") {
                assert_eq!(got, k, "a mutated file decrypted to a different key");
            }
        }
    }
}
