//! Import of an OpenSSH `id_ed25519` private key (`openssh-key-v1`), plain or passphrase-protected.

use crate::error::Error;
use crate::key::KeySeed;
use ssh_key::private::KeypairData;
use ssh_key::PrivateKey;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Extract the 32-byte Ed25519 seed from an OpenSSH private key file.
///
/// `passphrase` is used as raw bytes exactly as typed (OpenSSH does not normalize it).
/// Only Ed25519 keys are accepted; hardware-backed (`sk-`) keys have no seed and are rejected.
pub fn import_openssh(file: &[u8], passphrase: Option<&[u8]>) -> Result<KeySeed, Error> {
    let key = PrivateKey::from_openssh(file)
        .map_err(|_| Error::Format("not a valid OpenSSH private key"))?;
    let key = if key.is_encrypted() {
        let pass = passphrase.ok_or(Error::PassphraseRequired)?;
        key.decrypt(pass).map_err(|_| Error::Decrypt)?
    } else {
        key
    };
    match key.key_data() {
        KeypairData::Ed25519(kp) => {
            let seed = Zeroizing::new(kp.private.to_bytes());
            // Verify the public key really belongs to this seed.
            let sk = ed25519_dalek::SigningKey::from_bytes(&seed);
            let derived = sk.verifying_key().to_bytes();
            if !bool::from(derived.ct_eq(&kp.public.0)) {
                return Err(Error::Format("public key does not match private key"));
            }
            Ok(KeySeed::from_bytes(*seed))
        }
        KeypairData::SkEd25519(_) => Err(Error::Format("hardware-backed keys cannot be used")),
        _ => Err(Error::Format("only Ed25519 keys are supported")),
    }
}
