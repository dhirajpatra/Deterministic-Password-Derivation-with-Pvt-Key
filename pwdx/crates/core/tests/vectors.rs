//! Known-answer tests: every vector in testvectors/v1.json must reproduce exactly.
//! The expected values were produced by the independent Python reference (testvectors/reference.py).

use pwdx_core::{derive, normalize_site, KeySeed, Profile, SiteRequest, SELF_TEST_EXPECTED};
use serde::Deserialize;

#[derive(Deserialize)]
struct File {
    spec: String,
    vectors: Vec<Vector>,
    errors: Vec<ErrCase>,
}

#[derive(Deserialize)]
struct Vector {
    id: String,
    key_hex: String,
    secret: String,
    site: String,
    site_normalized: String,
    user: String,
    counter: u32,
    profile: String,
    length: usize,
    expected: String,
}

#[derive(Deserialize)]
struct ErrCase {
    id: String,
    key_hex: String,
    secret: String,
    site: String,
    user: String,
    counter: u32,
    profile: String,
    length: usize,
}

fn load() -> File {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../testvectors/v1.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read v1.json"))
        .expect("parse v1.json")
}

#[test]
fn all_vectors_match() {
    let f = load();
    assert_eq!(f.spec, "v1");
    assert!(f.vectors.len() >= 30);
    for v in &f.vectors {
        let key = KeySeed::from_hex(&v.key_hex).unwrap();
        let profile = Profile::parse(&v.profile).unwrap();
        let req = SiteRequest {
            site: &v.site,
            user: &v.user,
            counter: v.counter,
            profile,
            length: v.length,
        };
        let got = derive(&key, &v.secret, &req).unwrap_or_else(|e| panic!("{}: {e}", v.id));
        assert_eq!(*got, v.expected, "vector {}", v.id);
        assert_eq!(
            normalize_site(&v.site).unwrap(),
            v.site_normalized,
            "site normalization of {}",
            v.id
        );
    }
}

#[test]
fn error_cases_are_rejected() {
    for e in &load().errors {
        let key = KeySeed::from_hex(&e.key_hex).unwrap();
        let profile = Profile::parse(&e.profile).unwrap();
        let req = SiteRequest {
            site: &e.site,
            user: &e.user,
            counter: e.counter,
            profile,
            length: e.length,
        };
        assert!(
            derive(&key, &e.secret, &req).is_err(),
            "case {} must be rejected",
            e.id
        );
    }
}

#[test]
fn self_test_constant_matches_vector_file() {
    let f = load();
    let v = f
        .vectors
        .iter()
        .find(|v| v.id == "full-default-24")
        .unwrap();
    assert_eq!(v.expected, SELF_TEST_EXPECTED);
    pwdx_core::self_test().unwrap();
}

#[test]
fn derivation_is_idempotent_and_input_sensitive() {
    let key = KeySeed::from_hex(&"11".repeat(32)).unwrap();
    let base = SiteRequest {
        site: "example.com",
        user: "u",
        counter: 1,
        profile: Profile::Full,
        length: 24,
    };
    let a = derive(&key, "a long enough passphrase", &base).unwrap();
    let b = derive(&key, "a long enough passphrase", &base).unwrap();
    assert_eq!(*a, *b);
    let other_key = KeySeed::from_hex(&"22".repeat(32)).unwrap();
    assert_ne!(
        *a,
        *derive(&other_key, "a long enough passphrase", &base).unwrap()
    );
    assert_ne!(
        *a,
        *derive(&key, "a long enough passphrasf", &base).unwrap()
    );
    // length is part of the derivation, so a longer password is not an extension of a shorter one
    let longer = SiteRequest { length: 25, ..base };
    assert_ne!(
        a[..],
        derive(&key, "a long enough passphrase", &longer).unwrap()[..24]
    );
}
