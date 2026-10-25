//! Derivation and encoding (spec v1 sections 3-6).

use crate::error::Error;
use crate::key::KeySeed;
use crate::normalize;
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};
use std::sync::OnceLock;
use zeroize::Zeroizing;

const UPPER: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*-_=+";

const ALNUM_CLASSES: &[&[u8]] = &[UPPER, LOWER, DIGITS];
const FULL_CLASSES: &[&[u8]] = &[UPPER, LOWER, DIGITS, SYMBOLS];

/// Frozen v1 Argon2id parameters. Never lowered, on any platform.
const ARGON_M_KIB: u32 = 65536;
const ARGON_T: u32 = 3;
const ARGON_P: u32 = 1;
const STREAM_LEN: usize = 1024;

static WORDLIST_RAW: &str = include_str!("../data/eff_large_wordlist.txt");

fn words() -> &'static [&'static str] {
    static W: OnceLock<Vec<&'static str>> = OnceLock::new();
    W.get_or_init(|| WORDLIST_RAW.lines().filter(|l| !l.is_empty()).collect())
}

/// Output profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// `A-Z a-z 0-9`
    Alnum,
    /// `A-Z a-z 0-9 !@#$%^&*-_=+`
    Full,
    /// Words from the EFF large list joined by `-`.
    Passphrase,
    /// Digits only (weak by nature).
    Pin,
}

impl Profile {
    /// Stable profile id used inside the derivation.
    pub fn id(self) -> &'static str {
        match self {
            Profile::Alnum => "alnum",
            Profile::Full => "full",
            Profile::Passphrase => "passphrase",
            Profile::Pin => "pin",
        }
    }

    /// Parse a profile id.
    pub fn parse(s: &str) -> Option<Profile> {
        match s {
            "alnum" => Some(Profile::Alnum),
            "full" => Some(Profile::Full),
            "passphrase" => Some(Profile::Passphrase),
            "pin" => Some(Profile::Pin),
            _ => None,
        }
    }

    /// Inclusive allowed length (characters, or words for `Passphrase`).
    pub fn length_range(self) -> (usize, usize) {
        match self {
            Profile::Alnum | Profile::Full => (8, 128),
            Profile::Passphrase => (4, 16),
            Profile::Pin => (4, 12),
        }
    }

    /// Default length.
    pub fn default_length(self) -> usize {
        match self {
            Profile::Alnum | Profile::Full => 24,
            Profile::Passphrase => 8,
            Profile::Pin => 6,
        }
    }

    fn classes(self) -> &'static [&'static [u8]] {
        match self {
            Profile::Alnum => ALNUM_CLASSES,
            Profile::Full => FULL_CLASSES,
            _ => &[],
        }
    }
}

/// What to derive a password for.
#[derive(Clone, Copy, Debug)]
pub struct SiteRequest<'a> {
    /// Site as the user typed it (normalized inside the core).
    pub site: &'a str,
    /// Username or email for the site.
    pub user: &'a str,
    /// Rotation counter, starting at 1.
    pub counter: u32,
    /// Output profile.
    pub profile: Profile,
    /// Length in characters (words for `Passphrase`).
    pub length: usize,
}

/// Sequential byte source with a single, never-reset cursor.
pub(crate) struct Stream {
    buf: Zeroizing<Vec<u8>>,
    pos: usize,
}

impl Stream {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Stream {
            buf: Zeroizing::new(bytes),
            pos: 0,
        }
    }

    fn next_u8(&mut self) -> Result<u8, Error> {
        let b = *self.buf.get(self.pos).ok_or(Error::StreamExhausted)?;
        self.pos += 1;
        Ok(b)
    }

    /// Uniform integer in `[0, n)` by rejection sampling, `1 <= n <= 65536`.
    pub(crate) fn pick(&mut self, n: usize) -> Result<usize, Error> {
        debug_assert!((1..=65536).contains(&n));
        if n <= 256 {
            let limit = 256 - (256 % n);
            loop {
                let b = self.next_u8()? as usize;
                if b < limit {
                    return Ok(b % n);
                }
            }
        } else {
            let limit = 65536 - (65536 % n);
            loop {
                let hi = self.next_u8()? as usize;
                let lo = self.next_u8()? as usize;
                let v = (hi << 8) | lo;
                if v < limit {
                    return Ok(v % n);
                }
            }
        }
    }
}

fn lp(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
}

/// Derive a password. All normalization happens here.
pub fn derive(
    key: &KeySeed,
    secret: &str,
    req: &SiteRequest<'_>,
) -> Result<Zeroizing<String>, Error> {
    if req.counter == 0 {
        return Err(Error::InvalidInput("counter must be at least 1"));
    }
    let (lo, hi) = req.profile.length_range();
    if req.length < lo || req.length > hi {
        return Err(Error::InvalidInput(
            "length is out of range for this profile",
        ));
    }
    let s = normalize::secret(secret)?;
    let site = normalize::site(req.site)?;
    let user = normalize::user(req.user)?;

    let mut stream = derive_stream(
        key,
        s.as_bytes(),
        &site,
        &user,
        req.counter,
        req.profile,
        req.length,
    )?;
    encode(&mut stream, req.profile, req.length)
}

fn derive_stream(
    key: &KeySeed,
    secret_nfkc: &[u8],
    site: &str,
    user: &str,
    counter: u32,
    profile: Profile,
    length: usize,
) -> Result<Stream, Error> {
    // salt = SHA-256("pwdx/v1/salt" || K)   (the seed, never the public key)
    let salt = {
        let mut h = Sha256::new();
        h.update(b"pwdx/v1/salt");
        h.update(key.as_bytes());
        h.finalize()
    };

    let params = Params::new(ARGON_M_KIB, ARGON_T, ARGON_P, Some(32)).map_err(|_| Error::Kdf)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut stretch = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(secret_nfkc, salt.as_slice(), &mut *stretch)
        .map_err(|_| Error::Kdf)?;

    // master = HMAC-SHA512(K, "pwdx/v1/master" || stretch)
    let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(key.as_bytes()).map_err(|_| Error::Kdf)?;
    mac.update(b"pwdx/v1/master");
    mac.update(&*stretch);
    let mut master = Zeroizing::new([0u8; 64]);
    master.copy_from_slice(&mac.finalize().into_bytes());

    // info = "pwdx/v1/site" || LP(site) || LP(user) || u32_be(counter) || LP("<profile>:<length>")
    let mut info = Vec::new();
    info.extend_from_slice(b"pwdx/v1/site");
    lp(&mut info, site.as_bytes());
    lp(&mut info, user.as_bytes());
    info.extend_from_slice(&counter.to_be_bytes());
    lp(&mut info, format!("{}:{}", profile.id(), length).as_bytes());

    let hk = Hkdf::<Sha512>::from_prk(&*master).map_err(|_| Error::Kdf)?;
    let mut okm = vec![0u8; STREAM_LEN];
    hk.expand(&info, &mut okm).map_err(|_| Error::Kdf)?;
    Ok(Stream::new(okm))
}

pub(crate) fn encode(
    stream: &mut Stream,
    profile: Profile,
    length: usize,
) -> Result<Zeroizing<String>, Error> {
    let mut out = Zeroizing::new(String::new());
    match profile {
        Profile::Alnum | Profile::Full => {
            let classes = profile.classes();
            let full: Vec<u8> = classes.concat();
            let mut chars: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(length));
            for class in classes {
                chars.push(class[stream.pick(class.len())?]);
            }
            while chars.len() < length {
                chars.push(full[stream.pick(full.len())?]);
            }
            for i in (1..length).rev() {
                let j = stream.pick(i + 1)?;
                chars.swap(i, j);
            }
            out.extend(chars.iter().map(|&b| b as char));
        }
        Profile::Passphrase => {
            let w = words();
            for i in 0..length {
                if i > 0 {
                    out.push('-');
                }
                out.push_str(w[stream.pick(w.len())?]);
            }
        }
        Profile::Pin => {
            for _ in 0..length {
                out.push(DIGITS[stream.pick(10)?] as char);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordlist_is_pinned() {
        assert_eq!(words().len(), 7776);
        let d = Sha256::digest(WORDLIST_RAW.as_bytes());
        let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "6d557f0693958fb5e650b68b5bee585eb82cf4da32965505c789e924743bc522"
        );
    }

    #[test]
    fn pick_rejects_at_limit_and_consumes_in_order() {
        // n = 10: limit = 250. 250..=255 are rejected, 249 accepted -> 9.
        let mut s = Stream::new(vec![255, 250, 251, 249, 7]);
        assert_eq!(s.pick(10).unwrap(), 9);
        assert_eq!(s.pick(10).unwrap(), 7);
        // n = 256: nothing is ever rejected.
        let mut s = Stream::new(vec![255]);
        assert_eq!(s.pick(256).unwrap(), 255);
        // n = 7776 uses two bytes: limit = 65536 - 3328 = 62208 = 0xF300.
        let mut s = Stream::new(vec![0xF3, 0x00, 0xF2, 0xFF]);
        assert_eq!(s.pick(7776).unwrap(), 0xF2FF % 7776);
        // n = 1 always yields 0 but still consumes a byte.
        let mut s = Stream::new(vec![9, 9]);
        assert_eq!(s.pick(1).unwrap(), 0);
        assert_eq!(s.pick(1).unwrap(), 0);
        assert_eq!(s.pick(1), Err(Error::StreamExhausted));
    }

    #[test]
    fn stream_exhaustion_is_an_error() {
        let mut s = Stream::new(vec![255, 255]);
        assert_eq!(s.pick(10), Err(Error::StreamExhausted));
    }

    #[test]
    fn encoding_meets_class_rules_for_many_streams() {
        // Pseudo-random but deterministic streams (no KDF needed): checks classes and length only.
        let mut x: u64 = 0x1234_5678_9abc_def0;
        for _ in 0..2000 {
            let mut bytes = Vec::with_capacity(1024);
            for _ in 0..1024 {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                bytes.push((x >> 24) as u8);
            }
            for (profile, len) in [
                (Profile::Full, 8),
                (Profile::Full, 24),
                (Profile::Alnum, 8),
                (Profile::Alnum, 24),
            ] {
                let mut st = Stream::new(bytes.clone());
                let p = encode(&mut st, profile, len).unwrap();
                assert_eq!(p.len(), len);
                assert!(p.bytes().any(|b| UPPER.contains(&b)));
                assert!(p.bytes().any(|b| LOWER.contains(&b)));
                assert!(p.bytes().any(|b| DIGITS.contains(&b)));
                if profile == Profile::Full {
                    assert!(p.bytes().any(|b| SYMBOLS.contains(&b)));
                } else {
                    assert!(p.bytes().all(|b| b.is_ascii_alphanumeric()));
                }
            }
        }
    }

    #[test]
    fn full_symbol_set_is_frozen() {
        assert_eq!(SYMBOLS, b"!@#$%^&*-_=+");
        assert_eq!(FULL_CLASSES.concat().len(), 74);
    }
}

#[cfg(test)]
mod bias_tests {
    //! Statistical checks that the encoder is unbiased. Inputs come from a deterministic hash-based generator,
    //! so results are reproducible (never flaky). Thresholds are about 5 standard deviations.

    use super::*;

    /// Deterministic byte generator: SHA-512 in counter mode.
    fn bytes(seed: u64, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len + 64);
        let mut ctr: u64 = 0;
        while out.len() < len {
            let mut h = Sha512::new();
            h.update(seed.to_be_bytes());
            h.update(ctr.to_be_bytes());
            out.extend_from_slice(&h.finalize());
            ctr += 1;
        }
        out.truncate(len);
        out
    }

    /// Standardized chi-square statistic: (chi2 - df) / sqrt(2 df).
    fn z_chi2(observed: &[f64], expected: &[f64]) -> f64 {
        let chi2: f64 = observed
            .iter()
            .zip(expected)
            .map(|(o, e)| (o - e) * (o - e) / e)
            .sum();
        let df = (observed.len() - 1) as f64;
        (chi2 - df) / (2.0 * df).sqrt()
    }

    fn uniform_z(n: usize, samples: usize, biased: bool) -> f64 {
        let width = if n <= 256 { 1 } else { 2 };
        let mut st = Stream::new(bytes(n as u64, samples * width * 2 + 4096));
        let mut counts = vec![0f64; n];
        for _ in 0..samples {
            let v = if biased {
                // deliberately wrong: plain modulo without rejection
                let b = st.next_u8().unwrap() as usize;
                b % n
            } else {
                st.pick(n).unwrap()
            };
            counts[v] += 1.0;
        }
        z_chi2(&counts, &vec![samples as f64 / n as f64; n])
    }

    #[test]
    fn pick_is_uniform_for_every_size_we_use() {
        for n in [2, 3, 7, 10, 12, 26, 62, 74, 100, 255, 256, 257, 1000, 7776] {
            let z = uniform_z(n, 400_000, false);
            assert!(z.abs() < 5.0, "pick({n}) looks non-uniform: z = {z:.2}");
        }
    }

    #[test]
    fn the_test_has_power_a_modulo_biased_pick_is_detected() {
        // Control experiment: without rejection sampling these sizes are clearly biased.
        for n in [10, 62, 74, 100] {
            let z = uniform_z(n, 400_000, true);
            assert!(z > 5.0, "modulo bias at n={n} was not detected: z = {z:.2}");
        }
    }

    fn password_stream(seed: u64, i: u64) -> Stream {
        Stream::new(bytes(seed ^ (i << 20), 512))
    }

    #[test]
    fn full_profile_character_frequencies_match_theory() {
        // Each class contributes one guaranteed char; the remaining L-4 come from the 74-char alphabet.
        let (n, l) = (60_000u64, 24usize);
        let classes = FULL_CLASSES;
        let full: Vec<u8> = classes.concat();
        let mut counts = std::collections::HashMap::new();
        for i in 0..n {
            let mut st = password_stream(11, i);
            let p = encode(&mut st, Profile::Full, l).unwrap();
            for b in p.bytes() {
                *counts.entry(b).or_insert(0f64) += 1.0;
            }
        }
        let (mut obs, mut exp) = (vec![], vec![]);
        for class in classes {
            for &c in *class {
                obs.push(*counts.get(&c).unwrap_or(&0.0));
                exp.push(
                    n as f64
                        * (1.0 / class.len() as f64
                            + (l - classes.len()) as f64 / full.len() as f64),
                );
            }
        }
        let z = z_chi2(&obs, &exp);
        assert!(
            z.abs() < 5.0,
            "character frequencies deviate from theory: z = {z:.2}"
        );
    }

    #[test]
    fn no_position_is_predictable() {
        // After the shuffle, every position must show the same class mix (no "first char is uppercase" pattern).
        let (n, l) = (60_000u64, 24usize);
        let sizes = [26f64, 26.0, 10.0, 12.0];
        let rand_chars = (l - 4) as f64;
        let mut by_pos = vec![[0f64; 4]; l];
        for i in 0..n {
            let mut st = password_stream(23, i);
            let p = encode(&mut st, Profile::Full, l).unwrap();
            for (pos, b) in p.bytes().enumerate() {
                let class = if b.is_ascii_uppercase() {
                    0
                } else if b.is_ascii_lowercase() {
                    1
                } else if b.is_ascii_digit() {
                    2
                } else {
                    3
                };
                by_pos[pos][class] += 1.0;
            }
        }
        let mut worst: f64 = 0.0;
        for row in &by_pos {
            for (c, &obs) in row.iter().enumerate() {
                let frac = (1.0 + rand_chars * sizes[c] / 74.0) / l as f64;
                let (mean, sd) = (n as f64 * frac, (n as f64 * frac * (1.0 - frac)).sqrt());
                worst = worst.max(((obs - mean) / sd).abs());
            }
        }
        assert!(
            worst < 5.0,
            "some position favours a class: worst |z| = {worst:.2}"
        );
    }

    #[test]
    fn pin_and_passphrase_are_uniform() {
        let n = 80_000u64;
        let mut digits = [0f64; 10];
        for i in 0..n {
            let mut st = password_stream(31, i);
            for b in encode(&mut st, Profile::Pin, 6).unwrap().bytes() {
                digits[(b - b'0') as usize] += 1.0;
            }
        }
        let z = z_chi2(&digits, &[n as f64 * 6.0 / 10.0; 10]);
        assert!(z.abs() < 5.0, "PIN digits non-uniform: z = {z:.2}");

        // word indices: tally by first letter vs. the wordlist's own letter distribution
        let w = words();
        let mut first = [0f64; 26];
        let mut expect = [0f64; 26];
        for word in w {
            expect[(word.as_bytes()[0] - b'a') as usize] += 1.0;
        }
        let draws = 50_000u64;
        for i in 0..draws {
            let mut st = password_stream(41, i);
            let p = encode(&mut st, Profile::Passphrase, 4).unwrap();
            for word in p.split('-').filter(|x| !x.is_empty()) {
                first[(word.as_bytes()[0] - b'a') as usize] += 1.0;
            }
        }
        // hyphenated words (drop-down, felt-tip, t-shirt, yo-yo) split into extra fragments; exclude such draws' effect
        // by comparing only letters with enough mass and a loose bound.
        let total: f64 = first.iter().sum();
        let (mut o, mut e) = (vec![], vec![]);
        for l in 0..26 {
            if expect[l] > 0.0 {
                o.push(first[l]);
                e.push(total * expect[l] / 7776.0);
            }
        }
        let z = z_chi2(&o, &e);
        assert!(z.abs() < 6.0, "passphrase words non-uniform: z = {z:.2}");
    }
}
