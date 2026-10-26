// SPDX-FileCopyrightText: 2026 Dhiraj Patra
// SPDX-License-Identifier: MIT
//! Error type. Variants never carry secret data and messages are deliberately generic.

use core::fmt;

/// All failures of the core library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An input was empty, malformed or out of range. The message names the rule, never the value.
    InvalidInput(&'static str),
    /// Key stretching failed (for example the 64 MiB allocation was refused). Parameters are never lowered.
    Kdf,
    /// The derived byte stream ran out (practically unreachable).
    StreamExhausted,
    /// The OS random number generator failed.
    Random,
    /// A key or backup file is malformed or unsupported.
    Format(&'static str),
    /// Decryption failed: wrong passphrase, corrupt data or tampering (deliberately indistinguishable).
    Decrypt,
    /// An encrypted OpenSSH key needs a passphrase.
    PassphraseRequired,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidInput(m) => write!(f, "invalid input: {m}"),
            Error::Kdf => f.write_str("key stretching failed"),
            Error::StreamExhausted => f.write_str("internal error: derived stream exhausted"),
            Error::Random => f.write_str("random number generator failed"),
            Error::Format(m) => write!(f, "unsupported or malformed key data: {m}"),
            Error::Decrypt => f.write_str("decryption failed"),
            Error::PassphraseRequired => {
                f.write_str("this key is encrypted and needs a passphrase")
            }
        }
    }
}

impl std::error::Error for Error {}
