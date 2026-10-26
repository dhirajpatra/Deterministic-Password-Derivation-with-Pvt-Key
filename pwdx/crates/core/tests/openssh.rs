// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! OpenSSH import against fixtures made with the real ssh-keygen. Expected seeds come from an independent
//! implementation (Python `cryptography`). The fixture keys are throwaway test keys.

use pwdx_core::{import_openssh, Error, KeySeed};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn expected_seed(name: &str) -> KeySeed {
    let hex = String::from_utf8(fixture(&format!("{name}.seed.hex"))).unwrap();
    KeySeed::from_hex(hex.trim()).unwrap()
}

#[test]
fn plain_key() {
    assert_eq!(
        import_openssh(&fixture("plain"), None).unwrap(),
        expected_seed("plain")
    );
}

#[test]
fn protected_key() {
    let k = import_openssh(&fixture("protected"), Some(b"hunter2")).unwrap();
    assert_eq!(k, expected_seed("protected"));
    assert_eq!(k, import_openssh(&fixture("plain"), None).unwrap()); // same key, same seed
}

#[test]
fn protected_key_errors() {
    assert_eq!(
        import_openssh(&fixture("protected"), None).unwrap_err(),
        Error::PassphraseRequired
    );
    assert_eq!(
        import_openssh(&fixture("protected"), Some(b"wrong")).unwrap_err(),
        Error::Decrypt
    );
}

#[test]
fn rejects_other_key_types_and_garbage() {
    assert!(matches!(
        import_openssh(&fixture("rsa"), None),
        Err(Error::Format(_))
    ));
    assert!(matches!(
        import_openssh(b"not a key", None),
        Err(Error::Format(_))
    ));
    let mut f = fixture("plain");
    let n = f.len();
    f.truncate(n / 2);
    assert!(import_openssh(&f, None).is_err());
}

#[test]
fn same_seed_derives_same_password_from_either_source() {
    use pwdx_core::{derive, Profile, SiteRequest};
    let req = SiteRequest {
        site: "example.com",
        user: "u",
        counter: 1,
        profile: Profile::Full,
        length: 24,
    };
    let a = derive(
        &import_openssh(&fixture("plain"), None).unwrap(),
        "some memorable passphrase",
        &req,
    )
    .unwrap();
    let b = derive(
        &import_openssh(&fixture("protected"), Some(b"hunter2")).unwrap(),
        "some memorable passphrase",
        &req,
    )
    .unwrap();
    assert_eq!(*a, *b);
}
