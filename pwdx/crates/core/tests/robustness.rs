// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! Mutation-based robustness tests for the parsers (a stable-Rust complement to the cargo-fuzz targets in `fuzz/`).
//! Properties: never panic; and if a mutated key file still parses, it must yield the *same* seed (integrity).

use pwdx_core::{import_openssh, KeySeed};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn rng(seed: u64) -> impl FnMut() -> u64 {
    let mut x = seed;
    move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    }
}

fn mutate(good: &[u8], next: &mut impl FnMut() -> u64) -> Vec<u8> {
    let mut f = good.to_vec();
    match next() % 5 {
        0 => {
            let i = (next() as usize) % f.len();
            f[i] ^= 1 << (next() % 8);
        }
        1 => {
            let n = (next() as usize) % f.len();
            f.truncate(n);
        }
        2 => {
            let i = (next() as usize) % f.len();
            f.insert(i, next() as u8);
        }
        3 => {
            let i = (next() as usize) % f.len();
            f.remove(i);
        }
        _ => {
            for _ in 0..4 {
                let i = (next() as usize) % f.len();
                f[i] = next() as u8;
            }
        }
    }
    f
}

#[test]
fn mutated_plain_ssh_keys_never_panic_or_change_the_seed() {
    let good = fixture("plain");
    let want =
        KeySeed::from_hex(String::from_utf8(fixture("plain.seed.hex")).unwrap().trim()).unwrap();
    let mut next = rng(0xC0FFEE);
    let mut accepted = 0;
    for _ in 0..20_000 {
        let f = mutate(&good, &mut next);
        if let Ok(k) = import_openssh(&f, None) {
            accepted += 1;
            assert_eq!(k, want, "a mutated key file yielded a different seed");
        }
    }
    // Some mutations (comment, padding, whitespace) legitimately still parse; that is fine.
    println!("mutations that still parsed: {accepted}");
}

#[test]
fn mutated_protected_ssh_keys_never_panic() {
    let good = fixture("protected");
    let want = KeySeed::from_hex(
        String::from_utf8(fixture("protected.seed.hex"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    let mut next = rng(0xBADC0DE);
    for _ in 0..40 {
        // bcrypt makes each decryption slow, so fewer iterations here
        let f = mutate(&good, &mut next);
        if let Ok(k) = import_openssh(&f, Some(b"hunter2")) {
            assert_eq!(k, want);
        }
    }
}

#[test]
fn arbitrary_garbage_never_panics() {
    let mut next = rng(0x1234_5678);
    for len in [0usize, 1, 7, 15, 100, 4096] {
        for _ in 0..200 {
            let f: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            let _ = import_openssh(&f, None);
            let _ = import_openssh(&f, Some(b"x"));
        }
    }
    let armored = b"-----BEGIN OPENSSH PRIVATE KEY-----\n\n-----END OPENSSH PRIVATE KEY-----\n";
    assert!(import_openssh(armored, None).is_err());
}
