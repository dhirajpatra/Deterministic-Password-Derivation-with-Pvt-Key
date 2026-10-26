// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! The API exposed to Dart. Keep it small and dumb: all logic lives in `pwdx-core`.
//!
//! Key handling model: the 32-byte key lives in a Rust-side *session* (`Arc<KeySeed>`, zeroized when the last
//! reference drops). Dart holds no key object; it only calls `unlock_*` / `lock`. Functions that are not marked
//! `#[frb(sync)]` run on the generated thread pool, so the ~1 s Argon2 run never blocks the UI thread.
#![forbid(unsafe_code)]

use flutter_rust_bridge::frb;
use pwdx_core as core;
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

/// Category of an error (a plain enum, so the generated Dart code needs no extra packages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// An input was empty, malformed or out of range.
    InvalidInput,
    /// Key stretching failed (for example not enough memory). Parameters are never lowered.
    Kdf,
    /// A key or backup file is malformed or unsupported.
    Format,
    /// Wrong passphrase, corrupt data or tampering (deliberately indistinguishable).
    Decrypt,
    /// An encrypted OpenSSH key needs a passphrase.
    PassphraseRequired,
    /// No key is unlocked.
    Locked,
    /// Anything else (random generator failure, internal error).
    Internal,
}

/// Error surfaced to Dart. `message` is generic and never contains secret data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PwdxError {
    /// Category, for the UI to branch on.
    pub kind: ErrorKind,
    /// Human-readable, non-secret description.
    pub message: String,
}

impl PwdxError {
    fn new(kind: ErrorKind, message: &str) -> Self {
        PwdxError {
            kind,
            message: message.to_string(),
        }
    }
    fn locked() -> Self {
        Self::new(ErrorKind::Locked, "no key is unlocked")
    }
}

impl From<core::Error> for PwdxError {
    fn from(e: core::Error) -> Self {
        let kind = match e {
            core::Error::InvalidInput(_) => ErrorKind::InvalidInput,
            core::Error::Kdf => ErrorKind::Kdf,
            core::Error::Format(_) => ErrorKind::Format,
            core::Error::Decrypt => ErrorKind::Decrypt,
            core::Error::PassphraseRequired => ErrorKind::PassphraseRequired,
            core::Error::StreamExhausted | core::Error::Random => ErrorKind::Internal,
        };
        PwdxError {
            kind,
            message: e.to_string(),
        }
    }
}

/// Output profile (mirrors `pwdx_core::Profile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    /// `A-Z a-z 0-9`
    Alnum,
    /// `A-Z a-z 0-9 !@#$%^&*-_=+`
    Full,
    /// Words joined by `-`.
    Passphrase,
    /// Digits only (weak by nature).
    Pin,
}

impl From<ProfileKind> for core::Profile {
    fn from(p: ProfileKind) -> Self {
        match p {
            ProfileKind::Alnum => core::Profile::Alnum,
            ProfileKind::Full => core::Profile::Full,
            ProfileKind::Passphrase => core::Profile::Passphrase,
            ProfileKind::Pin => core::Profile::Pin,
        }
    }
}

/// Length limits of a profile (characters, or words for `Passphrase`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileInfo {
    /// Default length.
    pub default_length: u32,
    /// Minimum allowed length.
    pub min_length: u32,
    /// Maximum allowed length.
    pub max_length: u32,
}

/// Non-secret information about the unlocked key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    /// 8 hex characters; identical on every device that holds the same key.
    pub key_id: String,
}

static SESSION: Mutex<Option<Arc<core::KeySeed>>> = Mutex::new(None);

fn set_session(k: core::KeySeed) -> KeyInfo {
    let info = KeyInfo { key_id: k.key_id() };
    *SESSION.lock().unwrap_or_else(|p| p.into_inner()) = Some(Arc::new(k));
    info
}

fn session() -> Result<Arc<core::KeySeed>, PwdxError> {
    SESSION
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
        .ok_or_else(PwdxError::locked)
}

/// Spec version implemented by the core (for the About screen).
#[frb(sync)]
pub fn spec_version() -> String {
    core::SPEC_VERSION.to_string()
}

/// Length limits for a profile.
#[frb(sync)]
pub fn profile_info(profile: ProfileKind) -> ProfileInfo {
    let p: core::Profile = profile.into();
    let (min, max) = p.length_range();
    ProfileInfo {
        default_length: p.default_length() as u32,
        min_length: min as u32,
        max_length: max as u32,
    }
}

/// Run the embedded known-answer check. The app calls this at start-up and refuses to derive on failure.
pub fn self_test() -> Result<(), PwdxError> {
    core::self_test().map_err(Into::into)
}

/// Show which domain a site will be keyed on (so the user can see what is actually used).
#[frb(sync)]
pub fn normalize_site(site: String) -> Result<String, PwdxError> {
    core::normalize_site(&site).map_err(Into::into)
}

/// Generate a new random key and unlock it.
pub fn create_key() -> Result<KeyInfo, PwdxError> {
    Ok(set_session(core::KeySeed::generate()?))
}

/// Import an OpenSSH `id_ed25519` file (plain or passphrase-protected) and unlock it.
/// `passphrase` is raw bytes as typed. Returns `PassphraseRequired` if the key is encrypted and none was given.
pub fn import_ssh_key(file: Vec<u8>, passphrase: Option<Vec<u8>>) -> Result<KeyInfo, PwdxError> {
    let pass = passphrase.map(Zeroizing::new);
    let k = core::import_openssh(&file, pass.as_ref().map(|p| p.as_slice()))?;
    Ok(set_session(k))
}

/// Unlock from an encrypted backup file (107 bytes) and its passphrase.
pub fn unlock_with_backup(backup: Vec<u8>, passphrase: String) -> Result<KeyInfo, PwdxError> {
    let pass = Zeroizing::new(passphrase);
    Ok(set_session(core::import_backup(&backup, &pass)?))
}

/// Unlock from raw key bytes read out of the OS keystore (exactly 32 bytes). Dart should zero its copy afterwards.
pub fn unlock_with_key_bytes(key: Vec<u8>) -> Result<KeyInfo, PwdxError> {
    let key = Zeroizing::new(key);
    let arr: [u8; 32] = key
        .as_slice()
        .try_into()
        .map_err(|_| PwdxError::new(ErrorKind::InvalidInput, "key must be 32 bytes"))?;
    Ok(set_session(core::KeySeed::from_bytes(arr)))
}

/// Encrypt the unlocked key into a backup/transfer file under a (strong, separate) passphrase.
pub fn export_backup_file(passphrase: String) -> Result<Vec<u8>, PwdxError> {
    let pass = Zeroizing::new(passphrase);
    Ok(core::export_backup(&*session()?, &pass)?)
}

/// The raw key bytes, for storing in the OS keystore (behind biometrics). Dart must zero its copy after storing.
pub fn export_key_bytes_for_keystore() -> Result<Vec<u8>, PwdxError> {
    Ok(core::key_bytes(&*session()?).to_vec())
}

/// Forget the unlocked key (the memory is zeroized once any in-flight derivation finishes).
#[frb(sync)]
pub fn lock() {
    *SESSION.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

/// Whether a key is currently unlocked.
#[frb(sync)]
pub fn is_unlocked() -> bool {
    SESSION.lock().unwrap_or_else(|p| p.into_inner()).is_some()
}

/// Key ID of the unlocked key, if any.
#[frb(sync)]
pub fn current_key_id() -> Option<String> {
    SESSION
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .map(|k| k.key_id())
}

/// Derive the password for one site. Normalization and validation happen in the core.
pub fn derive_password(
    secret: String,
    site: String,
    user: String,
    counter: u32,
    profile: ProfileKind,
    length: u32,
) -> Result<String, PwdxError> {
    let secret = Zeroizing::new(secret);
    let key = session()?;
    let req = core::SiteRequest {
        site: &site,
        user: &user,
        counter,
        profile: profile.into(),
        length: length as usize,
    };
    let pw = core::derive(&key, &secret, &req)?;
    Ok(pw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The session is process-global, so tests that touch it must not run concurrently.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn vectors() -> serde_json::Value {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testvectors/v1.json");
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn profile(s: &str) -> ProfileKind {
        match s {
            "alnum" => ProfileKind::Alnum,
            "full" => ProfileKind::Full,
            "passphrase" => ProfileKind::Passphrase,
            "pin" => ProfileKind::Pin,
            _ => panic!("profile"),
        }
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn every_vector_passes_through_the_ffi_api() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let v = vectors();
        for c in v["vectors"].as_array().unwrap() {
            unlock_with_key_bytes(hex(c["key_hex"].as_str().unwrap())).unwrap();
            let got = derive_password(
                c["secret"].as_str().unwrap().into(),
                c["site"].as_str().unwrap().into(),
                c["user"].as_str().unwrap().into(),
                c["counter"].as_u64().unwrap() as u32,
                profile(c["profile"].as_str().unwrap()),
                c["length"].as_u64().unwrap() as u32,
            )
            .unwrap();
            assert_eq!(got, c["expected"].as_str().unwrap(), "vector {}", c["id"]);
        }
        lock();
    }

    #[test]
    fn session_lifecycle_and_errors() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        lock();
        assert!(!is_unlocked());
        assert_eq!(current_key_id(), None);
        let e = derive_password(
            "pw pw pw".into(),
            "example.com".into(),
            "u".into(),
            1,
            ProfileKind::Full,
            24,
        )
        .unwrap_err();
        assert_eq!(e.kind, ErrorKind::Locked);
        assert_eq!(
            export_backup_file("long enough pass".into())
                .unwrap_err()
                .kind,
            ErrorKind::Locked
        );

        let info = create_key().unwrap();
        assert_eq!(info.key_id.len(), 8);
        assert!(is_unlocked());
        assert_eq!(current_key_id(), Some(info.key_id.clone()));

        // keystore round trip
        let bytes = export_key_bytes_for_keystore().unwrap();
        assert_eq!(bytes.len(), 32);
        lock();
        assert_eq!(unlock_with_key_bytes(bytes).unwrap(), info);
        assert_eq!(
            unlock_with_key_bytes(vec![0u8; 31]).unwrap_err().kind,
            ErrorKind::InvalidInput
        );

        // backup round trip (default Argon2 parameters)
        let file = export_backup_file("a good backup passphrase".into()).unwrap();
        lock();
        assert_eq!(
            unlock_with_backup(file.clone(), "wrong passphrase!".into())
                .unwrap_err()
                .kind,
            ErrorKind::Decrypt
        );
        assert_eq!(
            unlock_with_backup(file, "a good backup passphrase".into()).unwrap(),
            info
        );

        // input validation surfaces as typed errors
        let e = derive_password(
            "pw pw pw".into(),
            "example.com".into(),
            "u".into(),
            0,
            ProfileKind::Full,
            24,
        )
        .unwrap_err();
        assert_eq!(e.kind, ErrorKind::InvalidInput);
        lock();
    }

    #[test]
    fn ssh_import_through_ffi() {
        let _g = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/core/tests/fixtures"
        );
        let protected = std::fs::read(format!("{dir}/protected")).unwrap();
        let plain = std::fs::read(format!("{dir}/plain")).unwrap();
        assert_eq!(
            import_ssh_key(protected.clone(), None).unwrap_err().kind,
            ErrorKind::PassphraseRequired
        );
        assert_eq!(
            import_ssh_key(protected.clone(), Some(b"nope".to_vec()))
                .unwrap_err()
                .kind,
            ErrorKind::Decrypt
        );
        let a = import_ssh_key(protected, Some(b"hunter2".to_vec())).unwrap();
        let b = import_ssh_key(plain, None).unwrap();
        assert_eq!(a, b);
        lock();
    }

    #[test]
    fn metadata_helpers() {
        assert_eq!(spec_version(), "v1");
        assert_eq!(
            profile_info(ProfileKind::Full),
            ProfileInfo {
                default_length: 24,
                min_length: 8,
                max_length: 128
            }
        );
        assert_eq!(profile_info(ProfileKind::Pin).max_length, 12);
        assert_eq!(
            normalize_site("HTTPS://WWW.Example.COM/x".into()).unwrap(),
            "example.com"
        );
        self_test().unwrap();
    }
}
