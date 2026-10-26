// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! pwdx core: deterministic, offline password derivation (spec v1).
//!
//! One implementation, used by every platform. No network, no logging, no `unsafe`.
//! See `skills/pwdx-derivation-spec/SKILL.md` for the normative specification.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod backup;
mod derive;
mod error;
mod key;
mod normalize;
mod openssh;

pub use backup::{export_backup, import_backup, BACKUP_LEN};
pub use derive::{derive, Profile, SiteRequest};
pub use error::Error;
pub use key::{key_bytes, KeySeed};
pub use openssh::import_openssh;

/// Spec version implemented by this crate.
pub const SPEC_VERSION: &str = "v1";

/// Normalize a site exactly as `derive` does (useful for showing the user which domain is keyed).
pub fn normalize_site(site: &str) -> Result<String, Error> {
    normalize::site(site)
}

/// Run one embedded known-answer vector. Apps call this at start-up and refuse to derive if it fails,
/// which catches broken builds and toolchain bugs.
pub fn self_test() -> Result<(), Error> {
    let key = KeySeed::from_bytes(core::array::from_fn(|i| i as u8));
    let req = SiteRequest {
        site: "example.com",
        user: "alice@example.com",
        counter: 1,
        profile: Profile::Full,
        length: 24,
    };
    let got = derive(&key, "correct horse battery staple", &req)?;
    if *got == *SELF_TEST_EXPECTED {
        Ok(())
    } else {
        Err(Error::Format("self-test failed"))
    }
}

/// Expected output of the embedded self-test vector (also asserted against `testvectors/v1.json`).
pub const SELF_TEST_EXPECTED: &str = "oeTJYA7$yI7&s@kh*9Jm*D@k";
