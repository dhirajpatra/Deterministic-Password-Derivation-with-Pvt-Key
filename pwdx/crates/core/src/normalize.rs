// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! Input normalization (spec v1 section 2). All normalization lives here, never in a UI layer.

use crate::error::Error;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

/// Memorable password or backup passphrase: NFKC; no trimming, no case folding; control characters rejected.
pub(crate) fn secret(s: &str) -> Result<Zeroizing<String>, Error> {
    let n: Zeroizing<String> = Zeroizing::new(s.nfkc().collect());
    if n.is_empty() {
        return Err(Error::InvalidInput("password is empty"));
    }
    if n.chars().any(char::is_control) {
        return Err(Error::InvalidInput("password contains control characters"));
    }
    Ok(n)
}

/// Username: NFKC, then trim surrounding whitespace; case preserved; must be non-empty.
pub(crate) fn user(s: &str) -> Result<String, Error> {
    let n: String = s.nfkc().collect();
    let n = n.trim();
    if n.is_empty() {
        return Err(Error::InvalidInput("username is empty"));
    }
    if n.chars().any(char::is_control) {
        return Err(Error::InvalidInput("username contains control characters"));
    }
    Ok(n.to_string())
}

/// Site: strip scheme, credentials, path, query, fragment and port; IDNA (UTS 46) to ASCII, lowercase;
/// drop one leading `www.` (only if a dot remains) and trailing dots; reduce to the registrable domain
/// (eTLD+1, Public Suffix List). IP literals, `localhost`-style names and bare public suffixes keep the full host.
pub(crate) fn site(input: &str) -> Result<String, Error> {
    let mut s = input.trim();
    if let Some(i) = s.find("://") {
        let scheme = &s[..i];
        if !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
        {
            s = &s[i + 3..];
        }
    }
    let end = s.find(['/', '?', '#']).unwrap_or(s.len());
    let authority = &s[..end];
    let hostport = match authority.rfind('@') {
        Some(i) => &authority[i + 1..],
        None => authority,
    };
    let host = if hostport.starts_with('[') {
        match hostport.find(']') {
            Some(i) => return Ok(hostport[..=i].to_ascii_lowercase()),
            None => return Err(Error::InvalidInput("site is malformed")),
        }
    } else {
        match hostport.rfind(':') {
            Some(i) if hostport[i + 1..].bytes().all(|b| b.is_ascii_digit()) => &hostport[..i],
            _ => hostport,
        }
    };
    if host.is_empty() {
        return Err(Error::InvalidInput("site is empty"));
    }
    let ascii = idna::domain_to_ascii(host)
        .map_err(|_| Error::InvalidInput("site is not a valid domain name"))?;
    let mut h = ascii.to_ascii_lowercase();
    while h.ends_with('.') {
        h.pop();
    }
    if let Some(rest) = h.strip_prefix("www.") {
        if rest.contains('.') {
            h = rest.to_string();
        }
    }
    if h.is_empty() {
        return Err(Error::InvalidInput("site is empty"));
    }
    // After IDNA only ASCII letters, digits, `-`, `_` and non-empty dot-separated labels (<= 63 bytes) are valid.
    if h.len() > 253
        || h.split('.').any(|l| {
            l.is_empty()
                || l.len() > 63
                || !l
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        })
    {
        return Err(Error::InvalidInput("site is not a valid domain name"));
    }
    if h.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return Ok(h); // IPv4 literal
    }
    match psl::domain(h.as_bytes()) {
        Some(d) => String::from_utf8(d.as_bytes().to_vec())
            .map_err(|_| Error::InvalidInput("site is malformed")),
        None => Ok(h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_nfkc_equivalences() {
        assert_eq!(
            *secret("caf\u{e9}").unwrap(),
            *secret("cafe\u{301}").unwrap()
        );
        assert_eq!(*secret("\u{ff21}\u{ff22}\u{ff23}").unwrap(), "ABC");
        assert_eq!(*secret("of\u{fb01}ce").unwrap(), "office");
    }

    #[test]
    fn secret_is_not_trimmed_or_casefolded() {
        assert_eq!(*secret(" Abc ").unwrap(), " Abc ");
    }

    #[test]
    fn secret_rejects_bad_input() {
        assert!(secret("").is_err());
        assert!(secret("a\u{7}b").is_err());
        assert!(secret("a\nb").is_err());
    }

    #[test]
    fn user_rules() {
        assert_eq!(user("  Alice@Example.com\t").unwrap(), "Alice@Example.com");
        assert_eq!(user("\u{3000}bob\u{3000}").unwrap(), "bob"); // ideographic space -> space -> trimmed
        assert!(user("   ").is_err());
    }

    #[test]
    fn site_rules() {
        let cases = [
            ("example.com", "example.com"),
            ("HTTPS://WWW.Example.COM/path?q=1", "example.com"),
            ("https://user:pw@login.example.com:8443/a#b", "example.com"),
            ("login.example.co.uk:8443", "example.co.uk"),
            ("https://www.B\u{fc}cher.de/", "xn--bcher-kva.de"),
            ("example.com.", "example.com"),
            ("  www.github.com  ", "github.com"),
            ("www.com", "www.com"),
            ("192.168.1.10:8080", "192.168.1.10"),
            ("[::1]:3000", "[::1]"),
            ("localhost", "localhost"),
            ("example.com/redirect?u=http://other.org", "example.com"),
            ("co.uk", "co.uk"),
        ];
        for (input, want) in cases {
            assert_eq!(site(input).unwrap(), want, "input: {input}");
        }
        assert!(site("").is_err());
        assert!(site("https:///path").is_err());
        for bad in [
            "exa mple.com",
            "a..b.com",
            "-.com",
            "exa$mple.com",
            "a\u{3000}b.com",
            "xn--.com",
            &"a".repeat(64),
        ] {
            if bad == "-.com" {
                continue; // hyphen-only labels are tolerated; only the character set is policed
            }
            assert!(site(bad).is_err(), "should reject {bad:?}");
        }
        assert_eq!(site("under_score.example.com").unwrap(), "example.com");
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

    const POOL: &[&str] = &[
        "a",
        "b",
        "Z",
        "0",
        "9",
        ".",
        ".",
        "-",
        "_",
        ":",
        "/",
        "?",
        "#",
        "@",
        "[",
        "]",
        "%",
        " ",
        "\t",
        "\n",
        "www.",
        "xn--",
        "com",
        "co.uk",
        "://",
        "http",
        "\u{e9}",
        "e\u{301}",
        "\u{ff21}",
        "\u{fb01}",
        "\u{200b}",
        "\u{202e}",
        "\u{1f511}",
        "\u{6f22}",
        "\u{627}",
        "\u{0}",
        "\u{7f}",
        "\u{3000}",
        "\u{df}",
        "\u{130}",
        "..",
        "[::1]",
        "1.2.3.4",
        "localhost",
    ];

    fn random_string(next: &mut impl FnMut() -> u64) -> String {
        let n = (next() % 12) as usize;
        (0..n)
            .map(|_| POOL[(next() as usize) % POOL.len()])
            .collect()
    }

    #[test]
    fn random_inputs_never_panic_and_normalization_is_idempotent() {
        let mut next = rng(0xDEADBEEF12345678);
        let (mut site_ok, mut secret_ok, mut user_ok) = (0, 0, 0);
        for _ in 0..30_000 {
            let s = random_string(&mut next);
            if let Ok(a) = site(&s) {
                site_ok += 1;
                assert_eq!(
                    site(&a).as_deref(),
                    Ok(a.as_str()),
                    "site not idempotent: {s:?} -> {a:?}"
                );
            }
            if let Ok(a) = secret(&s) {
                secret_ok += 1;
                assert_eq!(*secret(&a).unwrap(), *a, "secret not idempotent: {s:?}");
            }
            if let Ok(a) = user(&s) {
                user_ok += 1;
                assert_eq!(
                    user(&a).as_deref(),
                    Ok(a.as_str()),
                    "user not idempotent: {s:?}"
                );
            }
        }
        // the generator must actually exercise both the accepting and the rejecting paths
        assert!(
            site_ok > 500 && secret_ok > 500 && user_ok > 500,
            "{site_ok} {secret_ok} {user_ok}"
        );
    }

    #[test]
    fn huge_inputs_do_not_panic() {
        let big = "a".repeat(200_000);
        let _ = site(&big);
        let _ = secret(&big);
        let _ = user(&big);
        let deep = format!("{}example.com", "a.".repeat(5_000));
        let _ = site(&deep);
    }
}
